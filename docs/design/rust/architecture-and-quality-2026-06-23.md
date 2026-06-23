# Rust architecture & code-quality review (2026-06-23)

> A whole-workspace architecture and code-quality review of the **Rust-only**
> go-forward (Python core and the Zig runtime are being retired). Companion to the
> [production-readiness assessment](production-readiness-2026-06-23.md) — that doc
> grades "can we ship Rust alone"; this one grades **how well the Rust is built**:
> sharing/DRY across layers, registry leverage, the buffer architecture, algorithm
> and data-structure choices, clippy-override discipline, dependency/MSRV currency,
> documentation accuracy, FP-catalog coverage, and configuration layering.
>
> Reviewed on branch `claude/exciting-planck-q7rj94`; claims anchored with
> file:line. Scope is the Rust workspace under `rust/` plus the go-forward WASM
> runtime `runtime/rust/`. `runtime/zig/` and the Python tree are **out of scope**
> (being deleted).

## Verdict

**The Rust architecture is genuinely well-shaped — the sharing discipline the
maintainer wants is, on the Rust side, already largely implemented and good.** The
crate graph is a clean acyclic layering; there is **one** Tcl tokeniser
(`tcl-lexer`) and the runtime already converged onto it; the two Rust runtimes
(`tcl-vm` bytecode VM, `runtime/rust` WASM runtime) **share** their command logic
through a `ValueOps` trait seam despite having opposite ABIs; and the
shared-vocabulary crates (`tcl-core-types`, `tcl-runtime-api`, `tcl-platform`,
`tcl-cmd-core`, `tcl-syntax`) are real, not bypassed.

**But "well-shaped" is not "production-ready," and the brief is the latter.** Read
harshly, the workspace has two genuine landmines that block retiring Python, plus a
soft dataflow core and a documentation tree that lies about the architecture:

1. **The precision contract has no Rust test net (the sharpest production risk).**
   FP.md's ~177 paired false-positive tests run **only against the Python analyser**;
   they `import analyse`/`get_diagnostics` and never invoke Rust (verified across all
   `tests/test_fp_*.py`). The Rust analyser is guarded solely by its own inline
   `#[test]`s, which cover many FP ids but **not the highest-risk gaps** — TNT-01,
   OBJ-10/OBJ-08, SH-08, RBS-07/09/11. **The day Python is deleted, those gaps lose
   their only guard and there is no Rust replacement asserting them** (section F).
2. **Config resolution silently disagrees with itself.** In a multi-root workspace,
   push-diagnostics honour per-folder `disabled`/`nonAscii` but hover / completion /
   code-actions / pull-diagnostics resolve from the **global** config — so the
   squiggles and the language features show different analyses of the same file, and
   `getEffectiveConfig` mis-reports what's in force (section G).
3. **The command *table* (which commands exist + arity + sub-commands) is still
   hand-maintained in the runtimes** instead of generated from `tcl-registry`, and
   the only drift gate checks the two layers being deleted (Python ↔ Zig). The
   registry is the source of truth for the *compiler/analyser* — lowering and codegen
   are fully registry-driven — but **not** for the runtimes, and ~30 analyser/LSP
   consumer sites re-hardcode command knowledge the registry already answers
   (section C).
4. **The dataflow core is built on the slow data structures.** Every one of ~2,546
   `HashMap`/`HashSet` uses is default-SipHash (FxHashMap: zero sites); the entire
   CFG/SSA/SCCP/GVN/taint layer is keyed by heap `String` block-labels and
   `(String, Version)`/`Vec<String>` value-keys with **no interner** (one exists in
   `tcl-bytecode`, unused by the compiler); and there are confirmed O(n²)/O(n³)
   hotspots (`compute_dominators`, GVN-PRE set clones, `dict_pairs`, `dict
   create`/`merge`) (section D).
5. **The source-buffer is re-derived, not shared** (~99 `LineIndex` rebuilds, the CST
   owning a second copy of the source), and the hand-rolled green/red CST **reinvents
   `rowan` without the structural-sharing that justifies the green/red split** — its
   own doc comments promise subtree reuse that its by-value green nodes make
   impossible (sections B, D8).
6. **The docs describe a retired architecture.** `AGENTS.md`, the design tree, and
   ~16 crate/module headers still name the Python registry as source of truth, the
   Zig runtime as shipping, a `TCL_LSP_RUST_ANALYSER` Python-dispatch gate, and core
   LSP features as "(future)" or "returns `MethodNotFound`" — all false today
   (section E). **No Rust-side layering contract** enforces the clean crate graph
   (`.importlinter` is Python-only), `deny.toml` exists but is **not wired into CI**,
   and **351 clippy-override sites** sit above the "absolutely minimal" bar (though
   the suppressions are almost all pedantic, not dangerous — section D5).

What is *not* wrong matters too, and is credited throughout: the crate graph is a
clean acyclic layering, edition 2024 / MSRV 1.96 is uniform with modern idioms and
zero nightly features, the `ValueOps` sharing seam is exemplary, lowering/codegen are
genuinely registry-driven, and the three landed correctness fixes (UTF-16 encoding,
document-version guard, panic containment) are real and verified. This is a **clean
foundation with a soft middle** — the structure is right; the dataflow data
structures, the test net, the config resolver, the runtime command tables, and the
docs are the production work.

Each section below is concrete and prioritised, anchored with file:line.

## A. Cross-layer sharing & DRY — strong, with one real gap

### A1 — What is already shared well (credit, because it is the answer to the concern)

The "are we re-implementing the lexer/primitives across layers?" worry is, for the
Rust go-forward, **mostly already solved**:

- **One tokeniser.** `tcl-lexer` is the single Tcl scanner. The WASM runtime's
  `runtime/rust/src/parse.rs` does **not** re-implement it — it lexes via
  `tcl_lexer::Lexer::new(s).tokenise_all()` (`parse.rs:336`) and lowers the
  canonical tokens into its eval `Command`/`Word` model ("the 'parse once'
  convergence"). The bytecode VM has no source tokeniser at all (it consumes
  `tcl-bytecode`).
