"""Go-to-declaration provider.

Uses the parser (:func:`segment_commands`) to walk ``variable`` /
``global`` / ``upvar`` statements that are visible at the cursor's
scope, and returns the source token range for the declared variable.

Scoping works like this: the analyser's lexical scope tree
(:func:`find_scope_at_line`) identifies the proc/namespace enclosing the
cursor.  We then walk the tokens of the enclosing scope's body range
(plus every enclosing scope up to the global one) so declarations in an
unrelated proc never leak into the result.

No regex: the command segmenter already produces proper tokens with
source ranges.  Falls back to :func:`get_definition` when no explicit
declaration is found (typical for proc names, class names, and plain
``set`` variables).
"""

from __future__ import annotations

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, Range, Scope
from core.analysis.var_scoping import (
    global_declaration_indices,
    upvar_local_declaration_indices,
    variable_declaration_indices,
)
from core.commands.registry.runtime import iter_body_arguments
from core.common.lsp import to_lsp_location
from core.parsing.command_segmenter import segment_commands
from core.parsing.tokens import Token

from .definition import get_definition
from .symbol_resolution import find_scope_at_line, find_var_at_position


def _stripped(name: str) -> str:
    """Normalise a variable token for comparison (drop leading ``$`` / ``::``)."""
    return name.lstrip("$").lstrip(":")


def _declaration_tokens(
    cmd_name: str,
    arg_tokens: list[Token],
) -> list[Token]:
    """Return the tokens declared by a ``variable``/``global``/``upvar`` command.

    Delegates to :mod:`core.analysis.var_scoping` so the LSP provider and
    the compiler's memory-SSA alias detector share a single source of
    truth for the upvar/global/variable grammar.
    """
    if cmd_name == "global":
        indices = global_declaration_indices(arg_tokens)
    elif cmd_name == "variable":
        indices = variable_declaration_indices(arg_tokens)
    elif cmd_name == "upvar":
        indices = upvar_local_declaration_indices("upvar", arg_tokens)
    else:
        return []
    return [arg_tokens[i] for i in indices]


def _range_contains(outer: Range, inner: Range) -> bool:
    """True when ``inner`` lies entirely inside ``outer``."""
    if inner.start.line < outer.start.line:
        return False
    if inner.start.line == outer.start.line and inner.start.character < outer.start.character:
        return False
    if inner.end.line > outer.end.line:
        return False
    if inner.end.line == outer.end.line and inner.end.character > outer.end.character:
        return False
    return True


def _collect_scope_ranges(scope: Scope) -> list[Range]:
    """Return every body range from ``scope`` outward to the global scope.

    The global scope has no ``body_range``; we treat it as visible always
    by returning an empty list of narrowing ranges, which the caller
    interprets as "walk the whole file".
    """
    ranges: list[Range] = []
    cursor: Scope | None = scope
    while cursor is not None:
        if cursor.body_range is not None:
            ranges.append(cursor.body_range)
        cursor = cursor.parent
    return ranges


def _collect_declaration_ranges(
    source: str,
    var_name: str,
    visible_ranges: list[Range] | None = None,
) -> list[Range]:
    """Walk *source* for declaration statements naming ``var_name``.

    When ``visible_ranges`` is non-empty the result is filtered to tokens
    that lie inside at least one of those ranges, so the cursor's scope
    is honoured.  When empty (i.e. global scope) every declaration in
    the file is considered.

    Recursion into body arguments (proc bodies, namespace eval blocks,
    class bodies, etc.) is driven by
    :func:`core.commands.registry.runtime.iter_body_arguments`.
    """
    target = _stripped(var_name)
    ranges: list[Range] = []
    seen: set[tuple[int, int, int, int]] = set()

    def _visible(tok_range: Range) -> bool:
        if not visible_ranges:
            return True
        return any(_range_contains(r, tok_range) for r in visible_ranges)

    def _walk(text: str, body_token: Token | None, depth: int = 0) -> None:
        if depth > 20:  # guard against pathological nesting
            return
        for cmd in segment_commands(text, body_token):
            if cmd.is_partial or not cmd.texts:
                continue
            name = cmd.name
            if name in ("global", "variable", "upvar"):
                for tok in _declaration_tokens(name, cmd.arg_tokens):
                    tok_range = Range(start=tok.start, end=tok.end)
                    if _stripped(tok.text) != target:
                        continue
                    if not _visible(tok_range):
                        continue
                    key = (
                        tok_range.start.line,
                        tok_range.start.character,
                        tok_range.end.line,
                        tok_range.end.character,
                    )
                    if key not in seen:
                        seen.add(key)
                        ranges.append(tok_range)
            # Recurse into body arguments (proc bodies, namespace eval, etc.).
            for body in iter_body_arguments(cmd.name, cmd.args, cmd.arg_tokens):
                _walk(body.text, body.token, depth + 1)

    _walk(source, None)
    return ranges


def get_declaration(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
) -> list[types.Location]:
    """Return declaration locations for the symbol at ``(line, character)``."""
    if analysis is None:
        analysis = analyse(source)

    var_name = find_var_at_position(source, line, character)
    if var_name:
        scope = find_scope_at_line(analysis.global_scope, line)
        visible = _collect_scope_ranges(scope)
        ranges = _collect_declaration_ranges(source, var_name, visible)
        if ranges:
            return [to_lsp_location(uri, r) for r in ranges]
    # Fall back to regular definition for procs / classes / variables without
    # an explicit declaration.
    return get_definition(source, uri, line, character, analysis=analysis)
