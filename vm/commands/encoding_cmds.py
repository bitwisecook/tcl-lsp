"""The ``encoding`` command — stub for init.tcl support."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_encoding(interp: TclInterp, args: list[str]) -> TclResult:
    """encoding subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "encoding subcommand ?arg ...?"')

    match args[0]:
        case "dirs":
            if len(args) > 1:
                interp._encoding_dirs = args[1]  # type: ignore[attr-defined]
                return TclResult()
            return TclResult(value=getattr(interp, "_encoding_dirs", ""))
        case "system":
            if len(args) > 1:
                return TclResult()  # ignore set, always utf-8
            return TclResult(value="utf-8")
        case "convertfrom":
            # encoding convertfrom ?encoding? data
            return TclResult(value=args[-1])
        case "convertto":
            # encoding convertto ?encoding? data
            return TclResult(value=args[-1])
        case "names":
            return TclResult(value="utf-8 ascii iso8859-1 unicode")
        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{args[0]}": must be '
                "convertfrom, convertto, dirs, names, or system"
            )


def register() -> None:
    """Register encoding command."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("encoding", _cmd_encoding)
