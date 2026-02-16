'use client';

import { useRef, useEffect, useCallback } from 'react';

interface SyntaxEditorProps {
    value: string;
    onChange: (value: string) => void;
    placeholder?: string;
    readOnly?: boolean;
}

// Syntax highlighter for Solidity code
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
    // Match numbers that are preceded by whitespace, operators, or start of line
    // and followed by whitespace, operators, semicolon, or end of line
    highlighted = highlighted.replace(/(?<=^|[\s(,=+\-*/<>!&|;:])(\d+)(?=[\s),;:+\-*/<>!&|]|$)/gm, '<span class="sol-number">$1</span>');
    
    // Restore preserved content
    for (let i = 0; i < preserved.length; i++) {
        highlighted = highlighted.replace(`<<<PRESERVED_${i}>>>`, preserved[i]);
    }
    
    return highlighted;
}

export default function SyntaxEditor({ value, onChange, placeholder, readOnly = false }: SyntaxEditorProps) {
    const textareaRef = useRef<HTMLTextAreaElement>(null);
    const highlightRef = useRef<HTMLPreElement>(null);
    const lineNumbersRef = useRef<HTMLDivElement>(null);

    // Sync scroll between textarea, highlight layer, and line numbers
    const syncScroll = useCallback(() => {
        if (textareaRef.current && highlightRef.current) {
            highlightRef.current.scrollTop = textareaRef.current.scrollTop;
            highlightRef.current.scrollLeft = textareaRef.current.scrollLeft;
        }
        if (textareaRef.current && lineNumbersRef.current) {
            lineNumbersRef.current.scrollTop = textareaRef.current.scrollTop;
        }
    }, []);

    useEffect(() => {
        const textarea = textareaRef.current;
        if (textarea) {
            textarea.addEventListener('scroll', syncScroll);
            return () => textarea.removeEventListener('scroll', syncScroll);
        }
    }, [syncScroll]);

    const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
        // Handle Tab key for indentation
        if (e.key === 'Tab') {
            e.preventDefault();
            const textarea = textareaRef.current;
            if (!textarea) return;

            const start = textarea.selectionStart;
            const end = textarea.selectionEnd;
            const newValue = value.substring(0, start) + '    ' + value.substring(end);
            onChange(newValue);

            // Restore cursor position
            requestAnimationFrame(() => {
                textarea.selectionStart = textarea.selectionEnd = start + 4;
            });
        }
    };

    const lines = value.split('\n');
    const lineCount = lines.length;

    // Common text styles to ensure both layers render identically
    const textStyles = {
        fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
        fontSize: '14px',
        lineHeight: '26px',
        letterSpacing: 'normal',
        wordSpacing: 'normal',
        tabSize: 4,
        whiteSpace: 'pre' as const,
        overflowWrap: 'normal' as const,
        wordBreak: 'normal' as const,
    };

    return (
        <div className="relative w-full h-full flex bg-zinc-950">
            {/* Inline styles for syntax highlighting */}
            <style jsx global>{`
                .sol-comment { color: #71717a; font-style: italic; }
                .sol-string { color: #fbbf24; }
                .sol-keyword { color: #c084fc; }
                .sol-type { color: #4ade80; }
                .sol-function { color: #60a5fa; }
                .sol-contract { color: #fde047; font-weight: 600; }
                .sol-builtin { color: #22d3ee; }
                .sol-number { color: #fb923c; }
                
                .syntax-editor-textarea,
                .syntax-editor-highlight {
                    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace !important;
                    font-size: 14px !important;
                    line-height: 26px !important;
                    letter-spacing: normal !important;
                    word-spacing: normal !important;
                    tab-size: 4 !important;
                    -moz-tab-size: 4 !important;
                    white-space: pre !important;
                    overflow-wrap: normal !important;
                    word-break: normal !important;
                }
            `}</style>
            
            {/* Line numbers */}
            <div 
                ref={lineNumbersRef}
                className="flex-shrink-0 bg-zinc-900/50 border-r border-zinc-800 select-none overflow-hidden"
            >
                <div 
                    className="p-4 pr-3 text-right"
                    style={{ ...textStyles, color: '#52525b' }}
                >
                    {Array.from({ length: Math.max(lineCount, 1) }, (_, i) => (
                        <div key={i}>
                            {i + 1}
                        </div>
                    ))}
                </div>
            </div>

            {/* Editor area */}
            <div className="relative flex-1 overflow-hidden">
                {/* Syntax highlighted layer (background) */}
                <pre
                    ref={highlightRef}
                    className="syntax-editor-highlight absolute inset-0 p-4 m-0 overflow-auto pointer-events-none text-zinc-300"
                    style={textStyles}
                    aria-hidden="true"
                    dangerouslySetInnerHTML={{ __html: highlightSolidity(value) || '\u00A0' }}
                />

                {/* Editable textarea (foreground, transparent text) */}
                <textarea
                    ref={textareaRef}
                    value={value}
                    onChange={(e) => onChange(e.target.value)}
                    onKeyDown={handleKeyDown}
                    onScroll={syncScroll}
                    readOnly={readOnly}
                    spellCheck={false}
                    placeholder={placeholder}
                    className="syntax-editor-textarea absolute inset-0 w-full h-full p-4 m-0 bg-transparent text-transparent caret-white resize-none focus:outline-none selection:bg-blue-500/30 placeholder-zinc-600 border-0"
                    style={{ 
                        ...textStyles,
                        caretColor: 'white',
                        outline: 'none',
                    }}
                />
            </div>
        </div>
    );
}
