import { NextRequest, NextResponse } from 'next/server';

// GitHub OAuth configuration
const GITHUB_CLIENT_ID = process.env.GITHUB_CLIENT_ID || '';
const GITHUB_CLIENT_SECRET = process.env.GITHUB_CLIENT_SECRET || '';

// Handle GitHub OAuth callback
interface GitHubTokenResponse {
  access_token?: string;
  token_type?: string;
  scope?: string;
  error?: string;
  error_description?: string;
}

interface GitHubUser {
  login: string;
  id: number;
  avatar_url: string;
  name: string;
  [key: string]: unknown; // Allow other properties
}

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const code = searchParams.get('code');
  const state = searchParams.get('state');
  const error = searchParams.get('error');

  // Check for errors from GitHub
  if (error) {
    return NextResponse.redirect(new URL('/app/settings?error=github_denied', request.url));
  }

  if (!code) {
    return NextResponse.redirect(new URL('/app/settings?error=no_code', request.url));
  }

  // Verify state matches
  const storedState = request.cookies.get('github_oauth_state')?.value;
  if (!storedState || storedState !== state) {
    return NextResponse.redirect(new URL('/app/settings?error=invalid_state', request.url));
  }

  try {
    // Exchange code for access token
    const tokenResponse = await fetch('https://github.com/login/oauth/access_token', {
      method: 'POST',
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        client_id: GITHUB_CLIENT_ID,
        client_secret: GITHUB_CLIENT_SECRET,
        code,
      }),
    });

    if (!tokenResponse.ok) {
      throw new Error('Failed to exchange code for token');
    }

    const tokenData: GitHubTokenResponse = await tokenResponse.json();

    if (tokenData.error || !tokenData.access_token) {
      throw new Error(tokenData.error_description || tokenData.error || 'No access token received');
    }

    const accessToken = tokenData.access_token;

    // Fetch user info
    const userResponse = await fetch('https://api.github.com/user', {
      headers: {
        'Authorization': `Bearer ${accessToken}`,
        'Accept': 'application/vnd.github.v3+json',
      },
    });

    if (!userResponse.ok) {
      throw new Error('Failed to fetch user info');
    }

    const userData: GitHubUser = await userResponse.json();

    // Create response with redirect to settings
    const response = NextResponse.redirect(new URL('/app/settings?success=github_connected', request.url));

    // Store token and user info in secure cookies
    // In production, you'd want to encrypt this or use a session store
    response.cookies.set('github_access_token', accessToken, {
      httpOnly: true,
      secure: process.env.NODE_ENV === 'production',
      sameSite: 'lax',
      maxAge: 60 * 60 * 24 * 30, // 30 days
    });

    response.cookies.set('github_username', userData.login, {
      httpOnly: false, // Allow client-side access
      secure: process.env.NODE_ENV === 'production',
      sameSite: 'lax',
      maxAge: 60 * 60 * 24 * 30,
    });

    // Clear the state cookie
    response.cookies.delete('github_oauth_state');

    return response;

  } catch (error) {
    console.error('GitHub OAuth error:', error);
    const message = error instanceof Error ? error.message : String(error);
    return NextResponse.redirect(
      new URL(`/app/settings?error=oauth_failed&message=${encodeURIComponent(message)}`, request.url)
    );
  }
}
