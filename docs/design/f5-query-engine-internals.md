# `f5 query` — Engine Internals

Architecture, invariants, and data-flow for the `f5 query` engine.
This is the contributor doc — what you need to know to extend or
debug the query DSL.  The user-facing surface (every function,
operator, flag, behaviour) lives in
[`docs/references/f5_query/`](../references/f5_query/).

Symbol names below are quoted verbatim from the source so the doc
is grep-able against the codebase.

## Module layout

```
rust/tcl-bigip-query/src/
├── lib.rs                # Public re-exports: run_query, render, the value model
├── lexer.rs              # Tokeniser — single pass, one-char lookahead
├── parser.rs             # Recursive-descent parser → ast::Expr nodes
├── ast.rs                # The Expr enum: every expression form
├── eval.rs               # Walks the AST against a Root / BigipConfig; EvalContext
├── special.rs            # Special-form builtins (select, map, if, as, …)
├── projection.rs         # Lazy Container / ObjectRef tree over a BigipConfig
├── builtins/
│   ├── mod.rs            # BuiltinSpec, the registry, shared coercion helpers
│   ├── string.rs, math.rs, time_dt.rs, regex_str.rs, encoding.rs, value2.rs
│   ├── net.rs, graph.rs, rename.rs, files.rs, f5profile.rs
│   ├── inputs_load.rs    # json_load / jsonl_load / csv_load / f5log_load
│   └── extras.rs
├── probes/               # Network builtins (gated by --enable-probes)
│   ├── http.rs
│   └── tls.rs
├── inputs.rs             # External input parsers (JSON / JSONL / CSV / f5log / zone)
├── value.rs              # Value, ObjectRef, PathRef, FieldSlot
├── edit_plan.rs          # EditOp, EditPlan, PrefixRewrite, apply()
├── rewrite.rs            # Token-bounded source rewriter (the rename half)
├── output.rs             # Renderers: auto / scf / raw / paths / json / tmsh / table
├── renderers/            # Pluggable output renderers: gantt, ascii-blocks, mermaid
├── jsonfmt.rs            # Canonical JSON serialisation of a Value
├── architecture.rs       # Multi-device tiering across loaded configs
├── runner.rs             # run_query orchestration, multi-statement loop
├── grammar.rs            # `--help-dsl` plain-text grammar reference
├── examples.rs           # `--help-examples` cookbook
├── manual.rs             # The combined `--help-manual` surface
└── errors.rs             # QueryError — parse / eval / builtin / input variants
```

## Pipeline

A query goes through five distinct stages, each owned by one
module:

1. **Lex** — `lexer::tokenise(source)` → `Vec<Token>`.
2. **Parse** — `parser::parse(source)` → `ast::Program`.
3. **Project** — `projection` lazily wraps a
   `BigipConfig` in navigable `Container` / `ObjectRef` nodes
   the evaluator can index into.
4. **Evaluate** — `eval::evaluate(program, ctx)` →
   `Vec<Value>`.  Assignment nodes emit `EditOp` into
   `ctx.edits.ops` rather than mutating in place.
5. **Apply + render** — `runner::run_query` applies the edit
   plan to each source's text via `edit_plan::apply`, re-parses
   the rewritten text, and hands the values + sources to
   `output::render`.

Multi-statement queries (`a ; b ; c`) loop over statements; each
statement sees the edits applied by the previous one.

### Data flow

```
                source text                      query text
                     │                                │
                     │                                ▼
                     │                       ┌────────────────┐
                     │                       │   lexer.rs     │
                     │                       │  tokenise()    │
                     │                       └───────┬────────┘
                     │                               │ Vec<Token>
                     │                               ▼
                     │                       ┌────────────────┐
                     │                       │   parser.rs    │
                     │                       │   parse()      │
                     │                       └───────┬────────┘
                     │                               │ ast::Program
                     │                               │
                     ▼                               │
            ┌────────────────┐                       │
            │ parse_bigip    │                       │
            │ _conf()        │                       │
            └───────┬────────┘                       │
                    │ BigipConfig                    │
                    │   + source ranges              │
                    ▼                                │
            ┌────────────────┐                       │
            │ projection.rs  │  ◄──────── lazy access from evaluator
            │                │                       │
            │ Container /    │                       │
            │ ObjectRef tree │                       │
            └───────┬────────┘                       │
                    │                                ▼
                    │            ┌──────────────────────────────┐
                    └───────────►│           eval.rs            │
                                 │   evaluate(program, ctx)     │
                                 │                              │
                                 │  reads  ─► ObjectRef / Stream│
                                 │  emits  ─► EditOp into       │
                                 │            ctx.edits.ops     │
                                 └──────────────┬───────────────┘
                                                │ Vec<Value>
                                                │  + EditPlan
                                                ▼
                                        ┌───────────────┐
                                        │ edit_plan.rs  │
                                        │   apply()     │
                                        └───────┬───────┘
                                                │ AppliedSource
                                                │  (new_source)
                                                ▼
                                        ┌───────────────┐
                                        │   output.rs   │
                                        │   render()    │
                                        └───────┬───────┘
                                                │ rendered text
                                                ▼
                                          stdout / files
```

