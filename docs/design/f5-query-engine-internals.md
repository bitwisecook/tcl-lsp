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
core/bigip/query/
├── __init__.py           # Public re-exports: run_query, format_*, list_builtins
├── lexer.py              # Tokeniser — single pass, no lookahead beyond one char
├── parser.py             # Recursive-descent parser → ast.* nodes
├── ast.py                # Frozen dataclasses for every expression form
├── evaluator.py          # Walks the AST against a Root / BigipConfig
├── projection/
│   ├── _engine.py        # _project_field — the dispatch core, ObjectRef caching
│   ├── _classes.py       # Container, FieldSpec, ObjectRef construction helpers
│   └── _data.py          # Per-kind field maps (manually curated)
├── builtins.py           # @_register decorator + 116 builtin catalogue
├── _probes.py            # Network builtins (gated by --enable-probes)
├── _inputs.py            # External input parsers (JSON / JSONL / CSV / f5log)
├── values.py             # ObjectRef, PathRef, Stream, Root, FieldSlot
├── edit_plan.py          # EditOp, EditPlan, PrefixRewrite, apply()
├── output.py             # Renderers: auto / scf / raw / paths / json / tmsh
├── runner.py             # run_query orchestration, multi-statement loop
├── source_map.py         # offset → (line, col) for error reporting
├── graph.py              # refs / referenced_by builtin plumbing
├── grammar.py            # `--help-dsl` plain-text grammar reference
├── examples.py           # `--help-examples` cookbook
└── errors.py             # ParseError, EvalError, BuiltinError, QueryError
```

## Pipeline

A query goes through five distinct stages, each owned by one
module:

1. **Lex** — `lexer.py::tokenise(source)` → `list[Token]`.
2. **Parse** — `parser.py::parse(source)` → `ast.Program`.
3. **Project** — `projection._engine` lazily wraps a
   `BigipConfig` in navigable `Container` / `ObjectRef` nodes
   the evaluator can index into.
4. **Evaluate** — `evaluator.py::evaluate(program, ctx)` →
   `list[Any]`.  Assignment nodes emit `EditOp` into
   `ctx.edits.ops` rather than mutating in place.
5. **Apply + render** — `runner.py::run_query` applies the edit
   plan to each source's text via `edit_plan.apply`, re-parses
   the rewritten text, and hands the values + sources to
   `output.py::render`.

Multi-statement queries (`a ; b ; c`) loop over statements; each
statement sees the edits applied by the previous one.

### Data flow

```
                source text                      query text
                     │                                │
                     │                                ▼
                     │                       ┌────────────────┐
                     │                       │   lexer.py     │
                     │                       │  tokenise()    │
                     │                       └───────┬────────┘
                     │                               │ list[Token]
                     │                               ▼
                     │                       ┌────────────────┐
                     │                       │   parser.py    │
                     │                       │   parse()      │
                     │                       └───────┬────────┘
                     │                               │ ast.Program
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
            │ projection/    │  ◄──────── lazy access from evaluator
            │ _engine.py     │
            │ Container /    │
            │ ObjectRef tree │
            └───────┬────────┘                       │
                    │                                ▼
                    │            ┌──────────────────────────────┐
                    └───────────►│        evaluator.py          │
                                 │   evaluate(program, ctx)     │
                                 │                              │
                                 │  reads  ─► ObjectRef / Stream│
                                 │  emits  ─► EditOp into       │
                                 │            ctx.edits.ops     │
                                 └──────────────┬───────────────┘
                                                │ list[Any]
                                                │  + EditPlan
                                                ▼
                                        ┌───────────────┐
                                        │ edit_plan.py  │
                                        │   apply()     │
                                        └───────┬───────┘
                                                │ AppliedSource
                                                │  (new_source)
                                                ▼
                                        ┌───────────────┐
                                        │   output.py   │
                                        │   render()    │
                                        └───────┬───────┘
                                                │ rendered text
                                                ▼
                                          stdout / files
```

The reparse step inside `edit_plan.apply` (not shown — it round-
trips `new_source` through `parse_bigip_conf` to validate) is
what makes mutation safe: the engine never ships output a fresh
parse couldn't read back in.

## Module-by-module reference

### `lexer.py` — Tokeniser

Hand-rolled scanner.  Single pass, one-character lookahead.  Key
symbols:

- `TokenKind` (enum) — ~60 token classes.  Operators (`PIPE`,
  `PIPE_EQ`, `PLUS_EQ`, `EQ`, `EQ_EQ`, `BANG_EQ`, `LT`, `GT`,
  `LT_EQ`, `GT_EQ`), path syntax (`DOT`, `LBRACKET`, `RBRACKET`,
  `QUESTION`), keywords (`AND`, `OR`, `NOT`, `IF`, `THEN`,
  `ELIF`, `ELSE`, `END`, `AS`, `TRUE`, `FALSE`, `NULL`),
  literals (`NUMBER`, `STRING`, `REGEX`, `IDENT`, `VAR`).
- `Token` (frozen dataclass) — carries `kind`, `text`, byte
  `offset`, optional `value` for already-parsed literals.
- `tokenise(source: str) → list[Token]` — entry point.  Comment
  syntax is `#` to end-of-line.  Barewords (identifiers) may
  contain `-` so TMSH names like `source-address-translation`
  lex as one ident.
