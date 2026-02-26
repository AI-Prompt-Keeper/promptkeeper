"use client";

import { useState } from "react";
import DashboardLayout from "@/components/DashboardLayout";
import { mockModels } from "@/lib/mockData";

export default function FailoverPage() {
  const [primaryModel, setPrimaryModel] = useState(mockModels[0].id);
  const [backupModel, setBackupModel] = useState(mockModels[1].id);
  const [enabled, setEnabled] = useState(true);

  return (
    <DashboardLayout>
      <div className="mx-auto max-w-2xl">
        <h1 className="mb-2 text-2xl font-bold text-midnight">
          Failover Settings
        </h1>
        <p className="mb-6 text-sm text-[var(--text-light)]">
          Choose primary and backup models. If the primary fails, requests will
          automatically use the backup.
        </p>

        <div className="space-y-6 rounded-xl border border-cream-dark bg-white p-6 shadow-sm">
          <div className="flex items-center justify-between">
            <span className="font-medium text-midnight">Enable failover</span>
            <button
              type="button"
              role="switch"
              aria-checked={enabled}
              onClick={() => setEnabled((e) => !e)}
              className={`relative h-7 w-12 shrink-0 rounded-full transition ${
                enabled ? "bg-lavender" : "bg-cream-dark"
              }`}
            >
              <span
                className={`absolute top-1 h-5 w-5 rounded-full bg-white shadow transition ${
                  enabled ? "left-7" : "left-1"
                }`}
              />
            </button>
          </div>

          <div>
            <label className="mb-2 block text-sm font-medium text-midnight">
              Primary model
            </label>
            <div className="flex flex-wrap gap-2">
              {mockModels.map((m) => (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => setPrimaryModel(m.id)}
                  className={`rounded-lg border px-4 py-2 text-sm font-medium transition ${
                    primaryModel === m.id
                      ? "border-lavender bg-lavender/20 text-lavender-dark"
                      : "border-cream-dark bg-white text-[var(--text-light)] hover:border-lavender-light"
                  }`}
                >
                  {m.name}
                  <span className="ml-1 text-xs opacity-80">({m.provider})</span>
                </button>
              ))}
            </div>
            <p className="mt-1 text-xs text-[var(--text-light)]">
              Selected: {mockModels.find((m) => m.id === primaryModel)?.name}
            </p>
          </div>

          <div>
            <label className="mb-2 block text-sm font-medium text-midnight">
              Backup model
            </label>
            <div className="flex flex-wrap gap-2">
              {mockModels
                .filter((m) => m.id !== primaryModel)
                .map((m) => (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => setBackupModel(m.id)}
                    className={`rounded-lg border px-4 py-2 text-sm font-medium transition ${
                      backupModel === m.id
                        ? "border-lavender bg-lavender/20 text-lavender-dark"
                        : "border-cream-dark bg-white text-[var(--text-light)] hover:border-lavender-light"
                    }`}
                  >
                    {m.name}
                    <span className="ml-1 text-xs opacity-80">({m.provider})</span>
                  </button>
                ))}
            </div>
            <p className="mt-1 text-xs text-[var(--text-light)]">
              Selected: {mockModels.find((m) => m.id === backupModel)?.name}
            </p>
          </div>

          <div className="rounded-lg bg-cream/50 p-4 text-sm text-[var(--text-dark)]">
            <strong>Summary:</strong> Primary {mockModels.find((m) => m.id === primaryModel)?.name}
            {" → "}
            Backup {mockModels.find((m) => m.id === backupModel)?.name}
            {enabled ? "" : " (failover disabled)"}
          </div>
        </div>
      </div>
    </DashboardLayout>
  );
}