The reparse step inside `edit_plan::apply` (not shown — it round-
trips `new_source` through `parse_bigip_conf` to validate) is
what makes mutation safe: the engine never ships output a fresh
parse couldn't read back in.

## Module-by-module reference

### `lexer.rs` — Tokeniser

Hand-rolled scanner.  Single pass, one-character lookahead.  Key
symbols:

- `TokenKind` (enum) — 43 token classes.  Operators (`Pipe`,
  `PipeEq`, `Plus`, `PlusEq`, `Minus`, `MinusEq`, `Star`, `Slash`,
  `Eq`, `EqEq`, `Neq`, `Lt`, `Le`, `Gt`, `Ge`), path and grouping
  syntax (`Dot`, `LBracket`, `RBracket`, `LParen`, `RParen`,
  `LBrace`, `RBrace`, `Comma`, `Colon`, `Semicolon`, `Question`),
  keywords (`And`, `Or`, `Not`, `If`, `Then`, `Elif`, `Else`,
  `End`, `As`, `True`, `False`, `Null`), literals and names
  (`Number`, `String`, `Ident`, `DollarIdent`), and `Eof`.
  `TokenKind::as_str` gives each an upper-case display spelling for
  error messages and the differential fixtures — note that a few
  are not a mechanical shout-case of the variant (`EqEq` renders
  `EQEQ`, `Neq` renders `NEQ`, `Le` / `Ge` render `LE` / `GE`).
  There is no dedicated regex token: a regex literal is a `String`
  the builtins compile.
- `Token` (struct) — carries `kind`, `text`, `offset`, and a
  `value: LitValue` for already-parsed literals (`LitValue::Null`
  when the token carries none).
- `tokenise(source: &str) -> Result<Vec<Token>, QueryError>` — entry point.  Comment
  syntax is `#` to end-of-line.  Barewords (identifiers) may
  contain `-` so TMSH names like `source-address-translation`
  lex as one ident.
- `keyword(text) -> Option<TokenKind>` — the keyword table
  (module-private).

**Offsets are code-point indices, not byte offsets.** The source is
scanned as a `Vec<char>` and `Token::offset` matches `source[i]`
indexing exactly, so a non-ASCII string literal earlier in the query
does not skew a later reported position.

If you need to add a new operator, this is the first edit — add
the `TokenKind`, its `as_str` spelling, the two-char lookahead in
`tokenise`, then wire it into the parser's precedence cascade.

### `parser.rs` — Recursive-descent parser

Hand-written, no parser generator.  Grammar in
[`docs/references/f5_query/dsl.md`](../references/f5_query/dsl.md).

- The parser is stateful: it holds the token stream and a position
  index. Most of its methods are private precedence layers.
- `parse(source: &str) -> Result<Program, QueryError>` — public entry; calls
  `tokenise` then `_Parser(tokens).parse_program()`.
- `parse_program() → Program` — handles multi-statement `;`
  separation.
- Precedence cascade (top→bottom): pipe → comma-stream →
  assignment → ternary `if` → `or` → `and` → `not` →
  comparison → addition → multiplication → unary → primary.
  Each layer descends to the next.
- The pipe-stage layer emits assignment operators (`=`, `|=`,
  `+=`, `-=`) as trailing ops on the LHS, not infix nodes.
- The primary layer handles literals, variable refs (`$name`), object
  literals (`{k: v}`), list literals (`[…]`), `if/then/else`
  blocks, parenthesised expressions, and the path-expression
  prefix (`.` or `..`).
- The assignment- and comparison-operator tables map an operator token to
  the AST node tag.

To add a new infix operator: pick the precedence level, extend the
comparison-operator table (or write the layer yourself), mint a
`BinOp { op }` token, then handle it in `eval.rs`'s binary-operator arm.

### `ast.rs` — AST node types

Every form is a variant of one `Expr` enum, and every variant carries an
`offset: usize` — the byte offset into the source, used to underline the
error. The variants:

