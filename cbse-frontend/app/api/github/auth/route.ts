import { NextRequest, NextResponse } from 'next/server';

// GitHub OAuth configuration
const GITHUB_CLIENT_ID = process.env.GITHUB_CLIENT_ID || '';
const GITHUB_CLIENT_SECRET = process.env.GITHUB_CLIENT_SECRET || '';
const GITHUB_REDIRECT_URI = process.env.GITHUB_REDIRECT_URI || 'http://localhost:3000/api/github/callback';

// Initiate GitHub OAuth flow
export async function GET(request: NextRequest) {
  if (!GITHUB_CLIENT_ID) {
    return NextResponse.json({
      error: 'GitHub OAuth not configured',
      details: 'Set GITHUB_CLIENT_ID environment variable'
    }, { status: 500 });
  }

  // Generate a state for CSRF protection
  const state = Math.random().toString(36).substring(7);

  // Build the GitHub authorization URL
  const authUrl = new URL('https://github.com/login/oauth/authorize');
  authUrl.searchParams.set('client_id', GITHUB_CLIENT_ID);
  authUrl.searchParams.set('redirect_uri', GITHUB_REDIRECT_URI);
  authUrl.searchParams.set('scope', 'read:user repo');
  authUrl.searchParams.set('state', state);

  // Create response with redirect
  const response = NextResponse.redirect(authUrl.toString());

  // Store state in cookie for verification
  response.cookies.set('github_oauth_state', state, {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'lax',
    maxAge: 60 * 10, // 10 minutes
  });

  return response;
}
