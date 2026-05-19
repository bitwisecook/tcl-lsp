"""Document symbol provider -- outline view, breadcrumbs, Cmd+Shift+O."""

from __future__ import annotations

from lsprotocol import types

from analyser import analyse
from analyser.semantic_model import AnalysisResult, ClassDef, MethodDef, ProcDef, Scope
from shared.lsp import to_lsp_range


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


def _method_detail(method_def: MethodDef) -> str:
    """Format a short parameter list string for a method."""
    parts: list[str] = []
    for p in method_def.params:
        if p.has_default:
            parts.append(f"{{{p.name} {p.default_value}}}")
        else:
            parts.append(p.name)
    return f"({' '.join(parts)})" if parts else "()"


def _class_detail(class_def: ClassDef) -> str:
    """Format a short detail string for a class."""
    parts: list[str] = []
    if class_def.metaclass != "oo::class":
        parts.append(class_def.metaclass)
    if class_def.superclasses:
        parts.append(f": {', '.join(class_def.superclasses)}")
    return " ".join(parts)


def _class_method_symbols(class_def: ClassDef) -> list[types.DocumentSymbol]:
    """Build child symbols for a class: methods, constructors, properties."""
    children: list[types.DocumentSymbol] = []

    for ctor in class_def.constructors:
        ctor_range = to_lsp_range(ctor.body_range)
        children.append(
            types.DocumentSymbol(
                name="constructor",
                kind=types.SymbolKind.Constructor,
                range=ctor_range,
                selection_range=ctor_range,
                detail=_method_detail(ctor),
            )
        )

    if class_def.destructor:
        dtor_range = to_lsp_range(class_def.destructor.body_range)
        children.append(
            types.DocumentSymbol(
                name="destructor",
                kind=types.SymbolKind.Method,
                range=dtor_range,
                selection_range=dtor_range,
            )
        )

    for md in class_def.methods.values():
        body_range = to_lsp_range(md.body_range)
        name_range = to_lsp_range(md.name_range)
        sym_range = _merge_symbol_range(name_range, body_range)
        children.append(
            types.DocumentSymbol(
                name=md.name,
                kind=types.SymbolKind.Method,
                range=sym_range,
                selection_range=name_range,
                detail=_method_detail(md),
            )
        )

    for md in class_def.class_methods.values():
        body_range = to_lsp_range(md.body_range)
        name_range = to_lsp_range(md.name_range)
        sym_range = _merge_symbol_range(name_range, body_range)
        children.append(
            types.DocumentSymbol(
                name=md.name,
                kind=types.SymbolKind.Method,
                range=sym_range,
                selection_range=name_range,
                detail=f"classmethod {_method_detail(md)}",
            )
        )

    for pd in class_def.properties.values():
        prop_range = to_lsp_range(pd.name_range)
        children.append(
            types.DocumentSymbol(
                name=pd.name,
                kind=types.SymbolKind.Property,
                range=prop_range,
                selection_range=prop_range,
            )
        )

    return children


def _scope_symbols(scope: Scope) -> list[types.DocumentSymbol]:
    """Recursively collect symbols from a scope and its children."""
    symbols: list[types.DocumentSymbol] = []

    # Classes defined in this scope
    for class_def in scope.classes.values():
        body_range = to_lsp_range(class_def.body_range)
        name_range = to_lsp_range(class_def.name_range)
        symbol_range = _merge_symbol_range(name_range, body_range)
        child_symbols = _class_method_symbols(class_def)
        symbols.append(
            types.DocumentSymbol(
                name=class_def.name,
                kind=types.SymbolKind.Class,
                range=symbol_range,
                selection_range=name_range,
                detail=_class_detail(class_def),
                children=child_symbols or None,
            )
        )

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


def _symbols_from_chunks(
    chunks: list,
) -> list[types.DocumentSymbol]:
    """Build basic symbols from pre-segmented chunks (no analysis needed).

    Returns event handlers (``when EVENT``) and procedures (``proc NAME``)
    extracted from the command text in each chunk.  This gives the editor
    a useful outline immediately while the full analysis runs in the
    subprocess.
    """
    symbols: list[types.DocumentSymbol] = []
    irules_events = frozenset(
        {
            "when",
            "priority",
        }
    )
    for chunk in chunks:
        for cmd in chunk.commands:
            name = cmd.name
            args = cmd.args
            r = to_lsp_range(cmd.range)
            if name in irules_events and args:
                symbols.append(
                    types.DocumentSymbol(
                        name=args[0],
                        kind=types.SymbolKind.Event,
                        range=r,
                        selection_range=r,
                    )
                )
            elif name == "proc" and args:
                detail = ""
                if len(args) >= 2:
                    detail = f"({args[1]})" if args[1] else "()"
                symbols.append(
                    types.DocumentSymbol(
                        name=args[0],
                        kind=types.SymbolKind.Function,
                        range=r,
                        selection_range=r,
                        detail=detail,
                    )
                )
            elif name == "namespace" and args and args[0] == "eval" and len(args) >= 2:
                symbols.append(
                    types.DocumentSymbol(
                        name=args[1],
                        kind=types.SymbolKind.Namespace,
                        range=r,
                        selection_range=r,
                    )
                )
            elif (
                name in ("oo::class", "oo::configurable", "oo::abstract")
                and args
                and args[0] == "create"
                and len(args) >= 2
            ):
                symbols.append(
                    types.DocumentSymbol(
                        name=args[1],
                        kind=types.SymbolKind.Class,
                        range=r,
                        selection_range=r,
                    )
                )
    return symbols


def get_document_symbols(
    source: str,
    analysis: AnalysisResult | None = None,
    chunks: list | None = None,
    embedded_rules: list | None = None,
) -> list[types.DocumentSymbol]:
    """Return the document symbol hierarchy for a Tcl source file.

    When *analysis* is available, returns the full symbol tree from the
    scope hierarchy.  When only *chunks* are available (analysis not yet
    complete), returns basic event/proc symbols extracted from command
    text — enough for a useful outline while the subprocess runs.

    When *embedded_rules* is provided (conf-wrapped iRules mode), each
    rule stanza becomes a top-level Module symbol containing the
    per-body scope symbols.
    """
    if analysis is not None:
        if embedded_rules:
            return _conf_wrapped_symbols(analysis, embedded_rules)
        return _scope_symbols(analysis.global_scope)

    # Fast path: build symbols from chunks without running analysis.
    if chunks is not None:
        return _symbols_from_chunks(chunks)

    # Fallback: run analysis synchronously (legacy path).
    analysis = analyse(source)
    return _scope_symbols(analysis.global_scope)


def _conf_wrapped_symbols(
    analysis: AnalysisResult,
    embedded_rules: list,
) -> list[types.DocumentSymbol]:
    """Build document symbols for conf-wrapped iRules.

    Each ``ltm rule /path`` becomes a Module symbol containing the
    scope symbols from its rule body.
    """
    rule_symbols: list[types.DocumentSymbol] = []
    children = analysis.global_scope.children

    for i, rule in enumerate(embedded_rules):
        rule_range = to_lsp_range(rule.range)
        # Get child scope symbols if available.
        child_symbols: list[types.DocumentSymbol] = []
        if i < len(children):
            child_symbols = _scope_symbols(children[i])

        rule_sym = types.DocumentSymbol(
            name=rule.full_path,
            detail=rule.header,
            kind=types.SymbolKind.Module,
            range=rule_range,
            selection_range=rule_range,
            children=child_symbols,
        )
        rule_symbols.append(rule_sym)
    return rule_symbols
