# `f5 query` output contract — renderers, builtins, and input formats

The query engine (`rust/tcl-bigip-query`) carries three extension surfaces:
**output renderers**, **DSL builtin functions**, and **side-input parsers**.
Each is a static, in-tree registry built into the crate; the `f5 query` verb
in `rust/f5-cli` and the PyO3 binding in `rust/bigip-report-gen/python` both
consult the same registries, so the three surfaces behave identically
whichever front-end drives them.

This doc fixes the contracts: what each registry entry must do, what the
engine guarantees, and how errors propagate.

> **No runtime plugin loading.** There is no decorator API, no
> user-plugin directory, and no entry-point discovery. Every renderer,
> builtin, and input format is a Rust function compiled into the crate.
> Extending a surface means adding an entry to the matching registry and
> rebuilding. An earlier Python implementation loaded user plugins from an
> XDG directory; that engine is retired, and the `--help-plugins` action
> that listed them no longer exists.

## Registry layout

| Module | Role |
|---|---|
| `renderers` | `RendererSpec`, the static renderer registry, `lookup`, `list_renderers`, `render`. |
| `renderers::{gantt, ascii_blocks, mermaid}` | The three built-in renderers. |
| `inputs` | `InputFormatSpec`, `InputSpec`, `list_input_formats`, `is_registered`, `parse_input`. |
| `builtins` | `BuiltinSpec`, the `Builtin` dispatch enum, `lookup`, `all_specs`, `format_catalogue`. |
| `special` | Dispatch for the special-form builtins, against the unevaluated AST. |
| `output` | `render` / `render_with_opts` — the built-in output modes, falling through to the renderer registry for anything else. |

## Renderer contract

```rust
type RenderFn = fn(&[Value], &BTreeMap<String, String>) -> Result<String, QueryError>;

pub struct RendererSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub accepts: &'static str,
    pub details: &'static str,
    pub impl_fn: RenderFn,
}
```

- `name` — the lookup key used by `lookup`, `render`, and the CLI's
  `-R` / `--render NAME` flag. Convention: lowercase, dash-separated
  (`ascii-blocks`, not `AsciiBlocks` or `ascii_blocks`).
- `summary` — the one-line description `--help-renderers` prints.
- `accepts` — a free-text shape hint, also printed by `--help-renderers`.
  The registry does **not** enforce it; it is documentation, not
  validation.
- `details` — optional multi-paragraph prose, surfaced verbatim by
  `--help-renderers`.
- `impl_fn` — the renderer itself.

A renderer must be:

- **Pure.** It must not print, must not write files, and must not mutate
  its values. The CLI dispatches it once per source file.
- **Total over its declared shape.** If `accepts` names a shape, every
  legal instance of that shape must render.
- **Tolerant of unknown shapes.** When the values do not match `accepts`,
  prefer a reasonable fallback over an error. The `mermaid` renderer's
  chain mode is the model. Reserve `QueryError::Renderer` for input that
  genuinely cannot be rendered — a required key missing from a dict the
  renderer documented as mandatory, for instance.
- **Self-validating over its options.** Options arrive as raw strings from
  `--render-opt KEY=VALUE`. The renderer parses, type-checks, and clamps
  them itself, and raises `QueryError::Renderer` with a message the CLI can
  prefix with `error:`.

The return value is a single string written verbatim to stdout.
Conventionally it ends with a newline so multi-file runs stack cleanly.

`REGISTRY` is kept sorted by name at definition time, so `list_renderers`
and the "registered: …" list in the unknown-renderer error are both sorted.

### The built-in renderers

| Name | Accepts | Options |
|---|---|---|
| `ascii-blocks` | a `{title, rows}` dict, or a list of them | `style` (`rounded` default, `square`, `ascii`), `min-width` (default 12) |
| `gantt` | rows of `(timestamp, label, state)` — a TSV string, 3-tuples, or dicts | `unit-minutes` (a positive divisor of 60, default 5), `year` |
| `mermaid` | any ordered stream | `direction` (`LR` default, `RL`, `TB`, `BT`), `reverse`, `max-depth` |

`mermaid` renders a left-to-right chain of nodes. It also carries an
ObjectRef graph mode, but that mode is unreachable today: no source map is
threaded into `output::render`, because the renderer runs after the query
has completed and the CLI dispatch path holds no sources by then. Its
options are still validated, and the renderer falls back to chain mode
rather than erroring. For the reference graph, use
`f5 graph --format mermaid`, which owns the source text it walks.

## Output-mode dispatch

`output::render_with_opts(values, mode, opts)` is the single dispatch point.
The built-in modes — `auto`, `scf`, `raw`, `paths`, `json`, `table`, and
`table-lineart` — ignore `opts`. Any other mode falls through to
`renderers::lookup`, so `--render NAME` needs no separate code path in the
verb; it simply sets the mode. An unrecognised mode returns
`QueryError::Renderer("unknown output mode: …")`.

## Error mapping

Every caller-visible failure is a `QueryError` variant:
`Lex`, `Parse`, `Eval`, `Edit`, `Builtin`, and `Renderer`. `Lex` and
`Parse` carry a source offset and display as `{message} at offset {offset}`;
the rest are a bare message.

The CLI prints `error: {e}` to stderr and exits `2` for any of them. The
PyO3 binding maps every one to the Python `QueryError` exception with the
same message text, so a script sees exactly what the CLI would have
printed.