- `_KEYWORDS` (module dict) — keyword → `TokenKind` mapping.

If you need to add a new operator, this is the first edit — add
the `TokenKind`, add the two-char lookahead in `tokenise`, then
wire it into the parser's precedence cascade.

### `parser.py` — Recursive-descent parser

Hand-written, no parser generator.  Grammar in
[`docs/references/f5_query/dsl.md`](../references/f5_query/dsl.md).

- `_Parser` (internal class) — stateful, holds the token stream
  and a position index.  Most methods are private precedence
  layers.
- `parse(source: str) → ast.Program` — public entry; calls
  `tokenise` then `_Parser(tokens).parse_program()`.
- `parse_program() → Program` — handles multi-statement `;`
  separation.
- Precedence cascade (top→bottom): pipe → comma-stream →
  assignment → ternary `if` → `or` → `and` → `not` →
  comparison → addition → multiplication → unary → primary.
  Each layer descends to the next.
- `_parse_pipe_stage` — emits assignment operators (`=`, `|=`,
  `+=`, `-=`) as trailing ops on the LHS, not infix nodes.
- `_parse_primary` — literals, variable refs (`$name`), object
  literals (`{k: v}`), list literals (`[…]`), `if/then/else`
  blocks, parenthesised expressions, and the path-expression
  prefix (`.` or `..`).
- `_ASSIGN_OPS` / `_CMP_OPS` (module-level dicts) — operator
  token → AST node-tag lookup.

To add a new infix operator: pick the precedence level, add to
the `_*_OPS` map (if comparison-flavoured) or write the layer
yourself, mint a `BinOp.op` token, then handle it in
`evaluator.py::_eval_binop`.

### `ast.py` — AST node types

Every node is a `@dataclass(frozen=True, slots=True)`.  All
carry `offset: int` (byte offset into the source for error
underlining).  Key node types:

| Node | Carries | Form |
|---|---|---|
| `Literal` | `value: object` | `42`, `"foo"`, `true`, `null` |
| `Identity` | — | bare `.` |
| `Variable` | `name: str` | `$x` |
| `Field` | `name: str`, `optional: bool` | `.foo` / `.foo?` |
| `Subscript` | `index, stream, regex, optional` | `[i]` / `[]` / `[~"re"]` |
| `PathExpr` | `head: Expr | None`, `steps: tuple` | `.a.b[0]` / `$x.a.b` |
| `ObjectLiteral` | `entries: tuple[ObjectEntry]` | `{k: v, k2}` |
| `ListLiteral` | `body: Expr` | `[ ... ]` |
| `Call` | `name: str`, `args: tuple` | `select(.x > 1)` |
| `BinOp` | `op: str`, `lhs, rhs` | `a + b` |
| `UnaryOp` | `op: str`, `value` | `-x`, `not x` |
| `Pipe` | `lhs, rhs` | `a | b` |
| `Assignment` | `op: str`, `target: PathExpr`, `value` | `.x = 1` / `.x |= f` |
| `LetBinding` | `value, name, body` | `1 as $x | …` |
| `IfThenElse` | `condition, then_body, elif_branches, else_body` | `if/then/elif/else/end` |
| `CommaStream` | `parts: tuple[Expr]` | `a, b, c` |
| `Program` | `statements: tuple[Expr]` | `a ; b ; c` |

Assignment targets must always be `PathExpr` (or `Field` /
`Subscript` chains rooted in one); the evaluator rejects writes
to literals or calls at runtime.

### `evaluator.py` — AST walker

`EvalContext` (dataclass):

```
EvalContext:
    root: Root
    edits: EditPlan
    named_roots: dict[str, Root]      # --name=alias=path
    bindings: list[dict[str, Any]]    # let-binding stack
    merge_mode: bool                  # --merge active
```

Core entry points:

- `evaluate(program, ctx) → list[Any]` — loops
  `program.statements`; the last statement's values are the
  return; intermediate statements still emit edits.
- `evaluate_statement(stmt, ctx) → list[Any]` — runs one
  statement; flattens streams.

Dispatch core:

- `_eval(node, current, ctx)` — recursive walker.  Dispatches
  on `type(node)` via `isinstance` chain; this is the hottest
  function in the engine.
- `_pipe_through(values, rhs, ctx)` — the pipe operator's
  stream semantics.  Streams (from `[]` or stream-emitting
  builtins) are flattened element-by-element; lists pass
  through whole.
- `_eval_path(node, current, ctx)` — walks `PathExpr.steps`;
  returns (value, location-trail) so the assignment machinery
  can find the byte span to splice.
- `_step` / `_field_step` / `_subscript_step` — one path step;
  field-access dispatches through
  `QueryFieldProvider.query_fields` for typed value objects,
  plain `getattr` / dict lookup for scalars.
