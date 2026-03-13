"""Rename provider -- rename procs and variables across a file."""

from __future__ import annotations

import re

from lsprotocol import types

from core.analysis.analyser import analyse
from core.analysis.proc_lookup import find_proc_by_reference
from core.analysis.semantic_model import AnalysisResult, Range, Scope, VarDef
from core.commands.registry import REGISTRY
from core.common.lsp import find_var_in_scopes, to_lsp_range

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

    var_name = _find_assignment_var_name_at_position(source, line, character)
    if var_name:
        var_def = find_var_in_scopes(var_name, scope)
        if var_def is not None:
            return (var_name, var_def, scope)

    return None


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
        return types.PrepareRenamePlaceholder(
            range=to_lsp_range(var_def.definition_range),
            placeholder=var_name,
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

    # Variable rename
    resolved_var = _resolve_variable_symbol(source, line, character, analysis)
    if resolved_var is not None:
        var_name, var_def, scope = resolved_var
        if new_name == var_name:
            return None
        if find_var_in_scopes(new_name, scope) is not None:
            return None

        edits: list[types.TextEdit] = []
        # Definition site
        edits.append(_range_to_text_edit(var_def.definition_range, new_name))
        # Reference sites
        for ref in var_def.references:
            edits.append(_range_to_text_edit(ref, new_name))

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
        # Definition
        edits.append(_range_to_text_edit(proc_def.name_range, new_name))
        # Call sites
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
                edits.append(_range_to_text_edit(r, new_name))

        return types.WorkspaceEdit(changes={uri: edits})

    return None
