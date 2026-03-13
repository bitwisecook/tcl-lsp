"""Namespace command implementation."""

from __future__ import annotations

import fnmatch
from typing import TYPE_CHECKING

from ..scope import (
    ensure_namespace,
    namespace_qualifiers,
    namespace_tail,
    resolve_namespace,
)
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _resolve_ns_name(interp: TclInterp, name: str) -> str:
    """Turn a potentially relative namespace name into a fully-qualified one."""
    if name.startswith("::"):
        return name
    cur = interp.current_namespace.qualname
    if cur == "::":
        return f"::{name}"
    return f"{cur}::{name}"


def _cmd_namespace(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "namespace subcommand ?arg ...?"')

    subcmd = args[0]
    rest = args[1:]

    match subcmd:
        case "eval":
            return _ns_eval(interp, rest)
        case "current":
            return TclResult(value=interp.current_namespace.qualname)
        case "export":
            return _ns_export(interp, rest)
        case "import":
            return _ns_import(interp, rest)
        case "exists":
            return _ns_exists(interp, rest)
        case "children":
            return _ns_children(interp, rest)
        case "parent":
            return _ns_parent(interp, rest)
        case "delete":
            return _ns_delete(interp, rest)
        case "qualifiers":
            if not rest:
                raise TclError('wrong # args: should be "namespace qualifiers string"')
            return TclResult(value=namespace_qualifiers(rest[0]))
        case "tail":
            if not rest:
                raise TclError('wrong # args: should be "namespace tail string"')
            return TclResult(value=namespace_tail(rest[0]))
        case "which":
            return _ns_which(interp, rest)
        case "code":
            return _ns_code(interp, rest)
        case "inscope":
            return _ns_inscope(interp, rest)
        case "path":
            return _ns_path(interp, rest)
        case "upvar":
            return _ns_upvar(interp, rest)
        case "unknown":
            return _ns_unknown(interp, rest)
        case "ensemble":
            return _ns_ensemble(interp, rest)
        case "origin":
            return _ns_origin(interp, rest)
        case _:
            raise TclError(
                f'bad option "{subcmd}": must be children, code, current, delete, '
                f"ensemble, eval, exists, export, import, inscope, origin, parent, "
                f"path, qualifiers, tail, unknown, upvar, or which"
            )


def _ns_eval(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace eval ns body ?body ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "namespace eval name arg ?arg ...?"')

    ns_name = _resolve_ns_name(interp, args[0])
    ns = ensure_namespace(interp.root_namespace, ns_name)

    # Concatenate remaining args as the body
    body = " ".join(args[1:]) if len(args) > 2 else args[1]

    # Execute in the namespace context.  Each namespace has its own
    # persistent CallFrame so that variables set during ``namespace eval``
    # are scoped to the namespace (matching real Tcl behaviour).  The
    # global namespace's frame is the interpreter's global frame.
    saved_ns = interp.current_namespace
    saved_frame = interp.current_frame
    interp.current_namespace = ns
    interp.current_frame = ns.get_frame(interp)
    try:
        result = interp.eval(body)
        return result
    finally:
        interp.current_namespace = saved_ns
        interp.current_frame = saved_frame


def _ns_export(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace export ?-clear? ?pattern ...?"""
    ns = interp.current_namespace
    clear = False
    patterns = []
    for arg in args:
        if arg == "-clear":
            clear = True
        else:
            patterns.append(arg)

    if clear:
        ns._export_patterns.clear()

    if not patterns:
        # Return current export list
        return TclResult(value=" ".join(ns._export_patterns))

    for pat in patterns:
        ns.add_export(pat)
    return TclResult()


def _ns_import(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace import ?-force? ?pattern ...?"""
    force = False
    patterns = []
    for arg in args:
        if arg == "-force":
            force = True
        else:
            patterns.append(arg)

    for pattern in patterns:
        # Pattern like ::foo::bar::* or ::tcltest::*
        qual = namespace_qualifiers(pattern)
        tail_pat = namespace_tail(pattern)
        source_ns = resolve_namespace(interp.root_namespace, qual)
        if source_ns is None:
            raise TclError(f'unknown namespace in import pattern "{pattern}"')

        exported = source_ns.exported_commands()
        for cmd_name in exported:
            if not fnmatch.fnmatch(cmd_name, tail_pat):
                continue
            # Check if source has the command
            handler = source_ns.lookup_command(cmd_name)
            proc = source_ns.lookup_proc(cmd_name)
            if handler is not None:
                existing = interp.current_namespace.lookup_command(cmd_name)
                if existing is not None and not force:
                    raise TclError(f'can\'t import command "{cmd_name}": already exists')
                interp.current_namespace.register_command(cmd_name, handler)
            elif proc is not None:
                existing = interp.current_namespace.lookup_proc(cmd_name)
                if existing is not None and not force:
                    raise TclError(f'can\'t import command "{cmd_name}": already exists')
                interp.current_namespace.register_proc(cmd_name, proc)
                # Also register in flat procedures table for backwards compat
                interp.procedures[cmd_name] = proc

    return TclResult()


def _ns_exists(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace exists name"""
    if not args:
        raise TclError('wrong # args: should be "namespace exists name"')
    ns_name = _resolve_ns_name(interp, args[0])
    ns = resolve_namespace(interp.root_namespace, ns_name)
    return TclResult(value="1" if ns is not None else "0")


def _ns_children(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace children ?ns? ?pattern?"""
    if not args:
        ns = interp.current_namespace
    else:
        ns_name = _resolve_ns_name(interp, args[0])
        ns = resolve_namespace(interp.root_namespace, ns_name)
        if ns is None:
            raise TclError(f'namespace "{args[0]}" not found')

    pattern = args[1] if len(args) > 1 else None
    children = list(ns.children.values())

    if pattern:
        children = [c for c in children if fnmatch.fnmatch(c.qualname, pattern)]

    return TclResult(value=" ".join(c.qualname for c in children))


def _ns_parent(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace parent ?ns?"""
    if not args:
        ns = interp.current_namespace
    else:
        ns_name = _resolve_ns_name(interp, args[0])
        ns = resolve_namespace(interp.root_namespace, ns_name)
        if ns is None:
            raise TclError(f'namespace "{args[0]}" not found')

    if ns.parent is None:
        return TclResult(value="")
    return TclResult(value=ns.parent.qualname)


def _ns_delete(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace delete ?name ...?"""
    for name in args:
        ns_name = _resolve_ns_name(interp, name)
        ns = resolve_namespace(interp.root_namespace, ns_name)
        if ns is None:
            raise TclError(f'unknown namespace "{ns_name}" in namespace delete command')
        if ns is interp.root_namespace:
            raise TclError("cannot delete the global namespace")
        ns.delete()
    return TclResult()


def _ns_which(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace which ?-command|-variable? name"""
    which_type = "command"
    name = ""
    i = 0
    while i < len(args):
        if args[i] == "-command":
            which_type = "command"
            i += 1
        elif args[i] == "-variable":
            which_type = "variable"
            i += 1
        else:
            name = args[i]
            i += 1

    if not name:
        raise TclError('wrong # args: should be "namespace which ?-command? ?-variable? name"')

    if which_type == "command":
        # Check current namespace
        if interp.current_namespace.lookup_command(name) is not None:
            if interp.current_namespace.qualname == "::":
                return TclResult(value=f"::{name}")
            return TclResult(value=f"{interp.current_namespace.qualname}::{name}")
        if interp.current_namespace.lookup_proc(name) is not None:
            if interp.current_namespace.qualname == "::":
                return TclResult(value=f"::{name}")
            return TclResult(value=f"{interp.current_namespace.qualname}::{name}")
        # Check global builtins
        if interp.lookup_command(name) is not None:
            return TclResult(value=f"::{name}")
        if name in interp.procedures:
            return TclResult(value=f"::{name}")
        return TclResult(value="")
    else:
        # variable — check the namespace's own frame
        ns_frame = interp.current_namespace.get_frame(interp)
        if ns_frame.exists(name):
            if interp.current_namespace.qualname == "::":
                return TclResult(value=f"::{name}")
            return TclResult(value=f"{interp.current_namespace.qualname}::{name}")
        return TclResult(value="")


def _ns_code(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace code script"""
    if not args:
        raise TclError('wrong # args: should be "namespace code arg"')
    ns = interp.current_namespace.qualname
    return TclResult(value=f"::namespace inscope {ns} {args[0]}")


def _ns_inscope(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace inscope ns arg ?arg ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "namespace inscope name arg ?arg ...?"')
    ns_name = _resolve_ns_name(interp, args[0])
    ns = resolve_namespace(interp.root_namespace, ns_name)
    if ns is None:
        raise TclError(f'unknown namespace "{ns_name}"')

    body = args[1]
    if len(args) > 2:
        # Extra args are appended to the script as arguments
        body = body + " " + " ".join(args[2:])

    saved_ns = interp.current_namespace
    saved_frame = interp.current_frame
    interp.current_namespace = ns
    interp.current_frame = ns.get_frame(interp)
    try:
        return interp.eval(body)
    finally:
        interp.current_namespace = saved_ns
        interp.current_frame = saved_frame


def _ns_path(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace path ?nsList?"""
    ns = interp.current_namespace
    if not args:
        return TclResult(value=" ".join(n.qualname for n in ns._path))

    from ..machine import _split_list

    ns_names = _split_list(args[0])
    ns._path = []
    for name in ns_names:
        resolved = resolve_namespace(interp.root_namespace, _resolve_ns_name(interp, name))
        if resolved is None:
            raise TclError(f'namespace "{name}" not found in "namespace path"')
        ns._path.append(resolved)
    return TclResult()


def _ns_upvar(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace upvar ns ?otherVar myVar ...?"""
    if len(args) < 1 or (len(args) > 1 and len(args) % 2 == 0):
        raise TclError('wrong # args: should be "namespace upvar ns ?otherVar myVar ...?"')
    ns_name = _resolve_ns_name(interp, args[0])
    ns = resolve_namespace(interp.root_namespace, ns_name)
    if ns is None:
        raise TclError(f'namespace "{ns_name}" not found')

    # Create aliases from current frame to the namespace's own frame
    ns_frame = ns.get_frame(interp)
    for i in range(1, len(args), 2):
        other_var = args[i]
        my_var = args[i + 1]
        interp.current_frame.add_alias(my_var, ns_frame, other_var)

    return TclResult()


def _ns_unknown(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace unknown ?script?"""
    ns = interp.current_namespace
    if not args:
        return TclResult(value=ns._unknown_handler or "")
    ns._unknown_handler = args[0]
    return TclResult()


def _ns_ensemble(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace ensemble create/configure/exists"""
    if not args:
        raise TclError('wrong # args: should be "namespace ensemble subcommand ?arg ...?"')

    subcmd = args[0]
    rest = args[1:]

    match subcmd:
        case "create":
            return _ns_ensemble_create(interp, rest)
        case "configure":
            # Stub — return empty for now
            return TclResult()
        case "exists":
            if not rest:
                raise TclError('wrong # args: should be "namespace ensemble exists cmdname"')
            # Stub — check if the command exists
            handler = interp.lookup_command(rest[0])
            return TclResult(value="1" if handler is not None else "0")
        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{subcmd}": must be configure, create, or exists'
            )


def _ns_ensemble_create(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace ensemble create ?-option value ...?"""
    ns = interp.current_namespace
    command_name = namespace_tail(ns.qualname)
    subcommands: dict[str, str] | None = None
    _map: dict[str, str] | None = None

    i = 0
    while i < len(args):
        opt = args[i]
        if opt == "-command" and i + 1 < len(args):
            command_name = args[i + 1]
            i += 2
        elif opt == "-subcommands" and i + 1 < len(args):
            from ..machine import _split_list

            subcommands = {s: s for s in _split_list(args[i + 1])}
            i += 2
        elif opt == "-map" and i + 1 < len(args):
            from ..machine import _split_list

            items = _split_list(args[i + 1])
            _map = {}
            for j in range(0, len(items), 2):
                if j + 1 < len(items):
                    _map[items[j]] = items[j + 1]
            i += 2
        elif opt == "-prefixes" and i + 1 < len(args):
            i += 2  # skip, not implemented
        elif opt == "-unknown" and i + 1 < len(args):
            i += 2  # skip
        else:
            i += 1

    # Create the ensemble command as a dispatcher
    captured_ns = ns
    captured_map = _map
    captured_subcmds = subcommands

    def ensemble_handler(interp_: TclInterp, handler_args: list[str]) -> TclResult:
        if not handler_args:
            raise TclError(f'wrong # args: should be "{command_name} subcommand ?arg ...?"')
        sub = handler_args[0]
        sub_args = handler_args[1:]

        # Check map first
        if captured_map and sub in captured_map:
            target = captured_map[sub]
            return interp_.invoke(target, sub_args)

        # Check subcommands list
        if captured_subcmds and sub not in captured_subcmds:
            raise TclError(
                f'unknown or ambiguous subcommand "{sub}": must be '
                + ", ".join(sorted(captured_subcmds.keys()))
            )

        # Look up in the namespace
        qual = f"{captured_ns.qualname}::{sub}"
        return interp_.invoke(qual, sub_args)

    interp.register_command(command_name, ensemble_handler)
    return TclResult(
        value=f"::{command_name}" if not command_name.startswith("::") else command_name
    )


def _ns_origin(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace origin command

    Return the fully-qualified name of the original command that
    *command* refers to.  If *command* is not an imported command,
    return its fully-qualified name directly.
    """
    if not args:
        raise TclError('wrong # args: should be "namespace origin name"')

    name = args[0]

    # Fully-qualified name — look it up directly.
    if name.startswith("::"):
        tail = namespace_tail(name)
        qual = namespace_qualifiers(name)
        ns = resolve_namespace(interp.root_namespace, qual)
        if ns is not None:
            if ns.lookup_command(tail) is not None or ns.lookup_proc(tail) is not None:
                return TclResult(value=name)
        raise TclError(f'invalid command name "{name}"')

    # Unqualified name — search the current namespace, then global.
    cur = interp.current_namespace
    if cur.lookup_command(name) is not None or cur.lookup_proc(name) is not None:
        fqn = f"{cur.qualname}::{name}" if cur.qualname != "::" else f"::{name}"
        return TclResult(value=fqn)

    # Check global builtins / procs
    if interp.lookup_command(name) is not None or name in interp.procedures:
        return TclResult(value=f"::{name}")

    raise TclError(f'invalid command name "{name}"')


def register() -> None:
    """Register namespace commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("namespace", _cmd_namespace)
