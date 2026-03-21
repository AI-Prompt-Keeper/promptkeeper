"use client";

import { useRef, useEffect, useCallback } from "react";

/** Highlights {{variable}} in the mirror div */
function highlightVariables(text: string): string {
  return text.replace(
    /\{\{([^}]+)\}\}/g,
    '<span class="token-variable">{{$1}}</span>'
  );
}

interface PromptEditorProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  label: string;
  minRows?: number;
}

export default function PromptEditor({
  value,
  onChange,
  placeholder = "",
  label,
  minRows = 8,
}: PromptEditorProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const mirrorRef = useRef<HTMLDivElement>(null);

  const syncScroll = useCallback(() => {
    const ta = textareaRef.current;
    const mirror = mirrorRef.current;
    if (ta && mirror) mirror.scrollTop = ta.scrollTop;
  }, []);

  const syncSize = useCallback(() => {
    const ta = textareaRef.current;
    const mirror = mirrorRef.current;
    if (ta && mirror) {
      mirror.style.height = `${ta.scrollHeight}px`;
    }
  }, [value]);

  useEffect(() => {
    syncSize();
  }, [value, syncSize]);

  return (
    <div className="relative">
      <label className="mb-1 block text-sm font-medium text-midnight">
        {label}
      </label>
      <div className="relative overflow-hidden rounded-lg border border-cream-dark bg-white font-mono text-sm">
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onScroll={syncScroll}
          placeholder={placeholder}
          rows={minRows}
          className="absolute inset-0 w-full resize-none bg-transparent p-4 pr-6 text-transparent caret-midnight selection:bg-lavender/30 focus:outline-none"
          spellCheck={false}
          style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}
        />
        <div
          ref={mirrorRef}
          className="pointer-events-none overflow-hidden whitespace-pre-wrap break-words p-4 pr-6 text-transparent"
          aria-hidden
          style={{
            minHeight: `${minRows * 1.6}rem`,
            wordBreak: "break-word",
          }}
        >
          <span
            dangerouslySetInnerHTML={{
              __html: highlightVariables(
                value || placeholder
                  ? value || placeholder
                  : "\u00A0"
              ),
            }}
          />
        </div>
      </div>
    </div>
  );
}
