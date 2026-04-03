"""
Shared backend auth for one E2E run: single registration to satisfy per-IP rate limits
(see backend register_handler rate limiter).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, Optional

from e2e.common.backend_auth import random_email, register_user
from e2e.common.http_client import HttpClient


@dataclass
class BackendSharedAuth:
    host_url: str
    password: str = "securePassword123"
    http: HttpClient = field(init=False)
    _email: Optional[str] = None
    _register_payload: Optional[Dict[str, Any]] = None

    def __post_init__(self) -> None:
        self.http = HttpClient(self.host_url, timeout_s=120)

    def ensure_registered(self) -> Dict[str, Any]:
        """First call registers; later calls return the same registration payload."""
        if self._register_payload is not None:
            return self._register_payload
        self._email = random_email("e2e-shared")
        self._register_payload = register_user(self.http, self._email, self.password)
        return self._register_payload

    @property
    def email(self) -> str:
        self.ensure_registered()
        assert self._email is not None
        return self._email
