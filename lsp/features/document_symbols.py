"""Document symbol provider -- outline view, breadcrumbs, Cmd+Shift+O."""

from __future__ import annotations

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.semantic_model import AnalysisResult, ProcDef, Scope
from core.common.lsp import to_lsp_range


def _proc_detail(proc_def: ProcDef) -> str:
    """Format a short parameter list string for the detail field."""
    parts: list[str] = []
    for p in proc_def.params:
        if p.has_default:
            parts.append(f"{{{p.name} {p.default_value}}}")
        else:
            parts.append(p.name)
    return f"({' '.join(parts)})" if parts else "()"


def _pos_leq(left: types.Position, right: types.Position) -> bool:
    """Return True if left <= right in document order."""
    if left.line != right.line:
        return left.line < right.line
    return left.character <= right.character


def _merge_symbol_range(first: types.Range, second: types.Range) -> types.Range:
    """Return an outer range that contains both child ranges."""
    start = first.start if _pos_leq(first.start, second.start) else second.start
    end = first.end if _pos_leq(second.end, first.end) else second.end
    return types.Range(start=start, end=end)


def _scope_symbols(scope: Scope) -> list[types.DocumentSymbol]:
    """Recursively collect symbols from a scope and its children."""
    symbols: list[types.DocumentSymbol] = []

    # Procs defined in this scope
    for proc_def in scope.procs.values():
        # Collect any nested symbols from the proc's child scope
        child_symbols: list[types.DocumentSymbol] = []
        for child in scope.children:
            if child.kind == "proc" and child.name == proc_def.name:
                child_symbols = _scope_symbols(child)
                break

        body_range = to_lsp_range(proc_def.body_range)
        name_range = to_lsp_range(proc_def.name_range)
        symbol_range = _merge_symbol_range(name_range, body_range)
        symbols.append(
            types.DocumentSymbol(
                name=proc_def.name,
                kind=types.SymbolKind.Function,
                range=symbol_range,
                selection_range=name_range,
                detail=_proc_detail(proc_def),
                children=child_symbols or None,
            )
        )

    # Global/namespace-level variables
    if scope.kind in ("global", "namespace"):
        for var_def in scope.variables.values():
            var_range = to_lsp_range(var_def.definition_range)
            symbols.append(
                types.DocumentSymbol(
                    name=var_def.name,
                    kind=types.SymbolKind.Variable,
                    range=var_range,
                    selection_range=var_range,
                )
            )

    # Child namespace scopes
    for child in scope.children:
        if child.kind == "namespace" and child.body_range is not None:
            ns_range = to_lsp_range(child.body_range)
            child_syms = _scope_symbols(child)
            symbols.append(
                types.DocumentSymbol(
                    name=child.name,
                    kind=types.SymbolKind.Namespace,
                    range=ns_range,
                    selection_range=ns_range,
                    children=child_syms or None,
                )
            )

    return symbols


def get_document_symbols(
    source: str,
    analysis: AnalysisResult | None = None,
) -> list[types.DocumentSymbol]:
    """Return the document symbol hierarchy for a Tcl source file."""
    if analysis is None:
        analysis = analyse(source)

    return _scope_symbols(analysis.global_scope)
