"use client";

import { useState, useMemo } from "react";
import Link from "next/link";
import DashboardLayout from "@/components/DashboardLayout";
import { mockFunctions } from "@/lib/mockData";

export default function FunctionListPage() {
  const [search, setSearch] = useState("");
  const filtered = useMemo(
    () =>
      mockFunctions.filter((f) =>
        f.name.toLowerCase().includes(search.toLowerCase().trim())
      ),
    [search]
  );

  return (
    <DashboardLayout>
      <div className="mx-auto max-w-5xl">
        <h1 className="mb-6 text-2xl font-bold text-midnight">
          AI Functions
        </h1>
        <div className="mb-4 flex items-center gap-4">
          <input
            type="search"
            placeholder="Search by function name..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full max-w-sm rounded-lg border border-cream-dark bg-white px-4 py-2 text-sm shadow-sm focus:border-lavender focus:outline-none focus:ring-2 focus:ring-lavender/30"
          />
        </div>
        <div className="overflow-hidden rounded-xl border border-cream-dark bg-white shadow-sm">
          <table className="w-full text-left">
            <thead>
              <tr className="border-b border-cream-dark bg-cream/50">
                <th className="px-4 py-3 text-xs font-semibold uppercase tracking-wider text-midnight-light">
                  Function Name
                </th>
                <th className="px-4 py-3 text-xs font-semibold uppercase tracking-wider text-midnight-light">
                  Active Version
                </th>
                <th className="px-4 py-3 text-xs font-semibold uppercase tracking-wider text-midnight-light">
                  Avg Latency (ms)
                </th>
                <th className="px-4 py-3 text-xs font-semibold uppercase tracking-wider text-midnight-light">
                  Cost (Last 24h)
                </th>
                <th className="w-24 px-4 py-3" />
              </tr>
            </thead>
            <tbody>
              {filtered.map((fn) => (
                <tr
                  key={fn.id}
                  className="border-b border-cream-dark/50 transition hover:bg-cream/30"
                >
                  <td className="px-4 py-3 font-medium text-midnight">
                    {fn.name}
                  </td>
                  <td className="px-4 py-3 text-sm text-[var(--text-light)]">
                    {fn.activeVersion}
                  </td>
                  <td className="px-4 py-3 text-sm tabular-nums text-[var(--text-light)]">
                    {fn.avgLatencyMs} ms
                  </td>
                  <td className="px-4 py-3 text-sm tabular-nums text-[var(--text-light)]">
                    ${fn.costLast24h.toFixed(2)}
                  </td>
                  <td className="px-4 py-3">
                    <Link
                      href={`/editor?functionId=${fn.id}`}
                      className="text-sm font-medium text-lavender-dark hover:underline"
                    >
                      Edit
                    </Link>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {filtered.length === 0 && (
            <div className="py-12 text-center text-[var(--text-light)]">
              No functions match &quot;{search}&quot;
            </div>
          )}
        </div>
      </div>
    </DashboardLayout>
  );
}
