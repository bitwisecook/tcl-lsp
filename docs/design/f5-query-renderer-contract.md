# `f5 query` extension registries — renderers, builtins, and input formats

The query engine carries three registries: output **renderers**, DSL
**builtin** functions, and side-input **format** parsers. Each is a static,
compile-time catalogue in `rust/tcl-bigip-query`, so a lookup is
allocation-free and the whole set is enumerable for `--help-*` without
running anything.

This document fixes the contracts: what each shape must do, what the engine
guarantees, where the registrations live, and how errors propagate.

## Registry layout

| File | Role |
|---|---|
| `renderers/mod.rs` | `RendererSpec`, the `REGISTRY` const, `lookup`, `list_renderers`, `render`. |
| `renderers/gantt.rs` | The `gantt` renderer — ASCII timeline. |
| `renderers/ascii_blocks.rs` | The `ascii-blocks` renderer — Unicode line-art tree. |
| `renderers/mermaid.rs` | The `mermaid` renderer — Mermaid diagram. |
| `builtins/mod.rs` | `BuiltinSpec`, the `REGISTRY` `OnceLock`, `lookup`, `all_specs`. |
| `builtins/*.rs` | The builtins themselves, one module per category. |
| `special.rs` | The special-form builtins (`select`, `map`, `if`, `as`, the `paths` / `getpath` family). |
| `inputs.rs` | `InputFormatSpec`, `InputSpec`, `list_input_formats`, `parse_input` and the per-format parsers. |
| `output.rs` | `render(values, mode)` — falls through to the renderer registry on a mode that is not a built-in output shape. |

Everything is `const` or `OnceLock`-initialised. There is no dynamic
registration, no plugin discovery, and no user-supplied code path: the set
of renderers, builtins, and input formats is fixed at compile time, which
is what lets the engine run in a WASM console with no filesystem and no
loader.

## `RendererSpec`

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
  `-R / --render NAME` flag. Convention: lowercase, dash-separated
  (`ascii-blocks`, not `AsciiBlocks` or `ascii_blocks`).
