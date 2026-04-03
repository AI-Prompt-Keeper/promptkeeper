"""
TEST_CASES.md §7 — Store prompt (backend).

7.1 Success when KMS is configured; otherwise backend returns 503 — skip is not a failure.
"""

from __future__ import annotations

from e2e.suites.backend_api.base import BackendSuiteBase, BackendSuiteCtx


class BackendPromptStoreCases(BackendSuiteBase):
    def __init__(self, ctx: BackendSuiteCtx):
        super().__init__(ctx)

    def case_7_1_put_prompt_authenticated_success_or_kms_unavailable(self) -> None:
        if self.shared is None:
            raise AssertionError("BackendSharedAuth required")
        reg = self.shared.ensure_registered()
        api_key = reg["api_key"]

        status, body = self.http.post_json(
            "/v1/prompts",
            {
                "name": "e2e_prompt",
                "raw_secret": "Hello {{name}}",
                "provider": "openai",
                "surface": "cli",
            },
            headers={"Authorization": f"Bearer {api_key}"},
        )
        if status in (200, 201):
            print("OK: 7.1 backend POST /v1/prompts success (KMS available)")
            return
        if status == 503:
            err = str((body or {}).get("error", ""))
            print(f"SKIP: 7.1 backend prompt store — KMS unavailable ({err[:120]})")
            return
        raise AssertionError(f"Unexpected status for POST /v1/prompts: {status} {body}")
