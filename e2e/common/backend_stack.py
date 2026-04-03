import os
import subprocess
import time
import urllib.request
from typing import Optional, Sequence, Tuple


class BackendDockerStack:
    """
    Starts the existing local backend+Postgres stack once for the whole suite run.

    Note: docker-compose.yaml uses fixed `container_name`, so parallel CI jobs may collide.
    """

    def __init__(
        self,
        compose_files: Sequence[str] = ("docker-compose.yaml", "e2e/docker-compose.e2e.yaml"),
        host_url: str = "http://127.0.0.1:3000",
        reset_db: bool = True,
        ready_timeout_s: int = 600,
        docker_build: bool = True,
    ):
        self.compose_files: Tuple[str, ...] = tuple(compose_files)
        self.host_url = host_url.rstrip("/")
        self.reset_db = reset_db
        self.ready_timeout_s = ready_timeout_s
        self.docker_build = docker_build
        self._started = False

    def _compose(self, *args: str) -> None:
        cmd = ["docker", "compose"]
        for f in self.compose_files:
            cmd.extend(["-f", f])
        cmd.extend(args)
        subprocess.check_call(cmd, cwd=os.getcwd())

    def _wait_ready(self) -> None:
        url = f"{self.host_url}/health/ready"
        deadline = time.time() + self.ready_timeout_s
        while True:
            try:
                with urllib.request.urlopen(url, timeout=2) as r:
                    if r.status == 200:
                        return
            except Exception:
                pass
            if time.time() > deadline:
                raise SystemExit(f"Backend not ready after {self.ready_timeout_s}s: {url}")
            time.sleep(2)

    def start(self) -> None:
        if self._started:
            return

        if self.reset_db:
            # Full wipe for deterministic results.
            self._compose("down", "-v")

        up_args = ["up", "-d"]
        if self.docker_build:
            up_args.extend(["--build"])
        up_args.extend(["db", "backend"])
        self._compose(*up_args)

        self._wait_ready()
        self._started = True

    def stop(self) -> None:
        if not self._started:
            return
        self._compose("down")

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, exc_type, exc, tb):
        self.stop()

