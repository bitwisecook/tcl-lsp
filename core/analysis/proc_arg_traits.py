"""Proc argument trait inference.

Walks a proc body to determine how each parameter is used:
- EVAL: passed to eval/uplevel/subst (treated as code)
- BODY: used as a loop/control body (foreach body, while body, etc.)
- VAR_WRITE: names a variable the proc writes via upvar + set/incr/append
- VAR_READ: names a variable the proc reads via upvar (read-only alias)
- EXPR: evaluated as an expression
- LOOP_LIST: used as the list argument in foreach/lmap

Two analysis tiers:

1. **Shallow** (``infer_param_traits``) — single-pass, top-level command
   scan.  Fast enough for synchronous analysis during typing.  Detects
   direct patterns like ``eval $param`` and ``upvar 1 $param local``.

2. **Deep** (``infer_param_traits_deep``) — recursive descent into nested
   script bodies.  Catches traits hidden inside braced bodies, e.g.
   ``foreach item $items { uplevel 1 $body }``.  Intended to be called
   asynchronously and its results merged into the proc's trait map.
"""

from __future__ import annotations

import re

from ..commands.registry import REGISTRY
from ..commands.registry.signatures import ArgRole
from .semantic_model import ProcArgTrait

# Simple $varName reference pattern.
_SIMPLE_VAR_RE = re.compile(r"^\$(?:\{([A-Za-z_][\w:]*)\}|([A-Za-z_][\w:]*))\Z")

# Variable-writing commands: maps command -> arg index of var name (0-based).
_VAR_WRITE_COMMANDS: dict[str, int] = {
    "set": 0,
    "incr": 0,
    "append": 0,
    "lappend": 0,
    "unset": 0,
    "dict set": 0,
    "dict unset": 0,
    "dict incr": 0,
    "dict append": 0,
    "dict lappend": 0,
    "dict update": 0,
    "dict with": 0,
}


def _extract_var_name(text: str) -> str | None:
    """Extract a bare variable name from ``$var`` or ``${var}``."""
    m = _SIMPLE_VAR_RE.match(text)
    if m:
        return m.group(1) or m.group(2)
    return None


def _resolve_arg_roles(command: str, args: list[str]) -> dict[int, ArgRole]:
    """Get arg roles for a command from the registry."""
    spec = REGISTRY.get_any(command)
    if spec is None:
        return {}

    if spec.arg_role_resolver is not None:
        return spec.arg_role_resolver(args)

    if spec.arg_roles:
        return dict(spec.arg_roles)

    if spec.subcommands and args:
        sub = spec.subcommands.get(args[0])
        if sub is not None and sub.arg_roles:
            return {k + 1: v for k, v in sub.arg_roles.items()}

    return {}


def _extract_commands(source: str) -> list[tuple[str, list[str]]]:
    """Extract (command_name, args) pairs from source via command segmenter."""
    from ..parsing.command_segmenter import segment_commands

    commands: list[tuple[str, list[str]]] = []
    try:
        segments = segment_commands(source)
    except Exception:
        return commands

    for seg in segments:
        if not seg.texts:
            continue
        cmd_name = seg.texts[0]
        cmd_args = seg.texts[1:]
        commands.append((cmd_name, cmd_args))

    return commands


def _scan_commands(
    commands: list[tuple[str, list[str]]],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
    upvar_aliases: dict[str, str],
) -> None:
    """Core trait detection loop shared by both shallow and deep passes."""
    for cmd_name, cmd_args in commands:
        arg_roles = _resolve_arg_roles(cmd_name, cmd_args)

        for idx, arg in enumerate(cmd_args):
            var_name = _extract_var_name(arg)
            if var_name is None:
                continue

            source_param = var_name if var_name in param_set else upvar_aliases.get(var_name)
            if source_param is None:
                continue

            role = arg_roles.get(idx)

            if role is ArgRole.BODY:
                traits[source_param].add(ProcArgTrait.BODY)
            elif role is ArgRole.EXPR:
                traits[source_param].add(ProcArgTrait.EXPR)
            elif role is ArgRole.VAR_NAME:
                traits[source_param].add(ProcArgTrait.VAR_WRITE)
            elif role is ArgRole.VAR_READ:
                traits[source_param].add(ProcArgTrait.VAR_READ)

        # eval/uplevel/subst with param as the script arg
        if cmd_name in ("eval", "subst"):
            for arg in cmd_args:
                vn = _extract_var_name(arg)
                if vn and vn in param_set:
                    traits[vn].add(ProcArgTrait.EVAL)

        if cmd_name == "uplevel":
            if cmd_args:
                vn = _extract_var_name(cmd_args[-1])
                if vn and vn in param_set:
                    traits[vn].add(ProcArgTrait.EVAL)

        # upvar creates aliases
        if cmd_name == "upvar":
            _handle_upvar(cmd_args, param_set, traits, upvar_aliases)

        # foreach / lmap
        if cmd_name in ("foreach", "lmap"):
            _handle_foreach(cmd_args, param_set, traits)

        # Variable-writing commands where param is used as var name
        if cmd_name in _VAR_WRITE_COMMANDS:
            var_idx = _VAR_WRITE_COMMANDS[cmd_name]
            if var_idx < len(cmd_args):
                vn = _extract_var_name(cmd_args[var_idx])
                if vn and vn in param_set:
                    traits[vn].add(ProcArgTrait.VAR_WRITE)

        # Track writes through upvar aliases
        if cmd_name in ("set", "incr", "append", "lappend") and cmd_args:
            alias_target = upvar_aliases.get(cmd_args[0])
            if alias_target is not None:
                traits[alias_target].add(ProcArgTrait.VAR_WRITE)


