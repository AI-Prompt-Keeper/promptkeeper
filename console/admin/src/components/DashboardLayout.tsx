"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const nav = [
  { href: "/", label: "Functions" },
  { href: "/editor", label: "Prompt Editor" },
  { href: "/failover", label: "Failover" },
];

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const pathname = usePathname();
  return (
    <div className="flex min-h-screen">
      <aside className="w-56 shrink-0 border-r border-cream-dark bg-midnight text-white">
        <div className="sticky top-0 flex flex-col gap-1 p-4">
          <h2 className="mb-4 px-2 text-lg font-semibold text-lavender-light">
            Mission Control
          </h2>
          {nav.map(({ href, label }) => (
            <Link
              key={href}
              href={href}
              className={`rounded-lg px-3 py-2 text-sm transition ${
                pathname === href
                  ? "bg-lavender/30 text-white"
                  : "text-white/80 hover:bg-midnight-light hover:text-white"
              }`}
            >
              {label}
            </Link>
          ))}
        </div>
      </aside>
      <main className="flex-1 overflow-auto p-6">{children}</main>
    </div>
  );
}
