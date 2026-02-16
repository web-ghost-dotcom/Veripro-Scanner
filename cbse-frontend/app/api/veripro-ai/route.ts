import { NextRequest, NextResponse } from 'next/server';

const GEMINI_API_KEY = process.env.GOOGLE_API_KEY;
const API_URL = 'https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent';

interface VeriProRequest {
    prompt: string;
    contractCode: string;
    contractName?: string;
    contractPath?: string;
}

interface GeminiCandidate {
    content: {
        parts: {
            text: string;
        }[];
    };
}

interface GeminiResponse {
    candidates?: GeminiCandidate[];
    error?: {
        message: string;
    };
}

export async function POST(req: NextRequest) {
    if (!GEMINI_API_KEY) {
        return NextResponse.json(
            { error: 'GOOGLE_API_KEY is not configured' },
            { status: 500 }
        );
    }

    try {
        const { prompt, contractCode, contractName, contractPath } = await req.json() as VeriProRequest;

        if (!prompt) {
            return NextResponse.json(
                { error: 'Prompt is required' },
                { status: 400 }
            );
        }

        // Extract contract name from code if not provided
        const extractedContractName = contractName || extractContractName(contractCode) || 'Contract';

        const systemPrompt = `You are VeriPro AI, an expert AI Security Agent for Smart Contract Security.
Your goal is to scan smart contracts for vulnerabilities and generate formal verification specifications to prove their presence or absence.

ROLE:
1. Act as a Security Auditor specializing in Solidity, EVM, and Binance Smart Chain (BSC).
2. Identify potential vulnerabilities (Reentrancy, Access Control, Integer Overflows, Flash Loan attacks, ERC20 compliance).
3. Write formal specifications (Foundry/Solidity) to mathematically verify these properties.

ANALYSIS APPROACH:
- First, analyze the provided code for security risks.
- Second, generate a "Scan Report" comment block at the top of the test file explaining what you are testing.
- Third, write the Foundry tests to verify your findings.

CRITICAL IMPORT PATH RULES:
- The test file and the contract file will be in the SAME directory (src/)
- Always import the contract using: import "./${extractedContractName}.sol";
- For forge-std, use: import "forge-std/Test.sol";
- NEVER use "../src/" or "src/" in import paths
- NEVER use absolute paths

NAMING CONVENTIONS:
- Contract being tested: ${extractedContractName}
- Test contract name should be: ${extractedContractName}Test or ${extractedContractName}InvariantTest
- Test functions should start with: test_ or invariant_

TEMPLATE STRUCTURE:
\`\`\`solidity
// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "./${extractedContractName}.sol";

/**
 * @title VeriPro Security Scan Report
 * @notice Target: ${extractedContractName}
 * @dev This suite verifies the following security properties:
 * [List your findings and what these tests cover]
 */ 
contract ${extractedContractName}Test is Test {
    ${extractedContractName} public target;

    function setUp() public {
        target = new ${extractedContractName}();
        // Setup additional state if needed
    }

    // ... Generated Tests ...
}
\`\`\`

If the user asks for a specific property, focus on that. If they ask for a "scan" or "audit", perform a comprehensive analysis.
Keep responses concise and code-focused. Wrap generated code in markdown code blocks.
`;

        const userContent = `
Contract Name: ${extractedContractName}
${contractPath ? `Contract Path: ${contractPath}` : ''}

${contractCode ? `Contract Code:\n\`\`\`solidity\n${contractCode}\n\`\`\`\n` : ''}

User Request: ${prompt}
`;

        const payload = {
            contents: [{
                parts: [{
                    text: systemPrompt + "\n" + userContent
                }]
            }]
        };

        const response = await fetch(`${API_URL}?key=${GEMINI_API_KEY}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(payload)
        });

        if (!response.ok) {
            const errorData = await response.json() as GeminiResponse;
            console.error('Gemini API Error:', errorData);
            return NextResponse.json(
                { error: 'Failed to generate content from Gemini' },
                { status: response.status }
            );
        }

        const data = await response.json() as GeminiResponse;
        let generatedText = data.candidates?.[0]?.content?.parts?.[0]?.text || '';

        // Post-process to fix any incorrect import paths the AI might still generate
        generatedText = fixImportPaths(generatedText, extractedContractName);

        return NextResponse.json({ result: generatedText });

    } catch (error) {
        console.error('VeriPro AI Error:', error);
        return NextResponse.json(
            { error: 'Internal server error' },
            { status: 500 }
        );
    }
}

// Extract contract name from Solidity code
function extractContractName(code: string): string | null {
    if (!code) return null;
    const match = code.match(/contract\s+(\w+)/);
    return match ? match[1] : null;
}

// Fix common import path issues in AI-generated code
function fixImportPaths(text: string, contractName: string): string {
    let fixed = text;

    // Fix: "../src/ContractName.sol" -> "./ContractName.sol"
    fixed = fixed.replace(new RegExp(`"\\.\\./src/${contractName}\\.sol"`, 'g'), `"./${contractName}.sol"`);

    // Fix: "src/ContractName.sol" -> "./ContractName.sol"
    fixed = fixed.replace(new RegExp(`"src/${contractName}\\.sol"`, 'g'), `"./${contractName}.sol"`);

    // Fix: "../ContractName.sol" -> "./ContractName.sol"
    fixed = fixed.replace(new RegExp(`"\\.\\./${contractName}\\.sol"`, 'g'), `"./${contractName}.sol"`);

    // Fix generic patterns for any contract name
    // "../src/*.sol" -> "./*.sol"
    fixed = fixed.replace(/"\.\.\/src\/(\w+)\.sol"/g, '"./$1.sol"');

    // "src/*.sol" -> "./*.sol"
    fixed = fixed.replace(/"src\/(\w+)\.sol"/g, '"./$1.sol"');

    return fixed;
}
