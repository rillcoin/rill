import React from "react";

interface CodeBlockProps {
  children: React.ReactNode;
  language?: string;
  title?: string;
}

export default function CodeBlock({ children, language, title }: CodeBlockProps) {
  return (
    <div
      className="rounded-lg overflow-hidden mb-4"
      style={{
        border: "1px solid var(--border-dim)",
        background: "var(--raised)",
      }}
    >
      {(title || language) && (
        <div
          className="flex items-center justify-between px-4 py-2"
          style={{
            borderBottom: "1px solid var(--border-subtle)",
            background: "var(--surface)",
          }}
        >
          {title && (
            <span
              className="text-xs font-medium"
              style={{ color: "var(--text-muted)" }}
            >
              {title}
            </span>
          )}
          {language && !title && (
            <span
              className="text-xs font-mono"
              style={{ color: "var(--text-dim)" }}
            >
              {language}
            </span>
          )}
        </div>
      )}
      <pre
        className="overflow-x-auto p-4 text-sm leading-relaxed scrollbar-thin"
        style={{
          fontFamily: "var(--font-jetbrains-mono), Menlo, monospace",
          color: "var(--text-secondary)",
          margin: 0,
        }}
      >
        <code>{children}</code>
      </pre>
    </div>
  );
}
