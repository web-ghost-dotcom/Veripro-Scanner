import { NextRequest, NextResponse } from 'next/server';

interface GitHubRepo {
    id: number;
    name: string;
    full_name: string;
    html_url: string;
    description: string | null;
    private: boolean;
    default_branch: string;
    language: string | null;
    updated_at: string;
    owner: {
        login: string;
        avatar_url: string;
    };
}

// Fetch user's repositories using their access token
export async function GET(request: NextRequest) {
    // Get access token from cookie
    const accessToken = request.cookies.get('github_access_token')?.value;

    if (!accessToken) {
        return NextResponse.json({
            error: 'Not authenticated',
            details: 'Please connect your GitHub account first'
        }, { status: 401 });
    }

    try {
        // Fetch user's repositories
        const response = await fetch('https://api.github.com/user/repos?per_page=100&sort=updated', {
            headers: {
                'Authorization': `Bearer ${accessToken}`,
                'Accept': 'application/vnd.github.v3+json',
                'User-Agent': 'VeriPro-App',
            },
        });

        if (!response.ok) {
            if (response.status === 401) {
                return NextResponse.json({
                    error: 'Token expired',
                    details: 'Please reconnect your GitHub account'
                }, { status: 401 });
            }
            throw new Error(`GitHub API error: ${response.status}`);
        }

        const repos: GitHubRepo[] = await response.json();

        // Return simplified repo info
        const simplifiedRepos = repos.map((repo) => ({
            id: repo.id,
            name: repo.name,
            full_name: repo.full_name,
            html_url: repo.html_url,
            description: repo.description,
            private: repo.private,
            default_branch: repo.default_branch,
            language: repo.language,
            updated_at: repo.updated_at,
            owner: {
                login: repo.owner.login,
                avatar_url: repo.owner.avatar_url,
            }
        }));

        return NextResponse.json({
            success: true,
            repos: simplifiedRepos,
        });

    } catch (error) {
        console.error('Failed to fetch repos:', error);
        return NextResponse.json({
            error: 'Failed to fetch repositories',
            details: error instanceof Error ? error.message : String(error)
        }, { status: 500 });
    }
}
