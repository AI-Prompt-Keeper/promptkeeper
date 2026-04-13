"""
TEST_CASES.md §2 — Login (backend).

2.1.1 Valid login → token + management key for default workspace.
2.1.2 Invalid password → 401.
"""

from __future__ import annotations

from e2e.common.backend_auth import login_user
from e2e.common.key_utils import validate_scoped_key_checksum
from e2e.suites.backend_api.base import BackendSuiteBase, BackendSuiteCtx


class BackendLoginCases(BackendSuiteBase):
    def __init__(self, ctx: BackendSuiteCtx):
        super().__init__(ctx)

    def case_2_1_1_login_valid_returns_token_and_mgt_key(self) -> None:
        if self.shared is None:
            raise AssertionError("BackendSharedAuth required")
        reg = self.shared.ensure_registered()
        body = login_user(self.http, self.shared.email, self.shared.password)

        token = body.get("token")
        api_key = body.get("api_key")
        ws_id = body.get("default_workspace_id")
        scope = body.get("api_key_scope")

        if not isinstance(token, str) or len(token) < 32:
            raise AssertionError(f"Unexpected token: {body}")
        if not isinstance(api_key, str) or not api_key:
            raise AssertionError(f"Missing api_key: {body}")
        if str(ws_id) != str(reg.get("default_workspace_id")):
            raise AssertionError("default_workspace_id mismatch vs register")
        if scope != "mgt":
            raise AssertionError(f"Expected api_key_scope=mgt, got {scope}")

        validate_scoped_key_checksum(api_key, "pk_mgt_live_")
        print("OK: 2.1.1 backend login valid")

    def case_2_1_2_login_invalid_returns_error(self) -> None:
        if self.shared is None:
            raise AssertionError("BackendSharedAuth required")
        self.shared.ensure_registered()
        status, body = self.http.post_json(
            "/v1/auth/login",
            {"email": self.shared.email, "password": "wrongPasswordWrong!!", "surface": "cli"},
        )
        if status != 401:
            raise AssertionError(f"Expected 401 for bad password, got {status} {body}")
        print("OK: 2.1.2 backend login invalid")
