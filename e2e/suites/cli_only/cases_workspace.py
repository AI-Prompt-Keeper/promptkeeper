"""
TEST_CASES.md §10–13, §16 — Workspaces (CLI).

Uses shared registered user + one CLI HOME (rate limit).
"""

from __future__ import annotations

from e2e.common.cli_runner import CliRunner
from e2e.common.key_utils import validate_scoped_key_checksum
from e2e.suites.cli_only.base import CliSuiteBase, CliSuiteCtx


class CliWorkspaceCases(CliSuiteBase):
    def __init__(self, ctx: CliSuiteCtx):
        super().__init__(ctx)

    def _ensure_user(self) -> None:
        if self.shared is None:
            raise AssertionError("CliSharedSession required")
        self.shared.ensure_registered()

    def case_16_1_workspace_list_without_auth_fails(self) -> None:
        cli = CliRunner(self.ctx.cli_bin, self.ctx.base_url)
        try:
            res = cli.run_raw(["workspace", "list", "--json"])
            if res.returncode == 0:
                raise AssertionError("workspace list should fail without auth")
        finally:
            cli.close()
        print("OK: 16.1 CLI workspace list without auth fails")

    def case_10_2_and_16_3_create_workspace_and_list(self) -> None:
        self._ensure_user()

        created = self.cli.run_json(["workspace", "create", "E2E_WS"], expect_json=True)
        if created.returncode != 0:
            raise AssertionError(created.stderr)
        if not created.json:
            raise AssertionError(created.stdout)
        key = created.json.get("api_key")
        if not key:
            raise AssertionError(created.json)
        validate_scoped_key_checksum(key, "pk_mgt_live_")

        listed = self.cli.run_raw(["workspace", "list"])
        if listed.returncode != 0:
            raise AssertionError(listed.stderr)
        if "Personal" not in listed.stdout and "E2E_WS" not in listed.stdout:
            raise AssertionError(f"unexpected list output: {listed.stdout}")

        lj = self.cli.run_json(["workspace", "list", "--json"], expect_json=True)
        if lj.returncode != 0:
            raise AssertionError(lj.stderr)
        wss = (lj.json or {}).get("workspaces")
        if not isinstance(wss, list) or len(wss) < 2:
            raise AssertionError(lj.json)

        print("OK: 10.2 + 16.3 CLI workspace create + list")

    def case_11_2_1_switch_nonexistent_workspace_errors(self) -> None:
        self._ensure_user()
        bad = self.cli.run_raw(["workspace", "switch", "no_such_workspace_zzzzz"])
        if bad.returncode == 0:
            raise AssertionError("switch should fail for missing workspace")
        print("OK: 11.2.1 CLI switch missing workspace errors")

    def case_12_cli_edit_workspace_name(self) -> None:
        self._ensure_user()
        c = self.cli.run_json(["workspace", "create", "EditMe"], expect_json=True)
        if c.returncode != 0:
            raise AssertionError(c.stderr)

        r = self.cli.run_raw(["workspace", "edit", "EditMe", "EditedName"])
        if r.returncode != 0:
            raise AssertionError(r.stderr)

        lj = self.cli.run_json(["workspace", "list", "--json"], expect_json=True)
        names = [w.get("name") for w in (lj.json or {}).get("workspaces") or []]
        if "EditedName" not in names:
            raise AssertionError(names)
        print("OK: 12 CLI workspace edit")

    def case_13_cli_delete_workspace(self) -> None:
        self._ensure_user()
        self.cli.run_json(["workspace", "create", "DelMe"], expect_json=True)

        d = self.cli.run_raw(["workspace", "delete", "DelMe"])
        if d.returncode != 0:
            raise AssertionError(d.stderr)

        lj = self.cli.run_json(["workspace", "list", "--json"], expect_json=True)
        names = [w.get("name") for w in (lj.json or {}).get("workspaces") or []]
        if "DelMe" in names:
            raise AssertionError("workspace still listed after delete")

        sw = self.cli.run_raw(["workspace", "switch", "DelMe"])
        if sw.returncode == 0:
            raise AssertionError("switch to deleted workspace should fail")
        print("OK: 13 CLI workspace delete")
