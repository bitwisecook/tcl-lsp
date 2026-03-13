"""Shared compilation pipeline for the compiler explorer.

Provides `run_pipeline()` — the single entry point for both CLI and web
explorers — along with supporting data types and view constants.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass

from core.commands.registry.runtime import available_dialects, configure_signatures
from core.compiler.cfg import CFGFunction
from core.compiler.compilation_unit import CompilationUnit, ensure_compilation_unit
from core.compiler.core_analyses import FunctionAnalysis
from core.compiler.gvn import RedundantComputation, find_redundant_computations
from core.compiler.interprocedural import InterproceduralAnalysis
from core.compiler.ir import IRModule
from core.compiler.irules_flow import (
    EventOrderEntry,
    IrulesFlowWarning,
    extract_event_order,
    find_irules_flow_warnings,
)
from core.compiler.optimiser import Optimisation, apply_optimisations, find_optimisations
from core.compiler.shimmer import ShimmerWarning, ThunkingWarning, find_shimmer_warnings
from core.compiler.ssa import SSAFunction
from core.compiler.taint import (
    CollectWithoutReleaseWarning,
    ReleaseWithoutCollectWarning,
    TaintWarning,
    find_taint_warnings,
)

# Data types


@dataclass(slots=True)
class FunctionSnapshot:
    """Per-function compilation artefacts."""

    name: str
    cfg: CFGFunction
    ssa: SSAFunction
    analysis: FunctionAnalysis


@dataclass(slots=True, frozen=True)
class CompilerExplorerResult:
    """Complete result of running the compilation pipeline."""

    source: str
    ir_module: IRModule
    snapshots: list[FunctionSnapshot]
    total_blocks: int
    interproc: InterproceduralAnalysis
    optimised_source: str
    optimisations: list[Optimisation]
    shimmer_warnings: list[ShimmerWarning | ThunkingWarning]
    gvn_warnings: list[RedundantComputation]
    taint_warnings: list[TaintWarning | CollectWithoutReleaseWarning | ReleaseWithoutCollectWarning]
    irules_flow_warnings: list[IrulesFlowWarning]
    event_order: list[EventOrderEntry]


# View selection

ALL_VIEWS = frozenset(
    {
        "ir",
        "cfg",
        "ssa",
        "interproc",
        "types",
        "opt",
        "gvn",
        "shimmer",
        "taint",
        "irules",
        "callouts",
        "asm",
        "wasm",
        "asm-opt",
        "wasm-opt",
    }
)

VIEW_GROUPS: dict[str, frozenset[str]] = {
    "all": ALL_VIEWS,
    "compiler": ALL_VIEWS - {"opt"},
    "optimiser": frozenset({"opt", "callouts"}),
}

AVAILABLE_DIALECTS = tuple(sorted(available_dialects()))


def expand_show(raw: str) -> frozenset[str]:
    """Expand a comma-separated --show value into a set of view names."""
    views: set[str] = set()
    for token in raw.split(","):
        token = token.strip()
        if not token:
            continue
        if token in VIEW_GROUPS:
            views |= VIEW_GROUPS[token]
        elif token in ALL_VIEWS:
            views.add(token)
        else:
            raise argparse.ArgumentTypeError(
                f"unknown view {token!r}; choose from: "
                f"{', '.join(sorted(ALL_VIEWS | set(VIEW_GROUPS)))}"
            )
    return frozenset(views)


# Pipeline helpers


def build_snapshots(cu: CompilationUnit) -> tuple[list[FunctionSnapshot], int]:
    """Build per-function snapshots from a shared ``CompilationUnit``."""
    snapshots: list[FunctionSnapshot] = [
        FunctionSnapshot("::top", cu.top_level.cfg, cu.top_level.ssa, cu.top_level.analysis)
    ]

    for qname in sorted(cu.procedures):
        fu = cu.procedures[qname]
        snapshots.append(FunctionSnapshot(qname, fu.cfg, fu.ssa, fu.analysis))

    total_blocks = sum(len(s.cfg.blocks) for s in snapshots)
    return snapshots, total_blocks


# Main pipeline


def run_pipeline(source: str, *, dialect: str | None = None) -> CompilerExplorerResult:
    """Run the full explorer pipeline on source text.

    If *dialect* is provided, calls ``configure_signatures(dialect=...)``
    before compilation.  Each additional pass degrades gracefully on
    TypeError/AttributeError (returning empty lists).
    """
    if dialect is not None:
        configure_signatures(dialect=dialect)

    cu = ensure_compilation_unit(source, context="compiler_explorer")
    if cu is None:
        return CompilerExplorerResult(
            source=source,
            ir_module=IRModule(),
            snapshots=[],
            total_blocks=0,
            interproc=InterproceduralAnalysis(procedures={}),
            optimised_source=source,
            optimisations=[],
            shimmer_warnings=[],
            gvn_warnings=[],
            taint_warnings=[],
            irules_flow_warnings=[],
            event_order=[],
        )

    ir_module = cu.ir_module
    snapshots, total_blocks = build_snapshots(cu)
    interproc = cu.interproc

    try:
        optimisations = find_optimisations(source, cu=cu)
        optimised_source = apply_optimisations(source, optimisations)
    except (TypeError, AttributeError):
        optimised_source, optimisations = source, []

    try:
        shimmer_warnings = find_shimmer_warnings(source, cu=cu)
    except (TypeError, AttributeError):
        shimmer_warnings = []

    try:
        gvn_warnings = find_redundant_computations(source, cu=cu)
    except (TypeError, AttributeError):
        gvn_warnings = []

    try:
        taint_warnings = find_taint_warnings(source, cu=cu)
    except (TypeError, AttributeError):
        taint_warnings = []

    try:
        irules_flow_warnings = find_irules_flow_warnings(source, cu=cu)
    except (TypeError, AttributeError):
        irules_flow_warnings = []

    try:
        event_order = extract_event_order(source)
    except (TypeError, AttributeError):
        event_order = []

    return CompilerExplorerResult(
        source=source,
        ir_module=ir_module,
        snapshots=snapshots,
        total_blocks=total_blocks,
        interproc=interproc,
        optimised_source=optimised_source,
        optimisations=optimisations,
        shimmer_warnings=shimmer_warnings,
        gvn_warnings=gvn_warnings,
        taint_warnings=taint_warnings,
        irules_flow_warnings=irules_flow_warnings,
        event_order=event_order,
    )


def compute_stats(result: CompilerExplorerResult) -> dict[str, int]:
    """Compute summary statistics from a pipeline result."""
    return {
        "procedures": len(result.ir_module.procedures),
        "functions": len(result.snapshots),
        "blocks": result.total_blocks,
        "deadStores": sum(len(s.analysis.dead_stores) for s in result.snapshots),
        "unreachableBlocks": sum(len(s.analysis.unreachable_blocks) for s in result.snapshots),
        "rewrites": len(result.optimisations),
        "shimmerWarnings": len(result.shimmer_warnings),
        "gvnWarnings": len(result.gvn_warnings),
        "taintWarnings": len(result.taint_warnings),
        "irulesFlowWarnings": len(result.irules_flow_warnings),
    }
