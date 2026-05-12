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
from .evaluator import EvalContext, evaluate
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
    """The combined output of a single ``f5 query`` invocation."""

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


def run_query(query: str, sources: dict[str, str]) -> QueryResult:
    """Parse *query* and run it against each file in *sources*.

    Returns one :class:`QueryResult` capturing the projected values
    (per file) and the applied edits (per file).  The caller decides
    whether to print the dry-run diff, write the new source out, or
    propagate the values.
    """
    program = parse_query(query)
    result = QueryResult()
    for uri, source in sources.items():
        config = parse_bigip_conf(source)
        root = Root(
            uri=uri,
            source=source,
            config=config,
            source_map=SourceMap.build(source),
        )
        ctx = EvalContext(root=root)
        with _active_root(root):
            values = evaluate(program, ctx)
        result.values_per_file[uri] = values
        if ctx.edits.has_edits():
            result.has_mutation = True
            edits = apply(ctx.edits, {uri: source})
            result.edits_per_file.update(edits)
    return result