def infer_param_traits(
    params: tuple[str, ...],
    body_source: str,
) -> dict[str, frozenset[ProcArgTrait]]:
    """Shallow trait inference — top-level commands only.

    Fast enough for synchronous analysis.  Detects direct patterns
    like ``eval $param``, ``upvar 1 $param local``, ``foreach x $list body``.
    Does not recurse into braced body arguments.
    """
    if not params or not body_source.strip():
        return {}

    param_set = set(params)
    traits: dict[str, set[ProcArgTrait]] = {p: set() for p in params}
    upvar_aliases: dict[str, str] = {}

    commands = _extract_commands(body_source)
    _scan_commands(commands, param_set, traits, upvar_aliases)

    return {p: frozenset(t) for p, t in traits.items() if t}


def infer_param_traits_deep(
    params: tuple[str, ...],
    body_source: str,
) -> dict[str, frozenset[ProcArgTrait]]:
    """Deep trait inference — recursively descends into nested script bodies.

    Catches traits hidden inside braced bodies, e.g.::

        foreach item $items {
            uplevel 1 $body    ;# $body EVAL trait detected here
        }

    This is more expensive than ``infer_param_traits`` and should be
    called asynchronously.  Its results are merged (union) with the
    shallow pass results.
    """
    if not params or not body_source.strip():
        return {}

    param_set = set(params)
    traits: dict[str, set[ProcArgTrait]] = {p: set() for p in params}
    upvar_aliases: dict[str, str] = {}

    _scan_deep(body_source, param_set, traits, upvar_aliases, depth=0)

    return {p: frozenset(t) for p, t in traits.items() if t}


_MAX_DEPTH = 8  # prevent runaway recursion on pathological input


def _scan_deep(
    source: str,
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
    upvar_aliases: dict[str, str],
    depth: int,
) -> None:
    """Recursively scan commands, descending into body arguments."""
    if depth > _MAX_DEPTH:
        return

    from ..commands.registry.runtime import body_arg_indices

    commands = _extract_commands(source)
    _scan_commands(commands, param_set, traits, upvar_aliases)

    # Now recurse into body arguments to find deeper usage.
    from ..parsing.command_segmenter import segment_commands

    try:
        segments = segment_commands(source)
    except Exception:
        return

    for seg in segments:
        if not seg.texts:
            continue
        cmd_name = seg.texts[0]
        args = seg.texts[1:]
        body_indices = body_arg_indices(cmd_name, args)

        for idx in body_indices:
            if idx >= len(args):
                continue
            body_text = args[idx]
            if not body_text.strip():
                continue
            # Only recurse into braced bodies (not $var references which
            # are already handled at the top level).
            if "$" in body_text[:2] or "[" in body_text[:2]:
                continue
            _scan_deep(body_text, param_set, traits, upvar_aliases, depth + 1)


def merge_traits(
    shallow: dict[str, frozenset[ProcArgTrait]],
    deep: dict[str, frozenset[ProcArgTrait]],
) -> dict[str, frozenset[ProcArgTrait]]:
    """Merge shallow and deep trait results (union per parameter)."""
    merged: dict[str, frozenset[ProcArgTrait]] = dict(shallow)
    for param, deep_traits in deep.items():
        if param in merged:
            merged[param] = merged[param] | deep_traits
        else:
            merged[param] = deep_traits
    return merged


def _handle_upvar(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
    upvar_aliases: dict[str, str],
) -> None:
    """Process upvar to detect variable aliasing and mutation patterns."""
    start = 0
    if args and (args[0].isdigit() or args[0].startswith("#")):
        start = 1

    i = start
    while i + 1 < len(args):
        other_var = args[i]
        my_var = args[i + 1]
        i += 2

        other_vn = _extract_var_name(other_var)
        my_vn = _extract_var_name(my_var)

        if other_vn and other_vn in param_set:
            traits[other_vn].add(ProcArgTrait.VAR_WRITE)

        if other_vn and other_vn in param_set:
            upvar_aliases[my_var] = other_vn

        if my_vn and my_vn in param_set:
            traits[my_vn].add(ProcArgTrait.VAR_WRITE)


def _handle_foreach(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process foreach/lmap to detect loop list and body usage."""
    if len(args) < 3:
        return

    body_vn = _extract_var_name(args[-1])
    if body_vn and body_vn in param_set:
        traits[body_vn].add(ProcArgTrait.BODY)

    i = 0
    remaining = args[:-1]
    while i + 1 < len(remaining):
        list_vn = _extract_var_name(remaining[i + 1])
        if list_vn and list_vn in param_set:
            traits[list_vn].add(ProcArgTrait.LOOP_LIST)
        i += 2
