"""
TEST_CASES.md §8–10 — Execute (optional; requires provider keys + E2E_RUN_EXEC).
"""

from __future__ import annotations

from e2e.common import e2e_env
from e2e.suites.cli_only.base import CliSuiteBase, CliSuiteCtx


class CliExecOptionalCases(CliSuiteBase):
    def __init__(self, ctx: CliSuiteCtx):
        super().__init__(ctx)

    def _need_exec(self) -> bool:
        if not e2e_env.run_exec_tests():
            print("SKIP: exec tests — set E2E_RUN_EXEC=1 and provider API keys")
            return False
        return True

    def case_8_exec_openai_stream_if_configured(self) -> None:
        if not self._need_exec():
            return
        key = e2e_env.openai_key()
        if not key:
            print("SKIP: §8 OpenAI exec — set E2E_OPENAI_API_KEY")
            return

        if self.shared is None:
            raise AssertionError("CliSharedSession required")
        self.shared.ensure_registered()

        self.cli.run_raw(["store", "key", "openai", key])
        self.cli.run_raw(["store", "prompt", "e2e_exec", "Say hi in one word", "openai"])

        res = self.cli.run_raw(["exec", "e2e_exec", "query=hello"])
        if res.returncode != 0:
            raise AssertionError(f"exec failed: {res.stderr}")
        if not (res.stdout or "").strip():
            raise AssertionError("expected streamed stdout from exec")
        print("OK: 8 CLI exec OpenAI (stream)")

    def case_9_exec_gemini_stream_if_configured(self) -> None:
        if not self._need_exec():
            return
        key = e2e_env.gemini_key()
        if not key:
            print("SKIP: §9 Gemini exec — set E2E_GEMINI_API_KEY")
            return

        if self.shared is None:
            raise AssertionError("CliSharedSession required")
        self.shared.ensure_registered()

        self.cli.run_raw(["store", "key", "gemini", key])
        self.cli.run_raw(["store", "prompt", "e2e_gem", "Say hi", "gemini"])

        res = self.cli.run_raw(["exec", "e2e_gem", "query=hello", "--provider", "gemini"])
        if res.returncode != 0:
            raise AssertionError(res.stderr)
        print("OK: 9 CLI exec Gemini (stream)")

    def case_10_exec_anthropic_stream_if_configured(self) -> None:
        if not self._need_exec():
            return
        key = e2e_env.anthropic_key()
        if not key:
            print("SKIP: §10 Anthropic exec — set E2E_ANTHROPIC_API_KEY")
            return

        if self.shared is None:
            raise AssertionError("CliSharedSession required")
        self.shared.ensure_registered()

        self.cli.run_raw(["store", "key", "anthropic", key])
        self.cli.run_raw(["store", "prompt", "e2e_ant", "Say hi", "anthropic"])

        res = self.cli.run_raw(["exec", "e2e_ant", "query=hello", "--provider", "anthropic"])
        if res.returncode != 0:
            raise AssertionError(res.stderr)
        print("OK: 10 CLI exec Anthropic (stream)")
