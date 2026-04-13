"""
TEST_CASES.md §3–5 (partial) — Store provider key without auth (CLI).

3.1 / 4.1 / 5.1 style: non-authorized user gets an error.
"""

from __future__ import annotations

from e2e.common.cli_runner import CliRunner
from e2e.suites.cli_only.base import CliSuiteBase, CliSuiteCtx


class CliStoreUnauthorizedCases(CliSuiteBase):
    def __init__(self, ctx: CliSuiteCtx):
        super().__init__(ctx)

    def case_3_1_cli_store_key_without_register_fails(self) -> None:
        cli = CliRunner(self.ctx.cli_bin, self.ctx.base_url)
        try:
            res = cli.run_raw(["store", "key", "openai", "sk-test-dummy"])
            if res.returncode == 0:
                raise AssertionError("store key should fail without auth")
        finally:
            cli.close()
        print("OK: 3.1 CLI store key without auth fails")
