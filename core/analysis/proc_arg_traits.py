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
from ..parsing.lexer import TclParseError
from .semantic_model import ProcArgTrait

# Simple $varName reference pattern.
_SIMPLE_VAR_RE = re.compile(r"^\$(?:\{([A-Za-z_][\w:]*)\}|([A-Za-z_][\w:]*))\Z")

# Variable-writing commands derived from CommandSpec.assigns_variable_at.
# "dict set" etc. are handled via the registry's ArgRole.VAR_NAME on
# subcommand specs, not here (cmd_name from the segmenter is just "dict").
_var_write_cache: dict[str, int] | None = None


def _var_write_commands() -> dict[str, int]:
    global _var_write_cache
    if _var_write_cache is None:
        from core.commands.registry.runtime import variable_writing_commands

        _var_write_cache = variable_writing_commands()
    return _var_write_cache


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
    except TclParseError:
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

        # Commands that evaluate code (eval, uplevel, subst, etc.)
        spec = REGISTRY.get_any(cmd_name)
        if spec is not None and (spec.evaluates_code or spec.performs_substitution):
            if spec.evaluates_code and cmd_name == "uplevel":
                # uplevel: last arg is the script
                if cmd_args:
                    vn = _extract_var_name(cmd_args[-1])
                    if vn and vn in param_set:
                        traits[vn].add(ProcArgTrait.EVAL)
            else:
                for arg in cmd_args:
                    vn = _extract_var_name(arg)
                    if vn and vn in param_set:
                        traits[vn].add(ProcArgTrait.EVAL)

        # upvar creates aliases
        if spec is not None and spec.creates_scope_alias and cmd_name == "upvar":
            _handle_upvar(cmd_args, param_set, traits, upvar_aliases)

        # foreach / lmap
        if cmd_name in ("foreach", "lmap"):
            _handle_foreach(cmd_args, param_set, traits)

        # while {cond} body
        if cmd_name == "while":
            _handle_while(cmd_args, param_set, traits)

        # for {init} {cond} {next} body
        if cmd_name == "for":
            _handle_for(cmd_args, param_set, traits)

        # after ms ?script ...? / after idle ?script ...?
        if cmd_name == "after":
            _handle_after(cmd_args, param_set, traits)

        # scan string format ?varName ...? — args 2+ are written
        if cmd_name == "scan":
            _handle_variadic_var_write(cmd_args, param_set, traits, start=2)

        # lassign list ?varName ...? — args 1+ are written
        if cmd_name == "lassign":
            _handle_variadic_var_write(cmd_args, param_set, traits, start=1)

        # regexp ?switches? exp string ?matchVar ?subMatchVar ...??
        if cmd_name == "regexp":
            _handle_regexp_vars(cmd_args, param_set, traits)

        # regsub ?switches? exp string subSpec ?varName?
        if cmd_name == "regsub":
            _handle_regsub_var(cmd_args, param_set, traits)

        # switch — body args handled via registry, but dynamic
        # pattern/body pairs also detected via _resolve_arg_roles.

        # Variable-writing commands where param is used as var name
        vwc = _var_write_commands()
        if cmd_name in vwc:
            var_idx = vwc[cmd_name]
            if var_idx < len(cmd_args):
                vn = _extract_var_name(cmd_args[var_idx])
                if vn and vn in param_set:
                    traits[vn].add(ProcArgTrait.VAR_WRITE)

        # Track writes through upvar aliases
        if cmd_name in ("set", "incr", "append", "lappend") and cmd_args:
            alias_target = upvar_aliases.get(cmd_args[0])
            if alias_target is not None:
                traits[alias_target].add(ProcArgTrait.VAR_WRITE)

        # foreach/lmap loop variables are written to — upgrade aliases.
        if cmd_name in ("foreach", "lmap") and len(cmd_args) >= 3:
            remaining = cmd_args[:-1]
            i = 0
            while i < len(remaining):
                alias_target = upvar_aliases.get(remaining[i])
                if alias_target is not None:
                    traits[alias_target].add(ProcArgTrait.VAR_WRITE)
                i += 2


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
    except TclParseError:
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
    """Process upvar to detect variable aliasing and mutation patterns.

    Marks the source param as VAR_READ initially.  If a write through
    the alias is later observed (``set local ...``), the alias tracking
    in ``_scan_commands`` upgrades it to VAR_WRITE.
    """
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
            # Start as VAR_READ; upgraded to VAR_WRITE if alias is written.
            traits[other_vn].add(ProcArgTrait.VAR_READ)
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