- `_resolve_pathref(ref, ctx)` — derefs a `PathRef` against the
  active root's `BigipConfig`.  Uses `expected_kind` when set
  to keep lookup proportional.
- `_eval_call(node, current, ctx)` — builtin dispatch.
  Implements two jq-style shorthands:

  - *Implicit dot*: a single-arg builtin called as a pipeline
    stage without parens (`length`, `sort`) gets the current
    input as its single arg.
  - *Implicit receiver*: a multi-arg builtin called with
    `min_args - 1` args has the current input prepended.
    `.x | contains("foo")` desugars to `contains(.x, "foo")`.

  Special-form builtins (those declared with `special_form=True`)
  receive the AST nodes, not pre-evaluated values, so they can
  iterate them themselves (`select`, `map`, `if`, `as`).
  `with_ctx=True` adds the `EvalContext` as a kwarg.

`_eval_call` is the hand-off between the AST walker and the
builtin catalogue; if a new operator needs short-circuit
semantics it almost always wants to be a special-form builtin
rather than an AST-level construct.

### `values.py` — DSL value types

Five types:

- `FieldSlot` (frozen) — byte range for one field's value in
  source text.  `(start, end, raw_text, source_uri)`.  Drives
  field-level edits.
- `ObjectRef` (mutable dataclass) — a wrapped BIG-IP object.
  Fields:
  - `kind: str` — TMSH kind label (`"ltm virtual"`).
  - `full_path: str` — `/Common/name`.
  - `fields: dict[str, Any]` — projected scalar fields.
  - `field_slots: dict[str, FieldSlot]` — byte spans, parallel
    to `fields`.
  - `stanza_slot: FieldSlot | None` — span of the whole `kind
    /partition/name { … }` block.  Used by the SCF renderer.
  - `config_uri: str` — back-pointer to the source URI.
- `PathRef` (frozen) — a typed full-path reference to another
  object (`pool = /Common/svc_pool`).  Carries `root` (the
  originating `Root`) and `expected_kind` (hint to bound
  resolution lookup) and auto-derefs under further field
  access.
- `Stream` (mutable) — a lazy sequence.  Distinguished from a
  Python `list` so `|` can iterate streams element-wise but
  pass lists whole.
- `Root` (mutable) — per-file evaluation root.  Holds the
  `BigipConfig`, the original source text, a `SourceMap`, the
  `json_value` (when the source was an external JSON file via
  `--name=alias=path.json`), the URI, and `_object_cache:
  dict[(kind, full_path), ObjectRef]` for identity stability.

The protocol `QueryFieldProvider` is what the typed value
objects (`BigipPool`, `BigipVirtual`, …) implement so the
evaluator can call `.query_fields() → Mapping[str, …]`
generically rather than knowing about every dataclass.

```
                       Root
                        │ owns
                ┌───────┼─────────────┐
                │       │             │
                ▼       ▼             ▼
          BigipConfig  SourceMap    _object_cache
          (parsed       (line/col   {(kind, full_path):
           model)        index)      ObjectRef}
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
                  │            via evaluator._resolve_pathref
                  └── carries expected_kind hint to bound the lookup

            Stream  (lazy sequence; flatten under `|`, collect with `[ … ]`)
```

`FieldSlot` is what makes byte-accurate mutation possible: when
an assignment lands on `.x = "new"`, the evaluator looks up
`obj.field_slots["x"]`, the edit planner records the byte range,
and the apply step splices new bytes into the original source.
No regex; no source-text scan.

### `projection/` — BigipConfig → navigable tree

The projection layer is *lazy*.  Containers don't materialise
their entries until first access; ObjectRefs are built only for
objects the query actually touches.

```
                Root                                    BigipConfig
                 │                                          ▲
                 │ root_container()                         │
                 ▼                                          │
        ┌────────────────────┐                              │
        │  <root> Container  │   .virtuals / .pools / …     │
        │   kind = "<root>"  │  ◄───────────────────────────┤
        └─────────┬──────────┘                              │
                  │ .ltm                                    │
                  ▼                                         │
        ┌────────────────────┐                              │
        │  ltm Container     │                              │
        │   kind = "ltm"     │                              │
        └─────────┬──────────┘                              │
                  │ .virtual                                │
                  ▼                                         │
        ┌─────────────────────┐                             │
        │ ltm virtual         │  _MODULE_KINDS["ltm"]       │
        │ Container           │    ["virtual"] = ("virtuals",│
        │ kind = "ltm virtual"│     "ltm virtual")           │
        └─────────┬───────────┘                             │
                  │ ["/Common/web_vs"]                      │
                  ▼                                         │
        ┌─────────────────────┐  _build_object_ref()        │
        │  ObjectRef          │ ─── caches in ──────► Root._object_cache
        │   kind, full_path   │                       [(kind, full_path)]
        │   fields            │                              │
        │   field_slots ◄─────┼── _project_field via         │
        │   stanza_slot       │   _KIND_FIELD_MAPS["ltm     │
        └─────────────────────┘   virtual"] = (BigipVirtual,│
                                  _VS_FIELDS)                │
```

