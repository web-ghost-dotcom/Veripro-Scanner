'use client';

import { useState, useRef, useEffect } from 'react';
import { motion } from 'framer-motion';

interface VeriProAIProps {
    contractCode: string;
    contractName?: string;
    onApplyCode: (code: string) => void;
    autoPrompt?: string | null;
    onPromptHandled?: () => void;
}

interface Message {
    role: 'user' | 'assistant';
    content: string;
}

// Syntax highlighter for Solidity code (same as SyntaxEditor)
function highlightSolidity(code: string): string {
    if (!code) return '';

    // Escape HTML first
    let highlighted = code
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');

    // Store preserved content (comments, strings) to prevent them from being modified
    const preserved: string[] = [];

    // Helper to preserve matches - use unique markers that won't appear in code
    const preserve = (html: string): string => {
        const index = preserved.length;
        preserved.push(html);
        return `<<<PRESERVED_${index}>>>`;
    };

    // Preserve multi-line comments first
    highlighted = highlighted.replace(/(\/\*[\s\S]*?\*\/)/g, (match) => {
        return preserve(`<span class="sol-comment">${match}</span>`);
    });

    // Preserve single-line comments
    highlighted = highlighted.replace(/(\/\/[^\n]*)/g, (match) => {
        return preserve(`<span class="sol-comment">${match}</span>`);
    });

    // Preserve strings (double quotes)
    highlighted = highlighted.replace(/"(?:[^"\\]|\\.)*"/g, (match) => {
        return preserve(`<span class="sol-string">${match}</span>`);
    });

    // Preserve strings (single quotes)
    highlighted = highlighted.replace(/'(?:[^'\\]|\\.)*'/g, (match) => {
        return preserve(`<span class="sol-string">${match}</span>`);
    });

    // Contract/Interface/Library names (must come before keywords)
    highlighted = highlighted.replace(/\b(contract|interface|library)\s+(\w+)/g, (_, keyword, name) => {
        return `<span class="sol-keyword">${keyword}</span> <span class="sol-contract">${name}</span>`;
    });

    // Function names (must come before keywords)
    highlighted = highlighted.replace(/\b(function)\s+(\w+)/g, (_, keyword, name) => {
        return `<span class="sol-keyword">${keyword}</span> <span class="sol-function">${name}</span>`;
    });

    // Built-in globals
    highlighted = highlighted.replace(/\b(msg\.sender|msg\.value|msg\.data|block\.timestamp|block\.number|tx\.origin|gasleft)\b/g,
        '<span class="sol-builtin">$1</span>');

    // Keywords
    const keywords = [
        'pragma', 'solidity', 'import', 'is', 'abstract', 'using', 'struct', 'enum',
        'mapping', 'event', 'emit', 'modifier', 'require', 'assert', 'revert',
        'public', 'private', 'internal', 'external', 'view', 'pure', 'payable',
        'returns', 'return', 'if', 'else', 'for', 'while', 'do', 'break', 'continue',
        'memory', 'storage', 'calldata', 'constant', 'immutable', 'virtual', 'override',
        'constructor', 'receive', 'fallback', 'try', 'catch', 'new', 'delete',
        'this', 'super', 'selfdestruct', 'type'
    ];
    const keywordRegex = new RegExp(`\\b(${keywords.join('|')})\\b`, 'g');
    highlighted = highlighted.replace(keywordRegex, '<span class="sol-keyword">$1</span>');

    // Types
    const types = [
        'uint256', 'uint128', 'uint64', 'uint32', 'uint16', 'uint8', 'uint',
        'int256', 'int128', 'int64', 'int32', 'int16', 'int8', 'int',
        'bool', 'address', 'bytes32', 'bytes', 'string',
        'bytes1', 'bytes2', 'bytes4', 'bytes8', 'bytes16', 'bytes20'
    ];
    const typeRegex = new RegExp(`\\b(${types.join('|')})\\b`, 'g');
    highlighted = highlighted.replace(typeRegex, '<span class="sol-type">$1</span>');

    // Numbers - be very careful to only match standalone numbers
    highlighted = highlighted.replace(/(?<=^|[\s(,=+\-*/<>!&|;:])(\d+)(?=[\s),;:+\-*/<>!&|]|$)/gm, '<span class="sol-number">$1</span>');

    // Restore preserved content
    for (let i = 0; i < preserved.length; i++) {
        highlighted = highlighted.replace(`<<<PRESERVED_${i}>>>`, preserved[i]);
    }

    return highlighted;
}

