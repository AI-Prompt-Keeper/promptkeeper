"""
TEST_CASES.md §1.2.2 — Interactive register (guided flow).

Automated E2E cannot drive TTY prompts; this case documents the gap.
"""

from __future__ import annotations

from e2e.suites.cli_only.base import CliSuiteBase, CliSuiteCtx


class CliRegistrationInteractiveNote(CliSuiteBase):
    def __init__(self, ctx: CliSuiteCtx):
        super().__init__(ctx)

    def case_1_2_2_interactive_register_documented_manual_only(self) -> None:
        print(
            "DOC: 1.2.2 interactive register requires a TTY — run manually: "
            "prke --debug --use-local-config register"
        )
