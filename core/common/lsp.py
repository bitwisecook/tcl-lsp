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


def find_var_in_scopes(name: str, scope: Scope) -> VarDef | None:
    """Walk up the scope chain to find a variable definition."""
    current: Scope | None = scope
    while current is not None:
        if name in current.variables:
            return current.variables[name]
        current = current.parent
    return None
