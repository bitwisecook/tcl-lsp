"""Core variable-manipulation commands: set, unset, append, global, variable, upvar, uplevel."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _is_int_like(s: str) -> bool:
    """Return True if *s* looks like a (possibly negative) integer literal.

    Accepts decimal (``-5``), hex (``0xff``, ``-0xff``), octal (``0o7``),
    and binary (``0b10``) — i.e. everything ``int(s, 0)`` would accept.
    """
    stripped = s.lstrip("-+")
    if not stripped:
        return False
    if stripped.isdigit():
        return True
    # Check for hex/octal/binary prefixes
    if len(stripped) > 2 and stripped[0] == "0" and stripped[1] in "xXoObB":
        prefix = stripped[1].lower()
        rest = stripped[2:]
        if not rest:
            return False
        if prefix == "x":
            return all(c in "0123456789abcdefABCDEF" for c in rest)
        if prefix == "o":
            return all(c in "01234567" for c in rest)
        if prefix == "b":
            return all(c in "01" for c in rest)
    return False


def _cmd_set(interp: TclInterp, args: list[str]) -> TclResult:
    """set varName ?value?"""
    if not args or len(args) > 2:
        raise TclError('wrong # args: should be "set varName ?newValue?"')
    name = args[0]
    if len(args) == 2:
        value = args[1]
        interp.current_frame.set_var(name, value)
        return TclResult(value=value)
    value = interp.current_frame.get_var(name)
    return TclResult(value=value)


def _cmd_unset(interp: TclInterp, args: list[str]) -> TclResult:
    """unset ?-nocomplain? ?--? ?name ...?

    Tcl uses simple, restrictive argument parsing: only the FIRST
    argument may be ``-nocomplain``, then optionally ``--``.
    """
    nocomplain = False
    i = 0
    if i < len(args) and args[i] == "-nocomplain":
        nocomplain = True
        i += 1
    if i < len(args) and args[i] == "--":
        i += 1
    for name in args[i:]:
        interp.current_frame.unset_var(name, nocomplain=nocomplain)
    return TclResult()


def _cmd_append(interp: TclInterp, args: list[str]) -> TclResult:
    """append varName ?value ...?"""
    if not args:
        raise TclError('wrong # args: should be "append varName ?value ...?"')
    name = args[0]
    old = interp.current_frame.get_var(name, default="")
    new_val = old + "".join(args[1:])
    interp.current_frame.set_var(name, new_val)
    return TclResult(value=new_val)


def _cmd_lappend(interp: TclInterp, args: list[str]) -> TclResult:
    """lappend varName ?value ...?"""
    if not args:
        raise TclError('wrong # args: should be "lappend varName ?value ...?"')
    name = args[0]
    old = interp.current_frame.get_var(name, default="")
    from ..machine import _list_escape

    for v in args[1:]:
        escaped = _list_escape(v)
        old = (old + " " + escaped) if old else escaped
    interp.current_frame.set_var(name, old)
    return TclResult(value=old)


def _cmd_global(interp: TclInterp, args: list[str]) -> TclResult:
    """global varName ?varName ...?"""
    for name in args:
        interp.current_frame.declare_global(name)
        # If the global frame has this variable, it's now accessible.
        # If not, it will be created on first write.
    return TclResult()


def _cmd_variable(interp: TclInterp, args: list[str]) -> TclResult:
    """variable name ?value? ?name value? ..."""
    from ..scope import namespace_qualifiers, namespace_tail, resolve_namespace

    ns = interp.current_namespace
    ns_frame = ns.get_frame(interp)
    in_proc = interp.current_frame is not ns_frame

    i = 0
    while i < len(args):
        name = args[i]
        has_value = i + 1 < len(args)
        value = args[i + 1] if has_value else None

        if "::" in name:
            # Qualified variable name — resolve target namespace
            if not name.startswith("::"):
                # Relative qualified — resolve from current namespace
                if ns.qualname == "::":
                    name = "::" + name
                else:
                    name = ns.qualname + "::" + name
            qual = namespace_qualifiers(name)
            tail = namespace_tail(name)
            target_ns = resolve_namespace(interp.root_namespace, qual)
            if target_ns is None:
                raise TclError(f"can't define \"{args[i]}\": parent namespace doesn't exist")
            target_frame = target_ns.get_frame(interp)
        else:
            tail = name
            target_frame = ns_frame

        # Store the value in the namespace's own frame.
        # When no value is given, declare the variable so it shows up
        # in ``info vars`` but ``info exists`` returns 0.
        if has_value:
            target_frame.set_var(tail, value)  # type: ignore[arg-type]
            target_frame._declared.discard(tail)
        else:
            # Only declare if the variable doesn't already exist
            if tail not in target_frame._scalars and tail not in target_frame._arrays:
                target_frame._declared.add(tail)

        # Inside a proc, create an alias from the local name to the
        # namespace's frame so that ``$tail`` resolves to the namespace
        # variable.  During ``namespace eval`` the current frame IS the
        # namespace frame so no alias is needed.
        if in_proc:
            interp.current_frame.add_alias(tail, target_frame, tail)

        i += 2 if has_value else 1
    return TclResult()


def _cmd_upvar(interp: TclInterp, args: list[str]) -> TclResult:
    """upvar ?level? otherVar myVar ?otherVar myVar ...?"""
    if len(args) < 2:
        raise TclError(
            'wrong # args: should be "upvar ?level? otherVar localVar ?otherVar localVar ...?"'
        )

    # Determine the level.  Tcl's rules:
    # - "#<int>" is always an absolute level
    # - A plain integer is consumed as a relative level
    # - Otherwise, if the remaining arg count is odd (suggesting first is
    #   a level), try to parse it as a level → "bad level" error
    # - If even, default level to 1 (caller's frame)
    start = 0
    level_str = args[0]
    if level_str.startswith("#"):
        rest = level_str[1:]
        try:
            target_level = int(rest)
        except ValueError:
            raise TclError(f'bad level "{level_str}"') from None
        target_frame = interp.frame_at_level(target_level, level_str)
        start = 1
    elif _is_int_like(level_str):
        try:
            rel = int(level_str, 0)
        except ValueError:
            rel = int(level_str, 10)
        target_frame = interp.frame_at_relative(rel, level_str)
        start = 1
    elif (len(args) - 0) % 2 == 1:
        # Odd arg count → first arg is assumed to be a level but isn't valid
        raise TclError(f'bad level "{level_str}"')
    else:
        # Default: level 1 (caller's frame)
        target_frame = interp.frame_at_relative(1)

    # Remaining args must be pairs
    remaining = len(args) - start
    if remaining % 2 != 0 or remaining == 0:
        raise TclError(
            'wrong # args: should be "upvar ?level? otherVar localVar ?otherVar localVar ...?"'
        )

    # When running at global scope (target == global frame) inside a
    # namespace context, Tcl resolves unqualified variable names
    # relative to the current namespace.
    at_global_scope = interp.current_frame.parent is None
    ns_prefix = ""
    if at_global_scope and interp.current_namespace and interp.current_namespace.name != "::":
        ns_prefix = interp.current_namespace.name.lstrip(":") + "::"

    i = start
    while i + 1 < len(args):
        other_name = args[i]
        my_name = args[i + 1]
        if ns_prefix:
            if not other_name.startswith("::"):
                other_name = ns_prefix + other_name
            if not my_name.startswith("::"):
                my_name = ns_prefix + my_name

        # Validate: can't upvar from variable to itself (direct or via aliases)
        if target_frame is interp.current_frame and other_name == my_name:
            raise TclError("can't upvar from variable to itself")
        # Check if the target resolves back to my_name in the current frame
        # (circular alias detection, e.g. upvar 0 a b; upvar 0 b a)
        resolved_frame, resolved_name = target_frame._resolve(other_name)
        if resolved_frame is interp.current_frame and resolved_name == my_name:
            raise TclError("can't upvar from variable to itself")

        # Validate: local variable must not already exist as a scalar/array
        if my_name in interp.current_frame._scalars or my_name in interp.current_frame._arrays:
            raise TclError(f'variable "{my_name}" already exists')

        # Validate: local variable must not have traces
        if my_name in interp.variable_traces:
            raise TclError(f'variable "{my_name}" has traces: can\'t use for upvar')

        # Validate: target of upvar with array-like name rejects array syntax
        if "(" in my_name and my_name.endswith(")"):
            raise TclError(
                f'bad variable name "{my_name}": upvar won\'t create a'
                " variable that looks like an array element"
            )

        interp.current_frame.add_alias(my_name, target_frame, other_name)
        i += 2
    return TclResult()


def _cmd_uplevel(interp: TclInterp, args: list[str]) -> TclResult:
    """uplevel ?level? command ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "uplevel ?level? command ?arg ...?"')

    # Determine the level.  Tcl's rules:
    # - "#<int>" is always an absolute level (even with only 1 arg)
    # - A plain integer is a relative level ONLY when len(args) > 1
    #   or when it's the sole argument inside a proc (→ wrong # args)
    # - Anything else is treated as the script body (level defaults to 1)
    level_str = args[0]
    if level_str.startswith("#"):
        rest = level_str[1:]
        try:
            target_level = int(rest)
        except ValueError:
            raise TclError(f'bad level "{level_str}"') from None
        target_frame = interp.frame_at_level(target_level, level_str)
        script_args = args[1:]
    elif _is_int_like(level_str):
        if len(args) < 2:
            raise TclError('wrong # args: should be "uplevel ?level? command ?arg ...?"')
        try:
            rel = int(level_str, 0)
        except ValueError:
            rel = int(level_str, 10)
        target_frame = interp.frame_at_relative(rel, level_str)
        script_args = args[1:]
    else:
        target_frame = interp.frame_at_relative(1)
        script_args = args

    if not script_args:
        raise TclError('wrong # args: should be "uplevel ?level? command ?arg ...?"')

    script = " ".join(script_args) if len(script_args) > 1 else script_args[0]

    # Execute in the target frame
    saved_frame = interp.current_frame
    interp.current_frame = target_frame
    try:
        return interp.eval(script)
    finally:
        interp.current_frame = saved_frame


