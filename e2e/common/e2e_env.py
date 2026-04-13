"""
Optional secrets for provider-key / execute tests (see temp_mds/TEST_CASES.md §3–10).
Unset = skip tests that need real provider credentials or KMS.
"""

from __future__ import annotations

import os
from typing import Optional


def openai_key() -> Optional[str]:
    return os.environ.get("E2E_OPENAI_API_KEY") or os.environ.get("OPENAI_API_KEY")


def anthropic_key() -> Optional[str]:
    return os.environ.get("E2E_ANTHROPIC_API_KEY") or os.environ.get("ANTHROPIC_API_KEY")


def gemini_key() -> Optional[str]:
    return os.environ.get("E2E_GEMINI_API_KEY") or os.environ.get("GEMINI_API_KEY")


def run_exec_tests() -> bool:
    return os.environ.get("E2E_RUN_EXEC", "").lower() in ("1", "true", "yes")
