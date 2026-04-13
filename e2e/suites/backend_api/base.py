from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

from e2e.common.backend_session import BackendSharedAuth
from e2e.common.http_client import HttpClient


@dataclass
class BackendSuiteCtx:
    host_url: str
    """One registration per E2E run (rate limit)."""
    shared: Optional[BackendSharedAuth] = None


class BackendSuiteBase:
    """Shared HTTP client for backend-only suites."""

    def __init__(self, ctx: BackendSuiteCtx):
        self.ctx = ctx
        if ctx.shared is not None:
            self.http = ctx.shared.http
            self.shared = ctx.shared
        else:
            self.http = HttpClient(ctx.host_url, timeout_s=120)
            self.shared = None
