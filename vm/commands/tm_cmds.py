"""The ``tm`` command — Tcl Module management stub."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_tm(interp: TclInterp, args: list[str]) -> TclResult:
    """tm subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "tm subcommand ?arg ...?"')

    match args[0]:
        case "path":
            return _tm_path(interp, args[1:])
        case "roots":
            return _tm_roots(interp, args[1:])
        case "list":
            return TclResult()
        case "UnknownHandler":
            return _cmd_tm_unknown_handler(interp, args[1:])
        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{args[0]}": must be '
                "list, path, roots, or UnknownHandler"
            )


def _tm_path(interp: TclInterp, args: list[str]) -> TclResult:
    """tm path ?subcommand? ?path?"""
    if not args:
        # Return the current module path
        paths = getattr(interp, "_tm_paths", [])
        return TclResult(value=" ".join(paths))

    match args[0]:
        case "add":
            if len(args) < 2:
                raise TclError('wrong # args: should be "tm path add path"')
            if not hasattr(interp, "_tm_paths"):
                interp._tm_paths = []  # type: ignore[attr-defined]
            interp._tm_paths.insert(0, args[1])  # type: ignore[attr-defined]
            return TclResult()
        case "remove":
            if len(args) < 2:
                raise TclError('wrong # args: should be "tm path remove path"')
            paths = getattr(interp, "_tm_paths", [])
            try:
                paths.remove(args[1])
            except ValueError:
                pass
            return TclResult()
        case "list":
            paths = getattr(interp, "_tm_paths", [])
            return TclResult(value=" ".join(paths))
        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{args[0]}": must be add, list, or remove'
            )


def _tm_roots(interp: TclInterp, args: list[str]) -> TclResult:
    """tm roots"""
    return TclResult()


def _cmd_tm_unknown_handler(interp: TclInterp, args: list[str]) -> TclResult:
    """::tcl::tm::UnknownHandler — package unknown fallback.

    Called by ``package unknown`` when a package isn't found via
    ``package ifneeded``.  The real implementation searches ``.tm``
    module files.  Our stub delegates directly to the fallback handler
    (typically ``::tclPkgUnknown``) since we don't have a ``.tm``
    file system to search.
    """
    # args: fallback_handler package_name ?version?
    if args:
        fallback = args[0]
        rest = args[1:]
        interp.invoke(fallback, rest)
    return TclResult()


def register() -> None:
    """Register tm command."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("tm", _cmd_tm)