| Node | Carries | Form |
|---|---|---|
| `Literal` | `value: LitValue` | `42`, `"foo"`, `true`, `null` |
| `Identity` | — | bare `.` |
| `Variable` | `name: String` | `$x` |
| `Field` | `name: String`, `optional: bool` | `.foo` / `.foo?` |
| `Subscript` | `index, stream, regex, optional` | `[i]` / `[]` / `[~"re"]` |
| `PathExpr` | `head: Option<Box<Expr>>`, `steps: Vec<PathStep>` | `.a.b[0]` / `$x.a.b` |
| `ObjectLiteral` | `entries: Vec<(String, Expr)>` | `{k: v, k2}` |
| `ListLiteral` | `inner: Option<Box<Expr>>` | `[ ... ]` |
| `Call` | `name: String`, `args: Vec<Expr>` | `select(.x > 1)` |
| `BinOp` | `op: String`, `lhs`, `rhs` | `a + b` |
| `UnaryOp` | `op: String`, `operand` | `-x`, `not x` |
| `Pipe` | `lhs, rhs` | `a | b` |
| `Assignment` | `op: String`, `target: PathExpr`, `value` | `.x = 1` / `.x |= f` |
| `LetBinding` | `source`, `name`, `body` | `1 as $x | …` |
| `IfThenElse` | `condition, then_body, elif_branches, else_body` | `if/then/elif/else/end` |
| `CommaStream` | `parts: Vec<Expr>` | `a, b, c` |
| `Program` | `statements: Vec<Expr>` | `a ; b ; c` |

Assignment targets must always be `PathExpr` (or `Field` /
`Subscript` chains rooted in one); the evaluator rejects writes
to literals or calls at runtime.

### `eval.rs` — AST walker

`EvalContext` carries every piece of per-run state the walker needs, so
nothing is ambient:

```rust
pub struct EvalContext {
    pub root: Rc<Root>,
    pub named_roots: HashMap<String, Rc<Root>>,   // --name=alias=path
    pub merge_mode: bool,                         // --merge active
    pub merge_roots: Vec<Rc<Root>>,               // every loaded root, in order
    pub bindings: HashMap<String, Value>,         // `expr as $name` bindings
    pub edits: EditPlan,                          // queued by assignments
    pub probes_enabled: bool,                     // --enable-probes
    pub ca_bundle: Option<String>,                // TLS trust override
    pub ucs_cert_reader: Option<UcsCertReader>,   // host hooks; None = unwired
    pub files_reader: Option<FilesReader>,
    pub merge_graph: RefCell<Option<Rc<ObjectGraph>>>,  // memoised in merge mode
}
```

The two reader hooks are how the engine stays embeddable: the CLI wires
them, and the in-browser console leaves them `None`, which makes the
forensic `files` / `glob` / `grep` and `ucs_cert` builtins report that
they are unavailable rather than reaching for a filesystem that isn't
there.

Core entry points:

- `evaluate(program, ctx) -> Result<Vec<Value>, QueryError>` — loops
  `program.statements`; the last statement's values are the return, and
  intermediate statements still emit edits.
- `evaluate_statement(stmt, ctx) -> Result<Vec<Value>, QueryError>` — runs
  one statement and flattens streams.

Dispatch core:

- The recursive walker matches on the `Expr` variant. This is the hottest
  code in the engine.
- The pipe arm implements the operator's stream semantics: streams (from
  `[]` or a stream-emitting builtin) flatten element-by-element, while
  lists pass through whole.
- The path arm walks `PathExpr.steps` and returns the value *plus* a
  location trail, so the assignment machinery knows which byte span to
  splice.
- `resolve_pathref(ref, ctx)` derefs a `PathRef` against the active root's
  `BigipConfig`, using `expected_kind` when set to keep the lookup
  proportional.
- `eval_call` is builtin dispatch, and implements two jq-style shorthands:

  - *Implicit dot*: a single-argument builtin used as a bare pipeline
    stage (`length`, `sort`) receives the current input as its argument.
  - *Implicit receiver*: a multi-argument builtin called with
    `min_args - 1` arguments has the current input prepended, so
    `.x | contains("foo")` desugars to `contains(.x, "foo")`.

  A builtin declared `special_form` receives the AST nodes rather than
  evaluated values, so it can iterate them itself (`select`, `map`, `if`,
  `as`); `with_ctx` passes the `EvalContext` alongside.

`eval_call` is the hand-off between the AST walker and the builtin
catalogue. A new operator that needs short-circuit semantics almost always
wants to be a special-form builtin rather than an AST-level construct.

### `value.rs` — DSL value types

`Value` is the runtime model — `Null`, `Bool`, `Int`, `Float`, `Str`,
`List`, `Object` (an insertion-ordered `IndexMap`), `Stream`, `PathRef`,
`ObjectRef`, `Container`, and the `Drop` sentinel `select` uses to mean
"discard this value".

