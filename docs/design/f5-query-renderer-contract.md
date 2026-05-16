# `f5 query` renderer plugin contract

The query engine ships with a decorator-registered **renderer
registry** that turns a list of evaluator output values into a
displayable string.  Built-in renderers live under
`core/bigip/query/renderers/`; user-supplied renderers register with
the same decorator the built-ins use and are dispatched through the
same code path the CLI uses for `-R / --render NAME`.

This doc fixes the contract: what a renderer must do, what the engine
guarantees, where the registration lives, and how errors propagate.

## Registry layout

| File | Role |
|---|---|
| `core/bigip/query/renderers/__init__.py` | `RendererSpec`, `@renderer` decorator, `lookup`, `list_renderers`, `render`, `bind_render_sources`. |
| `core/bigip/query/renderers/gantt.py` | `@renderer("gantt")` — ASCII Gantt timeline. |
| `core/bigip/query/renderers/ascii_blocks.py` | `@renderer("ascii-blocks")` — Unicode line-art tree. |
| `core/bigip/query/renderers/mermaid.py` | `@renderer("mermaid")` — Mermaid diagram. |
| `core/bigip/query/output.py` | `render(values, *, mode, **opts)` — falls through to the renderer registry on an unknown built-in mode. |
| `core/bigip/query/api.py` | `QueryRun.render(name, **opts)` — wraps `render` with `bind_render_sources` so renderers can reach the originating source text. |

The pattern mirrors two existing in-repo registries:

- `@verb` in `explorer/verbs/f5/_registry.py` (CLI subcommands)
- `@_register` in `core/bigip/query/builtins.py` (DSL builtins)
- `@tool` in `ai/mcp/tcl_mcp_server.py` (MCP tools)

— same decorator-then-spec shape, same duplicate-registration guard,
same introspection helpers (`list_renderers()` ↔ `list_builtins()` ↔
`get_verb_catalogue()`).

## `RendererSpec`

```python
@dataclass(frozen=True, slots=True)
class RendererSpec:
    name: str
    summary: str
    accepts: str
    impl: Callable[..., str]
    details: str = ""
```

- `name` — the lookup key used by `lookup`, `render`, and the CLI's
  `-R / --render NAME` flag.  Convention: lowercase, dash-separated
  (`ascii-blocks`, not `AsciiBlocks` or `ascii_blocks`).
- `summary` — one-line description rendered by `--help-renderers`.
- `accepts` — free-text shape hint shown by `--help-renderers`
  ("stream of `ObjectRef`", "rows of `(ts, label, state)`").  The
  registry **does not enforce** it — it's documentation, not
  validation.