- **One set of value/grammar primitives.** List parse/quote, number parse, expr
  parse+eval, glob, and backslash decoding all live once in `tcl-syntax` /
  `tcl-cmd-core` and are shared by the compiler, the LSP, *and* both runtimes
  (`runtime/rust/parse.rs:284,577`, `cmd_string.rs:1013`, `tcl-vm` `command.rs:11`,
  `interp.rs:15`). The regex engine is the one `tcl-regex` crate, consumed by the
  VM and the runtime via a C-ABI re-export.
- **`ValueOps` is the exemplar of "modify the architecture to share."** The two
  Rust runtimes have opposite command ABIs (`Completion<Value>` vs
  set-result-then-return-`Code`), so `tcl-cmd-core` exports the portable command
  bodies generic over a `ValueOps` trait, with `impl ValueOps for Vm`
  (`tcl-vm/src/value_ops.rs:20`) and `impl ValueOps for Interp`
  (`runtime/rust/src/value_ops.rs:24`) as the thin adapters. This is precisely the
  pattern the maintainer asked for, already in place.

### A2 — GAP (the maintainer's "registries properly leveraged"): the runtime command tables are hand-maintained, not registry-generated

`tcl-registry` (634 declarative `CommandSpec`s with arity + `SubCommand`s) is the
source of truth for the compiler and analyser — but the **runtimes re-encode the
same facts by hand**: `runtime/rust/src/builtins.rs` registers commands with arity
spelled inline per handler, and `tcl-vm/src/command.rs` does the same (~142
registrations). The only drift gate, `scripts/check/wasm_command_parity.py`,
cross-checks the **Python registry against the Zig runtime** — *both being
deleted* — so once Python and Zig are gone, **nothing verifies either Rust
runtime's command surface against `tcl-registry`.** There is even precedent that
the pattern was once codegen'd (`tcl-registry/src/.../mathop_generated.rs` cites a
`scripts/registry-audit/gen_missing_tcl.py` that no longer exists — a frozen
orphan).

**Recommendation (P0, high leverage, registry-leverage detail folded in from the
registry agent below):** add a `tcl-registry` codegen that emits the runtime arity
/ subcommand tables from `CommandSpec`, consumed by `runtime/rust` and `tcl-vm` for
validation (handlers keep their bodies, stop re-spelling bounds); **port the parity
gate to Rust, pointing it at `tcl-registry`** so it survives the Python/Zig
deletion and actually covers the Rust runtimes. This makes the registry the single
source of truth across *all* layers — the stated goal.

### A3 — GAP: no Rust-side layering contract

The clean acyclic graph (lexer → syntax → registry → compiler → {vm, runtime, lsp,
tooling}) is held by Cargo's acyclicity + header conventions alone. The repo-root
`.importlinter` enforces only the **Python** packages, and `deny.toml` has no
internal-edge rule. Nothing would fail CI if a future edit made `tcl-registry`
depend on `tcl-compiler`, or `tcl-cmd-core` pull in `tcl-bytecode` — the exact
diamonds the design avoids. **Recommendation (P3):** a `cargo metadata`-based
dependency-contract test (or a `cargo-deny [bans]` rule) wired into `check-rust`,
mirroring what `.importlinter` does for Python.

## B. The buffer architecture — the unified editor↔lexer↔CST↔runtime store

This is the integration-and-reuse opportunity the maintainer named, and it is real.
Today the source text is represented **four different ways with no sharing**:

| Layer | Representation | Cost |
|---|---|---|
| Editor (LSP) | `DocumentState { text: String }` (`tcl-lsp-server/src/lib.rs:114`) | re-cloned + re-spliced on **every** edit (`apply_content_change` double-allocates a `String`, `:5426`) |
| Lexer | `Lexer<'src>` borrows `&'src str` (`tcl-lexer/src/lexer.rs:291`) | zero-copy ✓ — but a *transient* borrow, not a persisted shared buffer |
| CST | `SyntaxTree { text: String, line_starts: Vec<u32> }` (`parsing/syntax/red.rs:57`) | **owns a second full copy** of the source per tree, rebuilt at each of **165** `segment_commands*` call sites |
| Runtime | `Value(Rc<Obj>)` with `Rc<str>` string rep (`tcl-vm/src/value.rs:22`) | COW values (correct for a runtime) |

The redundant re-derivation is large and measurable: **~99 `LineIndex::new`
constructions** (each an O(n) `\n` scan) and **38 `line_starts` builds** across the
workspace, plus the 165 CST rebuilds — so a single `did_change` → diagnostics →
tokens cycle re-scans and re-owns the document many times.

