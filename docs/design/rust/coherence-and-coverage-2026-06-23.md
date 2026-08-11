# Rust coherence & coverage review (2026-06-23)

> The closing pass of the whole-project Rust review. It does two things:
>
> 1. **Coverage map** — a single table proving every aspect of the review goal is
>    documented *somewhere* across the six Rust review docs, with the section that
>    covers it. This is the completeness contract.
> 2. **The remaining axes** that the other five docs do not address head-on:
>    **type-system coherence across crates**, **naming coherence + glossary
>    currency**, the **explorer trio (CLI / TUI / GUI)**, and the **"information"
>    subsystem** (Info-severity diagnostics + the `info` command family).
>
> Scope: the Rust workspace under `rust/` plus the go-forward WASM runtime
> `runtime/rust/`. The Python tree is **out of scope** — it is
> being deleted, and this review treats it as already gone. Reviewed on branch
> `claude/exciting-planck-q7rj94`; claims anchored file:line. Companion to:
> [workspace deep review](workspace-deep-review-2026-06-22.md),
> [LSP-server deep review](lsp-server-deep-review-2026-06-22.md),
> [Python→Rust parity audit](python-rust-parity-audit-2026-06-22.md),
> production-readiness assessment,
> architecture & code-quality review.

## Verdict

The four closing axes split cleanly into one bright spot and three "coherent-half /
fractured-half" stories:

- **Type system (§2): bimodal.** The runtime / value / registry layer is production-grade
  and should be the template (one `ValueOps` seam, real bitflags, compile-time-exhaustive
  typed hook-ids, arena newtypes). The editor-facing layer is fractured *along exactly the
  UTF-16 seam that already shipped a bug*: byte vs UTF-16 offsets are both raw `u32` with
  no type protection, there are **three `Severity` enums and two `Diagnostic` structs**,
  diagnostic codes are stringly-typed and dispatched by `starts_with`, and the whole
  SSA/CFG/GVN IR is `String`-keyed while `tcl-bytecode` right beside it interns.
- **Naming (§3): acronyms good, words pervasively mixed.** One real casing break
  (`MemorySSAFunction` vs `SsaFunction`); but abbreviate-or-spell-out flips within a single
  line (`statement_index: e.stmt_index`, `argument_index: arg_index`), "a Tcl command" has
  three competing module conventions, and `MethodDef`/`ProcDef` are each defined 2–3×. The
  glossary is conceptually strong but cites stale type names and is still half Python-framed.
- **Explorers (§4): the model of the whole codebase.** One shared core
  (`run_pipeline`→`serialise_result`), three thin surfaces (CLI/TUI/GUI) that physically
  cannot drift, driving the **real** compiler pipeline and reusing lexer + registries with
  zero re-implementation, clean feature-gating, `wasm32` enforced in CI. This is precisely
  the sharing discipline the goal asks for — already done. Only cosmetic nits.
- **Information (§5): two parity bugs.** Info-severity I230/I231 collapse to `Hint` (no
  `Info` in the analyser `Severity`) so they render differently than Python's
  `Information`; and the `info` command family is `VM ⊂ WASM` (15 vs 29 subcommands, VM
  fakes 3 version constants), with neither runtime driven by the registry spec that already
  enumerates 28 — a registry-leverage miss.

**The throughline across all four:** the workspace already contains the *right* pattern
for every incoherence — `ValueOps` for value sharing, the registry's typed vocabulary,
`tcl-bytecode`'s interner, the explorer's one-core reuse, `compiler_checks::Severity::Info`
for severity. The production work is **propagating those patterns to the editor-facing
half and consolidating the shared vocabulary into `tcl-core-types`**, not inventing
anything new.

### §0. Reconciliation with the just-landed `origin/rust` work

While this review was in flight, **43 commits landed on `origin/rust`** (merged into the
review branch here): the **API-PYO3** effort (a dedicated `tcl-lsp-py` public surface,
~1,098 LOC under `src/public/{errors,facades,options,results}.rs` — ARCH9), the
**scripts→xtask** migration (`rust/xtask/`, ~2,122 LOC), and a **TEST-MIGRATE** half
porting Python tests to Rust unit tests. The change is **additive** (53 files,
+5,667/−19) and touches the reviewed subsystems only lightly (`tcl-compiler/src` saw a
single new file, `static_loops.rs`), so **every file:line anchor in this review and its
five companions was re-verified to still hold.** Three points matter for the findings:

1. **It does not close the P0 FP test-net gap — and sharpens it.** TEST-MIGRATE ported VM
   / cmd-core / bigip / registry helper tests to Rust, but **the FP precision suite was
   not ported**: zero `rust/**/tests/*.rs` reference any FP-id, and all 12
   `tests/test_fp_*.py` still exist as Python-only. They are actively porting tests yet
   skipped the precision contract — the AQ §F
   headline stands, now with a live test-migration effort that should simply include it.
