# Lane C1 / D1 — the executable-IR re-key

**Status: code complete, validated.** This file is the resumable record of
the lane: a fresh agent should be able to pick the work up cold from here.

Ledger row: `docs/design/dialect-and-package-registry-centralisation.md` C1.
Redesign rows: `docs/design/dialect-and-package-registry-redesign.md` §11.2
**D1** (this lane) and **D5** (what it unblocks), §11.4 **E1** (landed here).

## 1. The vocabulary, and why this one

`DialectSet` is retired from the semantic-analysis and executable-IR path in
favour of **`tcl_registry::model::semantic::SemanticContext`** — a `Copy`
generation-bound handle wrapping the `&'static ContextRegistry` that
`model::ingress::static_context_for` already interns, one view per resolved
environment id.

Why a handle rather than a `ResolvedContext` value:

- `SemanticAnalysisBundle` is a field of `FunctionUnit`, which is a field of
  `CompilationUnit`, which salsa memoises on `PartialEq`. A `ResolvedContext`
  is an `Arc<EnvironmentDefinition>` plus four owned vectors, and is neither
  `Eq` nor `Copy`. Cloning one per function unit per keystroke and structurally
  comparing one per memo probe would put allocation and a deep compare on the
  per-edit path for a value that is *the same object* for every unit of a
  document — standing principle **P-B** forbids exactly that.
- The interning gives pointer equality == environment identity, so
  `PartialEq`/`Eq`/`Hash` are three instructions.
