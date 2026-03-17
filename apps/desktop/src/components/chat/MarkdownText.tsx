import { useMemo } from 'react';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
import remarkGfm from 'remark-gfm';

interface MarkdownTextProps {
  text: string;
}

export function MarkdownText({ text }: MarkdownTextProps) {
  const components = useMemo(
    () => ({
      code({ node, inline, className, children, ...props }: any) {
        const match = /language-(\w+)/.exec(className || '');
        return !inline && match ? (
          <SyntaxHighlighter
            style={vscDarkPlus}
            language={match[1]}
            PreTag="div"
            {...props}
          >
            {String(children).replace(/\n$/, '')}
          </SyntaxHighlighter>
        ) : (
          <code className="rounded bg-muted px-1 py-0.5 text-sm" {...props}>
            {children}
          </code>
        );
      },
    }),
    []
  );

  return (
    <div className="prose prose-invert max-w-none prose-pre:bg-transparent prose-pre:p-0">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={components}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}
