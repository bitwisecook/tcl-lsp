"""Rename provider -- rename procs and variables across a file."""

from __future__ import annotations

import re

from lsprotocol import types

from core.analysis import analyse
from core.analysis.proc_lookup import find_proc_by_reference
from core.analysis.semantic_model import AnalysisResult, Range, Scope, VarDef
from core.commands.registry import REGISTRY
from core.common.lsp import find_var_in_scopes, to_lsp_range
from core.parsing.tokens import SourcePosition

from .references import find_proc_call_sites
from .symbol_resolution import (
    find_command_context_details_at_position,
    find_scope_at_line,
    find_var_at_position,
    find_word_at_position,
    find_word_span_at_position,
)

_SAFE_SYMBOL_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


def _range_to_text_edit(r: Range, new_text: str) -> types.TextEdit:
    return types.TextEdit(range=to_lsp_range(r), new_text=new_text)


def _is_safe_symbol_name(name: str) -> bool:
    return bool(_SAFE_SYMBOL_RE.fullmatch(name))


def _is_builtin_command_name(name: str) -> bool:
    return name in set(REGISTRY.command_names())


def _tail_name(qualified: str) -> str:
    """Return the last ``::``-separated segment of a qualified name."""
    idx = qualified.rfind("::")
    return qualified[idx + 2 :] if idx >= 0 else qualified


def _namespace_prefix(qualified: str) -> str:
    """Return everything up to and including the last ``::``."""
    idx = qualified.rfind("::")
    return qualified[: idx + 2] if idx >= 0 else ""


def _find_var_in_all_variables(
    name: str,
    analysis: AnalysisResult,
) -> VarDef | None:
    """Look up a variable by its qualified name in the flat all_variables dict.

    The dict uses several key patterns depending on context:
    ``"::::ns::var"``, ``"ns::var"``, etc.  Try the most common forms.
    """
    candidates = [name, f"::::{name}", f"::{name}"]
    for key in candidates:
        if key in analysis.all_variables:
            return analysis.all_variables[key]
    # Brute-force: match by tail if the exact key wasn't found
    for key, vdef in analysis.all_variables.items():
        stripped = key.lstrip(":")
        if stripped == name or stripped == name.lstrip(":"):
            return vdef
    return None


def _find_assignment_var_name_at_position(
    source: str,
    line: int,
    character: int,
) -> str | None:
    """Return variable name when cursor is on assignment var-name argument."""
    span = find_word_span_at_position(source, line, character)
    if span is None:
        return None
    word, start_col, _end_col = span
    if not word:
        return None

    # Probe from inside the detected word so word_index resolves consistently
    # even when the cursor is at the token boundary.
    probe_col = start_col + 1
    cmd, _args, _current_word, word_index = find_command_context_details_at_position(
        source,
        line,
        probe_col,
    )
    spec = REGISTRY.get_any(cmd) if cmd else None
    if (
        spec is not None
        and spec.assigns_variable_at is not None
        and word_index == spec.assigns_variable_at + 1
    ):
        return word
    return None


def _resolve_variable_symbol(
    source: str,
    line: int,
    character: int,
    analysis: AnalysisResult,
) -> tuple[str, VarDef, Scope] | None:
    """Resolve variable symbol under cursor from reference or assignment site."""
    scope = find_scope_at_line(analysis.global_scope, line)

    var_name = find_var_at_position(source, line, character)
    if var_name:
        var_def = find_var_in_scopes(var_name, scope)
        if var_def is not None:
            return (var_name, var_def, scope)
        # Fallback: qualified names stored in all_variables
        var_def = _find_var_in_all_variables(var_name, analysis)
        if var_def is not None:
            return (var_name, var_def, scope)

    var_name = _find_assignment_var_name_at_position(source, line, character)
    if var_name:
        var_def = find_var_in_scopes(var_name, scope)
        if var_def is not None:
            return (var_name, var_def, scope)
        # Fallback: qualified names stored in all_variables
        var_def = _find_var_in_all_variables(var_name, analysis)
        if var_def is not None:
            return (var_name, var_def, scope)

    return None


def _array_index_suffix(body: str) -> str:
    """Return a trailing array-index suffix like ``(a)`` from a var reference body.

    A reference to an array element such as ``$arr(a)`` carries the whole
    ``arr(a)`` span, but renaming the base variable must rewrite only the name
    and keep the index intact.
    """
    paren = body.find("(")
    return body[paren:] if paren >= 0 else ""


def _build_var_replacement(
    ref_line: str,
    ref: Range,
    ns_prefix: str,
    new_tail: str,
) -> tuple[Range, str]:
    """Build the replacement text and (possibly adjusted) range for a variable reference.

    Handles ``$ns::var``, ``${ns::var}``, and bare ``ns::var`` forms,
    preserving the namespace prefix and ``$``/``${...}`` delimiters.
    """
    ch = ref.start.character
    has_dollar = ch < len(ref_line) and ref_line[ch] == "$"
    has_brace = has_dollar and ch + 1 < len(ref_line) and ref_line[ch + 1] == "{"

    new_qualified = ns_prefix + new_tail
    end = ref.end

    if has_brace:
        replacement = "${" + new_qualified + "}"
        # Extend range to cover closing brace
        close_ch = end.character + 1
        if close_ch < len(ref_line) and ref_line[close_ch] == "}":
            end = SourcePosition(
                line=end.line,
                character=close_ch,
                offset=end.offset + 1,
            )
    elif has_dollar:
        # Preserve a trailing array-index suffix, e.g. $arr(a) -> $new(a).
        body = ref_line[ch + 1 : end.character + 1]
        suffix = _array_index_suffix(body)
        replacement = "$" + new_qualified + suffix
    else:
        suffix = _array_index_suffix(ref_line[ch : end.character + 1])
        replacement = new_qualified + suffix

    return Range(start=ref.start, end=end), replacement


