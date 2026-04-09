"""Variable-scoping command detection.

Shared helpers for identifying which tokens in a ``global`` / ``variable`` /
``upvar`` (or ``namespace upvar``) command are declarations.  Both the LSP
``textDocument/declaration`` provider and the compiler's memory-SSA alias
detector parse these commands; putting the logic here keeps the semantics in
one place.

Each helper takes a command name plus the raw argument strings (or
arbitrary token-like objects whose ``text`` is a string) and returns
*indices into the argument list* for the positions that name a declared
variable.  Callers then map those indices back to whatever representation
they care about -- ``str`` names for memory-SSA, source tokens for the LSP
declaration provider.
"""

from __future__ import annotations

from typing import Sequence


def _arg_text(arg: object) -> str:
    """Return the text of an argument.

    Accepts either a plain string or a token-like object with a ``text``
    attribute.  This lets callers pass ``tuple[str, ...]`` (IR form) or
    ``list[Token]`` (parser form) without an explicit conversion step.
    """
    if isinstance(arg, str):
        return arg
    return getattr(arg, "text", "")


def global_declaration_indices(args: Sequence[object]) -> list[int]:
    """Return indices of declared variables in ``global var1 var2 ...``.

    Mirrors :func:`core.compiler.memory_ssa._detect_global` -- every
    argument whose text does not start with ``$`` (i.e. is a bare name,
    not a substituted reference) is a declaration.
    """
    return [i for i, a in enumerate(args) if (t := _arg_text(a)) and not t.startswith("$")]


def variable_declaration_indices(args: Sequence[object]) -> list[int]:
    """Return indices of declared variables in ``variable name ?value? ...``.

    Mirrors :func:`core.compiler.memory_ssa._detect_namespace_variable`.
    The ``variable`` command alternates (name, value?) pairs, so every
    even-indexed arg is a name; bare-name filter matches the compiler's
    ``not a.startswith('$')`` guard.
    """
    result: list[int] = []
    for i in range(0, len(args), 2):
        text = _arg_text(args[i])
        if text and not text.startswith("$"):
            result.append(i)
    return result


def upvar_local_declaration_indices(
    command: str,
    args: Sequence[object],
) -> list[int]:
    """Return indices of the *local-alias* tokens in an ``upvar`` command.

    Mirrors :func:`core.compiler.memory_ssa._detect_upvar`, but returns
    indices into the caller's own ``args`` sequence rather than
    ``(caller, local)`` name pairs.  Handles both ``upvar`` and
    ``namespace upvar`` (including the lowered form
    ``namespace args=('upvar', ns, src, dst, ...)``).

    ``command`` is the command word as the caller saw it; the
    token/string list in ``args`` should be the command's remaining
    arguments *excluding* the command word.  For the lowered
    ``namespace`` form where the first argument is ``'upvar'``, callers
    can either pre-strip it or pass ``command='namespace'``; the helper
    auto-detects and advances the start index.
    """
    if not args:
        return []

    # Offset into the original args sequence at which the "pair" walk begins.
    # Keeps the returned indices in the caller's coordinate system.
    offset = 0

    if command == "namespace":
        # Lowered ``namespace upvar ns src dst ...`` form.
        if _arg_text(args[0]) != "upvar":
            return []
        offset = 2  # skip 'upvar' subcommand + namespace argument
    elif command == "namespace upvar":
        offset = 1  # skip namespace argument
    elif command == "upvar":
        # Optional first token is the level: a bare integer or
        # ``#<digits>`` form.  Anything else and the level defaults to 1.
        head = _arg_text(args[0])
        is_level = head.lstrip("-").isdigit() or (head.startswith("#") and head[1:].isdigit())
        if is_level:
            offset = 1
    else:
        return []

    if offset >= len(args):
        return []

    # Remaining args are pairs: (otherVar, myVar).  The myVar positions
    # are the ones we report; skip any pair where either side looks like
    # a substituted reference (starts with ``$``) since the compiler's
    # aliasing logic does the same.
    result: list[int] = []
    pairs_start = offset
    for i in range(pairs_start, len(args) - 1, 2):
        caller_text = _arg_text(args[i])
        local_text = _arg_text(args[i + 1])
        if caller_text.startswith("$") or local_text.startswith("$"):
            continue
        result.append(i + 1)
    return result
