import { NextRequest, NextResponse } from 'next/server';

const COORDINATOR_URL = process.env.COORDINATOR_URL;

export async function POST(req: NextRequest) {
    // If coordinator URL is not configured
    if (!COORDINATOR_URL) {
        return NextResponse.json(
            {
                status: 'Error',
                message: 'Verification service is not configured. The CBSE coordinator is not deployed yet. Please check back later or run locally for testing.',
                job_id: null,
                attestation: null,
            },
            { status: 503 }
        );
    }

    try {
        // Forward the request to the coordinator
        const body = await req.json();

        const response = await fetch(`${COORDINATOR_URL}/verify`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(body),
        });

        const data: unknown = await response.json();
        return NextResponse.json(data, { status: response.status });
    } catch (error) {
        console.error('Verification error:', error);
        return NextResponse.json(
            {
                status: 'Error',
                message: `Failed to connect to verification service: ${error instanceof Error ? error.message : 'Unknown error'}`,
                job_id: null,
                attestation: null,
            },
            { status: 502 }
        );
    }
}

// Handle other methods
export async function GET() {
    return NextResponse.json(
        { error: 'Method not allowed. Use POST to submit verification requests.' },
        { status: 405 }
    );
}
