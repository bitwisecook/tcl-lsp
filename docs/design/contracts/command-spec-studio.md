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
| Front-end | `rust/tcl-spec-studio/web` | Strict TypeScript, bundled by esbuild into two files: the controller (an IIFE, inlined into the page) and the editor chunk (an ES module, loaded on demand). Generates its form from the schema; knows no field names. |
| Language server | `rust/tcl-lsp-server-wasm` | The **real** `tcl-lsp-server` `LspService`, compiled to wasm and driven over `postMessage` from a Web Worker. The studio's editors are a client of it. |
| Assembly | `rust/tcl-spec-studio-wasm/build-wasm.sh` | Inlines wasm + glue + stylesheet + controller + logo into `dist/index.html`, and copies the editor chunk and the server worker in beside it. |

`make spec-studio-wasm` runs all of the above in order — it depends on
`make lsp-server-wasm` for the worker.

## Dist layout

The dist is a **directory**, deployed whole:

```
index.html                       the page; studio wasm + glue + CSS + controller inlined
assets/monaco-host.js            Monaco + the language client (lazy, ~2.7 MB)
assets/monaco-host.css           its stylesheet, with the codicon font as a data: URI
lsp/worker.js                    the language server worker
lsp/tcl_lsp_server_wasm.js       its wasm-bindgen glue
lsp/tcl_lsp_server_wasm_bg.wasm  the server (~21 MB raw, ~5.6 MB gzipped)
```

Two properties hold this together and must be kept:

1. **No static relative reference.** `index.html` links no file. The editor
   chunk is reached by `new URL("assets/monaco-host.js", document.baseURI)`,
   its stylesheet by `new URL("./monaco-host.css", import.meta.url)`, and the
   worker's own two files by `new URL(".", self.location.href)`. That is what
   lets the same dist work at a site root, under Pages' `/spec-studio/`, and
   under a local `python3 -m http.server` with no rewriting. `build-wasm.sh`
   asserts it: a `<script src=…>` or `<link href=…>` in the page's markup
   fails the build.
2. **The three `lsp/` files stay together under their own names.**
   `worker.js` derives the glue and the `.wasm` from its own location.

### Content-Security-Policy

`connect-src` is no longer `'none'` — it is `'self'` plus exactly
`https://api.github.com` and `https://codeload.github.com`, which exist for the
one opt-in panel described below. Everything else is same-origin: `script-src
'self' 'unsafe-inline' 'wasm-unsafe-eval'`, `worker-src 'self'` (**not**
`blob:`), `style-src 'self' 'unsafe-inline'`, `font-src data:`. The page's
privacy notice names the GitHub panel as the sole exception, and the boot check
(below) fails if the page reaches GitHub without being asked.

## The editors are clients of the real language server

The Pack DSL pane and the Test pane are Monaco editors whose language features
all come from `rust/tcl-lsp-server-wasm` — the same `LspService<Backend>` the
native binary runs, with a `postMessage` transport in place of stdio. Nothing in
the front-end decides what a word means.

| Concern | Where it lives |
|---|---|
| Transport | `vscode-jsonrpc`'s `BrowserMessageReader`/`Writer` over `new Worker("lsp/worker.js")` — one JSON-RPC message per `postMessage`, no `Content-Length` framing, which is exactly what `worker.js` was built to speak. |
| Session | `web/src/lspClient.ts` — the `initialize` handshake behind a 30 s deadline, open-document bookkeeping, push-diagnostics fan-out. No protocol code: the request types come from `vscode-languageserver-protocol`. |
| Providers | `web/src/monacoHost.ts` — semantic tokens, hover, completion, signature help, formatting, and diagnostics, each a translation of one LSP reply into one Monaco shape. |
| Contract | `web/src/editorHost.ts` — the interface `studio.ts` holds. `studio.ts` imports no Monaco and no LSP type. |

**Dialect is the language id.** The `.tclspec` document opens as `tclspec`; the
Tcl sample opens under whichever dialect the studio's selector names
(`tcl9.0`, `f5-irules`, …), and changing the selector closes and re-opens it.
The server's `dialect_from_language_id` accepts both spellings, so there is one
rule for how a document's dialect is decided rather than a `didChangeConfiguration`
round-trip that can disagree with it.

