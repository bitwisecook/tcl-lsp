"""Public entry point for taint analysis."""

from __future__ import annotations

import logging

from ..compilation_unit import CompilationUnit, ensure_compilation_unit
from ._collect_release import _find_collect_without_release, _find_release_without_collect
from ._interprocedural import _solve_interprocedural_taints
from ._sinks import _find_setter_constraint_violations, _find_taint_sinks
from ._types import CollectWithoutReleaseWarning, ReleaseWithoutCollectWarning, TaintWarning
from ._uri_split import _find_uri_split_suggestions

log = logging.getLogger(__name__)


def find_taint_warnings(
    source: str,
    cu: CompilationUnit | None = None,
) -> list[TaintWarning | CollectWithoutReleaseWarning | ReleaseWithoutCollectWarning]:
    """Run taint analysis and return warnings."""
    cu = ensure_compilation_unit(source, cu, logger=log, context="taint")
    if cu is None:
        return []

    solved = _solve_interprocedural_taints(cu)
    all_warnings: list[
        TaintWarning | CollectWithoutReleaseWarning | ReleaseWithoutCollectWarning
    ] = []

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

    # Collect/release is a syntactic check over the IR.
    all_warnings.extend(_find_collect_without_release(cu.ir_module.top_level))
    all_warnings.extend(_find_release_without_collect(cu.ir_module.top_level))
    for proc in cu.ir_module.procedures.values():
        all_warnings.extend(_find_collect_without_release(proc.body))
        all_warnings.extend(_find_release_without_collect(proc.body))

    return all_warnings
