"""Query DSL for BIG-IP configurations.

A small jq-flavoured language for inspecting and rewriting
``bigip.conf`` / SCF files.  See :doc:`docs/design/f5-query-dsl` for
the full grammar and semantics.

Public entry points:

- :func:`run_query` — parse, evaluate, and (optionally) apply edits in
  one call.  This is what the ``f5 query`` verb drives.
- :func:`parse_query` — produce an AST for a query string.  Useful for
  the help-time validation tests.
- :func:`format_grammar` / :func:`format_builtins` /
  :func:`format_examples` — render the same documentation surfaced by
  the ``f5 query --help-dsl`` / ``--help-builtins`` / ``--help-examples``
  actions.  Kept here so the help text and the design doc generate from
  the same source.
"""

from __future__ import annotations

from .builtins import format_builtins, list_builtins
from .examples import format_examples, list_examples
from .grammar import format_grammar
from .parser import parse_query
from .runner import QueryResult, run_query

__all__ = (
    "QueryResult",
    "format_builtins",
    "format_examples",
    "format_grammar",
    "list_builtins",
    "list_examples",
    "parse_query",
    "run_query",
)
