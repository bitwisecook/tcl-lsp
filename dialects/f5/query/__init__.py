"""Query DSL for BIG-IP configurations.

A small jq-flavoured language for inspecting and rewriting
``bigip.conf`` / SCF files.  See :doc:`docs/design/f5-query-dsl` for
the full grammar and semantics.

Public entry points:

- :class:`Query` / :class:`QueryRun` — the fluent script-friendly API.
  ``Query(text).run(paths=[...]).values()`` is the one-call shape
  external Python callers normally want; ``.render("gantt")`` and
  friends dispatch through the renderer registry without the script
  having to know where the renderer lives.
- :func:`run_query` — the lower-level entry point :class:`Query`
  wraps.  Keeps the per-URI :class:`QueryResult` shape the CLI uses.
- :func:`parse_query` — produce an AST for a query string.  Useful for
  the help-time validation tests.
- :func:`render` / :func:`renderer` / :func:`list_renderers` — the
  renderer registry (:mod:`dialects.f5.query.renderers`).  ``@renderer``
  registers a new plugin, :func:`render` dispatches by name.
- :func:`format_grammar` / :func:`format_builtins` /
  :func:`format_examples` — render the same documentation surfaced by
  the ``f5 query --help-dsl`` / ``--help-builtins`` / ``--help-examples``
  actions.  Kept here so the help text and the design doc generate from
  the same source.
- :class:`ObjectRef` / :class:`PathRef` / :class:`Stream` / :class:`Root`
  — the value model the evaluator hands back.  Re-exported here so
  scripts can pattern-match on result shapes without reaching into a
  private sub-module.
"""

from __future__ import annotations

from ._inputs import InputSpec
from .api import Query, QueryRow, QueryRun, Sources, load, q
from .builtins import builtin, format_builtins, list_builtins
from .errors import (
    BuiltinError,
    EditError,
    EvalError,
    LexError,
    ParseError,
    QueryError,
    RendererError,
)
from .examples import format_examples, list_examples
from .grammar import format_grammar
from .inputs import input_format, list_input_formats
from .parser import parse_query
from .plugins import load_user_plugins, xdg_plugin_dir
from .renderers import list_renderers, render, renderer
from .runner import QueryResult, run_query
from .values import ObjectRef, PathRef, Root, Stream

__all__ = (
    "BuiltinError",
    "EditError",
    "EvalError",
    "InputSpec",
    "LexError",
    "ObjectRef",
    "ParseError",
    "PathRef",
    "Query",
    "QueryError",
    "QueryResult",
    "QueryRow",
    "QueryRun",
    "RendererError",
    "Root",
    "Sources",
    "Stream",
    "builtin",
    "format_builtins",
    "format_examples",
    "format_grammar",
    "input_format",
    "list_builtins",
    "list_examples",
    "list_input_formats",
    "list_renderers",
    "load",
    "load_user_plugins",
    "parse_query",
    "q",
    "render",
    "renderer",
    "run_query",
    "xdg_plugin_dir",
)
