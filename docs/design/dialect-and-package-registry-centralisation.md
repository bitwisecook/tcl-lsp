# Registration and resolution: the centralisation contract and retirement ledger

> **Status: PROPOSAL — companion to
> [dialect-and-package-registry-redesign.md](dialect-and-package-registry-redesign.md)
> (revision 2).** That document defines the model (core profiles, packages,
> environments, realms). This one is the end-to-end audit of every
> registration and resolution seam in the workspace — front end, compiler,
> analyser, backends, runtimes and VMs, and all tooling — against that
> model: the two centralised systems every consumer moves onto, the
> complete retirement ledger for the mechanisms they replace, the gap
> rulings the audit forced, the `tcl spec upgrade` specification that
> discharges the one sanctioned backwards-compatibility obligation, and
> — §7 — the name-resolution oracle programme that grounds namespaces,
> variables, procs/commands, and packages in the C Tcl test suites, the
> stdlib's executable specifications, tcllib, Tk, and the corpus.
> Sources: the four-lane audit sweep of 2026-08-26 over
> `claude/tcl-dialect-registry-design-lrzbsn` (compiler/analyser,
> runtime/VM/codegen, tooling/AI/CLI, LSP front end) plus the two-lane
> oracle survey of the same date (C tests per domain across all five
> release trees; existing conformance/capture infrastructure). File:line
> references are from that snapshot.

The owner's constraint, restated: **the new centralised system is the only
system.** Old mechanisms are retired entirely — no shims, no wrappers, no
parallel tables kept "for now". The sole backwards-compatibility exception
is SpecTcl: every published 1.x pack keeps loading, and the tooling to
upgrade 1.x sources to the newest vocabulary ships with the format (§6).

## 1. The two centralised systems

### 1.1 One registration pipeline

All command, dialect, package, environment, and variable knowledge enters
through **one loader** and lives in **one generation-owned catalogue**:

```text
sources                       ingestion                 catalogue (per generation)
──────────────────────────    ─────────────────────     ──────────────────────────
SpecTcl dialect blocks     ─┐                           CoreProfiles (family × release × build)
SpecTcl package packs      ─┤   one loader              EnvironmentDefinitions (+ overlays)
  (bundled | user |         ├─► (vocab-classified,  ──► SurfaceDeclarations (provider,
   workspace | studio)     ─┤    trust-stamped,          VersionSet, predicate, provenance)
native core specs           │    generation-owned)       special-variable declarations
  (until AOT-converted)    ─┤                            detection facts, aliases, policies
inline / sidecar stubs     ─┘
                                        │
                     ┌──────────────────┼──────────────────────┐
              per-context registries    │            derived projections
              (environment, overlay,    │     editors, AI catalogues, docs,
               generation)-keyed        │     engine availability gates, fuzz
                                        │     oracles, TMM simulator data
```

Rules:

- **Every source is provenance- and trust-stamped at ingestion** (redesign
  §6.4). Inline/sidecar stubs are not a separate overlay type consulted
  ad hoc: they ingest as `SurfaceDeclaration`s with `Document`/`Workspace`
  provenance (gap ruling R1, §4), so one query path serves all three of
  today's spec sources.
- **Dynamic data is generation-owned** (review B8): loaded packs, stubs,
  and config-declared environments live in `Arc<RegistryGeneration>`;
  nothing dynamic is leaked `&'static`. Compiled built-ins stay true
  statics.
- **Every projection is generated, none hand-maintained**: the editor
  catalogues, AI prompt manifests, engine gates, TMM simulator data, and
  docs tables all derive from the catalogue behind `--check` drift gates.
  The audit found three hand-maintained or orphaned projections that this
  rule retires (ledger rows T13, B10, F12).

**Status: implemented (P1-E).** The registry on the new model landed in
`rust/tcl-registry/src/model/` alongside the old code:
`surface.rs` (`SurfaceDeclaration` + the mechanical
`declarations_for_spec` translation of every compiled spec's
`dialects`/`required_package`/`tcllib_package`/`lifecycle`),
`context.rs` (`ResolvedContext` + `FloorMap` + `KeyedVersions` and the
`ContextQueries` assistance view with `is_available` /
`available_at_targets`), `assembly.rs` (`ContextRegistry` — Arc-owned
per-context registry generations, provider-filtered from the same spec
sources `build_default`/`load_dialect` use, cached by
`(environment identity, keyed-versions hash)`; dynamic pack ingestion is
a documented P2 seam — whose **environment half landed with P2-H**:
`registration.rs` swaps pack-declared environment definitions and
`-extend` contributions into the live registry at the next generation
under the §6.4 trust lattice, and the ingress resolves against it — see
the §6 status note), and `binding.rs` (`BindingKnowledge`,
`PackageStateMap`, and the `PackageTransition` family as a parallel type
pending P1a realm integration). **P5 added `tcllib.rs`** — the
200-row per-module identity census (`package require` name, version
trains, Tcl-core floor, source evidence) read out of `tmp/tcllib-2.0`,
which supplies the applicability set of every tcllib module's package
declaration and the per-module Tcl floor `commands::tcllib` gates on.
The equivalence sweeps pass with **zero
divergences** (the deliberate-divergence allowlist is empty): per-spec
visibility agrees with `ProfileQueries::is_available` for 3,647 compiled
specs × 18 catalogue profiles (65,646 checks), and per-environment
visible command-name sets and per-name resolution answers reproduce
`registry_for_profile` + `best_visible` exactly (38,713 resolved names
across the 18 environments).

### 1.2 One resolution stack

Five resolution questions, each with exactly one owner:

| # | Question | Single owner | What it replaces |
|---|---|---|---|
| R-a | user-written name → environment | `Environment::resolve(name)` — canonical names + aliases + editor language ids, one function for every ingress (settings, directives, language ids, CLI flags, MCP enums, pack rows, persisted studio sessions) | **six** divergent validators found in the audit: `available_dialects()` membership, `is_known_dialect_name`, the directive's `KNOWN_DIALECTS` match, `resolve_known`, `special_vars::resolve_dialect`, and raw `DialectSet::parse` in `tcl-lsp-db`/completion |
| R-b | document bytes → resolved context | the §5.1 detection chain over environment detection facts, plus overlays and targets; output `(environment, generation, overlay hash, targets, primary)` | `TCL_SOURCE_EXTENSIONS`, per-editor extension tables, content-signature ladders, `LanguageDialect::{Profile,Set}` |
| R-c | command name at a call site → binding | candidate ordering stays `tcl_syntax::naming::command_resolution_candidates` (already the single home, conformance-gated against tclsh); the **`exists` oracle becomes one function**: realm `BindingKnowledge` (`Absent`/`Must`/`May`/`Unknown`) produced by the unified transition state (§4.2 of the redesign). **Landed (P1a)**: `Analyser::command_existence_oracle` / `command_binding_knowledge` answer the four values per program point (W123 is the `Absent` verdict; the document-wide package/provider widenings are typed oracle state); the registry tier is the one context-filtered set (`builtin_command_names`, C5) shared by settlement, const-dispatch, W113 and W123; head-identity consumers read the realm state's `knowledge_at` (`tcl_compiler::realm`, C4). The unfiltered `registry.get` residue at ~40 compiler and ~10 LSP sites remains the open tail of this row | every consumer-local `exists` oracle: `KnownPredicateCtx`'s unfiltered `command_names()`, `head_identity.rs`, the analyser's ad-hoc alias/rename/delete tables, the LSP's four mixed lookup APIs, unfiltered `registry.get` at ~40 compiler and ~10 LSP sites |
| R-d | `package require` → train, floor, targets | one axis-typed `VersionSet` algebra (differentially tested against `package vsatisfies`) plus **one** floor engine with the assistance/semantic split; `PackageResolver` remains the pkgIndex/tclIndex ingest and joins the floor engine as a source | the `DocumentFloor` ⟷ `package_version_floor` duplication (different source rules today: no `PackAmbient` in one, unconditional-require gating of hosted pins in the other), the three coexisting version comparators (`tcl_dialect`, `tcl_registry::version`, `tcl_pkg::version`) |
| R-e | resolved binding → semantic hook | hook selection requires binding proof (invariant I4); the WASM backend's `ProofStatus` discipline (`Unavailable ≠ permission`; only `NotRequired \| Satisfied` specialise) generalises to lowering, inline-codegen, analyser-hook, and const-fold selection. **Landed (P1a)** at the model's three selection primitives: with a context carried, the head must resolve to a spec's declaration under the document's environment (`ResolvedContext::resolve_spec` — environment-level `Must`) before any hook, type fact, or hint is selected; `Absent` ⇒ no selection; no context ⇒ `NotRequired` (unit harnesses, shape-only widening queries, each documented at its site). Analyser hooks, lowering hooks, and type-infer's spec-fact specialisation thread real contexts; the codegen `resolve_call(…, own_availability_mask)` sites already carry their environment mask | the `resolve_call(…, DialectSet::empty()) → get()` dialect-blind bypass in the analyser-hook path, lowering hooks, and lsp-db; `side_effects.rs`'s hand-rolled fourth selection rule |

The split the redesign §5.2 mandates is enforced by type: **assistance
queries** (completion, hover, annotations, W120) take
`(environment, floors)`; **semantic queries** (diagnostics that assert,
code actions that edit, taint, lowering, codegen) take realm
`BindingKnowledge` at a program point. A semantic consumer cannot call the
assistance API — different names, different types (invariant I3).

## 2. What the audit established, per stage

Condensed to the load-bearing findings; the ledger in §3 carries the
retirement rows.

### 2.1 Front end (LSP server, lsp-core, lsp-db)

- No single resolver exists: providers mix four lookup APIs, and the
  weakest (`registry.get`, dialect-agnostic, last-registered-wins) feeds
  semantic decisions — definition/rename's `has_builtin`, the server's
  W123-suppression oracle, signature help, inlay hints.
- The workspace index is **environment-blind** (no dialect/environment
  field on any symbol), so cross-file arity, W123 suppression, and
  cross-document definition let an `f5-irules` proc satisfy a `tcl9.0`
  call. It already models imports/aliases/renames with force and ordering
  semantics — that vocabulary migrates into the realm layer rather than
  being rebuilt, gaining realm identity and `Must`/`May`.
- A latent availability split: `ProfileQueries::package_available`
  (closed-world gate) reads only compiled-catalogue pins while
  `is_ambient_package` reads the pack union — two answers to "is this
  package ambient here". Fixed by construction under placements.
- `registry_with_overlay` **fails open**: a pack-overlay cache miss
  silently serves the un-overlaid registry. Under generation-keyed
  environments this becomes a correctness hazard; misses must rebuild or
  error, never degrade silently.
- Promised-but-unimplemented surfaces to build (not re-key): package
  name/version completion, `tcl-lsp.listKnownPackages` (hardcoded empty
  today), the "known, unspecced → spec-author" flow.
  `PackageResolver::package_names()` exists and is entirely unconsumed.

### 2.2 Compiler and analyser

- Candidate ordering is genuinely centralised and conformance-gated; the
  drift gate, however, is a 240-byte grep window that cannot see a second
  availability rule, a second binding table, or a hardcoded name set —
  all three exist (ledger C7, C2, C11).
- Hook selection is dialect-blind by construction
  (`DialectSet::empty()` → `get()`), in the analyser-hook path, lowering
  hooks, and lsp-db's per-proc compile. *(P1a: retired — the selection
  primitives require the environment binding proof; ledger C3/B8.)*
