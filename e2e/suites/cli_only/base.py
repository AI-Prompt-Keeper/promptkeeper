from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

from e2e.common.cli_runner import CliRunner
from e2e.common.cli_session import CliSharedSession


@dataclass
class CliSuiteCtx:
    base_url: str
    cli_bin: str
    shared: Optional[CliSharedSession] = None


class CliSuiteBase:
    def __init__(self, ctx: CliSuiteCtx):
        self.ctx = ctx
        if ctx.shared is not None:
            self.cli = ctx.shared.runner
            self.shared = ctx.shared
        else:
            self.cli = CliRunner(cli_bin=ctx.cli_bin, base_url=ctx.base_url)
            self.shared = None

    def close(self) -> None:
        if self.ctx.shared is None:
            self.cli.close()