// Parse message content and render with syntax highlighting
function renderMessageContent(content: string) {
    const parts: React.ReactNode[] = [];
    const codeBlockRegex = /```(\w*)\n([\s\S]*?)\n```/g;
    let lastIndex = 0;
    let match;
    let keyIndex = 0;

    while ((match = codeBlockRegex.exec(content)) !== null) {
        // Add text before code block
        if (match.index > lastIndex) {
            parts.push(
                <span key={keyIndex++}>
                    {content.slice(lastIndex, match.index)}
                </span>
            );
        }

        // Add highlighted code block
        const language = match[1] || 'solidity';
        const code = match[2];
        const highlightedCode = highlightSolidity(code);

        parts.push(
            <div key={keyIndex++} className="my-3 rounded-lg overflow-hidden border border-zinc-700">
                <div className="bg-zinc-800 px-3 py-1.5 text-xs text-zinc-400 flex items-center justify-between border-b border-zinc-700">
                    <span className="font-mono">{language || 'code'}</span>
                    <div className="flex gap-1.5">
                        <div className="w-2.5 h-2.5 rounded-full bg-red-500/60" />
                        <div className="w-2.5 h-2.5 rounded-full bg-yellow-500/60" />
                        <div className="w-2.5 h-2.5 rounded-full bg-green-500/60" />
                    </div>
                </div>
                <pre className="bg-zinc-900 p-3 overflow-x-auto text-xs font-mono leading-relaxed text-zinc-300">
                    <code dangerouslySetInnerHTML={{ __html: highlightedCode }} />
                </pre>
            </div>
        );

        lastIndex = match.index + match[0].length;
    }

    // Add remaining text
    if (lastIndex < content.length) {
        parts.push(
            <span key={keyIndex++}>
                {content.slice(lastIndex)}
            </span>
        );
    }

    return parts.length > 0 ? parts : content;
}

