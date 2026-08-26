# Registration and resolution: the centralisation contract and retirement ledger

> **Status: PROPOSAL — companion to
> [dialect-and-package-registry-redesign.md](dialect-and-package-registry-redesign.md)
> (revision 2).** That document defines the model (core profiles, packages,
> environments, realms). This one is the end-to-end audit of every
> registration and resolution seam in the workspace — front end, compiler,
> analyser, backends, runtimes and VMs, and all tooling — against that
> model: the two centralised systems every consumer moves onto, the
> complete retirement ledger for the mechanisms they replace, the gap
> rulings the audit forced, and the `tcl spec upgrade` specification that
> discharges the one sanctioned backwards-compatibility obligation.
> Sources: the four-lane audit sweep of 2026-08-26 over
> `claude/tcl-dialect-registry-design-lrzbsn` (compiler/analyser,
> runtime/VM/codegen, tooling/AI/CLI, LSP front end). File:line references
> are from that snapshot.

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

### 1.2 One resolution stack

Five resolution questions, each with exactly one owner:

| # | Question | Single owner | What it replaces |
|---|---|---|---|
| R-a | user-written name → environment | `Environment::resolve(name)` — canonical names + aliases + editor language ids, one function for every ingress (settings, directives, language ids, CLI flags, MCP enums, pack rows, persisted studio sessions) | **six** divergent validators found in the audit: `available_dialects()` membership, `is_known_dialect_name`, the directive's `KNOWN_DIALECTS` match, `resolve_known`, `special_vars::resolve_dialect`, and raw `DialectSet::parse` in `tcl-lsp-db`/completion |
| R-b | document bytes → resolved context | the §5.1 detection chain over environment detection facts, plus overlays and targets; output `(environment, generation, overlay hash, targets, primary)` | `TCL_SOURCE_EXTENSIONS`, per-editor extension tables, content-signature ladders, `LanguageDialect::{Profile,Set}` |
| R-c | command name at a call site → binding | candidate ordering stays `tcl_syntax::naming::command_resolution_candidates` (already the single home, conformance-gated against tclsh); the **`exists` oracle becomes one function**: realm `BindingKnowledge` (`Absent`/`Must`/`May`/`Unknown`) produced by the unified transition state (§4.2 of the redesign) | every consumer-local `exists` oracle: `KnownPredicateCtx`'s unfiltered `command_names()`, `head_identity.rs`, the analyser's ad-hoc alias/rename/delete tables, the LSP's four mixed lookup APIs, unfiltered `registry.get` at ~40 compiler and ~10 LSP sites |
| R-d | `package require` → train, floor, targets | one axis-typed `VersionSet` algebra (differentially tested against `package vsatisfies`) plus **one** floor engine with the assistance/semantic split; `PackageResolver` remains the pkgIndex/tclIndex ingest and joins the floor engine as a source | the `DocumentFloor` ⟷ `package_version_floor` duplication (different source rules today: no `PackAmbient` in one, unconditional-require gating of hosted pins in the other), the three coexisting version comparators (`tcl_dialect`, `tcl_registry::version`, `tcl_pkg::version`) |
| R-e | resolved binding → semantic hook | hook selection requires binding proof (invariant I4); the WASM backend's `ProofStatus` discipline (`Unavailable ≠ permission`; only `NotRequired \| Satisfied` specialise) generalises to lowering, inline-codegen, analyser-hook, and const-fold selection | the `resolve_call(…, DialectSet::empty()) → get()` dialect-blind bypass in the analyser-hook path, lowering hooks, and lsp-db; `side_effects.rs`'s hand-rolled fourth selection rule |

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
  hooks, and lsp-db's per-proc compile.
- Three "known command" oracles disagree: settlement uses the unfiltered
  registry name set, W123 uses profile-filtered resolution,
  W002's known-anywhere uses a hardcoded 11-pack list — so settlement
  believes in commands W123 does not.
- The LSP settlement path holds **zero** references to
  `state_transition.rs`; it uses parallel ad-hoc tables
  (`command_aliases`, `renamed_commands`, `deleted_commands`, offsets)
  plus `head_identity.rs` — a second, weaker, top-level-only binding
  table with 20+ consumers.
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
  WASM-only and per-command). Both verbs are P2 deliverables (§5).
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

### Front end

