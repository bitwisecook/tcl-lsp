"""High-level entry point — ties parser, evaluator, and edit planner together.

This is what the ``f5 query`` verb calls.  Splitting the orchestration
out here keeps the CLI module focused on argparse plumbing and makes
the runner reusable from tests, MCP tools, or future ad-hoc scripts.
"""

from __future__ import annotations

import contextlib
from dataclasses import dataclass, field
from typing import Any

from ..parser import parse_bigip_conf
from .edit_plan import AppliedSource, apply
from .evaluator import EvalContext, evaluate_statement
from .parser import parse_query
from .source_map import SourceMap
from .values import Root

# Active roots are looked up by URI by graph builtins (``refs`` /
# ``referenced_by``) which need the per-file context without it being
# threaded through every call.  This is a module-level map rather than
# a thread-local because the CLI is single-threaded.
_ACTIVE_ROOTS: dict[str, Root] = {}


@dataclass
class QueryResult:
    """The combined output of a single ``f5 query`` invocation.

    ``has_mutation`` reports whether the query *attempted* a mutation
    (queued any edit op or prefix rewrite).  The actual textual diff
    may still be empty when the query targets nothing — the CLI uses
    ``has_mutation`` to pick between the diff-or-write code path and
    the value-rendering one, and then independently checks whether
    each :class:`AppliedSource` actually changed to decide the exit
    code.
    """

    values_per_file: dict[str, list[Any]] = field(default_factory=dict)
    edits_per_file: dict[str, AppliedSource] = field(default_factory=dict)
    has_mutation: bool = False


@contextlib.contextmanager
def _active_root(root: Root):
    prior = _ACTIVE_ROOTS.get(root.uri)
    _ACTIVE_ROOTS[root.uri] = root
    try:
        yield
    finally:
        if prior is None:
            _ACTIVE_ROOTS.pop(root.uri, None)
        else:
            _ACTIVE_ROOTS[root.uri] = prior


def _build_root(uri: str, source: str) -> Root:
    return Root(
        uri=uri,
        source=source,
        config=parse_bigip_conf(source),
        source_map=SourceMap.build(source),
    )


def run_query(query: str, sources: dict[str, str]) -> QueryResult:
    """Parse *query* and run it against each file in *sources*.

    Statements separated by ``;`` are evaluated and applied in order:
    each statement's edits land before the next runs, so a
    ``rename_partition("Common","Tenant_A"); .ltm.virtual[].destination = "..."``
    chain sees a coherent post-rewrite source by the second statement.
    Inside a single statement the planner still rejects mixing a
    prefix-cascade rewrite with field edits — the byte ranges captured
    at projection time would otherwise be wrong after the rewrite.
    """
    program = parse_query(query)
    from ..rewrite import RenameReport

    result = QueryResult()
    for uri, source in sources.items():
        current_source = source
        # Each accumulated rename is summary-only — we drop the
        # ``new_source`` payload from every per-step report so multi-
        # step queries don't retain k * source-size bytes for the run.
        # The post-edit text is on ``current_source`` (and ends up on
        # the final ``AppliedSource``); the CLI uses only the summary
        # fields for the stderr "renamed X -> Y (N occurrence(s))" line.
        accumulated_renames: list[RenameReport] = []
        accumulated_field_edits = 0
        last_values: list[Any] = []

        attempted_mutation = False
        for index, stmt in enumerate(program.statements):
            root = _build_root(uri, current_source)
            ctx = EvalContext(root=root)
            with _active_root(root):
                values = evaluate_statement(stmt, ctx)
            if index == len(program.statements) - 1:
                last_values = values
            if ctx.edits.has_edits():
                attempted_mutation = True
                applied = apply(ctx.edits, {uri: current_source})[uri]
                current_source = applied.new_source
                for rep in applied.rename_reports:
                    accumulated_renames.append(
                        RenameReport(
                            old=rep.old,
                            new=rep.new,
                            occurrences=rep.occurrences,
                            new_source="",
                        )
                    )
                accumulated_field_edits += applied.field_edits

        result.values_per_file[uri] = last_values
        if attempted_mutation:
            result.has_mutation = True
            result.edits_per_file[uri] = AppliedSource(
                uri=uri,
                original=source,
                new_source=current_source,
                rename_reports=tuple(accumulated_renames),
                field_edits=accumulated_field_edits,
            )
    return result