- Three "known command" oracles disagree: settlement uses the unfiltered
  registry name set, W123 uses profile-filtered resolution,
  W002's known-anywhere uses a hardcoded 11-pack list — so settlement
  believes in commands W123 does not. *(P1a collapsed the first two onto
  the one context-filtered set — ledger C5; the known-anywhere tiers are
  C10's row.)*
- The LSP settlement path holds **zero** references to
  `state_transition.rs`; it uses parallel ad-hoc tables
  (`command_aliases`, `renamed_commands`, `deleted_commands`, offsets)
  plus `head_identity.rs` — a second, weaker, top-level-only binding
  table with 20+ consumers. *(P1a retired `head_identity.rs` onto the
  realm command-binding state — ledger C4; the ad-hoc tables now feed
  the one oracle but their `state_transition.rs` re-homing is C6's open
  tail.)*
- `command_binding.rs` (flow-sensitive `BindingKind` lattice with
  `trusts()`) is the in-tree prefiguration of `BindingKnowledge`; it
  lacks the realm and package axes. `RealmState` ≈ `command_binding` ×
  `state_transition::InterpreterTransition` × a **new package transition
  family** (`state_transition.rs` currently has no `Package` domain).

### 2.3 Backends (runtime, VM, codegen, BPF)

- Both engines gate availability through near-identical private functions
  and register commands via hand-ordered lists; the offline parity gate
  is a **runtime-only textual regex scan** — no VM gate exists, which is
  how the VM's hand-typed mathfunc list silently missed the entire
  TIP 745 batch while the runtime (derived table) carries it.
- `package provide Tcl` returns hardcoded `"9.0.4"` in both engines
  regardless of the pinned release; the VM additionally no-ops
  `package ifneeded|forget|unknown|prefer`, so the two engines disagree
  on every package-loading script the fuzzer pairs them on.
- Safe-interp hiding is a byte-identical duplicated 12-name list in both
  engines that has already drifted from `Traits::SAFE_INTERP_HIDDEN`'s
  14 specs (both miss `unload`, `tcl::zipfs`). The analyser honours the
  trait; the engines do not.
- The runtime's per-interp `PackageState` with a real
  `ifneeded` → `unknown` → retry loop is the behavioural oracle for the
  new package transition family; its expr path, however, parses with
  `dialect = None` and both engines call
  `RuntimeExprSurface::for_tcl_version` (release-keyed, family-blind)
  while `for_profile` sits unused — the §3.1 `ExprGrammar` hole at
  runtime.
- The engine trait (`tcl-engine-api`) has no surface contract — a bare
  `restrict_commands(&[&str])` — and `TclVmEngine` never pins a profile,
  so **hook bodies execute with availability gating disabled**. The
  33-name `SANDBOX_COMMANDS` closed world lives outside the registry.
- The WASM backend's `BackendRegistry`/`ProofStatus` and
  `try_bytecoded`'s trust gate already implement I4's discipline; BPF is
  the cleanest fully registry-derived backend (one prefix-heuristic
  residue, self-documented as a missing Thread pack).

### 2.4 Tooling, AI, editors

- The CLI's dialect ingress is nearly centralised already
  (`resolve_dialect` has the right shape); the residue is the `tk`
  special-casing, the explorer's second resolution with a silent
  plain-Tcl fallback, the debugger's hardcoded `"tcl9.0"`, f5-cli's
  unvalidated bare-`String` `--dialect`, and hardcoded `tcl8.6` defaults.
- `tcl spec` has exactly two verbs (`import`, `upgrade`); `spec check`
  exists only as the MCP tool, and `spec build --emit rust` — now ruled
  in by Q1 — does not exist anywhere (the only Rust renderer is
  WASM-only and per-command). Both verbs are P2 deliverables (§4 R7).
- Spec Studio's web client treats **the dialect as the LSP language id**
  (document close/reopen on change) — structurally incompatible with the
  fixed contributed-identity ruling (B7); it must switch to a generic
  contributed identity with the environment tracked out of band.
- The `spec-author` skill instructs authors to declare `speclib … 1.1` —
  one vocabulary behind the loader, producing wrong instructions today,
  refreshed to 2.0 in P2.
- The TMM simulator's `_registry_data.tcl` (2,086 lines) is orphaned —
  its generator was retired with Python — and encodes a frozen
  "tcl8.4 minus f5-irules" `DialectSet` subtraction; it joins the two
  already-generated simulator assets under `gen-irule-test-data`.
- `ai/prompts/manifest.json` keys Tk guidance to the five Tcl releases
  because `tk` has no profile; once `tk` is an environment the manifest
  names it directly. The VS Code prompt loader must alias-resolve before
  its `dialects[]` includes-check.

## 3. The retirement ledger

Every entry: mechanism → replacement → phase (redesign §8) → the gate
that proves the retirement. **F** front end, **C** compiler/analyser,
**B** backends, **T** tooling. No entry may be wrapped or kept alongside
its replacement.

**Status: P1-G (the deletion phase) is done.** After the P1-F waves left
every production ingress on the seam, the remaining test-fixture call
sites (~1,600 `by_name`-family and ~490 registry-door mentions across
~220 files: `rust/tcl-vm/tests/*`, `#[cfg(test)]` modules across the
workspace, the lsp-core/compiler/registry fixtures) were ported to
`tcl_registry::model::ingress` — `resolve_environment(name)
.analyser_profile()` is the exact `by_name` twin, `static_context_for
(name).commands()` the exact `registry_for_dialect` store — and the old
APIs were then removed:

- **Deleted outright** (zero references, compiler-enforced):
  `DialectProfile::by_name`, `by_opt_name`, `resolve_known`,
  `availability_for_name`; `tcl_registry::registry_for_dialect` and
  `registry_handle_for_dialect`. The three lower-crate compatibility
  name boundaries that used them (`tcl_lexer::LexerConfig::for_dialect`
  / `for_file_dialect` / `tokenise_expr*`, `tcl_syntax`'s
  `parse_expr*`/`diagnose` name forms) survive — they are ledger
  F8/C12's to retire, with ~200 callers — and now inline the
  find-then-plain-sink resolution at the documented boundary.
- **Narrowed to `pub(crate)`** (the compile-level half of the gate):
  `tcl_registry::cache` (`registry_for_profile`,
  `registry_handle_for_profile`, `registry_for_profile_if_built`) — the
  cache stays the command-store owner behind the model's
  `command_store` seam until the store re-homes with C1 — and the
  `ProfileQueries` trait, which keeps only its crate-internal
  consumers (`command_snapshot`, the option-descriptor layout) plus a
  `cfg(test)` `LegacyProfileOracle` carrying the six sweep-only query
  rules verbatim, so the P1-F parity sweeps keep pinning
  `ResolvedContext` against the exact old rules after the deletion.
- **Kept, with the reason recorded**: `DialectProfile::find` (the one
  catalogue lookup — environment-id-keyed, what the seam and the
  documented per-crate interop twins are built on; it resolves canonical
  ids, never user strings); the named handles `plain_tcl`/`irules`/`tk`
  (interned-statics accessors the seam itself consumes — not name
  validators); `registry_for_profile_with_overlay` (the loader door
  only `tcl-spectcl` writes); `DialectSet::parse` (no name-ingress
  caller left — every remaining use projects an already-resolved
  profile's canonical name into the C1 mask vocabulary, e.g. the unit
  build's `semantic_dialect_set`, and retires with C1's re-type, which
  P1-G deliberately does not chase); `tcl_lsp_core::registry_for_dialect`
  (the crate's own wave-2 seam wrapper, not the retired cache door).
- **The zero-reference gate**: `cargo xtask retired-api-gate` (wired
  into `make xtask-check`) fails on any code-line reintroduction of the
  retired spellings — comment citations exempt, the `tcl-registry`
  crate-internal survivors scoped, escape hatch
  `// retired-api-ok: <reason>` requiring a ledger entry here. The
  gate's own tests prove it fails on a seeded violation of every
  retired family.
- **Old-catalogue correction (redesign §2 rows, measurements §4a)**:
  the falsified `f5-iapps`/`f5-tmsh` rows now carry the `f5-tcl` trunk
  facts — `GRAMMAR_F5_TCL` (R-rules, N-rules, inert `{*}`, 8.4
  numerals; the same value `f5-irules` selects), `TCL84|vendor`
  availability masks, 8.4 version ceiling/signature/runtime/expr bases
  and VM pin, `operators_as_commands: false` (`::tcl::mathop` measured
  absent). The `f5_reclassified_oracle` twin the parity sweeps carried
  is deleted — the sweeps compare the corrected rows directly.

### Front end

**Status: LSP side ported (P1-F wave 2).** `tcl-lsp-core`, `tcl-lsp-db`
and `tcl-lsp-server` now resolve every dialect-name ingress through the
**one shared seam**, `tcl_registry::model::ingress` — the wave-1
implementation moved out of `tcl-compiler`'s `environment_ingress`
(which now delegates to it) into the registry model, the only crate that
can express both halves of a resolved document environment (the
`tcl-dialect` definition *and* its `ContextRegistry` generation).
`tcl-lsp-core`'s `profile_for_dialect` / `optional_profile_for_dialect` /
`registry_for_dialect_profile` are re-expressed on it and joined by
`environment_for_dialect`, `stated_profile_for_dialect`,
`context_for_dialect` (the generation) and `document_context_for_dialect`
(the assistance view); the salsa `registry` / `registry_with_overlay`
doors, the server's `Backend::registry_for_dialect`, the editor
language-id ingress, and both configuration validators go through it too.
Availability, option, subcommand, keyed-range and placement-floor
questions are answered by `ResolvedContext`, not `ProfileQueries` — under
the **document authoring mask**
(`DocumentEnvironment::document_authoring_mask`), which equals the
threaded profile's `availability_mask` for every profile an ingress can
produce and is test-pinned to it, so the `tk` ingress keeps the additive
`TK` bit its environment's own derivation deliberately lacks. Old APIs
remain for other crates; deletion is P1-G.

| # | Retired mechanism | Replacement | Phase |
|---|---|---|---|
| F1 | The four mixed lookup APIs as provider-facing surface (`get`, `get_for_dialect`/`best_visible`, `ProfileQueries::resolve_command`, `DocumentFloor`) | two typed views: assistance `(environment, floors)` and semantic realm `BindingKnowledge`; `get` becomes registry-internal | P1a *(partial: the assistance half is ported — no LSP crate calls `ProfileQueries`, and `DocumentFloor` now reads placements/floors off the resolved context. The realm view now **exists** (the analyser oracle and `tcl_compiler::realm` answer `BindingKnowledge`), and the compiler's semantic selections go through it; `get`/`get_for_dialect` stay provider-facing for the LSP's remaining direct readers — the ~10-site tail R-c enumerates)* |
| F2 | `profile_for_dialect` + `registry_for_dialect_profile` (ruling B's hop) and their pin tests | environment registry ingress | P1 **done** *(both are now thin faces of `resolve_environment`; the `tk` triangle the hop existed to reproduce is gone — `tk` is an environment, its store is the same plain-Tcl `Arc`, and the `TK` fact is `is_tk`)* |
| F3 | `LanguageDialect::{Profile,Set}` (Set exists only for `tk`) | environment handle | P1 **done** *(one arm: the resolved environment's `unit_profile`; the language-id ingress accepts a contributed identity, never a legacy alias — review B7)* |
| F4 | `tk_loaded` computations, `tk_preview`'s `profile_for_dialect("tk")`, `hosts_tk()` consumers, `TK_PACKAGE` substring activation | "provider `Tk` active" placement query | **done (P3)** *(the `tk` environment places `Tk` **ambient** on Tk's own axis, every plain-Tcl environment — the lenient sink included — places it **hosted**, and the three questions are three context queries: `package_active("Tk")` (in this document's world, §5.3 policy), `ambient_package("Tk")` (there with no `package require` — the Tk-checks activation fact and W120's silence), `can_host_package("Tk")` (this environment declares a placement). No consumer reads an environment **name** any more: `DocumentEnvironment::is_tk` is private and has two callers, the profile-identity interop and the seam's own pins. The document mask's `TK` bit is derived from the ambient placement, so `ResolvedContext::with_authoring_mask` and the second leaked document-context value are deleted)* |
| F5 | Semantic-token's process-wide `f5-irules` registry `OnceLock`; `bigip.rs`'s hardcoded dialect strings | environment-keyed, generation-aware handles | P1 **done** *(both resolve through the seam; the `OnceLock` memoises the generation's store rather than a name-keyed cache entry)* |
| F6 | Environment-blind workspace index symbols | realm/environment-keyed index rows feeding the four-tier known-anywhere model | P1a |
| F7 | `registry_with_overlay`'s silent un-overlaid fallback | fail-closed rebuild-or-error on generation miss | P2 *(unchanged by wave 2: the door is now `DocumentEnvironment::context_registry`, which threads the overlay key exactly as `registry_for_profile_if_built` did — including the silent fallback)* |
| F8 | Salsa `dialect: String` inputs and interned keys; `LexerCfgKey`/`ProcBodyKey` two-field truncation | `(environment id, generation, overlay hash, targets)` keys; grammar-id lexer keys | P1 *(partial: every read of a salsa `dialect` string now resolves through the seam; the key types themselves are still `String`-shaped)* |
| F9 | `getEffectiveConfig`'s dialect fields, `listDialects`, `setDialect` validators | environment/targets/realm status surface, `listEnvironments`, `Environment::resolve` | P1 *(partial: all four validators — `folderDialects`, folder `tclLsp.dialect`, `setDialect`, `setSessionDialectOverride` — are one `Environment::resolve`, and `getEffectiveConfig`'s labels are reached from the resolved environment; `listDialects` still enumerates `DialectProfile::all()` because the environment list has different contents and no `short_name`, so it is a payload change, not a refactor)* |
| F10 | W120 fix-from-whole-file, the package-require code action's name-matching gate | assistance-labelled diagnostics; edits gated on `Must`/`May` declarations and the `PackageResolver` | P1a |
| F11 | `TclVersion::from_dialect` in W123 refinement | target `VersionSet` evaluation with honest `Unknown` on guard straddles | P1b |
| F12 | Hand-written Sublime `_SYNTAX_DIALECT_MAP` (missing `tcl8.6`/`tcl9.1` rows today) | generated projection + drift gate | P1 |

### Compiler / analyser

**Status: compiler side ported (P1-F wave 1).** `tcl-compiler` now
resolves every dialect-name ingress through
`EnvironmentRegistry::resolve` (one `environment_ingress` module: the
analyser's `analyse*` entries, the per-item path, incremental
re-segmentation, and `CompilationUnit::build_for_dialect`), obtains its
registries as per-environment `ContextRegistry` generations
(`registry_for_environment_if_built`, pack-overlay key threaded exactly
as `registry_for_profile_if_built` did — each generation's command store
is the old cache's `(profile, overlay)` `Arc`, shared by handle), and
answers command/subcommand/option availability, keyed version ranges,
and profile-pin/pack-ambient floors through `ResolvedContext`'s query
surface, whose derived facts (authoring mask, ceiling, operator-head
rule, placements, floors) are sweep-pinned to the old profile's answers
for every catalogue environment. `availability_for_name`'s `TK` union
became the resolved-environment fact, and **P3 finished the move**: it is
now `ResolvedContext::ambient_package("Tk")` — a placement query on the
walk's own generation, not the environment's name — carried on the
analyser as `tk_ambient` (was `tk_dialect`), so any environment that
declares Tk ambient activates the Tk checks and silences W120 without
being spelled `tk`. The hook-path
`DialectSet::empty()` bypasses route through the model's
`resolve_call_in_context` / `resolve_invocation_in_context` primitives
(context-carrying, selection unchanged — the `// P1a:` seam naming
invariant I4), and `side_effects.rs`'s hand-rolled selection retired
onto `side_effect_hints_in_context` (C7) — measured **not** collapsible
onto single-winner selection without behaviour change (`classvariable`,
`next` under `bpf`), so the primitive keeps the availability-filtered
newest-first-with-hints walk until P1a's binding proof. The old APIs
this left for other crates are now deleted — see the P1-G status at the
head of this section.

**Status: realm state and binding proof landed (P1a).** The three model
selection primitives read their context (invariant I4, R-e): a carried
context is a proof obligation — the head must resolve to a spec's
declaration under the document's environment before an analyser hook,
lowering hook, type-infer spec fact, or side-effect hint is selected;
`Absent` ⇒ no selection (a version-gated command outside the release
window, an iRules-disabled builtin, take the generic/conservative path);
no context ⇒ `NotRequired`, and every deliberately context-less reader
is a documented widening query (taint sink shapes, the inline-uplevel
frame-reach decline, the lowering barrier gate, the shimmer mutation
scan, the command-prefix reference walker). The analyser's `exists`
bookkeeping unified onto the model's `BindingKnowledge` (R-c):
`command_existence_oracle`/`command_binding_knowledge` answer
`Absent`/`Must`/`May`/`Unknown` per program point with the package /
dynamic-provider / dynamic-`unknown` widenings as typed oracle state,
W123 fires exactly on `Absent`, the §4b iRules interpreter-present
extension and the closed-world policy (B12) are policy over the one
oracle, and `head_identity.rs` is deleted wholesale onto the realm
command-binding state (`tcl_compiler::realm`, spec-keyed facts with a
`BindingKnowledge` view). C7's decision evidence is now a test
(`c7_hint_walk_counterexamples`): proved-single-winner hint selection
diverges from the shipped walk at 9 measured points (`next` under
`bpf`; `exit`/`send`/`close` under `expect`; `option` and four of its
subcommands under `spectcl`), so the walk stays inside the primitive —
now behind the I4 head proof, which the catalogue-wide sweep measured
as widening **zero** hint selections.

| # | Retired mechanism | Replacement | Phase |
|---|---|---|---|
| C1 | `DialectSet` + unions, `availability_for_name`, `TK_PROFILE`/`tk()` synthesis, `DIALECT_BITS`/`BIT_ONLY_LABELS` | `VersionSet` declarations + environments; optional internal `FamilySet` fast path | P1 *(partial: `availability_for_name` is deleted (P1-G) and nothing synthesises `TK_PROFILE` from names. P1a's survey of the residue: the `DialectSet`-typed plumbing that remains all funnels through the semantic-facts bundle — `SemanticAnalysisBundle`'s `dialect: DialectSet` field, its `unavailable(DialectSet::empty())` constructors (compilation-unit lattice declines, branch-folding tests), and the `semantic_dialect_set` name→bit projection `tcl-lsp-db`'s per-item build shares — plus the executable-IR builder (`build_linear_executable_ir(registry, dialect, …)`) and world-state SSA it keys, and `bpf-tcl-ir`'s `semantic_bridge` consuming the same bundle. None of that is unblocked by the P1a oracle/context typing alone: the bundle's dialect key gates on re-keying **executable-IR invocation resolution** onto the context primitives (a `ResolvedContext`-keyed bundle), which drags the WASM/BPF semantic bridges with it — a coordinated re-type left for the C1 completion wave. The selection primitives' own `DialectSet::empty()`/authoring-mask arguments are now internal to the I4 proof and disappear with the same re-type)* |
| C2 | The six divergent dialect-name validators (incl. `special_vars::resolve_dialect`, raw `DialectSet::parse`) | `Environment::resolve` | P1 **done (P1-G)** *(`by_name`, `by_opt_name`, `resolve_known` and `availability_for_name` are deleted from the tree — test fixtures ported to the seam, the lower-crate compat name boundaries inline the find-then-plain-sink resolution (F8/C12 retire the boundaries themselves) — and `DialectSet::parse` has no name-ingress caller left (its remaining uses are C1 projections of canonical profile names). `special_vars::resolve_dialect` is deleted too (its production callers already threaded profiles through `dialect_set_for_profile`); the `retired-api-gate` holds the set at zero)* |
| C3 | `resolve_call`/`resolve_legacy_call_selection`'s `dialect.is_empty() → get()` bypass and its callers (analyser hooks, lowering hooks, lsp-db) | binding-proof-gated `InvocationSpecId` selection (I4) | **done (P1a)** *(the primitives enforce the proof: with a context, the head resolves through `ResolvedContext::resolve_spec` and sub/form gating runs under the authoring mask; the analyser-hook, lowering-hook, and type-infer callers thread real contexts, `lsp-db` holds no bypass of its own, and each remaining `invocation_traits(…, empty)` reader is a documented conservative **widening** query, never hook selection)* |
| C4 | `head_identity.rs` (parallel offset-keyed binding table, 20+ consumers) | realm `BindingKnowledge` | **done (P1a)** *(module deleted; the scan and its offset-keyed facts are the realm command-binding state — `tcl_compiler::realm::CommandBindingRealm`, spec-keyed `Spec`/`TakenOver`/`Deleted` facts with a `knowledge_at` `BindingKnowledge` view composing document facts over the environment under the world policy; all 35 consumer files ported; the `retired-api-gate` holds `head_identity`/`HeadIdentityMap`/`HeadIdentity`/`command_head_identities` at zero)* |
| C5 | `KnownPredicateCtx`'s unfiltered `builtin_command_names()` and the settlement-vs-W123 oracle split | the one `exists` oracle (R-c) | **done (P1a)** *(`builtin_command_names` is the oracle's registry tier — context-resolved names plus the §4b iRules extension — and W123's `w123_registry_known_names` reads the same cached set, so settlement, const-dispatch, W113 and W123 share one answer; the enumerated delta: settlement no longer believes in environment-disabled commands, pinned by `builtin_command_names_caches_per_dialect`)* |
| C6 | The analyser's ad-hoc alias/rename/delete tables + `indirection.rs`'s bounded link walk as settlement inputs | `state_transition.rs`-fed realm state (which already carries the vocabulary) | P1a *(partial: the tables now feed the one oracle — `command_binding_knowledge` is their sole W123-class reader, and the realm module owns the top-level scan — but the tables themselves (`command_aliases`, `renamed_commands`, `deleted_commands`, the offset maps) are not yet re-homed onto `state_transition.rs`-fed realm state; that re-homing and `indirection.rs` remain)* |
| C7 | `side_effects.rs`'s hand-rolled spec-selection rule | the single selection primitive | P1 *(compiler ported: the rule lives in the model as `side_effect_hints_in_context`, now behind the I4 head proof (measured: the proof gate widens zero selections catalogue-wide); the walk itself survives inside the primitive because proved-single-winner selection is **measured non-equivalent** at 9 points — `next` under `bpf`, `exit`/`send`/`close` under `expect`, `option`(+4 subcommands) under `spectcl` — pinned by `c7_hint_walk_counterexamples`, which fails the day the catalogue moves those hints onto the winning specs and the collapse can complete)* |
| C8 | `CommandTableEffect` (third transition vocabulary) | `CommandBindingTransition` | P1a *(open: the realm scan still dispatches on `CommandTableEffect`; folding it into `CommandBindingTransition` is a coordinated registry+SpecTcl vocabulary change untouched by the P1a realm landing)* |
| C9 | Whole-file `package_version_floor` as a semantic input; the `DocumentFloor` duplication | one floor engine, two typed views (R-d) | P1a/P1b *(P1b lands the §5.4 range verdicts **beside** the floor engine, not as a second one: the version-gate flush asks the same `axis_floor` for the primary (semantic-floor) verdict and, only when that is satisfied, the document context's declared-target sets — `ResolvedContext::targets_outside_window` / `targets_uncovered_by_gate` — for the W150 range remainder, so a primary failure always outranks a range warning at the same word. `package_version_floor` and `DocumentFloor` themselves remain the two copies this row retires)* |
| C10 | `all_dialect_command_names()`'s hardcoded 11-pack list and its EDA/SpecTcl exclusion policy | the four-tier known-anywhere model, with the exclusion policy restated as explicit tier data | P1 |
| C11 | The ~20 hardcoded command-name match sites (terminal-action sets, `global\|variable\|upvar\|trace`, `set\|incr\|append\|lappend`, oo keywords, `on\|trap`, …) and the hardcoded `tcl8.5\|tcl8.6` profile-name match in the optimiser | registry descriptor data (traits, roles, clause grammars, definer grammars) and core-profile predicates | P1, gated |
| C12 | `RuntimeExprSurface::for_tcl_version` and the duplicated operator/word tables; free-function `binary_bp` | `ExprGrammar` per core profile (precedence, symbolic operators, mathfunc sets, arity, substitution) with `for_profile`-only derivation | P1 |
| C13 | `optimiser`'s and `sccp`'s direct spec reads that bypass trust where they still do | `CommandTrustSnapshot`/`BindingKnowledge` everywhere a fold rewrites | P1a |

### Backends

**Status: engine and runtime ingress ported (P1-F wave 3).** Both engines
(`tcl-vm`, `runtime/rust`), the engine adapter (`tcl-engine-tclvm`), the
two VM-driving hosts (`tcl-vm-cli`, `tcl-debugger`) and the `xtask` sweeps
and generators that exercise them now resolve every dialect **name**
through the one shared seam, `tcl_registry::model::ingress`, and reach
the registry as per-environment `ContextRegistry` generations. Each crate
carries one small `environment` module — the backend twin of
`tcl-lsp-core`'s ingress functions — so a call site names an ingress
rather than a validator: `profile_for_dialect` (`resolve_environment(…)
.unit_profile()`, replacing `DialectProfile::by_name` and the named
constructors), `store_for_profile` / `store_for_dialect` (the generation's
command store, replacing `registry_for_profile` /
`registry_for_dialect`), and `surface_mask` (the resolved environment's
**document authoring mask**, replacing the direct `availability_mask`
reads). Both engines resolve mask *and* store once per profile pin and
cache them on the interpreter, because the builtin-surface gate is
consulted on every command resolution and the generation lookup takes a
lock where the retired mask read was a field read. `codegen_abi`'s three
raw `DialectSet::parse` ingresses — the backends' last — become
`resolve_known_environment`, keeping the fail-closed decline rather than
falling through to the lenient environment's permissive mask.

Behaviour is unchanged by construction: the only names these crates
accept are the closed release set `TclVersion::dialect_profile_name`
spells plus the fixed `f5-irules`/`tk`/`expect`/`f5-iapps` projection
targets, whose environments are their same-named catalogue entries; the
generation's store is the very `Arc` the old `(profile, overlay)` cache
owns (profile stamp included, which the Zed projection reads back); and
the document authoring mask is test-pinned equal to the threaded
profile's `availability_mask` for every profile an ingress can produce.
The generator `--check` modes are the gate. The old APIs this left for
other crates are now deleted — see the P1-G status at the head of this
section.

| # | Retired mechanism | Replacement | Phase |
|---|---|---|---|
| B1 | Duplicated `builtin_command_visible_for_surface`/`profile_admits_registry_builtin` in both engines | one shared availability query over declarations | P1 *(partial: both engines' gates now read a per-environment generation and its document authoring mask, resolved once at pin time through the seam — no `by_name`, `registry_for_profile` or `availability_mask` read left in either engine's dispatch path. The **duplication** is what remains: the two bodies still differ because the two engines carry different command tables and different "unknown to the registry" rules, so collapsing them onto one declaration-level query is an engine-contract change, not an ingress port)* |
| B2 | Both engines' hardcoded 12-name `UNSAFE` (+ platform) safe-interp lists | `Traits::SAFE_INTERP_HIDDEN` (already 14 specs) queried generically | **done** (command half) — `tcl_registry::safe_interp_hidden_commands()` is the one generic query; both `make_safe`s call it and narrow by what the interpreter carries. The `UNSAFE_PLATFORM` scrub stays a name list under `TODO(ledger B2-platform)`: `special_vars` models `tcl_platform`'s keys but has no "scrubbed when made safe" flag, so driving it needs a `SpecialVarKey` field, not a query. See the measured-evidence subsection below. |
| B3 | VM's hand-typed 37-name mathfunc registration and 27-op `mathops!` macro | derived tables (as the runtime already does) | **done** — the VM registers `tcl_syntax::expr::mathfunc::all()` and every `mathop_shape` spelling from `expr::operators`, each behind one fn pointer that reads the op off the invoked word. This added the 21-name TIP 745 batch to the VM (listed below); the bodies already existed in `dispatch_with_backend`, only the command bindings were missing. |
| B4 | `TCL_PATCH_LEVEL`/`"9.0.4"` literals behind `package provide Tcl` and `tcl::build-info` | the pinned core release as provider (§3.2) | **done** — `TclVersion::core_provided_packages()` and `tcl_dialect::build_info` are the single tables; both engines re-derive their pre-provided core packages on every profile pin and compose `::tcl::build-info` from `TclVersion::patchlevel()`. `patchlevel()`'s 9.1 entry is now the measured `9.1b0`. |
| B5 | VM `package ifneeded\|forget\|unknown\|prefer` silent no-ops | real handling or honest `Unknown`-widening errors; fuzz-paired with the runtime | P1a |
| B6 | Runtime expr parsing under `dialect = None`; both engines' `for_tcl_version` expr surfaces | core-profile `ExprGrammar` threading | P1 *(untouched by wave 3: `RuntimeExprSurface::for_tcl_version` is keyed by `TclVersion`, not by a dialect name, so it is not an ingress this wave reaches — it needs the `ExprGrammar` threading itself)* |
| B7 | `tcl-engine-api`'s bare `restrict_commands(&[&str])` with no profile pinning; `SANDBOX_COMMANDS`'s out-of-registry closed world | an environment/policy handle on the engine contract; the sandbox surface as a closed-world environment | P1a |
| B8 | `try_lower_hook`'s proof-free selection; inline/analyser/const-fold hook selection without trust | the `ProofStatus` discipline generalised (I4) | **done (P1a, compiler half)** *(`try_lower_hook`, the analyser-hook dispatch, and type-infer's const-fold-facing spec facts select only under the environment-level binding proof — `Unavailable ≠ permission`, `Absent` ⇒ no hook; the codegen inline-hook reads already resolve under `own_availability_mask`. The **trust** conjunct — `CommandTrustSnapshot` where a fold rewrites — remains C13's row)* |
| B9 | Runtime-only, regex-shaped command-backing scan; no VM parity gate | structural registration parity for **both** engines against the catalogue, with per-family exclusions (a `Core(jim)`-only command is not a WASM obligation) | P2 |
| B10 | `_registry_data.tcl` (orphaned, 2,086 lines, frozen `DialectSet` subtraction) and the misc unsupported-stub name list | `gen-irule-test-data` output; registry capability predicates | P1 *(partial: the replacement generator's own ingress is ported — `gen-irule-test-data` resolves the fixed `f5-irules` name through the seam, reads the environment's registry generation, and filters with `ResolvedContext::resolve_spec` instead of `ProfileQueries::resolve_command`, so the surface it projects is a context answer rather than a profile-mask one. `--check` is the drift gate and passes unchanged. Retiring `_registry_data.tcl` itself is the remaining half)* |
| B11 | Debugger's `by_name("tcl9.0")` literal; vm-cli's release-only ingress | `Environment::resolve`, environments incl. non-plain-Tcl | P1 *(partial: both literals are now `resolve_environment(…)` through the seam, and both hosts' `CompileService` reads the environment's registry generation. The **other half** — accepting a non-plain-Tcl environment at all — is a payload change in both (a wider `--tcl-version` acceptance set in vm-cli, a dialect input the DAP surface does not have in the debugger), so it stays open and is marked `// P1:` at both sites)* |
| B12 | Fuzzer's three-value `Engine` enum + generator name lists + persisted `TclVersion` findings field | environment-driven engine pairing against the oracle ledger; a findings-registry migration for the persisted release field | P1b/P2 |

#### Measured evidence for rows B2–B4

Every figure below was read off the reference interpreters on `PATH`
(`tclsh8.4` 8.4.20, `tclsh8.5` 8.5.19, `tclsh8.6` **8.6.14**, `tclsh9.0`
9.0.4, `tclsh9.1` 9.1b0) on 2026-08-26. Note the 8.6 build measured is
8.6.14 while the engines pin 8.6.16 (the tarball under `tmp/`); only the
patch digit differs, and `TclVersion::patchlevel()` remains the one place it
is written down.

**B2 — safe-interp hidden sets** (`interp create -safe s; lsort [interp
hidden s]`, top-level command names only; the `tcl:file:*` / `tcl:zipfs:*` /
`tcl:clock:*` entries 8.6+ also lists are C's internal rewrite names for an
ensemble's unsafe *subcommands*, which neither engine models):

| release | hidden set |
|---|---|
| 8.4.20 | `cd encoding exec exit fconfigure file glob load open pwd socket source` (12) |
| 8.5.19 | + `unload` (13) |
| 8.6.14 | + `unload` (13) |
| 9.0.4 | + `unload zipfs` (14) |
| 9.1b0 | + `unload zipfs clock` (15) |

The registry trait's 14 specs are exactly the 9.0 set. The per-release
differences need no second availability rule: `unload` and `zipfs` are
release-gated commands, so "hide what the trait names, if this interpreter
carries it" reproduces every row. Two rulings came out of the measurement:

- **`clock` is deliberately not given the trait.** 9.1 hides the C `clock`
  and immediately re-provides a safe one, so `s eval {clock format 0 -gmt 1}`
  succeeds inside a 9.1 safe child exactly as inside an 8.6 one (measured).
  The trait means *not callable in a safe interpreter*; marking `clock` would
  make the analyser's safe-context walk report a false positive on working
  code. Its appearance in `interp hidden` is a hide-then-alias artefact.
- **Neither engine implements `load`, `unload`, `socket`, or `zipfs`** as
  commands at all, so their hidden sets are the 8.4 row on every pin. That is
  an engine gap, not a `make_safe` bug; both engines' tests assert the
  residue is genuinely absent from `info commands`, so implementing any of
  them forces the expectation to be revisited.

**B3 — the mathfunc delta the VM was missing** (21 names, TIP 745's C99
batch plus the `ldexp`/`logb`/`signbit`/`trunc` group that arrived with it):
`acosh asinh atanh cbrt copysign dim erf erfc exp2 expm1 fma gamma ldexp
lgamma log1p log2 logb nextafter remainder signbit trunc`. No function
bodies were needed — `tcl_syntax::expr::mathfunc::dispatch_with_backend`
already implemented all 21 and `runtime/rust` (which derives its list)
already registered them; the VM simply never bound the command names, so
`expr {cbrt(27)}` was `invalid command name "tcl::mathfunc::cbrt"` under
every pin including 9.1.

**B4 — the core provide matrix** (`package provide <name>` in a fresh
`tclsh`):

| release | `Tcl` | `tcl` | `TclOO` | `tcl::oo` |
|---|---|---|---|---|
| 8.4.20 | `8.4` | — | — | — |
| 8.5.19 | `8.5.19` | — | — | — |
| 8.6.14 | `8.6.14` | — | `1.1.0` | — |
| 9.0.4 | `9.0.4` | `9.0.4` | `1.3.1` | `1.3.1` |
| 9.1b0 | `9.1b0` | `9.1b0` | `1.3.1` | `1.3.1` |

Three release facts, all previously frozen at the 9.0 answer in both
engines: the lowercase spellings are Tcl 9 only (TIP 590), 8.4 provides
`TCL_VERSION` rather than `TCL_PATCH_LEVEL`, and `TclOO` is absent before
8.6 and one minor version behind there. `tcl::tommath` / `zlib` /
`tcl::zlib`, which the reference interpreters also pre-provide, are
deliberately *not* claimed: neither engine implements those surfaces, and
providing them would turn a `package require` failure into a later
`invalid command name`.

The user-visible consequence: `package require Tcl 8.5` (which means
`[8.5, 9)`) now fails under a 9.x pin, matching `tclsh9.0`'s
`version conflict for package "Tcl": have 9.0.4, need 8.5`. Both engines
wrongly succeeded before.


### Tooling / AI / editors

**Status: CLI, MCP and studio ingress ported (P1-F wave 4, the final
wave).** The two CLIs (`tcl-cli`, through the shared `tcl-cli-support`),
the MCP server (`tcl-mcp`), the spec studio (`tcl-spec-studio`) and the
pack loader's name ingress (`tcl-spectcl`) now resolve every dialect
**name** through the one shared seam, `tcl_registry::model::ingress`, and
answer availability from the resolved environment's `ResolvedContext`
rather than from `ProfileQueries` over a threaded profile. Each crate
carries one small `environment` module in the wave-2/3 shape —
`profile_for_dialect`, `known_profile_for_dialect` (the validator),
`context_for_dialect` (the assistance view), `store_for_dialect` (the
generation's command store) — plus, where a crate genuinely needed both
ingress forms, the exact `by_name` twin
(`analyser_profile_for_dialect` / `analyser_mask_for_dialect`, built on
`DocumentEnvironment::analyser_profile`): the CLI's KCS help filter and
the pack-carrying registry cache key deliberately sink `tk` to the
permissive fallback rather than promoting it, and splitting the two forms
is what keeps that behaviour rather than silently widening it.

**P3 ruling — the analyser-vs-unit `tk` asymmetry is permanent.** The
half that was an *availability* split is gone: the `tk` environment's
ambient Tk placement derives the `TK` authoring bit, so
`document_authoring_mask()` is a plain read of the generation's own
context for every environment and the injected second value
(`ResolvedContext::with_authoring_mask`, plus a second leaked
document-context) is deleted. What remains is a *catalogue* asymmetry,
and the classification rule (redesign §2) is what fixes it: `tk` is a
package plus an environment, never a dialect, so `DialectProfile::find
("tk")` must keep answering `None` and `analyser_profile` must keep
sinking to the permissive fallback, while `unit_profile` keeps promoting
so a compilation unit carries the `tk` identity (name, label, Tk library
pins). It retires with the interned `DialectProfile` itself, under
ledger C1/F1 — not before.

`ProfileQueries` has no caller left in these crates: `tcl lookup` and the
MCP `command_info` answer `resolve_command` / `available_option_names` /
`keyed_version_range` from the document context, and
`spectcl_check`'s collision test threads the target dialect's mask
through the seam instead of reading the generation's profile *stamp*
back. The MCP session-dialect plumbing's manual `KNOWN_DIALECTS`
membership scan becomes `canonical_id_for_dialect`, and `set_dialect`'s
validator becomes the one `Environment::resolve` — the same accepted
widening wave 2 made at the LSP's `setDialect` (row F9): every *declared*
name resolves, an unknown spelling is still rejected, and the advertised
`enum` (row T6's payload) is untouched.

The two wave-3 `xtask` holds are cleared. `gen_ai`'s vendor-surface
summary moves to a new `ResolvedContext::vendor_command_surface`, pinned
equal to `ProfileQueries::vendor_surface` for every catalogue profile over
that profile's own generation (`vendor_surface_matches_the_profile_query`)
— the additive twin the hold was waiting for. `gen_zed_queries`'
ambient-package filter now threads each target's resolved context and asks
`placement_is_ambient` (the documented twin of the retired
`DialectProfile::is_ambient_package`) instead of reading the generation's
profile stamp; every generator `--check` stays byte-identical.

What deliberately stays: the `DialectProfile::all()` **enumerations** —
the CLI's `--dialect` possible values and its unknown-dialect message, the
MCP `dialect_schema` enum, the studio's picker, `registry-dump
--all-dialects`' Tcl-release list — because the environment list has
different contents and no `short_name` (the row F9 payload rule); those
are rows T1/T3/T6/T7's payload, not refactors. The old APIs this left
for other crates are now deleted — see the P1-G status at the head of
this section.

| # | Retired mechanism | Replacement | Phase |
|---|---|---|---|
| T1 | `dialect_possible_values()`'s `+ tk` special case; `known_dialect_names()`'s manual chain | environment name+alias enumeration | P1 *(partial after wave 4: the `tk` name in both chains now resolves through the seam, and the CLI's ingest validator is `resolve_known_environment` rather than `DialectProfile::resolve_known` — but the enumerations themselves are this row's payload (`--help`'s possible values and the unknown-dialect message), so re-keying them is a user-visible change and stays open)* |
| T2 | Explorer's second resolution with silent `by_name` plain-Tcl fallback | pass the resolved environment handle; unknown names error | **done** (wave 4b) — all twelve `serialise.rs`/`lib.rs` sites route through a new `tcl-explorer/src/environment.rs` with four exact resolver twins. One correctness note for every seam consumer: `resolve_known_environment("tcl")` is `Some` (the lenient sink's own environment id) where raw `DialectProfile::resolve_known("tcl")` was `None`, so the explorer's `known_profile_for_dialect` is composed from the catalogue twin plus the `tk` promotion rather than the environment call — pinned by `the_ingress_forms_differ_only_at_tk`. Wave 4's tcl-mcp/tcl-cli-support twins use the environment call and therefore *accept* literal `"tcl"`; for MCP `set_dialect` that is the documented accepted widening, but any future caller needing the old refusal must use the catalogue-composed form |
| T3 | `registry-dump --all-dialects`'s single-`tcl8.6`-registry shortcut | per-family enumeration over the catalogue | P1 *(partial after wave 4: the `tcl8.6` name resolves through the seam; the shortcut and the `const_fold_version` family list are the payload and stay)* |
| T4 | Hardcoded `tcl8.6` defaults (CLI `combined_effective_dialect`, VS Code `contextPack`, clap args) | the configured default environment | P1 *(partial after wave 4: both CLI defaults resolve through the seam and are marked `// T4:` at their sites; the spelling stays hardcoded until the configured default environment exists, because changing it changes what an unstated document is analysed as)* |
| T5 | f5-cli's unvalidated `--dialect: String` | validated ingress through `Environment::resolve` | **done** (wave 4b) — all six `irule.rs` sites (the two enumerated plus four found in-sweep: the formatter highlight, and three `registry_for_profile(irules())` reads now on `static_context_for("f5-irules").commands()`) route through `tcl_cli_support::environment`; `run_format`'s user-threaded name uses the exact `by_name` twin so acceptance is unchanged |
| T6 | MCP `dialect_schema`'s 18-name enum; `spectcl_check`'s `availability_mask` bit test | environment enumeration; `targets ⊆ applicable` collision checking | P1/P2 *(partial after wave 4: the bit test now takes the **threaded target's** mask through the seam rather than reading it off the generation's profile stamp, so it no longer depends on the stamp; the enum is the payload half and `targets ⊆ applicable` is a model change, both open)* |
| T7 | Studio's `DIALECT_BITS` editor, dialect-string APIs, `SOURCE_DIALECT_KEY`, dialect-as-language-id client | provider/`VersionSet` editing, environment ids, generic contributed LSP identity (B7) | P2 *(unchanged in substance by wave 4: the studio's `Builtins::for_dialect`, its command browser, the corpus scanner and the pack registry all resolve through the seam — `catalogue_dialect_or_default` is the `find(…).map_or("tcl9.0", …)` twin — but the DIALECT_BITS editor and the dialect-string APIs are the payload this row retires)* |
| T8 | `render_spectcl`'s `is_dialect_set` conflation of availability with `safe_on_uninit`/`two_arg_optionless_dialects` | distinct spellings per gap ruling R4 | P2 |
| T9 | `spec-author` skill's 1.1 instructions | 2.0 refresh (words, `dialect` blocks, `available`, upgrade workflow) | P2 |
| T10 | `callback-surfaces` `name@dialect+dialect` row ids (and the `.chain(tk())` special case) | environment/provider-keyed ids; one-shot regeneration | P1 *(unchanged in substance by wave 3: `callback_inventory`'s ingress and registry access now go through the seam — the `tk` arm is `profile_for_dialect("tk")` and `visible_in` is the resolved environment's document authoring mask, replacing `resolve_known(…).unwrap_or(plain_tcl).availability_mask` — but the row ids and the `.chain(…)` enumeration are the payload this row retires, and re-keying them regenerates the committed JSON, so it stays open)* |
| T11 | `gen_zed_queries`'s `grammar_union`/`TK_AND_TCL` inputs; `gen_editor_catalogs`/`gen_tmlanguage_keywords`'s `ALL_TCL` bit filters | declaration-derived fast paths | P1 *(unchanged in substance by wave 3: `gen_zed_queries`'s four targets are now `profile_for_dialect(id)` and its store is the environment's generation, but the `grammar_union` mask it projects under is `DialectSet` plumbing (row C1) and stays until P1-G. Its ambient-package filter reads the generation's profile **stamp**, the same `is_ambient_package` hold wave 2 left in the LSP, marked `// P1-G:` at the site)*. **Wave 4 cleared that hold**: each target now carries its canonical environment id, `classify` takes the target's `ResolvedContext`, and the filter asks `placement_is_ambient` — the stamp read is gone and `--check` is byte-identical. The `grammar_union` mask stays (row C1). |
| T12 | AI manifest's release-keyed Tk fragment; prompt loader's alias-blind `dialects[]` check | environment-keyed manifest; alias-resolved loading | P1 *(partial after wave 4: the manifest's catalogue-membership gate resolves through the seam (`resolve_environment(name).catalogue_profile()`, the exact `DialectProfile::find` twin); the release-keyed Tk fragment and the runtime prompt loader are the payload and stay)* |
| T13 | Every hand-maintained projection found (Sublime map; the orphaned simulator data) | generated + drift-gated (rule 1.1) | P1 |

## 4. Gap rulings

The audit surfaced questions the redesign had not answered. Proposed
rulings (▸ = recommendation; genuinely owner-level items are also
mirrored as Q22–Q25 in the main document):

- **R1 — stubs are declarations.** Inline `# tcl-lsp: stub` and sidecar
  `.stubs` ingest as `SurfaceDeclaration`s with `Document`/`Workspace`
  provenance and the lowest trust class; the separate
  `StubOverlay` type and its per-consumer consultation retire. The stub
  fingerprint's role in cache keys is subsumed by the generation/overlay
  hash. (Q22)
- **R2 — the variable axis is part of the model.** Special variables are
  family/build-sensitive (Jim's `env`, picol 2's capital-initial
  globals); `special_vars.rs`'s table becomes declarations authorable in
  SpecTcl `dialect`/package blocks, and its private dialect-name ingress
  retires (C2). `dynamic_names`'s three flow-insensitive bools become
  realm variable-domain widening. (Q23)
- **R3 — `FILE_SCOPED_ENVS` becomes a detection-scoped surface.** The
  `tclpkg.tcl` whole-file command environment is an
  environment-with-detection-facts (filename-keyed) whose surface is a
  pack, not a hardcoded Rust table.
- **R4 — non-availability `DialectSet` fields do not translate to
  `available`.** `safe_on_uninit` ("reading this uninitialised variable
  is safe in these dialects") and `two_arg_optionless_dialects` are
  behaviour predicates keyed by core profile; they gain their own 2.0
  spellings resolved through the environment alias table, and the
  renderer stops conflating them with availability.
- **R5 — hook `ctx` keys gain, not lose, a spelling.** Pack hook bodies
  read `dict get $ctx dialect`; 2.0 adds an `environment` key and keeps
  `dialect` as a documented legacy alias forever (same policy as legacy
  words). (Q25)
- **R6 — `tclpkg.tcl` grows a targets notion; MVS stays MVS.** The
  shipping manifest grammar accepts one operator + one version (the
  main document's §5.4 claim is corrected accordingly), and the resolver
  is deliberately Go-style minimum-version-selection with no upper
  bounds. Ruling: the `tcl` constraint gains a multi-clause range
  grammar, and a new, resolver-invisible `supports NAME RANGE` directive
  declares analysis targets — `require` stays a bare MVS floor. The
  three version comparators collapse onto the oracle-pinned
  `tcl_dialect` algebra behind `VersionSet` (I2); `tcl_pkg::version`
  becomes a wrapper. (Q24)
- **R7 — new CLI verbs are budgeted.** `tcl spec check` (promoting the
  MCP-only checker) and `tcl spec build --emit rust` (the Q1 AOT
  backend, requiring a pack-level and `dialect`-block-aware renderer —
  the current WASM-only per-command renderer cannot do it) are P2
  deliverables alongside `spec upgrade` (§6).
- **R8 — per-family gate scoping.** The command-backing obligation is
  scoped per family and backend: provider `tcl` at 9.0 for the WASM
  runtime as today, with `Core(jim)`-only commands excluded rather than
  becoming phantom obligations; the new structural parity gate (B9)
  carries the scoping.
- **R9 — docs vocabulary follows.** The KCS "Applies-to" controlled
  vocabulary and the docs link gate consume environment names; P8's
  sweep regenerates the tagged pages.
- **R10 — one-oracle gate.** A new invariant gate (extending
  `resolution_drift`'s spirit beyond a grep window) asserts: no consumer
  constructs its own command-existence oracle, availability rule, or
  binding table — mechanically, the only callers of the registry's raw
  lookup layer are the two typed views, enforced by visibility
  (`pub(crate)`) plus a call-site sweep, with the escape hatch requiring
  a ledger entry here.
- **R11 — F5 rows are evidence-generated** (the
  [BIG-IP evidence review](dialect-and-package-registry-redesign-bigip-evidence-review.md),
  accepted in full — redesign §0.2). Every F5 grammar/command/variable/
  package/policy record is keyed by `BigIpExecutionContext` and backed
  by `EmbeddedRuntimeEvidence` from a checked-in conformance corpus
  (per-build, per-context manifests with explicit `unknown` cells and a
  transcript validator that never runs in CI); a drift gate fails when
  prose, registry rows, or tests disagree with the corpus. Unmeasured
  contexts resolve `Unknown` for semantic passes and at most a labelled
  nearest-known assistance profile. The `}{` separator's scope, the
  tmsh syntax axis and its `tmsh::modify cli version active` transition,
  role/policy visibility overlays, `tcl_platform`'s CMP effect overlay,
  and iApp action metadata overlays all land here — before P4's F5
  migration, which holds until the review's acceptance matrix is
  covered. **Status (evidence lane, #15)**: the typed layer is in —
  `BigIpExecutionContext`, `EmbeddedRuntimeEvidence` with its
  semantic/assistance door split, the typed BIG-IP and tmsh-syntax axes
  with the `cli version active` transition, the iApp action overlays, and
  the hermetic corpus. Outstanding under R11: rows are *asserted* from
  the corpus but not yet *generated* from it, the transcript-schema
  validator is unwritten, the F4 role-visibility and F5 `tcl_platform`
  CMP overlays are recorded as evidence but not wired as overlays, and
  the corpus covers one build and four contexts.

## 5. Gates that prove the centralisation

| Gate | Proves | Invariant |
|---|---|---|
| One-oracle visibility gate (R10) | no parallel existence/availability/binding mechanisms reappear | I3–I5 |
| Structural engine parity, both engines, per family (B9) | registration derives from the catalogue; no hand-list drift (the TIP 745 class of bug) | I1 |
| Safe-interp trait conformance test | engines hide exactly `Traits::SAFE_INTERP_HIDDEN` | I1 |
| Core-provider version test | `package provide Tcl` ≡ pinned release in both engines | I1 |
| `Environment::resolve` property test | every ingress accepts exactly canonical names + aliases; all six retired validators' inputs covered | — |
| Generation drop test (1,000 reloads, allocator-accounted) | dynamic specs, stubs, environments release | I7 |
| Fail-closed overlay test | a generation/overlay miss never serves degraded data silently | I9-adjacent |
| Upgrade equivalence (`--verify`, §6 U9) | 1.x pack and its upgraded 2.0 form load to byte-identical registry snapshots | I10 |
| Behavioural parity suites per migration | user-observable behaviour, not just serialisation | I10 |
| Projection drift gates (existing + Sublime map + simulator data) | no hand-maintained projections | — |
| Retired-API gate (`cargo xtask retired-api-gate`, P1-G) | the deleted dialect-name validators and string-keyed registry doors never reappear under their old spellings; escape hatch requires a ledger entry | I3–I5-adjacent |

## 6. `tcl spec upgrade`: the 1.x → 2.0 specification

The one sanctioned backwards-compatibility surface. Two halves: the
loader reads every 1.x pack forever (redesign §6.1), and this tool
rewrites 1.x sources to 2.0. Three facts fix its shape:

1. **It is a source rewriter, never a load-render round-trip.** The
   renderer emits no pack-level rows (`ambient_package`,
   `file_extension`, pack `hook`s would be silently deleted) and 15
   `DraftOpaque` + 5 `Excluded` fields degrade to TODO comments. Edits
   are computed as content-range replacements located by the loader's own
   lexer (the `speclib_version_span` discipline: same lexing, same BOM
   handling, applied back-to-front, never reformatting) so author
   layout, comments, and delimiters survive and diffs are reviewable.
2. **The 1.x dialect vocabulary is closed**: 13 `DIALECT_BITS` names +
   `all-tcl` + `tcl8.x` + five `tclX.Y+` forms — 21 tokens. The
   translation table is total and provably exhaustive.
3. **The loader's per-site vocabulary log inverts**: the machinery that
   notices "this word is newer than your declaration" also tells the
   upgrader which sites force 2.0 and proves the rewritten pack needs
   nothing newer than it declares.

Capabilities (U-numbers are the implementation checklist):

- **U0** — keep today's behaviour as the degenerate case: `--check`,
  skip-on-no-`speclib`, refuse-on-non-vocabulary-word, reload-after-write
  proof, exit 1 on remaining work.
- **U1** — version word → `2.0`, permitted only when the body rewrite
  (U2–U5) completed on that file; a `2.0` header over 1.x spellings is
  loadable but reports as a failed upgrade.
- **U2** — `dialects`/`-dialects` → `available` at all 12 loader sites
  (pack defaults, command, subcommand, option, object-class method,
  command form, side-effect, option constraint, values rows), via the
  total table: `tclX.Y` → single-point set; `tclX.Y+` → open range;
  `all-tcl` → `{tcl 8.4-}`; `tcl8.x` → the 8.x set with the exclusive
  maximum stated; `f5-irules` → the core family; `tk` →
  `{package Tk}` on Tk's own axis; `f5-bigip` → **error** (leaves the
  Tcl axis; never translate). Output reuses the renderer's shorthand
  logic so upgraded packs read hand-written.
- **U3** — role discrimination, the hard part: availability tokens →
  `available`; environment-membership tokens (`f5-iapps`, `f5-tmsh`,
  `expect`, `spectcl`, `bpf`) → the environment's ambient package
  provider (requires the P1 environment registry); non-availability
  policy fields (R4) → their own 2.0 spellings, never `available`.
- **U4** — `ambient_package NAME VERSION` → environment-scoped
  `ambient` placement. The 1.x row never named its environment: wrap in
  the pack's unambiguous declared environment when one exists, else
  leave in place with a `# TODO(spectcl 2.0):` marker and report
  *partially upgraded*. Never guess. (No bundled pack uses the row; this
  path first bites user packs.)
- **U5** — `file_extension … -dialect D` → detection rows inside the
  `environment` block, same cannot-infer rule; the one bundled instance
  (`eda_synopsys`'s `upf` row) is the pilot.
- **U6** — optional `--infer-provides`: hoist a uniform per-command
  `required_package` to pack-level `provides`; off by default (changes
  shape, not spelling).
- **U7** — post-rewrite proof via the vocabulary log: re-load, assert
  nothing above `2.0` is required; report per file the sites translated,
  sites left TODO, and notice deltas.
- **U8** — byte-preservation contract (fact 1 above).
- **U9** — `--verify`: `load_pack(original)` and `load_pack(upgraded)`
  produce **byte-identical registry snapshots** (the `registry-dump`
  backend), wired into CI over the eight bundled packs — 1,165 `dialects`
  rows across them today, a real corpus from day one. The pack-level
  analogue of invariant I10.
- **U10** — explicit `--from`/`--to`; downgrades refused (an unsupported
  major fails closed, so a 2.0 → 1.x rewrite is a silent capability
  loss).

Sequencing: U0–U2, U7–U9 need only the 2.0 word set; U3–U5 additionally
need the P1 environment registry (and R4's ruling); U6 is independent.

**Status: U0–U2 and U7–U10 implemented** (P2-H part 1). The rewriter is
`tcl-spectcl/src/upgrade.rs`, driven by `tcl spec upgrade` with
`--from` / `--to` / `--check` / `--verify`; the 2.0 word set it targets is
`tcl-spectcl/src/loader/available.rs`. What landed, against the checklist:

- **U0** — `--check`, skip-on-no-`speclib`, refuse-on-non-vocabulary-word
  (the 1.x table is closed, so a word outside it refuses the *file* rather
  than being carried through unread), reload-after-write proof, exit 1 on
  remaining work.
- **U1** — the version word moves to `2.0` only when the body rewrite
  completed on that file; a file with any U3 row left keeps its 1.x header
  and reports *partially upgraded*.
- **U2** — `dialects` / `-dialects` → `available` at every loader site,
  through the total table. `tclX.Y` → single point, `tclX.Y+` → open range,
  `all-tcl` → `{tcl 8.4-}`, `tcl8.x` → `{tcl 8.4-9.0}` (exclusive maximum
  stated), `f5-irules` → the core family, `tk` → `{package Tk}` on Tk's own
  axis, `f5-bigip` → error.
- **U3** — role discrimination is performed against the live environment
  registry: a membership token whose environment declares exactly one
  ambient package translates to that provider (`f5-iapps` →
  `{package f5-iapps-cmds}`, `f5-tmsh` → `{package f5-tmsh-cmds}`,
  `expect` → `{package Expect}`), and the loader's `available` reader
  carries an environment-derived package↔bit table (the exact inverse),
  so the translated spec is byte-equal. `spectcl` and `bpf` stay markers
  **by measurement, not by gap**: their environments declare no ambient
  provider (their surfaces are compiled), so no `available` row can carry
  the claim yet — the marker names that reason.
- **U7** — post-rewrite proof through the vocabulary log: the rewritten
  file is re-loaded and every site needing a vocabulary above its own
  declaration is reported.
- **U8** — byte preservation: edits are content-range replacements located
  by the loader's own lexer and applied back-to-front; `hook` bodies are
  never descended into, because they are arbitrary Tcl rather than pack
  vocabulary.
- **U4** — `ambient_package NAME VERSION` rehomes into
  `environment OWNER -extend { ambient NAME VERSION }`, where OWNER is
  derived by the cannot-infer rule: the pack's sole declared
  `environment` block, else its sole membership token across `dialects`
  rows; an ambiguous pack (or a non-plain version) keeps a
  `# TODO(spectcl 2.0):` marker and reports partial. (No bundled pack
  uses the row; the path is test-covered.)
- **U5** — `file_extension … -dialect D` moves its detection into
  `environment D -extend { … }` (the flag dropped, everything else
  verbatim); an unresolvable `D` keeps the marker. The bundled pilot —
  `upf.tclspec`'s `upf` row — translates and verifies.
- **U6** — `--infer-provides` hoists a uniform `required_package` (the
  pack-level default, else one identical row in every command) to a
  pack-level `provides`, whose loader semantics (`provides` is the
  fallback provider default) keep the snapshot byte-equal. Off by
  default.
- **U9** — `--verify` compares `command_entry_json` snapshots of the
  original and the rewritten pack across every dialect a 1.x row can gate
  on — the Tcl ladder, `f5-irules`, **and** (since U3 translates them)
  `f5-iapps`, `f5-tmsh`, `expect`, `spectcl`, `bpf` — plus the
  **environment-effect snapshot** (`upgrade::environment_effect_snapshot`):
  the scoped detection and placement rows both forms load to, which is
  what licenses U4/U5 to move a row's home while proving the registry
  effect stayed put.
- **U10** — explicit `--from` / `--to`, downgrades refused.

Over the eight bundled packs today, `--verify` reports **1,168 rows would
translate, 0 left as TODO, 8/8 byte-identical registry snapshots**
(cadence 77, mentor 69, microchip 1, quartus 77, synopsys 68, xilinx 788,
sdc_base 86, upf 2 — its `all-tcl` default and its U5 `file_extension`
row). No bundled pack was rewritten.

**The rest of the P2-H remainder landed with it** (same change):

- The §6.2 words `provides` (+ the fallback-provider default),
  `co_provides` (parsed and carried as data; the loader-alias mechanics
  that consume it are P3+), `dynamic_surface` / `unknown_members` (the
  M8/G-series honesty hatch, mapping to `allow_unknown_subcommands` on
  commands and `-dynamic-surface`/`-unknown-members` →
  `allow_unknown_methods` on `object_class`), and `include` (pack-file
  inclusion under the determinism contract: literal pack-scope names
  only, resolver-bounded IO — `pack::load` scopes it to the including
  pack's own directory and bypasses the compiled cache for
  include-bearing packs — content-hash-keyed cycle rejection, bounded
  depth, provenance inherited, and the registration record carries the
  *included statements* so export writes the expansion). Every word is
  implemented once at the shared row-reader seam (both loaders), rides
  the export gates, and classifies **semantic** in the §6.1 downgrade
  table so an older build abstains rather than strengthening.
- `environment NAME -extend { … }` — the additive form the U4/U5
  rewrites target: detection rows and placements contributed to an
  environment declared elsewhere (compiled included); identity rows are
  rejected, and the §6.4 trust gate on extending a compiled base lives
  at registration and in the E-R2 evaluation gate.
- **Live environment registration** (the §1.1 P2 seam):
  `tcl_registry::model::registration::register_environments` swaps a
  rebuilt `EnvironmentRegistry` (compiled seed + extensions + dynamic
  definitions) in at the next generation, transactionally and
  idempotently; `model::ingress` resolves against the live registry, so
  invalidation is the generation bump the per-context caches already
  key on. Trust: reserved-name claims fail with the provenance-naming
  error for every non-built-in tier; workspace/studio tiers cannot
  extend compiled environments (`tcl_spectcl::registration` re-applies
  the E-R2 tier check for CST-loaded packs). The integration gate
  (`tcl-spectcl/tests/environment_registration.rs`) proves a
  pack-declared environment resolves through
  `model::ingress::resolve_environment` with its declared detection
  facts and ambient placements, and that reserved/untrusted claims fail.

**Production wiring (P3).** The seam is now what discovery and the server
actually run:

- **A second registration channel, keyed by source.**
  `model::registration::sync_environment_sources` takes the *whole* set of
  `EnvironmentSource`s (one per pack, id `tier:name`) and replaces the
  source-keyed half of the dynamic state, so an environment whose pack has
  left the workspace **retires** on the next rebuild. A source that breaks
  a rule is reported by id and dropped while the rest register — one
  malformed workspace pack must not un-register every other pack's
  environments — and a set identical to the registered one is a no-op,
  generation included, so an unchanged reload does not invalidate
  downstream caches. `register_environments` keeps its all-or-nothing
  anonymous semantics for single-pack callers.
- **One publish point.** `tcl_spectcl::bundled::set_active` — the process's
  answer to "which packs are loaded", called by the LSP server on every
  reload and by `tcl-cli-support` once per CLI process — now calls
  `registration::publish_pack_set`, which registers the set's environments
  and converts and registers its `dialect` blocks.
- **Detection routing follows the same claim.**
  `registration::extension_routes` is a pure function of a pack set —
  explicit `file_extension … -dialect D` rows first, then each
  `environment` block's own detection rows, first claim winning — and the
  pack *merge* publishes it, beside the `-dialect` rows it already
  published. Deliberately at the merge and not only at the registration
  seam: a reload whose content turns out identical never reaches the
  publish point, and routing that depended on registration order would
  lose its environment rows on exactly those reloads. So a pack-declared
  environment is resolvable *and reachable* — a document whose extension it
  claims routes to it through the ordinary ingress, with no setting and
  nothing in the file — and the server advertises the same extensions to
  the client (`pack_file_extensions`), so an editor that can register
  associations at runtime opens the file as Tcl in the first place. The
  server puts a refused pack's reason on the pack file as a `SPECTCL`
  notice and logs the counts.
- **`PackDialect` → runtime family data.** `tcl_spectcl::dialect_conversion`
  converts a validated `dialect` block to a
  `tcl_dialect::model::DynamicFamily`: namespaced `PACK/DIALECT` id (§3.3's
  pack-name-prefixed scheme), one `LexerGrammar` per declared release, the
  declaring tier's provenance. `tcl_dialect::model::dynamic` holds the store
  — compiled family names reserved at every tier, a generation, the same
  sync-and-retire shape. An `environment … { core DIALECT RELEASE }` row
  naming a pack-declared dialect is carried as `PackCore` and resolved
  against the pack's own `dialect` blocks after the whole file is read
  (so a forward reference works); it registers as a `DynamicCore` binding,
  and `dynamic_core_grammar(environment)` answers with the real grammar.
  **The boundary**: a converted family is not a `Family` variant and cannot
  be — that enum is closed and `grammar()` is a `const fn` over ladder
  ordinals — so the environment carries `core: None` plus the binding, and
  nothing on the analysis path consumes the dynamic grammar yet, because
  `tcl_lexer::LexerConfig` is built from a `&'static DialectProfile` the
  ingress hands out of a compiled table. Closing that last step is the
  `DialectProfile` re-type (ledger C1), not more conversion.
- The gates: `tcl-spectcl/tests/environment_registration.rs` (publish →
  resolve → route → retire, environments and dialects together) and the
  server e2e
  `spec_packs::a_pack_declared_environment_routes_its_documents_through_the_ingress`
  (a workspace pack's `environment` with a `file_extension` row, a document
  with that extension resolving to it through `getEffectiveConfig`, and the
  routing stopping when the pack is deleted).

Still open from this section: nothing in U0–U10. The invocation-refinement
descriptor remains §6.2 work; of the seven ratified-but-unimplemented words
six now load (`result_stability` at command and subcommand scope,
`event_requirement_form`, `data_collection`, `body_scope`,
`side_switch_target`, `event_handler_priority`), each at the shared
row-reader seam of both loaders and riding the export gates.
`bpf_op` stays deferred: `CommandSpec::bpf_op` is
`Option<&'static BpfOpSpec>` and every shipped value is a per-command
private `static OP` in `tcl-registry/src/commands/bpf/`, so the documented
`bpf_op -native ID` spelling has no id catalogue to resolve against — the
missing model piece is a named `id → &'static BpfOpSpec` table in
`tcl_registry::bpf_op`, not a loader reader.

## 7. The name-resolution oracle programme

The owner directive: use the C Tcl 9 codebase and tests, the stdlib, tcllib,
Tk, and the corpus to make name registration/resolution — namespaces,
variables, procs/commands, packages — rock solid, centralised, and
consumed by every consumer. The 2026-08-26 oracle survey (C test suites
across all five trees in `tmp/`, the library scripts as executable
specifications, and the repo's existing conformance infrastructure)
grounds the following programme. Everything below extends mechanisms that
already exist; nothing invents a parallel harness.

### 7.1 Reference interpreters — close the binary gap first

The trees for 8.4.20/8.5.19/8.6.16/9.0.4/9.1b0 are fetched, but only
tclsh 8.6 and 9.0 exist as binaries; **nothing builds 8.4/8.5/9.1**, so
`audit_option_dialects` silently degrades their columns to "unsupported
everywhere" and six VM cross-version suites' `TCL_LSP_TCLSH84/85/91` legs
never run. Deliverables:

- `ensure-test-deps.sh` builds in-tree `tclsh` for 8.4, 8.5, and 9.1 the
  same way it builds 9.0 (the recipe exists; the trees are present) —
  the full five-binary matrix becomes the standard oracle environment,
  keyed the way `cross_version_info_surface_e2e.rs` already keys it
  (`TCL_LSP_TCLSH{84,85,86,90,91}`).
- Fix the `tcltest_sweep` reference-path mismatch (it hardcodes
  `tmp/tcl9-install/bin/tclsh9.0`, which `ensure-test-deps` never
  creates) and make `audit_option_dialects` **fail loudly** on an
  unbuilt tree instead of emitting a degenerate column.
- Extend `fetch-tcl-source` (and the session hook) to fetch **Tk** trees
  per release; the adversarial review already pins Tk permalinks with no
  tree to verify against.
- Single-binary probes (`find_tclsh`'s first-hit) upgrade to the
  five-version matrix wherever a behaviour is release-differentiating.

**Status: implemented (P0-B).** The first three bullets above have landed:

- **Binaries.** `ensure-test-deps.sh`'s `ensure_tclsh` now builds all five
  reference interpreters, each left at `<tree>/unix/tclsh` (the path
  `audit_option_dialects` reads) and additionally exposed on `PATH` as
  `/usr/local/bin/tclsh{8.4,8.5,8.6,9.0,9.1}`. 8.4/8.5 build with
  `CFLAGS="-O2 -fcommon -Wno-implicit-int -Wno-implicit-function-declaration"`
  against modern gcc (no other overrides were needed) and, unlike 9.0/9.1's
  zipfs-embedded library, only find `init.tcl` via a path relative to their
  own real on-disk location — a bare copy or symlink into `/usr/local/bin`
  breaks that lookup, so those two are exposed via a thin wrapper script
  that `exec`s the tree binary by its real path (`unset TCL_LIBRARY` first,
  so a stale global export can't shadow the wrong version in) instead of a
  symlink. 9.1 tolerates a plain symlink. The build is idempotent, keyed on
  `<tree>/unix/tclsh` reporting the exact expected `info patchlevel`.
- **`tcltest_sweep` path.** The hardcoded `tmp/tcl9-install/bin/tclsh9.0`
  constant is gone; `resolve_tclsh90` in `rust/xtask/src/tcltest_sweep.rs`
  now tries, in order, the `TCL_LSP_TCLSH90` env var, `/usr/local/bin/tclsh9.0`,
  `tclsh9.0` on `PATH`, then the in-tree `tmp/tcl9.0.4/unix/tclsh`, and fails
  with a message naming `make ensure-test-deps` when none exist.
- **`audit_option_dialects` fail-loud.** The audit run (not `--check`, which
  still runs no tclsh) now calls `require_all_tclsh_built` before probing:
  if any of the four trees it audits has no `<tree>/unix/tclsh`, it fails
  immediately listing the missing binaries and naming `make
  ensure-test-deps`, rather than degrading that tree's column to
  `"tclsh not found"` on every probe. `AUDIT_ALLOW_MISSING_TCLSH=1` is the
  documented escape hatch for a deliberately partial run.
- **Tk trees.** `fetch_tcl_source.sh` gained `tk84`/`tk8.4`, …, `tk91`/`tk9.1`,
  and `tkall` selectors against the `tcltk/tk` repo, which shares Tcl's
  `core-M-N-P` tag scheme for every release fetched here (verified via
  `git ls-remote --tags`). `session-start.sh` runs `fetch_tcl_source.sh
  tkall` as a new step right after the Tcl fetch. All five landed at
  `tmp/tk8.4.20/`, `tmp/tk8.5.19/`, `tmp/tk8.6.16/`, `tmp/tk9.0.4/`,
  `tmp/tk9.1b0/` (89/104/109/115/119 `.test` files respectively), matching
  the Tcl versions already pinned. The codeload tarball CDN 403'd through
  this environment's proxy for all five, so every tree landed via the
  existing shallow-clone fallback — both paths are exercised, not just the
  happy one.
- The fourth bullet (upgrading single-binary probes to the five-version
  matrix) is **not yet done** — tracked as follow-up, not part of this
  landing.

### 7.2 Vector files — one format, five domains, per-release expectations

The command-resolution vector system is the proven shape: one declarative
pipe-separated file compiled into `tcl-syntax`, a renderer that turns
every row into executable Tcl, and five consumers (pure resolver,
analyser settlement, VM dispatch, WASM runtime, real tclsh) executing the
same bytes. Two generalisations:

- **Per-release expectation columns.** Today's 46 rows were chosen to
  agree on 8.6/9.0. The format gains release-tagged winners
  (`expected@8.4-8.6 | expected@9.0+` or equivalent) so
  version-*differentiating* rows — the whole point of a multi-release
  oracle — are expressible, and the tclsh pin runs the matrix, asserting
  each release's column against its own binary.
- **Sibling domain files** under the same `include_str!` + renderer
  discipline, each with its `vector_setup`/`vector_call`/`vector_script`
  triple (keeping the setup/call split the runtime needs):

| File | Domain | Seed content (anchors from the survey) |
|---|---|---|
| `variable_resolution_vectors.txt` | variable lookup/creation | promote the VM-local `cross_version_vars_e2e` const to shared data; the nine self-labelled TIP 278 sites (`namespace-14.3/14.12/17.7/17.10/34.7`, `namespace-old-5.4/6.12/6.14/6.15`) as 8.x/9.x pairs — commands keep the `::` fallback, variables lose it; `TclLookupVar` clusters (`var-1.x`: `global`/`variable` flags, creation-through-missing-ns errors, `:`-in-name literals); `upvar`/`namespace upvar` links (`var-3.x`, `upvar-NS-*`, dangling-into-deleted-ns); `const` and namespace constants (`var-25.x–28.x`, 9.0+); the creation-vs-lookup asymmetry rows (`var-1.9–1.13`) whose *direction inverts* at 9.0 |
| `namespace_op_vectors.txt` | namespace operations | the colon-normalisation matrix (`namespace-6.2–6.4`, `14.7–14.12`, `32.7/32.8`, `33.7/33.8`, `info-8.4`'s `{x {} x x x}`, `var-1.14`, `proc-1.6`, `init-1.7/1.8`) — the one area where namespaces, variables, and commands each treat the same bytes differently; import/`-force`/re-import idempotency and origin chains (`namespace-9.x`, `11.x`, `30.x`); export accumulation and `-clear` (`26.x`); `namespace which` (`34.x`); `namespace path` semantics incl. non-transitivity and deleted-entry non-restoration (`51.x`, with the 9.0 `51.13` delta); `namespace unknown` (`52.x` — only genuinely-unresolvable names reach the handler); ensembles (`42.x–50.x`, `-parameters` 8.6+); deletion cascades (`7.x/8.x`, imported commands dying with their origin ns) |
| `command_binding_vectors.txt` | registration/rename/shadowing | proc-into-namespace rules (`proc-1.1/1.2`: qualified proc names create in the named ns, which must exist); definition-ns execution (`proc-3.4`); `rename` across namespaces incl. epoch bumps and shadow checks (`rename-*`, `basic-18.x`, `basic-24.x`, `namespace-old-6.6–6.9`, `trace-19.6–19.11`'s qualified old/new names); hidden/expose invariants (`basic-12.1/13.1`: hidden names take no qualifiers, expose only into `::`); `interp alias` targets with `::` (`interp-27.x`); `info cmdtype`'s result vocabulary (9.0+, `info-40.x`) as the registry-model oracle; `resolver.test`'s six cache-invalidation paths as the epoch conformance set — these are the ground truth for `BindingKnowledge` transitions, so the realm layer (P1a) is a first-class consumer |
| `package_lookup_vectors.txt` | package resolution | provider selection over `ifneeded` sets (`package-3.1–3.5`); not-found error forms; `package unknown` handler argument shapes (`-exact t 1.5` ⇒ `t 1.5-1.5`; the 8.4 `name version ?-exact?` form as the 8.4 column); ifneeded-that-does-not-provide errors; `prefer` latching (`15.4`); the four release-discriminating `bad option` subcommand lists; tm path ancestry/descendant rejection and LIFO ordering (`tm-3.x`) |
| `autoload_vectors.txt` | the autoload tier | `init-1.1–1.8` (`auto_qualify`'s eight pairs) plus `init-2.x`'s two-stage chains with tripled colons — `auto_qualify` is byte-identical across all five releases and becomes the ported reference implementation for `PackageResolver::auto_qualify` |

Rows sourced from `knownBug`-constrained tests (`namespace-56.4`'s
namespace named `:`, `info-15.8`, `interp-27.5–27.8`) are recorded in a
**documented-non-conformance ledger** in the vector files' comments — so
nobody "fixes" our model to a behaviour real Tcl does not exhibit.

Two ready-made extractions ride alongside: the `package.test` vsatisfies/
vcompare tables (41 + ~140 rows, byte-identical 8.5→9.1, with an 8.4
column derived from `pkg.test`'s `!tip268` variants — mind the
8.4/8.5 `pkg.test` → 8.6+ `package.test` rename when anchoring) extend
the existing hermetic `package_version_oracle`; and the per-release
error-message strings (`namespace "X" not found in "Y"` vs 8.4's
`unknown namespace "X" in …`; `parent namespace doesn't exist`;
`variable is a constant`) become diagnostic-text conformance data.

**Status: implemented (P0-C)** — the format generalisation and the first two
sibling domain files have landed on this branch. What exists now:

| File | Rows | Release-tagged rows | Renderer | Consumers wired |
|---|---|---|---|---|
| `rust/tcl-syntax/tests/data/command_resolution_vectors.txt` | 46 | 16 | `tcl_syntax::naming::conformance` (unchanged API besides `want()`/`want_for()`) | pure resolver (newest column), analyser settlement, bytecode VM dispatch, WASM runtime dispatch, real tclsh **matrix** |
| `rust/tcl-syntax/tests/data/variable_resolution_vectors.txt` | 50 | 16 | `tcl_syntax::var_conformance` (`vector_setup` / `vector_call` / `vector_script`) | real tclsh matrix only — the analyser and VM legs are P1 work |
| `rust/tcl-syntax/tests/data/namespace_op_vectors.txt` | 73 | 15 | `tcl_syntax::ns_op_conformance` (same triple) | real tclsh matrix only, as above |

Mechanism:

- `tcl_syntax::release_expectations::PerRelease` parses an expectation
  field that is either one value every release shares or a
  `RANGE=VALUE;RANGE=VALUE` list over the ladder 8.4, 8.5, 8.6, 9.0, 9.1
  (`8.4-8.6=…`, `9.0+=…`). The entries must cover the ladder exactly once,
  and a `;` only separates entries when a range token follows it, so
  expectation values may contain semicolons. Single-value rows parse
  exactly as before.
- `tcl_syntax::vector_ops` holds the row syntax the two new files share:
  pipe-split rows and the `kind(argument)` setup mini-ops, split on
  parenthesis depth so an argument may contain commas and nested
  parentheses.
- `rust/tcl-syntax/tests/support/mod.rs` is the five-binary matrix,
  keyed `TCL_LSP_TCLSH84` … `TCL_LSP_TCLSH91` with PATH fallbacks, which
  verifies the interpreter's reported version matches the name it was
  found under, and skips a missing release loudly. All three suites
  (`command_resolution_conformance`, `variable_resolution_conformance`,
  `namespace_op_conformance`) run every row against every available
  release and assert that release's own column.
- The two new domains' observable is the two-element list
  `<catch code> <result-or-message>`, with the whole row inside the
  `catch`, so a row that uses a subcommand a release does not have states
  that release's real error text (8.4's `bad option "path": …`) rather
  than being excluded. The command domain keeps its `-` sentinel for
  `invalid command name` and gains `!ERROR` for a scenario a release
  cannot set up at all.
- Each file carries a `documented-non-conformance` block naming the
  `knownBug` rows deliberately excluded (`namespace-56.4`, `info-15.8`,
  `interp-27.5–27.8`, `var-3.3`/`var-3.4`'s `testupvar` constraint, and
  `namespace-51.13`'s mid-teardown observation).

All 169 rows were pinned live against 8.4.20, 8.5.19, 8.6.14, 9.0.4, and
9.1b0 — §7.1's five-binary matrix, which landed alongside this work. Three
release deltas the C suites do not state directly fell out of that run and
are now recorded: the creation path's direction inversion at 9.0 (an 8.x
`set v` inside a namespace writes *through* to an existing global, a 9.x
one does not), 8.4's acceptance of `upvar #0 a foo(bar)` that 8.5 turned
into `bad variable name`, and 8.4/8.5's refusal to create a procedure whose
name starts with `:` in a non-global namespace.

### 7.3 The stdlib as executable specification

The pure-Tcl library scripts *are* the reference implementation of the
tiers our resolver mirrors, and they differ per release:

- **`init.tcl`** — `unknown`'s step order (auto_load with the *caller's
  namespace* passed explicitly; interactive-only exec/history/
  abbreviation tiers; the final `TCL LOOKUP COMMAND` errorcode);
  `auto_load`'s candidate list (with the 8.4/8.5 duplicate-try vs 8.6+
  `ni` guard, and `namespace eval ::` vs `namespace inscope ::` — which
  changes behaviour for index entries containing spaces/braces);
  `auto_load_index`'s **back-to-front `auto_path` walk** (earlier
  entries win by overwrite) and its `auto_oldpath` memoisation.
- **`package.tcl`** — `tclPkgUnknown`'s scan order (back-to-front,
  subdirectory `pkgIndex.tcl` files before the directory's own, seen-dir
  memoisation, and the **mid-scan `auto_path` growth re-scan**), plus the
  9.0 `VERSIONCONFLICT` trap and `source -nopkg` deltas.
- **`tm.tcl`** — the module filename pattern, `::`→`/` mapping, root
  construction (`site-tcl` first via prepend-LIFO), env-var paths
  (9.0's `file tildeexpand`), the lowercase-`tcl` probe in 9.x, and the
  precedence rule: **tm paths beat `tclPkgUnknown`** via load-time
  handler chaining.
- **`safe.tcl`** — `RejectExcessColons` as a name-normalisation oracle;
  tokenised `auto_path` in safe children.

Each becomes a pinned behavioural contract on `PackageResolver` and the
autoload tier, per release, tested two ways: hermetically against
committed vectors, and (env-gated) by executing the real library scripts
under the matrix binaries and comparing our resolver's answers.

### 7.4 Real-corpus oracles — stop testing against synthetic indexes only

Today **no test reads a real `pkgIndex.tcl`**: every `PackageResolver`
test writes synthetic indexes, while eight real index files sit unread in
`tmp/tcl9.0.4/library` and all 135 tcllib modules beside them. New
skip-if-absent suites (CI-cheap, corpus-gated like
`differential_segment`):

- **Index-ingest parity**: run `PackageResolver` over
  `tmp/tcl{8.4.20,8.5.19,8.6.16,9.0.4,9.1b0}/library/**` and
  `tmp/tcllib-2.0/modules/**` (and Tk's `library/` once fetched), and
  cross-check the resulting package database against a live tclsh's
  `package names` + `ifneeded` registrations after forcing the same scan
  — per release. The Tcl 9 zipfs `apply`-shape index (today inlined as a
  three-package string literal) is covered by the real file.
- **Autoload parity**: `auto_qualify`/`auto_index` resolution over the
  real `tclIndex` files versus the ported reference implementation.
- **Resolution sweeps**: extend the existing self-consistency corpus
  differentials with *outcome* assertions — the analyser's settled
  resolutions over the tcl/tcllib library trees must be stable across
  the migration (a before/after snapshot gate during P1/P1a, retired
  after).
- **tcltest replay**: extend `tcltest_sweep`'s tier ladder with a
  name-resolution stem set (`namespace`, `namespace-old`, `resolver`,
  `var`, `upvar`, `uplevel`, `trace`, `rename`, `info`, `interp`,
  `package`, `init`, `tm`, `autoMkindex`, `pkgMkIndex`, `unknown`,
  `basic`, `safe`) as a tracked scoreboard column for **both** engines
  (the sweep currently runs VM-only against C), across releases in the
  manual tier. Re-capture `tests/test_reference/` (deleted in the Python
  purge; capture scripts and skill survived intact) for 8.6/9.0 now and
  8.4/8.5 once §7.1 lands — its per-test PASS/FAIL matrix is the
  coverage map that tells us which vectors we have not yet derived.
- **Tk's additive naming domain**: widget commands are a second dynamic
  command space — pathname-named (`.f.b`), implicitly created/destroyed,
  with a parent-must-exist rule enforced by the name and interaction
  with Tcl namespaces (`::ns::.f`). Once trees are fetched:
  `winfo.test`/`window.test` seed a widget-path registration vector set
  (feeding the tk-widget-instance-typing model), `option.test` documents
  the option-database's distinct resolution algorithm, and Tk's real
  `pkgIndex.tcl` + the `tk`/`Tk` co-provide chain is the B11 acceptance
  oracle. Scoped as an additive tier under the Tk epic (#1710); the Tcl
  model is unchanged by it.

### 7.5 The consumer conformance lattice

Every consumer of the §1.2 resolution stack is wired to the shared
vector data — none may pass on a private subset:

| Consumer | command | variable | namespace-op | binding | package | autoload |
|---|---|---|---|---|---|---|
| pure resolver (`tcl-syntax::naming`) | ✅ today | ● | ● | — | — | ● (auto_qualify) |
| analyser settlement (post-walk) | ✅ today | ● | ● | ● | ● (floors) | ● |
| realm layer / `BindingKnowledge` (P1a) | ● | ● | ● | ● **primary** | ● (package states) | ● |
| bytecode VM dispatch | ✅ today | ● (promote the local const) | ● | ● | ● | ● |
| WASM runtime dispatch | ✅ today | ● | ● | ● | ● (real package loop) | ● |
| codegen dispatch proofs (`ProofStatus`) | — | — | — | ● (epoch/rename rows) | — | — |
| `PackageResolver` + floor engine | — | — | — | — | ● **primary** | ● **primary** |
| LSP cross-file leg (`settle_call_against_workspace`) | ● (e2e) | ● | ● | ● | ● (W120/W123) | ● |
| engines behind `tcl-engine-api` | ● | — | — | ● | ● | — |
| real tclsh (the oracle itself) | ✅ today | ● | ● | ● | ● | ● |

(✅ = wired today; ● = wired by this programme; the realm layer and
`PackageResolver` are the *primary* implementations their columns prove.)
The one-oracle gate (§4 R10) enforces the lattice structurally: a
consumer that cannot pass a domain's vectors has no business holding a
private implementation of that domain.

### 7.6 Policy: hermetic vs live vs manual

- **Hermetic (CI, always)**: committed vector files and generated
  corpora (the `package_version_oracle` pattern — generator script,
  hand-maintained `#` header naming exact patchlevels, check =
  output-equals-file), including the extracted vsatisfies tables and
  error-message data.
- **Live matrix (env-gated, skip-loudly)**: `TCL_LSP_TCLSH{84..91}`
  suites re-pinning vectors against real binaries; the stdlib-execution
  parity tests; `dialect_oracle`-style existence probes.
- **Manual tier (`make test-exhaustive`)**: full tcltest replay
  scoreboards, whole-corpus index/resolution sweeps, multi-release
  sweeps. Never wired into CI, per the standing testing-tier policy.
- **Appliance tier (owner-run, checked-in transcripts)**: the BIG-IP
  probe corpus at `scripts/dev/bigip-probes/` with its results
  ([measurements](bigip-irule-parser-measurements.md)) is the iRules
  analogue of the live matrix — re-runnable against a real appliance,
  never CI. Its word-formation and continuation tables become hermetic
  lexer conformance vectors (R1–R7 and N1–N5 rows with the stock-Tcl
  control column, same format as §7.2), and the
  `simonkowallik/irulescan` `tests/bigip/syntax/` torture suite — which
  surfaced the N-rules — is the adoption candidate for regression
  fixtures (licence check before vendoring, per the corpus hygiene
  rules).

  **The hermetic half of this tier has landed** (evidence lane, #15):
  `rust/tcl-registry/src/f5/corpus.rs` holds 205 vectors derived from
  those transcripts — the §4a four-context parity cases, the §4a
  environment-difference table, the sixteen 8.4-vs-8.5 discriminators,
  §4b's 31-command two-class split, the 120-cell event-context matrix,
  and the §6/§8 priority facts — every row citing its measurements
  section and asserted against the model in ordinary `cargo test`. The
  boundary between the two tiers is now explicit: the **appliance tier
  produces transcripts** (owner-run, prefixed, cleanup-proved) and the
  **hermetic tier consumes them**; no test in this repository talks to an
  appliance, and no appliance result reaches the model except through a
  cited corpus row or an `EmbeddedRuntimeEvidence` record. What the
  appliance tier still owes the model is coverage, not machinery: a
  second and third build for the acceptance matrix, a restricted-role
  tmsh column, and the two APL contexts, which the E4 driver records as
  `Unknown` rather than inferring.

### 7.7 Phase placement

§7.1 (interpreter builds, path fixes, Tk fetch) is **P0 work** — it is
the oracle ledger the redesign's P0 already names, made concrete. The
vector-format generalisation and the variable/namespace-op files land
with **P1** (they pin today's behaviour before the model moves); the
binding vectors land with **P1a** as the realm layer's acceptance suite;
package/autoload vectors and the real-corpus index parity land with
**P1a/P1b** (they are the floor engine's and `PackageResolver`'s
acceptance suites); the tcltest scoreboard extension and test-reference
re-capture proceed in parallel from P0. The lattice (§7.5) is complete
when every ● is a passing gate — that table is the "every single
consumer leverages them properly" checklist.

## 8. Phase riders

Additions the audit forces onto the redesign's §8 phases (the phase
structure itself is unchanged):

- **P1** carries the mechanical ledger rows marked P1 (validators,
  literals, hand lists, projections) and the C11 hardcoded-name sweep
  with its gate.
- **P1a** — the realm phase — explicitly includes: the new **package
  transition family** in `state_transition.rs` (behavioural oracle: the
  runtime's package loop), unification of `command_binding` +
  `state_transition` + the workspace index's link vocabulary into
  `BindingKnowledge`, retirement of `head_identity`/`KnownPredicateCtx`/
  the ad-hoc tables, generalising `ProofStatus` to all four hook
  families, the engine contract's environment/policy handle, and the VM
  package machinery decision (B5).
- **P1b** additionally migrates the fuzz findings registry's persisted
  release field and moves the fuzzer to environment-driven pairing
  against the oracle ledger.
- **P2** carries: registry generations, the fail-closed overlay rule,
  `tcl spec check` and `tcl spec build --emit rust` (R7), the full
  upgrade tool (§6), the structural engine parity gate (B9), Studio's
  identity/renderer work (T7/T8), and the `spec-author` refresh (T9).
- **P8** adds the KCS Applies-to regeneration (R9) and this document's
  ledger as the completion checklist: the migration is done when every
  ledger row's retired mechanism no longer exists in the tree.