- The handle carries **both** halves a resolution needs (the `ResolvedContext`
  availability view and the generation's command store) without a second
  lookup, and it is the stable **id** row E1 needed for the salsa lexer key.
- Resolution costs one name ingress; the compiler resolves it **once per module
  build** and threads the handle, where the retired `DialectSet` projection ran
  once per function unit.

The selection primitive is C7/I4's, not a second one:
`model::semantic::resolve_structured_invocation_in_context` is the
structured-words face of `model::assembly::resolve_invocation_in_context`
(binding-proof obligation via `ResolvedContext::resolve_spec`; no context ⇒
the dialect-blind store selection, exactly as `DialectSet::empty()` behaved).

Absent context (`None`) is the whole of the old "no one explicit dialect
profile" state, so `ExecutableAnalysisAvailability::DialectUnavailable {
dialect }` becomes the payload-free `ContextUnavailable`.

## 2. Site inventory

All **done**. New file:

- `rust/tcl-registry/src/model/semantic.rs` — `SemanticContext`,
  `resolve_structured_invocation_in_context`, and the D1 equivalence sweep.
- `rust/tcl-registry/src/model/mod.rs` — module wiring and re-exports.

Re-keyed (`DialectSet` → `Option<SemanticContext>`):

| Crate | File | What moved |
|---|---|---|
| tcl-compiler | `registry_invocation.rs` | `resolve_word_exprs`, `resolve_command_tokens` |
| tcl-compiler | `executable_ir.rs` | `build_linear_executable_ir` + `plan_linear_stages` / `plan_source_statement` / `resolve_invocation_facts` |
| tcl-compiler | `semantic_analysis.rs` | `SemanticAnalysisBundle::{build, build_for_interactive_analysis, unavailable}`, `dialect()`→`context()`, `ExecutableAnalysisAvailability::ContextUnavailable` |
| tcl-compiler | `compilation_unit.rs` | `semantic_dialect_set` → `semantic_context`; `with_semantic_analysis`, `with_top_level_semantic_analysis`, `with_memory_ssa`, `with_deep_semantic_analysis`, `build_method_units`, `build_body_units`, both `unavailable(DialectSet::empty())` constructors |
| tcl-compiler | `memory_ssa.rs` | `build_memory_ssa`, `compute_aliases`, `has_wildcard_aliasing`, `statement_has_wildcard_aliasing`, `is_clobber`, and the private resolution helpers |
| tcl-compiler | `common_aot_plan.rs` | `CommonAotProofPlan::build`, `DirectProcEvidence::context`, `SemanticCallEvidence::context`, both `ContextUnavailable` declines |
| tcl-compiler | `var_escape/helpers.rs` | `invocation_facts_from_tokens` |
| tcl-compiler | `world_state_ssa.rs` | test harness (the SSA itself is keyed by the IR it is built from) |
| tcl-compiler | `mixed_region_plan.rs`, `dispatch_proof.rs`, `native_integer_proof.rs` | test harnesses |
| tcl-compiler | `optimiser/branch_folding.rs`, `optimiser/propagation.rs` | `unavailable(None)`; `ForwardEnv::context` |
| tcl-compiler | `codegen/wasm/backend.rs`, `codegen/wasm/pipeline.rs` | the WASM bridge: `plan_leaf_statement`, the trusted-builtin resolution, `CommonAotProofPlan::build`, `WasmExecutableAvailabilityDecline::ContextUnavailable` |
| tcl-lsp-db | `src/lib.rs` | `semantic_dialect_set` projection → `semantic_context` |
| tcl-lsp-core | `src/graphs.rs` | `with_memory_ssa` caller |
| tcl-explorer | `src/lib.rs`, `src/serialise.rs` | deep-analysis + memory-SSA callers; the decline payload |
| bpf-tcl-ir | `src/semantic_bridge.rs` | the BPF bridge: `EbpfSemanticBridge::assess`, `EbpfRegionDecline::ContextUnavailable` |

Tests updated: `tcl-compiler/tests/{alias_scoping, dataflow, dialect_threading,
entry_contract_abstention, pipeline_coverage, real_job_fixtures}.rs`.

`DialectSet` no longer appears anywhere on the executable-IR path. It survives
only where the ledger licenses it: **inside** `tcl-registry` as the authoring
mask a `ResolvedContext` derives and the store's own fast-path index (row C1's
"optional internal `FamilySet` fast path"), and in the analyser / diagnostics
surfaces that are other ledger rows' to retire.

## 3. Behavioural deltas (each carries a citing comment at its site)

1. **The semantic key widens.** The retired projection was
   `DialectSet::parse(profile.name)` — the *exact* bit the profile's own name
   parses to. The context's authoring mask is the environment's real mask
   (pinned equal to the old `availability_mask` by the P1-E parity sweep). The
   `model::semantic` sweep
   `the_re_key_never_loses_a_resolution_and_is_identical_on_the_single_bit_ladder`
   pins it over every command name of every catalogue environment:
   - **byte-identical** for `tcl8.4`, `tcl8.5`, `tcl8.6`, `tcl9.0`, `tcl9.1`,
     `f5-irules`, `f5-bigip` — the environments whose authoring mask *is* the
     bit their name parses to. `tcl8.6` is the session default, so the mainline
     LSP path is unchanged;
   - **widens** for `bpf` (`BPF` → `TCL90|BPF`), `expect`
     (`EXPECT` → `TCL86|EXPECT`), `spectcl` (`SPECTCL` → `TCL90|SPECTCL`),
     `f5-iapps` (`IAPPS` → `TCL84|IAPPS`), `f5-tmsh` (`TMSH` → `TCL84|TMSH`)
     and `tk`;
   - widens **from nothing** for the seven environments whose name owns no
     `DialectSet` bit at all — the lenient `tcl` sink and the six EDA shells.
     They previously produced `DialectSet::empty()`, whose `canonical_name()`
     is `None`, so the bundle recorded `DialectUnavailable` and the document
     got *no executable facts whatsoever*;
   - **never loses** a resolution: no environment resolves fewer names in
     context than the retired bit did.

   This is a consistency fix as much as a widening: `with_deep_semantic_analysis`
   and Explorer already passed `profile.availability_mask`, so the interactive
   path was seeing a *narrower* registry than the deep path for the same
   document. Both are now the one environment.

2. **`dialect: None` is unchanged.** `semantic_context(None)` is `None`, which
   records `ContextUnavailable` exactly where `DialectSet::empty()` did.

3. **`DialectUnavailable` → `ContextUnavailable`,** payload-free, in
   `ExecutableAnalysisAvailability`, `WasmExecutableAvailabilityDecline`,
   `EbpfRegionDecline`, `DirectProcDecline` and `SemanticCallDecline`.
   Explorer's stable JSON spelling `"dialect-unavailable"` becomes
   `"context-unavailable"` and the payload drops its now-meaningless
   `"dialect"` field. No consumer in the tree or the editor integrations reads
   either.

4. **The ambiguous-mask premise is unrepresentable.**
   `common_aot_plan`'s `input.dialect.bits().count_ones() != 1` gate becomes
   `input.context.is_none()`. A resolved environment names exactly one context
   or none, so a multi-bit premise cannot be constructed. The pre-existing
   assertion `dialect_tcloo_and_variable_trace_premises_are_retained` fed
   `DialectSet::ALL_TCL` (a value no production caller could produce) — it now
   pins the surviving decline, a unit built with no dialect. Rewritten with a
   citing comment.

5. **E1 — see §4.** The CFG/SSA analyser tail's lexer config.

## 4. E1 — the salsa lexer-config truncation, with the measured cost

**Landed, id-keyed.**

`LexerCfgKey` interned three of the six dialect-derived `LexerConfig` fields
and `to_config` restored the rest from `LexerConfig::default()`, so on the
memoised path `braced_var` was always `Tcl9Nesting` and `escapes` always
`Tcl90` whatever the document said. `ProcBodyKey` duplicated the same three
fields beside the `dialect` it already carried.

Measured over the 18 catalogue profiles plus `tcl` and `tk` (20 environments):

| | today (truncated 3-tuple) | widening the tuple to 6 | **landed (environment id)** |
|---|---|---|---|
| distinct `LexerCfgKey`s | 3 | 5 | 1 per environment, and both consumers agree |
| environments sharing **one** `compilation_unit` build per edit | 15 / 20 | **7 / 20** | **20 / 20** |
| correct `braced_var` / `escapes` | no (13 / 20 wrong) | yes | yes |

The middle column is why E1 sat open: widening the tuple costs the sharing for
`tcl8.5`, `tcl8.6`, `expect` and the five 8.x EDA shells — including the two
most common dialects in the tree — so the LSP would build **two** compilation
units per edit for a mainstream document. That is a straight P-B violation.

Two further measured facts:

- of the three truncated fields, `leading_bom` is `Content` for *every*
  environment under `for_dialect`, so only `braced_var` and `escapes` actually
  diverged — but they diverge for **13 of 20** environments, and **8** of those
  silently shared the tail's 9.0-shaped key and so were wrong without even
  paying for a second build;
- the five environments that already failed to share (`tcl8.4`, `f5-irules`,
  `f5-iapps`, `f5-tmsh`, `cadence-eda-tcl`) were paying for two builds *and*
  getting 9.0 escapes in one of them.

**The shape landed** is the one the D1 re-key made available: key on the
resolved **environment id**, not on expanded config fields.

```rust
#[salsa::interned]
pub struct LexerCfgKey<'db> { #[returns(ref)] pub environment: String }
// to_config == LexerConfig::for_dialect(environment)
```

and move all four hosts of the CFG/SSA tail's unit onto the document's own
environment grammar, where all four previously agreed on
`LexerConfig::default()`:

- `tcl_compiler::analyser::diagnostics::Analyser::emit_cfg_ssa_diagnostics`
  (its own in-branch build),
- `tcl_lsp_db::analyse_per_item_with` (the `cu_override` seed),
- `tcl_cli::commands::diag::collect_rows`,
- `xtask::fp_sweep`.

The agreement invariant those four maintain is preserved — they still all build
the same unit — and it is now the *right* unit. `tcl diag`'s second
`checks_cu` build disappears entirely (the two configs are now always equal).
`ProcBodyKey` drops the three duplicated fields and derives the config from the
`dialect` it already interned, so its memo splits exactly where `LexerCfgKey`
does and nowhere else.

The behavioural half: the CFG/SSA diagnostic tail now lexes an 8.x document
with 8.x escapes and the `FirstClose` braced-var rule, and an iRules document
with `{*}` expansion off, the F5 word break on and brace-line continuation on.
That is the §9.1 defect-1 fix. No test in the workspace asserted the old
behaviour.

## 5. What D5 still needs

D5 is "pack-declared dialects cannot lex": `Family` is a closed enum,
`DialectProfile::grammar()` is a `const fn` over ladder ordinals, and
`tcl_lexer::LexerConfig` is built from a `&'static DialectProfile` out of a
compiled table, so P3's `DynamicFamily` + `LexerGrammar` is reachable data with
no consumer.