`List` and `Stream` are deliberately distinct variants: `|` iterates a
stream element-wise but passes a list through whole (see
[Streams vs lists](#streams-vs-lists)).

The BIG-IP-specific types:

- `FieldSlot { source_uri, start, end, raw_text }` — the byte range of one
  field's value in the source. This is what drives field-level edits.
- `ObjectRef { kind, full_path, fields, field_slots, stanza_slot,
  config_uri }` — a projected BIG-IP object. `kind` is the TMSH label
  (`"ltm virtual"`), `full_path` is `/Common/name`, `fields` and
  `field_slots` are parallel `IndexMap`s, `stanza_slot` spans the whole
  `kind /partition/name { … }` block (the SCF renderer emits it verbatim),
  and `config_uri` points back at the source.
- `PathRef { full_path, expected_kind }` — a full-path reference to
  another object (`pool = /Common/svc_pool`). It behaves as a string
  wherever a scalar is expected, and the evaluator follows `.field` access
  through to the target when one resolves. An empty `expected_kind` means
  "any matching kind".
- `Root` — the per-file evaluation root, holding `uri`, `source`, the
  parsed `config: BigipConfig`, an optional `json_value` (when the source
  came in as external JSON via `--name=alias=path.json`), the
  `object_cache` keyed by `(kind, full_path)` for identity stability, and
  a lazily built `object_graph` memoised for the graph builtins.
- `Container` — a navigable namespace or kind node projected from a
  `BigipConfig`; see [`projection.rs`](#projectionrs--bigipconfig--navigable-tree).

```
                       Root
                        │ owns
                ┌───────┼─────────────┐
                │       │             │
                ▼       ▼             ▼
          BigipConfig   source      object_cache
          (parsed       (original   {(kind, full_path):
           model)        text)       ObjectRef}
                │
                │ wrapped lazily by
                ▼
            ObjectRef
            ├── kind, full_path
            ├── fields         ── plain values
            ├── field_slots    ── FieldSlot (byte range → source text)
            ├── stanza_slot    ── FieldSlot for the whole stanza
            ├── config_uri     ── back-pointer to Root
            └── (one field)
                  │
                  ▼
                PathRef ── auto-derefs to another ObjectRef on next step
                  │            via eval::resolve_pathref
                  └── carries expected_kind hint to bound the lookup

            Stream  (lazy sequence; flatten under `|`, collect with `[ … ]`)
```

`FieldSlot` is what makes byte-accurate mutation possible: when
an assignment lands on `.x = "new"`, the evaluator looks up
`obj.field_slots["x"]`, the edit planner records the byte range,
and the apply step splices new bytes into the original source.
No regex; no source-text scan.

### `projection.rs` — BigipConfig → navigable tree

The projection layer is *lazy*.  Containers don't materialise
their entries until first access; ObjectRefs are built only for
objects the query actually touches.

```
                Root                                    BigipConfig
                 │                                           ▲
                 │ root_container()                          │
                 ▼                                           │
        ┌────────────────────┐                               │
        │  <root> Container  │   .virtuals / .pools / …      │
        │   kind = "<root>"  │  ◄────────────────────────────┤
        └─────────┬──────────┘                               │
                  │ .ltm                                     │
                  ▼                                          │
        ┌────────────────────┐                               │
        │  ltm Container     │                               │
        │   kind = "ltm"     │                               │
        └─────────┬──────────┘                               │
                  │ .virtual                                 │
                  ▼                                          │
        ┌─────────────────────┐                              │
        │ ltm virtual         │  _MODULE_KINDS["ltm"]        │
        │ Container           │    ["virtual"] = ("virtuals",│
        │ kind = "ltm virtual"│     "ltm virtual")           │
        └─────────┬───────────┘                              │
                  │ ["/Common/web_vs"]                       │
                  ▼                                          │
        ┌─────────────────────┐  _build_object_ref()         │
        │  ObjectRef          │ ─── caches in ──────► Root._object_cache
        │   kind, full_path   │                       [(kind, full_path)]
        │   fields            │                              │
        │   field_slots ◄─────┼── _project_field via         │
        │   stanza_slot       │   _KIND_FIELD_MAPS["ltm      │
        └─────────────────────┘   virtual"] = (BigipVirtual, │
                                  _VS_FIELDS)                │
```

`module_kinds` is the module → kind dispatch, `kind_to_label` and
`is_object_kind_alias` the kind-label vocabulary, and `project_fields` the
per-kind field dispatch. All three are `match` arms in `projection.rs`
rather than lookup tables, so adding a kind is a compile-checked edit.

#### `Container`

A `Container` is `{ kind, root, entries }`, where `kind` is either a module
name (`"ltm"`), the full kind label (`"ltm virtual"`), or the synthetic
`"<root>"`, and `entries` is a `RefCell<Option<IndexMap<String, Value>>>` —
`None` until first navigation.

- `root_container(root) -> Value` — the synthetic `<root>` Container for a
  BIG-IP source, or the raw `json_value` for an external JSON source.
- `build_entries(container) -> IndexMap<String, Value>` — materialise
  children. Three flavours keyed by `container.kind`:
  1. `<root>` → the module map (`ltm`, `net`, `sys`, …).
  2. A module name (`"ltm"`) → the kind map (`virtual`, `pool`, `rule`, …).
  3. A kind label (`"ltm virtual"`) → the object map
     (`/Common/web_vs`, …).
- Key lookup applies partition shorthand (`pool["svc"]` resolves to
  `/Common/svc` when unambiguous) and regex subscripts (`[~"web_.*"]`)
  compile through the safe-regex layer.

#### Object construction

- `build_object_ref(kind, full_path, obj, root) -> Value` — wraps one
  parsed model object. Caches in `root.object_cache` by
  `(kind, full_path)`, and populates `field_slots` (via
  `collect_field_slots`) and `stanza_slot` (via `stanza_slot_for`) from the
  parser's source ranges.
- `project_fields(kind, obj, root) -> IndexMap<String, Value>` — projects
  every field of one object. Reference-valued fields become `PathRef`s
  through `path_ref` / `path_ref_list`; typed values (`Destination`,
  `MonitorExpression`, …) render through `typed_str`; list-valued
  properties go through `list_str_values`. Pool members, `ltm policy`
  rules, and iRule refs have their own arms.

The `(kind, full_path)` compound cache key matters because BIG-IP lets
different kinds share a path string — `/Common/shared` could
simultaneously be a pool, a node, and an iRule, and a single-key cache
would let one kind's `ObjectRef` leak into another's lookup.

The projection covers the core LTM kinds the cookbook and common queries
reach for: `ltm virtual`, `ltm virtual-address`, `ltm pool` (plus
members), `ltm node`, `ltm monitor`, `ltm rule`, `ltm data-group`,
`ltm persistence`, `ltm snatpool`, `ltm profile`, and `ltm policy` (plus
rules, conditions, and actions). The long tail of kinds projects the
minimal field set — `name`, `full-path`, `kind`, `description`.

### `builtins/` — Builtin catalogue and registration

`builtins/mod.rs` owns the registry and the shared argument-coercion and
value-walking helpers; the builtins themselves live in the category module
they belong to.

- `BuiltinSpec` — `name`, `category`, and five runtime knobs:
  - `min_args` / `max_args` — strict arity check.
  - `special_form: bool` — when true, the evaluator passes AST nodes
    rather than evaluated values. Used by `select`, `map`, `if`, `as`,
    `with_entries`, and the `paths` / `getpath` family, all of which live
    in `special.rs`.
  - `with_ctx: bool` — when true, the builtin receives the `EvalContext`.
    Used by anything needing the root, the edit plan, the merge state, or
    a reader hook.
  - `stream_aware: bool` — when true, the builtin handles its own stream
    semantics; otherwise the dispatch unwraps streams element by element.
  - `broadcasts: bool` — whether stream arguments broadcast element-wise.
    This is the scalar-builtin default. A `with_ctx` builtin normally
    skips broadcast; `refs` and `referenced_by` are the exception — they
    broadcast *and* need `ctx` for the config.
- `REGISTRY` — a `OnceLock<HashMap<&str, BuiltinSpec>>`, built once on
  first lookup.
- `lookup(name) -> Option<&'static BuiltinSpec>` — the evaluator's
  dispatch hook.
- `all_specs() -> Vec<&'static BuiltinSpec>` — every builtin, sorted by
  `(category, name)`, for `--help-builtins`.

`eval::eval_call` dispatches on the spec: a special form gets the raw AST
nodes; a stream-aware builtin gets the evaluated arguments and handles
streams itself; anything else has its stream arguments broadcast, invoking
the implementation once per element.

Plain builtins raise `QueryError::Builtin` for argument-type mistakes, so
the CLI maps every one of them to `error:` uniformly.

### `probes.rs` and `probes/` — Network builtins (gated)

Two surfaces with different output guarantees:

- **Byte-for-byte, golden-tested.** `x509_parse` turns a PEM certificate
  into the x509 parse dict (subject, issuer, serial, validity, SANs,
  fingerprint, key algorithm and size, signature algorithm, version, and
  the public-key PEM), plus the deterministic projections `x509_eq` and
  `x509_from_config`. The `--enable-probes` gating error is also
  byte-for-byte. **These pure helpers are not gated.**
- **Faithful but not golden.** The live probes — `dns`, `rev_dns`, `ping`,
  `portping`, `traceroute`, `socket_get`, `tls_handshake`, and the `url_*`
  HTTP family. They do real I/O in the reference output shape, but nothing
  asserts them byte-for-byte against live results, because the test
  environment has no reliable network.

#### Gating

There is no ambient global. `EvalContext` carries the two knobs
explicitly:

- `probes_enabled: bool` — the `--enable-probes` gate. Every live probe
  opens with `require_probes(name, ctx.probes_enabled)?`, which raises the
  one fixed gating message before touching the network.
- `ca_bundle: Option<String>` — an optional CA path overriding the system
  trust store for `url_*` and `tls_handshake`. `None` falls back to the
  system roots.

Because both live on the context rather than in process-global state,
concurrent query runs cannot interfere with each other, and a caller
embedding the engine (the WASM console, the report generator) gets the
gate closed by default.

#### TLS

`probes/tls.rs` drives `rustls` directly. `client_config(ca_bundle)`
builds the verifier; `build_result(conn, verify_error)` renders the
handshake outcome, and `classify_verify_kind(msg)` maps a verifier failure
into the structured `reason.kind` vocabulary the DSL exposes. A
verification failure is reported through `reason` rather than aborting, so
an audit query collects data from a device with a bad chain and still says
so. Only `reason.fatal` means the data is genuinely unavailable.

#### HTTP

`probes/http.rs` performs the `url_*` requests and captures the peer
certificate off the same connection, so reading the chain never costs a
second round-trip.

### `inputs.rs` — External input parsers

`parse_input(source, uri, spec)` is the dispatcher; each format has its
own parser:

- `parse_json(text, uri)` — a single JSON document.
- `parse_jsonl(text, source)` — NDJSON. Blank lines are skipped, and a
  per-line error carries its line number.
- `parse_csv(text, …)` — CSV. The header is auto-detected from row 1
  unless overridden; extra columns land in an `extra` list, and missing
  columns become the empty string.
- `parse_f5log(text)` — F5 syslog, one object per event with `timestamp`,
  `host`, `severity`, `daemon`, `pid`, `code`, `module`, `level`,
  `message`, and `raw`, against the eight-value syslog severity
  vocabulary.
- `parse_zone(source, uri)` — a DNS zone file.

These back the `json_load` / `jsonl_load` / `csv_load` / `f5log_load`
builtins in `builtins/inputs_load.rs`, and the
`--input-{json,jsonl,csv,f5log}` CLI flags.

### `edit_plan.rs` — Mutation queue

- `EditOp` — one byte-level rewrite: `source_uri`, `object_path`,
  `object_kind`, `field_name`, `operator` (`=`, `|=`, `+=`, `-=`),
  `old_value`, `new_value`, `field_slot`, `stanza_slot`, and a `strict`
  flag.
- `PrefixRewrite` — a whole-source regex substitution, used by
  `rename_partition` and `rename_prefix`.
- `EditPlan` — the queue: `ops` plus `prefix_rewrites`, with `add`,
  `add_prefix`, and `has_edits`.
- `AppliedSource` — the per-source result: `uri`, `original`,
  `new_source`, the rename reports, and the field-edit count.
- `apply(plan, sources) -> HashMap<String, AppliedSource>` — the
  workhorse. Three phases per source:
  1. Apply the `PrefixRewrite`s over the whole source.
  2. Apply identity-field renames (`name`, `full-path`) through
     `rewrite.rs`, which touches every reference site, not just the
     stanza header.
  3. Apply the field-level edits sorted by `(start_offset, end_offset)`
     descending, so a later splice never shifts a span an earlier one
     still needs.

  After splicing it re-parses through `parse_bigip_conf` to validate.
  Malformed output aborts with the offending edit named in the error.

The `name` / `full-path` pair is what dispatches between the
identity-rename branch and the field-edit branch.

Mixing a `PrefixRewrite` with non-identity field edits in the
*same statement* is rejected — the rewrite shifts byte offsets
of later edits.  Split the work across statements with `;`.

```
              EditPlan                                source text
              ┌─────────────────┐                 ┌─────────────────┐
              │ prefix_rewrites ├── 1 ─────────►  │  bigip.conf     │
              │  PrefixRewrite  │                 │   (bytes)       │
              │  PrefixRewrite  │                 └────────┬────────┘
              ├─────────────────┤                          │
              │     ops         ├── 2 ──► rename_object()  │
              │  EditOp (name)  │                          │
              │  EditOp (name)  │                          ▼
              ├─────────────────┤                 ┌─────────────────┐
              │     ops         ├── 3 ─────────►  │  rewritten      │
              │  EditOp (field) │  splice in      │  bytes          │
              │  EditOp (field) │  reverse offset └────────┬────────┘
              │  EditOp (field) │  order                   │
              └─────────────────┘                          ▼
                                                  parse_bigip_conf()
                                                           │
                                                           ▼
                                                  ┌─────────────────┐
                                                  │ AppliedSource   │
                                                  │  new_source     │
                                                  │  rename_reports │
                                                  │  field_edits=N  │
                                                  └─────────────────┘
```

Sort key for step 3 is `(start_offset, end_offset)` *descending*
so each splice writes into bytes downstream of every span the
later splices still need to find.

### `output.rs` — Renderers

`render(values, mode) -> Result<String, QueryError>` dispatches, with
`render_with_opts` adding renderer options:

- `render_auto` — the default. If every value is an `ObjectRef` with a
  `stanza_slot`, emit SCF; if every value is a scalar or a list of
  scalars, emit `raw`; otherwise emit JSON.
- `render_scf` — emit each object's `stanza_slot.raw_text` verbatim, which
  preserves comments and whitespace exactly.
- `render_raw` — one scalar per line. Rejects `ObjectRef` and object
  values so the output stays unambiguous.
- `render_paths` — the `full_path` of each `ObjectRef` / `PathRef`.
- `render_json` — a JSON array through `jsonfmt.rs`; an `ObjectRef`
  flattens to its field map.
- `render_table` — an aligned table, optionally with box-drawing line art.
- The TMSH mode emits `tmsh modify ltm virtual …` command lines, used for
  the mutation-as-tmsh output.

The `--output` CLI flag picks the mode; `auto` is the default because it
produces the most natural form for whatever the query returned. A mode
that is better modelled as a diagram or timeline lives in `renderers/`
instead and is selected with `-R / --render` — see
[`f5-query-renderer-contract.md`](f5-query-renderer-contract.md).

### `runner.rs` — Orchestration

`run_query(query, sources, opts) -> Result<QueryResult, QueryError>`.

`QueryOptions` bundles everything the run needs: `names` (`$name -> uri`
bindings, auto-derived from each URI's filename stem when empty),
`partitions` (per-URI BIG-IP partition, defaulting to `Common`),
`side_inputs` (each binding `$name` to a JSON-backed `Root` parsed per its
`InputSpec` — it counts toward the multi-file source count but never
iterates as the primary `.` input), `merge`, `enable_probes`,
`ca_bundle`, and the two reader hooks.

Per-source vs merged flow:

- `run_query_per_file` — the default. Runs once per source, each with its
  own `EvalContext`.
- `run_query_merged` — `--merge` mode. Every source lands in
  `named_roots` and `merge_roots`, and the evaluator sees them as one
  stream.

Multi-statement loop, per source:

```
for stmt in program.statements {
    let values = evaluate_statement(stmt, &mut ctx)?;
    let applied = edit_plan::apply(&ctx.edits, &sources)?;
    // replace each source with applied[uri].new_source,
    // reset ctx.edits, and rebuild ctx.root from the rewritten text
}
```

That's what makes `rename_partition("Common", "Tenant_A") ;
.ltm.virtual[] | .destination |= …` correct — the second
statement parses the renamed source.

```
    program.statements = [stmt₁, stmt₂, stmt₃]

       sources₀                           ┌──────────────┐
          │                               │   stmt₁      │
          │                ┌────────────► │  evaluate    │
          │                │              │   ↓ emits    │
          ▼                │              │  EditPlan₁   │
    ┌────────────┐         │              └──────┬───────┘
    │ build ctx, ├─────────┘                     │ apply
    │ ctx.root   │                               ▼
    └────────────┘                        ┌──────────────┐
          ▲                               │  sources₁    │
          │                               │  (rewritten) │
          │ rebuild root                  └──────┬───────┘
          │                                      ▼
          │                ┌────────────► ┌──────────────┐
          │                │              │   stmt₂      │
          ├────────────────┤              │  evaluate    │
          ▲                │              │   ↓ emits    │
          │                │              │  EditPlan₂   │
          │ rebuild root   │              └──────┬───────┘
          │                │                     │ apply
          │                │                     ▼
          │                │              ┌──────────────┐
          │                │              │  sources₂    │
          │                │              └──────┬───────┘
          │                │                     ▼
          │                ├────────────► ┌──────────────┐
          └────────────────┤              │   stmt₃      │
                           │              │ ← values     │
                           │              │   returned   │
                           │              └──────────────┘
                           │
                          (loop)
```

Each statement runs against the rewritten source of the previous
one — that's what lets the runner serialise side-effects across
`;` without exposing partial state.

There is no ambient per-query state. Everything a builtin might reach for
— the active roots for `refs` / `referenced_by`, the merge flag, the
partition remap, the side-input specs, the probe gate — is a field on
`EvalContext`, threaded explicitly from `QueryOptions`. That is what makes
the engine safe to embed and to run concurrently.

`QueryResult` is the final shape: `values_per_file` (`Vec<(uri, values)>`
in source order, so the verb renders files in input order),
`edits_per_file` (`Vec<(uri, AppliedSource)>`, empty for a read-only
query), and `has_mutation` — true when the query *queued* any edit op.

### Error positioning

Every AST node carries a byte `offset`, and `QueryError`'s parse and
evaluation variants carry it through to the CLI, which resolves it to a
line and column to underline the offending position.

### `builtins/graph.rs` — Reference traversal

Backs the `refs(obj)` / `referenced_by(obj)` builtins.

- Forward traversal runs the registry-driven reference walk from the
  object and returns the target full-paths.
- Reverse traversal does the same in the other direction.
- Scope is `obj.config_uri` alone by default; in merge mode it spans every
  root in `ctx.merge_roots`, using the `ctx.merge_graph` memoised on first
  use so repeated per-object queries do not rebuild the whole graph.

The actual reference index lives in
[`docs/design/bigip-registry-architecture.md`](bigip-registry-architecture.md)
— `compute_grep` consults the registry's reference table.

## Cross-cutting invariants

### Streams vs lists

`Value::Stream` is the flattening form and `Value::List` the materialised
one. `|` iterates a stream element-wise but passes a list through whole.  Aggregator builtins (`sort`, `unique`, `count` over a
collection) require a list; force collection with `[ … ]`.

The line is crossed by the bare `.foo[]` subscript, which produces a
`Stream`, and by `[ expr ]` (`ListLiteral`), which collects a stream back
into a list.

### PathRef auto-deref

`.pool` returns a `PathRef`; `.pool.members` then auto-derefs
the PathRef to the actual pool `ObjectRef` and indexes its
`members` field.  Resolution is lazy via
`eval::resolve_pathref` and uses `PathRef.expected_kind`
when set to keep lookup proportional.

### Object cache identity

`Root.object_cache` is keyed by `(kind, full_path)`. The compound key
prevents one kind's `ObjectRef` leaking into another kind's lookup —
BIG-IP lets different kinds share a path string, so `/Common/shared` can
be a pool, a node, and an iRule at once.

### Source-range fidelity

Every `ObjectRef.field_slots[name]` is a `FieldSlot` — a half-
open byte span in the originating source for that property.
The edit planner uses these to splice edits at exact bytes; no
regex search-and-replace step.  Slots come from the registry's
value-spec layer; see
[`bigip-registry-architecture.md`](bigip-registry-architecture.md).

### Probe gating

`EvalContext.probes_enabled` is the single gate. The CLI's
`--enable-probes` flag sets it for one query run; nothing is ambient, so
concurrent runs never share state. Every network builtin calls
`require_probes(name, ctx.probes_enabled)` before doing anything else.

### Permissive vs strict TLS

A TLS verification failure is reported, not fatal. `tls_handshake` and the
`url_*` family record the verifier's complaint in the structured `reason`
field and carry on, because the audit-friendly default is to collect the
data *and* say the chain is bad. `reason.fatal` is the only case where
data is genuinely unavailable. The peer certificate is captured off the
same connection either way.

## Edit-plan apply order

`edit_plan::apply(plan, sources)` walks `plan.ops` and:

1. Splits ops into per-source bins, one bin per loaded config file.
2. Applies `PrefixRewrite`s first — a whole-source regex substitution.
3. Applies identity-field renames through the token-bounded rewriter in
   `rewrite.rs`, which touches every reference site, not just the stanza
   header.
4. Sorts the remaining field-level edits by `(start_offset, end_offset)`
   *descending*, so a later splice never shifts a byte span an earlier one
   still needs.
5. Splices each op into the source bytes.
6. Re-parses the spliced source through `parse_bigip_conf` — malformed
   output aborts with the offending edit named in the error.

The reparse in step 6 is what makes mutating queries safe: the engine never
ships output that a fresh `parse_bigip_conf` could not read back in.

Mixing a `PrefixRewrite` with non-identity field edits in the *same*
statement is rejected, because the rewrite shifts the byte offsets the
later edits target. Split the work across statements with `;`.

## Extension points

### A new builtin

1. Add a `BuiltinSpec` to the registry in `builtins/mod.rs`, with the
   implementation in the category module it belongs to (`string.rs`,
   `net.rs`, `graph.rs`, …). The spec's runtime knobs are `min_args` /
   `max_args` (strict arity), `special_form` (receive AST nodes rather
   than evaluated values), `with_ctx` (receive the `EvalContext`),
   `stream_aware` (handle stream semantics yourself), and `broadcasts`
   (whether stream arguments broadcast element-wise).
2. A special form goes in `special.rs` instead — that is where `select`,
   `map`, `if`, and `as` live.
3. Add at least one example so `--help-builtins` documents it.
4. Add a regression test in the crate's own tests.

For a network builtin, also make `require_probes(name,
ctx.probes_enabled)?` the first call in the implementation, and route
anything TLS-shaped through `tls_handshake` or the `url_*` family so the
`reason` classifier and the certificate capture stay consistent.

### A new projection field on an existing kind

1. Add the field to the model type in `rust/tcl-bigip/src/model/`.
2. Populate it in `rust/tcl-bigip/src/parser/`.
3. Add the projection arm in `project_fields` in `projection.rs`.
4. Add a regression test that projects the new field.

### A new typed value

See [`bigip-registry-architecture.md`](bigip-registry-architecture.md).

### A new output mode

Add a `render_<mode>` function in `output.rs`, register it in `render`'s
dispatch, and extend the CLI's `--output` choices. A renderer that is
better modelled as a pluggable diagram or timeline belongs in
`renderers/` instead — see
[`f5-query-renderer-contract.md`](f5-query-renderer-contract.md).

## Related design docs

- [`bigip-registry-architecture.md`](bigip-registry-architecture.md)
  — registry contract for value specs, source ranges,
  references.
- [`f5-cli-architecture.md`](f5-cli-architecture.md) — verb
  registry, command dispatch, output format plumbing.