**The semantic-token legend is the server's.** The client sends an empty
`tokenTypes` list in `initialize` and maps whatever legend comes back, so a
token type the server gains later paints as plain text rather than shifting
every colour by one.

### The fallback ladder

Three rungs, each announced in the page's `#lspStatus` line rather than
swallowed:

1. **Monaco + the language server** — the full experience.
2. **Monaco alone** — the worker did not start (wasm disabled, a partial
   deploy). The editor still edits; the status line says there is no analysis.
3. **The textarea and the `dsl_highlight` overlay** — the editor chunk itself
   never loaded (a `file://` page, an old browser). This is the surface the
   studio had before Monaco, kept in the bundle for exactly this reason:
   `web/src/dslEditor.ts` is ~170 lines and stays.

The textarea is the state of record on every rung. Monaco writes through to it
on each change, and `studio.ts` writes to both, so the two never disagree and
rung 3 needs no separate state.

### Bundle discipline

The controller (~113 KB) is what every visitor loads; the editor chunk (~2.7 MB
minified, ~680 KB gzipped) and the server wasm (~5.6 MB gzipped) load only when
an editor tab is opened. esbuild's code splitting cannot express this — it needs
`format: "esm"` for the whole build and the controller must stay a classic
script — so `build.mjs` runs two builds and `studio.ts` reaches the second one
through a dynamic `import()` of a **runtime-built** URL, which is what stops
esbuild pulling Monaco back into the first.

Third-party licences: `web/THIRD-PARTY-NOTICES.md`, restated as a banner
comment at the top of the editor chunk.

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

### Long-form help rides the schema

`help.rs` carries the Tcl-developer-facing text behind the form's **?**
buttons and the Reference tab: one long-form entry per field key (shared
between the command and subcommand tables), one per group heading, and a
`(title, intro)` pair per catalogue id. `FieldSchema::to_json` resolves the
field entry into the schema JSON as `help`, and `schema::to_json` adds
`groupHelp` and `catalogueHelp` maps — so the front-end still knows no field
names, and the Reference tab is rendered entirely from the same wire schema
the form reads.

The tests in `help.rs` enforce coverage in both directions: every schema
field, group, and catalogue must have help (a new field fails by name until
its entry is written), and every help entry must name something that still
exists. "A **?** on everything" is therefore a property of the build, not a
review habit.

### Invariant: the schema covers every spec field

`rust/tcl-spec-studio/tests/schema_coverage.rs` reads `tcl-registry/src/spec.rs` at compile time
(via `include_str!`), extracts the field list from the `CommandSpec::DEFAULT`
and `SubCommand::DEFAULT` initialisers, and compares it against the schema in
both directions. A field added to the registry without a schema entry fails
the test by name — otherwise the studio would silently drop it, and a draft
seeded from a real command would render back having lost behaviour.

### Invariant: every `CommandSpec`-family field is surfaced or excluded

The text scan above only reaches the two `DEFAULT` initialisers, and only
compares against the schema. `coverage.rs` is the load-bearing gate: it makes
the property a **build failure** rather than a test, and it reaches the whole
family — the nested types a draft carries structurally (`OptionSpec`,
`OptionArg`, `ArgValue`, `FormSpec`, `HoverSnippet`, `SideEffect`,
`SetterConstraint`, `Arity`, `ArgTypeHint`, `Lifecycle`, `SubSubCommand`), the
plain-data descriptors (`RepeatedArgLayout`, `HandleBindingSpec`,
`HandleKeyword`, `SymbolDef`, `BytePayloadSpec`, `VersionedArgValue`), and the
shared `&'static` descriptors (`DefinitionBodyGrammar`, `MemberBodyCommand`,
`ObjectClassSpec`, `CaseListSpec`).

Each covered type gets a pair:

- a `witness_*` function holding an **exhaustive destructuring pattern with no
  `..` rest**, so a field added to the registry type fails to compile there,
  naming it (`error[E0027]: pattern does not mention field <name>`), and a
  field removed fails too;
