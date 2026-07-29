# `f5 query` plugin contract — renderers, builtins, input formats, and the loader

The query engine ships with **three decorator-registered registries**
— one each for output renderers, DSL builtin functions, and
side-input parsers — plus a **shared user-plugin loader** that
auto-imports Python files from `$XDG_CONFIG_HOME/dialects/f5/query/plugins/` on
first registry access.  External users can extend any of the three
surfaces without forking the project; the CLI and the Python API
both consult the same registries so plugins are visible from both.

This doc fixes the contracts: what each plugin shape must do, what
the engine guarantees, where the registrations live, and how errors
propagate.

## Registry layout

| File | Role |
|---|---|
| `dialects/f5/query/renderers/__init__.py` | `RendererSpec`, `@renderer` decorator, `lookup`, `list_renderers`, `render`, `bind_render_sources`. |
| `dialects/f5/query/renderers/gantt.py` | `@renderer("gantt")` — ASCII Gantt timeline. |
| `dialects/f5/query/renderers/ascii_blocks.py` | `@renderer("ascii-blocks")` — Unicode line-art tree. |
| `dialects/f5/query/renderers/mermaid.py` | `@renderer("mermaid")` — Mermaid diagram. |
| `dialects/f5/query/inputs.py` | `InputFormatSpec`, `@input_format` decorator, `lookup`, `list_input_formats`. |
| `dialects/f5/query/_builtin_inputs.py` | Registers the four built-in formats (json / jsonl / csv / f5log) by wrapping the parsers in `_inputs.py`. |
| `dialects/f5/query/builtins.py` | Public `@builtin` decorator wrapping the private `_register` used by in-tree builtins. |
| `dialects/f5/query/plugins.py` | `xdg_plugin_dir`, `load_user_plugins`, `iter_plugin_files`. |
| `dialects/f5/query/output.py` | `render(values, *, mode, **opts)` — falls through to the renderer registry on an unknown built-in mode. |
| `dialects/f5/query/api.py` | `QueryRun.render(name, **opts)` — wraps `render` with `bind_render_sources` so renderers can reach the originating source text. |

The pattern mirrors two existing in-repo registries:

- `@verb` in `tooling/f5/verbs/_registry.py` (CLI subcommands)
- `@_register` in `dialects/f5/query/builtins.py` (DSL builtins)
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
  `~dialects.f5.query.errors.RendererError` for inputs that genuinely
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
once.  This keeps `import dialects.f5.query` cheap for callers that
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

The user then runs `python -c "import my_pkg; from tooling.f5.main import main; main()"`
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
calls `f5q.q(...).render("mermaid")` Just Works without threading
source text by hand.  The runner-side contextvar exists for the
rarer case of a renderer invoked from inside a builtin, where
`run_query` is still on the stack.

When neither contextvar yields a source, the mermaid renderer falls
back to chain mode rather than raising — same "tolerant of unknown
shapes" rule as elsewhere in the contract.

## CLI integration

`tooling/f5/verbs/query.py` adds:

1. `--render NAME` / `-R NAME` — added to the existing mutually
   exclusive output-format group (alongside `--scf` / `--raw` /
   `--paths-only` / `--json` / `--table` / `--table-lineart`).
2. `--render-opt KEY=VALUE` — repeatable; parsed into a flat
   `{KEY: VALUE}` dict and forwarded to the renderer as kwargs.
3. `--input KIND NAME=PATH` — generic side-input flag that
   dispatches through the input-format registry.  Covers user
   plugins and any future built-in formats without needing a
   dedicated `--input-<kind>` flag.  The typed `--input-json`,
   `--input-jsonl`, `--input-csv`, `--input-f5log` shorthands stay
   for back-compat.
4. `--help-renderers`, `--help-inputs`, `--help-builtins`,
   `--help-plugins` — argparse actions that import the relevant
   registry contents and print the catalogue.

The `_run_query` handler sets `args.output_mode = args.render_name`
when `--render` is passed and forwards `args.render_opts` through
the standard `_emit_values` path.  `output.render` does the actual
dispatch — its existing `mode` argument is the single source of
truth for "which formatter handles this batch".

### Side-input name dedup

`_load_side_input` is the shared helper every `--input-*` flag
goes through.  It checks the requested `$NAME` against
`side_resolved_names` before claiming it and errors with a pointer
to the prior binding if the name is already taken — so
``--input-json routes=a.json --input yaml routes=b.yaml`` fails
fast rather than silently rebinding `$routes`.  The check covers
every typed-vs-typed and typed-vs-generic combination because all
flags route through the same helper.

## Python API integration

`dialects/f5/query/api.py` exposes the public surface external
scripts use:

| Symbol | Role |
|---|---|
| `Query` / `Query.run` | Lower-level entry — parsed-and-ready query, run against pre-staged sources.  Forwards to `run_query` and stashes the source map on the returned `QueryRun` so renderers can recover it. |
| `q(*args, parser=None)` | Polymorphic single-call entry.  First non-file-path string is the expression; everything else is an input (file path / Path / Sources / prior QueryRun / dict / list).  See "Chain semantics" below. |
| `load(*inputs, parser=None)` | Pre-stage files / in-memory values into a `Sources` object that ``q()`` consumes the same way it consumes a path.  `parser` accepts a registered format name OR an inline callable. |
| `Sources` | Container of `{uri: text}` + per-uri `InputSpec` + inline parsers.  Composes via `.merged(other)`. |
| `QueryRun` | Result wrapper: list-like (iter / len / `[i]` / `bool`), plus `.values()` / `.first()` / `.objects()` / `.paths()` / `.rows()`. |
| `QueryRun.q(expr, *extras)` | Method form of progressive chaining — `run.q(expr)` ≡ `q(expr, run)`. |
| `QueryRun.render(name_or_callable, **opts)` | Dispatches through the renderer registry on a string name, or calls the function directly on a callable.  Binds source text into `RENDER_SOURCES` for the call. |
| `QueryRun.out()` | Walks values and coerces `ObjectRef` / `PathRef` / `Stream` to plain JSON-compatible Python.  The explicit boundary between engine-typed handles and serialisable data. |
| `QueryRun.edits(uri)` | Post-edit source text for *uri*, or `None`.  CLI's `--in-place` is `Path(uri).write_text(run.edits(uri))`. |

```python
def render(self, renderer, **opts):
    from .renderers import bind_render_sources, render as _render

    with bind_render_sources(self._sources):
        if isinstance(renderer, str):
            return _render(renderer, self.values(), **opts)
        return renderer(self.values(), **opts)
```

The `isinstance(renderer, str)` branch — rather than `callable()` —
is deliberate: ty's `call-top-callable` rule rejects narrowing a
`str | Callable[...]` union by `callable()` because every `str` is
also a `Top[Callable[...]]` for type-checking purposes.

### Chain semantics

`q()` and `QueryRun.q()` accept one or more prior `QueryRun`
instances as inputs.  Their flattened values are serialised to JSON
and synthesised as a single source:

- **Only priors** — the synthesised JSON list becomes the primary
  input; `.` reads it, `.[]` iterates per prior value.  The runner's
  all-JSON fallback (in `run_query`) makes this work without a
  BIG-IP source to act as primary.
- **Priors mixed with other inputs** — the priors land under the
  name `$_chain` (one combined list across every prior); files /
  in-memory data act as primary.  The runner's primary-iteration
  loop skips URIs that appear in `names` even in all-JSON mode, so
  the chain source never double-iterates as `.`.

The `$_chain` skip rule is what makes
`f5q.q(expr, prior, [in_memory_data])` evaluate *expr* exactly
once against the in-memory data — without it the loop would iterate
both JSON sources as primary and the user would see duplicated
output.

## Builtin contract (`@builtin`)

```python
@builtin(
    name: str,
    *,
    summary: str = "",
    signatures: tuple[str, ...] = (),
    examples: tuple[str, ...] = (),
    category: str = "user",
    min_args: int = 0,
    max_args: int | None = None,
    details: str = "",
    special_form: bool = False,
    with_ctx: bool = False,
    stream_aware: bool = False,
)
```

`@builtin` is a thin public wrapper over the private
`_register` decorator the in-tree builtins use.  The default
`category="user"` keeps plugin builtins out of the curated
in-tree categories shown at the top of `--help-builtins`; setting
`category` to one of the in-tree slots (`stream`, `string`,
`path`, `rename`, `net`, `graph`, `value`) groups the plugin
alongside the matching in-tree builtins.

The advanced flags map directly onto `BuiltinSpec`:

- `stream_aware=True` — receives the whole stream as one argument
  (`count`, `sort`, `any`, …).
- `special_form=True` — receives the unevaluated AST plus an
  `EvalContext`; the function must drive its own `current`
  rebinding (`select`, `map`).
- `with_ctx=True` — receives the `EvalContext` as a keyword
  argument so the builtin can queue cascading edits or look up the
  active `Root` (`rename_partition`).

Plugin builtins raise `BuiltinError` for argument-type mistakes;
the runner re-raises as `error: <msg>` from the CLI just like the
in-tree builtins.

## Input-format contract (`@input_format`)

```python
@input_format(name: str, *, summary: str = "", details: str = "")
def parse(source: str, *, uri: str, options: tuple[tuple[str, Any], ...] = ()) -> Any: ...
```

- *source* — file contents as text (UTF-8 decoded by the loader;
  binary formats need pre-decoding).
