"""Find-references provider -- locate all usages of a proc or variable."""

from __future__ import annotations

from lsprotocol import types

from analyser import analyse
from analyser.proc_lookup import find_proc_by_reference
from analyser.semantic_model import AnalysisResult, Range
from server._lsp_conv import find_var_in_scopes, to_lsp_location

from .symbol_resolution import find_scope_at_line, find_var_at_position, find_word_at_position

# Short names: r = Range.


def _bare_call_target(name: str, call_ns: str, proc_qnames: frozenset[str]) -> str | None:
    """Resolve an unqualified call the way Tcl would — current namespace then global.

    *call_ns* is the ``::``-qualified namespace the call sits in. Returns the
    qualified name of the proc the bare ``name`` resolves to, or ``None`` when
    no such proc exists. Mirrors command lookup: a name is sought in the
    enclosing namespace first, then falls back to the global namespace.
    """
    if call_ns and call_ns != "::":
        candidate = f"{call_ns}::{name}"
        if candidate in proc_qnames:
            return candidate
    candidate = f"::{name}"
    return candidate if candidate in proc_qnames else None


def find_proc_call_sites(name: str, qualified_name: str, analysis: AnalysisResult) -> list[Range]:
    """Find all locations where a proc is called."""
    qualified_no_prefix = qualified_name[2:] if qualified_name.startswith("::") else qualified_name
    call_forms = {name, qualified_name, qualified_no_prefix}
    proc_qnames = frozenset(analysis.all_procs)
    # Namespace disambiguation only kicks in when the simple name is genuinely
    # ambiguous *within this analysis* — i.e. two or more procs share it (e.g.
    # ``::a::foo`` and ``::b::foo``). When it is unique (or absent, as for a
    # cross-file call whose proc lives in another document), plain name-form
    # matching is both correct and necessary: requiring the proc to exist
    # locally would drop legitimate cross-file references (#637).
    name_is_ambiguous = sum(1 for proc in analysis.all_procs.values() if proc.name == name) >= 2
    locations: list[Range] = []
    seen: set[tuple[int, int, int, int]] = set()
    for invocation in analysis.command_invocations:
        resolved_name = invocation.resolved_qualified_name
        if resolved_name is not None:
            matches_target = resolved_name == qualified_name
        elif "::" in invocation.name:
            # Qualified but unresolved (forward / cross-file): the written form
            # already names the namespace, so match it exactly.
            matches_target = invocation.name in call_forms
        elif name_is_ambiguous and invocation.name == name:
            # Bare unresolved call to an ambiguous name: resolve it the way Tcl
            # would (current namespace, then global) so it is credited only to
            # the proc it actually targets — not every same-named proc (#644).
            call_ns = invocation.enclosing_namespace or "::"
            matches_target = (
                _bare_call_target(invocation.name, call_ns, proc_qnames) == qualified_name
            )
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

    # Check for class/method references
    word = find_word_at_position(source, line, character)
    if not word:
        return []

    # Check for class name references
    for _qname, class_def in analysis.all_classes.items():
        if (
            class_def.name == word
            or class_def.qualified_name == word
            or class_def.qualified_name == f"::{word}"
        ):
            locations: list[types.Location] = []
            if include_declaration:
                locations.append(to_lsp_location(uri, class_def.name_range))

            # Superclass / mixin references to this class from *other* classes,
            # using the precise token range captured by the analyser.
            target_names = {class_def.name, class_def.qualified_name, f"::{class_def.name}"}
            for other in analysis.all_classes.values():
                for ref_name, ref_range in (*other.superclass_refs, *other.mixin_refs):
                    if ref_name in target_names:
                        locations.append(to_lsp_location(uri, ref_range))

            # Find references in command invocations
            for invocation in analysis.command_invocations:
                if invocation.name == class_def.name or invocation.name == class_def.qualified_name:
                    loc = to_lsp_location(uri, invocation.range)
                    if not any(
                        existing.range.start.line == loc.range.start.line
                        and existing.range.start.character == loc.range.start.character
                        for existing in locations
                    ):
                        locations.append(loc)
            return locations

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
