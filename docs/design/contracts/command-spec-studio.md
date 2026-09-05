# Contract: the command-registry spec studio

The spec studio is a browser front-end over `tcl-registry`: it browses the
live command surface pack by pack, edits every field of a `CommandSpec`, and
renders the result back to a registry `.rs` module or a Tcl dialect stub.
This document describes the contract between its layers and the invariants
that keep it from drifting away from the registry it edits.

Related: [`command-registry.md`](../compiler/command-registry.md) (the field
reference), [`dialect-stubs.md`](dialect-stubs.md) (the stub language),
[`proc-arg-traits.md`](proc-arg-traits.md) (the inference the importer reads).

## Layers

| Layer | Crate / path | Role |
|---|---|---|
| Provenance | `rust/tcl-registry` (`commands/mod.rs`, `registry.rs`) | `SPEC_PACKS`, the table of `commands/<id>/` authoring modules, and `spec_pack_of`, which of them declares a resolved spec. The registry's, not the studio's: the studio reads it through `command_index` and `pack_catalogue`. |
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

## The documentation dock

The form has 137 settings, and each carries a **?** that expands an inline
panel — which pushes the field being edited out from under the cursor to make
room for its own explanation, and says nothing about which settings are read
together. The dock is a third region that documents whatever has focus
without moving anything. `web/src/docsDock.ts` decides what it shows and how
a view is named in the URL, as pure functions of the wire schema with no
DOM; `studio.ts` paints the decision, so the two cannot drift apart.

**A second surface, never a second copy.** The dock and the inline panels
render the same `help` and `example` entries through the same
`helpParagraphs` and `annotatedExample`, so the text exists once. The inline
panels stay: on a narrow viewport they are still the primary surface, and
replacing them would swap one help path for another rather than add one.

| Viewport | Shape | Open | Collapsed |
|---|---|---|---|
| ≥ 75rem | a third column of the browser/workbench grid, sticky, scrolling inside itself | — | header only |
| < 75rem | a bar fixed to the bottom of the viewport | 15rem | 3rem |
| ≤ 34rem | one summary line, collapsed until tapped, with the **Docs** label dropped | 50vh | 2.9rem |

The sidebar overlays nothing. The two bar shapes can cover the control being
edited, so `--dock-h` is reserved as padding at the end of the page, every
field carries it as `scroll-margin-bottom`, and a control focused underneath
the bar is scrolled clear. Open or collapsed rides the IndexedDB session
record beside the browser's expand state; a record written before the dock
existed restores as the viewport's default, not as a failure. The body is an
`aria-live` region, so a screen reader hears a re-target it cannot see.

### Focus, not hover

The dock re-targets on `focusin`, `click`, and `change` in the form and on
`focusin` and `click` in the browser — never on the pointer passing over,
because a cursor crossing 137 settings must not churn the panel. There are
five subjects, one per thing an author can be touching:

| Subject | Recognised by | Decision |
|---|---|---|
| A setting | `data-field-key` on its row, top-level or inside an option row | A property inside a composite row is documented from the `nestedFields` table, named by the row it is a property of, without clusters. |
| A group heading | `data-group` on its `<details>` | Asked about first: a subcommand's groups sit *inside* the field holding the subcommand, so testing the field first would never reach them. |
| A catalogue picker | `data-catalogue` on the control | Opening a `<select>` asks about the vocabulary, not about whatever was already chosen in it. |
| One picker value | `data-variant` on a toggle, or a `<select>`'s value on `change` | A dropdown's value counts once it is committed. |
| A pack section | `data-pack` on its `<details>` in the browser | Label, repository path, blurb, and command count. |

A subject the schema cannot describe leaves the dock showing what it had,
which is also what "nothing focused" does: it is seeded with the first group
at boot and never blanks after that. Only a focused *setting* is written into
the address bar; a group or a catalogue is context around it.

### Related settings are links

