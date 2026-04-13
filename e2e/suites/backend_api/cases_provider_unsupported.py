"""
TEST_CASES.md §6 — Unsupported provider (backend).

6.1 POST /v1/keys with an unsupported provider must error (no KMS required for validation path).
"""

from __future__ import annotations

from e2e.suites.backend_api.base import BackendSuiteBase, BackendSuiteCtx


class BackendUnsupportedProviderCases(BackendSuiteBase):
    def __init__(self, ctx: BackendSuiteCtx):
        super().__init__(ctx)

    def case_6_1_store_key_unsupported_provider_returns_error(self) -> None:
        if self.shared is None:
            raise AssertionError("BackendSharedAuth required")
        reg = self.shared.ensure_registered()
        api_key = reg["api_key"]

        status, body = self.http.post_json(
            "/v1/keys",
            {
                "raw_secret": "dummy-secret",
                "provider": "zzz_e2e_unsupported_provider",
                "surface": "cli",
            },
            headers={"Authorization": f"Bearer {api_key}"},
        )
        if status in (200, 201):
            raise AssertionError(f"Expected failure for unsupported provider, got {status} {body}")
        err = str((body or {}).get("error", "")).lower()
        if "not supported" not in err and "unsupported" not in err:
            raise AssertionError(f"Expected 'not supported' style error, got: {body}")
        print("OK: 6.1 backend unsupported provider for POST /v1/keys")
