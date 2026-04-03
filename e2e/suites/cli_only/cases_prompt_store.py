"""
TEST_CASES.md §7 — Store prompt (CLI).

7.2 Success when KMS configured; otherwise skip-style message.
"""

from __future__ import annotations

from e2e.suites.cli_only.base import CliSuiteBase, CliSuiteCtx


class CliPromptStoreCases(CliSuiteBase):
    def __init__(self, ctx: CliSuiteCtx):
        super().__init__(ctx)

    def case_7_2_cli_store_prompt_success_or_kms_unavailable(self) -> None:
        if self.shared is None:
            raise AssertionError("CliSharedSession required")
        self.shared.ensure_registered()

        res = self.cli.run_raw(
            ["store", "prompt", "e2e_hello", "Hello {{name}}", "openai"],
        )
        if res.returncode == 0:
            print("OK: 7.2 CLI store prompt success")
            return
        err = (res.stderr + res.stdout).lower()
        if "503" in err or "unavailable" in err or "kms" in err:
            print("SKIP: 7.2 CLI store prompt — KMS unavailable or backend 503")
            return
        raise AssertionError(f"Unexpected failure: {res.stderr}\n{res.stdout}")
