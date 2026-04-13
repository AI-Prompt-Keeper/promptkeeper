"""
TEST_CASES.md §10, §12–13 (backend), §16.2 — Workspaces API.

10.1 Create workspace returns management key.
12 Rename non-default workspace.
13 Delete workspace (not signup default).
16.2 List workspaces returns ids and names.
"""

from __future__ import annotations

from e2e.common.key_utils import validate_scoped_key_checksum
from e2e.suites.backend_api.base import BackendSuiteBase, BackendSuiteCtx


class BackendWorkspaceCases(BackendSuiteBase):
    def __init__(self, ctx: BackendSuiteCtx):
        super().__init__(ctx)

    def case_10_1_and_16_2_create_list_workspaces(self) -> None:
        if self.shared is None:
            raise AssertionError("BackendSharedAuth required")
        reg = self.shared.ensure_registered()
        mgt = reg["api_key"]

        status, created = self.http.post_json(
            "/v1/workspaces",
            {"name": "E2E Secondary", "surface": "cli"},
            headers={"Authorization": f"Bearer {mgt}"},
        )
        if status != 201:
            raise AssertionError(f"POST /v1/workspaces failed: {status} {created}")

        new_key = created.get("api_key")
        if not new_key:
            raise AssertionError(f"Missing api_key in create workspace response: {created}")
        validate_scoped_key_checksum(new_key, "pk_mgt_live_")
        if created.get("api_key_scope") != "mgt":
            raise AssertionError(created)

        status, listed = self.http.get_json("/v1/workspaces", headers={"Authorization": f"Bearer {mgt}"})
        if status != 200:
            raise AssertionError(f"GET /v1/workspaces failed: {status} {listed}")

        workspaces = listed.get("workspaces")
        if not isinstance(workspaces, list) or len(workspaces) < 2:
            raise AssertionError(f"Expected at least 2 workspaces: {listed}")

        names = {w.get("name") for w in workspaces if isinstance(w, dict)}
        if "Personal" not in names or "E2E Secondary" not in names:
            raise AssertionError(f"Expected Personal + E2E Secondary in {names}")

        print("OK: 10.1 + 16.2 backend create workspace + list")

    def case_12_edit_non_default_workspace(self) -> None:
        if self.shared is None:
            raise AssertionError("BackendSharedAuth required")
        reg = self.shared.ensure_registered()
        mgt = reg["api_key"]

        status, created = self.http.post_json(
            "/v1/workspaces",
            {"name": "Rename Me", "surface": "cli"},
            headers={"Authorization": f"Bearer {mgt}"},
        )
        if status != 201:
            raise AssertionError(f"create workspace failed: {status} {created}")
        ws_id = created["id"]

        status, updated = self.http.patch_json(
            f"/v1/workspaces/{ws_id}",
            {"name": "Renamed WS"},
            headers={"Authorization": f"Bearer {mgt}"},
        )
        if status != 200:
            raise AssertionError(f"PATCH workspace failed: {status} {updated}")
        if updated.get("name") != "Renamed WS":
            raise AssertionError(updated)

        status, listed = self.http.get_json("/v1/workspaces", headers={"Authorization": f"Bearer {mgt}"})
        names = {w.get("name") for w in (listed.get("workspaces") or []) if isinstance(w, dict)}
        if "Renamed WS" not in names:
            raise AssertionError(f"Renamed name not in list: {names}")

        print("OK: 12 backend rename workspace")

    def case_13_delete_workspace_not_in_list(self) -> None:
        if self.shared is None:
            raise AssertionError("BackendSharedAuth required")
        reg = self.shared.ensure_registered()
        mgt = reg["api_key"]

        status, created = self.http.post_json(
            "/v1/workspaces",
            {"name": "To Delete", "surface": "cli"},
            headers={"Authorization": f"Bearer {mgt}"},
        )
        if status != 201:
            raise AssertionError(created)
        ws_id = created["id"]

        status, _ = self.http.delete(
            f"/v1/workspaces/{ws_id}",
            headers={"Authorization": f"Bearer {mgt}"},
        )
        if status not in (200, 204):
            raise AssertionError(f"DELETE workspace expected 200/204, got {status}")

        status, listed = self.http.get_json("/v1/workspaces", headers={"Authorization": f"Bearer {mgt}"})
        ids = {str(w.get("id")) for w in (listed.get("workspaces") or []) if isinstance(w, dict)}
        if str(ws_id) in ids:
            raise AssertionError("Deleted workspace still listed")

        status, err = self.http.get_json(
            f"/v1/workspaces/{ws_id}",
            headers={"Authorization": f"Bearer {mgt}"},
        )
        if status != 404:
            raise AssertionError(f"Expected 404 for deleted workspace, got {status} {err}")

        print("OK: 13 backend delete workspace")
