# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Source-callout annotations for the compiler explorer.

Single backend API that walks a :class:`CompilerExplorerResult` and emits a
typed list of :class:`Annotation`s.  Every renderer (text CLI, web GUI, future
adapters) consumes this list directly — no surface re-derives annotation
sources or classifies severity.

The data model:

* ``kind`` identifies the source of the annotation (``barrier``,
  ``deadStore``, ``constantBranch``, ``unreachable``, ``optimisation``,
  ``shimmer``/``thunking``, ``gvn``, ``taint``, ``irulesFlow``).
* ``severity`` is the backend's classification of how a renderer should
  *display* it (``error`` / ``warning`` / ``info``).  S102 shimmer is
  error; T1xxx and T3001-T3004 taint sinks are error; constant branches
  and optimiser rewrites are info; everything else is warning.
* ``priority`` controls sort order when multiple annotations land on the
  same source offset (lower = render first).
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING

from compiler.cfg import CFGBranch, CFGGoto, CFGReturn
from compiler.ir import IRBarrier, IRFor, IRIf, IRScript, IRSwitch
from compiler.shimmer import ThunkingWarning

from .pipeline import ALL_VIEWS, CompilerExplorerResult

if TYPE_CHECKING:
    from analyser.semantic_model import Range


class Severity(str, Enum):
    """Renderer-agnostic severity classification.

    The string value is what each renderer (CLI ANSI, GUI CSS class)
    keys off — neither classifies its own annotations.
    """

    ERROR = "error"
    WARNING = "warning"
    INFO = "info"


@dataclass(frozen=True, slots=True)
class Annotation:
    """One source callout — typed, classified, ready to render.

    Renderers do not parse ``kind`` or ``label`` or classify
    ``severity``; they just place the marker and colour it according to
    ``severity``.
    """

    range: "Range"
    label: str
    kind: str
    severity: Severity
    priority: int


# Severity classification — the single place that knows which warning
# codes are dangerous enough to render as errors.  Adding a new dangerous
# code is a one-line edit here, picked up by every renderer.
_DANGER_TAINT_PREFIXES = ("T1",)
_DANGER_TAINT_CODES = frozenset({"T3001", "T3002", "T3003", "T3004"})
_DANGER_SHIMMER_CODES = frozenset({"S102"})


def taint_severity(code: str) -> Severity:
    """Classify a taint warning code into a renderer severity.

    Renderers don't pattern-match on warning codes; they read this.
    """
    if code.startswith(_DANGER_TAINT_PREFIXES) or code in _DANGER_TAINT_CODES:
        return Severity.ERROR
    return Severity.WARNING


def shimmer_severity(code: str) -> Severity:
    """Classify a shimmer warning code into a renderer severity."""
    if code in _DANGER_SHIMMER_CODES:
        return Severity.ERROR
    return Severity.WARNING


# Annotation walkers


def _walk_barriers(script: IRScript, scope: str, out: list[Annotation]) -> None:
    for stmt in script.statements:
        if isinstance(stmt, IRBarrier):
            out.append(
                Annotation(
                    range=stmt.range,
                    label=f"{scope}: compiler barrier ({stmt.reason})",
                    kind="barrier",
                    severity=Severity.WARNING,
                    priority=2,
                )
            )
            continue
        if isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                _walk_barriers(clause.body, scope, out)
            if stmt.else_body is not None:
                _walk_barriers(stmt.else_body, scope, out)
            continue
        if isinstance(stmt, IRFor):
            _walk_barriers(stmt.init, scope, out)
            _walk_barriers(stmt.body, scope, out)
            _walk_barriers(stmt.next, scope, out)
            continue
        if isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if arm.body is not None:
                    _walk_barriers(arm.body, scope, out)
            if stmt.default_body is not None:
                _walk_barriers(stmt.default_body, scope, out)


