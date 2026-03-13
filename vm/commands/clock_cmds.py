"""The ``clock`` command — minimal stubs for init.tcl/tcltest support."""

from __future__ import annotations

import time
from typing import TYPE_CHECKING

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_clock(interp: TclInterp, args: list[str]) -> TclResult:
    """clock subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "clock subcommand ?arg ...?"')

    match args[0]:
        case "seconds":
            return TclResult(value=str(int(time.time())))
        case "clicks":
            return TclResult(value=str(int(time.time_ns() // 1000)))
        case "milliseconds":
            return TclResult(value=str(int(time.time() * 1000)))
        case "microseconds":
            return TclResult(value=str(int(time.time_ns() // 1000)))
        case "format":
            # clock format seconds ?-format fmt? ?-timezone tz?
            if len(args) < 2:
                raise TclError(
                    'wrong # args: should be "clock format clockValue ?-option value ...?"'
                )
            secs = int(args[1])
            fmt = "%a %b %d %H:%M:%S %Z %Y"  # default Tcl format
            i = 2
            while i < len(args):
                if args[i] == "-format" and i + 1 < len(args):
                    fmt = args[i + 1]
                    i += 2
                else:
                    i += 1
            return TclResult(value=time.strftime(fmt, time.localtime(secs)))
        case "scan":
            return TclResult(value=str(int(time.time())))
        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{args[0]}": must be '
                "clicks, format, microseconds, milliseconds, scan, or seconds"
            )


def register() -> None:
    """Register clock command."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("clock", _cmd_clock)