`_KIND_FIELD_MAPS` is the per-kind field dispatch; `_MODULE_KINDS`
is the module → kind dispatch.  Both are flat dicts in
`projection/_data.py`.

#### `projection/_classes.py`

- `Container` (mutable) — `kind` (module name like `"ltm"` or
  the full kind label `"ltm virtual"`), `root` back-pointer,
  `_entries` (lazy dict), `_entry_source` (debug label).
  - `entries() → dict[str, Any]` — lazy populate via
    `_engine._build_entries`.
  - `lookup(key) → Any` — dict access with partition-shorthand
    (`pool["svc"]` → `/Common/svc` when
    unambiguous).
  - `regex_keys(pattern) → list[str]` — wildcard / regex
    subscript dispatch (`[~"web_.*"]`); compiles through the
    safe-regex layer (`_safe_regex_compile`).
- `FieldSpec` (frozen) — describes one projected field:
  - `attr: str` — dataclass attribute name.
  - `ref_kind: str` — non-empty means values are PathRefs to
    this kind (`"ltm pool"`, `"sys file ssl-cert"`).
  - `list_ref: bool` — values are a tuple of PathRefs.
  - `typed: bool` — wrap value via `str()` (for typed value
    objects).

#### `projection/_engine.py`

The dispatch core:

- `root_container(root) → Container | object` — returns a
  synthetic `<root>` Container for BIG-IP sources, or the raw
  `json_value` for external JSON.
- `_build_entries(container) → dict[str, Any]` — materialise
  children.  Three flavours keyed by `container.kind`:
  1. `<root>` → modules dict (`ltm`, `net`, `sys`, …).
  2. Module name (`"ltm"`) → kinds dict (`virtual`, `pool`,
     `rule`, …).
  3. Kind label (`"ltm virtual"`) → objects dict
     (`/Common/web_vs`, …).
- `_build_object_ref(kind, full_path, obj, root) → ObjectRef` —
  wraps one typed dataclass.  Caches in `root._object_cache`
  by `(kind, full_path)`; populates `field_slots` and
  `stanza_slot` from the parser's source ranges.
- `_project_field(kind, obj, spec, root, *, tmsh_name) → Any` —
  projects one field.  Branches on `FieldSpec.ref_kind` (PathRef
  construction), `list_ref` (tuple of PathRefs), `typed`
  (`str()` wrap), with special handling for ltm pool members,
  ltm policy rules, and iRule refs.

The `(kind, full_path)` compound cache key matters because
BIG-IP lets different kinds share a path string —
`/Common/shared` could simultaneously be a pool, a node, and an
iRule, and a single-key cache would let one kind's ObjectRef
leak into another's lookup.

#### `projection/_data.py`

Three master tables:

- `_KIND_FIELD_MAPS: dict[str, tuple[type, dict[str, FieldSpec]]]`
  — kind label → `(dataclass type, {tmsh-name → FieldSpec})`.
  Example: `"ltm virtual"` → `(BigipVirtual, _VS_FIELDS)` where
  `_VS_FIELDS["destination"] = FieldSpec("destination",
  typed=True)` and `_VS_FIELDS["pool"] = FieldSpec("pool",
  ref_kind="ltm pool")`.
- `_MODULE_KINDS: dict[str, dict[str, tuple[str, str]]]` —
  module → `{label → (attr-name, tmsh-kind)}`.  Example:
  `_MODULE_KINDS["ltm"]["virtual"] = ("virtuals", "ltm
  virtual")` — the `BigipConfig` attribute is `virtuals`, the
  dispatch label is `"ltm virtual"`.
- `_OBJECT_KIND_ALIASES` — frozenset of every TMSH kind label.
  Used for reverse-lookup / DSL-level kind checks (`kind ==
  "ltm virtual"`).

`_MINIMAL_FIELDS` is the shared minimal
projection (`name`, `full-path`, `kind`, `description`) used by
the ~hundreds of long-tail kinds without dedicated fields.  Each
module gets its own alias (`_LTM_MINIMAL_FIELDS`,
`_NET_MINIMAL_FIELDS`, …) for grep-ability; all aliases point at
the same dict.

### `builtins.py` — Builtin catalogue + registration

The registration mechanics live at the top of the file; the
~116 builtins follow.

- `BuiltinSpec` (frozen dataclass) — name, summary, signatures
  tuple, examples tuple, category, `impl` callable, `details`
  (markdown), and four runtime knobs:
  - `min_args` / `max_args` — strict arity check.
  - `special_form: bool` — when true, the evaluator passes AST
    nodes rather than pre-evaluated values.  Used for `select`,
    `map`, `if`, `as`, `with_entries`, etc.
  - `with_ctx: bool` — when true, the evaluator passes
    `EvalContext` as an extra `ctx=` kwarg.  Used for builtins
    that need access to the root, the edit plan, or merge mode.
  - `stream_aware: bool` — when true, the builtin handles its
    own stream-broadcast semantics; otherwise the dispatch
    unwraps streams element-by-element.