- *uri* — the file path / URI, for inclusion in error messages.
- *options* — `InputSpec.options` verbatim; the parser reads the
  keys it understands and ignores the rest.  CSV uses
  ``("headers", (col1, col2, ...))`` as the only built-in option
  today.
- Return value — any Python value the DSL can navigate naturally.
  Lists become iterable streams via the DSL's `$name[]` operator;
  dicts navigate with `$name.field`.

The runner replaces its previous hard-coded `if spec.kind == "json"
…` chain with `inputs.lookup(spec.kind)`.  An unknown kind raises
`QueryError(f"unknown input format {kind!r} (registered: ...)")`
listing every registered name — much more useful than a bare
"unknown" error when the user typo'd `--input jzon`.

User parsers raise `QueryError` for caller-visible problems; the
runner's catch wraps the more permissive `ValueError` /
`InputError` shapes the in-tree parsers raise so they still
surface cleanly.

## Plugin loader (XDG auto-discovery)

`dialects/f5/query/plugins.py`:

```python
def xdg_plugin_dir() -> Path:
    """$XDG_CONFIG_HOME/dialects/f5/query/plugins/, falling back to ~/.config/dialects/f5/query/plugins/."""


def load_user_plugins(*, force: bool = False) -> list[Path]:
    """Auto-import *.py under xdg_plugin_dir().  Idempotent."""
```

Each registry's `_ensure_loaded` (`builtins._ensure_plugins_loaded`,
`inputs._ensure_loaded`, `renderers._ensure_builtins_loaded`) calls
`load_user_plugins()` after the in-tree built-ins finish
registering.  The first call to any `lookup` / `list_*` helper
across any registry triggers the scan; subsequent calls are a flag
check.

The loader's invariants:

- **Hidden files (`_*.py`) are skipped** — convention for helper
  modules a plugin imports privately.  Sub-directories are
  recursively scanned.
- **Load order is `(depth, full path)`** — top-level plugin files
  load before sub-folder files; within each tier files load
  alphabetically.  The order is reproducible across runs and
  matches the way users read the tree.
- **Sibling imports work** — `_import_plugin_file` temporarily
  prepends `path.parent` to `sys.path` for the duration of
  `exec_module` and removes it in a `finally`.  A plugin can
  `import helper` to pull in a sibling `helper.py` (or
  `_helper.py` — the scanner skips them but they stay
  importable).  `sys.path` doesn't accumulate state between
  plugin imports.
- **Module names are uniquified** (`f5q_user_plugin_<stem>_<hash>`)
  so a `force=True` re-scan doesn't collide with the prior
  incarnation in `sys.modules` — required for the test fixture
  pattern.
- **Warn-not-crash** — every `Exception` raised during import is
  reported to stderr as `f5q: warning: failed to load plugin
  <path>: <exc>` and the file is skipped.  One broken plugin must
  never kill the CLI or a script that doesn't use it.
- **Idempotent by default** — `load_user_plugins()` returns
  immediately after the first call.  `force=True` re-scans **only
  files the loader hasn't already imported successfully**
  (tracked in `_LOADED_FILES`).  This keeps force-reload free of
  duplicate-registration warnings the decorators would otherwise
  emit, and surfaces only the genuinely new file in the return
  value.  Files that previously *failed* (broken syntax, missing
  import) are still retried in case the user just fixed them.

The CLI exposes `f5 q --help-plugins` as a diagnostic action that
prints the scan directory and the list of loaded files; pair with
`2>&1` to see warnings inline.

## Open questions / future work

- **Entry-point discovery.**  We deliberately defer
  `setuptools.entry_points`-based plugin discovery to v2.  When we
  add it, the natural group names are `f5q.renderers`,
  `f5q.builtins`, and `f5q.input_formats` and the discovery should
  happen inside the same `_ensure_loaded` hooks the XDG loader
  uses so the CLI and the Python API pick up the same plugin set.
- **APM-policy renderer.**  The `ascii-blocks` renderer is the
  scaffold for the future TMUI-style APM policy view (tracked in a
  separate spec).  When that lands it can re-use `_Box` and
  `_render_box` directly rather than reimplementing the tree walker.
- **Multi-file Mermaid.**  Today the Mermaid renderer's
  ObjectRef-mode emits one diagram per CLI invocation.  Per-file
  dispatch (one `graph` block per source URI, with cross-file edges
  inside a `subgraph`) is the natural follow-on for `--merge` mode.
- **Custom operators.**  True new infix operators (`a <op> b`) would
  need lexer + parser + precedence-table changes — an order of
  magnitude more work than the function-form `op(a, b)` builtins
  cover today.  Tracked but not planned; the `@builtin` decorator
  is the recommended extension point for any operator-like
  function.
