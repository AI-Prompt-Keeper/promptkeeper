"""
Mint execution key + list prompts (CLI smoke; complements TEST_CASES §2 / §16).
"""

from __future__ import annotations

from e2e.common.key_utils import validate_scoped_key_checksum
from e2e.suites.cli_only.base import CliSuiteBase, CliSuiteCtx


class CliMintAndListCases(CliSuiteBase):
    def __init__(self, ctx: CliSuiteCtx):
        super().__init__(ctx)

    def case_cli_mint_execution_key_and_list_prompts(self) -> None:
        if self.shared is None:
            raise AssertionError("CliSharedSession required")
        self.shared.ensure_registered()

        mint = self.cli.run_json(["mint", "key", "e2e-exe"], expect_json=True)
        if mint.returncode != 0:
            raise AssertionError(mint.stderr)
        if not mint.json:
            raise AssertionError(mint.stdout)
        exe_key = mint.json.get("api_key")
        if not exe_key or mint.json.get("scope") != "exe":
            raise AssertionError(mint.json)
        validate_scoped_key_checksum(exe_key, "pk_exe_live_")

        lp = self.cli.run_json(["list", "prompts"], expect_json=True)
        if lp.returncode != 0:
            raise AssertionError(lp.stderr)
        if not isinstance((lp.json or {}).get("titles"), list):
            raise AssertionError(lp.json)

        print("OK: CLI mint execution key + list prompts")