2. **PyO3 is being *reinforced*, not retired — distinguish binding from core.** ARCH9 just
   built a structured PyO3 public surface for wheel consumers; `tcl-lsp-rust` remains a
   transitional alias to it "for one release cycle." So "retire Python" means the Python
   **implementation/core**, while a PyO3 **binding** persists transitionally. The AQ §E
   doc-accuracy fixes should re-point headers at `tcl-lsp-py` (not delete the PyO3 framing
   wholesale).
3. **xtask is now the tooling home, but `cargo deny` is still unwired.** The
   scripts→xtask migration is the natural place to wire the supply-chain gate, yet no
   `cargo deny` invocation exists in `xtask`, the `Makefile`, or CI — AQ §D6
   holds. (New xtask verbs of note: `refcount_contract`, `audit_option_dialects`,
   `tzdata_bundle`.)

## 1. Coverage map — every goal aspect → where it is documented

The review goal enumerates a large set of aspects. This table maps each to the doc
and section that reviews it, so completeness is auditable at a glance. Abbreviations:
**WS** = workspace-deep-review-2026-06-22, **SRV** = lsp-server-deep-review-2026-06-22,
**PAR** = python-rust-parity-audit-2026-06-22, **PROD** = production-readiness-2026-06-23,
**AQ** = architecture-and-quality-2026-06-23, **CC** = this doc.

| Goal aspect | Primary coverage | Headline finding |
|---|---|---|
| **Parser / lexer** | WS (lexer/CST), AQ §A1 | One tokeniser (`tcl-lexer`); runtime converged onto it — no re-implementation |
| **Compiler** | WS, PAR | Faithful port; CST→IR→CFG→SSA→codegen pipeline intact |
| **Lowering** | AQ §C, PAR | Fully registry-driven via `LoweringHookId`; string-match dispatch deleted |
| **Optimisers** | WS, PAR, PROD §E | 4 Rust-only miscompiles (O122/O109/O126/O129); O129 gate dead in prod |
| **Diagnostics** | WS, PAR, AQ §F | Ported; ~18 severity-tier divergences (PAR); FP precision gaps (AQ §F) |
| **Information** (Info severity + `info` cmd) | **CC §5** | Info-severity collapses to `Hint` (parity bug); `info` cmd `VM ⊂ WASM`, neither registry-driven |
| **Shimmer (S-codes)** | AQ §F, PAR | PARTIAL — SH-08 missing; phi loop-downgrade fixed |
| **Taint tracking (T-codes)** | AQ §F, PROD §E | PARTIAL — TNT-01 missing; not wired into the `tcl` CLI (PROD) |
| **Lattices** | AQ §D3 | Lattice fixpoints written correctly (in-place); only the String keys hurt |
| **Registries** | AQ §A2/§C | Source of truth for compiler/codegen; runtimes hand-maintain tables |
| **Docs true to Rust** | AQ §E | Stale: AGENTS.md + ~16 headers describe a Python/bootstrap world |
| **WASM** | WS, PROD §E2 | WASM codegen path broken (PROD) |
| **tclvm** | WS, AQ §C | Outlier: unused `tcl-registry` dep + separate HashMap dispatch |
| **Runtime layers** | WS, AQ §A1/§C | `ValueOps` seam shares command logic across both runtimes |
| **FP doc coverage** | AQ §F | **No Rust test net** — 177 paired FP tests are Python-only |
| **Previous issues implemented** | AQ §F(4), PROD, SRV | C1/C2/C3 verified fixed; residuals honestly tracked |
| **Configuration layering** | AQ §G | Global vs per-folder divergence; CLI config-file-blind |
| **tcl & f5 tooling** | PROD §F | CLI not yet shipped; taint not wired into CLI; docker/stubs |
| **Explorers (CLI/TUI/GUI)** | **CC §4** | The model of reuse — one core, real pipeline, clean gating; the bright spot |
| **Sharing / no re-implementation** | AQ §A/§C | Strong (one lexer, ValueOps); runtime tables + ~30 consumers cheat |
| **Registries leveraged / lowering hooked** | AQ §C | Lowering+codegen registry-driven; ~30 consumer hardcode sites |
| **Clean code / minimal clippy** | AQ §D5 | 351 allows, almost all pedantic/justified; target ~325 |
| **Algorithms & data structures** | AQ §D1–D3 | SipHash everywhere; String-keyed SSA; confirmed O(n²)/O(n³) |
| **Architecture coherent & solid** | AQ (whole), CC §2 | Clean acyclic graph; soft dataflow core |
| **MSRV 2024+** | AQ §D (MSRV) | Edition 2024 / rust-version 1.96 uniform; zero nightly |
| **Dependencies latest** | AQ §D6 | `tower-lsp 0.20` unmaintained; `rusqlite` behind; `deny.toml` unwired |
| **Type-system coherence across crates** | **CC §2** | Bimodal: registry/value half coherent; editor half fractured (raw offsets, 3×Severity, 2×Diagnostic, stringly IR) |
| **Naming coherence across the stack** | **CC §3** | Acronyms good (1 break: `MemorySSAFunction`); words mixed to one line; `MethodDef`/`ProcDef` dup'd |
| **Glossary up to date** | **CC §3** | Strong coverage but stale type-names + ~half Python-framed; missing `ValueOps`/`salsa` |
| **PyO3 public surface** (ARCH9, just landed) | **CC §0** | New `tcl-lsp-py/src/public/`; binding persists transitionally as Python core retires |
| **xtask tooling migration** (just landed) | **CC §0**, AQ §D6 | scripts→`rust/xtask/`; `cargo deny` still unwired despite new home |

