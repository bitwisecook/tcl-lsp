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
a documented P2 seam), and `binding.rs` (`BindingKnowledge`,
`PackageStateMap`, and the `PackageTransition` family as a parallel type
pending P1a realm integration). The equivalence sweeps pass with **zero
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
| B2 | Both engines' hardcoded 12-name `UNSAFE` (+ platform) safe-interp lists | `Traits::SAFE_INTERP_HIDDEN` (already 14 specs) queried generically | **done** (command half) — `tcl_registry::safe_interp_hidden_commands()` is the one generic query; both `make_safe`s call it and narrow by what the interpreter carries. The `UNSAFE_PLATFORM` scrub stays a name list under `TODO(ledger B2-platform)`: `special_vars` models `tcl_platform`'s keys but has no "scrubbed when made safe" flag, so driving it needs a `SpecialVarKey` field, not a query. See the measured-evidence subsection below. |
| B3 | VM's hand-typed 37-name mathfunc registration and 27-op `mathops!` macro | derived tables (as the runtime already does) | **done** — the VM registers `tcl_syntax::expr::mathfunc::all()` and every `mathop_shape` spelling from `expr::operators`, each behind one fn pointer that reads the op off the invoked word. This added the 21-name TIP 745 batch to the VM (listed below); the bodies already existed in `dispatch_with_backend`, only the command bindings were missing. |
| B4 | `TCL_PATCH_LEVEL`/`"9.0.4"` literals behind `package provide Tcl` and `tcl::build-info` | the pinned core release as provider (§3.2) | **done** — `TclVersion::core_provided_packages()` and `tcl_dialect::build_info` are the single tables; both engines re-derive their pre-provided core packages on every profile pin and compose `::tcl::build-info` from `TclVersion::patchlevel()`. `patchlevel()`'s 9.1 entry is now the measured `9.1b0`. |
| B5 | VM `package ifneeded\|forget\|unknown\|prefer` silent no-ops | real handling or honest `Unknown`-widening errors; fuzz-paired with the runtime | P1a |
| B6 | Runtime expr parsing under `dialect = None`; both engines' `for_tcl_version` expr surfaces | core-profile `ExprGrammar` threading | P1 |
| B7 | `tcl-engine-api`'s bare `restrict_commands(&[&str])` with no profile pinning; `SANDBOX_COMMANDS`'s out-of-registry closed world | an environment/policy handle on the engine contract; the sandbox surface as a closed-world environment | P1a |
| B8 | `try_lower_hook`'s proof-free selection; inline/analyser/const-fold hook selection without trust | the `ProofStatus` discipline generalised (I4) | P1a |
| B9 | Runtime-only, regex-shaped command-backing scan; no VM parity gate | structural registration parity for **both** engines against the catalogue, with per-family exclusions (a `Core(jim)`-only command is not a WASM obligation) | P2 |
| B10 | `_registry_data.tcl` (orphaned, 2,086 lines, frozen `DialectSet` subtraction) and the misc unsupported-stub name list | `gen-irule-test-data` output; registry capability predicates | P1 |
| B11 | Debugger's `by_name("tcl9.0")` literal; vm-cli's release-only ingress | `Environment::resolve`, environments incl. non-plain-Tcl | P1 |
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
  covered.

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
