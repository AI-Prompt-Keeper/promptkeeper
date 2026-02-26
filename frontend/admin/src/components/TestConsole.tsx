"use client";

import { useState, useCallback } from "react";

/** Extract {{variable}} names from template text */
function extractVariables(text: string): string[] {
  const set = new Set<string>();
  const re = /\{\{([^}]+)\}\}/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) set.add(m[1].trim());
  return Array.from(set);
}

interface TestConsoleProps {
  systemPrompt: string;
  userPrompt: string;
  functionId: string;
}

export default function TestConsole({
  systemPrompt,
  userPrompt,
  functionId,
}: TestConsoleProps) {
  const variables = extractVariables(systemPrompt + "\n" + userPrompt);
  const [inputs, setInputs] = useState<Record<string, string>>(() => {
    const o: Record<string, string> = {};
    variables.forEach((v) => (o[v] = ""));
    return o;
  });
  const [output, setOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const varsMap = Object.fromEntries(
    variables.map((k) => [k, inputs[k] ?? ""])
  );

  const runTest = useCallback(async () => {
    setError(null);
    setOutput("");
    setLoading(true);
    try {
      const res = await fetch(
        process.env.NEXT_PUBLIC_EXECUTE_URL || "http://localhost:8080/v1/execute",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            function_id: functionId,
            variables: varsMap,
          }),
        }
      );
      if (!res.ok) {
        const t = await res.text();
        throw new Error(t || `HTTP ${res.status}`);
      }
      const reader = res.body?.getReader();
      const decoder = new TextDecoder();
      if (!reader) {
        setOutput("(No response body)");
        return;
      }
      let out = "";
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        const chunk = decoder.decode(value, { stream: true });
        const lines = chunk.split("\n").filter((l) => l.startsWith("data: "));
        for (const line of lines) {
          const data = line.slice(6);
          if (data === "[DONE]") continue;
          try {
            const j = JSON.parse(data);
            if (j.text) out += j.text;
            if (j.error) out += `[Error: ${j.error}]`;
          } catch {
            // skip non-JSON lines
          }
        }
      }
      setOutput(out || "(Empty response)");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setOutput("");
    } finally {
      setLoading(false);
    }
  }, [functionId, varsMap]);

  return (
    <div className="flex h-full flex-col rounded-lg border border-cream-dark bg-white">
      <div className="border-b border-cream-dark px-3 py-2 font-medium text-midnight">
        Test
      </div>
      <div className="flex-1 overflow-auto p-3">
        <div className="space-y-2">
          {variables.length === 0 ? (
            <p className="text-sm text-[var(--text-light)]">
              No variables in prompts. Add {"{{variable_name}}"} to test.
            </p>
          ) : (
            variables.map((v) => (
              <div key={v}>
                <label className="mb-0.5 block text-xs font-medium text-midnight-light">
                  {v}
                </label>
                <input
                  type="text"
                  value={inputs[v] ?? ""}
                  onChange={(e) =>
                    setInputs((prev) => ({ ...prev, [v]: e.target.value }))
                  }
                  placeholder={`${v}...`}
                  className="w-full rounded border border-cream-dark px-2 py-1.5 text-sm focus:border-lavender focus:outline-none focus:ring-1 focus:ring-lavender"
                />
              </div>
            ))
          )}
        </div>
        <button
          type="button"
          onClick={runTest}
          disabled={loading}
          className="mt-3 w-full rounded-lg bg-lavender-dark px-3 py-2 text-sm font-medium text-white transition hover:bg-lavender disabled:opacity-50"
        >
          {loading ? "Running…" : "Run"}
        </button>
        {(output || error) && (
          <div className="mt-3 rounded border border-cream-dark bg-cream/30 p-2">
            <div className="mb-1 text-xs font-medium text-midnight-light">
              Output
            </div>
            <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-words text-sm text-[var(--text-dark)]">
              {error ? `Error: ${error}` : output}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
