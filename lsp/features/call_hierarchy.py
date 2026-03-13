"""Call hierarchy provider -- incoming/outgoing calls for procs."""

from __future__ import annotations

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.proc_lookup import find_proc_by_reference
from core.analysis.semantic_model import AnalysisResult, ProcDef, Scope
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
    word = find_word_at_position(source, line, character)
    if not word:
        return None
    proc_match = find_proc_by_reference(analysis, word)
    if proc_match is None:
        return None
    _qname, proc_def = proc_match
    return proc_def


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


def prepare_call_hierarchy(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
) -> list[types.CallHierarchyItem]:
    """Return the call hierarchy item for the proc at the cursor."""
    if analysis is None:
        analysis = analyse(source)

    proc_def = _find_proc_at_position(analysis, line, character, source)
    if proc_def is None:
        return []

    return [_proc_to_item(proc_def, uri)]


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

    # Find the target proc
    target_name = item.name
    target_proc: ProcDef | None = None
    for qname, proc_def in analysis.all_procs.items():
        if proc_def.name == target_name:
            target_proc = proc_def
            break
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

    # Find the source proc
    source_name = item.name
    source_proc: ProcDef | None = None
    for qname, proc_def in analysis.all_procs.items():
        if proc_def.name == source_name:
            source_proc = proc_def
            break
    if source_proc is None:
        return []

    # Find all command invocations within the proc's body range
    br = source_proc.body_range
    calls_by_target: dict[str, list[types.Range]] = {}
    targets: dict[str, ProcDef] = {}

    for inv in analysis.command_invocations:
        # Check if invocation is within proc body
        if inv.range.start.line < br.start.line or inv.range.end.line > br.end.line:
            continue
        # Check if it calls a known proc
        for qname, target in analysis.all_procs.items():
            if (
                target.name == inv.name
                or qname == inv.name
                or (inv.resolved_qualified_name and inv.resolved_qualified_name == qname)
            ):
                calls_by_target.setdefault(qname, []).append(to_lsp_range(inv.range))
                targets[qname] = target
                break

    results: list[types.CallHierarchyOutgoingCall] = []
    for qname, ranges in calls_by_target.items():
        results.append(
            types.CallHierarchyOutgoingCall(
                to=_proc_to_item(targets[qname], uri),
                from_ranges=ranges,
            )
        )

    return results