Everything above the four **CC** rows is already reviewed in the earlier docs; the
rest of this document supplies the four remaining axes.

## 2. Type-system coherence across crates

**Verdict: the type system is bimodal.** The **runtime / value / registry** half is
genuinely production-grade and should be the template for everything else; the
**editor-facing** half (offsets, severities, diagnostic codes, compiler-IR identity,
range/dialect representations) is fractured — and it fractures along *exactly* the
seam that already produced the shipped UTF-16 bug. The same concept is modelled
differently at almost every layer boundary that touches the editor.

**The coherent half (credit — this is the bar):**

- **One value seam.** Two value *representations* exist by design — `tcl_vm::Value(Rc<Obj>)`
  (`tcl-vm/src/value.rs:22`) and the 24-byte C-ABI `TclObj` (`runtime/rust/src/obj.rs:106`)
  — but they unify behind the single `ValueOps` trait (`tcl-syntax/src/value.rs:87`) with
  thin impls on `Vm` and `Interp`. No duplication of intent.
- **Typed registry vocabulary.** `Traits: u64` bitflags (`traits.rs:9`), `DialectSet: u16`
  (`dialects.rs:11`), the `LoweringHookId`/`CodegenHookId` enums with
  **compile-time-exhaustive** dispatch (`hooks.rs:19,104,149`), `ArgRole`/`Arity`/`BodyKind`,
  one `CommandSpec` (`spec.rs:83`), the `Code`/`Completion<V>` core types
  (`tcl-core-types/src/lib.rs:24,82`), and arena-handle newtypes `NsId/CommandId/VarId(u32)`.
  This half never identifies anything by string.

**The fractured half (the production work):**

1. **Byte vs UTF-16 offsets are both raw `u32` — the #1 correctness landmine.** `Span`
   carries *byte* offsets as `u32` (`span.rs:22`); `SourcePosition.character` is a `u32`
   that means **byte-column or UTF-16-column depending solely on which function filled
   it** (`position_at` vs `position_at_utf16`, both writing the same field —
   `line_index.rs:103-114` vs `:155-181`; the struct doc at `tokens.rs:74-85` admits it).
   Conversion signatures are all bare `u32`, so nothing stops a byte offset being passed
   where a UTF-16 column is expected. The UTF-16 math is **implemented twice**
   (`tcl-lexer/line_index.rs` *and* `tcl-lsp-core/definition.rs:75-88`
   `utf16_col_to_char_col`/`utf16_len`), and the one correct hub `lift_span`
   (`tcl-lsp-server/src/lib.rs:5443`) is **bypassed by ≥6 inline re-derivations**
   (`lib.rs:1424,1573,1724,1872`; `code_actions.rs:289`; `refactor/mod.rs:76`). This is the
   exact shape of the bug class the codebase already shipped (C1), and the type system
   provides **zero** protection against its recurrence. **Fix:** `ByteOffset(u32)` /
   `Utf16Col(u32)` / `LineCol` newtypes in `tcl-core-types`; one UTF-16 implementation;
   `lift_span` the only range constructor.
2. **Three divergent `Severity` enums; the analyser one is missing `Info`.**
   `compiler_checks::Severity` has 5 variants incl. `Info` (`compiler_checks.rs:41`),
   `analyser::Severity` has **4 with no `Info`** (`analyser/types.rs:30`),
   `explorer::Severity` has 3 (`views.rs:53`). Because the analyser can't express `Info`,
   its Info-class findings collapse to `Hint` (`lib.rs:5762-5771`) — a real editor-visible
   parity defect (see §5 Facet A for the I230/I231 consequence). **Fix:** one `Severity`
   in `tcl-core-types`, re-exported; the LSP `From` becomes total and lossless.
3. **Two `Diagnostic` structs.** `compiler_checks::Diagnostic` (`compiler_checks.rs:77`,
   has `category`/`replacement`) and `analyser::Diagnostic` (`analyser/types.rs:114`, does
   not), each hand-lifted to the LSP type. Confirmed independently. They should be one type.
