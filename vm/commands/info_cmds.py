"""The ``info`` command and its subcommands."""

from __future__ import annotations

import os
import sys
from typing import TYPE_CHECKING

from ..machine import _list_escape
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_info(interp: TclInterp, args: list[str]) -> TclResult:
    """info subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "info subcommand ?arg ...?"')
    sub = args[0]
    rest = args[1:]

    # Resolve unique subcommand abbreviation — must match C Tcl 9.0's list
    # exactly so that error messages and abbreviation resolution are conformant.
    _INFO_SUBCMDS = (
        "args",
        "body",
        "class",
        "cmdcount",
        "cmdtype",
        "commands",
        "complete",
        "constant",
        "consts",
        "coroutine",
        "default",
        "errorstack",
        "exists",
        "frame",
        "functions",
        "globals",
        "hostname",
        "level",
        "library",
        "loaded",
        "locals",
        "nameofexecutable",
        "object",
        "patchlevel",
        "procs",
        "script",
        "sharedlibextension",
        "tclversion",
        "vars",
    )
    matches = [s for s in _INFO_SUBCMDS if s.startswith(sub)]
    if len(matches) == 1:
        sub = matches[0]
    elif len(matches) > 1 and sub not in _INFO_SUBCMDS:
        raise TclError(
            f'unknown or ambiguous subcommand "{sub}": must be '
            + ", ".join(_INFO_SUBCMDS[:-1])
            + ", or "
            + _INFO_SUBCMDS[-1]
        )

    match sub:
        case "exists":
            if not rest:
                raise TclError('wrong # args: should be "info exists varName"')
            return TclResult(value="1" if interp.current_frame.exists(rest[0]) else "0")

        case "commands":
            pattern = rest[0] if rest else "*"
            import fnmatch

            from ..scope import resolve_namespace

            names: list[str] = list(interp.command_names())
            names.extend(interp.procedures.keys())

            # If the pattern has namespace qualifiers, also search that namespace
            if "::" in pattern:
                # Normalise relative patterns to fully-qualified so fnmatch
                # works against the ``::ns::name`` entries we collect.
                if not pattern.startswith("::"):
                    cur = interp.current_namespace.qualname
                    if cur == "::":
                        pattern = "::" + pattern
                    else:
                        pattern = cur + "::" + pattern

                ns_part = pattern[: pattern.rfind("::")]
                ns = resolve_namespace(interp.root_namespace, ns_part if ns_part else "::")
                if ns is not None:
                    # Add namespace procs with fully-qualified names
                    for pname in ns.proc_names():
                        fq = ns.qualname + "::" + pname
                        names.append(fq)
                    # Add namespace commands with fully-qualified names
                    for cname in ns.command_names():
                        fq = ns.qualname + "::" + cname
                        names.append(fq)

            matched = [n for n in sorted(set(names)) if fnmatch.fnmatch(n, pattern)]
            return TclResult(value=" ".join(_list_escape(n) for n in matched))

        case "procs":
            pattern = rest[0] if rest else "*"
            import fnmatch

            matched = [n for n in sorted(interp.procedures.keys()) if fnmatch.fnmatch(n, pattern)]
            return TclResult(value=" ".join(_list_escape(n) for n in matched))

        case "vars":
            pattern = rest[0] if rest else "*"
            import fnmatch

            from ..scope import resolve_namespace

            if "::" in pattern:
                # Qualified pattern — search the target namespace's frame
                if not pattern.startswith("::"):
                    cur = interp.current_namespace.qualname
                    if cur == "::":
                        pattern = "::" + pattern
                    else:
                        pattern = cur + "::" + pattern
                ns_part = pattern[: pattern.rfind("::")]
                ns = resolve_namespace(interp.root_namespace, ns_part if ns_part else "::")
                if ns is not None:
                    ns_frame = ns.get_frame(interp)
                    prefix = ns.qualname + "::" if ns.qualname != "::" else "::"
                    names = [prefix + n for n in ns_frame.var_names()]
                else:
                    names = []
            else:
                names = interp.current_frame.var_names()
            matched = [n for n in sorted(names) if fnmatch.fnmatch(n, pattern)]
            return TclResult(value=" ".join(_list_escape(n) for n in matched))

        case "globals":
            pattern = rest[0] if rest else "*"
            import fnmatch

            names = interp.global_frame.var_names()
            matched = [n for n in sorted(names) if fnmatch.fnmatch(n, pattern)]
            return TclResult(value=" ".join(_list_escape(n) for n in matched))

        case "locals":
            pattern = rest[0] if rest else "*"
            import fnmatch

            names = interp.current_frame.var_names()
            matched = [n for n in sorted(names) if fnmatch.fnmatch(n, pattern)]
            return TclResult(value=" ".join(_list_escape(n) for n in matched))

        case "body":
            if not rest:
                raise TclError('wrong # args: should be "info body procname"')
            proc = interp.procedures.get(rest[0])
            if proc is None:
                raise TclError(f'"{rest[0]}" isn\'t a procedure')
            return TclResult(value=proc.body)

        case "args":
            if not rest:
                raise TclError('wrong # args: should be "info args procname"')
            proc = interp.procedures.get(rest[0])
            if proc is None:
                raise TclError(f'"{rest[0]}" isn\'t a procedure')
            return TclResult(value=" ".join(_list_escape(p) for p in proc.param_names))

        case "default":
            if len(rest) < 3:
                raise TclError('wrong # args: should be "info default procname arg varname"')
            proc = interp.procedures.get(rest[0])
            if proc is None:
                raise TclError(f'"{rest[0]}" isn\'t a procedure')
            param_name = rest[1]
            var_name = rest[2]
            for pn, default in proc.params_with_defaults:
                if pn == param_name:
                    if default is not None:
                        interp.current_frame.set_var(var_name, default)
                        return TclResult(value="1")
                    interp.current_frame.set_var(var_name, "")
                    return TclResult(value="0")
            raise TclError(f'procedure "{rest[0]}" doesn\'t have an argument "{param_name}"')

        case "level":
            if not rest:
                return TclResult(value=str(interp.current_frame.level))
            level = int(rest[0])
            # Non-positive levels are relative: 0 = current, -1 = one up
            if level <= 0:
                frame = interp.frame_at_relative(-level)
            else:
                frame = interp.frame_at_level(level)
            if frame.proc_name:
                # Return the full invocation: command name + args
                parts = [_list_escape(frame.proc_name)]
                if frame.call_args:
                    parts.extend(_list_escape(a) for a in frame.call_args)
                return TclResult(value=" ".join(parts))
            return TclResult(value="")

        case "script":
            return TclResult(value=interp.script_file or "")

        case "nameofexecutable":
            return TclResult(value=sys.executable)

        case "patchlevel":
            return TclResult(value=interp.global_frame.get_var("tcl_patchLevel", default="9.0.3"))

        case "tclversion":
            return TclResult(value=interp.global_frame.get_var("tcl_version", default="9.0"))

        case "hostname":
            import socket

            return TclResult(value=socket.gethostname())

        case "library":
            return TclResult(value=interp.global_frame.get_var("tcl_library", default=""))

        case "sharedlibextension":
            if sys.platform == "darwin":
                return TclResult(value=".dylib")
            if sys.platform == "win32":
                return TclResult(value=".dll")
            return TclResult(value=".so")

        case "loaded":
            return TclResult(value="")

        case "cmdcount":
            return TclResult(value=str(interp.cmd_count))

        case "frame":
            if not rest:
                return TclResult(value=str(interp.current_frame.level))
            return TclResult(value="")

        case "complete":
            if not rest:
                raise TclError('wrong # args: should be "info complete command"')
            return TclResult(value="1" if interp.is_complete(rest[0]) else "0")

        case "object":
            # Stub for TclOO
            return TclResult()

        case "coroutine":
            return TclResult(value="")

        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{sub}": must be '
                + ", ".join(_INFO_SUBCMDS[:-1])
                + ", or "
                + _INFO_SUBCMDS[-1]
            )


def _cmd_pid(interp: TclInterp, args: list[str]) -> TclResult:
    """pid"""
    return TclResult(value=str(os.getpid()))


def _cmd_pwd(interp: TclInterp, args: list[str]) -> TclResult:
    """pwd"""
    return TclResult(value=os.getcwd())


def _cmd_cd(interp: TclInterp, args: list[str]) -> TclResult:
    """cd ?dirName?"""
    directory = args[0] if args else os.path.expanduser("~")
    try:
        os.chdir(directory)
    except OSError as e:
        raise TclError(f'couldn\'t change working directory to "{directory}": {e}') from e
    return TclResult()


def register() -> None:
    """Register info commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("info", _cmd_info)
    REGISTRY.register_handler("pid", _cmd_pid)
    REGISTRY.register_handler("pwd", _cmd_pwd)
    REGISTRY.register_handler("cd", _cmd_cd)
