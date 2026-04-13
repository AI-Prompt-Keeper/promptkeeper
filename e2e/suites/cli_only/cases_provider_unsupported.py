"""
TEST_CASES.md §6 — Unsupported provider (CLI).

6.2 store key with unsupported provider → error.
"""

from __future__ import annotations

from e2e.suites.cli_only.base import CliSuiteBase, CliSuiteCtx


class CliUnsupportedProviderCases(CliSuiteBase):
    def __init__(self, ctx: CliSuiteCtx):
        super().__init__(ctx)

    def case_6_2_cli_store_key_unsupported_provider(self) -> None:
        if self.shared is None:
            raise AssertionError("CliSharedSession required")
        self.shared.ensure_registered()

        res = self.cli.run_raw(["store", "key", "zzz_unknown_provider_x", "dummy-secret-value"])
        if res.returncode == 0:
            raise AssertionError("expected non-zero exit for unsupported provider")
        combined = (res.stderr + res.stdout).lower()
        if "not supported" not in combined and "unsupported" not in combined:
            raise AssertionError(f"expected unsupported message, got: {res.stderr}")
        print("OK: 6.2 CLI unsupported provider")
