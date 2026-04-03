"""
Register / login helpers (PoW + HTTP). Used by backend_api suites to avoid duplication.
"""

from __future__ import annotations

import uuid
from typing import Any, Dict, Tuple

from e2e.common.http_client import HttpClient
from e2e.common.pow import solve_pow


def pow_headers_from_challenge(challenge: Dict[str, Any]) -> Dict[str, str]:
    nonce = challenge["nonce"]
    difficulty = int(challenge["difficulty"])
    valid_until = challenge["valid_until"]
    solution = solve_pow(nonce, valid_until, difficulty)
    return {
        "X-Pow-Nonce": nonce,
        "X-Pow-Solution": solution,
        "X-Pow-Valid-Until": valid_until,
    }


def register_user(http: HttpClient, email: str, password: str, surface: str = "cli") -> Dict[str, Any]:
    status, ch = http.get_json("/v1/auth/register-challenge")
    if status != 200:
        raise AssertionError(f"GET register-challenge failed: {status} {ch}")
    headers = pow_headers_from_challenge(ch)
    status, reg = http.post_json(
        "/v1/auth/register",
        {"email": email, "password": password, "surface": surface},
        headers=headers,
    )
    if status != 201:
        raise AssertionError(f"POST /v1/auth/register failed: {status} {reg}")
    return reg


def random_email(prefix: str = "e2e") -> str:
    return f"{prefix}-{uuid.uuid4().hex}@example.com"


def login_user(http: HttpClient, email: str, password: str, surface: str = "cli") -> Dict[str, Any]:
    status, body = http.post_json(
        "/v1/auth/login",
        {"email": email, "password": password, "surface": surface},
    )
    if status != 200:
        raise AssertionError(f"POST /v1/auth/login failed: {status} {body}")
    return body


def register_new_user(http: HttpClient, password: str = "securePassword123") -> Tuple[str, str, Dict[str, Any]]:
    email = random_email("e2e")
    reg = register_user(http, email, password)
    return email, password, reg
