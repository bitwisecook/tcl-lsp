"""Call hierarchy provider -- incoming/outgoing calls for procs."""

from __future__ import annotations

from lsprotocol import types

from core.analysis import analyse
from core.analysis.proc_lookup import find_proc_by_reference
from core.analysis.semantic_model import AnalysisResult, ClassDef, MethodDef, ProcDef, Scope
from core.common.lsp import to_lsp_range

from .references import find_proc_call_sites
from .symbol_resolution import find_word_at_position


def _find_proc_at_position(
    analysis: AnalysisResult,
    line: int,
    character: int,
    source: str,
) -> ProcDef | None:
    """Find the proc definition at the cursor position."""
    # Prefer the proc whose definition-name range contains the cursor; this
    # disambiguates same-named procs in different namespaces (a bare-name
    # lookup would always return the first).
    for proc_def in analysis.all_procs.values():
        nr = proc_def.name_range
        if nr.start.line == nr.end.line:
            if nr.start.line == line and nr.start.character <= character <= nr.end.character + 1:
                return proc_def
        elif nr.start.line <= line <= nr.end.line:
            return proc_def
    # Fallback: the cursor is on a call site, not a definition.
    word = find_word_at_position(source, line, character)
    if not word:
        return None
    proc_match = find_proc_by_reference(analysis, word)
    if proc_match is None:
        return None
    _qname, proc_def = proc_match
    return proc_def


def _target_proc_for_item(
    item: types.CallHierarchyItem, analysis: AnalysisResult
) -> ProcDef | None:
    """Resolve the exact proc an item refers to.

    Prefers the item's ``detail`` (the proc's qualified name) so that
    same-named procs in different namespaces are distinguished; falls back to
    a bare-name match for items that carry no qualified detail.
    """
    if item.detail:
        for proc_def in analysis.all_procs.values():
            if proc_def.qualified_name == item.detail:
                return proc_def
    for proc_def in analysis.all_procs.values():
        if proc_def.name == item.name:
            return proc_def
    return None


def _proc_to_item(proc_def: ProcDef, uri: str) -> types.CallHierarchyItem:
    """Convert a ProcDef to a CallHierarchyItem."""
    return types.CallHierarchyItem(
        name=proc_def.name,
        kind=types.SymbolKind.Function,
        uri=uri,
        range=to_lsp_range(proc_def.body_range),
        selection_range=to_lsp_range(proc_def.name_range),
        detail=proc_def.qualified_name,
    )


def _method_to_item(
    method_def: MethodDef, class_def: ClassDef, uri: str
) -> types.CallHierarchyItem:
    """Convert a MethodDef to a CallHierarchyItem."""
    return types.CallHierarchyItem(
        name=method_def.name,
        kind=types.SymbolKind.Method,
        uri=uri,
        range=to_lsp_range(method_def.body_range),
        selection_range=to_lsp_range(method_def.name_range),
        detail=f"{class_def.qualified_name}::{method_def.name}",
    )


def _find_method_at_position(
    analysis: AnalysisResult,
    line: int,
    character: int,
    source: str,
) -> tuple[MethodDef, ClassDef] | None:
    """Find the method definition at the cursor position."""
    word = find_word_at_position(source, line, character)
    if not word:
        return None
    for _qname, class_def in analysis.all_classes.items():
        if word in class_def.methods:
            return (class_def.methods[word], class_def)
        if word in class_def.class_methods:
            return (class_def.class_methods[word], class_def)
    return None


def prepare_call_hierarchy(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
) -> list[types.CallHierarchyItem]:
    """Return the call hierarchy item for the proc or method at the cursor."""
    if analysis is None:
        analysis = analyse(source)

    proc_def = _find_proc_at_position(analysis, line, character, source)
    if proc_def is not None:
        return [_proc_to_item(proc_def, uri)]

    method_match = _find_method_at_position(analysis, line, character, source)
    if method_match is not None:
        method_def, class_def = method_match
        return [_method_to_item(method_def, class_def, uri)]

    return []


def _find_containing_proc(
    scope: Scope,
    line: int,
) -> ProcDef | None:
    """Find the proc whose body contains the given line."""
    for proc_def in scope.procs.values():
        br = proc_def.body_range
        if br.start.line <= line <= br.end.line:
            # Check children for a more specific match
            for child in scope.children:
                if child.kind == "proc" and child.name == proc_def.name:
                    inner = _find_containing_proc(child, line)
                    if inner is not None:
                        return inner
            return proc_def
    for child in scope.children:
        if child.body_range and child.body_range.start.line <= line <= child.body_range.end.line:
            result = _find_containing_proc(child, line)
            if result is not None:
                return result
    return None