4. **Diagnostic codes are stringly-typed and dispatched by `starts_with`.** `code: String`
   everywhere (`compiler_checks.rs:80`, `analyser/types.rs:116`), filtered by
   `code.starts_with('O')` and `disabled.contains(&code)` (`lib.rs:5890-5905`). Typos and
   unknown codes are invisible; the W/E/I/O/S/T/IRULE catalogue is not enumerable. **Fix:**
   a `DiagCode` enum (or per-category enums) with a `category()` accessor and `Display` to
   the wire string, making the optimiser/disable filters exhaustive matches.
5. **The compiler IR is fully stringly-typed while `tcl-bytecode` right beside it
   interns.** `ValueKey = (String, Version)` (`ssa.rs:41`), block identity as `String`
   keys (`cfg.rs:161`, `ssa.rs:103`, `Goto{target:String}`), `ExprKey = Vec<String>`
   (`gvn.rs:45`), `Version` a bare alias not a newtype (`ssa.rs:38`) — cloned through every
   pass — whereas `tcl-bytecode` interns to numeric ids (`lib.rs:799-895`). This is the
   type-level root of the performance findings in
   AQ §D1–D3; **Fix:** `BlockId`/`ValueId`/
   `Symbol` newtypes backed by the bytecode interner pattern.
6. **"Range" exists 5 ways, "Dialect" 2 ways.** Range: `Span`, `SourcePosition`,
   `LspRange` (`definition.rs:64`), the `tower_lsp` wire type, plus separate clones in
   `tcl-bigip/range.rs:15` and `f5-xc/diagnostics.rs:26`. Dialect: typed `DialectSet`/
   `TclVersion` in the registry but `&str`/`String` (`.starts_with("tcl9")`) once it reaches
   the compiler/server (`tcl_expr_eval.rs:111,133`, `lib.rs:116`). **Fix:** one
   `Range`/`Position` over the offset newtypes in `tcl-core-types`, `From` into the wire
   types only at the edge; thread `DialectSet`/`TclVersion` end-to-end.

**Conversion seams:** **10 `From` impls, 0 `TryFrom`, 0 explicit `Into`** vs **~34
`lift_*` + ~14 `project_*`** ad-hoc free functions. The value/error seam is idiomatic
(`Span→Range<usize>` `span.rs:80`, `ValueError→CmdError` `cmd-core/error.rs:67`); every
**cross-layer editor conversion** is a hand-written `lift_*` in the server crate
(`lib.rs:5476,5531,5750,5797,5866`) with **no `From<compiler::Diagnostic> for
lsp::Diagnostic`**, so completeness is unverifiable and the five independent
`match severity` blocks (`lib.rs:5492,5559,5762,5836,5911`) must each be edited in
lockstep. **Conversions belong as `From`/`TryFrom` on shared `tcl-core-types`, not as
scattered `lift_*` in the server.** Minor coherence nit: `FrameId` is `usize` while its
sibling arena handles `NsId`/`CommandId`/`VarId` are `u32`
(`tcl-core-types/src/lib.rs:114-126`).

## 3. Naming coherence & glossary currency

**Verdict: acronym casing is good (one outlier); whole-word abbreviation is
pervasively mixed — down to a single line — and a few core types are duplicated; the
glossary is conceptually strong but cites stale type names and is half Python-framed.**

### 3a — Acronym casing: consistent, one real break

A rigorous identifier-only sweep (excluding comments, string literals, and
upstream-crate spellings) found the acronyms are **casing-consistent** under Rust
convention: `Lsp`, `Wasm`/`WASM_`const, `Cst`, `Cfg` (`CfgModule`), `Gvn`, `Sccp`,
`Bigip`/`BIGIP_`const, `Oo`, `Ir`, `Vm`, `Uri`, `Http`, `Mro`, `utf16`(fn)/`Utf8`(variant).
(My initial raw grep flagged `Wasm`/`WASM` and `Lsp`/`LSP` splits, but those resolve to
correct Camel-type-vs-SCREAMING-const usage and env-var *strings*, not identifier
inconsistency — a good reminder to exclude non-identifiers.)

**The one genuine break: `SSA`.** `memory_ssa.rs:288` defines `MemorySSAFunction`
(all-caps) while the rest of the compiler uses Camel `Ssa` — `SsaFunction` (`ssa.rs:97`),
`SsaBlock`, `SsaStatement`, and the field `memory_ssa: Option<MemorySSAFunction>`
(`compilation_unit.rs:144`). Even `build_memory_ssa(ssa: &SsaFunction) ->
MemorySSAFunction` (`memory_ssa.rs:583`) mixes both spellings in one signature. It should
be `MemorySsaFunction`. This propagates through `dataflow_graph.rs` and
`optimiser/branch_folding.rs:458`.

### 3b — Whole-word abbreviation: pervasively MIXED (down to one line)

The abbreviate-or-spell-out choice flips not just across crates but **within one file,
one struct-copy, and one line** — the worst tier:

| concept | both forms, same crate | smoking gun |
|---|---|---|
| `idx` / `index` | `stmt_idx` (`var_observability.rs:179`) vs `stmt_index` (`optimiser/elimination.rs:432`) | `DeadStore { statement_index: e.stmt_index }` (`elimination.rs:586`) — one line |
| `arg` / `argument` | `arg_role`/`arg_index` vs field `argument_index` | `argument_index: arg_index` (`tcl-irules/src/walker.rs:121`) — one line |
| `var` / `variable` | `var_name` (`def_use.rs:252`, `cfg_builder/cfg_lower.rs:799`) vs `variable_name` (`type_infer.rs`, `ssa.rs`, `taint.rs`) | same concept, same crate, both spellings |
| `ns` / `namespace` | `current_ns` (`vars.rs:59`) vs `home_namespace`/`ensure_namespace` (`vars.rs:179,792`) | same file |
| `cmd` / `command` | loop var `cmd_name` (`tcl-vm/interp.rs:583`) vs `fn command_name` (`interp.rs:1985`) | same file; the trait method is long across the ABI seam, so call sites guarantee mixing |

`param`/`parameter`, `sig`/`signature` (`StubSig` vs module `signature_scan`),
`expr`/`expression`, `decl`/`declaration`, `def`/`definition` are all similarly split
(mostly cross-crate, more forgivable). **Highest-value, lowest-risk fixes:** the
`stmt_idx`/`stmt_index` and `argument_index: arg_index` one-line collisions, and the
`var_name`/`variable_name` split across the analyser.

### 3c — Three module conventions for "a Tcl command", and an arbitrary file-vs-dir split

"A Tcl command implementation" is spelled three ways at once: flat `cmd_*.rs` (31 files
in `runtime/rust/src/`, mirrored in `tcl-vm/src/`), **bare modules inside the
`cmd`-named crate** `tcl-cmd-core` (`array.rs`, `string.rs`, `namespace.rs`), and a
`commands/` directory (`tcl-registry`, `tcl-cli`, `f5-cli`). `namespace` alone lives as
`runtime/rust/src/cmd_namespace.rs`, `tcl-vm/src/cmd_namespace.rs`,
`tcl-cmd-core/src/namespace.rs`, *and* `runtime/rust/src/namespace.rs` (impl beside
adapter). And `commands/` means opposite things: ~2081 generated per-command metadata
leaves in `tcl-registry` vs a dozen hand-written subcommand handlers in the CLIs.

The analyser passes' **file-vs-directory split is uncorrelated with size**: `taint.rs`
is the single largest concern (**4007 LOC**) yet stays a flat file, while `inlining/`
(2867 LOC total) gets a whole directory. `taint` is itself fragmented across `taint.rs`
+ `taint_interproc.rs` (`tcl-compiler`) + a third `taint.rs` in `tcl-registry`. The
boundary is historical, not principled.

### 3d — Duplicated core types (the same concept, two structs)

Beyond the §2 `Diagnostic` ×2 and `Severity` ×3: **`MethodDef` is defined twice inside
`tcl-compiler`** (`ir.rs:601` and `analyser/types.rs:227`), and **`ProcDef` is defined
2–3×** (`tcl-vm/src/command.rs:29`, `tcl-compiler/analyser/types.rs:199`, and a runtime
form). The analyser's `emit_*` family is overloaded with `check_*`/`collect_*`/`scan_*`
siblings for the same "produce diagnostics" verb, and the analyser dispatch splits
`analyse_*` vs `handle_*` for the same walk role. These are navigation hazards: a reader
cannot tell which `ProcDef`/`MethodDef` a function means without tracing imports.

### 3e — Crate/module misnomers

- **`tcl-syntax` does not hold the syntax tree.** It parses Tcl *values* (backslash,
  glob, list, number, expr) — the actual CST (green/red tree) lives in
  `tcl-compiler/src/parsing/syntax/`. Two different "syntax" homes; the crate name
  invites the wrong file.
- **`tcl-lsp-rust` is a transitional alias**, not a crate — its `lib.rs` says
  "Transitional alias for `tcl_lsp_py` … retires in vNext" (ARCH9 split the real PyO3
  surface into `tcl-lsp-py`, see §0). Headers across the workspace still point at
  `tcl-lsp-rust` as the live binding (AQ §E).
- **No internal codename leaks into identifiers** (good): the `C41d*`/`ARCH8`/`ARCH9`/
  `SYNC-JUN-FRAME356`/`#527`/"strip"/"chunk" tokens appear only in comments/docs as
  provenance, never in struct/fn/module names.

### 3f — Glossary currency (`docs/GLOSSARY.md`, 1020 lines)