- `_REGISTRY: dict[str, BuiltinSpec]` — the master dispatch
  table.
- `_CATEGORY_ORDER` — tuple of seven category labels used for
  `--help-builtins` ordering: `("stream", "string", "path",
  "rename", "net", "graph", "value")`.
- `_register(name, *, summary, signatures, examples, category,
  min_args, max_args, details, special_form, with_ctx,
  stream_aware)` — decorator.  Populates `_REGISTRY[name]` and
  returns the function unchanged so it's still callable in
  Python.
- `lookup(name) → BuiltinSpec | None` — runtime dispatch hook
  for the evaluator.
- `list_builtins() → list[BuiltinSpec]` /
  `format_builtins(name=None) → str` — `--help-builtins`
  rendering.

`evaluator._eval_call` dispatches:

```
spec = builtins.lookup(node.name)
if spec.special_form:
    return spec.impl(*node.args, ctx=ctx)   # AST nodes
elif spec.stream_aware:
    return spec.impl(*evaluated_args, ctx=ctx)
else:
    # stream broadcast: invoke spec.impl once per stream element
```

### `_probes.py` — Network builtins (gated)

Two context variables drive the network surface:

- `PROBES_ENABLED: ContextVar[bool]` — the `--enable-probes`
  gate.  Per-task so concurrent queries don't share state.
- `TLS_CA_BUNDLE: ContextVar[str | None]` — optional CA path
  that overrides system trust for `url_request` /
  `tls_handshake`.

Six process-lifetime caches memoise probe
results within one query run:

```
_PING_CACHE, _PORTPING_CACHE, _TRACEROUTE_CACHE,
_URL_CACHE, _SOCKET_CACHE, _TLS_CACHE
```

Keys: `(host)` / `(host, port, proto)` / `(host, port, headers,
ca_path)` etc.  Multiple references to the same endpoint in one
query share the result.

Key functions:

- `_require_probes(name: str)` — gate check; raises
  `BuiltinError` with a fixed message every gated builtin uses.
- `ping(ip, *, timeout_s)` — ICMP via `subprocess` (`ping`
  binary).
- `portping(ip, port, *, protocol, timeout_s)` — TCP
  `socket.connect()` or UDP `sendto` / recv.
- `traceroute(ip, *, max_hops, timeout_s)` — `traceroute`
  subprocess.
- `url_request(method, url, headers, body, *, cookie_jar)` —
  `urllib` with `_CertCapturingHTTPSHandler` (see below) so the
  peer cert lands in the response dict.  On strict-TLS failure
  (`ssl.SSLCertVerificationError`), retries once with
  `_permissive_ssl_context()`; the structured `reason` field
  records which OpenSSL verify code triggered the retry.
- `tls_handshake(host, port, sni, alpn)` — explicit TLS probe;
  same strict→permissive retry, same `reason` shape.

Cert-projection support (added by the recent audit work):

- `x509_parse(pem)` — PEM → dict via `cryptography` when
  installed, with an `ssl`-only fallback path
  (`_x509_parse_ssl_fallback`) for environments without
  `cryptography`.
- `x509_from_config(cert)` — projects any BIG-IP config-object
  cert (`BigipSysFileSslCert`, `BigipCmCert`) into the same
  dict shape `x509_parse` produces.  Pure duck-typing on the
  dataclass field names so it works on both kinds without an
  `isinstance` switch.
- `x509_eq(a, b)` — cert-identity compare.  Fingerprint-first,
  falls back to subject + issuer + serial when one side is a
  BIG-IP projection (no SHA-256 fingerprint).
- `_classify_cert_error(exc) → (kind, message, fatal)` — the
  `reason` classifier; mapping table below.
- `_permissive_ssl_context()` — returns a context with
  `check_hostname=False` and `CERT_NONE`; used for the retry
  path.
- `_CertCapturingHTTPSHandler` — subclass of
  `urllib.request.HTTPSHandler` whose `_Conn.connect()` saves
  `self.sock.getpeercert(binary_form=True)` onto the handler so
  the response dict's `peer_cert` lands in the same handshake
  the body uses (no extra round-trip).

Reason classifier (`_VERIFY_CODE_TO_KIND` table):

| OpenSSL code | `reason.kind` | Meaning |
|---|---|---|
| 10 | `expired` | `X509_V_ERR_CERT_HAS_EXPIRED` |
| 9 | `not_yet_valid` | `X509_V_ERR_CERT_NOT_YET_VALID` |
| 18, 19 | `self_signed` | Depth-zero / chain self-signed |
| 20, 21, 24 | `untrusted_ca` | Chain doesn't reach a trusted CA |
| 62 | `hostname_mismatch` | SNI / SAN doesn't match |
| (other) | `other_verification` | Catch-all |
| (socket) | `connection_error` | DNS / refused / timeout — `fatal=True` |

`reason.fatal == True` is the only case where the response body
or peer cert may be absent.

