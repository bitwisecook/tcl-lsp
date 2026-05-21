"""Public entry point for taint analysis."""

from __future__ import annotations

import logging
from dataclasses import replace

from ...analysis.semantic_model import Range
from ...common.document_buffer import DocumentBuffer
from ...common.naming import normalise_var_name
from ...parsing.command_segmenter import segment_commands
from ...parsing.tokens import TokenType
from ..compilation_unit import CompilationUnit, ensure_compilation_unit
from ._interprocedural import _solve_interprocedural_taints
from ._path_concat import _find_path_concat_warnings
from ._sinks import (
    _find_destructive_file_warnings,
    _find_setter_constraint_violations,
    _find_taint_sinks,
)
from ._types import TaintWarning
from ._uri_split import _find_uri_split_suggestions

log = logging.getLogger(__name__)


def _narrow_to_tainted_variable(
    source: str, buffer: DocumentBuffer, warning: TaintWarning
) -> TaintWarning:
    """Narrow a taint warning's range to the tainted ``$var`` reference.

    Many sinks report the whole statement range; the offending span is the
    tainted variable.  Re-segment the statement span, find the ``$var`` token
    naming the warning's variable, and tighten the range to it.  Only narrows
    when a strictly-smaller span is found, so warnings that already point at a
    precise location are left unchanged.
    """
    variable = warning.variable
    if not variable:
        return warning
    r = warning.range
    base = r.start.offset
    seg = source[base : r.end.offset + 1]
    target = normalise_var_name(variable)
    for cmd in segment_commands(seg):
        for tok in cmd.all_tokens:
            if tok.type is not TokenType.VAR or normalise_var_name(tok.text) != target:
                continue
            start_off = base + tok.start.offset
            end_off = base + tok.end.offset
            if (
                source[start_off : start_off + 2] == "${"
                and end_off + 1 < len(source)
                and source[end_off + 1] == "}"
            ):
                end_off += 1
            # Only tighten — never widen — relative to the reported range.
            if start_off <= r.start.offset and end_off >= r.end.offset:
                return warning
            return replace(
                warning,
                range=Range(
                    start=buffer.offset_to_position(start_off),
                    end=buffer.offset_to_position(end_off),
                ),
            )
    return warning


def find_taint_warnings(
    source: str,
    cu: CompilationUnit | None = None,
) -> list[TaintWarning]:
    """Run taint analysis and return warnings."""
    cu = ensure_compilation_unit(source, cu, logger=log, context="taint")
    if cu is None:
        return []

    solved = _solve_interprocedural_taints(cu)
    all_warnings: list[TaintWarning] = []

    top_exec = set(cu.top_level.cfg.blocks) - cu.top_level.analysis.unreachable_blocks
    all_warnings.extend(
        _find_taint_sinks(
            cu.top_level.cfg,
            cu.top_level.ssa,
            solved.top_taints,
            top_exec,
        )
    )
    all_warnings.extend(
        _find_setter_constraint_violations(
            cu.top_level.cfg,
            cu.top_level.ssa,
            solved.top_taints,
            top_exec,
        )
    )
    all_warnings.extend(
        _find_uri_split_suggestions(
            cu.top_level.cfg,
            cu.top_level.ssa,
            cu.top_level.analysis.values,
            top_exec,
        )
    )
    all_warnings.extend(
        _find_path_concat_warnings(
            cu.top_level.cfg,
            cu.top_level.ssa,
            solved.top_taints,
            top_exec,
            rendered_props=cu.top_level.analysis.rendered_props,
        )
    )
    all_warnings.extend(
        _find_destructive_file_warnings(
            cu.top_level.cfg,
            cu.top_level.ssa,
            solved.top_taints,
            top_exec,
        )
    )

    for qname, fu in cu.procedures.items():
        proc_taints = solved.proc_taints.get(qname, fu.analysis.taints)
        executable = set(fu.cfg.blocks) - fu.analysis.unreachable_blocks
        all_warnings.extend(
            _find_taint_sinks(
                fu.cfg,
                fu.ssa,
                proc_taints,
                executable,
            )
        )
        all_warnings.extend(
            _find_setter_constraint_violations(
                fu.cfg,
                fu.ssa,
                proc_taints,
                executable,
            )
        )
        all_warnings.extend(
            _find_uri_split_suggestions(
                fu.cfg,
                fu.ssa,
                fu.analysis.values,
                executable,
            )
        )
        all_warnings.extend(
            _find_path_concat_warnings(
                fu.cfg,
                fu.ssa,
                proc_taints,
                executable,
                rendered_props=fu.analysis.rendered_props,
            )
        )
        all_warnings.extend(
            _find_destructive_file_warnings(
                fu.cfg,
                fu.ssa,
                proc_taints,
                executable,
            )
        )

    buffer = DocumentBuffer.from_source(source)
    return [_narrow_to_tainted_variable(source, buffer, w) for w in all_warnings]
