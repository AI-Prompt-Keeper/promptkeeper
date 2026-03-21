"use client";

import { useState } from "react";
import type { PromptVersion } from "@/types";

function formatDate(iso: string) {
  const d = new Date(iso);
  return d.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

interface VersionHistoryProps {
  versions: PromptVersion[];
  currentVersionId: string | null;
  onSelectVersion: (id: string) => void;
  onPromoteToProd: (id: string) => void;
  onCompare: (leftId: string, rightId: string) => void;
}

export default function VersionHistory({
  versions,
  currentVersionId,
  onSelectVersion,
  onPromoteToProd,
  onCompare,
}: VersionHistoryProps) {
  const [compareLeft, setCompareLeft] = useState<string | null>(null);
  const [compareRight, setCompareRight] = useState<string | null>(null);
  const [showCompare, setShowCompare] = useState(false);

  const startCompare = () => {
    if (compareLeft && compareRight && compareLeft !== compareRight) {
      onCompare(compareLeft, compareRight);
      setShowCompare(true);
    }
  };

  return (
    <div className="flex h-full flex-col rounded-lg border border-cream-dark bg-white">
      <div className="border-b border-cream-dark px-3 py-2 font-medium text-midnight">
        Version History
      </div>
      <ul className="flex-1 overflow-auto p-2">
        {versions.map((v) => (
          <li
            key={v.id}
            className={`group rounded-lg border p-2 text-sm ${
              currentVersionId === v.id
                ? "border-lavender bg-lavender/10"
                : "border-transparent hover:bg-cream/50"
            }`}
          >
            <div className="flex items-center justify-between gap-1">
              <span className="font-medium text-midnight">
                {v.label || formatDate(v.createdAt)}
              </span>
              {v.isProduction && (
                <span className="rounded bg-lavender/30 px-1.5 py-0.5 text-xs text-lavender-dark">
                  Prod
                </span>
              )}
            </div>
            <div className="mt-0.5 text-xs text-[var(--text-light)]">
              {formatDate(v.createdAt)}
            </div>
            <div className="mt-2 flex flex-wrap gap-1">
              <button
                type="button"
                onClick={() => onSelectVersion(v.id)}
                className="text-xs font-medium text-lavender-dark hover:underline"
              >
                View
              </button>
              {!v.isProduction && (
                <button
                  type="button"
                  onClick={() => onPromoteToProd(v.id)}
                  className="text-xs font-medium text-lavender-dark hover:underline"
                >
                  Promote to Prod
                </button>
              )}
              <label className="flex items-center gap-1 text-xs text-[var(--text-light)]">
                <input
                  type="radio"
                  name="compareLeft"
                  checked={compareLeft === v.id}
                  onChange={() => setCompareLeft(v.id)}
                />
                L
              </label>
              <label className="flex items-center gap-1 text-xs text-[var(--text-light)]">
                <input
                  type="radio"
                  name="compareRight"
                  checked={compareRight === v.id}
                  onChange={() => setCompareRight(v.id)}
                />
                R
              </label>
            </div>
          </li>
        ))}
      </ul>
      <div className="border-t border-cream-dark p-2">
        <button
          type="button"
          onClick={startCompare}
          disabled={!compareLeft || !compareRight || compareLeft === compareRight}
          className="w-full rounded bg-midnight-light py-1.5 text-xs font-medium text-white disabled:opacity-50"
        >
          Compare L vs R
        </button>
      </div>
    </div>
  );
}