```
        url_get(url) / tls_handshake(host, port)
                       │
                       ▼
            ┌─────────────────────┐
            │ strict TLS context  │  ssl.create_default_context()
            │ + cert_capturing    │  + TLS_CA_BUNDLE if set
            │ HTTPS handler       │
            └──────────┬──────────┘
                       │ connect
                       ▼
                ┌──────────────┐                ┌────────────────┐
            ┌───┤  success     │──► ok ───►     │ peer_cert from │
            │   └──────────────┘                │ same handshake │
            │                                   │ reason.kind="ok"│
            │   ┌──────────────┐                └────────────────┘
            └───┤  SSL verify  │
                │  failure     │
                └──────┬───────┘
                       │
                       ▼
            ┌─────────────────────┐
            │ _classify_cert_     │  exc.verify_code →
            │ error()             │  ("expired" | "self_signed" |
            └──────────┬──────────┘   "untrusted_ca" | …)
                       │
                       ▼
            ┌─────────────────────┐
            │ _permissive_ssl_    │  check_hostname=False
            │ context()           │  CERT_NONE
            └──────────┬──────────┘
                       │ retry
                       ▼
                ┌──────────────┐                ┌────────────────┐
            ┌───┤  success     │──► ok ───►     │ peer_cert from │
            │   └──────────────┘                │ retry handshake│
            │                                   │ reason.kind=   │
            │                                   │   <classified> │
            │                                   │ reason.fatal=  │
            │                                   │   False        │
            │                                   └────────────────┘
            │
            │   ┌──────────────┐                ┌────────────────┐
            └───┤  socket fail │──► error ───►  │ peer_cert=null │
                │ (DNS/refused │                │ reason.kind=   │
                │  /timeout)   │                │   "connection_ │
                └──────────────┘                │    error"      │
                                                │ reason.fatal=  │
                                                │   True         │
                                                └────────────────┘
```

`_CertCapturingHTTPSHandler` is installed on the urllib opener
*once*; the peer cert is recorded onto the handler during each
`_Conn.connect()`, so a follow-up `getpeercert(binary_form=True)`
isn't needed (no extra round-trip).

### `_inputs.py` — External input parsers

- `parse_jsonl(text, *, source)` — NDJSON.  Blank lines
  skipped; per-line errors carry a line number.
- `parse_csv(text, *, headers, source)` — CSV.  Header
  auto-detected from row 1 unless `headers=` overrides; extra
  columns land in an `_extra` list; missing columns become
  empty string.
- `parse_f5log(text, *, source)` — F5 syslog → list of
  `F5LogEvent` dicts.  Severity vocabulary is `_F5_SEVERITIES`
  (frozenset of the eight syslog severities).
- `F5LogEvent` (frozen dataclass) — 14 fields: `timestamp`,
  `host`, `severity`, `daemon`, `pid`, `code`, `module`,
  `level`, `message`, `raw`, etc.

These feed the `json_load` / `jsonl_load` / `csv_load` /
`f5log_load` builtins and the `--name=alias=path.{json,jsonl,csv}`
CLI sugar.

### `edit_plan.py` — Mutation queue

- `EditOp` (frozen) — one byte-level rewrite.  Fields:
  `source_uri`, `object_path`, `object_kind`, `field_name`,
  `operator` (`=`, `|=`, `+=`, `-=`), `old_value`, `new_value`,
  `field_slot`, `stanza_slot`, `strict` flag.
- `PrefixRewrite` (frozen) — whole-source regex sub used by
  `rename_partition` / `rename_prefix`.
- `EditPlan` (mutable) — `ops: list[EditOp]` +
  `prefix_rewrites: list[PrefixRewrite]`.  `add(op)`,
  `add_prefix(rewrite)`, `has_edits()`.
- `AppliedSource` (frozen) — result type per source.  `uri`,
  `original`, `new_source`, `rename_reports`, `field_edits`
  count.
- `apply(plan, sources) → dict[str, AppliedSource]` — the
  workhorse.  Three-phase per source:
  1. Apply `PrefixRewrite`s (whole-source `re.sub`).
  2. Apply identity-field renames (`name` / `full-path`) via
     `rename_object` — touches every reference site, not just
     the stanza header.
  3. Apply field-level edits sorted by `(start_offset,
     end_offset)` descending so later edits don't shift
     earlier spans.

  After splicing, re-parses through `parse_bigip_conf` to
  validate.  Malformed output aborts with `BuiltinError`
  carrying the offending edit.
- `_IDENTITY_FIELDS = frozenset({"name", "full-path"})` —
  drives the dispatch between identity-rename and field-edit
  branches.

Mixing a `PrefixRewrite` with non-identity field edits in the
*same statement* is rejected — the rewrite shifts byte offsets
of later edits.  Split the work across statements with `;`.

