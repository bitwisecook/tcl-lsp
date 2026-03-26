"""Shared command-alias detection and resolution utilities.

Used by both the analyser and the IR lowerer to avoid duplicating the
``interp alias`` parsing and namespace-aware lookup logic.
"""

from __future__ import annotations

from .naming import normalise_qualified_name

# Alias store type: qualified_name -> (target_cmd, prepended_args).
CommandAliasMap = dict[str, tuple[str, tuple[str, ...]]]


def detect_interp_alias(cmd_name: str, args: list[str]) -> tuple[str, str, tuple[str, ...]] | None:
    """Detect ``interp alias {} name {} target ?args?``.

    Returns ``(qualified_alias_name, target_cmd, prepended_args)`` or
    ``None`` if this is not a current-interpreter alias definition.

    Only aliases in the current interpreter (empty source and target
    paths) are tracked.  The 4-argument query form
    (``interp alias {} name``) is correctly ignored.
    """
    if cmd_name != "interp" or len(args) < 5:
        return None
    if args[0] != "alias":
        return None
    src_path, alias_name, target_path = args[1], args[2], args[3]
    target_cmd = args[4]
    prepended = tuple(args[5:])
    if src_path not in ("", "{}") or target_path not in ("", "{}"):
        return None
    qualified = normalise_qualified_name(alias_name) if alias_name else alias_name
    return qualified, target_cmd, prepended


def resolve_alias(
    cmd_name: str,
    aliases: CommandAliasMap,
    namespace: str = "::",
) -> tuple[str, tuple[str, ...]] | None:
    """Look up a command alias, namespace-aware.

    Returns ``(target_cmd, prepended_args)`` or ``None``.

    Lookup mirrors Tcl's command resolution: if *cmd_name* is qualified
    (starts with ``::``), look it up directly.  Otherwise try the
    current *namespace* first, then fall back to global (``::``).
    """
    if cmd_name.startswith("::"):
        return aliases.get(normalise_qualified_name(cmd_name))

    if namespace != "::":
        candidate = normalise_qualified_name(f"{namespace}::{cmd_name}")
        alias = aliases.get(candidate)
        if alias is not None:
            return alias

    return aliases.get(normalise_qualified_name(f"::{cmd_name}"))


def expr_alias_names(aliases: CommandAliasMap) -> frozenset[str]:
    """Return names that are aliases for ``expr`` (no prepended args).

    Returns both the qualified keys (``::=``) and stripped short
    names (``=``) so callers that match against bare command words
    get a hit.
    """
    result: set[str] = set()
    for name, (target, prepended) in aliases.items():
        if target == "expr" and not prepended:
            result.add(name)
            if name.startswith("::"):
                short = name.rsplit("::", 1)[-1]
                if short:
                    result.add(short)
    return frozenset(result)


def lookup_alias_for_word(
    word: str, aliases: CommandAliasMap
) -> tuple[str, tuple[str, ...]] | None:
    """Find an alias entry matching a bare command word.

    Tries ``::word`` (global qualified form).  Returns
    ``(target_cmd, prepended_args)`` or ``None``.
    """
    qualified = normalise_qualified_name(word)
    return aliases.get(qualified)