What this lane changed for it, and what is left:

- **Crossed:** the analysis path no longer speaks `DialectSet` at all, so a
  pack-declared environment can now *carry* semantic facts — `SemanticContext`
  is constructed from an environment id through `ingress::resolve_environment`,
  which already resolves live pack-registered environments, and nothing on the
  executable-IR path asks a profile for a bit any more.
- **Not crossed, and it is now the only boundary:** two `&'static
  DialectProfile` projections remain, both narrow and both named at their
  sites. `SemanticContext::runtime_version` is `DialectProfile::find(id)
  .and_then(runtime_version)`, and — the load-bearing one —
  `tcl_lexer::LexerConfig::for_dialect(name)` is
  `from_grammar(DialectProfile::find(name).unwrap_or(plain_tcl).grammar)`.
  A pack-declared environment id misses `find`, silently sinks to plain Tcl,
  and lexes as plain Tcl.
- **The shape of the fix:** `EnvironmentDefinition` must own a
  `LexerGrammar` (and a runtime release) directly, sourced from the compiled
  `DialectProfile` for a catalogue environment and from
  `tcl_dialect::model::DynamicFamily` for a pack-declared one; then
  `LexerConfig::for_environment(&ResolvedContext)` — or
  `SemanticContext::lexer_config()` — replaces `for_dialect(name)` at the ~200
  `for_dialect` / `for_file_dialect` call sites the F8/C12 boundary rows
  already enumerate. E1's id-keyed `LexerCfgKey` is *already* in that shape:
  it interns the environment id and asks for the grammar by id, so the D5
  change is a one-line change of which door answers, not a re-key.
  `Family`'s closed enum can stay: nothing on this path matches on it once the
  grammar is a field.

## 6. Validation

- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets` — zero warnings from this lane.
- `cargo fmt --all` clean.
- `cargo test` green for `tcl-registry` (including the P1-E equivalence
  sweeps, the P1a C7 sweep and the F5 conformance vector suites), `tcl-compiler`,
  `bpf-tcl-ir`, `tcl-lsp-db`, `tcl-lsp-core`, `tcl-lsp-server`, `tcl-explorer`,
  `tcl-cli` (bar the known headless `explorer_gui` wasm-asset failure).