**Conceptually strong, mechanically stale.** It is a genuine Rust-pipeline glossary
(Phases 1–8 + alphabetic index + mermaid pipeline) and covers the right concepts well:
CST/red-green, IR, CommandSpec, SubCommand, FormSpec, CFG, SSA, Phi, dominator/idom,
SCCP, Lattice, Shimmer, type inference, def-use, data-flow graph, Memory-SSA, IPA, ICIP,
LICM, GVN, CSE, DCE, taint analysis/colour/source/sink, codegen, LVT. But:

1. **It cites stale type-name casing.** The pipeline diagram/sections name `SSAFunction`
   and `CFGModule`, but the code uses `SsaFunction` (`ssa.rs:97`) and `CfgModule`
   (`cfg.rs:313`). The glossary preserves the all-caps spelling the code abandoned —
   the same `SSA` casing inconsistency as §3a, now baked into the docs.
2. **It omits central Rust vocabulary:** `ValueOps` (the runtime-sharing trait — 0
   mentions), `salsa` (the incremental DB — 0), and it under-glosses `dialect` (1
   mention) despite the new Tcl 9.1 dialect and the dialect-type incoherence in §2.
3. **It is still ~half Python-framed:** 55 references to `.py` paths / `python` /
   `self._*` / `core.analysis`/`_analyser` across 1020 lines. For a Rust-only
   go-forward these provenance pointers should be re-aimed at the Rust types
   (`tcl_compiler::…`) so the glossary describes the shipping implementation.

**Verdict: not up to date.** Re-point the 55 Python references to Rust, fix
`SSAFunction`→`SsaFunction`/`CFGModule`→`CfgModule`, and add `ValueOps`/`salsa`/a proper
`dialect` entry. Conceptual coverage needs no work.

## 4. The explorer trio — CLI / TUI / GUI

**Verdict: this is the model of the sharing the maintainer wants — one core, three
thin surfaces, driving the real pipeline. It is the example the rest of the workspace
should follow, and it is the strongest part of this whole review.**

**One shared core, not three reimplementations.** Every surface funnels through a
single entry point — `tcl_explorer::run_pipeline(source, dialect) -> ExplorerResult`
(`tcl-explorer/src/lib.rs:109`) → `serialise_result(&result) -> serde_json::Value`
(`serialise.rs:1690`) — and is a thin presentation layer over that one JSON value:

- **CLI** (`tcl-cli/src/commands/explore.rs:33-44`): `run_pipeline` → `serialise_result`,
  then `--json` / `render::render_all` (`render.rs:56`) / compact summary — three views of
  the *same* value.
- **TUI** (`tcl-cli/src/tui.rs:173`): drives the shared `view_tree::build_view`
  `ViewNode` forest (`tui.rs:69,87`) — it does **not** re-render the pipeline.
- **GUI/WASM** (`tcl-explorer-wasm/src/lib.rs:28-32`): a **4-line** `#[wasm_bindgen]
  compile()` facade — `run_pipeline` → `serialise_result` → string, with a JSON-error
  fallback. It consumes `serialise.rs` exactly, so GUI and CLI **cannot drift**.

The `ViewNode` model (`view_tree.rs`) feeds both the TUI and the text renderer, and both
build from the serialised JSON, not the compiler types — so there is exactly one
serialisation and the three surfaces are structurally prevented from disagreeing.

**It drives the real pipeline and reuses the shared crates — zero re-implementation.**
`run_pipeline` builds the actual `tcl_compiler::CompilationUnit::build_for_with_config(…)
.with_interprocedural(…).with_memory_ssa()` (`lib.rs:119-121`); tokenises via real
`tcl_lexer::LexerConfig::for_dialect` (`lib.rs:115`); CST via real
`parsing::syntax::build::build_document` (`cst.rs:13`); bytecode/WASM via the real
`codegen_module`/`wasm_codegen_module` (`asm.rs:32`, `serialise.rs:195`); command/event
knowledge via `tcl_registry::registry_for_dialect`/`EventRegistry` (`lib.rs:110`,
`serialise.rs:1198`). A search for hand-rolled `tokenize`/`lex`/`segment` across all
three surfaces returns **only test-function names**. This is exactly the "don't
re-implement the lexer/registries in another layer" discipline the maintainer asked for
— already achieved here.

**Per-surface quality:** CLI exposes the full 26-view set (`views.rs:18`) incl. the
double-pipeline optimised re-run; **TUI** (417 LOC, not a god-file) is cleanly
feature-gated (`tui = ["dep:ratatui"]`, `tcl-cli/Cargo.toml:52`) so building without
`tui` pulls no ratatui and `run_tui` degrades to a `bail!`; **GUI** wasm glue is 32 LOC
with a `console.error` panic hook and a size-optimised profile. `tcl-explorer` carries
`#![forbid(unsafe_code)]` and only **6 clippy allows, all justified**;
`tcl-explorer-wasm` is intentionally **excluded** from the workspace (wasm-bindgen needs
`unsafe`, which the workspace forbids), and **CI enforces `wasm32` clippy with `-D
warnings`** (Makefile:840-844), transitively compiling the whole pipeline to wasm32.