- a `&[Field]` table saying where the studio surfaces each of those fields:
  `Surface::Key` (a draft/schema key), `Surface::Keys` (a `Lifecycle`'s three
  releases), `Surface::Expression` (rendered into the Rust expression a named
  field holds), or `Surface::Excluded` **with a reason**.

This is the field-level twin of the catalogue witnesses below, which use
exhaustive `match`es so a new enum *variant* breaks the build.

The tests then prove the claim: a `Key` entry must be both a schema key and a
key the seeder writes (which, because `render_rs` walks the schema and the WASM
`schema()` serialises it, is what carries a field through all four layers); an
`Expression` entry's field name must appear in the literal a rendered spec
emits; and no schema key may be left without a coverage entry.

**Excluded by decision, not by accident.** The only exclusions today are the
fields of `DefinitionBodyGrammar`, `MemberBodyCommand`, `ObjectClassSpec`, and
`CaseListSpec`: each is a shared registry constant that many commands
reference, so the studio's editor takes the *constant's path*
(`Some(&definer::SNIT_GRAMMAR)`) and authoring a new grammar is an edit to the
registry module that owns it. Every field is still listed, so adding one to a
grammar is a stated decision rather than an oversight.

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
`taint_sink_gate`, …) or a reference to a **named** registry descriptor or
constant (`definition_body`, `case_list`, `object_class`, `body_scope`,
`frame_effect`, `bpf_op`, `event_requires`, `event_requirement_forms`,
`data_collection`, `side_switch_target`, `event_handler_priority`, and
`command_forms`). Rust can observe that such a field is set, but not recover
the expression — the constant's path — that set it.

Seeding records those keys under `draft::UNRENDERABLE_KEY` (`__unrenderable`).
The form warns about them and the renderer emits a `TODO` comment naming each
one. **A field the studio cannot recover is never dropped silently** — the
rendered file says what is missing.

Those fields use `FieldKind::RustExpr`: the value is a string emitted
verbatim, so it carries its own `Some(…)` and type path. The schema's `hint`
shows the exact expression shape expected.

A descriptor that is **plain data** is a different case and does round-trip.
`repeated_args`, `binds_handle`, `byte_array_payload`, `defines_symbol`,
`oo_context_facts`, and a subcommand's `versioned_arg_values` are still edited
as one `RustExpr` field, but seeding renders them back out as **full struct
literals** — every field spelled, never a defaulting constructor like
`RepeatedArgLayout::strided` that would hide the ones it defaults. Drafting a
command that sets one and re-rendering it therefore loses nothing, and the
`Surface::Expression` half of `coverage.rs` is what keeps each literal
complete: a new field on `HandleBindingSpec` breaks the destructuring, and a
field the renderer forgets fails the test that looks for it in the emitted
spec.

One unrecoverable expression is not a top-level field. `OptionArity::Hook`
holds a function pointer inside an *option row*, so it gets a `hook fn` text
box in that row rather than an entry under Advanced, and its
`__unrenderable` key is `draft::OPTION_HOOK_KEY` (`options.arity_hook`)
instead of a field name. Both the form's warning list and the renderer's
`TODO` resolve that key against the `options` array, so they name the exact
options still missing a hook — `return`'s `-errorstack` is the registry's live
example — and both clear once every hook holds an expression. Reporting the
whole `options` field as unreadable instead would be wrong (only one option's
arity is) and the note could never clear, because a filled-in check that only
understands string-valued fields never sees the hook arrive.

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

`rust/tcl-spec-studio/tests/render_sweep.rs` renders every command in every browsable dialect and
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

Roles map through the inverse of `tcl_registry::model::role_for_word`, so a stub the
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
| `required_package`, `introduced_version` | `package provide` |

`ProcArgTrait::DynamicNameLocal` maps to **no** role: it is callee-local, so
passing a literal does not consume the caller's variable and marking it
`VarWrite` would be wrong.

Every inferred draft carries `Inferred::notes` — one line of evidence per
guess, surfaced in the UI. **Inference reports its reasoning, never a bare
assertion.**

Procedures are deduplicated by qualified name across files, last definition
winning, matching what the interpreter would end up with.

### Multi-snapshot import: `import_package_versions`

