"""
TEST_CASES.md §1 — Registration (CLI).

1.2.1 With email and password: register returns management key (checksum-valid).
"""

from __future__ import annotations

from e2e.common.key_utils import validate_scoped_key_checksum
from e2e.suites.cli_only.base import CliSuiteBase, CliSuiteCtx


class CliRegistrationCases(CliSuiteBase):
    def __init__(self, ctx: CliSuiteCtx):
        super().__init__(ctx)

    def case_1_2_1_register_with_email_password(self) -> None:
        if self.shared is None:
            raise AssertionError("CliSharedSession required (register rate limit)")
        reg = self.shared.ensure_registered()

        api_key = reg.get("api_key")
        if not api_key:
            raise AssertionError(reg)
        validate_scoped_key_checksum(api_key, "pk_mgt_live_")
        if reg.get("api_key_scope") != "mgt":
            raise AssertionError(reg)
        print("OK: 1.2.1 CLI register with email/password")
