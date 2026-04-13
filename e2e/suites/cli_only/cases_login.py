"""
TEST_CASES.md §2 — Login (CLI).

2.2.1 Valid login (separate HOME) with credentials from shared registered user.
2.2.2 Invalid login fails; list prompts fails without auth in fresh HOME.
"""

from __future__ import annotations

from e2e.common.cli_runner import CliRunner
from e2e.common.key_utils import validate_scoped_key_checksum
from e2e.suites.cli_only.base import CliSuiteBase, CliSuiteCtx


class CliLoginCases(CliSuiteBase):
    def __init__(self, ctx: CliSuiteCtx):
        super().__init__(ctx)

    def case_2_2_1_login_valid(self) -> None:
        if self.shared is None:
            raise AssertionError("CliSharedSession required")
        self.shared.ensure_registered()

        cli2 = CliRunner(self.ctx.cli_bin, self.ctx.base_url)
        try:
            r2 = cli2.run_json(["login", self.shared.email, self.shared.password], expect_json=True)
            if r2.returncode != 0:
                raise AssertionError(f"login failed: {r2.stderr}")
            if not r2.json:
                raise AssertionError(r2.stdout)
            body = r2.json
            if not body.get("token"):
                raise AssertionError(body)
            if body.get("api_key"):
                validate_scoped_key_checksum(body["api_key"], "pk_mgt_live_")
        finally:
            cli2.close()

        print("OK: 2.2.1 CLI login valid")

    def case_2_2_2_login_invalid_stays_unauthorized(self) -> None:
        if self.shared is None:
            raise AssertionError("CliSharedSession required")
        self.shared.ensure_registered()

        cli2 = CliRunner(self.ctx.cli_bin, self.ctx.base_url)
        try:
            r2 = cli2.run_json(["login", self.shared.email, "wrongPasswordWrong!!"], expect_json=True)
            if r2.returncode == 0:
                raise AssertionError("login should fail for bad password")
            r3 = cli2.run_json(["list", "prompts"], expect_json=True)
            if r3.returncode == 0:
                raise AssertionError("list prompts should fail without auth after failed login")
        finally:
            cli2.close()

        print("OK: 2.2.2 CLI login invalid")