def _handle_while(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process ``while {cond} body`` to detect EXPR and BODY traits."""
    if len(args) < 2:
        return

    # while $condParam $bodyParam
    cond_vn = _extract_var_name(args[0])
    if cond_vn and cond_vn in param_set:
        traits[cond_vn].add(ProcArgTrait.EXPR)

    body_vn = _extract_var_name(args[1])
    if body_vn and body_vn in param_set:
        traits[body_vn].add(ProcArgTrait.BODY)


def _handle_for(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process ``for {init} {cond} {next} body`` to detect EXPR and BODY traits."""
    if len(args) < 4:
        return

    # for $initParam $condParam $nextParam $bodyParam
    init_vn = _extract_var_name(args[0])
    if init_vn and init_vn in param_set:
        traits[init_vn].add(ProcArgTrait.BODY)

    cond_vn = _extract_var_name(args[1])
    if cond_vn and cond_vn in param_set:
        traits[cond_vn].add(ProcArgTrait.EXPR)

    next_vn = _extract_var_name(args[2])
    if next_vn and next_vn in param_set:
        traits[next_vn].add(ProcArgTrait.BODY)

    body_vn = _extract_var_name(args[3])
    if body_vn and body_vn in param_set:
        traits[body_vn].add(ProcArgTrait.BODY)


def _handle_after(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process ``after ms ?script?`` and ``after idle ?script?``.

    ``after cancel`` and ``after info`` do not take bodies.
    """
    if len(args) < 2:
        return
    # after cancel / after info — no body
    if args[0] in ("cancel", "info"):
        return
    # after ms script... / after idle script...
    # Script args are everything after the first arg (ms or "idle")
    # and optional -periodic flag (iRules extension to after).
    start = 1
    if start < len(args) and args[start] == "-periodic":
        start += 1
    for arg in args[start:]:
        vn = _extract_var_name(arg)
        if vn and vn in param_set:
            traits[vn].add(ProcArgTrait.EVAL)


def _handle_variadic_var_write(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
    start: int,
) -> None:
    """Mark all args from *start* onwards as VAR_WRITE if they reference params.

    Used for commands like ``scan`` (start=2) and ``lassign`` (start=1)
    where trailing arguments are variable names written by the command.
    """
    for arg in args[start:]:
        vn = _extract_var_name(arg)
        if vn and vn in param_set:
            traits[vn].add(ProcArgTrait.VAR_WRITE)


# Options that regexp/regsub accept before the positional arguments.
_REGEXP_SWITCHES = frozenset(
    {
        "-nocase",
        "-expanded",
        "-line",
        "-linestop",
        "-lineanchor",
        "-all",
        "-inline",
        "-indices",
        "--",
    }
)
_REGEXP_VALUE_SWITCHES = frozenset({"-start"})


def _skip_regexp_switches(args: list[str]) -> int:
    """Return the index of the first positional argument after switches."""
    i = 0
    while i < len(args):
        if args[i] == "--":
            return i + 1
        if args[i] in _REGEXP_SWITCHES:
            i += 1
        elif args[i] in _REGEXP_VALUE_SWITCHES:
            i += 2
        else:
            break
    return i


def _handle_regexp_vars(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process ``regexp ?switches? exp string ?matchVar ?subMatchVar ...??``."""
    pos = _skip_regexp_switches(args)
    # positional: exp(pos), string(pos+1), matchVar(pos+2), subMatchVar(pos+3)...
    var_start = pos + 2
    _handle_variadic_var_write(args, param_set, traits, start=var_start)


def _handle_regsub_var(
    args: list[str],
    param_set: set[str],
    traits: dict[str, set[ProcArgTrait]],
) -> None:
    """Process ``regsub ?switches? exp string subSpec ?varName?``."""
    pos = _skip_regexp_switches(args)
    # positional: exp(pos), string(pos+1), subSpec(pos+2), varName(pos+3)
    var_idx = pos + 3
    if var_idx < len(args):
        vn = _extract_var_name(args[var_idx])
        if vn and vn in param_set:
            traits[vn].add(ProcArgTrait.VAR_WRITE)