export default function VeriProAI({ contractCode, contractName, onApplyCode, autoPrompt, onPromptHandled }: VeriProAIProps) {
    const [messages, setMessages] = useState<Message[]>([
        { role: 'assistant', content: 'Hello! I am VeriPro AI. I can help you write formal specifications for your smart contracts. What would you like to verify?' }
    ]);
    const [input, setInput] = useState('');
    const [isLoading, setIsLoading] = useState(false);
    const messagesEndRef = useRef<HTMLDivElement>(null);

    const scrollToBottom = () => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    };

    useEffect(() => {
        scrollToBottom();
    }, [messages]);

    const sendMessage = async (messageText: string) => {
        if (!messageText.trim() || isLoading) return;

        setMessages(prev => [...prev, { role: 'user', content: messageText }]);
        setIsLoading(true);

        try {
            const response = await fetch('/api/veripro-ai', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    prompt: messageText,
                    contractCode,
                    contractName
                })
            });

            const data = await response.json();

            if (data.error) {
                throw new Error(data.error);
            }

            setMessages(prev => [...prev, { role: 'assistant', content: data.result }]);
        } catch (error) {
            console.error('Failed to get AI response:', error);
            setMessages(prev => [...prev, { role: 'assistant', content: 'Sorry, I encountered an error while processing your request. Please ensure the API key is configured.' }]);
        } finally {
            setIsLoading(false);
        }
    };

    useEffect(() => {
        if (autoPrompt && !isLoading) {
            sendMessage(autoPrompt);
            if (onPromptHandled) {
                onPromptHandled();
            }
        }
    }, [autoPrompt]);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        const userMessage = input.trim();
        setInput('');
        await sendMessage(userMessage);
    };

    const extractCode = (content: string) => {
        const match = content.match(/```(?:solidity)?\n([\s\S]*?)\n```/);
        return match ? match[1] : null;
    };

    return (
        <div className="flex flex-col h-full bg-zinc-950">
            {/* CSS for syntax highlighting */}
            <style jsx global>{`
                .sol-comment { color: #71717a; font-style: italic; }
                .sol-string { color: #fbbf24; }
                .sol-keyword { color: #c084fc; }
                .sol-type { color: #4ade80; }
                .sol-function { color: #60a5fa; }
                .sol-contract { color: #fde047; font-weight: 600; }
                .sol-builtin { color: #22d3ee; }
                .sol-number { color: #fb923c; }
            `}</style>

            <div className="flex-1 overflow-y-auto p-4 space-y-4">
                {messages.map((msg, index) => (
                    <motion.div
                        key={index}
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                        className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
                    >
                        <div
                            className={`max-w-[95%] rounded-lg p-3 text-sm ${msg.role === 'user'
                                ? 'bg-zinc-800 text-white whitespace-pre-wrap'
                                : 'bg-zinc-900/50 text-zinc-300 border border-zinc-800'
                                }`}
                        >
                            {msg.role === 'assistant' ? renderMessageContent(msg.content) : msg.content}
                            {msg.role === 'assistant' && extractCode(msg.content) && (
                                <div className="mt-3 pt-3 border-t border-zinc-800">
                                    <button
                                        onClick={() => {
                                            const code = extractCode(msg.content);
                                            if (code) onApplyCode(code);
                                        }}
                                        className="text-xs flex items-center gap-1.5 text-green-400 hover:text-green-300 transition-colors bg-green-900/20 px-3 py-1.5 rounded border border-green-900/30 hover:bg-green-900/30"
                                    >
                                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 7v8a2 2 0 002 2h6M8 7V5a2 2 0 012-2h4.586a1 1 0 01.707.293l4.414 4.414a1 1 0 01.293.707V15a2 2 0 01-2 2h-2M8 7H6a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2v-2" />
                                        </svg>
                                        Insert Code to Editor
                                    </button>
                                </div>
                            )}
                        </div>
                    </motion.div>
                ))}
                {isLoading && (
                    <div className="flex justify-start">
                        <div className="bg-zinc-900/50 p-3 rounded-lg border border-zinc-800">
                            <div className="flex gap-1">
                                <div className="w-2 h-2 bg-zinc-500 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                                <div className="w-2 h-2 bg-zinc-500 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                                <div className="w-2 h-2 bg-zinc-500 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
                            </div>
                        </div>
                    </div>
                )}
                <div ref={messagesEndRef} />
            </div>

            <div className="p-4 border-t border-zinc-900">
                <form onSubmit={handleSubmit} className="relative">
                    <input
                        type="text"
                        value={input}
                        onChange={(e) => setInput(e.target.value)}
                        placeholder="Ask VeriPro AI..."
                        disabled={isLoading}
                        className="w-full px-4 py-3 pr-10 bg-zinc-900 border border-zinc-800 rounded text-sm text-white focus:outline-none focus:border-zinc-700 placeholder-zinc-600"
                    />
                    <button
                        type="submit"
                        disabled={!input.trim() || isLoading}
                        className="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 text-zinc-500 hover:text-white disabled:opacity-50 transition-colors"
                    >
                        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 5l7 7-7 7M5 5l7 7-7 7" />
                        </svg>
                    </button>
                </form>
            </div>
        </div>
    );
}
