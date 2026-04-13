"""
TEST_CASES.md §1 — Registration (backend).

1.1 Backend: registration creates user and returns management key + default workspace.
"""

from __future__ import annotations

from e2e.common.key_utils import validate_scoped_key_checksum
from e2e.suites.backend_api.base import BackendSuiteBase, BackendSuiteCtx


class BackendRegistrationCases(BackendSuiteBase):
    def __init__(self, ctx: BackendSuiteCtx):
        super().__init__(ctx)

    def case_1_1_registration_returns_credentials_and_default_workspace(self) -> None:
        if self.shared is None:
            raise AssertionError("BackendSharedAuth required (rate limit: one register per run)")
        reg = self.shared.ensure_registered()

        api_key = reg.get("api_key")
        ws_id = reg.get("default_workspace_id")
        api_scope = reg.get("api_key_scope")

        if not isinstance(api_key, str) or not api_key:
            raise AssertionError(f"Missing api_key: {reg}")
        if not isinstance(ws_id, str) or not ws_id:
            raise AssertionError(f"Missing default_workspace_id: {reg}")
        if api_scope != "mgt":
            raise AssertionError(f"Expected api_key_scope=mgt, got {api_scope}")

        validate_scoped_key_checksum(api_key, "pk_mgt_live_")

        status, ws = self.http.get_json(
            f"/v1/workspaces/{ws_id}",
            headers={"Authorization": f"Bearer {api_key}"},
        )
        if status != 200:
            raise AssertionError(f"GET workspace failed: {status} {ws}")

        api_tokens = ws.get("api_tokens")
        ok = any(t.get("scope") == "mgt" and t.get("label") == "Default" for t in (api_tokens or []))
        if not ok:
            raise AssertionError(f"Default mgt token not found: {api_tokens}")

        print("OK: 1.1 backend registration + default workspace metadata")
