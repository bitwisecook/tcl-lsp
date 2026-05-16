"""``f5q`` — public Python alias for :mod:`core.bigip.query`.

Re-exports the same names :mod:`core.bigip.query` exposes under a
shorter, version-stable import path designed for external scripts:

.. code-block:: python

    from f5q import Query, renderer

    rows = (
        Query(".ltm.virtual[] | .name")
        .run(paths=["bigip.conf"])
        .values()
    )

    @renderer("my-report", summary="Markdown table of results.", accepts="any")
    def _render(values, **opts):
        return "| name |\\n| ---- |\\n" + "\\n".join(f"| {v} |" for v in values)

The shim is intentionally thin — it does no work of its own, just
forwards the names — so adding a new public symbol to
``core.bigip.query.__all__`` automatically makes it importable from
``f5q``.
"""

from __future__ import annotations

from core.bigip.query import (  # noqa: F401  (re-export)
    BuiltinError,
    EditError,
    EvalError,
    LexError,
    ObjectRef,
    ParseError,
    PathRef,
    Query,
    QueryError,
    QueryResult,
    QueryRow,
    QueryRun,
    RendererError,
    Root,
    Stream,
    format_builtins,
    format_examples,
    format_grammar,
    list_builtins,
    list_examples,
    list_renderers,
    parse_query,
    render,
    renderer,
    run_query,
)

__all__ = (
    "BuiltinError",
    "EditError",
    "EvalError",
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
    "Stream",
    "format_builtins",
    "format_examples",
    "format_grammar",
    "list_builtins",
    "list_examples",
    "list_renderers",
    "parse_query",
    "render",
    "renderer",
    "run_query",
)
