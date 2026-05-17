"""``f5q`` — public Python alias for :mod:`core.bigip.query`.

Re-exports the same names :mod:`core.bigip.query` exposes under a
shorter, version-stable import path designed for external scripts:

.. code-block:: python

    from f5q import Query, builtin, renderer, input_format

    # Run a query.
    rows = (
        Query(".ltm.virtual[] | .name")
        .run(paths=["bigip.conf"])
        .values()
    )

    # Register your own builtin / renderer / input format.
    @builtin("uppercase", summary="ASCII upper-case a string.",
             min_args=1, max_args=1)
    def _upper(s):
        return str(s).upper()

    @renderer("md-table", summary="Markdown table of results.", accepts="any")
    def _render(values, **opts):
        return "| name |\\n| ---- |\\n" + "\\n".join(f"| {v} |" for v in values)

    @input_format("yaml", summary="YAML side-input.")
    def _parse_yaml(source, *, uri, options=()):
        import yaml
        return yaml.safe_load(source)

Plugins dropped into ``$XDG_CONFIG_HOME/f5q/plugins/`` (default
``~/.config/f5q/plugins/``) auto-load on the first registry access
— see :func:`load_user_plugins` and :func:`xdg_plugin_dir`.

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
    InputSpec,
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
    Sources,
    Stream,
    builtin,
    format_builtins,
    format_examples,
    format_grammar,
    input_format,
    list_builtins,
    list_examples,
    list_input_formats,
    list_renderers,
    load,
    load_user_plugins,
    parse_query,
    q,
    render,
    renderer,
    run_query,
    xdg_plugin_dir,
)

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