Each `related` cluster renders as its name, its `why`, and one chip per
member. The member being shown is a dashed chip rather than a link, and a
key this registry's schema lacks is inert. Following a chip switches to the
editor tab if needed, opens every `<details>` between the form and the
target, scrolls to it, outlines it for 1.2 s, focuses its first control, and
re-targets the dock to it — so a chain of chips reads as navigation. Under
`prefers-reduced-motion` the outline is held static and the scroll is not
animated, rather than the arrival going unmarked.

A link needs exactly one place to land, so `fieldAnchorId(key)` —
`field-<key>` — is set only on the top-level command form; a subcommand's
nested form builds the same keys again inside its row and gets none. The
properties edited inside an option row or a Tk geometry descriptor
(`variable_scope`, `script_timing`, `method_prefix_matching`, …) are cluster
members with no row of their own, so `NestedFieldSchema::field` names the
top-level key each is edited *under* — `options`, `object_class`,
`tk_geometry` — and following the chip reveals that row while the dock goes
on to document the property. The row is the true answer rather than a
consolation: it is where the author edits the setting. The owning Rust type
is not, which is why the schema had to learn the field as a separate fact,
and a test holds every one of them to a live `COMMAND_FIELDS` or
`SUBCOMMAND_FIELDS` key.

`ownerField` is the single answer to "where is this edited", so a key the
schema can place nowhere is drawn inert instead — a link that goes nowhere is
worse than no link. The status line's "not a setting this command has" is
left for the case it really names: a row this command's form does not build.

### Every view has a URL

Two shapes, because there are two things worth sending someone:
`#/c/<dialect>/<command>`, plus the setting key when one has focus, and
`#/ref/<catalogue>[/<variant>]`. That setting key is always a top-level one:
a nested property is written as the row it is edited in, so every fragment
the studio writes names something the form can restore. On load the fragment is applied *after* the
resumed session, so a link someone was sent names the view they were sent to;
a dialect it names is switched through the picker's own `change` event, so
the language server, the pack's collisions, and the browser follow as they
always do.

**One history, not two.** The visit stack stays the record of which commands
were opened and in what order; every visit is mirrored as one session-history
entry tagged `{index, visit}`, and the in-page ◀ ▶ (and `Alt+←`/`Alt+→`) no
longer open anything themselves. They compute the delta from the entry they
are on to the entry carrying the visit they want and call `history.go`, so
`popstate` does the opening and the page's buttons and the browser's Back are
one act rather than two stacks racing. `index` is a *position* in this
page's run of session history, not a serial: a push from a back position
takes the place the browser just vacated, and a counter would climb on and
make `go()` overshoot. A Reference entry is written so it is linkable but is
not a visit, so the buttons step over it; two visits the address bar cannot
tell apart — one name opened from the pack and from the registry — have a
zero delta and are opened directly, because `go(0)` reloads the page.

**An entry restores the view it was written for.** A Reference entry is
linkable without being a visit, and it inherits the `visit` tag of whichever
command was open when it was written — so the tag alone cannot say what to
re-open. `restoreFor` reads the fragment first: a `#/ref/…` entry restores
the Reference view, and a tagged visit is opened only where the fragment
agrees it names that command. Opening the visit there rather than the route
is what carries *how* the command was reached — from the pack, or from the
registry list — through a Back, which the fragment does not record. Take the
tag first instead and Back off a command opened after a Reference entry
re-opens the command that Reference entry was opened *from*, with the address
bar still reading `#/ref/…`.

**Focus moves replace.** `historyMode` pushes for another command, another
dialect, or another kind of view, and replaces for anything within the
command already open — a focused setting, a return to the editor tab from
one of the un-routed panes, a route re-written over itself when a Back is
restored. Back therefore steps between *commands*, not between every field
the pointer touched. The entry the page was loaded on is adopted the same
way: the first command opened in a session is where the session starts, not
a second place it went. The other tabs write nothing; they are panes over the
command already in the bar, and an entry for each would fill Back with places
nothing came from.

**`file://` keeps the buttons.** `pushState` throws on a page opened from
disk — an opaque origin cannot own a URL — and this page is built to be saved
and opened that way. The first throw switches routing off for the session:
`navigate` opens the visit directly and ◀ ▶ carry on from the visit stack
alone. Deep links are lost there and the Back button is not, which is what
the "opening `index.html` straight off disk still works" promise needs.

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
line in `draft`.** No UI, serialiser, or renderer change is needed. The help
gates below then ask for three more entries — long-form help, a worked
example, and a place among the related-settings clusters — each failing by
name until it is written.

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

