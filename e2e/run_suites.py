"""
E2E runner: starts Docker backend once, runs backend_api and/or cli_only case modules.

Usage:
  python3 e2e/run_suites.py --suite all
  python3 e2e/run_suites.py --suite backend_api --cases case_2_1_2_login_invalid_returns_error
"""

from __future__ import annotations

import argparse
import os
import random
import string
import subprocess
import sys
from typing import Callable, Dict, List, Optional, Sequence, Tuple, Type


def _project_root() -> str:
    return os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))


def _random_suffix(n: int = 8) -> str:
    alphabet = string.ascii_lowercase + string.digits
    return "".join(random.choice(alphabet) for _ in range(n))


def _go_build_env() -> dict:
    """Use a real module cache when Cursor/sandbox points Go at a broken truncated cache."""
    env = os.environ.copy()
    gmc = env.get("GOMODCACHE", "")
    if "cursor-sandbox" in gmc or "sandbox-cache" in gmc:
        home = os.path.expanduser("~")
        env["GOMODCACHE"] = os.path.join(home, "go", "pkg", "mod")
        if sys.platform == "darwin":
            env["GOCACHE"] = os.path.join(home, "Library", "Caches", "go-build")
        else:
            env["GOCACHE"] = os.path.join(home, ".cache", "go-build")
    return env


def _build_cli(cli_dir: str, out_path: str) -> None:
    subprocess.check_call(
        ["go", "build", "-o", out_path, "."],
        cwd=cli_dir,
        env=_go_build_env(),
    )


def _case_methods(suite_obj) -> Dict[str, Callable[[], None]]:
    cases: Dict[str, Callable[[], None]] = {}
    for name in dir(suite_obj):
        if name.startswith("case_"):
            fn = getattr(suite_obj, name)
            if callable(fn):
                cases[name] = fn
    return dict(sorted(cases.items(), key=lambda kv: kv[0]))


def _run_suite_group(
    label: str,
    suite_classes: Sequence[Type],
    ctx_factory,
    selected_cases: Optional[List[str]],
) -> int:
    """Returns number of case methods executed."""
    ran = 0
    for cls in suite_classes:
        suite = ctx_factory(cls)
        try:
            cases = _case_methods(suite)
            if selected_cases is None:
                run_names = list(cases.keys())
            else:
                run_names = [n for n in selected_cases if n in cases]
            for name in run_names:
                print(f"[{label}::{cls.__name__}] {name} ...")
                cases[name]()
                ran += 1
        finally:
            close = getattr(suite, "close", None)
            if callable(close):
                close()
    return ran


