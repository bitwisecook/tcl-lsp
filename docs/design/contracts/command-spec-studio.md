# Contract: the command-registry spec studio

The spec studio is a browser front-end over `tcl-registry`: it browses the
live command surface, edits every field of a `CommandSpec`, and renders the
result back to a registry `.rs` module or a Tcl dialect stub. This document
describes the contract between its four layers and the invariants that keep it
from drifting away from the registry it edits.

Related: [`command-registry.md`](../compiler/command-registry.md) (the field
reference), [`dialect-stubs.md`](dialect-stubs.md) (the stub language),
[`proc-arg-traits.md`](proc-arg-traits.md) (the inference the importer reads).

## Layers

| Layer | Crate / path | Role |
|---|---|---|
| Schema + renderers | `rust/tcl-spec-studio` | One table describing every spec field; the draft model; the `.rs` and stub renderers; package inference. No browser, no WASM. |
| WASM facade | `rust/tcl-spec-studio-wasm` | `wasm-bindgen` cdylib. Every export takes and returns a JSON string. Excluded from the workspace (the glue needs `unsafe`). |
| Front-end | `rust/tcl-spec-studio/web` | Strict TypeScript, bundled by esbuild to one IIFE. Generates its form from the schema; knows no field names. |
| Assembly | `rust/tcl-spec-studio-wasm/build-wasm.sh` | Inlines wasm + glue + stylesheet + bundle + logo into one `dist/index.html`. |

`make spec-studio-wasm` runs the last three in order. The published page is a
single self-contained file with `connect-src 'none'`.

## The schema is the single source of truth

`schema::COMMAND_FIELDS` and `schema::SUBCOMMAND_FIELDS` carry one
`FieldSchema` per Rust field: its key (the Rust field name, the draft's JSON
key, and the identifier the renderer emits), a label, a one-line help string,
a group, and a `FieldKind`.

Everything else reads that table:

- the form builds one editor per `FieldKind` — it never names a field;
- `draft` seeds one JSON key per schema entry;
- `render_rs` walks the same table in order.

**Adding a field to `CommandSpec` means adding one `FieldSchema` entry and one
line in `draft`.** No UI, serialiser, or renderer change is needed.

### Invariant: the schema covers every spec field

`tests/schema_coverage.rs` reads `tcl-registry/src/spec.rs` at compile time
(via `include_str!`), extracts the field list from the `CommandSpec::DEFAULT`
and `SubCommand::DEFAULT` initialisers, and compares it against the schema in
both directions. A field added to the registry without a schema entry fails
the test by name — otherwise the studio would silently drop it, and a draft
seeded from a real command would render back having lost behaviour.

### Invariant: the catalogues cover every variant

`catalogue` holds the registry's enum and bitflag vocabularies. Each catalogue
over a plain enum has a `covered` witness in the test module: an exhaustive
`match` that fails to compile the moment a variant is added, with a doc
comment naming the catalogue to update alongside. `AppendedArity` and
`BodyKind` are `#[non_exhaustive]`, so their witnesses need a wildcard and
cannot compile-gate; that limitation is stated at each one.

## The draft model

A draft is a plain JSON object keyed by Rust field name. `Draft` is
`serde_json::Map<String, Value>` — deliberately untyped, because the schema
already describes the shape and a parallel Rust struct would be a second
place to update.

`draft::from_command_spec` seeds a draft from a live `CommandSpec`. This is
what makes the studio a *browser* of the registry as well as an editor.

### Fields that cannot round-trip

Some fields hold a function pointer (`arg_role_resolver`, `const_fold`,
`taint_sink_gate`, …) or a reference to a `&'static` descriptor
(`definition_body`, `case_list`, `object_class`, …). Rust can observe that
such a field is `Some`, but not recover the expression that set it.

Seeding records those keys under `draft::UNRENDERABLE_KEY` (`__unrenderable`).
The form warns about them and the renderer emits a `TODO` comment naming each
one. **A field the studio cannot recover is never dropped silently** — the
rendered file says what is missing.

Those fields use `FieldKind::RustExpr`: the value is a string emitted
verbatim, so it carries its own `Some(…)` and type path. The schema's `hint`
shows the exact expression shape expected.

One unrecoverable expression is not a top-level field. `OptionArity::Hook`
holds a function pointer inside an *option row*, so it gets a `hook fn` text
box in that row rather than an entry under Advanced, and its
`__unrenderable` key is `draft::OPTION_HOOK_KEY` (`options.arity_hook`)
instead of a field name. Both the form's warning list and the renderer's
`TODO` resolve that key against the `options` array, so they name the exact
options still missing a hook — `return`'s `-errorstack` is the registry's
live example — and both clear once every hook holds an expression. Before
this, the whole `options` field was reported unreadable even though only one
option's arity was, and the note could never clear because the filled-in
check only understood string-valued fields.

