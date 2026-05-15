# `f5 query` — Engine Internals

Architecture, invariants, and data-flow for the `f5 query` engine.
This is the contributor doc — what you need to know to extend or
debug the query DSL.  The user-facing surface (every function,
operator, flag, behaviour) lives in
[`docs/references/f5_query/`](../references/f5_query/).

## Module layout

```
core/bigip/query/
├── __init__.py          # Public re-exports: run_query, format_*, list_builtins
├── lexer.py             # Tokeniser — single pass, no lookahead beyond one char
├── parser.py            # Recursive-descent parser → ast.* nodes
├── ast.py               # Frozen dataclasses for every expression form
├── evaluator.py         # Walks the AST against a BigipConfig root
├── projection/          # Typed-value projection (FieldSpec dispatch)
│   ├── _engine.py       # _project_field — the dispatch core
│   ├── _classes.py      # Container, FieldSpec, ObjectRef
│   └── _data.py         # Per-kind field maps (manually curated)
├── builtins.py          # The @_register'd builtin function catalogue
├── _probes.py           # Network builtins (gated by --enable-probes)
├── _inputs.py           # External input parsers (JSON / JSONL / CSV / f5log)
├── values.py            # PathRef, Stream, Root — DSL value types
├── edit_plan.py         # EditOp, EditPlan, PrefixRewrite — mutation queue
├── output.py            # Renderers: raw / paths / json / scf / tmsh
├── runner.py            # Top-level: run_query() orchestration
├── grammar.py           # `--help-dsl` plain-text grammar reference
├── examples.py          # `--help-examples` cookbook
└── errors.py            # ParseError, EvalError, BuiltinError, QueryError
```

## Pipeline

A query goes through five distinct stages, each owned by one
module:

1. **Lex** — `lexer.py::tokenise(text)` → `list[Token]`.  Single
   pass, no lookahead beyond one character.  Recognises path
   tokens (`.`, `..`), operators (`|`, `==`, `|=`, etc.),
   literals (numbers, strings, `true`/`false`/`null`), and
   identifiers (which can contain `-` for TMSH-spelt names like
   `source-address-translation`).
2. **Parse** — `parser.py::parse(tokens)` → `ast.Program`.
   Recursive-descent, hand-written.  Grammar in
   [`docs/references/f5_query/dsl.md`](../references/f5_query/dsl.md).
   Output is a tree of frozen dataclasses (`ast.py`).
3. **Evaluate** — `evaluator.py::evaluate(program, ctx)` → list
   of values.  Walks the AST against `ctx.root` (a
   `values.Root`).  Streams flatten lazily through `_flatten`;
   `_pipe_through` is the core pipe semantics.  Builtins
   dispatch through `_eval_call` which consults
   `builtins._builtins`.
4. **Plan edits** — every assignment node (`Assignment`,
   `rename(...)`, `rename_partition(...)`) emits an `EditOp`
   into `ctx.edits.ops` rather than mutating during evaluation.
   The plan is a flat list of ops with a stable application
   order (see `_apply` below).
5. **Apply + render** — `runner.py::run_query` applies the edit
   plan to a fresh copy of each source's text, parses the
   rewritten text back into a `BigipConfig` to validate, and
   hands the values + rewritten sources to `output.py::render`.

## Key invariants

### Streams vs lists

A `values.Stream` is the lazy form; a Python `list` is the
materialised form.  `|` iterates streams element-wise but passes
lists whole.  Aggregator builtins (`sort`, `unique`, `count`
over a collection) require a list; force collection with
`[ ... ]`.

### PathRef auto-deref

`values.PathRef` is a typed full-path that auto-dereferences
under field access: `.pool` returns a `PathRef`; `.pool.members`
then resolves the PathRef to the actual pool ObjectRef and
indexes its `members` field.  Resolution is lazy via
`projection._engine._resolve_pathref` and uses an `expected_kind`
hint when available to keep lookup proportional to the kinds the
query actually touches.

### Object cache identity

`projection._engine` caches built ObjectRefs by `(kind,
full_path)` — BIG-IP lets different kinds share a path string
(`/Common/shared` could be a pool, a node, and an iRule), so a
single-key cache would let one kind's ObjectRef leak into
another's lookup.  The compound key keeps the dispatch correct.

### Source-range fidelity

Every `values.ObjectRef` carries `field_slots: dict[str,
FieldSlot]` — half-open byte spans in the originating source for
each projected field.  The edit planner uses these to splice
edits at the exact bytes; no regex search-and-replace step.
Field spans come from the registry's value-spec layer (see
[`docs/design/bigip-registry-architecture.md`](bigip-registry-architecture.md)).

### Probe gating

`_probes.PROBES_ENABLED` is a `ContextVar`.  The CLI's
`--enable-probes` flag flips it for the duration of one query
run; the contextvar is per-task so concurrent queries don't
share state.  Every network builtin calls `_require_probes(name)`
before doing anything else; gate failures raise `BuiltinError`
with the documented message.