```
              EditPlan                                source text
              ┌───────────────┐                  ┌─────────────────┐
              │ prefix_rewrites│ ── 1 ──►        │  bigip.conf     │
              │  PrefixRewrite │                  │   (bytes)       │
              │  PrefixRewrite │                  └────────┬────────┘
              ├───────────────┤                            │
              │     ops       │ ── 2 ──► rename_object()  │
              │  EditOp (name)│                            │
              │  EditOp (name)│                            ▼
              ├───────────────┤                  ┌─────────────────┐
              │     ops       │ ── 3 ──►         │  rewritten      │
              │  EditOp (field)│  splice in       │  bytes          │
              │  EditOp (field)│  reverse offset  └────────┬────────┘
              │  EditOp (field)│  order                    │
              └───────────────┘                            ▼
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

### `output.py` — Renderers

`render(values, *, mode="auto") → str`
dispatches:

- `_render_auto(values)` — default.  If every
  value is an `ObjectRef` with `stanza_slot`, render SCF; if
  every value is a scalar or list of scalars, render `raw`
  (one per line); otherwise render JSON.
- `_render_scf(values)` — emit each object's
  `stanza_slot.raw_text` verbatim.  Preserves comments and
  whitespace exactly.
- `_render_raw(values)` — one scalar per line.  Rejects
  `ObjectRef` / `dict` to keep the output unambiguous.
- `_render_paths(values)` — print `full_path` of `ObjectRef` /
  `PathRef`.
- `_render_json(values)` — JSON array; `ObjectRef` flattens to
  its field dict.
- `_render_tmsh(values)` — TMSH command lines (`tmsh modify
  ltm virtual …`); only used for the mutation-as-tmsh output
  mode.

The `--output` CLI flag picks the mode; `auto` is the default
because it produces the most "natural" form for the values.

### `runner.py` — Orchestration

`run_query(query, sources, *, names, merge, partitions,
json_sources, input_specs) → QueryResult`.

Per-source vs merged flow:

- `_run_query_per_file(program, sources, names) → QueryResult` —
  default.  Runs once per source; each gets its own
  `EvalContext`.
- `_run_query_merged(program, sources, names) → QueryResult` —
  `--merge` mode.  All sources land in `named_roots`; the
  evaluator sees them as a single stream.

Multi-statement loop in `run_query`:

```
for stmt in program.statements:
    values = evaluate_statement(stmt, ctx)
    applied = edit_plan.apply(ctx.edits, sources)
    sources = {uri: a.new_source for uri, a in applied.items()}
    ctx.edits = EditPlan()        # reset between statements
    # rebuild ctx.root from the rewritten sources
```

That's what makes `rename_partition("Common", "Tenant_A") ;
.ltm.virtual[] | .destination |= …` correct — the second
statement parses the renamed source.

```
    program.statements = [stmt₁, stmt₂, stmt₃]

       sources₀                          ┌──────────────┐
          │                              │   stmt₁      │
          │                ┌────────────►│  evaluate    │
          │                │              │   ↓ emits   │
          ▼                │              │  EditPlan₁  │
    ┌────────────┐         │              └──────┬──────┘
    │ build ctx, │─────────┘                     │ apply
    │ ctx.root   │                               ▼
    └────────────┘                        ┌──────────────┐
          ▲                               │  sources₁    │
          │                               │  (rewritten) │
          │ rebuild root                  └──────┬───────┘
          │                                      │
          │                ┌────────────►┌──────────────┐
          │                │              │   stmt₂      │
          └────────────────┤              │  evaluate    │
          ▲                │              │   ↓ emits   │
          │                │              │  EditPlan₂  │
          │ rebuild root   │              └──────┬──────┘
          │                │                     │ apply
          │                │                     ▼
          │                │              ┌──────────────┐
          │                │              │  sources₂    │
          │                │              └──────┬───────┘
          │                │                     │
          │                └────────────►┌──────────────┐
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

Four contextvars carry per-query state across the call stack
without threading it through every helper:

- `_ACTIVE_ROOTS: ContextVar[dict[str, Root]]` — per-URI roots
  for `refs` / `referenced_by` builtins.
- `_MERGE_ACTIVE: ContextVar[bool]` — merge-mode flag for graph
  builtins.
- `_ACTIVE_PARTITIONS: ContextVar[dict[str, str]]` — partition
  remap for `rename_partition`.
- `_INPUT_SPECS: ContextVar[dict[str, InputSpec]]` — side-input
  format specs for `--name=alias=path.json`.

`QueryResult` (dataclass) is the final shape: `values_per_file`
(URI → values), `edits_per_file` (URI → `AppliedSource`),
`has_mutation` bool.

### `source_map.py` — Error positioning

Small (48 lines).  `SourceMap` precomputes line-start offsets
once per source; `line_col(offset) → (line, col)` is a binary
search.  Used by `ParseError` / `EvalError` to underline the
offending position in error output.

### `graph.py` — Reference traversal

Backs the `refs(obj)` / `referenced_by(obj)` builtins.

- `forward_refs(obj) → list[str]` — runs `compute_grep`
  (registry-driven) in forward direction; returns target
  full-paths.
- `reverse_refs(obj) → list[str]` — reverse direction.
- `_grep_inputs(obj) → (sources, configs)` — scope: just
  `obj.config_uri` by default; all active roots when
  `_MERGE_ACTIVE` is true.

The actual reference index lives in
[`docs/design/bigip-registry-architecture.md`](bigip-registry-architecture.md)
— `compute_grep` consults the registry's reference table.

