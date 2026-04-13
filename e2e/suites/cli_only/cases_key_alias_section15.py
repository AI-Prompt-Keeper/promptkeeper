"""
TEST_CASES.md §15 — Key alias / `prke use` (documentation smoke).
"""

from __future__ import annotations

from e2e.suites.cli_only.base import CliSuiteBase, CliSuiteCtx


class CliKeyAliasSection15Note(CliSuiteBase):
    def __init__(self, ctx: CliSuiteCtx):
        super().__init__(ctx)

    def case_15_section_smoke_documented(self) -> None:
        if self.shared is None:
            raise AssertionError("CliSharedSession required")
        self.shared.ensure_registered()
        print(
            "DOC: §15 key alias / `prke use` / `--key` — see cli/cmd/key.go; extend with vault mocks if needed."
        )
