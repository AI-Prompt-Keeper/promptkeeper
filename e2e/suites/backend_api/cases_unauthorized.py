"""
TEST_CASES.md §3–5 (partial), §7 (partial) — Non-authorized API access (backend).

3.1 / 4.1 / 5.1 style: unauthenticated requests to protected routes must fail.
"""

from __future__ import annotations

from e2e.suites.backend_api.base import BackendSuiteBase, BackendSuiteCtx


class BackendUnauthorizedCases(BackendSuiteBase):
    def __init__(self, ctx: BackendSuiteCtx):
        super().__init__(ctx)

    def case_3_1_put_key_without_auth_returns_401(self) -> None:
        status, body = self.http.post_json(
            "/v1/keys",
            {"raw_secret": "sk-test", "provider": "openai", "surface": "cli"},
        )
        if status != 401:
            raise AssertionError(f"Expected 401 without auth for POST /v1/keys, got {status} {body}")
        print("OK: 3.1 backend POST /v1/keys without auth → 401")

    def case_7_1_put_prompt_without_auth_returns_401(self) -> None:
        status, body = self.http.post_json(
            "/v1/prompts",
            {"name": "e2e_fn", "raw_secret": "Hello {{name}}", "surface": "cli"},
        )
        if status != 401:
            raise AssertionError(f"Expected 401 without auth for POST /v1/prompts, got {status} {body}")
        print("OK: 7.1 (partial) backend POST /v1/prompts without auth → 401")