## Cross-cutting invariants

### Streams vs lists

`Stream` is the lazy form, a Python `list` is the materialised
form.  `|` iterates streams element-wise but passes lists
whole.  Aggregator builtins (`sort`, `unique`, `count` over a
collection) require a list; force collection with `[ … ]`.

The line gets crossed by `Subscript(stream=True)` (the bare
`.foo[]` form), which produces a `Stream`, and by `[ expr ]`
(ListLiteral), which collects a stream back into a list.

### PathRef auto-deref

`.pool` returns a `PathRef`; `.pool.members` then auto-derefs
the PathRef to the actual pool `ObjectRef` and indexes its
`members` field.  Resolution is lazy via
`evaluator._resolve_pathref` and uses `PathRef.expected_kind`
when set to keep lookup proportional.

### Object cache identity

`Root._object_cache: dict[(kind, full_path), ObjectRef]`.  The
compound key prevents one kind's ObjectRef from leaking into
another kind's lookup (BIG-IP lets different kinds share a path
string).

### Source-range fidelity

Every `ObjectRef.field_slots[name]` is a `FieldSlot` — a half-
open byte span in the originating source for that property.
The edit planner uses these to splice edits at exact bytes; no
regex search-and-replace step.  Slots come from the registry's
value-spec layer; see
[`bigip-registry-architecture.md`](bigip-registry-architecture.md).

### Probe gating

`PROBES_ENABLED` is a per-task `ContextVar`.  The CLI's
`--enable-probes` flag flips it for one query run; concurrent
queries don't share state.  Every network builtin calls
`_require_probes(name)` before doing anything else.

### Permissive vs strict TLS

`url_request` and `tls_handshake` retry once with verification
disabled when strict TLS fails.  The retry is the audit-
friendly default — verification status flows out via the
structured `reason` field rather than aborting collection.
`reason.fatal == True` is the only case where data is
unavailable.

The peer cert is captured during the strict handshake when it
succeeds, and during the permissive retry when it doesn't —
both via `_CertCapturingHTTPSHandler` / `tls_handshake`'s
explicit `getpeercert(binary_form=True)` call.

### ContextVar scoping

The runner uses contextvars for state that would otherwise
need to be threaded through dozens of helpers.  All
`_ACTIVE_*` vars are scoped per-query: the runner `set()`s
them on entry, `reset()`s on exit.  Builtins read them
directly via `var.get()`.

## Edit-plan apply order

`edit_plan.apply(plan, sources)` walks `plan.ops` and:

1. Splits ops into per-source bins (one bin per loaded config
   file).
2. Applies `PrefixRewrite`s first (`re.sub` over the whole
   source).
3. Applies identity-field renames via `rename_object` (touches
   every reference site, not just the stanza header).
4. Sorts the remaining field-level edits by `(start_offset,
   end_offset)` *descending* so later edits don't shift earlier
   byte spans.
5. Splices each op into the source bytes.
6. Re-parses the spliced source through `parse_bigip_conf` —
   any malformed output aborts with a `BuiltinError` carrying
   the offending edit.

The reparse step is what makes mutating queries safe: the
engine never ships output a fresh `parse_bigip_conf` couldn't
read back in.

## Builtin extension points

To add a new builtin:

1. Decorate it with `@_register(name, …)` in `builtins.py`.
2. Add at least one example to the spec's `examples` tuple.
3. Add a test in `tests/test_f5_query*.py`.
4. Regenerate the catalogue: `python3 scripts/dev/gen_query_builtins_doc.py`.
5. CI verifies the catalogue is up to date via
   `test_generated_builtins_doc_is_up_to_date`.

For network builtins, also:

- Add the gate check (`_require_probes(name)`) as the first
  call in the implementation.
- Add a process-lifetime cache if multiple references to the
  same target in one query are likely.
- If you're touching TLS, route through `url_request` /
  `tls_handshake` so the `reason` classifier and the cert-
  capturing handler stay consistent.

To add a new projection field on an existing kind:

1. Add the field to the dataclass in `core/bigip/model/`.
2. Populate it in `core/bigip/parser/_parsers.py`.
3. Add a `FieldSpec` entry in the per-kind map in
   `core/bigip/query/projection/_data.py`.
4. Add a regression test that projects the new field.

To add a new value-spec (typed property handling): see
[`bigip-registry-architecture.md`](bigip-registry-architecture.md).

To add a new output mode: add a `_render_<mode>` function in
`output.py`, register it in `render`'s dispatch, and update the
CLI's `--output` choices in `explorer/verbs/f5/query.py`.

## Related design docs

- [`bigip-registry-architecture.md`](bigip-registry-architecture.md)
  — registry contract for value specs, source ranges,
  references.
- [`f5-query-projection-gaps.md`](f5-query-projection-gaps.md) —
  working checklist of TMSH properties the typed projection
  doesn't surface yet.
- [`f5-cli-architecture.md`](f5-cli-architecture.md) — verb
  registry, command dispatch, output format plumbing.