**The only action items are cosmetic** (this section has no production blockers):
a **stale comment** at `serialise.rs:1803-1806` claims the rich per-instruction WASM
view "is not ported yet" — it *is* (the path runs at `serialise.rs:204`); the `--show`
filter is duplicated in 3 small spots; and `TREE_VIEWS` (`view_tree.rs:1125`) vs
`VIEW_META` (`views.rs:18`) keep two hand-synced view-id vocabularies (`event-order` vs
`eventOrder`) with no compile-time check — add a test asserting `TREE_VIEWS` ⊆
`VIEW_META`. The two large files (`serialise.rs` 2450, `view_tree.rs` 1228) are flat
per-view dispatch tables, not tangled god-files.

## 5. The "information" subsystem

"Information" resolves to three facets; two carry real parity bugs.

### Facet A — Info-severity (the "I" diagnostic family) — a confirmed parity bug

The canonical "I" codes are **I230** (constant branch) and **I231** (constant
switch-arm) — the only `I2xx` codes (everything else matching `I…` is the separate
`IRULE####` family). They are emitted on the **analyser** path
(`analyser/diagnostics.rs:7078-7084,7125-7131`), **hardcoded `severity:
Severity::Hint`** — because, per §2, the analyser's `Severity` enum has no `Info`
variant. The code even documents the shortcut: *"Severity is mapped to `Hint` because the
Rust `Severity` enum has no `Info` variant"* (`diagnostics.rs:7006-7008`).

The consequence is editor-visible: **Python emits these same codes as
`Severity.INFO → DiagnosticSeverity.Information`, but the Rust server renders them as
`Hint`** (`lib.rs:5762-5771`). Identical code I230/I231 shows an *Information* squiggle
under Python and a *Hint* squiggle under Rust — a round-trip parity defect. The fix is
mechanical and the pattern already exists one file over: the sibling
`compiler_checks::Severity::Info` **does** map correctly to `INFORMATION` (`lib.rs:5913`,
used today by S100 single-shimmer). Add `Info` to the analyser `Severity` (the §2
"one Severity in tcl-core-types" fix subsumes this).

**Plus a second, separate I230 false-positive gap:** Python suppresses constant-**true**
*loop* conditions (`while 1`, the idiomatic infinite loop) via an `is_loop` guard
(`_diag_branches.py:37,45`), but the Rust emitter computes only `is_switch`/`is_if`
(`diagnostics.rs:7036-7037`) — **no `is_loop` check** — so Rust emits a spurious I230 on
`while 1 {…}` that Python deliberately drops. (This is an FP-precision gap in the
same family as §F of the architecture review; add it to that list.)

The I-code *catalogue* otherwise matches Python (both define exactly I230 + I231, same
messages), and both are on-by-default in Rust.

### Facet B — the Tcl `info` command family — real coverage + correctness parity gap

The two Rust runtimes implement the `info` ensemble and **are not at parity**:

- **WASM runtime** (`runtime/rust/src/cmd_info.rs`): **29 subcommands**, prefix/ensemble
  resolver (`SUBS` table `:37-67`), all functionally implemented.
- **VM runtime** (`tcl-vm/src/cmd_info.rs`): **15 subcommands**, exact-match dispatch
  (`:22`), no prefix resolution.

**The VM is a strict subset (VM ⊂ WASM); 14 subcommands exist in the WASM runtime but
not the VM** — notably **all TclOO introspection** (`info object`/`info class`) and
**`info coroutine`**, plus `frame`, `errorstack`, `cmdcount`, `cmdtype`, `functions`,
`loaded`, `library`, `hostname`, `constant`, `consts`, `sharedlibextension`. Worse than
coverage, it's a **correctness divergence**: the VM hard-codes `tclversion → "9.0"`,
`patchlevel → "9.0.0"`, `nameofexecutable → ""` (`tcl-vm/src/cmd_info.rs:89,90,92`),
where the WASM runtime reads live state — and the VM's `9.0.0` even disagrees with the
WASM runtime's `9.0.3`.

**This is a registry-leverage failure (ties to AQ §A2/§C).**
The 15 common subcommands *are* shared via `tcl_cmd_core::info::*`
(`tcl-cmd-core/src/info.rs`), but the WASM runtime's 14 extras are hand-rolled with no
shared backing, and **neither runtime is registry-driven** even though
`tcl-registry/src/commands/tcl/info_.rs:291` already declares a full `CommandSpec` with
**28 subcommands**. The registry is the obvious single source of truth; both runtimes
ignoring it is *why* the VM silently drifted 14 subcommands behind.

### Facet C — LSP information providers — healthy

