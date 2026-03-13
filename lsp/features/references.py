"""Find-references provider -- locate all usages of a proc or variable."""

from __future__ import annotations

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.proc_lookup import find_proc_by_reference
from core.analysis.semantic_model import AnalysisResult, Range
from core.common.lsp import find_var_in_scopes, to_lsp_location

from .symbol_resolution import find_scope_at_line, find_var_at_position, find_word_at_position

# Short names: r = Range.


def find_proc_call_sites(name: str, qualified_name: str, analysis: AnalysisResult) -> list[Range]:
    """Find all locations where a proc is called."""
    qualified_no_prefix = qualified_name[2:] if qualified_name.startswith("::") else qualified_name
    call_forms = {name, qualified_name, qualified_no_prefix}
    locations: list[Range] = []
    seen: set[tuple[int, int, int, int]] = set()
    for invocation in analysis.command_invocations:
        resolved_name = invocation.resolved_qualified_name
        if resolved_name is not None:
            matches_target = resolved_name == qualified_name
        else:
            matches_target = invocation.name in call_forms
        if matches_target:
            key = (
                invocation.range.start.line,
                invocation.range.start.character,
                invocation.range.end.line,
                invocation.range.end.character,
            )
            if key in seen:
                continue
            seen.add(key)
            locations.append(invocation.range)
    return locations


def get_references(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
    include_declaration: bool = True,
) -> list[types.Location]:
    """Find all references to the symbol at the given position."""
    if analysis is None:
        analysis = analyse(source)

    # Check for variable references
    var_name = find_var_at_position(source, line, character)
    if var_name:
        scope = find_scope_at_line(analysis.global_scope, line)
        var_def = find_var_in_scopes(var_name, scope)
        if not var_def:
            return []
        refs = [var_def.definition_range, *var_def.references]
        return [to_lsp_location(uri, r) for r in refs]

    # Check for proc references
    word = find_word_at_position(source, line, character)
    if not word:
        return []

    proc_match = find_proc_by_reference(analysis, word)
    if proc_match is not None:
        _qname, proc_def = proc_match
        locations: list[types.Location] = []

        # Include declaration
        if include_declaration:
            locations.append(to_lsp_location(uri, proc_def.name_range))

        # Find call sites
        call_sites = find_proc_call_sites(proc_def.name, proc_def.qualified_name, analysis)
        for r in call_sites:
            loc = to_lsp_location(uri, r)
            # Avoid duplicating the definition location
            if not any(
                existing.range.start.line == loc.range.start.line
                and existing.range.start.character == loc.range.start.character
                for existing in locations
            ):
                locations.append(loc)

        return locations

    return []