- `impl` — the renderer callable; see [Renderer contract](#renderer-contract).
- `details` — optional multi-paragraph prose surfaced verbatim by
  `--help-renderers`.

## Renderer contract

A renderer is a callable

```python
def renderer_impl(values: list[Any], **opts: Any) -> str: ...
```

with the following contract:

- **Pure** — must not print, must not write files, must not mutate
  *values*.  Side-effecting renderers break the CLI's per-file
  dispatch and break `QueryRun.render`'s composability.
- **Total over its declared shape** — if `accepts` says "stream of
  `ObjectRef`", the renderer must handle every legal `ObjectRef`
  shape (with or without `stanza_slot`, with or without `config_uri`).
- **Tolerant of unknown shapes** — when *values* doesn't match
  `accepts`, the renderer SHOULD fall back to a reasonable default
  ("emit the values as a chain"); only raise
  `~core.bigip.query.errors.RendererError` for inputs that genuinely
  cannot be rendered (e.g. a dict missing a required key the
  renderer documented as mandatory).  This matches the way built-in
  `output` modes coerce mixed-shape input to JSON rather than
  refusing.
- **Self-validating options** — `opts` arrives as plain strings from
  `--render-opt KEY=VALUE`.  Renderers parse, type-check, and clamp
  their own options, raising `RendererError` on invalid input with a
  message the CLI can prefix with `error:`.

The return value is a single string written verbatim to stdout (or
returned from `QueryRun.render`).  Renderers conventionally end with
a trailing newline so multi-file CLI runs stack cleanly.

## Error mapping

Two exceptions matter:

- `RendererError(QueryError)` — caller-visible problem (unknown
  option, malformed input shape, missing required argument).  The
  CLI's `_emit_values` catches `QueryError` and prints `error: <msg>`
  with exit code 2.  Python callers can catch either the specific
  class or the broader `QueryError` umbrella.
- `ValueError` — only raised by `output.render` itself, when the
  requested mode is neither a built-in nor a registered renderer.
  Also surfaced via `error:` exit 2 by `_emit_values`.

Any other exception propagates with its original traceback so a
renderer bug surfaces as a bug, not as a CLI error message.

## Registration lifecycle

Built-in renderer modules are imported **lazily**: the first call to
`lookup`, `list_renderers`, or `render` triggers
`_ensure_builtins_loaded()`, which imports the three sibling modules
once.  This keeps `import core.bigip.query` cheap for callers that
never ask for a renderer (the LSP server, the bytecode compiler,
unit tests) while still ensuring the CLI's `--help-renderers` action
sees them without an explicit import.

Third-party renderer plugins must be imported before the registry
consults them — there is no entry-point-based auto-discovery today.
The recommended packaging shape is:

```python
# my_pkg/__init__.py
from . import renderers  # side-effect: registers @renderer plugins
```

The user then runs `python -c "import my_pkg; import f5_cli; f5_cli.main()"`
or ships a tiny wrapper script that imports the plugin package
before calling into the CLI.  Entry-point discovery is deliberately
out of scope for v1 — see the **Open questions** section.

## Source-text recovery (renderer-side)

Renderers like `mermaid` need the originating BIG-IP source text to
build a reference graph.  Two cooperating contextvars surface it:

| ContextVar | Set by | Read by |
|---|---|---|
| `RENDER_SOURCES` (in `renderers/__init__.py`) | `QueryRun.render` via `bind_render_sources` | `mermaid._recover_source_text` (preferred path) |
| `_ACTIVE_ROOTS` (in `runner.py`) | `run_query` during an in-flight query | `mermaid._recover_source_text` (fallback for renderers invoked mid-evaluation) |

The render-time contextvar is the **primary** path so a script that
calls `Query(...).run(paths=[...]).render("mermaid")` Just Works
without threading source text by hand.  The runner-side contextvar
exists for the rarer case of a renderer invoked from inside a
builtin, where `run_query` is still on the stack.

When neither contextvar yields a source, the mermaid renderer falls
back to chain mode rather than raising — same "tolerant of unknown
shapes" rule as elsewhere in the contract.

## CLI integration

`explorer/verbs/f5/query.py` adds three pieces:

1. `--render NAME` / `-R NAME` — added to the existing mutually
   exclusive output-format group (alongside `--scf` / `--raw` /
   `--paths-only` / `--json` / `--table` / `--table-lineart`).
2. `--render-opt KEY=VALUE` — repeatable; parsed into a flat
   `{KEY: VALUE}` dict and forwarded to the renderer as kwargs.
3. `--help-renderers` — argparse action that imports the built-in
   renderers and prints the registry.

The `_run_query` handler sets `args.output_mode = args.render_name`
when `--render` is passed and forwards `args.render_opts` through
the standard `_emit_values` path.  `output.render` does the actual
dispatch — its existing `mode` argument is the single source of
truth for "which formatter handles this batch".

## Python API integration

`QueryRun.render(name, **opts)` (in `core/bigip/query/api.py`):

```python
def render(self, name, **opts):
    from .renderers import bind_render_sources, render as _render
    with bind_render_sources(self._sources):
        return _render(name, self.values(), **opts)
```

`Query.run` stashes the source map onto the `QueryRun` so the
context-var binding can re-expose it to the renderer.

## Open questions / future work

- **Entry-point discovery.**  We deliberately defer
  `setuptools.entry_points`-based plugin discovery to v2.  When we
  add it, the natural group name is `f5q.renderers` and the
  discovery should happen inside `_ensure_builtins_loaded()` so the
  CLI and the Python API pick up the same plugin set.
- **APM-policy renderer.**  The `ascii-blocks` renderer is the
  scaffold for the future TMUI-style APM policy view (tracked in a
  separate spec).  When that lands it can re-use `_Box` and
  `_render_box` directly rather than reimplementing the tree walker.
- **Multi-file Mermaid.**  Today the Mermaid renderer's
  ObjectRef-mode emits one diagram per CLI invocation.  Per-file
  dispatch (one `graph` block per source URI, with cross-file edges
  inside a `subgraph`) is the natural follow-on for `--merge` mode.
