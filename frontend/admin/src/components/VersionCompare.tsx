"use client";

import type { PromptVersion } from "@/types";

interface VersionCompareProps {
  left: PromptVersion | null;
  right: PromptVersion | null;
  onClose: () => void;
}

function formatDate(iso: string) {
  return new Date(iso).toLocaleString();
}

export default function VersionCompare({ left, right, onClose }: VersionCompareProps) {
  if (!left || !right) return null;

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-black/50 backdrop-blur-sm">
      <div className="flex items-center justify-between border-b border-cream-dark bg-white px-4 py-2">
        <h3 className="text-lg font-semibold text-midnight">Compare versions</h3>
        <button
          type="button"
          onClick={onClose}
          className="rounded px-3 py-1 text-sm font-medium text-midnight hover:bg-cream"
        >
          Close
        </button>
      </div>
      <div className="flex flex-1 overflow-hidden">
        <div className="flex-1 overflow-auto border-r border-cream-dark bg-cream/30 p-4">
          <div className="mb-2 text-xs font-semibold uppercase text-midnight-light">
            {left.label || formatDate(left.createdAt)} (L)
          </div>
          <div className="space-y-4">
            <div>
              <div className="mb-1 text-xs font-medium text-midnight">System</div>
              <pre className="whitespace-pre-wrap rounded border bg-white p-3 text-sm">
                {left.systemPrompt}
              </pre>
            </div>
            <div>
              <div className="mb-1 text-xs font-medium text-midnight">User</div>
              <pre className="whitespace-pre-wrap rounded border bg-white p-3 text-sm">
                {left.userPrompt}
              </pre>
            </div>
          </div>
        </div>
        <div className="flex-1 overflow-auto bg-cream/30 p-4">
          <div className="mb-2 text-xs font-semibold uppercase text-midnight-light">
            {right.label || formatDate(right.createdAt)} (R)
          </div>
          <div className="space-y-4">
            <div>
              <div className="mb-1 text-xs font-medium text-midnight">System</div>
              <pre className="whitespace-pre-wrap rounded border bg-white p-3 text-sm">
                {right.systemPrompt}
              </pre>
            </div>
            <div>
              <div className="mb-1 text-xs font-medium text-midnight">User</div>
              <pre className="whitespace-pre-wrap rounded border bg-white p-3 text-sm">
                {right.userPrompt}
              </pre>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
