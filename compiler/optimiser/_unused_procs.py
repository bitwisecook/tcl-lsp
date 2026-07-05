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

"""Unused proc commenting pass (O124) for iRules.

When an iRule defines procs that are never called from any event handler
(transitively), this pass suggests commenting them out.  It only applies
to iRules that have at least one non-RULE_INIT event — iRules that look
like libraries (only procs and RULE_INIT setting static variables) are
excluded.
"""

from __future__ import annotations

from compiler.registry.dialect import active_dialect
from compiler.registry.dialects import is_irules_dialect
from shared.codes import opt

from ..ir import when_event_name
from ._types import Optimisation, PassContext


def _is_library_irule(event_names: frozenset[str]) -> bool:
    """Return True if the iRule looks like a library.

    A library iRule has only procs and at most a RULE_INIT event
    (used for setting static variables).  It has no "real" events
    that would actually invoke the procs at runtime.
    """
    non_init = event_names - {"RULE_INIT"}
    return len(non_init) == 0


def _reachable_procs(
    roots: set[str],
    call_graph: dict[str, tuple[str, ...]],
) -> set[str]:
    """Return all proc qualified names reachable from *roots*."""
    visited: set[str] = set()
    stack = list(roots)
    while stack:
        current = stack.pop()
        if current in visited:
            continue
        visited.add(current)
        for callee in call_graph.get(current, ()):
            if callee not in visited:
                stack.append(callee)
    return visited


def _comment_out(text: str, proc_name: str) -> str:
    """Comment out a proc definition, preserving indentation."""
    lines = text.split("\n")
    commented: list[str] = []
    for line in lines:
        if line.strip():
            commented.append(f"# {line}")
        else:
            commented.append("#")
    return f"# [O124] Unused proc — '{proc_name}' is not called from any event\n" + "\n".join(
        commented
    )


@opt(
    code="O124",
    description="Comment out unused procs in iRules (not called from any event).",
    opt_category="dce",
)
def optimise_unused_procs(ctx: PassContext) -> None:
    """O124: comment out procs not called from any event handler."""
    if not is_irules_dialect(active_dialect()):
        return

    ir_module = ctx.ir_module
    if ir_module is None:
        return

    # Separate event procs (::when::*) from user procs.
    event_procs: set[str] = set()
    event_names: set[str] = set()
    user_procs: set[str] = set()

    for qname in ir_module.procedures:
        if qname.startswith("::when::"):
            event_procs.add(qname)
            event_names.add(when_event_name(qname))
        else:
            user_procs.add(qname)

    if not user_procs:
        return

    # Skip library iRules (only procs + optional RULE_INIT).
    if _is_library_irule(frozenset(event_names)):
        return

    # Build call graph from interprocedural analysis.
    call_graph: dict[str, tuple[str, ...]] = {}
    for qname, summary in ctx.interproc.procedures.items():
        call_graph[qname] = summary.calls

    # Find all procs reachable from any event handler.
    reachable = _reachable_procs(event_procs, call_graph)

    # Conservative escape hatch: if any reachable proc has a dynamic barrier
    # (eval, uplevel, etc.), dynamic dispatch could target any "unused" proc.
    # Suppress O124 to avoid false positives.  We intentionally do NOT check
    # has_unknown_calls here — that flag fires for any impure built-in command
    # (pool, puts, etc.) which cannot dynamically invoke user procs.
    for qname in reachable:
        summary = ctx.interproc.procedures.get(qname)
        if summary is not None and summary.has_barrier:
            return

    # Report unused user procs.
    for qname in sorted(user_procs):
        if qname in reachable:
            continue

        ir_proc = ir_module.procedures[qname]
        start = ir_proc.range.start.offset
        end = ir_proc.range.end.offset
        proc_text = ctx.source[start : end + 1]
        replacement = _comment_out(proc_text, ir_proc.name)

        ctx.optimisations.append(
            Optimisation(
                code="O124",
                message=f"Proc '{ir_proc.name}' is not called from any event and can be removed",
                range=ir_proc.range,
                replacement=replacement,
            )
        )