- `summary` — the one-line description `--help-renderers` prints.
- `accepts` — a short free-text shape hint ("stream of `ObjectRef`", "rows
  of `(timestamp, label, state)`"). The registry **does not enforce** it;
  it is documentation, not validation.
- `details` — optional multi-paragraph prose surfaced verbatim by
  `--help-renderers`.
- `impl_fn` — the renderer itself.

`REGISTRY` is sorted by name at definition time, so `list_renderers()` and
the registered-names list in the unknown-renderer error are both already
sorted.

## Renderer contract

A renderer is `fn(&[Value], &BTreeMap<String, String>) -> Result<String, QueryError>`.

1. **Take the values as they come.** The engine hands over whatever the
   query produced. A renderer that needs a particular shape validates it
   and returns `QueryError::Renderer` with a message naming the shape it
   expected — it never panics on a shape mismatch.
2. **Read options from the map, never from the environment.** Options
   arrive from `--render-opt key=value`; a `BTreeMap` keeps iteration
   order stable so error messages and help text are deterministic.
   Unknown keys are ignored rather than rejected, so a shared option name
   across renderers stays harmless.
3. **Return text, not bytes, and do not print.** The caller owns stdout
   and file routing.
4. **Be pure.** A renderer does no I/O and consults no network. Anything
   needing live data is a probe builtin, not a renderer.

The three built-in renderers show the range:

| Renderer | Accepts | Options |
|---|---|---|
| `ascii-blocks` | a `{title, rows}` dict or a list of them | `style=rounded\|square\|ascii` |
| `gantt` | rows of `(timestamp, label, state)` as a TSV string, 3-tuples, or dicts | `unit-minutes=N` (a divisor of 60) |
| `mermaid` | a stream of `ObjectRef` (BIG-IP graph mode) or any ordered stream (chain mode) | `max-depth=N`, `reverse=true` |

`mermaid`'s ObjectRef mode reuses the same engine as
`f5 graph --format mermaid`, walking the reference graph from each
`ObjectRef` seed. Its generic mode renders the values as a left-to-right
chain of nodes, which suits an ad-hoc `{stage, next}` projection.

`gantt` renders one row per distinct label, with `v` for a DOWN
transition, `^` for UP, and `#` for the spans the label was down.

## Error mapping

`render(name, values, opts)` looks the name up and dispatches. An
unregistered name produces

```
unknown renderer '<name>' (registered: ascii-blocks, gantt, mermaid)
```

as a `QueryError::Renderer`, listing every registered name so a typo is
self-correcting. Any error the renderer itself raises propagates
unchanged, so the CLI maps all of them to `error:` uniformly.

## `BuiltinSpec`

```rust
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

The five behaviour knobs are the whole contract between a builtin and the
evaluator:

- `min_args` / `max_args` — a strict arity check the evaluator performs
  before dispatch, so no builtin re-implements arity errors.
- `special_form` — the builtin receives the AST nodes rather than
  evaluated values, so it controls when and whether each argument is
  evaluated. This is how `select`, `map`, `if`, and `as` get their
  short-circuit and iteration semantics. Special forms live in
  `special.rs`.
- `with_ctx` — the builtin receives the `EvalContext`, which is the only
  route to the root, the edit plan, the merge state, the probe gate, and
  the host reader hooks.
- `stream_aware` — the builtin handles stream semantics itself rather than
  having the dispatch unwrap streams for it.
- `broadcasts` — whether stream arguments broadcast element-wise. This is
  the scalar default. A `with_ctx` builtin normally skips broadcast;
  `refs` and `referenced_by` are the deliberate exception, broadcasting
  *and* taking `ctx` for the config.

`lookup(name)` is the evaluator's dispatch hook; `all_specs()` returns
every builtin sorted by `(category, name)` for `--help-builtins`. The
category vocabulary orders that help output: stream, string, math, time,
path, rename, net, graph, value.

A plain builtin raises `QueryError::Builtin` for an argument-type mistake,
so the CLI maps every one of them to `error:` uniformly.

An operator that needs short-circuit evaluation should almost always be a
special-form builtin rather than a new AST node — that is the cheap
extension point. A true new infix operator (`a <op> b`) needs lexer,
parser, and precedence-table changes, an order of magnitude more work than
the function form `op(a, b)` a builtin already gives you.

## `InputFormatSpec`

```rust
pub struct InputFormatSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub details: &'static str,
}
```

`list_input_formats()` returns the catalogue sorted by name — `csv`,
`f5log`, `json`, `jsonl` — for `--help-inputs`. `zone` parses DNS zone
files through the same dispatcher.

An `InputSpec` is what a *call site* passes: `kind` names the format, and
`csv_headers` carries the one per-format knob the CLI exposes
(`--input-csv NAME=PATH:hdr1,hdr2,…`, which skips header-row discovery so
every line of the source is treated as data).

`parse_input(source, uri, spec)` dispatches to the format's parser, each
of which returns a `Value` tree the DSL navigates with native
dict/list semantics:

- **json** — a single document.
- **jsonl** — NDJSON. Blank lines are skipped; a per-line error carries
  its line number.
- **csv** — a list of row objects. The header is auto-detected from row 1
  unless `csv_headers` overrides it; extra columns land in an `extra`
  list, and missing columns become the empty string.
- **f5log** — F5 syslog, one object per event, against the eight-value
  syslog severity vocabulary.

Every parser reports a bad document as a `QueryError` naming the format
and the position, so a malformed side-input never surfaces as an
unhelpful evaluation error deep in the query.

## CLI integration

| Flag | Effect |
|---|---|
| `-R / --render NAME` | Route the query's values through the named renderer instead of `--output`'s mode. |
| `--render-opt KEY=VALUE` | Add one entry to the renderer's option map; repeatable. |
| `--help-renderers` | Print every `RendererSpec`'s name, summary, accepts hint, and details. |
| `--help-builtins [NAME]` | Print the builtin catalogue, or one builtin's entry. |
| `--input-{json,jsonl,csv,f5log} NAME=PATH` | Bind `$NAME` to a side-input parsed with that format. |

A side input participates in the multi-file source count — a single config
plus one side input renders with a per-file banner — but never iterates as
the primary `.` input.

## Adding to a registry

**A renderer:** add a module under `renderers/`, add its `RendererSpec` to
`REGISTRY` in name order, and give it a real `accepts` hint and `details`
so `--help-renderers` documents it without a second edit.

**A builtin:** add the implementation to the category module it belongs to
under `builtins/`, register its `BuiltinSpec`, and set the behaviour knobs
deliberately — most builtins want the plain scalar defaults. A special
form goes in `special.rs` instead.

**An input format:** add the parser to `inputs.rs`, add its arm to
`parse_input`, and add its `InputFormatSpec` to `list_input_formats` in
name order.

In every case, add a regression test in the crate's own tests, because the
`--help-*` output is part of the CLI's observable surface.

## Related

- [`f5-query-engine-internals.md`](f5-query-engine-internals.md) — the
  engine these registries plug into.
- [`f5-cli-architecture.md`](f5-cli-architecture.md) — verb registry and
  command dispatch.
