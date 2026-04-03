import json
import os
import subprocess
import tempfile
from dataclasses import dataclass
from typing import Any, Dict, List, Optional


def _first_json_object(text: str) -> Dict[str, Any]:
    start = text.find("{")
    if start < 0:
        raise AssertionError("CLI output: cannot find JSON object start")
    dec = json.JSONDecoder()
    obj, _ = dec.raw_decode(text[start:])
    if not isinstance(obj, dict):
        raise AssertionError("CLI output: first JSON object is not a dict")
    return obj


@dataclass
class CliResult:
    returncode: int
    stdout: str
    stderr: str
    json: Optional[Dict[str, Any]] = None


class CliRunner:
    """
    Runs `prke`/`promptkeeper` with:
      --debug --use-local-config
    so config file `.prke-config.yaml` is read from HOME and base_url can be pointed to localhost.
    """

    def __init__(self, cli_bin: str, base_url: str):
        self.cli_bin = cli_bin
        self.base_url = base_url
        self._home_ctx: Optional[tempfile.TemporaryDirectory] = None
        self._home_path: Optional[str] = None

    def _ensure_home(self) -> str:
        if self._home_path:
            return self._home_path
        self._home_ctx = tempfile.TemporaryDirectory()
        self._home_path = self._home_ctx.name

        # Write ~/.prke-config.yaml used by the CLI when --debug && --use-local-config are set.
        cfg_path = os.path.join(self._home_path, ".prke-config.yaml")
        with open(cfg_path, "w", encoding="utf-8") as f:
            f.write(f'base_url: "{self.base_url}"\n')
        return self._home_path

    def run_json(self, args: List[str], expect_json: bool = True) -> CliResult:
        home = self._ensure_home()

        cmd = [self.cli_bin, "--debug", "--use-local-config", *args]
        env = dict(os.environ)
        env["HOME"] = home

        p = subprocess.run(
            cmd,
            env=env,
            cwd=os.getcwd(),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        stdout = p.stdout or ""
        stderr = p.stderr or ""

        parsed = None
        if expect_json and stdout.strip():
            parsed = _first_json_object(stdout)

        return CliResult(
            returncode=p.returncode,
            stdout=stdout,
            stderr=stderr,
            json=parsed,
        )

    def run_raw(self, args: List[str]) -> CliResult:
        """Run CLI without requiring JSON on stdout (e.g. workspace list text)."""
        home = self._ensure_home()
        cmd = [self.cli_bin, "--debug", "--use-local-config", *args]
        env = dict(os.environ)
        env["HOME"] = home
        p = subprocess.run(
            cmd,
            env=env,
            cwd=os.getcwd(),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return CliResult(
            returncode=p.returncode,
            stdout=p.stdout or "",
            stderr=p.stderr or "",
            json=None,
        )

    def close(self) -> None:
        if self._home_ctx:
            self._home_ctx.cleanup()
        self._home_ctx = None
        self._home_path = None