## CLI integration

`f5 query` (alias `f5 q`) exposes:

1. `--render NAME` / `-R NAME` — dispatch through the renderer registry.
   Overrides the output-mode flags (`--scf`, `--raw`, `--paths-only`,
   `--json`, `--table`, `--table-lineart`).
2. `--render-opt KEY=VALUE` — repeatable, parsed into a flat map and
   forwarded to the renderer. A duplicate key takes the last value; an
   entry with no `=`, or an empty key, is an error.
3. `--input KIND NAME=PATH` — the generic side-input flag, dispatched
   through the input-format registry. The typed `--input-json`,
   `--input-jsonl`, `--input-csv`, and `--input-f5log` shorthands remain.
4. `--help-renderers`, `--help-inputs`, `--help-builtins [NAME]`,
   `--help-dsl`, `--help-examples`, and `--help-manual` — catalogue
   actions that print and exit. The builtins catalogue is generated from
   the registry metadata (name, category, arity, and flags), so it stays
   in step with the registry by construction.

### Side-input name dedup

Every `--input*` flag routes through one shared binder, which refuses a
`$NAME` already claimed by a prior side input, and refuses a URI already
loaded as a positional or side input. So
`--input-json routes=a.json --input yaml routes=b.yaml` fails fast rather
than silently rebinding `$routes`. The check covers every typed-versus-typed
and typed-versus-generic combination because all flags share the binder.

## Builtin contract

```rust
pub enum Builtin {
    Plain(fn(&[Value]) -> Result<Value, QueryError>),
    Ctx(fn(&[Value], &mut EvalContext) -> Result<Value, QueryError>),
    Special,
}

pub struct BuiltinSpec {
    pub name: &'static str,
    pub category: &'static str,
    pub min_args: usize,
    pub max_args: Option<usize>,
    pub special_form: bool,
    pub with_ctx: bool,
    pub stream_aware: bool,
    pub broadcasts: bool,
    pub imp: Builtin,
}
```

The registry is a `OnceLock<HashMap<&str, BuiltinSpec>>` built on first
lookup from the per-family modules (`string`, `regex_str`, `math`, `net`,
`graph`, `rename`, `time_dt`, `value2`, `encoding`, `files`,
`inputs_load`, `f5profile`, and `extras`), plus the special forms and the
feature-gated network probes. `all_specs` returns them sorted by
`(category, name)`, which is the order `--help-builtins` prints.

The dispatch flags:

- `stream_aware` — the builtin receives the whole stream as one argument
  rather than element-wise.
- `special_form` — dispatched by `special` against the unevaluated AST, so
  the builtin drives its own rebinding of the current value (`select`,
  `map`).
- `with_ctx` — receives the `EvalContext`, for builtins that queue
  cascading edits or need the active root.
- `broadcasts` — whether a stream argument broadcasts element-wise. This
  is the scalar default; `with_ctx` builtins normally opt out, with
  `refs` / `referenced_by` the exception.

Builtins raise `QueryError::Builtin` for argument-type mistakes, which the
CLI maps to `error:` uniformly.

## Input-format contract

```rust
pub struct InputFormatSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub details: &'static str,
}

pub struct InputSpec {
    pub kind: String,
    pub csv_headers: Option<Vec<String>>,
}
```

`parse_input(source, uri, spec)` turns a side-input file's text into a
`Value` the DSL can navigate: a list becomes an iterable stream under
`$name[]`, and a dict navigates with `$name.field`. `uri` is carried for
error messages. `csv_headers` is the only per-format option the CLI
exposes, set from `--input-csv NAME=PATH:hdr1,hdr2,…`.

Five formats are registered: `csv`, `f5log`, `json`, `jsonl`, and `zone`.
An unknown kind raises a `QueryError` naming every registered format,
which is far more useful than a bare "unknown" when the user has typed
`--input jzon`.

## Python binding surface

`rust/bigip-report-gen/python` exposes the engine to Python as the
`f5report` package (native module `f5report._engine`). `query()` runs an
expression against in-memory `(uri, text)` sources and converts engine
values to native Python objects — an object reference becomes a dict of
`kind` / `full-path` / `fields`, a path reference becomes its full-path
string, and a stream becomes a list. The binding is read-only: a mutating
expression raises `QueryError` rather than rewriting anything.

Renderers are not exposed to Python. Rendered output is a CLI surface; a
script that wants a chart either calls `f5 query --render …` or formats the
returned values itself. See
[the scripting how-to](../kcs/kcs-howto-script-against-f5-query-from-python.md).

## Open questions / future work

- **Reachable ObjectRef mermaid mode.** Threading the source map into
  `output::render` would let the `mermaid` renderer build the reference
  graph in place, instead of deferring to `f5 graph --format mermaid`.
- **Multi-file mermaid.** One diagram per source URI, with cross-file
  edges inside a subgraph, is the natural follow-on for `--merge` mode.
- **APM-policy renderer.** `ascii-blocks` is the scaffold for a future
  TMUI-style APM policy view; that work can reuse its box renderer rather
  than reimplementing the tree walker.
- **Custom operators.** True infix operators (`a <op> b`) would need
  lexer, parser, and precedence-table changes — an order of magnitude more
  work than the function-form `op(a, b)` builtins already cover. The
  builtin registry stays the recommended extension point for anything
  operator-shaped.
