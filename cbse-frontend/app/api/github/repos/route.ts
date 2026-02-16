import { NextRequest, NextResponse } from 'next/server';

// GitHub API base URL
const GITHUB_API = 'https://api.github.com';

interface GitHubFile {
  name: string;
  path: string;
  type: 'file' | 'dir';
  download_url: string | null;
  sha: string;
}

interface GitHubContent {
  name: string;
  path: string;
  content: string;
  encoding: string;
}

// Parse GitHub URL to extract owner and repo
function parseGitHubUrl(url: string): { owner: string; repo: string } | null {
  // Handle various GitHub URL formats
  const patterns = [
    /github\.com\/([^\/]+)\/([^\/]+?)(?:\.git)?(?:\/.*)?$/,
    /^([^\/]+)\/([^\/]+)$/,  // owner/repo format
  ];

  for (const pattern of patterns) {
    const match = url.match(pattern);
    if (match) {
      return { owner: match[1], repo: match[2].replace(/\.git$/, '') };
    }
  }
  return null;
}

// Recursively fetch all Solidity files from a directory
async function fetchSolidityFiles(
  owner: string,
  repo: string,
  branch: string,
  path: string,
  accessToken?: string
): Promise<{ name: string; path: string; content: string }[]> {
  const headers: HeadersInit = {
    'Accept': 'application/vnd.github.v3+json',
    'User-Agent': 'VeriPro-App',
  };

  if (accessToken) {
    headers['Authorization'] = `Bearer ${accessToken}`;
  }

  const url = `${GITHUB_API}/repos/${owner}/${repo}/contents/${path}?ref=${branch}`;
  const response = await fetch(url, { headers });

  if (!response.ok) {
    if (response.status === 404) {
      return [];
    }
    throw new Error(`GitHub API error: ${response.status} ${response.statusText}`);
  }

  const data: GitHubFile[] | GitHubContent = await response.json();
  const files: { name: string; path: string; content: string }[] = [];

  // Handle single file response
  if (!Array.isArray(data)) {
    const fileData = data as GitHubContent;
    if (fileData.name?.endsWith('.sol') && fileData.content) {
      const content = Buffer.from(fileData.content, 'base64').toString('utf-8');
      return [{ name: fileData.name, path: fileData.path, content }];
    }
    return [];
  }

  // Process directory listing
  for (const item of data) {
    if (item.type === 'dir') {
      // Recursively fetch subdirectories
      const subFiles = await fetchSolidityFiles(owner, repo, branch, item.path, accessToken);
      files.push(...subFiles);
    } else if (item.type === 'file' && item.name.endsWith('.sol')) {
      // Fetch file content
      const fileUrl = `${GITHUB_API}/repos/${owner}/${repo}/contents/${item.path}?ref=${branch}`;
      const fileResponse = await fetch(fileUrl, { headers });

      if (fileResponse.ok) {
        const fileData: GitHubContent = await fileResponse.json();
        if (fileData.content) {
          const content = Buffer.from(fileData.content, 'base64').toString('utf-8');
          files.push({ name: item.name, path: item.path, content });
        }
      }
    }
  }

  return files;
}

interface RepoRequest {
  githubUrl: string;
  branch?: string;
  contractsPath?: string;
  accessToken?: string;
}

export async function POST(request: NextRequest) {
  try {
    const body: RepoRequest = await request.json();
    const { githubUrl, branch = 'main', contractsPath = 'src/', accessToken } = body;

    if (!githubUrl) {
      return NextResponse.json({ error: 'GitHub URL is required' }, { status: 400 });
    }

    const parsed = parseGitHubUrl(githubUrl);
    if (!parsed) {
      return NextResponse.json({ error: 'Invalid GitHub URL format' }, { status: 400 });
    }

    const { owner, repo } = parsed;

    // Fetch Solidity files from the repository
    const files = await fetchSolidityFiles(owner, repo, branch, contractsPath.replace(/^\/|\/$/g, ''), accessToken);

    // If no files found in the specified path, try common alternatives
    if (files.length === 0) {
      const alternativePaths = ['contracts/', 'src/', '.'];
      for (const altPath of alternativePaths) {
        if (altPath !== contractsPath.replace(/^\/|\/$/g, '')) {
          const altFiles = await fetchSolidityFiles(owner, repo, branch, altPath, accessToken);
          if (altFiles.length > 0) {
            return NextResponse.json({
              success: true,
              owner,
              repo,
              branch,
              files: altFiles,
              note: `Found files in '${altPath}' instead of '${contractsPath}'`
            });
          }
        }
      }
    }

    if (files.length === 0) {
      return NextResponse.json({
        error: 'No Solidity files found in the repository',
        details: `Searched in '${contractsPath}' and common alternatives`
      }, { status: 404 });
    }

    return NextResponse.json({
      success: true,
      owner,
      repo,
      branch,
      files,
    });

  } catch (error) {
    console.error('GitHub API error:', error);
    const message = error instanceof Error ? error.message : String(error);

    if (message.includes('rate limit')) {
      return NextResponse.json({
        error: 'GitHub API rate limit exceeded',
        details: 'Try again later or connect your GitHub account for higher limits'
      }, { status: 429 });
    }

    return NextResponse.json({
      error: 'Failed to fetch repository',
      details: message
    }, { status: 500 });
  }
}

// GET endpoint to validate a GitHub URL
export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const url = searchParams.get('url');

  if (!url) {
    return NextResponse.json({ error: 'URL parameter required' }, { status: 400 });
  }

  const parsed = parseGitHubUrl(url);
  if (!parsed) {
    return NextResponse.json({ valid: false, error: 'Invalid GitHub URL format' });
  }

  const { owner, repo } = parsed;

  // Check if repo exists
  try {
    const response = await fetch(`${GITHUB_API}/repos/${owner}/${repo}`, {
      headers: {
        'Accept': 'application/vnd.github.v3+json',
        'User-Agent': 'VeriPro-App',
      },
    });

    if (response.ok) {
      const data = await response.json();
      return NextResponse.json({
        valid: true,
        owner,
        repo,
        defaultBranch: data.default_branch,
        private: data.private,
      });
    } else if (response.status === 404) {
      return NextResponse.json({
        valid: false,
        error: 'Repository not found or is private'
      });
    } else {
      return NextResponse.json({
        valid: false,
        error: `GitHub API error: ${response.status}`
      });
    }
  } catch (error) {
    return NextResponse.json({
      valid: false,
      error: error instanceof Error ? error.message : String(error)
    });
  }
}