| # | Retired mechanism | Replacement | Phase |
|---|---|---|---|
| F1 | The four mixed lookup APIs as provider-facing surface (`get`, `get_for_dialect`/`best_visible`, `ProfileQueries::resolve_command`, `DocumentFloor`) | two typed views: assistance `(environment, floors)` and semantic realm `BindingKnowledge`; `get` becomes registry-internal | P1a |
| F2 | `profile_for_dialect` + `registry_for_dialect_profile` (ruling B's hop) and their pin tests | environment registry ingress | P1 |
| F3 | `LanguageDialect::{Profile,Set}` (Set exists only for `tk`) | environment handle | P1 |
| F4 | `tk_loaded` computations, `tk_preview`'s `profile_for_dialect("tk")`, `hosts_tk()` consumers, `TK_PACKAGE` substring activation | "provider `Tk` active" placement query | P1/P3 |
| F5 | Semantic-token's process-wide `f5-irules` registry `OnceLock`; `bigip.rs`'s hardcoded dialect strings | environment-keyed, generation-aware handles | P1 |
| F6 | Environment-blind workspace index symbols | realm/environment-keyed index rows feeding the four-tier known-anywhere model | P1a |
| F7 | `registry_with_overlay`'s silent un-overlaid fallback | fail-closed rebuild-or-error on generation miss | P2 |
| F8 | Salsa `dialect: String` inputs and interned keys; `LexerCfgKey`/`ProcBodyKey` two-field truncation | `(environment id, generation, overlay hash, targets)` keys; grammar-id lexer keys | P1 |
| F9 | `getEffectiveConfig`'s dialect fields, `listDialects`, `setDialect` validators | environment/targets/realm status surface, `listEnvironments`, `Environment::resolve` | P1 |
| F10 | W120 fix-from-whole-file, the package-require code action's name-matching gate | assistance-labelled diagnostics; edits gated on `Must`/`May` declarations and the `PackageResolver` | P1a |
| F11 | `TclVersion::from_dialect` in W123 refinement | target `VersionSet` evaluation with honest `Unknown` on guard straddles | P1b |
| F12 | Hand-written Sublime `_SYNTAX_DIALECT_MAP` (missing `tcl8.6`/`tcl9.1` rows today) | generated projection + drift gate | P1 |

### Compiler / analyser

| # | Retired mechanism | Replacement | Phase |
|---|---|---|---|
| C1 | `DialectSet` + unions, `availability_for_name`, `TK_PROFILE`/`tk()` synthesis, `DIALECT_BITS`/`BIT_ONLY_LABELS` | `VersionSet` declarations + environments; optional internal `FamilySet` fast path | P1 |
| C2 | The six divergent dialect-name validators (incl. `special_vars::resolve_dialect`, raw `DialectSet::parse`) | `Environment::resolve` | P1 |
| C3 | `resolve_call`/`resolve_legacy_call_selection`'s `dialect.is_empty() → get()` bypass and its callers (analyser hooks, lowering hooks, lsp-db) | binding-proof-gated `InvocationSpecId` selection (I4) | P1a |
| C4 | `head_identity.rs` (parallel offset-keyed binding table, 20+ consumers) | realm `BindingKnowledge` | P1a |
| C5 | `KnownPredicateCtx`'s unfiltered `builtin_command_names()` and the settlement-vs-W123 oracle split | the one `exists` oracle (R-c) | P1a |
| C6 | The analyser's ad-hoc alias/rename/delete tables + `indirection.rs`'s bounded link walk as settlement inputs | `state_transition.rs`-fed realm state (which already carries the vocabulary) | P1a |
| C7 | `side_effects.rs`'s hand-rolled spec-selection rule | the single selection primitive | P1 |
| C8 | `CommandTableEffect` (third transition vocabulary) | `CommandBindingTransition` | P1a |
| C9 | Whole-file `package_version_floor` as a semantic input; the `DocumentFloor` duplication | one floor engine, two typed views (R-d) | P1a/P1b |
| C10 | `all_dialect_command_names()`'s hardcoded 11-pack list and its EDA/SpecTcl exclusion policy | the four-tier known-anywhere model, with the exclusion policy restated as explicit tier data | P1 |
| C11 | The ~20 hardcoded command-name match sites (terminal-action sets, `global\|variable\|upvar\|trace`, `set\|incr\|append\|lappend`, oo keywords, `on\|trap`, …) and the hardcoded `tcl8.5\|tcl8.6` profile-name match in the optimiser | registry descriptor data (traits, roles, clause grammars, definer grammars) and core-profile predicates | P1, gated |
| C12 | `RuntimeExprSurface::for_tcl_version` and the duplicated operator/word tables; free-function `binary_bp` | `ExprGrammar` per core profile (precedence, symbolic operators, mathfunc sets, arity, substitution) with `for_profile`-only derivation | P1 |
| C13 | `optimiser`'s and `sccp`'s direct spec reads that bypass trust where they still do | `CommandTrustSnapshot`/`BindingKnowledge` everywhere a fold rewrites | P1a |

### Backends

| # | Retired mechanism | Replacement | Phase |
|---|---|---|---|
| B1 | Duplicated `builtin_command_visible_for_surface`/`profile_admits_registry_builtin` in both engines | one shared availability query over declarations | P1 |
| B2 | Both engines' hardcoded 12-name `UNSAFE` (+ platform) safe-interp lists | `Traits::SAFE_INTERP_HIDDEN` (already 14 specs) queried generically | P1 |
| B3 | VM's hand-typed 37-name mathfunc registration and 27-op `mathops!` macro | derived tables (as the runtime already does) | P1 |
| B4 | `TCL_PATCH_LEVEL`/`"9.0.4"` literals behind `package provide Tcl` and `tcl::build-info` | the pinned core release as provider (§3.2) | P1 |
| B5 | VM `package ifneeded\|forget\|unknown\|prefer` silent no-ops | real handling or honest `Unknown`-widening errors; fuzz-paired with the runtime | P1a |
| B6 | Runtime expr parsing under `dialect = None`; both engines' `for_tcl_version` expr surfaces | core-profile `ExprGrammar` threading | P1 |
| B7 | `tcl-engine-api`'s bare `restrict_commands(&[&str])` with no profile pinning; `SANDBOX_COMMANDS`'s out-of-registry closed world | an environment/policy handle on the engine contract; the sandbox surface as a closed-world environment | P1a |
| B8 | `try_lower_hook`'s proof-free selection; inline/analyser/const-fold hook selection without trust | the `ProofStatus` discipline generalised (I4) | P1a |
| B9 | Runtime-only, regex-shaped command-backing scan; no VM parity gate | structural registration parity for **both** engines against the catalogue, with per-family exclusions (a `Core(jim)`-only command is not a WASM obligation) | P2 |
| B10 | `_registry_data.tcl` (orphaned, 2,086 lines, frozen `DialectSet` subtraction) and the misc unsupported-stub name list | `gen-irule-test-data` output; registry capability predicates | P1 |
| B11 | Debugger's `by_name("tcl9.0")` literal; vm-cli's release-only ingress | `Environment::resolve`, environments incl. non-plain-Tcl | P1 |
| B12 | Fuzzer's three-value `Engine` enum + generator name lists + persisted `TclVersion` findings field | environment-driven engine pairing against the oracle ledger; a findings-registry migration for the persisted release field | P1b/P2 |

### Tooling / AI / editors

| # | Retired mechanism | Replacement | Phase |
|---|---|---|---|
| T1 | `dialect_possible_values()`'s `+ tk` special case; `known_dialect_names()`'s manual chain | environment name+alias enumeration | P1 |
| T2 | Explorer's second resolution with silent `by_name` plain-Tcl fallback | pass the resolved environment handle; unknown names error | P1 |
| T3 | `registry-dump --all-dialects`'s single-`tcl8.6`-registry shortcut | per-family enumeration over the catalogue | P1 |
| T4 | Hardcoded `tcl8.6` defaults (CLI `combined_effective_dialect`, VS Code `contextPack`, clap args) | the configured default environment | P1 |
| T5 | f5-cli's unvalidated `--dialect: String` | validated ingress through `Environment::resolve` | P1 |
| T6 | MCP `dialect_schema`'s 18-name enum; `spectcl_check`'s `availability_mask` bit test | environment enumeration; `targets ⊆ applicable` collision checking | P1/P2 |
| T7 | Studio's `DIALECT_BITS` editor, dialect-string APIs, `SOURCE_DIALECT_KEY`, dialect-as-language-id client | provider/`VersionSet` editing, environment ids, generic contributed LSP identity (B7) | P2 |
| T8 | `render_spectcl`'s `is_dialect_set` conflation of availability with `safe_on_uninit`/`two_arg_optionless_dialects` | distinct spellings per gap ruling R4 | P2 |
| T9 | `spec-author` skill's 1.1 instructions | 2.0 refresh (words, `dialect` blocks, `available`, upgrade workflow) | P2 |
| T10 | `callback-surfaces` `name@dialect+dialect` row ids (and the `.chain(tk())` special case) | environment/provider-keyed ids; one-shot regeneration | P1 |
| T11 | `gen_zed_queries`'s `grammar_union`/`TK_AND_TCL` inputs; `gen_editor_catalogs`/`gen_tmlanguage_keywords`'s `ALL_TCL` bit filters | declaration-derived fast paths | P1 |
| T12 | AI manifest's release-keyed Tk fragment; prompt loader's alias-blind `dialects[]` check | environment-keyed manifest; alias-resolved loading | P1 |
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

## 7. Phase riders

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
