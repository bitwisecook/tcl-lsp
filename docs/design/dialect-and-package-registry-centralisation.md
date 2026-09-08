# Registration and resolution: the centralisation contract and retirement ledger

> Companion to
> [dialect-and-package-registry-redesign.md](dialect-and-package-registry-redesign.md),
> which defines the model (core profiles, packages, environments, realms).
> This document is the registration/resolution contract every consumer is
> held to, the ledger of retired mechanisms that still exist in the tree,
> the gap rulings, the `tcl spec upgrade` specification, and the
> name-resolution oracle programme. Every open row here is repeated in the
> redesign's §11.

The constraint: **the centralised system is the only system.** Retired
mechanisms are deleted — no shims, no wrappers, no parallel tables. The
sole backwards-compatibility exception is SpecTcl: every published 1.x
pack keeps loading, and `tcl spec upgrade` rewrites 1.x sources to the
newest vocabulary (§6).

## 1. The two centralised systems

### 1.1 One registration pipeline

All command, dialect, package, environment, and variable knowledge enters
through **one loader** and lives in **one catalogue**. For `.tclspec`
sources "one loader" is literal: `tcl_spectcl::evaluate_pack` (cached
behind `tcl_spectcl::evaluate_pack_cached`) is the only door.

```text
sources                       ingestion                 catalogue (per generation)
──────────────────────────    ─────────────────────     ──────────────────────────
SpecTcl dialect blocks     ─┐                           CoreProfiles (family × release × build)
SpecTcl package packs      ─┤   one loader              EnvironmentDefinitions (+ overlays)
  (bundled | user |         ├─► (vocab-classified,  ──► SurfaceDeclarations (provider,
   workspace | studio)     ─┤    trust-stamped)          VersionSet, predicate, provenance)
native core specs          ─┤                            special-variable declarations
inline / sidecar stubs     ─┘                            detection facts, aliases, policies
                                        │
                     ┌──────────────────┼──────────────────────┐
              per-context registries    │            derived projections
              (environment, overlay,    │     editors, AI catalogues, docs,
               generation)-keyed        │     engine availability gates, fuzz
                                        │     oracles, TMM simulator data
```

- **Every source is provenance- and trust-stamped at ingestion** (redesign
  §6.4). Inline/sidecar stubs ingest as `SurfaceDeclaration`s
  (`tcl_registry::model::declaration`, read through the one
  `DocumentCommandSurface` door), so one query path serves every spec
  source.
- **Every projection is generated, none hand-maintained**: the editor
  catalogues, AI prompt manifests, engine gates, and docs tables derive
  from the catalogue behind `--check` drift gates. Two hand-maintained
  projections remain (ledger rows T13 and B10).
- **Dynamic data should be generation-owned**; loaded packs are still
  leaked `&'static` (redesign §11 D10).