### Permissive vs strict TLS

`_probes.url_request` and `_probes.tls_handshake` retry once
with verification disabled when strict TLS fails.  The retry is
audit-friendly default behaviour — verification status flows out
via the structured `reason` field rather than aborting the
collection.  `reason.fatal == True` is the only case where data
is unavailable.

## Edit-plan apply order

`runner._apply` walks `ctx.edits.ops` in source order and:

1. Splits the ops into per-source bins (one bin per loaded
   config file).
2. Sorts each bin by `(start_offset, end_offset)` descending so
   later edits don't shift earlier byte spans.
3. Splices each op into the source bytes; preserves untouched
   surrounding text.
4. Re-parses the spliced result through `parse_bigip_conf` —
   any rejected output (malformed SCF) aborts the run with a
   `BuiltinError` carrying the offending edit.

The reparse step is what makes mutating queries safe: the engine
never ships output a fresh `parse_bigip_conf` couldn't read back
in.

### Multi-statement edit semantics

For `a ; b ; c` mutating queries the runner drives one statement
at a time:

```
for stmt in program.statements:
    values = evaluate_statement(stmt, ctx)
    apply(ctx.edits, sources)
    ctx.edits = EditPlan()        # reset between statements
    sources = reparsed_sources
```

This way `rename_partition("Common", "Tenant_A") ; .ltm.virtual[]
| .destination |= ...` sees the renamed objects when the second
statement runs.

## Builtin registration

`builtins._register(name, ...)` is a decorator that captures the
function's signature, docstring, examples, and category into a
`BuiltinSpec`.  Specs land in the module-level `_builtins`
registry.  Three runtime knobs:

- `min_args` / `max_args` — strict arity check at call time.
- `special_form` — when true, the evaluator passes the AST nodes
  to the function (not pre-evaluated values).  Used for
  `select`, `map`, `if`, `as`, etc.
- `with_ctx` — when true, the evaluator passes the `EvalContext`
  as an extra `ctx=` kwarg.  Used for builtins that need access
  to the root, the edit plan, or the merge mode.
- `stream_aware` — when true, the builtin handles its own
  stream-broadcast semantics; otherwise the dispatch unwraps
  streams before calling.

## Implicit-dot and implicit-receiver call forms

`_eval_call` recognises two jq-style shorthands:

- **Implicit dot** — when a one-arg builtin is invoked without
  parens (`length`, `sort`, `count` as pipeline stages), the
  current input becomes the single argument.  Skipped for
  `special_form` builtins (they need explicit bodies).
- **Implicit receiver** — when a multi-arg builtin is invoked
  with `min_args - 1` arguments, the current input is prepended
  as the first arg.  `.x | contains("foo")` desugars to
  `contains(.x, "foo")`.

These are tested in `tests/test_f5_query.py::test_bare_builtin_*`.

## Reason classifier

`_probes._classify_cert_error` maps OpenSSL verify codes to the
`reason.kind` taxonomy users filter on.  Codes the table doesn't
recognise fall through to `"other_verification"`.  Connection-
level failures (DNS / refused / timeout) are tagged
`"connection_error"` with `fatal=True`.

| Verify code | `kind` | Meaning |
|---|---|---|
| 10 | `expired` | `X509_V_ERR_CERT_HAS_EXPIRED` |
| 9 | `not_yet_valid` | `X509_V_ERR_CERT_NOT_YET_VALID` |
| 18, 19 | `self_signed` | Depth-zero / chain self-signed |
| 20, 21, 24 | `untrusted_ca` | Chain doesn't reach a trusted CA |
| 62 | `hostname_mismatch` | SNI / SAN doesn't match |
| (other) | `other_verification` | Catch-all |

## Extension points

To add a new builtin:

1. Decorate it with `@_register(name, ...)` in `builtins.py`.
2. Add at least one example to the spec's `examples` tuple.
3. Add a test in `tests/test_f5_query*.py`.
4. Regenerate the catalogue: `python3 scripts/dev/gen_query_builtins_doc.py`.
5. CI verifies the catalogue is up to date via
   `test_generated_builtins_doc_is_up_to_date`.

To add a new projection field on an existing kind:

1. Add the field to the dataclass in `core/bigip/model/`.
2. Populate it in `core/bigip/parser/_parsers.py`.
3. Add a `FieldSpec` entry in the per-kind map in
   `core/bigip/query/projection/_data.py`.
4. Add a regression test that projects the new field.

To add a new value-spec (typed property handling): see
[`bigip-registry-architecture.md`](bigip-registry-architecture.md).

## Related design docs

- [`bigip-registry-architecture.md`](bigip-registry-architecture.md)
  — registry contract for value specs, source ranges,
  references.
- [`f5-query-projection-gaps.md`](f5-query-projection-gaps.md) —
  working checklist of TMSH properties the typed projection
  doesn't surface yet.
- [`f5-cli-architecture.md`](f5-cli-architecture.md) — verb
  registry, command dispatch, output format plumbing.