`import_package_versions` (`rust/tcl-spec-studio/src/versions.rs`) is the
multi-release sibling of `import_package` above: given several labelled
`VersionedSnapshot`s of one package, it derives the version ranges the
releases actually witness instead of stamping every command with
whichever version the newest sources declare. It is the shared engine
behind `tcl spec import` and the MCP `spec_import` tool (see [how to
derive version ranges from release
history](../../kcs/kcs-howto-derive-version-ranges-from-releases.md)).

| Rule | What it means |
|---|---|
| Snapshot ordering | Labels are sorted with `tcl_registry::version::compare`, never trusted in caller order; a disagreement is a warning, and a duplicate label is a warning too. |
| Base shape | Each snapshot is drafted independently by the ordinary single-snapshot importer; a command's merged draft is the draft from the **newest** snapshot defining it. |
| `introduced_version` | The first snapshot defining the command — but only written when that appearance is *definitive*: either an earlier snapshot lacks the command, or the caller declared `complete_history` so the earliest snapshot really is the package's first release. Otherwise left unset with a note. |
| `retired_version` | The first snapshot where a previously-present command is gone — an exclusive bound, matching `tcl_registry::lifecycle` exactly. |
| `deprecated_version` | Never derived structurally. The first snapshot whose doc comment says "deprecated" becomes a *suggested* version recorded only in the notes. |
| Option rows | Diffed by name across the snapshots in which the command exists; an option that later disappears keeps its row, carrying its `retired_version`, rather than being dropped. |
| Closed value sets | Diffed by membership. On a subcommand-shaped draft the result lands in `versioned_arg_values`, the draft vocabulary's existing per-value gate; a command-level value has no field yet, so it becomes a structured `version-gate:` note instead (below). |
| Arity changes | **Derived** into `arity_windows` (issue #1627): runs of equal shape across the snapshots become windows, each closed where the next shape arrives — the spelling the loader requires, since an unclosed window never ends and two would overlap. A signature that never changed derives none; the plain `arity` already says it. The note naming both releases and both shapes is kept beside the derived field as its evidence. |
| Role changes | Reported as a note naming both releases and both shapes, never invented — which argument moved is not recoverable from a count. |
| A present → absent → present pattern | Leaves the lifecycle unbounded and raises a warning naming the gap; a range cannot describe a hole. |

`VersionedImportOptions::complete_history` is the one caller-supplied fact
the derivation cannot infer for itself — see `tcl spec import`'s paired
`--complete-history`/`--partial-history` flags, off by default.

**`version-gate:` notes.** A fact the draft model has no field for yet —
today, a command-level closed-value gate — is emitted as a note carrying
the stable `VERSION_GATE_NOTE` prefix (`version-gate:`) so a later pass
can mechanically upgrade it into a field once the registry extension
lands:

```text
version-gate: command=encode arg=0 value=utf-8 introduced=1.2
```

`tcl_cli_support::spec_import` renders every derived range and every
`version-gate:` note into the pack's `#` comment header, so the evidence
travels with the pack rather than only living in the CLI's stderr
summary.

## Trait names come from the registry

`catalogue::trait_keys` renders a spec's traits by asking the registry for
them (`Traits::iter_names`), and `catalogue::trait_bit` resolves a name
against `Trait::ALL`. There is no name↔bit table in the studio to drift out of
step with the registry, and
`catalogue::tests::traits_catalogue_matches_the_registry_flags` asserts the
studio's descriptive catalogue covers exactly the registry's declared flags,
in order.

Each trait bit is derived from an enum discriminant, so two flags cannot
collide on one bit and `trait_keys` needs no deduplication by bit value — a
hand-numbered `1 << N` table can silently give two traits the same bit, and the
studio would then render a spec claiming a trait its author never set.

## Publishing

The page ships to GitHub Pages at `/spec-studio/` alongside the compiler
explorer, the BIG-IP report generator, and the BIG-IP report demo — one Pages
artefact holds all four (see `.github/workflows/github-pages.yml`).

`wasm-opt` is deliberately not run, for the same reason as the explorer and
the report generator: binaryen mis-rebinds `__wbindgen_externrefs` onto the
fixed-size funcref table, breaking `Table.grow` at run time. The build
verifies the externref table is growable instead.