## Renderer contract

`render_rs::render` produces a complete `tcl-registry/src/commands/<pack>/`
module: the AGPL copyright banner, a module doc line, the imports the emitted
literals need, hoisted `const` tables for options / forms / subcommands, and a
`spec()` returning the `CommandSpec`.

Only fields differing from `CommandSpec::DEFAULT` are emitted; the rest come
from the trailing `..CommandSpec::DEFAULT`, matching every hand-written spec.

Four rules the output must satisfy, each of which a real bug violated:

1. **Bitflag unions use `.union(…)`, never `|`.** The option and subcommand
   tables are hoisted into `const` items and `bitflags`' `BitOr` is not
   `const`, so a `|` chain there fails to compile.
2. **A nested enum payload keeps its own type path.** `Debug` prints
   `VarWriteTyping::Fixed(TclType::String)` as `Fixed(String)`, so the three
   enums with payloads have explicit, exhaustively-matched expression
   builders in `draft`.
3. **`Arity::stepped` is an associated function**, taking all three bounds —
   not a builder method off `at_least`.
4. **An unknown dialect name renders as a comment, not a bare identifier.**
   `DialectSet::f5-tmsh` is not valid Rust; emitting it silently produced a
   file that only failed at `cargo build`.

### Verifying the output compiles

`tests/render_sweep.rs` renders every command in every browsable dialect and
asserts the structural invariants. Those assertions cannot prove the result is
valid Rust — all four bugs above passed them. The real check is to render the
specs into the registry and build it; the procedure is documented at the top
of that test file. Running it found and fixed all four.

## Stub renderer contract

`render_stub` emits the `stub NAME {params} ?flags?` line of
[`dialect-stubs.md`](dialect-stubs.md), in either the inline
`# tcl-lsp: stubs-begin` block or a standalone `<dialect>.tcl.stubs` file.

The stub language is narrower than `CommandSpec`: no subcommands, options,
types, or hooks. **What a stub cannot carry is emitted as a comment beside
it**, never dropped silently — declared subcommands and options are listed,
the return type is stated, and any argument role with no stub spelling is
named as falling back to `value`.

Roles map through the inverse of `StubOverlay::parse_role`, so a stub the
studio renders parses back to the roles the draft declared.

## Inference contract

`infer::import_package` runs the analyser over a package's sources and turns
each `proc` into a draft:

| Draft field | Derived from |
|---|---|
| `arity` | the parameter list — defaults are optional, trailing `args` is variadic |
| `arg_roles` | `ProcArgTrait` from [proc-arg-trait inference](proc-arg-traits.md), deep pass enabled |
| `traits` | the same trait observations |
| `hover`, `forms` | the `proc`'s doc comment and parameter list |
| `required_package`, `min_version` | `package provide` |

`ProcArgTrait::DynamicNameLocal` maps to **no** role: it is callee-local, so
passing a literal does not consume the caller's variable and marking it
`VarWrite` would be wrong.

Every inferred draft carries `Inferred::notes` — one line of evidence per
guess, surfaced in the UI. **Inference reports its reasoning, never a bare
assertion.**

Procedures are deduplicated by qualified name across files, last definition
winning, matching what the interpreter would end up with.

## Trait names come from the registry

`catalogue::trait_keys` renders a spec's traits by asking the registry for
them (`Traits::iter_names`), and `catalogue::trait_bit` resolves a name
against `Trait::ALL`. There is no name↔bit table in the studio to drift out of
step with the registry, and
`catalogue::tests::traits_catalogue_matches_the_registry_flags` asserts the
studio's descriptive catalogue covers exactly the registry's declared flags,
in order.

This previously needed a workaround. `Traits` was a `bitflags` `u64` holding
65 flags, with `SAFE_INTERP_HIDDEN` and `TRANSFERS_CONTROL` both spelled
`1 << 61` — the same flag at run time (issue #1031) — so `trait_keys` had to
deduplicate by bit *value* to avoid rendering a spec that claimed a trait its
author never set. The registry now derives each bit from an enum discriminant,
so two flags cannot share one and the deduplication is gone.

## Publishing

The page ships to GitHub Pages at `/spec-studio/` alongside the compiler
explorer, the BIG-IP report generator, and the BIG-IP report demo — one Pages
artefact holds all four (see `.github/workflows/github-pages.yml`).

`wasm-opt` is deliberately not run, for the same reason as the explorer and
the report generator: binaryen mis-rebinds `__wbindgen_externrefs` onto the
fixed-size funcref table, breaking `Table.grow` at run time. The build
verifies the externref table is growable instead.