Hover (`hover.rs:113`), signature help (`signature_help.rs:98`), inlay hints
(`inlay_hints.rs:103`), and document symbols (`document_symbols.rs:133`) are all full and
advertised in `ServerCapabilities`. Two documented limitations: **workspace symbols** is
a "minimal port" operating over open in-memory docs rather than a true workspace index
(`workspace_symbols.rs:11,48`), and **`completionItem/resolve` is absent**
(`resolve_provider: None`, `lib.rs:3355`) — acceptable since completion items are fully
populated eagerly, but it precludes lazy documentation resolution. No `todo!()` in the
information-provider handlers.

## 6. Additions to the production-readiness roadmap

These extend the architecture-and-quality roadmap
(P0 = blocks deleting Python; P1 = ship-quality; P2 = throughput; P3 = hygiene).

**P0 — add to the "before Python is deleted" set:**

- **Include the FP precision suite in the live TEST-MIGRATE effort** (§0). The team is
  already porting Python tests to Rust; the 177 paired `test_fp_*.py` are the ones that
  guard precision and were skipped. This is the same gate as AQ §F P0-1, now with an
  obvious vehicle.

**P1 — ship-quality (correctness + parity):**

- **Consolidate the shared editor vocabulary into `tcl-core-types`** (§2): one `Severity`
  (Error/Warning/Info/Hint), one `Diagnostic`, a `DiagCode` enum, and `ByteOffset`/
  `Utf16Col`/`LineCol` offset newtypes. The single `Severity` fix *also* closes the
  Information parity bug (§5 Facet A: I230/I231 → `Information` not `Hint`) and makes the
  LSP `From` total; the offset newtypes structurally prevent the UTF-16 bug class from
  recurring (collapse the two UTF-16 implementations and route every range through
  `lift_span`).
- **Add the `is_loop` guard to I230** so `while 1 {…}` is not flagged (§5 Facet A) — an
  FP-precision item alongside AQ §F.
- **Make the `info` command family registry-driven and bring the VM to parity** (§5 Facet
  B): both runtimes should resolve subcommands from `tcl-registry`'s `info` spec (28
  subcommands) rather than hand-rolled tables; this fixes the `VM ⊂ WASM` 15-vs-29 gap and
  the VM's faked version constants, and is the same registry-leverage fix as AQ §A2/§C.
- **Re-point the PyO3/doc headers at `tcl-lsp-py`** (§0/§3e), not delete the PyO3 framing —
  the binding is live and transitional, the Python *core* is what retires.

**P2 — throughput (already in AQ, reinforced by the type view):**

- **Intern the compiler IR** (§2 item 5 = AQ §D2/P2-11): the `String`-keyed SSA/CFG/GVN is
  a *type-system* problem (`BlockId`/`ValueId`/`Symbol` newtypes) as much as a perf one;
  the `tcl-bytecode` interner is the template.
- **Thread `DialectSet`/`TclVersion` end-to-end** and delete the `&str` dialect parameters
  (§2 item 6) — especially relevant now that Tcl 9.1 just landed.

**P3 — hygiene (navigation):**

- **Naming cleanups** (§3): rename `MemorySSAFunction`→`MemorySsaFunction`; fix the
  one-line abbreviation collisions (`stmt_idx`/`stmt_index` in `optimiser/elimination.rs`,
  `argument_index: arg_index` in `tcl-irules/walker.rs`, `var_name`/`variable_name` across
  the analyser); dedup `MethodDef` (`ir.rs` vs `analyser/types.rs`) and `ProcDef`.
- **Refresh `docs/GLOSSARY.md`** (§3f): re-point the 55 Python references to Rust types,
  fix `SSAFunction`→`SsaFunction`/`CFGModule`→`CfgModule`, add `ValueOps`/`salsa`/`dialect`.
- **Wire `cargo deny` into `xtask`/CI** (§0 = AQ §D6) now that xtask is the tooling home.
- **Explorer cosmetics** (§4): fix the stale `serialise.rs:1803` "not ported" comment;
  add a `TREE_VIEWS ⊆ VIEW_META` test; unify the 3 `--show` filter copies.

### Bottom line

The goal's full surface is now reviewed across six documents (the coverage map in §1 is
the index). The four closing axes confirm the recurring shape of the whole review: **a
codebase that already contains the right pattern for nearly every problem it has.** The
explorer trio proves the team can build exemplary cross-layer reuse; the registry, the
`ValueOps` seam, and the `tcl-bytecode` interner prove the typed-vocabulary discipline
exists. The production work — and it is real, not cosmetic — is to **propagate those
existing patterns into the editor-facing half** (consolidate offsets/severity/diagnostic/
code/IR-identity into `tcl-core-types`), **drive the runtimes and the `info`/command
surface from the registry**, **build the Rust FP test net before Python is deleted**, and
**re-point the docs and glossary at the Rust that actually ships.** None of it is an
architecture rebuild; all of it is finishing the architecture that is already correct in
the half that doesn't touch the editor.
