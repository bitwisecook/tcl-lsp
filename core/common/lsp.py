"""Shared LSP conversion helpers."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from lsprotocol import types

from ..analysis.semantic_model import Range, Scope, VarDef


def _types() -> Any:  # noqa: ANN401
    from lsprotocol import types as _t

    return _t


# Short names: r = Range.


def to_lsp_location(uri: str, r: Range) -> types.Location:
    """Convert an analysis Range to an LSP Location."""
    t = _types()
    return t.Location(
        uri=uri,
        range=t.Range(
            start=t.Position(line=r.start.line, character=r.start.character),
            end=t.Position(line=r.end.line, character=r.end.character + 1),
        ),
    )


def to_lsp_range(r: Range) -> types.Range:
    """Convert an analysis Range to an LSP Range."""
    t = _types()
    return t.Range(
        start=t.Position(line=r.start.line, character=r.start.character),
        end=t.Position(line=r.end.line, character=r.end.character + 1),
    )


def _namespace_path_of(scope: Scope) -> str:
    """Full ``::``-qualified namespace path of *scope* (``::`` at global)."""
    from ..common.naming import normalise_qualified_name

    parts: list[str] = []
    s: Scope | None = scope
    while s is not None:
        if s.kind == "namespace":
            parts.append(s.name)
        s = s.parent
    if not parts:
        return "::"
    parts.reverse()
    ns = "::"
    for p in parts:
        ns = normalise_qualified_name(p if p.startswith("::") else f"{ns}::{p}")
    return ns


def _resolve_qualified_var(name: str, scope: Scope) -> VarDef | None:
    """Resolve a namespace-qualified variable name to its VarDef."""
    from ..common.naming import normalise_qualified_name

    ns_part, _, var_part = name.rpartition("::")
    if not var_part:
        return None
    if ns_part.startswith("::"):
        target = normalise_qualified_name(ns_part)
    elif ns_part == "":
        target = "::"
    else:
        current = _namespace_path_of(scope)
        joined = ns_part if current == "::" else f"{current}::{ns_part}"
        target = normalise_qualified_name(joined if joined.startswith("::") else f"::{joined}")

    root = scope
    while root.parent is not None:
        root = root.parent
    stack = [root]
    while stack:
        candidate = stack.pop()
        if (
            candidate.kind == "namespace"
            and var_part in candidate.variables
            and _namespace_path_of(candidate) == target
        ):
            return candidate.variables[var_part]
        stack.extend(candidate.children)
    return None


def find_var_in_scopes(name: str, scope: Scope) -> VarDef | None:
    """Walk up the scope chain to find a variable definition.

    Falls back to namespace-qualified resolution (``ns::v`` / ``::ns::v`` /
    ``a::b::v``) so go-to-definition and find-references work from a
    qualified read of a ``namespace eval`` variable.
    """
    current: Scope | None = scope
    while current is not None:
        if name in current.variables:
            return current.variables[name]
        current = current.parent
    if "::" in name:
        return _resolve_qualified_var(name, scope)
    return None