The registry on the model lives in `rust/tcl-registry/src/model/`:
`surface.rs` (`SurfaceDeclaration` and `declarations_for_spec`, the
mechanical translation of every compiled spec's rows), `context.rs`
(`ResolvedContext`, `FloorMap`, `KeyedVersions`, and the `ContextQueries`
assistance view), `assembly.rs` (`ContextRegistry` — `Arc`-owned
per-context registry generations, provider-filtered from the same spec
sources, cached by `(environment identity, keyed-versions hash)`),
`binding.rs` (`BindingKnowledge`, `PackageStateMap`, the
`PackageTransition` family), `registration.rs` (environment registration
at the next generation under the trust lattice), `ingress.rs` (the one
name seam), `semantic.rs` (`SemanticContext`, the `Copy` generation-bound
handle the executable IR is keyed on), and `tcllib.rs` (the 200-row
per-module identity census). The parity sweeps hold the model to the
catalogue's answers: per-spec visibility agrees with
`ProfileQueries::is_available` for every compiled spec × catalogue
profile, and per-environment visible names and per-name resolution
reproduce `registry_for_profile` + `best_visible` exactly.

### 1.2 One resolution stack

Five resolution questions, each with exactly one owner:

| # | Question | Single owner | What remains beside it |
|---|---|---|---|
| R-a | user-written name → environment | `tcl_registry::model::ingress::resolve_environment` — canonical names + aliases + editor language ids, one function for every ingress (settings, directives, language ids, CLI flags, MCP enums, pack rows, persisted studio sessions) | — |
| R-b | document bytes → resolved context | the detection chain over environment detection facts, plus overlays and targets; output `(environment, generation, overlay hash, targets, primary)` | the salsa `dialect: String` input shape (F8) |
| R-c | command name at a call site → binding | candidate ordering is `tcl_syntax::naming::command_resolution_candidates` (conformance-gated against tclsh); the **`exists` oracle is one function**: `Analyser::command_existence_oracle` / `command_binding_knowledge` answer `Absent`/`Must`/`May`/`Unknown` per program point (W123 is the `Absent` verdict); the registry tier is the one context-filtered set (`builtin_command_names`) shared by settlement, const-dispatch, W113 and W123; head-identity consumers read the realm state's `knowledge_at` (`tcl_compiler::realm`) | unfiltered `registry.get` reads at ~40 compiler and ~10 LSP sites, for spec *content* rather than existence (F1) |
| R-d | `package require` → train, floor, targets | one axis-typed `VersionSet` algebra (differentially tested against `package vsatisfies`) plus one floor engine with the assistance/semantic split; `PackageResolver` is the pkgIndex/tclIndex ingest and a source to it | `package_version_floor` and `DocumentFloor` are two copies (C9); three version comparators coexist (redesign §11 D18, R6) |
| R-e | resolved binding → semantic hook | hook selection requires binding proof (invariant I4): with a context carried, the head must resolve to a spec's declaration under the document's environment (`ResolvedContext::resolve_spec`) before any analyser hook, lowering hook, type-infer spec fact, or side-effect hint is selected; `Absent` ⇒ no selection; no context ⇒ `NotRequired`, and every deliberately context-less reader is a documented widening query | the trust conjunct — `CommandTrustSnapshot` where a fold rewrites (C13) |

The split the redesign's §5.2 mandates is enforced by type: **assistance
queries** (completion, hover, annotations, W120) take `(environment,
floors)`; **semantic queries** (diagnostics that assert, code actions
that edit, taint, lowering, codegen) take realm `BindingKnowledge` at a
program point. A semantic consumer cannot call the assistance API —
different names, different types (invariant I3).

## 2. The consumers, per stage

### 2.1 Front end (LSP server, lsp-core, lsp-db)

`tcl-lsp-core`, `tcl-lsp-db` and `tcl-lsp-server` resolve every
dialect-name ingress through `tcl_registry::model::ingress`:
`profile_for_dialect` / `optional_profile_for_dialect` /
`registry_for_dialect_profile`, `environment_for_dialect`,
`stated_profile_for_dialect`, `context_for_dialect` (the generation) and
`document_context_for_dialect` (the assistance view); the salsa
`registry` / `registry_with_overlay` doors, `Backend::registry_for_dialect`,
the editor language-id ingress, and both configuration validators go
through it. Availability, option, subcommand, keyed-range and
placement-floor questions are answered by `ResolvedContext` under the
**document authoring point** (`DocumentEnvironment::document_authoring_mask`).
The Tk surface is one placement query — `ambient_package("Tk")` is the
Tk-checks activation fact and W120's silence; `can_host_package("Tk")`
is the placement's existence; `package_active("Tk")` is availability
under the world policy — so any environment that declares Tk ambient
activates the Tk checks without being spelled `tk`. The workspace index
is environment-blind (F6).

### 2.2 Compiler and analyser

`tcl-compiler` resolves every dialect-name ingress through one
`environment_ingress` module (the analyser's `analyse*` entries, the
per-item path, incremental re-segmentation, and
`CompilationUnit::build_for_dialect`), obtains registries as
per-environment `ContextRegistry` generations, and answers availability,
keyed version ranges, and profile-pin/pack-ambient floors through
`ResolvedContext`. The three model selection primitives read their
context (invariant I4, R-e). The analyser's `exists` bookkeeping is the
model's `BindingKnowledge` (R-c); the §4b iRules interpreter-present
extension and the closed-world policy are policy over the one oracle.
The side-effect hint walk stays inside the primitive because
proved-single-winner selection is measured non-equivalent at nine points
(`next` under `bpf`; `exit`/`send`/`close` under `expect`; `option` and
four of its subcommands under `spectcl`), pinned by
`c7_hint_walk_counterexamples`, which fails the day the catalogue moves
those hints onto the winning specs.

### 2.3 Backends (runtime, VM, codegen, BPF)

Both engines (`tcl-vm`, `runtime/rust`), the engine adapter
(`tcl-engine-tclvm`), the two VM-driving hosts (`tcl-vm-cli`,
`tcl-debugger`) and the `xtask` sweeps and generators that exercise them
resolve every dialect **name** through the seam and reach the registry
as per-environment `ContextRegistry` generations, through one small
`environment` module each: `profile_for_dialect`
(`resolve_environment(…).unit_profile()`), `store_for_profile` /
`store_for_dialect` (the generation's command store), and `surface_mask`
(the resolved environment's document authoring point). Both engines
resolve the point *and* the store once per profile pin and cache them on
the interpreter, because the builtin-surface gate is consulted on every
command resolution. `codegen_abi`'s three raw name ingresses are
`resolve_known_environment`, keeping the fail-closed decline. The only
names these crates accept are the closed release set
`TclVersion::dialect_profile_name` spells plus the fixed
`f5-irules`/`tk`/`expect`/`f5-iapps` projection targets.

The WASM backend's `BackendRegistry`/`ProofStatus` and `try_bytecoded`'s
trust gate implement I4's discipline; BPF is the cleanest fully
registry-derived backend (one prefix-heuristic residue, self-documented as
a missing Thread pack). The runtime's per-interp `PackageState`, with a
real `ifneeded` → `unknown` → retry loop, is the behavioural oracle for
the package transition family.

### 2.4 Tooling, AI, editors

The two CLIs (`tcl-cli`, through the shared `tcl-cli-support`), the MCP
server (`tcl-mcp`), the spec studio (`tcl-spec-studio`) and the pack
loader's name ingress (`tcl-spectcl`) resolve every dialect **name**
through the seam and answer availability from the resolved environment's
`ResolvedContext`, each through one small `environment` module —
`profile_for_dialect`, `known_profile_for_dialect` (the validator),
`context_for_dialect` (the assistance view), `store_for_dialect` (the
generation's command store) — plus, where a crate genuinely needs both
ingress forms, the exact analyser-profile twin
(`analyser_profile_for_dialect` / `analyser_mask_for_dialect`, built on
`DocumentEnvironment::analyser_profile`): the CLI's KCS help filter and
the pack-carrying registry cache key deliberately sink `tk` to the
permissive fallback rather than promoting it. One correctness note for
every seam consumer: `resolve_known_environment("tcl")` is `Some` (the
lenient sink's own environment id), so a caller needing the old refusal
of a bare `tcl` composes the catalogue twin (`DialectProfile::find`) plus
the `tk` promotion instead (`tcl-explorer/src/environment.rs`, pinned by
`the_ingress_forms_differ_only_at_tk`).

`ProfileQueries` is `pub(crate)` to `tcl-registry`: `tcl lookup` and the
MCP `command_info` answer `resolve_command` / `available_option_names` /
`keyed_version_range` from the document context, and `spectcl_check`'s
collision test threads the target dialect's mask through the seam.
`gen_ai`'s vendor-surface summary reads
`ResolvedContext::vendor_command_surface` (pinned equal to
`ProfileQueries::vendor_surface` for every catalogue profile);
`gen_zed_queries`' ambient-package filter threads each target's resolved
context and asks `placement_is_ambient`; every generator `--check` is
byte-identical.

`tcl spec` has three verbs — `import`, `upgrade`, `export`; `spectcl_check`
exists only as the MCP tool (redesign §11 D18, R7). What deliberately
stays are the `DialectProfile::all()` **enumerations** — the CLI's
`--dialect` possible values and its unknown-dialect message, the MCP
`dialect_schema` enum, the studio's picker, `registry-dump
--all-dialects`' Tcl-release list — because the environment list has
different contents and no `short_name`; those are the payload rows T1 /
T3 / T6 / T7, a user-visible change rather than a refactor.

## 3. The retirement ledger

Every row: a mechanism that still exists in the tree → its replacement.
**F** front end, **C** compiler/analyser, **B** backends, **T** tooling.
A retired mechanism may not be wrapped or kept alongside its replacement.
The retired mechanisms that no longer exist are held at zero references
by `cargo xtask retired-api-gate` (in `make xtask-check`): the
dialect-name validators (`by_name`, `by_opt_name`, `resolve_known`,
`availability_for_name`, `special_vars::resolve_dialect`), the
string-keyed registry doors (`registry_for_dialect`,
`registry_handle_for_dialect`), `DialectSet`, `head_identity`,
`StubOverlay`, `command_table_effect`, the `detect_rename` /
`detect_interp_alias` family, and the CST pack-loader front end. The
gate's escape hatch is `// retired-api-ok: <reason>` with a row here; its
own tests prove it fails on a seeded violation of every retired family.

The completion criterion: the migration is done when every row below is
gone. Most of the remaining rows are the *payload* halves — user-visible
enumerations, enums and row ids — held because re-keying them is a
user-facing change (redesign §11 D15).

### Front end

| # | Still in the tree | Replacement |
|---|---|---|
| F1 | `get` / `get_for_dialect` as provider-facing surface at the LSP's ~10 direct readers, for spec *content* | the two typed views: assistance `(environment, floors)` and semantic realm `BindingKnowledge`; `get` registry-internal |
| F6 | Environment-blind workspace index symbols (no dialect/environment field on any symbol), so cross-file arity, W123 suppression, and cross-document definition let an `f5-irules` proc satisfy a `tcl9.0` call | realm/environment-keyed index rows feeding the four-tier known-anywhere model |
| F7 | `DocumentEnvironment::context_registry`'s silent un-overlaid fallback on a pack-overlay cache miss | fail-closed rebuild-or-error on a generation miss |
| F8 | The `dialect: String` shape of the salsa `SourceFile` input (every read resolves through the seam; `LexerCfgKey` and `ProcBodyKey` already intern the resolved environment id and derive the whole `LexerConfig` from it) | `(environment id, generation, overlay hash, targets)` keys |
| F9 | `listDialects` enumerating `DialectProfile::all()` (all four validators — `folderDialects`, folder `tclLsp.dialect`, `setDialect`, `setSessionDialectOverride` — are one `resolve_environment`) | `listEnvironments` over the environment registry |
| F10 | W120's fix-from-whole-file and the package-require code action's name-matching gate | assistance-labelled diagnostics; edits gated on `Must`/`May` declarations and the `PackageResolver` |
| F11 | `TclVersion::from_dialect` in W123 refinement (`tcl-lsp-server/src/lib.rs`) | target `VersionSet` evaluation with honest `Unknown` on guard straddles |
| F12 | The hand-written Sublime `_SYNTAX_DIALECT_MAP` (`editors/sublime-text/plugin.py`) | a generated projection plus a drift gate |

### Compiler / analyser

| # | Still in the tree | Replacement |
|---|---|---|
| C1 | The interned `DialectProfile` the *lexer* is keyed on (`LexerConfig::for_dialect`), with `PLAIN_TCL` and `TK_PROFILE` as its two off-catalogue rows | a `LexerGrammar` on the environment definition — redesign §11 D5 |
| C6 | The analyser's `command_aliases` / `renamed_commands` / `deleted_commands` and offset maps — populated from `CommandBindingTransition` facts, but `AnalysisResult`-shaped, walked in that form by `indirection.rs` | `state_transition.rs`-typed realm state — redesign §11 D9 |
| C7 | `side_effect_hints_in_context`'s availability-filtered newest-first-with-hints walk inside the selection primitive | proved-single-winner selection, once the catalogue moves the nine measured hints onto the winning specs |
| C9 | `package_version_floor` and `DocumentFloor` as two copies of the floor engine (the range verdicts already sit beside them: the version-gate flush asks the same `axis_floor` for the primary verdict and, only when that is satisfied, the declared-target sets for the W150 remainder) | one floor engine, two typed views (R-d) |
| C10 | `all_dialect_command_names()`'s hardcoded pack list and its EDA/SpecTcl exclusion policy (W002's known-anywhere source) | the four-tier known-anywhere model, with the exclusion policy as explicit tier data |
| C11 | The ~20 hardcoded command-name match sites (terminal-action sets, `global\|variable\|upvar\|trace`, `set\|incr\|append\|lappend`, oo keywords, `on\|trap`, …) and the hardcoded `tcl8.5\|tcl8.6` profile-name match in the optimiser | registry descriptor data (traits, roles, clause grammars, definer grammars) and core-profile predicates |
| C12 | `RuntimeExprSurface::for_tcl_version`; the free-function `binary_bp` in `rust/tcl-syntax/src/expr/parser.rs` keyed on operator text with no dialect parameter; `tcl-syntax/src/expr/mathfunc.rs` keyed on `TclVersion` floors (the shape that would offer `min`/`max`/`entier`/`bool`/`isqrt` under Jim). The **data** side is complete: `ExprGrammar` carries the whole release-keyed binding-power table, the release each word and symbolic operator arrives at, and the mathfunc set with its per-function build gate | `ExprGrammar` per core profile, `for_profile`-only derivation |
| C13 | `optimiser`'s and `sccp`'s direct spec reads that bypass trust where a fold rewrites | `CommandTrustSnapshot` / `BindingKnowledge` everywhere a fold rewrites |

### Backends

| # | Still in the tree | Replacement |
|---|---|---|
| B1 | Duplicated `builtin_command_visible_for_surface` / `profile_admits_registry_builtin` in both engines (each reads a per-environment generation and its document authoring point; the two bodies differ because the engines carry different command tables and different "unknown to the registry" rules) | one shared availability query over declarations — an engine-contract change |
| B2 | The `UNSAFE_PLATFORM` scrub name list in both `make_safe`s (`TODO(ledger B2-platform)`): `special_vars` models `tcl_platform`'s keys but has no "scrubbed when made safe" flag. The command half is done: `tcl_registry::safe_interp_hidden_commands()` is the one query, narrowed by what the interpreter carries | a `SpecialVarKey` field |
| B5 | VM `package ifneeded\|forget\|unknown\|prefer` silent no-ops | real handling or honest `Unknown`-widening errors; fuzz-paired with the runtime |
| B6 | Runtime expr parsing under `dialect = None`; both engines' `RuntimeExprSurface::for_tcl_version` (keyed by `TclVersion`, family-blind) | core-profile `ExprGrammar` threading |
| B7 | `tcl-engine-api`'s bare `restrict_commands(&[&str])` with no profile pinning; `SANDBOX_COMMANDS`'s out-of-registry closed world in `tcl-spec-hooks` | an environment/policy handle on the engine contract; the sandbox surface as a closed-world environment |
| B9 | The runtime-only, regex-shaped command-backing scan; no VM parity gate | structural registration parity for **both** engines against the catalogue, with per-family exclusions (a `Core(jim)`-only command is not a WASM obligation) |
| B10 | `_registry_data.tcl` (orphaned, 2,086 lines, a frozen "tcl8.4 minus f5-irules" subtraction) bundled by `tcl-irule-test` (the replacement generator, `gen-irule-test-data`, already resolves through the seam and filters with `ResolvedContext::resolve_spec`) | `gen-irule-test-data` output |
| B11 | The debugger's and vm-cli's plain-Tcl-only acceptance (a wider `--tcl-version` set in vm-cli, a dialect input the DAP surface lacks; both literals resolve through the seam and are marked `// P1:`) | environments incl. non-plain-Tcl |
| B12 | The fuzzer's three-value `Engine` enum + generator name lists + persisted `TclVersion` findings field | environment-driven engine pairing against the oracle ledger; a findings-registry migration for the persisted release field |

**Measured per release** (the reference interpreters on `PATH`; the
engines' answers derive from these tables):

Safe-interp hidden sets (`interp create -safe s; lsort [interp hidden s]`,
top-level names only):

| release | hidden set |
|---|---|
| 8.4.20 | `cd encoding exec exit fconfigure file glob load open pwd socket source` (12) |
| 8.5.19 | + `unload` (13) |
| 8.6 | + `unload` (13) |
| 9.0.4 | + `unload zipfs` (14) |
| 9.1b0 | + `unload zipfs clock` (15) |

The registry trait `Traits::SAFE_INTERP_HIDDEN`'s 14 specs are exactly the
9.0 set; `unload` and `zipfs` are release-gated commands, so "hide what
the trait names, if this interpreter carries it" reproduces every row.
`clock` is deliberately not given the trait: 9.1 hides the C `clock` and
immediately re-provides a safe one, so `s eval {clock format 0 -gmt 1}`
succeeds inside a 9.1 safe child. Neither engine implements `load`,
`unload`, `socket`, or `zipfs` as commands, so their hidden sets are the
8.4 row on every pin; both engines' tests assert the residue is absent
from `info commands`.

Core provides (`package provide <name>` in a fresh `tclsh`):

| release | `Tcl` | `tcl` | `TclOO` | `tcl::oo` |
|---|---|---|---|---|
| 8.4.20 | `8.4` | — | — | — |
| 8.5.19 | `8.5.19` | — | — | — |
| 8.6 | `8.6.x` | — | `1.1.0` | — |
| 9.0.4 | `9.0.4` | `9.0.4` | `1.3.1` | `1.3.1` |
| 9.1b0 | `9.1b0` | `9.1b0` | `1.3.1` | `1.3.1` |

`TclVersion::core_provided_packages()` and `tcl_dialect::build_info` are
the single tables; both engines re-derive their pre-provided core packages
on every profile pin and compose `::tcl::build-info` from
`TclVersion::patchlevel()`. `tcl::tommath` / `zlib` / `tcl::zlib` are
deliberately *not* claimed: neither engine implements those surfaces.
`package require Tcl 8.5` (`[8.5, 9)`) fails under a 9.x pin, matching
`tclsh9.0`. The VM registers `tcl_syntax::expr::mathfunc::all()` and
every `mathop_shape` spelling from `expr::operators`, so the TIP 745
batch (`acosh asinh atanh cbrt copysign dim erf erfc exp2 expm1 fma gamma
ldexp lgamma log1p log2 logb nextafter remainder signbit trunc`) is bound
under every pin.

### Tooling / AI / editors

| # | Still in the tree | Replacement |
|---|---|---|
| T1 | `dialect_possible_values()`'s enumeration and the unknown-dialect message (the ingest validator is `resolve_known_environment`) | environment name+alias enumeration |
| T3 | `registry-dump --all-dialects`'s single-`tcl8.6`-registry shortcut and its `const_fold_version` family list | per-family enumeration over the catalogue |
| T4 | Hardcoded `tcl8.6` defaults (CLI `combined_effective_dialect`, VS Code `contextPack`, clap args), marked `// T4:` at their sites | the configured default environment |
| T6 | MCP `dialect_schema`'s enum; `spectcl_check`'s `availability_mask` bit test (it takes the threaded target's mask through the seam) | environment enumeration; `targets ⊆ applicable` collision checking |
| T7 | Studio's `DIALECT_BITS` editor, dialect-string APIs, `SOURCE_DIALECT_KEY`, and the dialect-as-language-id client (document close/reopen on change) | provider/`VersionSet` editing, environment ids, a generic contributed LSP identity |
| T8 | `render_spectcl`'s `is_dialect_set` conflation of availability with `safe_on_uninit` (ruling R4) | distinct spellings |
| T10 | `callback-surfaces` `name@dialect+dialect` row ids (and the `.chain(tk())` special case); re-keying regenerates the committed JSON | environment/provider-keyed ids |
| T12 | The AI manifest's release-keyed Tk fragment and the runtime prompt loader's alias-blind `dialects[]` check | an environment-keyed manifest; alias-resolved loading |
| T13 | The two hand-maintained projections (the Sublime map, the orphaned simulator data) | generated + drift-gated |

## 4. Gap rulings

- **R1 — stubs are declarations.** Inline `# tcl-lsp: stub` and sidecar
  `.stubs` ingest as `SurfaceDeclaration`s
  (`tcl_registry::model::declaration`): `DeclaredCommand` (registry
  `ArgRole` arguments plus a `SurfaceDeclaration` whose provider is
  `Provider::Document`, applicable over `VersionAxisId::document()`, with
  `Provenance::Document` for a buffer and `Provenance::WorkspaceUntrusted`
  for a sidecar), `DeclaredSurface` (the per-document generation) and
  `DocumentCommandSurface` (the one door). Cache invalidation rides the
  document's own text and lsp-db's `sidecar_stubs_epoch` input. The role
  lookup **unions** the catalogue's answer with the document's: an
  untrusted addition may improve assistance and can never weaken a
  shipped analysis fact.
- **R2 — the variable axis is part of the model.** Special variables are
  family/build-sensitive (Jim's `env`, picol 2's capital-initial
  globals); `special_vars.rs`'s table should become declarations
  authorable in SpecTcl `dialect`/package blocks, and `dynamic_names`'s
  three flow-insensitive bools realm variable-domain widening. **Open**
  (redesign §11 D18).
- **R3 — `FILE_SCOPED_ENVS` becomes a detection-scoped surface.** The
  `tclpkg.tcl` whole-file command environment should be an
  environment-with-detection-facts whose surface is a pack, not a
  hardcoded Rust table. **Open.**
- **R4 — non-availability surface fields do not translate to
  `available`.** `safe_on_uninit` and `two_arg_optionless_surface` are
  behaviour predicates keyed by core profile and need their own 2.0
  spellings; the renderer still conflates them with availability (T8).
  **Open.**
- **R5 — hook `ctx` keys gain, not lose, a spelling.** Pack hook bodies
  read `dict get $ctx dialect`; an `environment` key should be added with
  `dialect` kept as a documented alias. **Open.**
- **R6 — `tclpkg.tcl` grows a targets notion; MVS stays MVS.** The
  `# tcl-lsp: supports NAME RANGE` directive declares analysis targets
  (redesign §5.4) while `require` stays a bare MVS floor. The manifest's
  `tcl` constraint has no multi-clause range grammar and the three
  version comparators have not collapsed onto the oracle-pinned
  `tcl_dialect` algebra. **Half open.**
- **R7 — `tcl spec check`.** The MCP-only checker is not promoted to a
  CLI verb. **Open.**
- **R8 — per-family gate scoping.** The command-backing obligation is
  scoped per family and backend: provider `tcl` at 9.0 for the WASM
  runtime, with `Core(jim)`-only commands excluded rather than becoming
  phantom obligations. B9's structural parity gate would carry the
  scoping. **Open with B9.**
- **R9 — docs vocabulary follows.** The KCS "Applies-to" controlled
  vocabulary should consume environment names; it waits on the payload
  rows (T1/T3/T6/T7). **Open.**
- **R10 — the one-oracle gate** (`rust/xtask/src/retired_api_gate.rs`).
  No consumer constructs its own command-existence oracle, availability
  rule, or binding table. Visibility where that is enough:
  `Analyser::builtin_command_names` and
  `model::declaration::DeclaredSurface::get` are `pub(crate)`, beside the
  narrowed cache doors and `ProfileQueries`. A call-site sweep for the
  doors that cannot be narrowed: the `OWNED` pattern family names each
  centralised **answer** and the repository-relative path prefixes whose
  files may write it — `CommandExistenceOracle` /
  `command_existence_oracle` / `builtin_command_names` /
  `w123_registry_known_names` (owned by `rust/tcl-compiler/src/analyser/`),
  `has_command_in_this_dialect` / `all_dialect_command_names` (owned by
  `rust/tcl-registry/src/` and the compiler's constant folder),
  `command_binding_transitions` / `command_table_transitions`, and
  `DeclaredSurface`. A use elsewhere fails the gate; the escape hatch is a
  `// one-oracle-ok: <reason>` waiver **plus** a row in §3 — a marker
  deliberately distinct from `retired-api-ok:`. Full `pub(crate)` on
  `CommandRegistry::command_names` / `get_for_dialect` is not achievable:
  ~45 production call sites read them for spec *content*, so the sweep
  carries that half, keyed on the answers rather than the doors.
- **R11 — F5 rows are evidence-generated.** Every F5 grammar / command /
  variable / package / policy record is keyed by `BigIpExecutionContext`
  and backed by `EmbeddedRuntimeEvidence` from a checked-in conformance
  corpus (redesign §0.2). Rows are *asserted* from the corpus, not yet
  *generated* from it; the transcript-schema validator is unwritten; the
  tmsh role-visibility and `tcl_platform` CMP overlays are recorded as
  evidence but not wired (redesign §11 V9, V10).
- **R12 — one relation mechanism, and the security floor under it.**
  `tcl-registry::relation` is the one core for "X requires / conflicts
  with Y": `Relation<T>` over a `RelationTermKind`, judged by one
  `evaluate` against any `RelationFactSource`, with `closure_over` for the
  transitive case; `OptionTerm`/`OptionFacts` and
  `ProfileTerm`/`ProfileFacts` are its two domains. The design content is
  `RelationMode`: `-command` requires `-channel` **asserts**; `HTTP`
  requires `TCP` **infers**, because BIG-IP attaches the parent itself and
  a configuration naming only `HTTP` is not missing anything — the mode is
  per edge. `EventRequires` keeps its compact record (461 literals on the
  hover and completion path); its profile half reads through the shared
  fact source, and the R12 gate names the exemption. Invariant **I6**
  rides with it: `tcl-registry::security_floor` keeps an override from
  deleting a shipped relation or clearing `TAINT_SINK`, on every override
  from every tier.

## 5. Gates that prove the centralisation

| Gate | Proves | Invariant |
|---|---|---|
| Retired-API gate (`cargo xtask retired-api-gate`, in `make xtask-check`) | the retired mechanisms never reappear under their old spellings; escape hatch requires a ledger entry | I3–I5-adjacent |
| One-oracle sweep (R10, the same gate's `OWNED` family) | no parallel existence/availability/binding mechanisms reappear | I3–I5 |
| Parity sweeps (`per_spec_visibility_matches_the_old_model_for_every_profile`, `spec_queries_reproduce_profile_queries_for_every_profile`) | the model reproduces the catalogue's answers; closed-world has one predicate | — |
| Safe-interp trait conformance (both engines' `make_safe` tests) | engines hide exactly `Traits::SAFE_INTERP_HIDDEN` narrowed by what they carry | I1 |
| Core-provider version tests | `package provide Tcl` ≡ pinned release in both engines | I1 |
| `every_route_to_a_documents_grammar_agrees` and the `dialect-drift` gate | a document has one grammar | I1 |
| Upgrade equivalence (`tcl spec upgrade --verify`, §6) | a 1.x pack and its upgraded form load to byte-identical registry snapshots | I10 |
| Golden pack snapshots (`cargo test -p tcl-spectcl --test golden_packs`, regenerated by `cargo xtask pack-goldens`) | a loader change cannot silently alter what a shipped `.tclspec` means: all 24 shipped packs load to their checked-in snapshots or the diff is written down | I10 |
| Static fast-path gate (`tcl-spectcl/tests/eval_loader.rs`) | the loader's shortcut past the interpreter is an *optimisation*: all 24 shipped packs load byte-identically with `static_fast_path` on and off | I10 |
| Security floor (`tcl-spectcl/tests/i6_security_floor.rs`) | an override cannot weaken a shipped security fact | I6 |
| F5 conformance corpus (`rust/tcl-registry/src/f5/corpus.rs`) | every measured F5 row keeps agreeing, or diverging exactly where recorded | I1 |

Not built: a generation-drop test (I7, redesign §11 D10), a fail-closed
overlay test (F7), a structural engine parity gate (B9), and drift gates
for the two hand-maintained projections (T13).

## 6. `tcl spec upgrade`: the 1.x → 2.0 specification

The one sanctioned backwards-compatibility surface. The loader reads every
1.x pack forever (redesign §6.1); this tool rewrites 1.x sources to 2.0.
The rewriter is `tcl-spectcl/src/upgrade.rs`, driven by `tcl spec upgrade`
with `--from` / `--to` / `--check` / `--verify` / `--restyle`; the 2.0 word
set it targets is `tcl-spectcl/src/loader/available.rs`.

1. **It is a source rewriter, never a load-render round-trip.** Edits are
   content-range replacements located by the loader's own lexer (the
   `speclib_version_span` discipline: same lexing, same BOM handling,
   applied back-to-front, never reformatting), so author layout, comments,
   and delimiters survive and diffs are reviewable. `hook` bodies are never
   descended into.
2. **The 1.x dialect vocabulary is closed**: 13 dialect names + `all-tcl`
   + `tcl8.x` + five `tclX.Y+` forms — 21 tokens. The translation table is
   total; a word outside it refuses the *file* rather than being carried
   through unread.
3. **The loader's per-site vocabulary log inverts**: the machinery that
   notices "this word is newer than your declaration" also tells the
   upgrader which sites force 2.0 and proves the rewritten pack needs
   nothing newer than it declares.

What the tool does:

- `--check`, skip-on-no-`speclib`, refuse-on-non-vocabulary-word,
  reload-after-write proof, exit 1 on remaining work.
- The version word moves to `2.0` only when the body rewrite completed on
  that file; a file with any row left keeps its 1.x header and reports
  *partially upgraded*.
- `dialects` / `-dialects` → `available` at every loader site, through
  the total table: `tclX.Y` → single point, `tclX.Y+` → open range,
  `all-tcl` → `{tcl 8.4-}`, `tcl8.x` → `{tcl 8.4-9.0}` (exclusive maximum
  stated), `f5-irules` → the core family, `tk` → `{package Tk}` on Tk's
  own axis, `f5-bigip` → error (it leaves the Tcl axis). Output reuses the
  renderer's shorthand logic so upgraded packs read hand-written.
- Role discrimination against the live environment registry: a membership
  token whose environment declares exactly one ambient package translates
  to that provider (`f5-iapps` → `{package f5-iapps-cmds}`, `f5-tmsh` →
  `{package f5-tmsh-cmds}`, `expect` → `{package Expect}`), and the
  loader's `available` reader carries the exact inverse, so the translated
  spec is byte-equal. `spectcl` and `bpf` stay markers: their environments
  declare no ambient provider (their surfaces are compiled), so no
  `available` row can carry the claim, and the marker names that reason.
- `ambient_package NAME VERSION` rehomes into
  `environment OWNER -extend { ambient NAME VERSION }`, where OWNER is
  the pack's sole declared `environment` block, else its sole membership
  token across `dialects` rows; an ambiguous pack (or a non-plain version)
  keeps a `# TODO(spectcl 2.0):` marker and reports partial. Never guess.
- `file_extension … -dialect D` moves its detection into
  `environment D -extend { … }` (the flag dropped, everything else
  verbatim); an unresolvable `D` keeps the marker.
- `--infer-provides` (off by default) hoists a uniform `required_package`
  (the pack-level default, else one identical row in every command) to a
  pack-level `provides`, whose loader semantics keep the snapshot
  byte-equal.
- Post-rewrite proof through the vocabulary log: the rewritten file is
  re-loaded and every site needing a vocabulary above its own declaration
  is reported.
- `--verify` compares `command_entry_json` snapshots of the original and
  the rewritten pack across every dialect a 1.x row can gate on — the Tcl
  ladder, `f5-irules`, `f5-iapps`, `f5-tmsh`, `expect`, `spectcl`, `bpf` —
  plus the **environment-effect snapshot**
  (`upgrade::environment_effect_snapshot`): the scoped detection and
  placement rows both forms load to, which is what licenses moving a
  row's home while proving the registry effect stayed put.
- Explicit `--from` / `--to`; downgrades are refused (an unsupported major
  fails closed, so a 2.0 → 1.x rewrite would be a silent capability loss).
- `--restyle`: after the row rewrite, the pack is re-emitted through
  `export_pack` in canonical form — straight-line registration calls at
  the house layout, comments and author layout dropped — and `--verify`
  proves the restyled snapshot as it proves the plain one. A
  **programmed** pack is refused whole (`available?` at registration, or a
  top-level statement that is not one of the recorded registration
  calls); a partial upgrade keeps its TODO markers and is not restyled.

`sdc_base.tclspec` stays at `speclib … 1.1` as the live 1.x corpus the
tool and the loader are exercised on; the six EDA packs and `upf` declare
2.x.

## 7. The name-resolution oracle programme

The C Tcl codebase and tests, the stdlib, tcllib, Tk, and the corpus
ground name registration/resolution — namespaces, variables,
procs/commands, packages — in measured behaviour, consumed by every
consumer. Everything here extends mechanisms that exist; nothing invents
a parallel harness.

### 7.1 Reference interpreters

`ensure-test-deps.sh`'s `ensure_tclsh` builds all five reference
interpreters, each left at `<tree>/unix/tclsh` (the path
`audit_option_dialects` reads) and exposed on `PATH` as
`/usr/local/bin/tclsh{8.4,8.5,8.6,9.0,9.1}`. 8.4/8.5 build with
`CFLAGS="-O2 -fcommon -Wno-implicit-int -Wno-implicit-function-declaration"`
against modern gcc and only find `init.tcl` via a path relative to their
own real on-disk location, so those two are exposed through a thin wrapper
script that `exec`s the tree binary by its real path (`unset TCL_LIBRARY`
first); 9.1 tolerates a plain symlink. The build is idempotent, keyed on
`<tree>/unix/tclsh` reporting the expected `info patchlevel`.
`resolve_tclsh90` in `rust/xtask/src/tcltest_sweep.rs` tries, in order,
`TCL_LSP_TCLSH90`, `/usr/local/bin/tclsh9.0`, `tclsh9.0` on `PATH`, then
the in-tree `tmp/tcl9.0.4/unix/tclsh`, and fails naming `make
ensure-test-deps` when none exist. `audit_option_dialects` calls
`require_all_tclsh_built` before probing and fails immediately listing
any missing binary (`AUDIT_ALLOW_MISSING_TCLSH=1` is the documented escape
hatch for a deliberately partial run). `fetch_tcl_source.sh` fetches the
Tk trees (`tk84` … `tk91`, `tkall`) beside the Tcl ones, and
`session-start.sh` runs `tkall`.

Single-binary probes (`find_tclsh`'s first-hit callers) are not yet on
the five-version matrix (redesign §11 V11).

### 7.2 Vector files — one format, five domains, per-release expectations

One declarative pipe-separated file per domain, compiled into
`tcl-syntax`, a renderer that turns every row into executable Tcl, and
consumers executing the same bytes:

| File | Rows | Release-tagged rows | Renderer | Consumers wired |
|---|---|---|---|---|
| `rust/tcl-syntax/tests/data/command_resolution_vectors.txt` | 46 | 16 | `tcl_syntax::naming::conformance` (`want()`/`want_for()`) | pure resolver (newest column), analyser settlement, bytecode VM dispatch, WASM runtime dispatch, real tclsh **matrix** |
| `rust/tcl-syntax/tests/data/variable_resolution_vectors.txt` | 50 | 16 | `tcl_syntax::var_conformance` (`vector_setup` / `vector_call` / `vector_script`) | real tclsh matrix only |
| `rust/tcl-syntax/tests/data/namespace_op_vectors.txt` | 73 | 15 | `tcl_syntax::ns_op_conformance` (same triple) | real tclsh matrix only |

- `tcl_syntax::release_expectations::PerRelease` parses an expectation
  field that is either one value every release shares or a
  `RANGE=VALUE;RANGE=VALUE` list over the ladder 8.4, 8.5, 8.6, 9.0, 9.1
  (`8.4-8.6=…`, `9.0+=…`). The entries must cover the ladder exactly once,
  and a `;` only separates entries when a range token follows it, so
  expectation values may contain semicolons.
- `tcl_syntax::vector_ops` holds the row syntax the domain files share:
  pipe-split rows and the `kind(argument)` setup mini-ops, split on
  parenthesis depth so an argument may contain commas and nested
  parentheses.
- `rust/tcl-syntax/tests/support/mod.rs` is the five-binary matrix, keyed
  `TCL_LSP_TCLSH84` … `TCL_LSP_TCLSH91` with PATH fallbacks; it verifies
  the interpreter's reported version matches the name it was found under,
  and skips a missing release loudly. The three suites
  (`command_resolution_conformance`, `variable_resolution_conformance`,
  `namespace_op_conformance`) run every row against every available
  release and assert that release's own column.
- The variable and namespace-op observable is the two-element list
  `<catch code> <result-or-message>`, with the whole row inside the
  `catch`, so a row that uses a subcommand a release does not have states
  that release's real error text rather than being excluded. The command
  domain keeps its `-` sentinel for `invalid command name` and `!ERROR`
  for a scenario a release cannot set up at all.
- Each file carries a `documented-non-conformance` block naming the
  `knownBug` rows deliberately excluded (`namespace-56.4`, `info-15.8`,
  `interp-27.5–27.8`, `var-3.3`/`var-3.4`'s `testupvar` constraint, and
  `namespace-51.13`'s mid-teardown observation) — so nobody "fixes" the
  model to a behaviour real Tcl does not exhibit.

Three release deltas the C suites do not state directly are recorded in
the files: the creation path's direction inversion at 9.0 (an 8.x `set v`
inside a namespace writes *through* to an existing global, a 9.x one does
not), 8.4's acceptance of `upvar #0 a foo(bar)` that 8.5 turned into `bad
variable name`, and 8.4/8.5's refusal to create a procedure whose name
starts with `:` in a non-global namespace.

**Domains still to seed** (redesign §11 V12), each with its anchors in
the C test suites:

| File | Domain | Seed content |
|---|---|---|
| `command_binding_vectors.txt` | registration/rename/shadowing | proc-into-namespace rules (`proc-1.1/1.2`); definition-ns execution (`proc-3.4`); `rename` across namespaces incl. epoch bumps and shadow checks (`rename-*`, `basic-18.x`, `basic-24.x`, `namespace-old-6.6–6.9`, `trace-19.6–19.11`); hidden/expose invariants (`basic-12.1/13.1`); `interp alias` targets with `::` (`interp-27.x`); `info cmdtype` (9.0+, `info-40.x`); `resolver.test`'s six cache-invalidation paths — the ground truth for `BindingKnowledge` transitions |
| `package_lookup_vectors.txt` | package resolution | provider selection over `ifneeded` sets (`package-3.1–3.5`); not-found error forms; `package unknown` handler argument shapes (`-exact t 1.5` ⇒ `t 1.5-1.5`; the 8.4 `name version ?-exact?` form); ifneeded-that-does-not-provide errors; `prefer` latching (`15.4`); the four release-discriminating `bad option` lists; tm path ancestry/descendant rejection and LIFO ordering (`tm-3.x`) |
| `autoload_vectors.txt` | the autoload tier | `init-1.1–1.8` (`auto_qualify`'s eight pairs) plus `init-2.x`'s two-stage chains with tripled colons — `auto_qualify` is byte-identical across all five releases and is the reference implementation for `PackageResolver::auto_qualify` |

The `package.test` vsatisfies/vcompare tables (41 + ~140 rows, byte-
identical 8.5→9.1, with an 8.4 column derived from `pkg.test`'s `!tip268`
variants) extend the hermetic `package_version_oracle`; the per-release
error-message strings (`namespace "X" not found in "Y"` vs 8.4's `unknown
namespace "X" in …`; `parent namespace doesn't exist`; `variable is a
constant`) become diagnostic-text conformance data.

### 7.3 The stdlib as executable specification

The pure-Tcl library scripts *are* the reference implementation of the
tiers the resolver mirrors, and they differ per release:

- **`init.tcl`** — `unknown`'s step order (auto_load with the *caller's
  namespace* passed explicitly; interactive-only exec/history/abbreviation
  tiers; the final `TCL LOOKUP COMMAND` errorcode); `auto_load`'s
  candidate list (the 8.4/8.5 duplicate-try vs 8.6+ `ni` guard, and
  `namespace eval ::` vs `namespace inscope ::`); `auto_load_index`'s
  back-to-front `auto_path` walk and its `auto_oldpath` memoisation.
- **`package.tcl`** — `tclPkgUnknown`'s scan order (back-to-front,
  subdirectory `pkgIndex.tcl` files before the directory's own, seen-dir
  memoisation, and the mid-scan `auto_path` growth re-scan), plus the 9.0
  `VERSIONCONFLICT` trap and `source -nopkg` deltas.
- **`tm.tcl`** — the module filename pattern, `::`→`/` mapping, root
  construction (`site-tcl` first via prepend-LIFO), env-var paths (9.0's
  `file tildeexpand`), the lowercase-`tcl` probe in 9.x, and the precedence
  rule: **tm paths beat `tclPkgUnknown`** via load-time handler chaining.
- **`safe.tcl`** — `RejectExcessColons` as a name-normalisation oracle;
  tokenised `auto_path` in safe children.

Each should become a pinned behavioural contract on `PackageResolver` and
the autoload tier, per release, tested hermetically against committed
vectors and (env-gated) by executing the real library scripts under the
matrix binaries. Not yet done.

### 7.4 Real-corpus oracles

No test reads a real `pkgIndex.tcl`: every `PackageResolver` test writes
synthetic indexes, while the real index files sit unread in the Tcl trees
under `tmp/` and beside every tcllib module. Wanted, as skip-if-absent
suites corpus-gated like `differential_segment`: index-ingest parity (run
`PackageResolver` over `tmp/tcl*/library/**` and `tmp/tcllib-2.0/modules/**`
and cross-check against a live tclsh's `package names` + `ifneeded`
registrations, per release); autoload parity over the real `tclIndex`
files; resolution sweeps with *outcome* assertions over the tcl/tcllib
library trees; a name-resolution stem set (`namespace`, `namespace-old`,
`resolver`, `var`, `upvar`, `uplevel`, `trace`, `rename`, `info`, `interp`,
`package`, `init`, `tm`, `autoMkindex`, `pkgMkIndex`, `unknown`, `basic`,
`safe`) as a `tcltest_sweep` scoreboard column for both engines; and Tk's
additive naming domain (pathname-named widget commands, the option
database's own resolution algorithm, Tk's real `pkgIndex.tcl` and the
`tk`/`Tk` co-provide chain).

### 7.5 The consumer conformance lattice

Every consumer of the §1.2 resolution stack should be wired to the shared
vector data — none may pass on a private subset. ✅ = wired; ● = wanted:

| Consumer | command | variable | namespace-op | binding | package | autoload |
|---|---|---|---|---|---|---|
| pure resolver (`tcl-syntax::naming`) | ✅ | ● | ● | — | — | ● (auto_qualify) |
| analyser settlement (post-walk) | ✅ | ● | ● | ● | ● (floors) | ● |
| realm layer / `BindingKnowledge` | ● | ● | ● | ● **primary** | ● (package states) | ● |
| bytecode VM dispatch | ✅ | ● | ● | ● | ● | ● |
| WASM runtime dispatch | ✅ | ● | ● | ● | ● (real package loop) | ● |
| codegen dispatch proofs (`ProofStatus`) | — | — | — | ● (epoch/rename rows) | — | — |
| `PackageResolver` + floor engine | — | — | — | — | ● **primary** | ● **primary** |
| LSP cross-file leg (`settle_call_against_workspace`) | ● (e2e) | ● | ● | ● | ● (W120/W123) | ● |
| engines behind `tcl-engine-api` | ● | — | — | ● | ● | — |
| real tclsh (the oracle itself) | ✅ | ✅ | ✅ | ● | ● | ● |

The one-oracle gate (§4 R10) enforces the lattice structurally: a consumer
that cannot pass a domain's vectors has no business holding a private
implementation of that domain.

### 7.6 Policy: hermetic vs live vs manual

- **Hermetic (CI, always)**: committed vector files and generated corpora
  (the `package_version_oracle` pattern — generator script, hand-maintained
  `#` header naming exact patchlevels, check = output-equals-file),
  including the extracted vsatisfies tables and error-message data.
- **Live matrix (env-gated, skip-loudly)**: `TCL_LSP_TCLSH{84..91}` suites
  re-pinning vectors against real binaries; the stdlib-execution parity
  tests; `dialect_oracle`-style existence probes.
- **Manual tier (`make test-exhaustive`)**: full tcltest replay
  scoreboards, whole-corpus index/resolution sweeps, multi-release sweeps.
  Never wired into CI.
- **Appliance tier (owner-run, checked-in transcripts)**: the BIG-IP probe
  corpus at `scripts/dev/bigip-probes/` with its results
  ([measurements](bigip-irule-parser-measurements.md)) — re-runnable
  against a real appliance, never CI. The **appliance tier produces
  transcripts** and the **hermetic tier consumes them**:
  `rust/tcl-registry/src/f5/corpus.rs` holds 205 vectors derived from
  those transcripts, every row citing its measurements section and
  asserted against the model in ordinary `cargo test`; no test in this
  repository talks to an appliance, and no appliance result reaches the
  model except through a cited corpus row or an `EmbeddedRuntimeEvidence`
  record. What the appliance tier still owes is coverage, not machinery
  (redesign §11 V1–V4).
