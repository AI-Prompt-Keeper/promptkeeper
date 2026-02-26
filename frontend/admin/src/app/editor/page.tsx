"use client";

import { useSearchParams } from "next/navigation";
import { useMemo, useState, useCallback } from "react";
import DashboardLayout from "@/components/DashboardLayout";
import PromptEditor from "@/components/PromptEditor";
import TestConsole from "@/components/TestConsole";
import VersionHistory from "@/components/VersionHistory";
import VersionCompare from "@/components/VersionCompare";
import { mockVersions, mockFunctions } from "@/lib/mockData";
import type { PromptVersion } from "@/types";

const functionId = "fn-1"; // from URL in real app

function getVersionsForFunction(fnId: string): PromptVersion[] {
  return mockVersions.filter((v) => v.functionId === fnId);
}

export default function EditorPage() {
  const searchParams = useSearchParams();
  const fnId = searchParams.get("functionId") || functionId;
  const versions = useMemo(() => getVersionsForFunction(fnId), [fnId]);
  const fnName =
    mockFunctions.find((f) => f.id === fnId)?.name ?? "Unknown function";

  const [currentVersionId, setCurrentVersionId] = useState<string | null>(
    versions[0]?.id ?? null
  );
  const [systemPrompt, setSystemPrompt] = useState(
    versions[0]?.systemPrompt ?? ""
  );
  const [userPrompt, setUserPrompt] = useState(versions[0]?.userPrompt ?? "");
  const [compareLeft, setCompareLeft] = useState<PromptVersion | null>(null);
  const [compareRight, setCompareRight] = useState<PromptVersion | null>(null);
  const [showCompare, setShowCompare] = useState(false);

  const currentVersion = versions.find((v) => v.id === currentVersionId);

  const handleSelectVersion = useCallback(
    (id: string) => {
      const v = versions.find((x) => x.id === id);
      if (v) {
        setCurrentVersionId(v.id);
        setSystemPrompt(v.systemPrompt);
        setUserPrompt(v.userPrompt);
      }
    },
    [versions]
  );

  const handlePromoteToProd = useCallback((id: string) => {
    // In real app: PATCH /api/functions/:id/deploy { versionId, tag: 'production' }
    alert(`Promote version ${id} to production (API not connected).`);
  }, []);

  const handleCompare = useCallback((leftId: string, rightId: string) => {
    const left = versions.find((v) => v.id === leftId) ?? null;
    const right = versions.find((v) => v.id === rightId) ?? null;
    setCompareLeft(left);
    setCompareRight(right);
    setShowCompare(true);
  }, [versions]);

  return (
    <DashboardLayout>
      <div className="flex h-[calc(100vh-6rem)] gap-4">
        <div className="flex flex-1 flex-col gap-4 overflow-hidden">
          <h1 className="text-xl font-bold text-midnight">
            Prompt Editor — {fnName}
          </h1>
          <div className="flex flex-1 gap-4 overflow-hidden">
            <div className="flex min-w-0 flex-1 flex-col gap-4 overflow-auto">
              <PromptEditor
                label="System prompt (Markdown supported)"
                value={systemPrompt}
                onChange={setSystemPrompt}
                placeholder="You are a helpful assistant..."
                minRows={6}
              />
              <PromptEditor
                label="User prompt (Markdown supported)"
                value={userPrompt}
                onChange={setUserPrompt}
                placeholder="User message: {{user_input}}"
                minRows={6}
              />
            </div>
            <div className="w-72 shrink-0">
              <TestConsole
                functionId={fnId}
                systemPrompt={systemPrompt}
                userPrompt={userPrompt}
              />
            </div>
          </div>
        </div>
        <div className="w-64 shrink-0">
          <VersionHistory
            versions={versions}
            currentVersionId={currentVersionId}
            onSelectVersion={handleSelectVersion}
            onPromoteToProd={handlePromoteToProd}
            onCompare={handleCompare}
          />
        </div>
      </div>
      {showCompare && (
        <VersionCompare
          left={compareLeft}
          right={compareRight}
          onClose={() => setShowCompare(false)}
        />
      )}
    </DashboardLayout>
  );
}
