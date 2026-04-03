"""
TEST_CASES.md §14 — Workspace boundaries (CLI).
"""

from __future__ import annotations

from e2e.suites.cli_only.base import CliSuiteBase, CliSuiteCtx


class CliWorkspaceBoundaryCases(CliSuiteBase):
    def __init__(self, ctx: CliSuiteCtx):
        super().__init__(ctx)

    def case_14_prompt_isolation_between_workspaces(self) -> None:
        if self.shared is None:
            raise AssertionError("CliSharedSession required")
        self.shared.ensure_registered()

        res = self.cli.run_raw(
            ["store", "prompt", "boundary_fn", "Hello {{x}}", "openai"],
        )
        if res.returncode != 0:
            print(
                "SKIP: §14 workspace boundaries — need successful store prompt (KMS). "
                f"stderr: {res.stderr[:200]}"
            )
            return

        self.cli.run_json(["workspace", "create", "OtherWS"], expect_json=True)
        self.cli.run_raw(["workspace", "switch", "OtherWS"])

        lp = self.cli.run_json(["list", "prompts"], expect_json=True)
        if lp.returncode != 0:
            raise AssertionError(lp.stderr)
        titles = (lp.json or {}).get("titles") or []
        if "boundary_fn" in titles:
            raise AssertionError("Prompt from default workspace should not appear in OtherWS")

        print("OK: 14 CLI workspace boundary (prompt not visible in other WS)")
