"""
Single CLI registration per E2E run (same rate limit as backend register per IP).
"""

from __future__ import annotations

import uuid
from dataclasses import dataclass, field
from typing import Any, Dict, Optional

from e2e.common.cli_runner import CliRunner


@dataclass
class CliSharedSession:
    base_url: str
    cli_bin: str
    password: str = "securePassword123"
    runner: CliRunner = field(init=False)
    _email: Optional[str] = None
    _reg: Optional[Dict[str, Any]] = None

    def __post_init__(self) -> None:
        self.runner = CliRunner(self.cli_bin, self.base_url)

    def ensure_registered(self) -> Dict[str, Any]:
        if self._reg is not None:
            return self._reg
        self._email = f"e2e-cli-shared-{uuid.uuid4().hex}@example.com"
        res = self.runner.run_json(["register", self._email, self.password], expect_json=True)
        if res.returncode != 0:
            raise AssertionError(f"CLI register failed: {res.stderr}")
        if not res.json:
            raise AssertionError(res.stdout)
        self._reg = res.json
        api_key = self._reg.get("api_key")
        if isinstance(api_key, str) and api_key:
            self.runner.set_e2e_client_key(api_key)
        return self._reg

    @property
    def email(self) -> str:
        self.ensure_registered()
        assert self._email is not None
        return self._email

    def close(self) -> None:
        self.runner.close()