def main() -> None:
    root = _project_root()
    if root not in sys.path:
        sys.path.insert(0, root)

    from e2e.common.backend_session import BackendSharedAuth
    from e2e.common.backend_stack import BackendDockerStack
    from e2e.common.cli_session import CliSharedSession
    from e2e.suites.backend_api.base import BackendSuiteCtx
    from e2e.suites.backend_api.cases_login import BackendLoginCases
    from e2e.suites.backend_api.cases_prompt_store import BackendPromptStoreCases
    from e2e.suites.backend_api.cases_provider_unsupported import BackendUnsupportedProviderCases
    from e2e.suites.backend_api.cases_registration import BackendRegistrationCases
    from e2e.suites.backend_api.cases_unauthorized import BackendUnauthorizedCases
    from e2e.suites.backend_api.cases_workspaces import BackendWorkspaceCases

    from e2e.suites.cli_only.base import CliSuiteCtx
    from e2e.suites.cli_only.cases_exec_optional import CliExecOptionalCases
    from e2e.suites.cli_only.cases_key_alias_section15 import CliKeyAliasSection15Note
    from e2e.suites.cli_only.cases_login import CliLoginCases
    from e2e.suites.cli_only.cases_mint_and_list import CliMintAndListCases
    from e2e.suites.cli_only.cases_prompt_store import CliPromptStoreCases
    from e2e.suites.cli_only.cases_provider_unsupported import CliUnsupportedProviderCases
    from e2e.suites.cli_only.cases_registration import CliRegistrationCases
    from e2e.suites.cli_only.cases_registration_interactive import CliRegistrationInteractiveNote
    from e2e.suites.cli_only.cases_store_unauthorized import CliStoreUnauthorizedCases
    from e2e.suites.cli_only.cases_workspace import CliWorkspaceCases
    from e2e.suites.cli_only.cases_workspace_boundaries import CliWorkspaceBoundaryCases

    ap = argparse.ArgumentParser(description="PromptKeeper E2E regression suites")
    ap.add_argument(
        "--suite",
        choices=["backend_api", "cli_only", "all"],
        default="all",
    )
    ap.add_argument("--host-url", default="http://127.0.0.1:3000")
    ap.add_argument(
        "--compose-files",
        default="docker-compose.yaml,e2e/docker-compose.e2e.yaml",
        help="Comma-separated compose files (include e2e overlay so /service exists in backend container)",
    )
    ap.add_argument(
        "--reset-db",
        default=True,
        action=argparse.BooleanOptionalAction,
    )
    ap.add_argument("--cases", default="", help="Comma-separated case method names (optional filter)")
    args = ap.parse_args()

    host_url = args.host_url.rstrip("/")
    compose_files = tuple(
        os.path.join(root, p.strip()) if not os.path.isabs(p.strip()) else p.strip()
        for p in args.compose_files.split(",")
        if p.strip()
    )

    selected_cases: Optional[List[str]] = None
    if args.cases.strip():
        selected_cases = [c.strip() for c in args.cases.split(",") if c.strip()]

    backend_classes: Tuple[Type, ...] = (
        BackendRegistrationCases,
        BackendLoginCases,
        BackendUnauthorizedCases,
        BackendUnsupportedProviderCases,
        BackendPromptStoreCases,
        BackendWorkspaceCases,
    )

    cli_classes: Tuple[Type, ...] = (
        CliRegistrationCases,
        CliRegistrationInteractiveNote,
        CliLoginCases,
        CliStoreUnauthorizedCases,
        CliUnsupportedProviderCases,
        CliPromptStoreCases,
        CliWorkspaceCases,
        CliWorkspaceBoundaryCases,
        CliMintAndListCases,
        CliExecOptionalCases,
        CliKeyAliasSection15Note,
    )

    total_ran = 0

    with BackendDockerStack(
        compose_files=compose_files,
        host_url=host_url,
        reset_db=args.reset_db,
    ):
        shared_backend = BackendSharedAuth(host_url=host_url)

        if args.suite in ("backend_api", "all"):

            def _backend_ctx(cls: Type):
                return cls(BackendSuiteCtx(host_url=host_url, shared=shared_backend))

            total_ran += _run_suite_group("backend_api", backend_classes, _backend_ctx, selected_cases)

        if args.suite in ("cli_only", "all"):
            tmp_dir = os.path.join(root, "tmp")
            os.makedirs(tmp_dir, exist_ok=True)
            cli_bin = os.path.join(tmp_dir, f"prke-e2e-{_random_suffix()}")
            cli_dir = os.path.join(root, "cli")
            print(f"[cli_only] building CLI -> {cli_bin}")
            _build_cli(cli_dir=cli_dir, out_path=cli_bin)

            cli_shared = CliSharedSession(base_url=host_url, cli_bin=cli_bin)
            cli_ctx = CliSuiteCtx(base_url=host_url, cli_bin=cli_bin, shared=cli_shared)

            def _cli_ctx(cls: Type):
                return cls(cli_ctx)

            try:
                total_ran += _run_suite_group("cli_only", cli_classes, _cli_ctx, selected_cases)
            finally:
                cli_shared.close()

    if selected_cases is not None and total_ran == 0:
        raise SystemExit(f"No cases matched filter: {selected_cases}")

    print(f"E2E suites completed ({total_ran} case(s) executed).")


if __name__ == "__main__":
    main()