def incoming_calls(
    item: types.CallHierarchyItem,
    source: str,
    uri: str,
    analysis: AnalysisResult | None = None,
) -> list[types.CallHierarchyIncomingCall]:
    """Find all callers of the given proc."""
    if analysis is None:
        analysis = analyse(source)

    # Find the target proc.  Match the item's qualified name (carried in
    # ``detail``) first so that same-named procs in sibling namespaces are
    # disambiguated — matching by bare name would always pick the first.
    target_proc = _target_proc_for_item(item, analysis)
    if target_proc is None:
        return []

    # Find all call sites
    call_sites = find_proc_call_sites(
        target_proc.name,
        target_proc.qualified_name,
        analysis,
    )

    # Group by containing proc
    calls_by_proc: dict[str, list[types.Range]] = {}
    procs_by_name: dict[str, ProcDef] = {}

    for r in call_sites:
        containing = _find_containing_proc(analysis.global_scope, r.start.line)
        if containing is not None:
            key = containing.qualified_name
            procs_by_name[key] = containing
        else:
            key = "<top-level>"
        calls_by_proc.setdefault(key, []).append(to_lsp_range(r))

    results: list[types.CallHierarchyIncomingCall] = []
    for key, ranges in calls_by_proc.items():
        if key == "<top-level>":
            # Top-level calls: create a synthetic item
            caller = types.CallHierarchyItem(
                name="<top-level>",
                kind=types.SymbolKind.Module,
                uri=uri,
                range=types.Range(
                    start=types.Position(line=0, character=0),
                    end=types.Position(line=0, character=0),
                ),
                selection_range=types.Range(
                    start=types.Position(line=0, character=0),
                    end=types.Position(line=0, character=0),
                ),
            )
        else:
            caller = _proc_to_item(procs_by_name[key], uri)
        results.append(
            types.CallHierarchyIncomingCall(
                from_=caller,
                from_ranges=ranges,
            )
        )

    return results


def outgoing_calls(
    item: types.CallHierarchyItem,
    source: str,
    uri: str,
    analysis: AnalysisResult | None = None,
) -> list[types.CallHierarchyOutgoingCall]:
    """Find all procs called by the given proc."""
    if analysis is None:
        analysis = analyse(source)

    # Find the source proc (disambiguated by qualified name — see incoming).
    source_proc = _target_proc_for_item(item, analysis)
    if source_proc is None:
        return []

    # Find all command invocations within the proc's body range
    br = source_proc.body_range
    calls_by_target: dict[str, list[types.Range]] = {}
    targets: dict[str, ProcDef] = {}

    # Index procs once so resolving each invocation is O(1) instead of a
    # nested scan (which made the whole pass O(invocations × procs)).  The
    # original matched the *first* proc, in ``all_procs`` insertion order,
    # whose bare name, qualified name, or the invocation's resolved name
    # matched; we reproduce that by ranking candidates on insertion position.
    pos = {qname: i for i, qname in enumerate(analysis.all_procs)}
    first_qname_by_name: dict[str, str] = {}
    for qname, target in analysis.all_procs.items():
        first_qname_by_name.setdefault(target.name, qname)

    for inv in analysis.command_invocations:
        # Check if invocation is within proc body
        if inv.range.start.line < br.start.line or inv.range.end.line > br.end.line:
            continue
        # Candidate qnames matching the original three conditions; pick the one
        # that appears first in ``all_procs`` (matching the old break-on-first).
        candidates = [
            first_qname_by_name.get(inv.name),
            inv.name if inv.name in pos else None,
            inv.resolved_qualified_name if inv.resolved_qualified_name in pos else None,
        ]
        matched = [q for q in candidates if q is not None]
        if not matched:
            continue
        qname = min(matched, key=lambda q: pos[q])
        calls_by_target.setdefault(qname, []).append(to_lsp_range(inv.range))
        targets[qname] = analysis.all_procs[qname]

    results: list[types.CallHierarchyOutgoingCall] = []
    for qname, ranges in calls_by_target.items():
        results.append(
            types.CallHierarchyOutgoingCall(
                to=_proc_to_item(targets[qname], uri),
                from_ranges=ranges,
            )
        )

    return results