def collect_annotations(
    result: CompilerExplorerResult,
    *,
    views: frozenset[str] = ALL_VIEWS,
    max_annotations: int = -1,
) -> tuple[list[Annotation], int]:
    """Walk *result* and return a sorted, classified list of annotations.

    ``views`` filters which annotation *sources* contribute (e.g. when
    the CLI is rendering ``--show ir,cfg`` only, opt/shimmer/etc.
    annotations are suppressed).  Pass :data:`ALL_VIEWS` to include
    everything (the web GUI default — it renders every tab).

    Sort key: ``(start_offset, priority, span_width, label)`` — stable,
    deterministic, and identical to the historical CLI ordering so
    visual diffs against snapshots stay clean.

    ``max_annotations`` truncates the result and returns the number of
    omitted annotations as the second tuple element.  ``-1`` disables
    truncation.
    """
    annotations: list[Annotation] = []
    ir_module = result.ir_module
    snapshots = result.snapshots

    # Barriers, dead stores, constant branches, unreachable blocks come
    # in via the IR / CFG / SSA views.  When none of those are
    # requested, skip the whole IR scan.
    if views & {"ir", "cfg", "ssa", "interproc"}:
        _walk_barriers(ir_module.top_level, "::top", annotations)
        for qname, proc in ir_module.procedures.items():
            _walk_barriers(proc.body, qname, annotations)

        for snap in snapshots:
            for dead in snap.analysis.dead_stores:
                block = snap.cfg.blocks.get(dead.block)
                if block is None:
                    continue
                if dead.statement_index < 0 or dead.statement_index >= len(block.statements):
                    continue
                stmt = block.statements[dead.statement_index]
                annotations.append(
                    Annotation(
                        range=stmt.range,
                        label=f"{snap.name}: dead store {dead.variable}#{dead.version}",
                        kind="deadStore",
                        severity=Severity.WARNING,
                        priority=1,
                    )
                )
            for branch in snap.analysis.constant_branches:
                block = snap.cfg.blocks.get(branch.block)
                if block is None:
                    continue
                term = block.terminator
                if not isinstance(term, CFGBranch) or term.range is None:
                    continue
                direction = "true" if branch.value else "false"
                annotations.append(
                    Annotation(
                        range=term.range,
                        label=(
                            f"{snap.name}: branch is always {direction}; "
                            f"takes {branch.taken_target}"
                        ),
                        kind="constantBranch",
                        severity=Severity.INFO,
                        priority=0,
                    )
                )
            for block_name in sorted(snap.analysis.unreachable_blocks):
                block = snap.cfg.blocks.get(block_name)
                if block is None:
                    continue
                if block.statements:
                    target_range = block.statements[0].range
                else:
                    term = block.terminator
                    if isinstance(term, (CFGGoto, CFGBranch, CFGReturn)) and term.range is not None:
                        target_range = term.range
                    else:
                        continue
                annotations.append(
                    Annotation(
                        range=target_range,
                        label=f"{snap.name}: unreachable block {block_name}",
                        kind="unreachable",
                        severity=Severity.WARNING,
                        priority=3,
                    )
                )

    if "opt" in views:
        # ``preview`` is imported lazily so the annotations module
        # doesn't pull in formatters (and its transitive expr-AST deps)
        # for callers that only want the typed list.
        from tooling.cli.formatters import preview

        for opt in result.optimisations:
            annotations.append(
                Annotation(
                    range=opt.range,
                    label=f"{opt.code}: {opt.message} -> {preview(opt.replacement, 40)}",
                    kind="optimisation",
                    severity=Severity.INFO,
                    priority=-1,
                )
            )

    if "shimmer" in views:
        for w in result.shimmer_warnings:
            severity = shimmer_severity(w.code)
            annotations.append(
                Annotation(
                    range=w.range,
                    label=f"{w.code}: {w.message}",
                    kind="thunking" if isinstance(w, ThunkingWarning) else "shimmer",
                    severity=severity,
                    priority=1 if severity is Severity.ERROR else 2,
                )
            )

    if "gvn" in views:
        for w in result.gvn_warnings:
            annotations.append(
                Annotation(
                    range=w.range,
                    label=f"{w.code}: {w.message or w.expression_text}",
                    kind="gvn",
                    severity=Severity.INFO,
                    priority=1,
                )
            )

    if "taint" in views:
        for w in result.taint_warnings:
            annotations.append(
                Annotation(
                    range=w.range,
                    label=f"{w.code}: {w.message}",
                    kind="taint",
                    severity=taint_severity(w.code),
                    priority=0,
                )
            )

    if "irules" in views:
        for w in result.irules_flow_warnings:
            annotations.append(
                Annotation(
                    range=w.range,
                    label=f"{w.code}: {w.message}",
                    kind="irulesFlow",
                    severity=Severity.WARNING,
                    priority=1,
                )
            )

    annotations.sort(
        key=lambda a: (
            a.range.start.offset,
            a.priority,
            a.range.end.offset - a.range.start.offset,
            a.label,
        )
    )

    if max_annotations >= 0 and len(annotations) > max_annotations:
        omitted = len(annotations) - max_annotations
        return annotations[:max_annotations], omitted

    return annotations, 0