#### Every example shows the setting it sits under

`examples.rs` gives each field, group heading, catalogue, and vocabulary value
a compact Tcl snippet plus the spans to point at. The browser draws a bracket
and a numbered arrow beneath each span, and the **?** panel and the Reference
row read the same JSON, so one surface cannot explain a setting differently
from the other.

The stronger property is that a field's example is an example *of that
field*. `field_template` is an exhaustive keyed table split across
`examples/fields_core.rs` and `examples/fields_behaviour.rs`, and
`every_group_and_field_has_a_valid_example` fails by name for a key with no
entry. There is deliberately no group-level fallback: inheriting the group's
snippet is how most of the form once shipped illustrating something other
than itself — every taint sink drew the same line, and the whole Availability
group showed one `package require`. A group heading keeps its own snippet,
because a group's **?** is about the group.

Each snippet uses a shipped command that really declares the field, so the
arrows point at a consequence the analyser draws today: an output sink is
`HTTP::respond`, a log sink is `log local0.`, a network sink is `socket`.

Dropdown values are the same idea one level down. A vocabulary the registry
owns — `Trait`, `TaintColourAtom`, `SideEffectTarget`, `ArgRole`,
`AppendedArity`, `TclType`, `StorageType`, `ConnectionSide`,
`ByteArrayEffect`, `PatternType`, `FormatType` — carries a
`DocumentationExample` per variant, and `variant_example` serialises it. The
remaining pickers (`bodyKind`, `scriptTiming`, the hook ids, `dialects`, …)
still introduce a value with their catalogue's snippet. That is the boundary
today, stated rather than papered over; moving a vocabulary across it is the
registry change described next.

#### Where a vocabulary's example lives

Issue #1693 asked whether a trait's worked example should stay beside its
declaration or move out. Three arrangements were compared:

| Arrangement | What it buys | What it costs |
|---|---|---|
| Co-located: a second exhaustive `match` beside the declaration (`declare_trait_examples!`, `ArgRole::example`, …) | A variant cannot compile without one; the studio and the explorer serialise the same value, so neither grows a parallel table | A registry module carries prose and Tcl, and a large table sits in a compiler crate |
| A typed, registry-keyed catalogue elsewhere | Examples get their own file and their own reviewers | The key is a string the exhaustive `match` no longer checks — exactly the drift the `match` exists to prevent — and every consumer needs a projection test to prove nothing was omitted |
| Generated from doctests or executable fixtures | The program is proven to run | An example's value is the *annotation* — which span carries the fact, in what causal order — which a doctest cannot express; running the snippet proves the wrong thing |

Co-location stays. The second `match` already gives examples the separate
review lifecycle the split was meant to buy — a change to an example is a
diff to one arm, next to the semantics it illustrates — and ownership stays
in the registry, which is the rule everywhere else on this page. The cost is
the first row's, and it is accepted.

#### Arrow order is checked, not trusted