**The maintainers have already scoped this well.** `docs/design/rope/README.md`
(SRV-ROPE) is a measured design that reaches the right conclusion: a rope as a bare
`DocumentState` swap is *not* worth it (the paramount metric — time-to-first-tokens
— is a full-buffer `didOpen` a rope doesn't touch); the win is a **persisted,
incrementally-patched `LineIndex` on the existing `String` store** (most of the
edit/position-lookup win at ~zero memory cost — the recommended first step), and a
rope only later, as the *front of a chunk-aware incremental-analysis track* that
also makes the lexer, segmenter, and the salsa `SourceFile` input chunk-aware.

**The unified-buffer end-state to build toward** (synthesising the maintainer's
vision with the SRV-ROPE design):

1. **One owned source buffer per document, shared by reference.** A `SourceText`
   (an `Arc<str>` or, later, a rope) that carries its **own persisted
   `LineIndex`**, owned by `DocumentState` and handed to the lexer (`&str` view),
   the CST (hold `Arc<SourceText>` instead of copying `text: String`), the
   segmenter, and the analyser **by reference** — eliminating the 99 `LineIndex`
   rebuilds and the CST's duplicate `String`.
2. **Incremental `LineIndex` patch on edit** (SRV-ROPE Task 1) before any rope —
   splice the index instead of rebuilding it, and stop double-allocating the
   spliced `String`.
3. **Rope + chunk-aware salsa input** only when the analysis pipeline itself goes
   incremental (so the per-edit win is *analysis* reuse, not just text splicing).
4. **The runtime's `eval`/`subst` path** can borrow the same `SourceText` for
   dynamically-evaluated script fragments rather than re-owning substrings — closing
   the loop to the runtime buffer.

Status: design + experiment only; **nothing has landed**, so the duplication above
is the current reality. This is the single most architecturally-leveraged
performance/integration item, and it is well-understood — it needs building, not
re-designing.

---

## C. Registry leverage & lowering hookup — the core is right, the consumers cheat

The maintainer's explicit goal is "registries properly leveraged, lowering hooked to
the registry." **On the compiler hot path this is genuinely achieved and deserves
credit; the failure is at the edges** — ~30 analyser/LSP/runtime consumer sites still
hardcode command knowledge the registry already answers.

**What is correctly registry-driven (the hard part, done right):**

- **Lowering is fully hook-dispatched, with the string-match path deliberately
  deleted.** `lower_command` resolves a typed `LoweringHookId` from the `CommandSpec`
  and dispatches via `try_lower_hook` / `try_dispatch_structured_hook`
  (`tcl-compiler/src/lowering/mod.rs`); the comment at `lowering/mod.rs:1075-1084`
  states outright that *"the string-pattern `match cmd_name` block that used to handle
  these forms is gone."* There is **no `match cmd_name` bypass** — new commands are
  lowered by adding a registry hook id, not by editing a dispatch arm.
- **Bytecode codegen is the same shape.** `bytecoded.rs:39-47` resolves the
  `CodegenHookId` off the spec; the WASM backend resolves `WasmCodegenHookId`. The
  three typed hook-id enums (`LoweringHookId`/`CodegenHookId`/`WasmCodegenHookId`) are
  the registry-as-dispatch-table pattern done properly.

**Where the registry is bypassed (the gap, ~30 sites):** consumers that need "which
arg of this command is a variable write / a script body / a subcommand" re-derive it
with inline name matches and index literals instead of asking the spec. The registry
*already has the answer* for most of them:

- **P0 — variable-role index helpers re-hardcode `arg_roles`.**
  `analyser/diagnostics.rs::name_arg_indices`, the `var_scoping.rs` index helpers, and
  `tcl-lsp-core/src/declaration.rs` each special-case `set`/`lassign`/`global`/`variable`/
  `foreach`/`upvar`… with literal arg indices. The registry answers this exactly via
  `arg_indices_for_role(spec, ArgRole::VarWrite)` (the 3-tier `arg_role_resolver` >
  `arg_roles` > `assigns_variable_at` resolution the registry was built for). These
  are the highest-value rewires: they are precisely the data the registry exists to
  own, duplicated in the analyser.
- **Tier 1 (~11 sites)** have a direct registry answer: event-handler checks that
  string-match `when`/event names instead of reading `Traits::IS_EVENT_HANDLER`;
  event-ordering logic that hardcodes the iRules event order instead of
  `EventRegistry::master_order()`; subcommand/arity checks that re-walk literals
  instead of the spec's `SubCommand` table.
- **Tier 2 (~19 sites)** encode knowledge the registry *could* own but doesn't model
  yet (e.g. per-command "this arg is a glob pattern" / "this arg is an expr") — these
  need a registry field added first, then the consumer rewired.

**The VM is the outlier.** `tcl-vm` declares a `tcl-registry` dependency it does **not
use** for dispatch: `tcl-vm/src/command.rs` hand-registers ~142 commands with arity
spelled inline, dispatched through its own `HashMap`, while the unused registry dep
sits in `Cargo.toml`. Either wire the VM's command surface to `tcl-registry`
(validating arity/subcommands against the spec, per section A2) or drop the dependency
— shipping an unused declared dependency on the source-of-truth crate is the worst of
both.

**Recommendation (P0–P1):** rewire the P0 variable-role helpers and the Tier-1 sites
to the existing registry accessors (`arg_indices_for_role`, `Traits::*`,
`EventRegistry::master_order`, `SubCommand` tables) — this is mechanical and deletes
duplicated command knowledge. Add registry fields for the Tier-2 cases. Resolve the
VM's unused-dep contradiction. Net effect: the registry becomes the single answer to
"what does this command do with its args" across analyser, LSP, codegen, *and*
runtimes — closing the loop the lowering/codegen layers already model well.

## D. Code quality, algorithms & data structures — modern surface, soft dataflow core

**Verdict: the *surface* is modern and disciplined; the *dataflow core* is built on
the wrong data structures.** Edition 2024 / MSRV 1.96 is uniform, idioms are current
(~1,213 `let…else`, 348 let-chains, zero `extern crate`, zero nightly features), there
are **zero dangerous clippy suppressions** (no `unwrap_used`/`panic`/`indexing_slicing`
allows anywhere), and error-handling is broadly well-layered. But the compiler's
analysis passes — the thing that has to run on every keystroke — hash heap strings
with a cryptographic hasher, key everything by `String`, and contain confirmed
super-linear hotspots. None is a rewrite; together they are the difference between
"works on my 200-line iRule" and "production language server on a 10k-line file."

### D1 — SipHash everywhere, FxHashMap nowhere (free, broad, embarrassing-to-leave)

**2,546** `HashMap`/`HashSet` uses across 210 files; **`FxHashMap`/`FxHashSet`/
`rustc_hash` appear in exactly 0 of them.** Every map in the compiler uses the default
SipHash `RandomState` — cryptographic-strength, ~5–10× slower than FxHash for the
short-string / small-integer keys that dominate. The density is concentrated exactly
where it hurts: `analyser/diagnostics.rs` (177), `taint.rs` (125), `sccp.rs` (103),
`interprocedural.rs` (85), `ssa.rs` (79), `gvn.rs` (66) — the per-keystroke fixpoint
passes. The team *knew* and left it: `ssa.rs:1-2` says *"making them generic over
BuildHasher adds complexity for no real benefit,"* and several fns are **already**
generic over `S: BuildHasher` (`sccp.rs:140,390`, `subst_nocommands.rs:41`) yet are
**never instantiated with anything but SipHash** (zero `Fx`/`with_hasher` call sites —
unused plumbing). These maps face no untrusted keys; DoS-resistance is irrelevant.
**Fix:** add `rustc-hash` as a workspace dep, alias `FxHashMap`/`FxHashSet` across
`tcl-compiler`/`tcl-registry`/`tcl-lsp-core`. Near-zero risk, broad throughput win.

### D2 — String-keyed SSA/CFG with no interner (the real structural mismatch)

The whole analysis layer keys per-block and per-value data by **heap strings**:
`ssa.rs:41` `type ValueKey = (String, Version)`; `cfg.rs:161` `blocks: HashMap<String,
Block>`; `ssa.rs:103-109` `idom`/`dominance_frontier`/`dominator_tree` all
`HashMap<String, …>` (ssa.rs alone declares **42** `HashMap<String, _>`); GVN goes
further with `gvn.rs:45` `type ExprKey = Vec<String>` — *a heap vector of heap
strings* as a hash key. In the SCCP fixpoint inner loop (`sccp.rs:274-347`) this means
`sccp.rs:290` `executable_edges.contains(&((*p).clone(), bn.clone()))` — **two String
clones to build a tuple just for a set lookup**, per predecessor of every block, per
sweep; identical pattern in the intervals fixpoint. There is **no general string
interner in the compiler** — `rg "struct Symbol|Interner"` finds only domain-local
ones (`tcl-vm/src/interp.rs:454`, `tcl-bytecode/src/lib.rs:813` constant pool, the
WASM backend). **The enabling primitive (a `Symbol(u32)` interner) already exists in
`tcl-bytecode` and is simply not used by the compiler.** **Fix:** intern block names
to `BlockId(u32)` and variable/expr names to `Symbol(u32)`; then `ValueKey =
(Symbol, u32)` is `Copy`, per-block maps become `IndexVec<BlockId, _>`, and both the
hashing *and* the per-sweep String clones vanish. This is the single highest-leverage
compiler-throughput refactor.

### D3 — Confirmed super-linear hotspots

Independently verified, with file:line:

- **`compute_dominators` is O(B²·iters·|domset|) in String allocations**
  (`ssa.rs:311-360`): line 320 clones the entire reachable set into every block's
  init; `:348-353` do `dom[pred[0]].clone()` then `.intersection().cloned().collect()`
  per predecessor per sweep — and the file's own doc (`:305-310`) admits O(N²) memory /
  O(N³) worst-case. A **near-linear Cooper-Harvey-Kennedy `idom` already exists in the
  same file** (`~ssa.rs:365`); migrate callers to it and retire the set-based version.
- **GVN-PRE set intersections deep-clone every `Vec<String>` key, every block, every
  sweep** (`gvn.rs:1434-1440`). Fix: in-place `retain` + intern `ExprKey`.
- **`dict_pairs` is O(N²) on *every* dict operation** (`tcl-syntax/src/value.rs:217`):
  the default `ValueOps::dict_pairs` dedups with a `Vec<Rc<str>>` + linear
  `position()`. The VM does **not** override it (`tcl-vm/src/value_ops.rs:20`), so
  `dict get/exists/keys/values/size` all pay O(N²) to parse an N-entry dict; the doc
  says it also backs the WASM runtime's `TclDict`. The native `runtime/rust/src/dict.rs`
  already keeps an FNV-indexed map — **the good pattern exists one crate over and
  wasn't reused.** Fix: `HashMap<Rc<str>, usize>` → O(N).
- **`dict create`/`merge`/`replace` build loops are O(N²)** via linear `upsert`
  (`tcl-cmd-core/src/dict.rs:44,57-66,175-184,188-203`) — the normal way to build a
  large dict. `dict remove` is a milder O(|pairs|·|keys|) `Vec::contains` (`:216`).
- **`cfg_builder` dedup is O(n²) `Vec::contains`** (`cfg_builder/mod.rs:229-235,
  245-251`) — small-N in practice but a real data-structure choice; use a `HashSet`.
- **Inliner clones whole procedure bodies eagerly, per candidate, even if never
  inlined** (`inlining/mod.rs:643,636`), and `rewrite_script` clones every *unchanged*
  statement (`:724`) — on a module where little inlines, it deep-copies nearly the
  whole IR. Fix: `Rc<Procedure>`/`Rc<[Statement]>`, clone only at the splice site; give
  `rewrite_script` an `Option`/`Cow` no-change fast path.

The per-version lattice fixpoints (SCCP/taint/type-infer/intervals) are, by contrast,
**written correctly** — single in-place `HashMap<ValueKey, _>` mutation, no O(state)
snapshot per sweep; their only cost is D2's String keys.

### D4 — God-files and the 947-method god-struct

- `tcl-lsp-server/src/lib.rs` (**8,818 LOC**) is the entire server in one file —
  `impl LanguageServer` (`:2945-5108`) + `impl Backend` business logic (`:845-2942`) +
  ~120 free `lift_*` converters, with `Backend` carrying **20 `Arc<Mutex<…>>` fields**
  (`:570-687`, coarse lock-per-field). Extract `convert.rs` (the pure `lift_*` family,
  ~1,500 LOC), `capabilities.rs`, `commands.rs`, `cross_document.rs`, `config.rs`.
- `analyser/diagnostics.rs` (**13,569 LOC**) is cohesive (one concern) but oversized:
  one `impl Analyser` with **335 methods**, the worst being `emit_var_command_diagnostics`
  (~545 lines, `:8160`). It already has documented diagnostic-family seams (`:26-78`) —
  split into a `diagnostics/` directory along them.
- `runtime/rust/src/interp.rs` (**5,885**) folds ~10 orthogonal sub-domains into one
  ~4,670-line `impl Interp` (`InterpState` = 28 fields); `cmd_oo.rs` (**7,508**) bundles
  ≥7 TclOO concerns. Both mechanically splittable.
- The **`Analyser` god-struct** (`analyser/state.rs:77`): **34 fields, 947 methods**, a
  1:1 port of Python's cooperative-inheritance mixin. A whole Tk-checker working set
  (`tk_*`, `:127-139`) and the command surface live inline. Decompose into composed
  sub-states (`TkCheckState`, `CommandSurface`, `PendingDiagnostics`, `WalkPosition`,
  `AnalysisConfig`) — which also unblocks splitting the `diagnostics.rs` impl without
  every method borrowing all 34 fields.
- The `bigip/data/{s,l,a,g,n}.rs` files (32k/27k/20k/9k/8k) are **generated** (codegen
  banner line 1) and correctly left alone.

### D5 — Clippy overrides: high count, low danger, a few real masks

**351 per-site `#[allow(clippy::…)]`** + 3 justified workspace-level
(`Cargo.toml:110-112`). The honest read: **almost all are pedantic/style and
justified** — `not_unsafe_ptr_arg_deref` (32, all C-ABI), `implicit_hasher` (23, pub
generic fns), the cast lints (31+7+7+3, bit-pattern/numeric), `useless_conversion`
(6, PyO3 macro expansion). **No dangerous suppression exists** (`unwrap_used`,
`panic`, `indexing_slicing` allows: zero). `tcl-cmd-core/src/binary.rs` has the most
allows (11) and every one is annotated — a *model*. The genuine smells are narrow:
**`too_many_lines` (89) masks ~8 sequential god-functions** (worst:
`f5-cli/.../explain_flow.rs:1195` `format_report` 264 lines with zero match-arms;
`emit_var_command_diagnostics` 545 lines), and **`derivable_impls` (9) is generated
noise** (emit `#[derive(Default)]` from the generator → 0). Only ~40 of 351 carry a
justification comment. **Target ~325, not 0:** drive `derivable_impls`→0 in the
generator, split the ~8 god-functions, add 4–6 params-structs for the worst
`too_many_arguments` sites (`phi_can_undef` 10 args, `check_invocation` 11 args).
Chasing the pedantic lints to zero would only add noise — the "absolutely minimal"
bar should mean *minimal unjustified*, and the path there is ~26 removals plus
annotating the rest.

### D6 — Dependencies: one unmaintained, one stale, policy unenforced

- **`tower-lsp 0.20.0` is unmaintained (~2 years dead)** and drags in stale
  `lsp-types 0.94.1` + `dashmap 5.5.3` transitively. The active fork
  `tower-lsp-server` tracks `lsp-types 0.97` (LSP 3.17). **This is the single biggest
  staleness lever and a production-support risk** — migrate. (It is also why
  `type_hierarchy` can't return richer results — the 0.94 types lack the fields.)
- **`rusqlite 0.32.1` is 6 minors behind 0.38.0** — update (`tcl-cli`, bundled SQLite).
- `salsa 0.26.2` is **current** (Apr 2026 head) — *not* stale despite the low minor;
  keep. `wasmtime 46` / `pyo3 0.28` / `clap 4.6` / `thiserror 2` are current.
- **`deny.toml` exists and is well-formed but is NOT wired into CI** — no `cargo deny`
  invocation in the `Makefile` or any `.github/workflows/`, and its header comment
  claims a `make rust-deny` target that does not exist. The supply-chain policy is
  **decorative**. Wire it into `check-rust`.
- Duplicate transitive versions (`thiserror` 1+2, `bitflags` 1+2, `syn` 1+2) trace
  mostly to the optional `ratatui` TUI feature — bounded, not urgent.

### D7 — Error handling: one real boundary smell

Layering is broadly correct (binaries use `anyhow`; libraries with error types use
`thiserror`). The smell is **`tcl-bigip`**: a library that defines proper error types
(`ValueError` `value/error.rs:14`, `PcapError`, `TargetCollisionError`) **and** returns
stringly-typed `Result<_, String>` from **public** API on the same boundaries —
`redact::build_map` (`redact.rs:985`), `redact_secrets` (`:1412`),
`RedactionMap::from_toml` (`:568`), `pcapng::read_blocks`/`write_block`,
`f5_trailer::load_schema_overlay`, `flow::extract_flows` (13 stringly results). Return a
typed `BigipError`. `tcl-vm`/`tcl-pkg` stringly results are mostly internal — lower
priority.

### D8 — The hand-rolled CST reinvents `rowan` without the payoff

`tcl-compiler/src/parsing/syntax/{green,red,build,descend,segment}.rs` (~1,450 LOC) is
a from-scratch Roslyn/rust-analyzer-style green/red split — **not `rowan`** (no
`rowan`/`cstree` dep anywhere). The problem: it pays the green/red complexity cost
**without the property that justifies it.** Green nodes use **no `Rc`/`Arc`** (zero in
the whole `syntax/` dir): `GreenElement::Node` holds children **by value** in
`Vec<GreenElement>` with owned `String` fields (`green.rs:139,141,316,320`) — yet the
module doc (`green.rs:9-14`) claims subtrees are *"reusable verbatim across edits"* and
*"structurally identical regions… reusable."* **With by-value ownership that sharing
is structurally impossible** — every clone/edit deep-copies the subtree, the exact
thing `rowan`'s `Arc`'d hash-consed green nodes exist to avoid. Red nodes are
**not cached**: `SyntaxNode::children` (`red.rs:297`) recomputes offsets every call;
`tokens()` (`red.rs:324`) allocates a fresh `Vec` and re-walks the subtree every call.
And `SyntaxTree` stores a full reconstructed `text: String` + `line_starts` per tree
(`red.rs:57-58`). This is the clearest "reinventing the wheel with likely subtle bugs"
item in the workspace: the subtle round-trip conventions (the #527 empty-brace / `{*}`
ghost-token handling, trivia attachment) are exactly what a battle-tested library gets
right. **Recommendation:** adopt `rowan`/`cstree` (their `GreenNodeBuilder` + interner
deliver the sharing the comments promise and the incremental-reparse the LSP needs), or
— if the lexer-token-reshaping approach must stay — at minimum `Arc`-wrap green
children, cache red nodes, and **correct the doc comments that describe sharing that
doesn't exist.**

### D9 — Dead code: minimal (credit)

Only **1 genuinely removable item** (`tcl-bigip/src/graph.rs:57` `registry` field,
stored never read), **7 stale `#[allow(dead_code)]`** masking nothing (the `info`/debug
features they placeheld for have landed — `tcl-vm/src/frame.rs:36/39/43/49`,
`command.rs:42`, `tcl-lexer/src/lexer.rs:429` `warn_or_error` called from 8 sites),
and 2 legitimate WIP placeholders. **No `todo!()`/`unimplemented!()` in scope.** Delete
the 7 stale allows and the 1 field; otherwise this is clean.

## E. Documentation accuracy — the docs describe a retired architecture

**Verdict: the canonical docs are actively misleading about which architecture
ships.** This matters for production because the docs are the onboarding contract — a
new maintainer reading `AGENTS.md` today would believe the server is Python with a Zig
runtime and a two-method Rust bootstrap, all false. The *freshest* in-repo docs (the
2026-06-22/23 reviews, the parity audit) are accurate; staleness is concentrated in
the **mid-port** and **Python/Zig-framed** docs and the crate headers.

**Design-tree docs that overstate gaps or describe the wrong world:**

- **`AGENTS.md:5`** still opens "tcl-lsp is a Tcl LSP implementation **written in
  Python**"; its `:679-792` sections present `runtime/zig/` as the live WASM runtime
  and name Python lowering/codegen homes; `:219-316` documents a merge-blocking parity
  gate between the **Python registry and the Zig runtime** — all three layers being
  deleted. This is the highest-traffic doc and the most wrong.
- **`compiler-pipeline-parity.md:118-123`** lists ≥10 features as "Rust-missing/deferred"
  that are **all implemented**: IRULE1001 (`irules_event_checks.rs:287`), E001
  (`diagnostics.rs:3640`), W125 (`commands.rs:526`), IRULE5005 (`:548`), optimiser O128
  (`optimiser/end_offset.rs:193`), O130 (`chain_fold.rs:320`), snit OO
  (`commands.rs:406`+`oo.rs:85`). Its `:549,585` "phi.rs hardcodes S101" claim is fixed
  (`shimmer/phi.rs:139`).
- **`current-architecture.md:81-118`** describes the analyser as "default-on
  Python-supplemented" and the optimiser/GVN/interproc as "default-off Rust shims gated
  by `TCL_LSP_RUST_*` env vars with Python fallback" — the native server runs all of
  them ungated and is the default. (Ironically `:112` claims `TCL_LSP_RUST_ANALYSER`
  "does not exist in the tree" while it *does* exist as a stale comment at
  `analyser/mod.rs:29`.)
- **`rust-rewrite-registries.md`** (136 KB) snapshots the `tcl` registry at 126
  commands with "104 missing"; it is now 233 commands with only `ledit` missing.

**A doc-claim that is also a production correctness bug (cross-ref D/roadmap):**
`rust-optimiser-parity.md` presents the **O129 builtin-fold trust gate** as landed, but
`command_mutations` is populated **only in the test-path `optimise_raw`**
(`optimiser/manager.rs:625` is the *sole* assignment in the tree); production
`optimise_unit` (`manager.rs:70`) never sets it, so `try_o129_fold`
(`propagation.rs:1318`) reads an empty map in production. **The feature is dead on the
shipping path** — the doc says otherwise.

**Crate/module `//!` headers describing a Python-routed or bootstrap world (16+ sites,
worst first):** `tcl-lsp-server/src/lib.rs:9-13` ("every other LSP method returns
`MethodNotFound`" — the server is a full provider set); `analyser/mod.rs:26-31`
(PyO3/`TCL_LSP_RUST_ANALYSER` routing — retired); `var_escape/types.rs:289-295`
("`barriers` does not yet populate" — it does, `state.rs:123`);
`tcl-lsp-core/src/lib.rs:3-9` ("(future) completion, references, rename, semantic
tokens" — all shipped, lines 18/33/35/37); `document_symbols.rs:8-18,100-108` ("stay in
Python for now / the Python dispatcher materialises"); the `tcl-lsp-rust` references in
`tcl-lexer`/`tcl-registry`/`signature_scan` (now only a transitional alias for
`tcl-lsp-py`); `lexer.rs:21-25` (a skeleton header its own `:109-130` contradicts).
**These read as current architecture, not provenance.** Distinguish: the many "Mirrors
Python `…`" / "Ports `…`" notes are legitimate provenance (the porting source) and
should be left; only headers misrepresenting *current Rust state* need fixing.

**Recommendation (P1, low-effort/high-trust):** rewrite the `AGENTS.md` overview +
Zig/WASM-parity/codegen sections for Rust-only; refresh `compiler-pipeline-parity.md`/
`current-architecture.md`/`rust-optimiser-parity.md` (or mark them superseded by the
2026-06 reviews); fix the ~16 stale crate headers. The freshest review docs are the
ones to trust and should be linked from `AGENTS.md` as canonical.

## F. FP-catalog coverage — the precision contract has no Rust test net

**This is the sharpest production risk in the whole review.** FP.md is the
113-entry false-positive/true-positive precision contract — the thing that keeps the
analyser from crying wolf on real tcllib/iRules code. **The ~177 paired tests that
enforce it run *only* against the Python analyser** (every `tests/test_fp_*.py` does
`from analyser import analyse` / `from server.features.diagnostics import
get_diagnostics`; zero pyo3/subprocess/cargo references; verified across all 12 files +
`conftest.py`). The Rust analyser is guarded **solely** by its own inline
`#[cfg(test)]` tests (~37 FP-id mentions). Those cover many families but **not the
high-risk gaps.** The day Python is deleted, those gaps lose their only enforcement.

**Per-family coverage (cross-verified by three independent family audits + a synthesis
agent):**

| Family | Verdict | Highest-risk missing suppression |
|---|---|---|
| **NAB, DS, BND, INJ, RCH** | **PORTED** | none material — all TPs fire, suppressions wired |
| **RBS** (read-before-set) | **PARTIAL** | RBS-07 (dyn-named `namespace eval` body), RBS-09 (regexp/for-init in un-lowered switch arm), RBS-11 (`::foreach`/`::for`/`::while`) — no positive trace |
| **STY** (style) | **PARTIAL** | STY-03 (W104 usage-glyph `?optarg?`/`<placeholder>` exemption), STY-04 (W126 lassign downgrade) |
| **OBJ** (object dispatch) | **PARTIAL** | **OBJ-10** callback-array `$state(-command)` shape heuristic (only literal-harvest present); **OBJ-08** eval/W307↔W101 dedup |
| **SH** (shimmer) | **PARTIAL** | **SH-08** (`==`/`!=` both-non-numeric carve-out) |
| **TNT** (taint) | **PARTIAL** | **TNT-01** (expr direct-operand filter — exclude operands inside `[...]` cmd-subs) |

**Ranked regression risk once Python is deleted:**

1. **TNT-01** — `taint.rs::emit_expr_warnings` flags *every* tainted use in an `expr`,
   including operands inside command-subs (`expr {[string length $data] / 8}`); Python
   suppresses these. Concrete new false positives on common code.
2. **OBJ-10 + OBJ-08** — hits tcllib state-machine/callback-array code (HTTP/IRC/async
   modules keyed on `$state(-command)`); Rust will over-suppress *and* miss genuine
   W307, plus emit duplicate W307+W101 on eval-dispatch.
3. **SH-08** — spurious S100 numeric-shimmer on string `==`/`!=`.
4. **RBS-07/09/11** — spurious W210 on dynamically-named namespaces, opaque switch
   arms, `::`-qualified loop builtins (lower confidence: "no trace" ≠ "confirmed
   absent").
5. **STY-03/04** — minor spurious W104.

**Two corrections to earlier review claims (accuracy, since the brief is harsh):**

- **FP-DS-04 is NOT violated in the current tree.** The earlier production-readiness
  doc (E1) reported Rust emits W211 on a write-traced variable; the FP audit found the
  suppression **is wired**: `scan_scope_aliases` (`optimiser/elimination.rs:1031`) has a
  `"trace" =>` arm (`:1077-1098`) feeding the W211 skip (`diagnostics.rs:6131`) and W220
  skip (`:5942`). Either the earlier reproduction hit a narrower path (a specific trace
  form / top-level vs proc) or it predates the fix. **Action: settle it with a Rust
  regression test on the exact reproducer** — which is itself the point of section F.
- The "families with no `FP-XXX-NN` comment are unported" premise is wrong: the
  underlying W/O/S/T checks all exist; suppression is ported as logic + inline tests,
  not id-comments. The id-grep undercounts real coverage badly.

**Recommendation (P0 — this is the gate to deleting Python):** port the ~177 paired FP
tests to Rust integration tests (drive `Analyser::analyse` / the diagnostics path)
**before** the Python suite is deleted, prioritising the PARTIAL families. Until those
assertions exist in Rust, "retire Python" means "retire the only thing testing the
precision contract." Then implement the six missing suppressions (TNT-01, OBJ-10/08,
SH-08, RBS-07/09/11, STY-03/04).

## G. Configuration layering — two resolvers that disagree

**Verdict: partially coherent, not well-architected.** The editor-settings *push
diagnostics* path is centralised and correct; everything else reads a global config, a
prior cold-fallback divergence still exists, and the contract docs describe a config
system the Rust server does not implement.

**The real divergence (confirmed, prior finding still open):** push diagnostics
resolve the **per-folder** `AnalyserConfig` (`capture_job` →
`longest_folder_match(folder_db_configs, uri)`, `tcl-lsp-server/src/lib.rs:293-297`)
into `file_analysis_incremental` (`tcl-lsp-db/src/lib.rs:594-604`). But `analysis_for`
— the workhorse behind hover/completion/definition/references/rename/code-actions/
symbols/inlay — resolves disabled+non-ascii from the **global** config: on a salsa miss
it calls `analyser_config()` (`:1045`, no URI), on a hit it reads global `db_config`
via `db_file_analysis` (`:997`). **So in a multi-root workspace where a folder sets
`[diagnostics] disabled = W123` or `style.nonAscii`, the squiggles suppress W123 but
hover / completion / code-actions / pull-diagnostics analyse with the global set and
disagree with what's underlined.** The code-action disabled filter (`:4521`) is global
too, so a quick-fix can re-surface for a code the folder disabled.

**Supporting smells:**

- **Two resolvers:** folder-aware `resolved_analysis_settings(uri)` (`:2535`, used by
  push + pull base only) vs global `analyser_config()` (`:2523`, ~25 call sites). The
  split is the divergence's root.
- **`getEffectiveConfig` mis-reports:** `config-precedence.md:193` says it returns
  resolved per-folder values; `get_effective_config_command` (`:2129`) resolves dialect
  per-doc but reports **global** disabled/non-ascii (`:2150`) — the "trace where a
  setting comes from" tool lies about folder overrides.
- **`db_config` is not a single source of truth:** the global handle has one writer
  (`sync_db_config` `:952`), but only `capture_job` reads the per-folder handle;
  `db_file_analysis`/`db_document_symbols` read global. Per-folder config reaches the db
  for *push diagnostics only*.
- **The CLI has no config-file layering at all and a separate resolution path.** `tcl
  diag/lint/validate` build `Analyser::new()` and post-filter `--disable`/`--enable`
  (`tcl-cli/src/commands/diag.rs:71-90`; comment `:70` "sans config file"); `tcl opt`
  re-implements optimiser-profile layering (`transform.rs:93-128`) duplicating the
  LSP's `resolved_analysis_settings`. The only shared crate is `tcl-cli-support`
  (utilities, not config). No shared config-resolution module.
- **Doc/code mismatch (overlaps E):** `config-precedence.md` / `xdg-config.md` describe
  a 3-layer system (XDG `config.ini` + editor + project `.tcl-lsp.ini`) pointing at
  Python `server/settings.py`. The Rust server implements defaults + editor +
  per-folder-editor only — it *writes* `config.ini` for export but **never reads** it
  or `.tcl-lsp.ini`.

**What is clean (credit):** dialect resolution (`dialect_for_open` `:1136`), the
feature-toggle family (`feature_enabled` `:2326`, `will_save_format_enabled`,
`inlay_family_enabled`, `xc_diagnostics_enabled`), and `resolved_line_length` are all
folder-aware and single-chain.

**Highest-value fix:** make `analysis_for`/`cached_analysis`/`db_file_analysis`
URI-aware (resolve the folder `AnalyserConfig` the way `capture_job` does). One change
closes the divergence and fixes `getEffectiveConfig`, code-actions, and fix-all. Then
either implement the documented file layers in Rust or mark them Python-only, and
extract one shared "effective config" resolver for CLI+LSP.

---

## Consolidated production-readiness roadmap

Ordered by what actually blocks retiring Python, not by effort. P0 = blocks deletion;
P1 = ship-quality; P2 = throughput/polish; P3 = hygiene. (Robustness landmines — the
SIGABRT-on-deep-recursion server crash, fold-bomb OOM, and the Python-based
differential safety net — are catalogued in the companion
[production-readiness assessment](production-readiness-2026-06-23.md) §A/§C and are
assumed here; this roadmap covers the architecture/quality axis.)

**P0 — must land before Python is deleted:**

1. **Port the FP precision contract to Rust tests** (§F). The ~177 paired
   false-positive tests exist only in Python; porting them is the literal gate to
   deleting Python without regressing precision. Prioritise the PARTIAL families.
2. **Implement the six missing suppressions** TNT-01, OBJ-10, OBJ-08, SH-08, and the
   RBS-07/09/11 + STY-03/04 gaps (§F), each with a Rust assertion.
3. **Fix the config divergence** (§G): make `analysis_for`/`cached_analysis`/
   `db_file_analysis` URI-aware so features and squiggles agree in multi-root
   workspaces; fix `getEffectiveConfig`.
4. **Settle FP-DS-04** with a regression test on the maintainer's reproducer (§F) —
   reconcile the "still emits W211" repro against the now-present suppression.
5. **Wire O129 into production or document it off** (§E) — `command_mutations` is never
   populated on the shipping path, so the builtin-fold trust gate is dead.

**P1 — ship-quality:**

6. **Migrate off `tower-lsp 0.20`** (unmaintained, drags stale `lsp-types`/`dashmap`)
   to `tower-lsp-server`; update `rusqlite` 0.32→0.38 (§D6).
7. **Promote the registry to the runtimes' source of truth** and add a Rust drift gate
   against `tcl-registry` (the current gate checks the deleted Python↔Zig pair) (§A2);
   rewire the P0 registry-consumer sites (`name_arg_indices`, `var_scoping`,
   `declaration.rs`) to `arg_indices_for_role(...,VarWrite)` and resolve the VM's
   unused `tcl-registry` dep (§C).
8. **Correct the docs** (§E): rewrite `AGENTS.md` for Rust-only; refresh/supersede the
   mid-port parity docs; fix the ~16 stale crate headers — including the D8 CST
   comments that promise sharing that doesn't exist.
9. **Wire `cargo deny` into CI** (config exists, never runs) and add a Rust
   dependency-layering contract mirroring `.importlinter` (§A3/§D6).

**P2 — throughput & integration (the "soft core"):**

10. **Adopt `FxHashMap` across the compiler** (§D1) — free, broad, low-risk.
11. **Add a `Symbol(u32)`/`BlockId(u32)` interner** (one already exists in
    `tcl-bytecode`) and key CFG/SSA by dense ids (§D2); this also kills the per-sweep
    String clones.
12. **Replace `compute_dominators` with the in-file linear `idom`**; make GVN-PRE
    intersections in-place; fix `dict_pairs`/`dict create`/`merge` to map-indexed O(N)
    (the good pattern is in `runtime/rust/dict.rs`); `Rc`-share inliner bodies (§D3).
13. **Build the unified `SourceText`/`Arc<str>` + persisted `LineIndex`** (§B), starting
    with the incremental `LineIndex` patch (SRV-ROPE Task 1); evaluate `rowan`/`cstree`
    for the CST (§D8).

**P3 — hygiene:**

14. Split the `lib.rs`/`interp.rs`/`cmd_oo.rs` god-files and the `diagnostics.rs`
    mega-module along their existing seams; decompose the `Analyser` god-struct (§D4).
15. Drive the clippy allow count to ~325 (generator `derivable_impls`→0, split the ~8
    masked god-functions, params-structs) (§D5); delete the 7 stale `dead_code` allows
    and the 1 dead field (§D9); give `tcl-bigip` a typed `BigipError` (§D7).

### Bottom line

The Rust workspace is a **clean foundation with a soft middle and a missing test
net.** The architecture is right — acyclic crates, exemplary `ValueOps` sharing,
genuinely registry-driven lowering/codegen, modern edition/idioms, three real
correctness fixes verified. But "production-ready enough to delete Python" is a higher
bar than "well-shaped," and on that bar it is **not there yet**: the precision contract
has no Rust enforcement (P0-1/2), the config resolver disagrees with itself (P0-3), a
shipped optimiser gate is dead code (P0-5), the dataflow core runs on SipHash'd
String-keyed maps with confirmed O(n²)/O(n³) hotspots (P2), the source buffer and CST
duplicate work the design already knows how to share (P2-13), and the docs describe an
architecture that no longer exists (P1-8). None requires an architecture rebuild — but
the P0 items are non-negotiable gates, and shipping Rust-only before they land would
regress precision, confuse multi-root users, and strand a dead optimiser feature in
production.