def _cmd_concat(interp: TclInterp, args: list[str]) -> TclResult:
    """concat ?arg ...?"""
    parts = []
    for a in args:
        stripped = a.strip()
        if stripped:
            parts.append(stripped)
    return TclResult(value=" ".join(parts))


def _cmd_subst(interp: TclInterp, args: list[str]) -> TclResult:
    """subst ?-nobackslashes? ?-nocommands? ?-novariables? string"""
    from ..substitution import subst_command

    if not args:
        raise TclError(
            'wrong # args: should be "subst ?-nobackslashes? ?-nocommands? ?-novariables? string"'
        )
    nobackslashes = False
    nocommands = False
    novariables = False
    i = 0
    while i < len(args) - 1:
        flag = args[i]
        if flag == "-nobackslashes":
            nobackslashes = True
        elif flag == "-nocommands":
            nocommands = True
        elif flag == "-novariables":
            novariables = True
        else:
            raise TclError(
                f'bad switch "{flag}": must be -nobackslashes, -nocommands, or -novariables'
            )
        i += 1
    text = args[-1]
    result = subst_command(
        text,
        interp,
        nobackslashes=nobackslashes,
        nocommands=nocommands,
        novariables=novariables,
        handle_exceptions=True,
    )
    return TclResult(value=result)


def register() -> None:
    """Register core commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("set", _cmd_set)
    REGISTRY.register_handler("unset", _cmd_unset)
    REGISTRY.register_handler("append", _cmd_append)
    REGISTRY.register_handler("lappend", _cmd_lappend)
    REGISTRY.register_handler("global", _cmd_global)
    REGISTRY.register_handler("variable", _cmd_variable)
    REGISTRY.register_handler("upvar", _cmd_upvar)
    REGISTRY.register_handler("uplevel", _cmd_uplevel)
    REGISTRY.register_handler("concat", _cmd_concat)
    REGISTRY.register_handler("subst", _cmd_subst)