def _build_proc_call_replacement(
    source_lines: list[str],
    call_range: Range,
    new_tail: str,
) -> tuple[Range, str]:
    """Build replacement for a proc call site, preserving namespace qualifier.

    For ``utils::helper``, replaces only ``helper`` → ``assist``,
    producing ``utils::assist``.
    """
    line_text = source_lines[call_range.start.line]
    call_text = line_text[call_range.start.character : call_range.end.character + 1]
    last_sep = call_text.rfind("::")
    if last_sep >= 0:
        # Qualified call — replace only the tail
        prefix = call_text[: last_sep + 2]
        replacement = prefix + new_tail
    else:
        replacement = new_tail
    return call_range, replacement


def prepare_rename(
    source: str,
    uri: str,
    line: int,
    character: int,
    analysis: AnalysisResult | None = None,
) -> types.PrepareRenamePlaceholder | None:
    """Validate that the cursor is on a renameable symbol.

    Returns a range + placeholder, or None if the symbol cannot be renamed.
    """
    if analysis is None:
        analysis = analyse(source)

    # Variable?
    resolved_var = _resolve_variable_symbol(source, line, character, analysis)
    if resolved_var is not None:
        var_name, var_def, _scope = resolved_var
        tail = _tail_name(var_name)
        return types.PrepareRenamePlaceholder(
            range=to_lsp_range(var_def.definition_range),
            placeholder=tail,
        )

    # Proc?
    span = find_word_span_at_position(source, line, character)
    if span is None:
        return None
    word, _start_col, _end_col = span

    proc_match = find_proc_by_reference(analysis, word)
    if proc_match is not None:
        _qname, proc_def = proc_match
        return types.PrepareRenamePlaceholder(
            range=to_lsp_range(proc_def.name_range),
            placeholder=proc_def.name,
        )

    return None


def get_rename_edits(
    source: str,
    uri: str,
    line: int,
    character: int,
    new_name: str,
    analysis: AnalysisResult | None = None,
) -> types.WorkspaceEdit | None:
    """Compute all text edits for renaming the symbol at the cursor."""
    if analysis is None:
        analysis = analyse(source)
    if not _is_safe_symbol_name(new_name):
        return None

    lines = source.splitlines()

    # Variable rename
    resolved_var = _resolve_variable_symbol(source, line, character, analysis)
    if resolved_var is not None:
        var_name, var_def, scope = resolved_var
        old_tail = _tail_name(var_name)
        ns_prefix = _namespace_prefix(var_name)
        if new_name == old_tail:
            return None
        # Check for collision with new qualified name
        new_qualified = ns_prefix + new_name
        if not ns_prefix and find_var_in_scopes(new_name, scope) is not None:
            return None

        edits: list[types.TextEdit] = []
        # Definition site — range covers the qualified name (no $)
        def_range = var_def.definition_range
        if def_range.start.line < len(lines):
            def_line = lines[def_range.start.line]
            def_text = def_line[def_range.start.character : def_range.end.character + 1]
            # Replace only the tail in the definition
            last_sep = def_text.rfind("::")
            if last_sep >= 0:
                def_replacement = def_text[: last_sep + 2] + new_name
            else:
                def_replacement = new_name
            edits.append(_range_to_text_edit(def_range, def_replacement))
        else:
            edits.append(_range_to_text_edit(def_range, new_qualified))

        # Reference sites — may include $ or ${...} with namespace qualifier
        for ref in var_def.references:
            if ref.start.line < len(lines):
                ref_line = lines[ref.start.line]
                adj_range, replacement = _build_var_replacement(ref_line, ref, ns_prefix, new_name)
                edits.append(_range_to_text_edit(adj_range, replacement))
            else:
                edits.append(_range_to_text_edit(ref, new_qualified))

        return types.WorkspaceEdit(changes={uri: edits})

    # Proc rename
    word = find_word_at_position(source, line, character)
    if not word:
        return None

    proc_match = find_proc_by_reference(analysis, word)
    if proc_match is not None:
        qname, proc_def = proc_match
        if new_name == proc_def.name:
            return None
        if _is_builtin_command_name(new_name):
            return None
        for other_qname, other_proc in analysis.all_procs.items():
            if other_qname == qname:
                continue
            if other_proc.name == new_name:
                return None
        edits = []
        # Definition — name_range covers just the tail name
        edits.append(_range_to_text_edit(proc_def.name_range, new_name))
        # Call sites — may include namespace qualifier
        call_sites = find_proc_call_sites(
            proc_def.name,
            proc_def.qualified_name,
            analysis,
        )
        seen: set[tuple[int, int, int, int]] = set()
        # Mark definition as seen to avoid duplicate
        seen.add(
            (
                proc_def.name_range.start.line,
                proc_def.name_range.start.character,
                proc_def.name_range.end.line,
                proc_def.name_range.end.character,
            )
        )
        for r in call_sites:
            key = (r.start.line, r.start.character, r.end.line, r.end.character)
            if key not in seen:
                seen.add(key)
                adj_range, replacement = _build_proc_call_replacement(lines, r, new_name)
                edits.append(_range_to_text_edit(adj_range, replacement))

        return types.WorkspaceEdit(changes={uri: edits})

    return None
