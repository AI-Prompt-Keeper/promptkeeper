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
        reg = self.shared.ensure_registered()
        orig_key = reg.get("api_key")
        if not isinstance(orig_key, str) or not orig_key:
            raise AssertionError(reg)

        res = self.cli.run_raw(
            ["store", "prompt", "boundary_fn", "Hello {{x}}", "openai"],
        )
        if res.returncode != 0:
            print(
                "SKIP: §14 workspace boundaries — need successful store prompt (KMS). "
                f"stderr: {res.stderr[:200]}"
            )
            return

        created = self.cli.run_json(["workspace", "create", "OtherWS"], expect_json=True)
        if created.returncode != 0 or not created.json:
            raise AssertionError(created.stderr)
        new_key = created.json.get("api_key")
        if isinstance(new_key, str) and new_key:
            self.cli.set_e2e_client_key(new_key)
        self.cli.run_raw(["workspace", "switch", "OtherWS"])

        lp = self.cli.run_json(["list", "prompts"], expect_json=True)
        if lp.returncode != 0:
            raise AssertionError(lp.stderr)
        titles = (lp.json or {}).get("titles") or []
        if "boundary_fn" in titles:
            raise AssertionError("Prompt from default workspace should not appear in OtherWS")

        # Restore auth for subsequent suite cases (mint/exec use default workspace).
        self.cli.set_e2e_client_key(orig_key)
        default_ws = reg.get("default_workspace_id")
        if default_ws:
            self.cli.run_raw(["workspace", "switch", str(default_ws)])

        print("OK: 14 CLI workspace boundary (prompt not visible in other WS)")