Arrows are numbered by their position in the annotation array and drawn as
numbered steps, so their order is a claim about *when things happen* (issue
#1714). `causal_order_errors` in `examples.rs` checks that claim against the
source with three rules:

1. **Numbering runs forwards through the program.** An arrow on an earlier
   line may not be numbered after one on a later line; that tells the reader
   the consequence before the cause.
2. **A substitution is numbered before the word that consumes it.** In
   `puts [gets stdin]` the `[gets stdin]` is step 1. Left-to-right is
   deliberately *not* the rule — `error` before `catch` on one line is
   right, and only containment can say so. Only a `[…]` or `$…` needle
   counts: `%b` inside a braced format string is contained but not
   substituted, and Tcl's own rule is the one followed.
3. **Two arrows on a line may not start at the same column.** The browser
   finds a needle with `indexOf` and brackets its first occurrence, so
   `$item` on a line holding `$items` stacks two brackets on one token and
   at least one label describes something the reader is not shown. This rule
   found two real defects on landing: `LOOP_LIST_HEADER` had exactly that
   `$item`, and `FRAME_HASH_BUILTIN` had `set local` nested in
   `set local value`.

Spans that merely overlap or sit side by side are left alone. Their order is
the author's knowledge of the flow, and no rule here can recover it.

#### Related settings ride the schema too

A `CommandSpec` field is rarely a standalone switch: `arity` is read against
`arity_windows`, and setting `pure` while `side_effects` says otherwise is a
contradiction an author should see rather than meet in a failing gate.
`relations.rs` names twenty-five clusters of settings that are read together
(Taint sinks, Effects and purity, Bodies and frames, …), each with one
sentence saying why it hangs together, and `STANDALONE` names the one field
that belongs to none — `name` — with its reason.

Clusters, not pairs. A pair table states both directions and gets one of
them wrong the first time a member is added; membership of a named group is
symmetric by construction, and the group's name is the heading the UI wants
anyway. A field may sit in several clusters. A field in no cluster and not
declared standalone fails `every_field_is_clustered_or_declared_standalone`,
so a new setting is filed by a decision rather than by omission; a cluster
naming a key that is not a schema field fails too, because a link that goes
nowhere is worse than no link.

`FieldSchema::to_json` carries the result as `related` on every field, so
the front-end can link `pure` to `side_effects` without knowing either name.
The documentation dock draws them: each cluster is its name, its sentence,
and one chip per member, and following a chip is the navigation described
under [related settings are links](#related-settings-are-links). A studio
wasm built before the key existed sends no `related`, and the dock then ends
after the example rather than failing to render.

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

## The browser is a stack of packs

A dialect's command surface is not one list. It is `commands/tcl/` plus
whatever layers on it, and that is the shape a spec author works in: the
`.rs` renderer emits a path into one of those directories, and "which pack
does this belong in" is the first question an author has, not the last. So
the registry names **provenance**. `SPEC_PACKS`
(`rust/tcl-registry/src/commands/mod.rs`) is the table of the thirteen
`commands/<id>/` authoring modules, each with the label and blurb a browser
uses for it, and `spec_pack_of` (`registry.rs`) says which of them declares a
spec.

**Provenance is not availability.** `SpecSurface` already says where a
command is *reachable from*; a pack says where its spec is *written down*.
The two answer different questions and legitimately disagree — `open` is
surfaced by core Tcl and by iRules but authored once, in `tcl`; `wm` is
surfaced by the `Tk` package and authored in `tk`. A browser grouped by
surface would send the author to a directory that does not hold the file.

**Keyed by spec identity, not by name.** Seventeen names are declared in two
or three packs — `close` in `tcl`, `expect` and `irules`; `send` in `tk`,
`expect` and `irules` — and each dialect registers exactly one of them. A
by-name table could only pick a fixed winner, and would file the iRules
`close` under `tcl` while browsing iRules. `spec_pack_of` therefore takes the
very `&'static CommandSpec` the registry handed back, indexed by address over
the same leaked `shared_group!` slices the registry inserts, so it cannot
disagree with the registry. `spec_packs_of(name)` is the by-name question
asked on purpose: it backs the "also declared in" note, where the question
really is about the name.

Two things are deliberately not rows in `SPEC_PACKS`:

| Absent | Why |
|---|---|
| `tmsh` | The tmsh shell's specs are a filtered view of the same `commands/iapps/` sources, so their provenance is `iapps` — the directory an author would open. A consumer that wants the tmsh *surface* is asking `SpecSurface`'s question, and asks it there. |
| The EDA vendor libraries | They ship as bundled `.tclspec` loadables under `specs/` and reach a registry through `tcl_spectcl::bundled`, so their provenance is the pack *file*, which the loader reports. `spec_pack_of` answers `None` for any pack-loaded spec — a bundled library, a workspace pack, the document under the author's cursor — rather than guessing. |

`command_index` carries `pack` and `also_in` per command, and
`pack_catalogue(dialect)` lists only the packs that reach the dialect, with a
command count each, so the Tcl 8.4 picker never offers an empty **F5 iRules**
heading. Both come through the wasm facade.

### What the browser shows

`web/src/packs.ts` holds the decisions — grouping, filtering, what the
headers and the count line say — as pure functions of what the wasm module
reports; `studio.ts` paints them.

| Behaviour | Rule |
|---|---|
| Sections | One collapsible section per pack, in catalogue order. The pack under edit is the first section, not a differently-shaped panel beside the list: it is a pack that happens to be editable. Behind each shipped section's **?**: the blurb and the repository path. |
| Open state | Shut, except the section holding the open command; a filter opens every section it leaves something in; a dialect with one pack has no navigation to do. A section the row cap left empty stays shut — open and empty reads as a bug, not as "narrow the filter". |
| Remembered | A person's expand and collapse of the shipped sections rides the IndexedDB session record, across dialect switches and reloads. It is recorded from the summary's click, not the `toggle` event, so a section the studio opened for them is not mistaken for a preference. |
| Filtering | Only packs with a match are shown; each header carries `N of M`; the count line says what is being viewed — `187 Tcl 9.0 commands in 4 packs`, `12 of 187 Tcl 9.0 commands, in 3 packs` — instead of a bare number. |
| Chips | A command's pack is a chip wherever the heading is not already saying it — the command palette, the editor's source line — and a name several packs declare says so: `close is also declared in expect, irules.` |
| Unfiled | A command whose pack the catalogue does not describe is filed under a bare heading rather than dropped: an unknown dialect has an empty catalogue, and a browser showing nothing would be worse. |

Gate: `rust/tcl-registry/tests/spec_pack_provenance.rs` proves every command
in every browsable dialect resolves to a pack that is a real `commands/<id>/`
directory, and that the iRules `close` files under `irules` while Tcl 9.0's
files under `tcl`. The studio's own tests prove the catalogue's counts sum to
the index, so no command can fall between sections.

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

## Parity with native specs

Anything a native `.rs` `CommandSpec` (or an attached descriptor) can express,
a `.tclspec` pack must be able to express too, and the studio must round-trip
it. A registry change is **not complete** until four surfaces move together:

1. **Registry** — the field or descriptor on the type, with its coverage
   witness in `rust/tcl-spec-studio/src/coverage.rs` (an exhaustive
   destructuring plus a `Field` entry, so the build breaks until the studio
   knows about it).
2. **Loader** — a documented spelling in `rust/tcl-spectcl/src/loader.rs`,
   recorded in the frozen-syntax memo
   (`docs/design/spec-dsl-examples/README.md`) and its coverage matrix.
3. **Renderer** — `render_spectcl.rs` emits the spelling (non-default values
   only), and the `spectcl_roundtrip` gate proves draft → DSL → loader →
   draft loses nothing.
4. **Studio form** — `schema.rs` / `draft.rs` / `help.rs` surface it, with
   its example in `examples/` and its cluster in `relations.rs`.

The only sanctioned exception is an explicit entry in `render_spectcl.rs`'s
`GAPS` table naming the field, its documented spelling, and why it cannot
round-trip yet (`DraftOpaque` for genuinely opaque function pointers,
`Excluded` for deliberate design exclusions); a `TODO(spectcl)` comment in
rendered output is the visible trace. A field native specs can set but a pack
cannot say, with no `GAPS` entry, is a bug. New DSL words are **additive**:
the `speclib` version word revs (1.0 → 1.1), `VOCABULARY_VERSION` bumps only
on meaning changes, and the loader keeps accepting every older vocabulary.

## Publishing

The page ships to GitHub Pages at `/spec-studio/` alongside the compiler
explorer, the BIG-IP report generator, and the BIG-IP report demo — one Pages
artefact holds all four (see `.github/workflows/github-pages.yml`).

`wasm-opt` is deliberately not run, for the same reason as the explorer and
the report generator: binaryen mis-rebinds `__wbindgen_externrefs` onto the
fixed-size funcref table, breaking `Table.grow` at run time. The build
verifies the externref table is growable instead.
