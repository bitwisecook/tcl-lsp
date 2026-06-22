# Rust workspace — full subsystem deep review (2026-06-22)

> A whole-workspace deep review of **every** Rust subsystem in `tcl-lsp`:
> architecture, layout, key algorithms, and code quality/style. Reviewed on
> branch `claude/exciting-planck-q7rj94` against the working tree at the date
> above. Review-only — no code was changed.
>
> Scope: all 31 workspace crates (~206K LOC). The native LSP server and its four
> supporting crates have a dedicated companion,
> [`lsp-server-deep-review-2026-06-22.md`](lsp-server-deep-review-2026-06-22.md);
> this document covers the whole tree and condenses the LSP findings into one
> section that cross-references it. It complements the earlier workspace-wide
> [`review-findings.md`](review-findings.md) (commit `1abc0d35`).
>
> **Priority order** (the maintainers'): correctness and precision first,
> performance second, memory third. Findings are ordered to match.
>
> **Method.** Each subsystem was deep-reviewed independently (the foundation
> crates; the compiler front-end, CFG/SSA/dataflow, analyser/checks, and
> optimiser/codegen; the registry/VM/bytecode/regex runtime; the F5/BIG-IP
> stack; and the CLI/tooling crates), with the VM trampoline, the regex matcher,
> and the workspace-level metrics verified first-hand. Every finding carries a
> `file:line` anchor.

## Verdict

This is a **large, serious, and unusually disciplined** Rust workspace — a Tcl
8.4–9.0 lexer, a red-green CST, an optimizing compiler (CFG, SSA, memory-SSA,
SCCP, GVN, taint, interprocedural, type-inference, interval-bounds, shimmer,
inlining, var-escape), a bytecode VM, a from-scratch port of Tcl's ARE regex
engine, a 169-struct F5 BIG-IP model, and a fleet of CLIs — built to a
consistently high bar:

- **`unsafe` is forbidden workspace-wide and the rule holds** — a grep finds
  zero `unsafe` blocks in the 29 in-workspace crates (the only hits are
  identifiers like `leading_hash_unsafe`). The three crates that genuinely need
  raw pointers (`editors/zed`, `tcl-explorer-wasm`, `runtime/rust`) are
  explicitly excluded with documented reasons (`Cargo.toml`).
- **Clippy pedantic is on workspace-wide**, with a small, sensible global allow
  set and 334 per-site `#[allow(clippy::…)]` across 206K LOC (densest in
  `tcl-compiler` 85, `tcl-bigip` 67) — a modest suppression rate.
- **~4,703 inline tests** plus per-crate integration suites. `tcl-compiler`
  alone carries 2,910 inline tests + 10 integration files; `tcl-lsp-core` 666;
  `tcl-lexer` 279; `tcl-bigip` 182; `tcl-registry` 157; `tcl-syntax` 134.
- **A differential-parity strategy** against the live Python implementation and
  real `tclsh` (8.4–9.0), with bytecode verified against `tclsh` disassembly.
- **Robustness-by-construction in two runtime hot paths**: the VM is a
  non-recursive **NRE trampoline** (a `Vec<Frame>` activation stack, not native
  recursion, so deep Tcl recursion can't overflow the Rust stack —
  `tcl-vm/src/exec.rs:467`), and the VM's opcode dispatch is panic-safe against
  malformed bytecode (bounds-checked `pc`, `checked_sub` for stack ops, guarded
  `split_off`).

But the bar is **not** met uniformly, and three themes stand out — all confirmed
by running code during this review, and one of which corrected an over-optimistic
first impression:

1. **Unbounded recursion → reachable stack-overflow crashes (SIGABRT,
   uncatchable) is pervasive in the recursive-descent paths.** The lexer, the
   VM, and every CFG/SSA *graph* traversal were deliberately made iterative — but
   the **source→IR and pattern-parsing recursive descent** paths have no depth
   discipline. **Four are confirmed reachable crashes:** the analyser body-walk
   (`tcl diag` aborts at ~600-deep nesting — reproduced here), the `expr` parser
   (`expr {((((…))))}` aborts — reproduced here), the CFG builder
   (`cfg_builder` `lower_*`), and the **regex parser** (`(`×4000 aborts with
   exit 134 — measured by the subsystem review). This is the workspace's single
   biggest correctness theme; see *Cross-cutting theme A*.
2. **The regex engine is *not* ReDoS-resistant — an earlier draft of this verdict
   said it was, and that was wrong.** The design is NFA-style set-simulation for
   the regular subset with backtracking reserved for backreferences, which
   *sounds* linear — but the implementation has **three** empirically-measured
   blow-ups: the regular-subset `reach` core is **O(n²)/O(n³)** because `search`
   re-derives the reachable set from every start position (`a*` on 8 000 chars →
   ~21 s; `(a*)*` → cubic), the backreference engine is **exponential**
   (`(a+)+\1$` doubles per added char, ~4 s at 20 chars), and the parser
   stack-overflows. See the registry/VM/regex section.
3. **The optimiser's source-rewrites have multiple confirmed miscompiles.**
   `optimiseDocument` / `tcl opt` can silently change a program's behaviour —
   five confirmed cases, three reproduced first-hand here: O122 emits a *braced*
   `lassign` that breaks every multi-argument tail-recursive proc; O129 folds
   renamed builtins (the trust gate is dead in production); O109/O126 delete
   `::`-qualified global writes; O103 folds a conditionally-returning proc; and
   the known callee-`uplevel` case. See *Cross-cutting theme D*.

The *rest* of the issues are, as expected, latent / robustness / style: a
handful of reachable panics in specific tools (the LSP `minify` path), the
rebuild-don't-reuse performance theme the prior review named, and a confined set
of integer-overflow edges. The engines underneath are sound — the analyser's MRO,
the SSA/dominance machinery, the VM trampoline, taint's conservative lattice, the
const-fold overflow discipline — so these are fixable defects in specific passes
and parsers, not architectural rot. Each subsystem section below gives its own
architecture summary and findings.

## Workspace architecture and layout

### Crate graph (a clean acyclic layering)

The 31 crates form a clean DAG, leaves at the bottom:

```
            tcl-core-types   tcl-lexer   tcl-platform        (leaves, no deps)
                  │             │            │
        tcl-runtime-api   tcl-syntax   tcl-host-native
                  │       │   │   │
              tcl-cmd-core │   │   tcl-registry ── tcl-bytecode
                  │        │   │        │              │
              tcl-regex    │   └──► tcl-compiler ◄─────┘   (the 140K-LOC core)
                  │        │            │   │   │   │
                  └────────┴──► tcl-vm  │   │   │   └─► tcl-irules ─► tcl-bigip
                                        │   │   │                        │
                       tcl-lsp-core ◄───┘   │   │              tcl-bigip-io / -query
                          │   │             │   │                        │
            tcl-lsp-db ◄──┘   ├─► tcl-explorer, tcl-cli-support,         │
               │              │      tcl-pkg, tcl-fuzz, tcl-debugger     │
        tcl-lsp-server ◄──────┴──────────────────────────────► f5-xc, f5-cli
               │
          tcl-lsp-py ─► tcl-lsp-rust (alias shim)
```

Notable edges: **the VM depends on the compiler** (it executes compiled
bytecode and shares the IR/expr evaluator); `tcl-compiler` is the hub that
`tcl-lexer + tcl-syntax + tcl-bytecode + tcl-registry` feed and that nearly
everything above consumes. The "pure crates + one PyO3 binding crate"
(`tcl-lsp-py`) rule is upheld — pyo3 appears in exactly one crate.

### Build, edition, and lints

- Edition **2024**, MSRV **1.96** (tracks the toolchain; ≥1.85 needed for salsa
  0.26). `release` profile: `lto = "thin"`, `codegen-units = 1`.
- `[workspace.lints.rust] unsafe_code = "forbid"`;
  `[workspace.lints.clippy] pedantic = warn` with `module_name_repetitions`,
  `missing_errors_doc`, `missing_panics_doc` allowed globally. Default rustfmt
  (no `rustfmt.toml`).

### Code conventions (and how well they hold)

The project's house style — UK spelling in identifiers/comments
(`normalise`/`optimiser`/`analyse`), no banner-style `// ----` dividers, `match`
for 3+ branch dispatch, command metadata on the registry `CommandSpec` rather
than hardcoded `frozenset`s in consumers — is followed closely. The exceptions
found are small and listed per subsystem (e.g. a US-spelled `finalize` in the
CST builder, stale banner comments in `tcl_expr_eval.rs`).

### The dominant structural theme (carried over from the prior review)

One root cause recurs across subsystems and is the single biggest *architectural*
lever: **the pipeline rebuilds rather than reuses.** The CST is build-once-
throwaway (re-lexed and rebuilt per `segment_commands*` call), the
`CompilationUnit`/taint is built 2–3× per document, `LineIndex` is reconstructed
at dozens of sites, and names are copied at every layer with no interner. None of
this is *wrong*; it is the cost model the incremental work (salsa in `tcl-lsp-db`,
the planned rope store, CST subtree reuse) is progressively buying down. Each
subsystem section notes where it pays this cost.

---

## Subsystem: compiler front-end (source → IR)

*Crate area: `tcl-compiler/src/parsing/`, `segmenter.rs`, `lowering/`,
`lowering_hooks.rs`, `ir.rs`, `tcl_expr_eval.rs` (~12.5K LOC).*

### Architecture & layout

The front-end turns untrusted document text into the IR in four stages:
**(1) Lexing + CST** — `tcl-lexer` emits a flat `Vec<Token>`; `parsing/syntax/`
re-shapes it into a red-green concrete syntax tree (green = position-independent
width+children, lossless; red = green anchored at an absolute position, lazily
resolved), mirroring Roslyn/rust-analyzer (`syntax-tree.md`). **(2)
Segmentation** — `segment_commands_local` builds the green document and derives
the public `SegmentedCommand`; the old hand-rolled token loop is retired to a
frozen differential oracle, with three recovery layers for unclosed
delimiters. **(3) Lowering dispatch** — `Lowerer::lower_command` dispatches in
strict priority order (registry hook → `{*}` structured barrier → structured-form
hook → generic `IRCall` with `ArgRole` resolution), honouring the
fall-through-to-`IRCall` contract for anything it can't prove safe. **(4) `expr`
folding** — `tcl_expr_eval.rs` shares the runtime's tree-walk and supplies value
ops; it is overflow-checked throughout, declines bignums/`rand`/regex, and keeps
raw text so `5.00 eq 5.0` stays correct. This is the strongest-tested part of the
front-end.

**Verdict: no confirmed correctness bug on valid input, and no reachable panic.**
Findings are latent/robustness/style.

### Findings

- **FE-L1 (Latent) — `eval [list …]` inner-text slice assumes symmetric
  delimiter width.** `lowering/mod.rs:1598-1602` strips `content_offset` bytes
  from *both* ends of the reconstructed `[inner]` word; correct only because a
  `Cmd` token always has `content_offset == 1`. Fragile coupling of leading- to
  trailing-delimiter width; the `else` branch's `trim_*_matches` form is robust.
- **FE-L2 (Latent) — `command_span` widening can be fooled by a trailing escaped
  brace.** `segmenter.rs:216-232`: the non-empty braced-final-word case still
  uses the raw `source[end] == '}'` byte check the comment admits is "fooled by
  an enclosing closer". `set x {a\}` / `{a\}}` is the exact case; worth a
  targeted test (feeds real `cmd.span`).
- **FE-L3 (Low) — `lower_when … priority <non-numeric>` silently drops the
  priority word.** `lowering/mod.rs:1367-1372`: a non-parsing priority leaves
  `base_priority` at the default while `body_idx` still picks the body; the IR
  misrepresents the command (runtime would error).
- **FE-R1/R2/R3 (Robustness)** — the dynamic-barrier gate re-segments nested
  bodies quadratically in depth (`lowering/mod.rs:380-458`); `split_tcl_list`
  mis-splits backslash-escaped spaces in `in`/`ni` folding, which can flip an
  "unreachable branch" warning (`tcl_expr_eval.rs:683-730`); `extract_oo_methods`
  uses a hardcoded `+1` body offset vs `+content_offset` elsewhere
  (`lowering/mod.rs:1822` vs `:1863`).
- **FE-P1/P2 (Performance)** — `eval_list_literal_body` /
  `eval_subst_nocommands_body` re-segment already-tokenised regions from scratch
  (`lowering/mod.rs:309`, `:1493`); a documented deferred optimisation.
- **FE-Q1 (Style) — stale banner comments pointing at empty regions.**
  `tcl_expr_eval.rs:401-411` etc. — "Core dispatch" / "Math function calls"
  headers over code that has since moved; both a house-style violation and dead
  noise.
- **FE-Q2 (Quality) — `incr` `safe_on_uninit` hardcoded `false`**
  (`lowering/hooks/incr.rs:39`, `mod.rs:1723`), diverging from the Python
  registry query; conservative (safe) but a tracked gap.
- **FE-S1 (Style) — US spelling `fn finalize` / `self.finalize()`** in the CST
  builder (`parsing/syntax/build.rs:229,341`) — lone deviation from the
  UK-spelled codebase. **Verified.**
- **FE-S2/S3 (Style)** — file-wide `#![allow]` of four cast lints in
  `tcl_expr_eval.rs:26-32` (could mask a future wrong cast); the `Statement` enum
  has 17 variants, the largest 12 fields — `Box`-ing the rare large variants
  (`Try`, `Switch`) would shrink every `Vec<Statement>` element (`ir.rs:145`).

---

## Subsystem: native LSP server (condensed)

*Crates: `tcl-lsp-server`, `tcl-lsp-core`, `tcl-lsp-db`, `tcl-lsp-py`,
`tcl-lsp-rust` (~46K LOC). Full detail in the
[companion document](lsp-server-deep-review-2026-06-22.md).*

### Architecture & layout

A `tower-lsp` `Backend` (8.8K-LOC `lib.rs`) implements ~40 `LanguageServer`
methods over the pure-Rust providers in `tcl-lsp-core` (hover, completion,
definition, references, rename, semantic tokens, inlay hints, code actions,
folding, formatting, call hierarchy, …). Derived facts are memoised by a salsa
0.26 query database (`tcl-lsp-db`). The binary is pure transport; all decision
logic lives in `Backend` and the providers. `tcl-lsp-py` exposes the pure crates
to the Python server via PyO3; `tcl-lsp-rust` is a one-release alias shim.

The server is **the most disciplined crate in the workspace**: production code
has zero `unwrap`/`expect`/`panic` outside tests, a documented
`documents → workspace_index` lock order, a monotonic per-document `revision`
guard, a debounced coalescing diagnostics scheduler, and `spawn_blocking` panic
containment on nearly every CPU-bound handler. The prior review's three
correctness headlines (UTF-16 positions, the version guard, inline-handler panic
containment) are all resolved.

### Headline findings (see companion for the full 18)

- **F1 (High)** — the diagnostics worker treats a deterministic salsa-query
  *panic* the same as a *cancellation* and retries it forever (50 ms livelock,
  diagnostics never published) — `tcl-lsp-server/src/lib.rs:416,445,2817`.
- **F2 (Medium)** — two reachable panics in `tcl-lsp-core/src/minify.rs`: a
  UTF-8 char-boundary slice on `expr {[é}` (`:2820`, contained by
  `spawn_blocking`) and a `usize` underflow on a `line 0` error reference
  (`:356`) on the inline `unminifyError` path.
- **F3 (Medium)** — `signature_help.rs:180` adds a UTF-16 column to a byte
  offset (wrong active parameter on non-ASCII lines) — the one residual encoding
  mix-up.
- **F6 (High value)** — the per-item salsa firewall is used only by diagnostics;
  every other feature re-runs the whole-file analyser per edit.
- **F13 (High, threaded callers)** — the PyO3 bindings hold the GIL across all
  Rust compute (zero `allow_threads`).
- **F15 (High)** — the `main`-branch CI runs no `cargo test`; the differential
  oracles are unprovisioned, so the parity harnesses silently skip.

---

## Subsystem: foundation crates

*Crates: `tcl-core-types`, `tcl-lexer`, `tcl-syntax`, `tcl-platform`,
`tcl-host-native`, `tcl-runtime-api`, `tcl-cmd-core` (~22K LOC). These are the
base of the DAG; correctness here propagates everywhere.*

### Architecture & layout

- **`tcl-core-types`** (131 LOC, `#![no_std]`, zero deps) — the value-less
  runtime vocabulary: the completion `Code` enum, `Completion<V>`, and opaque
  arena handles (`NsId`/`FrameId`/…). Total and correct.
- **`tcl-lexer`** (~8.5K) — the streaming O(n) Tcl tokeniser plus the position
  layer. A 16-byte `Copy` `Token` carries only a `Span` + `content_offset`;
  text/positions are resolved on demand via `SourceMap` (rust-analyzer's
  span-only model). `line_index.rs` does O(log n) offset↔(line, UTF-16) via a
  sorted line-start array; `structural_index.rs` is an O(n) brace/bracket/paren
  balance index for unmatched-delimiter diagnostics and error recovery (with a
  novel zero-width "ghost delimiter" recovery mechanism). **Empirically verified
  panic-safe** against 40+ adversarial/truncated inputs in a debug (overflow-
  checked) build.
- **`tcl-syntax`** (~5K) — despite the name, Tcl *value semantics* (not a CST):
  list parse/quote/join (`Tcl_ScanElement` parity), number parsing +
  `format_double`, glob (`Tcl_StringCaseMatch`), variable-name normalisation,
  and an `expr/` Pratt parser + constant folder.
- **`tcl-platform`** / **`tcl-host-native`** (271 / 312 LOC) — the host-capability
  seam (pure traits, wasm-clean) and its std-backed impl. `Env::set` uses a
  `RefCell` override map to avoid edition-2024 `unsafe set_var`. Clean.
- **`tcl-runtime-api`** (246 LOC) — contract-only role traits (`VarStore`,
  `Frames`, `Commands`, …) generic over an associated `Value`. No impls.
- **`tcl-cmd-core`** (~7.9K) — core command primitives (`clock`, `binary`,
  `scan`, `string`, `dict`, `lsort`, `lsearch`, `switch`, `regex` glue, `lseq`).

**Verdict:** the lexer/position layer and the four trait/type crates are
**excellent**. The actionable risk is concentrated in `tcl-cmd-core`'s integer
arithmetic and `tcl-syntax`'s value/expr layer.

### Findings

- **FN-C1 (Confirmed bug) — `format_double` drops `.0` for integer-valued
  doubles ≥ 1e16, and renders `-0.0` as `"0.0"`.** `tcl-syntax/src/number.rs:107`
  — `format!("{}.0", f as i64)` is gated on `f.abs() < 1e16`, so `1e16` →
  `"10000000000000000"` (re-parses as `Int`, breaking the round-trip contract);
  `tclsh` gives `1e16`/`-0.0`. **Verified.**
- **FN-C2 (Confirmed bug) — unchecked integer arithmetic on user-controlled
  numbers in `clock`.** `tcl-cmd-core/src/clock.rs:211` (`t + count * scale`),
  `:148`, `:294`, `:322` — `clock add 0 9223372036854775807 weeks` overflows
  (panics in debug/test, **silently wraps in release** — the release profile
  sets no `overflow-checks`). `tclsh` reports a clean overflow error. The sibling
  `lseq.rs` already uses `i128` intermediates, so these are omissions, not policy.
- **FN-C3 (Confirmed bug) — `binary scan` field-count multiply overflow.**
  `tcl-cmd-core/src/binary.rs:711,739` — `cur + n * size` overflows before the
  bounds check it guards (`n` from the user format string). Also `scan` field-
  width accumulation (`scan.rs:92`) and `regexp -start end+…` (`regex.rs:199`).
- **FN-H1 (Robustness, DoS) — catastrophic O(2ⁿ) glob backtracking.**
  `tcl-syntax/src/glob.rs:52-68` — `a*a*…a*b` (16 stars) vs 32 `a`s measured at
  **13.8 s**. Reachable from `string match`, `lsearch -glob`, `switch -glob`,
  `array names`, and the compiler's `matches_glob` constant folding — all on
  attacker-controllable LSP buffers. Faithful to the C recursion *shape* (so
  correct), but a single-resume-point iterative star-backtrack caps it at O(n·m)
  with identical results.
- **FN-H2 (Confirmed crash) — unbounded `expr` parser/eval recursion → stack
  overflow.** `tcl-syntax/src/expr/parser.rs:155,203`; `eval.rs:84` — no depth
  counter. **Independently reproduced**: `./target/debug/tcl diag` on a file
  containing `expr {((((…))))}` (9,000 parens) aborts with `stack overflow`
  (`rc=134`, SIGABRT). A second, distinct unbounded-recursion crash class from
  AN-C1, reachable through the same shipped CLI / LSP path. The module doc
  promises "never crashes on malformed expressions"; a depth guard returning
  `Raw` honours it.
- **FN-M1 (Style — stale doc only; the code is correct) — the `**`/unary
  doc-comment is wrong, but the behaviour matches Tcl.** `parser.rs:118`
  (`UNARY_BP = 24`) vs `:58` (`"**" => (23,23)`) makes `-2 ** 2` parse as
  `(-2)**2 = 4`, and the doc-comment claims the answer "should" be `-4`. **Checked
  against the reference**: `tclsh8.6` gives `expr {-2 ** 2}` = **4** and `~2 ** 2`
  = **9** (unary binds tighter than `**`), and the Rust folder gives **4** too —
  they agree. So this is **not** a precedence bug; the code is right and must not
  be changed. Only the doc-comment (and the agent intuition that "tclsh returns
  -4") was wrong. Fix the comment. *(A reminder that the parity oracles, not
  intuition, are the arbiter — the project's own discipline.)*
- **FN-L (Latent panics)** — `switch::select` underflows on empty `patterns`
  (`switch.rs:207`, `npairs - 1`); regex byte-offset slicing trusts engine
  offsets without an upper bound (`regex.rs:440`); `min_max`/`lseq`
  `unreachable!()` are sound only via non-local invariants.
- **FN-P (Performance)** — `dict create/merge/replace` are O(n²)
  (`dict.rs:36-54`, linear scan per insert); `string map` rescans the whole map
  per position (`string.rs:353`); `i32` level prefix-sum + 4 GiB `u32` offset
  ceiling in `structural_index.rs` are unguarded (realistically unreachable at
  editor scale).
- **FN-Q (Quality/Style)** — `structural_index.rs` copy-pastes its
  build+sort+merge body 3× (`:139`, `:521`, `:876`) and four near-identical
  brace-word scanners; stale module doc in `string.rs:8` ("Phase-1 proving
  subset"); banner-style `// ===`/`// ----` dividers in `structural_index.rs`,
  `parser.rs`, `list.rs`, and ~14 sites in `tcl-cmd-core` (house-style violation).

---

## Subsystem: analyser + diagnostic checks

*Crate area: `tcl-compiler/src/analyser/` (~38.8K LOC, the single largest module
in the workspace), `irules_checks.rs`, `signature_scan/`. The correctness-
critical heart of the LSP — every E/W/IRULE diagnostic originates here, held to
strict Python parity.*

### Architecture & layout

A 1:1 Rust port of the Python `_analyser` package, deliberately collapsing the
Python mixin traits into **one `Analyser` god-struct** (~50 fields) whose
behaviour is spread across ~20 `impl Analyser` blocks in a flat 22-file
directory. `Analyser::analyse` (`state.rs:494`) is a clean linear pipeline:
suppression pre-scan → stub overlay → dialect registry build → segment with
recovery → `walk_commands_top_level` (the central `process_command` dispatch) →
a fixed 13-emitter post-walk tail shared verbatim across all three entry points
(the single-source ordering that underpins incremental==fresh parity). The
semantic model (`AnalysisResult`) is a flat record; `Scope` owns its children
inline with the parent link held externally as a path of indices (a sound,
documented departure from Python's back-pointer, chosen for cheap snapshot).

**Factoring is mixed.** The leaf modules (`mro.rs`, `class_hierarchy.rs`,
`scope.rs`, `dispatch.rs`, `signature_scan/`) are cohesive, small, and heavily
tested. The mass is in `diagnostics.rs` (13.5K LOC, **88% tests**, ~43 `emit_*`
methods) and `state.rs` (4.1K), and in the god-struct itself, whose ~50 fields
mix three lifetimes of state (per-walk cursor, caches, and a large amount of
per-item-incremental bookkeeping that bleeds across the whole struct). The
**MRO is excellent** — a faithful port of Tcl 8.6/9.0 `tclOOCall.c` two-pass DFS
with mixin-first late-placement (the diamond test correctly asserts
`[D,B,C,A]`, not C3), cycles → error + single-element fallback, never panic.
Scope/namespace resolution correctly models Tcl's command-resolution rule
(a proc resolves commands in its *defining* namespace, not its lexical parent).

### Findings

- **AN-C1 (CRITICAL — confirmed reachable crash) — unbounded body-walk recursion
  → stack overflow (SIGABRT, uncatchable).** `commands.rs:82`/`:230`/`:732`
  (`analyse_body` ↔ `process_command` ↔ `dispatch_body_arguments`). `body_depth`
  is tracked but never bounded. **The reviewing agent built a probe and confirmed
  that `Analyser::analyse` on ~600-deep nested braced bodies aborts the process**
  with `fatal runtime error: stack overflow` (500 survives, 700 crashes, default
  8 MB stack, debug). A stack overflow is a SIGABRT — `catch_unwind` cannot
  contain it — so it takes down the whole LSP worker, and an LSP thread typically
  has a *smaller* stack, lowering the threshold. Generated/minified Tcl and
  machine-emitted iRules nest deeply. Note `param_traits.rs:126` already
  establishes the fix pattern (`MAX_DEPTH = 8` with a guard); the body walk
  simply doesn't use it. **Independently re-confirmed during this review**: the
  shipped `./target/debug/tcl diag` binary (the same analyser the LSP server
  drives) aborts on a 700-deep `if {1} { … }` file with `thread 'main' has
  overflowed its stack / fatal runtime error: stack overflow, aborting`. So
  `diag`, `lint`, `validate`, and the LSP diagnostics pipeline all crash on the
  same input. **This is the most severe finding in the workspace.**
- **AN-H1 (High) — no panic firewall around `CompilationUnit::build_for`.**
  `diagnostics.rs:~5491` (`emit_cfg_ssa_diagnostics`) builds the full
  lowering→CFG→SSA→interprocedural stack with no `catch_unwind` — the only
  `catch_unwind` in the analyser is around `unknown`-proc lowering
  (`oo.rs:630`). Any panic in that stack on adversarial input crashes the whole
  document's diagnostics. Firewall it the way the `unknown` path already is.
- **AN-M1 (Medium, convention) — hardcoded command/event metadata in check
  consumers.** Against the registry-is-source-of-truth rule:
  `irules_checks.rs:472` (event→side prefix mapping), `:599` (side-flip names),
  `diagnostics.rs:333`/`:950` (`set`/`incr`/`upvar`/… variable-name arg indices,
  **hardcoded and duplicated** between two functions). Works for standard Tcl,
  won't track registry/dialect extensions.
- **AN-M2 (Medium, perf) — the registry is rebuilt 3–4× per `analyse`.**
  `state.rs:565`, `state.rs:1225`, `diagnostics.rs:~5473`, plus a fourth in
  `builtin_command_names` (`state.rs:1203`) — each a full `build_default()` +
  `load_dialect`, on the per-keystroke path, despite the registry being stashed
  on `self.registry` during the walk. `irules_checks` similarly rebuilds
  `EventRegistry::build()` per call instead of its existing `OnceLock`.
- **AN-M3 (Medium, memory) — full `AnalysisResult` deep-clone per chunk** in
  `analyse_chunked` (`snapshot.rs:99`) — O(N chunks × result size).
- **AN-L (Low/Style)** — incomplete brace-escape handling in the hand-rolled
  `when`-block scanner (`irules_event_checks.rs:~834`, should reuse the
  segmenter); UK/US naming schism (`unnormalised` wrapping the registry's
  `-normalized`); several `#[allow(too_many_lines)]` god-functions (justified).
  Two earlier-suspected slice panics (`diagnostics.rs:3188`, `:2919`) were
  **verified safe** (ASCII-delimiter / length-guarded).

**Strengths:** exceptional test density (`diagnostics.rs` ~231 test functions),
correctness-solid MRO and scope resolution, the shared diagnostic-emission tail,
zero `unsafe`, consistently length-guarded indexing. The gap is specifically
unbounded *nesting* (AN-C1) and the unfirewalled CFG/SSA build (AN-H1).

---

## Subsystem: CLI and developer tooling

*Crates: `tcl-cli`, `tcl-cli-support`, `tcl-pkg`, `tcl-explorer`, `tcl-fuzz`,
`tcl-irule-test`, `tcl-debugger`, `tcl-irules` (~24K LOC). User-facing tools at
the top of the DAG.*

### Architecture & layout

- **`tcl-cli`** — the `tcl` binary: a thin `main` → clap-derive verb dispatch
  (exhaustive `match`, no not-implemented fallthrough) → `commands::*`, each
  resolving inputs via `tcl-cli-support` and driving a pure engine crate. A
  documented exit-code contract (0/1/2), JSON output run through `ensure_ascii`
  for Python `json.dumps` parity. Feature-gated Ratatui TUI over the explorer's
  `ViewNode` forest.
- **`tcl-cli-support`** — genuinely shared plumbing (input resolution, output
  writers, a CPython `SequenceMatcher`/`unified_diff` port, the shared registry
  cache). No copy-paste across verbs.
- **`tcl-pkg`** — the package manager; a Go-style MVS resolver (three-phase
  max-of-minimums BFS with convergence re-walk and a cycle guard), a safe-mode
  13-directive manifest parser, atomic lockfile writes, content-addressable
  store. **The best-hardened crate in the set** (see below).
- **`tcl-explorer`** — pure `run_pipeline(source, dialect) → ExplorerResult`
  (wasm-clean) over the compiler's `CompilationUnit`, producing the
  contract-shaped JSON the compiler-explorer views consume, plus a structured
  bytecode disassembly view.
- **`tcl-fuzz`** — a differential fuzzer: a grammar-aware bounded generator, a
  subprocess oracle comparing `tclsh` (reference) vs `tclvm` (subject), and a
  standout `wasm_diff` arm that drives compiled-WASM control flow with an
  embedded VM evaluating leaf commands so a divergence isolates a *control-flow
  miscompile* (fuel-bounded against non-termination, codegen panics caught).
- **`tcl-irule-test`** — TMM simulation by running the *actual* Python
  orchestrator Tcl on the VM (not a Rust re-implementation — the right call for
  fidelity); guest `exit` routed through a VM completion so it can't kill the
  host.
- **`tcl-debugger`** — record-and-replay stepping over a `tcl-vm` trace; a
  VM-independent breakpoint/step-mode core, fully unit-tested; a DAP server.
- **`tcl-irules`** — a small table-driven iRules object-ref resolver.

**Verdict: high-quality, faithfully-ported, idiomatic Rust** with consistent
error handling and essentially no panics on user input.

### Findings

- **CLI-M1 (Medium, operational) — the fuzzer silently no-ops (exit 0, "0
  findings") when `tclsh` is absent.** `tcl-fuzz/src/main.rs:406` `resolve_tclsh`
  never checks existence — it falls back to `PathBuf::from("tclsh")`
  unconditionally; if neither `tclsh9.0` nor `tclsh` is on `PATH`, every
  comparison is `Unavailable` → `Skipped` → no finding → **exit 0 while testing
  nothing**. Contrast `tclvm` (validated, errors if missing) and `wasm-check`
  (guards on `have_wasmtime()`). A CI gate on the fuzzer would report green
  having checked nothing. Validate the reference `tclsh` up front, or treat
  `skipped == total > 0` as non-zero.
- **CLI-L** — `--json` on `registry-dump` is parsed but ignored
  (`tcl-cli/src/lib.rs:142`); `Rng::below(0)`/`pick(&[])` panic in release on a
  zero argument (`rng.rs:36`, latent — not reachable via the CLI today);
  `findings.rs:122` writes an empty JSON sidecar on a serialize failure
  (`unwrap_or_default`).
- **CLI-N** — the Rust fuzzer deliberately omits the Python `bad_input_pct`
  script-corruption strategy (a documented scoping divergence — the "exercise
  error recovery" capability is absent from the Rust arm); `tcl-pkg` /
  `tcl-irules` omit the crate-level `#![forbid(unsafe_code)]` attribute (purely
  cosmetic — the workspace lint covers them).

**Strength — `tcl-pkg` security hardening is exemplary:** zip-slip guard
(`safe_join` rejects absolute paths + `..`), decompression-bomb cap
(`MAX_EXTRACT_BYTES` 256 MiB on tar *and* zip), symlink/hardlink-injection skip
on extract, bz2/xz explicitly rejected, and canonicalised CAS hashing (sorted
POSIX paths, masked modes) for cross-machine stability.

---

## Subsystem: compiler middle-end (CFG / SSA / dataflow analyses)

*Crate area: `tcl-compiler/src/cfg_builder/`, `ssa.rs`, `memory_ssa.rs`,
`dataflow_graph.rs`, `compilation_unit.rs`, `taint.rs`, `interprocedural.rs`,
`type_infer.rs`, `interval_bounds.rs`/`intervals.rs`, `side_effects.rs`,
`var_escape/`, `shimmer/`, `rendered_properties.rs` (~32K LOC).*

### Architecture & layout

`CompilationUnit::build_for_inner` drives lower→specialise→inline-uplevel→
build-CFG→per-function `FunctionUnit::build`, and each function runs the full
lattice stack (SSA → def-use → SCCP → type-infer → return-type → rendered-props
→ taint). Structured IR lowers to basic blocks; `try`/handler control flow that a
single-successor terminator can't express is carried as `Function.exception_edges`
and consumed by SSA (extra phi predecessors) and SCCP (extra reachability). **The
graph machinery is exemplary and defensively engineered**: SSA uses Cooper-Harvey-
Kennedy immediate dominators (cross-validated against a reference set-based
implementation in a test), dominance frontiers, and semi-pruned (Briggs) phi
placement; *every* graph traversal — RPO, the dominator-tree rename walk, the
memory-SSA walk — is **iterative with an explicit work-stack, specifically to
avoid stack overflow on huge generated CFGs**; and there is a
`COMPLEXITY_GUARD_BLOCKS = 20_000` / 256 KiB body cap.

**Fixpoint termination is confirmed for every analysis** (monotone transfer
functions over finite-height bounded lattices; intervals add a widening operator
plus a `MAX_ITERS = 50` backstop; call-graph closures are DFS guarded by a
`visited` set). **Taint is soundly conservative** — `join` uses union for
"tainted" (may) and intersection for mitigations (must), opaque commands
propagate taint from all arguments, and mitigations are only ever inferred from
*positive* evidence (never from absence). The interval/bounds analysis
(`intervals.rs`) is called out as an exemplary reference: cycle protection,
`checked_*` arithmetic with overflow→TOP, proper widening.

### Findings

- **MID-H1 (High — recursion DoS) — unbounded recursion in CFG construction.**
  `cfg_builder/mod.rs:470` `lower_script` ↔ `lower_if`/`lower_for`/`lower_while`/
  `lower_foreach`/`lower_switch`/`lower_try` are mutually recursive with **no
  depth guard** — ironic given the RPO walk was deliberately made iterative for
  the same input class. Same crash family as the analyser and expr parser
  (*Cross-cutting theme A*); also present in `interprocedural.rs`
  (`scan_statement`/`scan_script`) and `var_escape` (`handle_eval`).
- **MID-H2 (High — miscompile) — constant-return fold ignores implicit
  fall-through `""`.** `interprocedural.rs:1882` `summarise_returns` derives
  `constant_return` from explicit `Statement::Return`s only; lowering synthesises
  no implicit return for a fall-through body. So `proc f {} { if {$::c} { return
  42 } }` yields `constant_return = Int(42)`, and if `f` is pure the optimiser
  folds `[f]` → `42` (`optimiser/propagation.rs:1359`, and the independent
  `resolve_return_constant` at `:799` shares the gap) — but at runtime `f` returns
  `""` whenever `$::c` is false. **Independently reproduced**: `tcl opt` on
  `proc f {x} { if {$x > 0} { return 42 } }; set y [f $arg]` rewrites it to
  `set y 42` under **O103 "Fold pure-proc call to '::f' to its constant return"**,
  even though `f -1` returns `""`. This is a **silent miscompile in the
  user-facing `optimiseDocument` / `tcl opt` surface**, which raises its real
  severity above a typical latent bug. Fix: only treat a return as constant when
  *every* CFG exit is a folded return (no reachable fall-through to `exit`).
- **MID-H3 (High, latent) — `var_escape` leaves a static local alias `LOCAL`
  under a dynamic `upvar` source.** `var_escape/handlers.rs:57` skips any
  `upvar` pair where either side starts with `$`, so `upvar 1 $src dst` never
  escapes `dst`; emitting `dst` as a WASM local would be a miscompile. **Shielded
  today** (the only consumer, the inliner, gates on `pure_leaf` which is false
  when the dynamic-source flag is set), but the documented WASM-emitter contract
  reads `wants_frame()` directly and would hit it.
- **MID-H4 (High — security false-negative) — `writes_global` /
  `has_unknown_calls` are not transitively propagated.** `interprocedural.rs:905`
  copies both straight from local facts despite the field docs promising
  transitive propagation. A proc that writes a global only via a callee reports
  `writes_global == false`, so `propagate_taints` skips seeding its globals as
  tainted → **missed taint flow**. (Purity *is* soundly propagated, so this is
  not a GVN miscompile — only a taint false-negative.) Fix the fixpoint or the
  docs.
- **MID-M (Medium)** — taint is rebuilt 2–3× per document and the
  interprocedural-summary pass re-runs propagation `1 + params×15` times per proc
  (`taint_interproc.rs`), the dominant analysis cost (MID-M1); `var_escape` alias
  inference is a flow-insensitive literal map despite advertising SSA
  flow-sensitivity (MID-M2); `shimmer` recomputes loop structure super-linearly
  per pass (MID-M3); the `pure` fixpoint defaults an absent callee to *pure*
  (`interprocedural.rs:812`, latent landmine — `fixpoint_effects` correctly
  defaults to UNKNOWN) (MID-M5); `upvar` level detection uses a string compare so
  `upvar 01`/`#1` miss caller-frame classification (MID-M6).
- **MID-L (Low)** — dead switch fall-through code (`cfg_lower.rs:566` routes all
  fall-through switches to the opaque path, making `:578-595` unreachable);
  **78 banner-comment lines** across `taint.rs` (20), `side_effects.rs` (16),
  `memory_ssa.rs` (12), etc.; several stale doc-comments (`var_escape/types.rs:289`
  claims fields "not yet populated" — they are); large duplication between the
  `var_escape` intraproc and CFG drivers.

**Verdict:** no CRITICAL — no reachable miscompile in a safety-critical path and
no reachable production panic (all flagged `unwrap`/`expect` are test-only or
guarded). Well-architected and defensively coded; the action items are MID-H1
(recursion), MID-H4 (taint FN), MID-H2 (fold miscompile), then the cost and
cleanup items.

---

## Subsystem: registry, VM, bytecode, regex

*Crates: `tcl-registry` (~208K LOC, mostly generated), `tcl-vm` (~14K),
`tcl-bytecode` (~1.6K), `tcl-runtime-api` (~0.25K), `tcl-regex` (~3.6K). The
runtime execution stack. (Reviewed first-hand for VM/registry/bytecode; the
regex engine via a dedicated sub-review.)*

### Architecture & layout

- **`tcl-registry`** is THE source of truth for which Tcl commands exist and
  their arity/subcommands/arg-roles, and consumers *query* it (the
  `CommandRegistry` exposes ~40 query methods — `get`, `get_for_dialect`,
  `resolve_call`, `arg_indices_for_role`, `taint_source`, `is_unsafe`, …). The
  hand-written core is ~12.5K LOC (`registry.rs`, `events.rs`, `profiles.rs`,
  `spec.rs`, `taint.rs`, …); the other ~195K is **generated data** — 106K in
  `bigip/` (14 files) and 87K across 2,095 per-command files in `commands/`. The
  taint-source index is built at compile time as a fixed-size `const` array via
  `const fn` scans — a nice touch.
- **`tcl-vm`** executes the bytecode. The value model is `Value(Rc<Obj>)` —
  reference-counted copy-on-write, the correct Tcl object model. **Execution is a
  non-recursive NRE trampoline** (`exec.rs:467`): a `Vec<Frame>` activation stack
  driven by a loop with `Tick::{Continue,Call,Return,Tailcall}`, mirroring C
  Tcl's NRE so deep Tcl recursion can't overflow the Rust stack. The opcode
  dispatch (130 opcodes) is **panic-safe by construction**: `pc` bounds-checked
  before indexing, literal/LVT lookups via `try_from(...).ok().and_then(get)
  .unwrap_or_default()`, `checked_sub` for `OVER`, explicit `stack.len() < n`
  guards before every `split_off`.
- **`tcl-bytecode`** is the shared instruction model (130-variant `Op` enum,
  operands, literal/local-var tables) consumed by both codegen and the VM.
- **`tcl-regex`** is a hand-written, pure-safe-Rust port of Tcl 9's Henry-Spencer
  ARE engine: `defs.rs` (constants/errors), `ast.rs` (compiled `Node` tree +
  `CharSet`), `parser.rs` (1.6K-LOC recursive-descent lexer+parser), `exec.rs`
  (two matchers — a two-phase reach/dissect set-simulation for the regular subset
  and a separate backtracking `Bt` engine for backreferences).

### Findings

- **RT-C1 (Critical — recursion DoS, reproduced) — the regex parser stack-
  overflows on nested groups.** `parser.rs:1288` `parse` → `parsebranch` →
  `parseqatom` → `parse` per `(`, no depth counter. The reviewing sub-agent built
  the crate and ran `(`×4000: `fatal runtime error: stack overflow, aborting`
  (exit 134, uncatchable). The matcher (`dissect`/`reach`/`Bt::m`) is likewise
  unbounded-recursive over the tree. Reachable via `regexp`/`regsub` patterns.
- **RT-C2 (Critical — ReDoS) — exponential backtracking in the backreference
  engine.** `exec.rs:584` `m_star` / `:612` `m_backref` recurse with no step/
  visited bound. Measured: `(a+)+\1$` on `"a"×n` doubles per added char —
  n=18 → 1.0 s, n=20 → 4.0 s. Any backref pattern routes here.
- **RT-C3 (Critical — algorithmic) — the *non-backtracking* reach core is
  O(n²)/O(n³), not linear.** This is the finding that corrected the verdict:
  `search` (`exec.rs:227`) calls `reach(root, start)` for *every* start position,
  and `reach_seq`/`reach_repeat` aren't memoised (only `reach` is), so the
  per-start work doesn't collapse. Measured: plain `a*` on `"a"×8000` → **20.9 s**
  (O(n²)); `(a*)*` → O(n³), 800 chars → **30.3 s** — with **no backreference
  involved**. A genuine NFA state×position worklist would restore O(states·len).
- **RT-H (High)** — the `reach` memo keys on raw pointer identity
  (`exec.rs:106`, `std::ptr::from_ref(node)`) — sound today but a correctness
  landmine if any node is ever built transiently mid-match (RT-H1); the
  backtracking lookahead leaks inner captures (`exec.rs:481` keeps `self.caps`
  mutations written inside a zero-width lookahead) (RT-H2).
- **RT-M (Medium)** — the VM's dict is a list-of-pairs with O(keys) per-access
  lookup (no hash side-cache) — acceptable for a reference VM but O(n) per `dict
  get`; `-nocase` case folding and POSIX classes lean on Rust's Unicode
  predicates, which diverge from Tcl's own tables outside ASCII (documented
  approximations); `have()` in the regex parser is an unchecked `usize`
  subtraction (latent underflow, currently shielded).
- **RT-S (Style)** — `tcl-regex` uses US `Color`/`WHITE` identifiers (mirroring
  the C engine) against UK prose; the VM has banner comments (`// -- stack --`);
  `anchor_ok`/`is_word` are duplicated verbatim between the two regex engines;
  the regex test corpus has **no** pathological-input/resource-bound cases
  despite the engine running on adversarial input.

**Verdict:** the VM and registry are **excellent** — panic-safe dispatch, a
trampoline that can't stack-overflow, a clean query-driven registry. The **regex
engine is the workspace's most concerning single component**: three empirically-
reproduced DoS classes (parser overflow, backref ReDoS, and a superlinear
"regular" core), all reachable from `regexp`/`regsub` on attacker-controlled
patterns or subjects. Its *semantic* fidelity is high (544-case corpus); its
*resource* discipline is absent.

---

## Subsystem: F5 / BIG-IP

*Crates: `tcl-bigip` (~50K LOC, ~169-struct model + a config toolkit),
`tcl-bigip-io` (~1.4K), `tcl-bigip-query` (~19.8K query DSL), `f5-cli` (~15.8K),
`f5-xc` (~2.5K). Consumes untrusted `.conf`/SCF files; its canonical JSON is a
PyO3 cross-language contract.*

### Architecture & layout

`tcl-bigip` is more than a model: beyond the ~169 model structs (20 files under
`model/`) and the `parse_bigip_conf` driver, it carries a real config toolkit —
redaction (`redact.rs`), pcap remap/enrich, wireshark profiles, tmsh emit, iRule
context extraction, grep, lint, and policy evaluation. **`tcl-bigip-query` is a
genuine jq-flavoured query DSL** — a hand-rolled lexer + recursive-descent
precedence-climbing parser → an `Expr` tree → a streaming evaluator
(`eval.rs`), with projection of the untrusted model into navigable values,
in-place source rewriting (field-slot splices), side-input parsers (JSON/CSV/
f5log), and renderers (mermaid/gantt/ascii). It is, on balance, **well-hardened**:
user regex routes through a `safe_regex_compile` (1024-char cap + nested-
quantifier rejection) over the non-backtracking `regex` crate, iterative builtins
are capped at `ITER_CAP = 100_000`, graph traversal has cycle protection, and
model source-slicing uses defensive `source.get(a..b)`.

### Findings

- **F5-C1 (Critical — recursion DoS) — the query parser *and* evaluator are
  unbounded-recursive.** `parser.rs` recurses into `parse_pipeline` on `(`/`[`
  and self-recurses on unary/not; `eval.rs:284` `eval` recurses one native frame
  per nested form. A crafted query (`((((…))))`, deep pipes) overflows the stack —
  the query string is user input. Same theme as RT-C1/MID-H1/AN-C1.
- **F5-C2 (Critical — panic/overflow) — `i64` arithmetic in the evaluator is
  unchecked** (`eval.rs:1138/1189/1218`, `li + ri` etc.): `9223372036854775807 +
  1` panics in debug, wraps in release — diverging from the Python `int` it
  mirrors. **F5-C3** — integer-literal lexing `.expect()`s the `i64` parse
  (`lexer.rs:445`), so `99999999999999999999` (valid digits, out of range)
  **panics**. Both reachable from the query string. (Same overflow family as the
  foundation `clock`/`binary`/`scan` findings.)
- **F5-H1 (High — corruption/panic) — stale field-slot offsets when an identity
  rename and a field edit hit one source in one statement.** `edit_plan.rs:277`
  runs the rename first (mutating `current`), then `splice_edits`
  (`edit_plan.rs:477`, **direct** `&source[cursor..*start]` slicing) applies field
  edits whose offsets were computed against the *original* source → out-of-bounds
  or mid-codepoint panic, or silent config corruption. The existing guard only
  blocks prefix-cascade + field mixes, not rename + field.
- **F5-H2 (High — recursion DoS) — `jsonfmt` serialisation is unbounded-
  recursive** (`jsonfmt.rs:51`/`85`/`107`); a deeply-nested `--input-json` side
  value or in-query construction overflows the stack on render.
- **F5-M1 (Medium — cross-language contract) — `canonical.rs` `field_offsets`
  emits nondeterministic key order.** `canonical.rs:91` iterates a `HashMap`
  into a `serde_json::Map`; because `f5-cli`/`tcl-cli`/`tcl-bigip-query` enable
  `serde_json/preserve_order`, **Cargo feature unification makes
  `serde_json::Map` an insertion-ordered `IndexMap` for the whole graph** — so
  the HashMap iteration order leaks into the canonical JSON consumed via PyO3.
  The sibling `convert.rs`/`irule_context.rs` deliberately hand-roll insertion-
  ordered JSON to avoid exactly this; sort the keys (or use a `BTreeMap`).
  **Verified the mechanism.**
- **F5-L (Low)** — banner comments across `probes.rs`/`inputs.rs`/
  `projection.rs`/`runner.rs`; lexer float-overflow yields `inf` rather than a
  Python-parity error. (The query-engine sub-review verified several other
  suspected panics — gantt divide-by-`unit`, table-width indexing, traceroute
  `parts[0]` — are all **guarded/safe**.)

**Verdict:** a large but maintainable subsystem; the model is data-shaped (not
copy-paste logic), and the query engine is thoughtfully hardened *except* for the
shared workspace weaknesses — unbounded recursion (F5-C1/H2) and integer overflow
(F5-C2/C3) — plus the feature-unification determinism trap (F5-M1) and the
rename-edit aliasing hazard (F5-H1).

---

## Subsystem: compiler back-end (optimiser + codegen)

*Crate area: `tcl-compiler/src/optimiser/`, `gvn.rs`, `sccp.rs`, `inlining/`,
`inline_uplevel.rs`, `codegen/` (~33K LOC). The optimiser both rewrites code for
`optimiseDocument`/`minify` **and** emits the O-code editor suggestions; codegen
emits bytecode verified against real `tclsh` disassembly and (separately) WASM.*

### Architecture & layout

The optimiser is a fixed-order pass pipeline (Propagation → BranchFolding →
StructureElimination → ExprSimplify → PatternRecognition → Elimination →
CodeSinking → TailCall → UnusedProcs) over a `CompilationUnit`, each pass
producing `Optimisation { code, message, span, replacement, … }` records that are
*source rewrites* surfaced as O-codes and applied by reverse-offset edits. Passes
are gated by an `OptimisationProfile` (Readability < Standard < Full <
Aggressive; only aggressive runs to a fixpoint). **Determinism is carefully
engineered** — every `HashMap`-order-dependent step is followed by a total-order
sort, which is load-bearing for salsa early-cutoff. Codegen has two back-ends: a
bytecode emitter (byte-for-byte tclsh 8.4–9.0 parity, with peephole + jump-
shortening fixpoint) and a WASM emitter. **Crucially, codegen folds only literal-
constant expressions (empty-env), never through variables — matching tclsh — so
the bytecode-boundary const-fold contract is honoured.**

**Termination and panic-safety are sound** (SCCP is a bounded monotone fixpoint;
GVN/ADCE fixpoints are monotone; jump-shortening is capped at 10 iterations;
multipass breaks on `next == current`; no reachable panics in the passes — index/
arith are guarded). The problem is **semantics preservation in the source-rewrite
transforms.**

### Findings — confirmed miscompiles (the headline)

The reviewing agent reproduced four optimiser miscompiles on the built binary;
this review independently re-confirmed two of them plus O103 via `tcl opt`:

- **OPT-M1 (CRITICAL — reproduced) — O122 tail-call rewrite emits a *braced*
  `lassign`, breaking every multi-argument tail-recursive proc.** The rewrite
  produces `lassign {[expr {$a - 1}] [expr {$b + $a}]} a b` — the `{…}` is a
  braced word, so Tcl does **no** command substitution and `a`/`b` are assigned
  the literal strings `[expr {$a - 1}]` / `[expr {$b + $a}]` instead of their
  values. The design doc specifies `lassign [list …]` precisely to avoid this;
  the implementation used braces. **Independently reproduced** (`tcl opt --profile
  full` and `aggressive`). A "readability" rewrite in `optimiseDocument` that
  silently corrupts the program — the single worst defect in the workspace
  alongside the recursion crashes. `optimiser/tail_call.rs`.
- **OPT-M2 (CRITICAL — reproduced) — the builtin const-fold trust gate is dead in
  production.** `scan_module_command_mutations` is called only in the test-only
  `optimise_raw` (`manager.rs:625`); the production `optimise_unit` leaves
  `command_mutations` at its default, whose `trusts()` returns `true` for
  everything. So O129/O116/O118 fold renamed/redefined builtins. **Independently
  reproduced**: `rename string orig_string; puts [string length abc]` → `puts 3`
  under **O129**. (The proc-fold O103 path *does* check `redefined_procedures`, so
  the asymmetry pinpoints the bug.) One-line wiring fix.
- **OPT-M3 (CRITICAL) — direct writes to `::`-qualified globals inside a proc are
  removed as dead stores.** `elimination.rs:482-574` gates O109/O126 on
  `scope_aliases` but lacks a `var.starts_with("::")` guard (which the SCCP side
  and the manager's `couple_propagated_const_dead_stores` *do* have). `proc setit
  {} { set ::counter 42 }; setit; puts $::counter` → body emptied → `::counter`
  undefined. The declared form (`global counter; set counter 42`) is correctly
  kept.
- **OPT-M4 (CRITICAL, known/deferred) — callee `uplevel`/`upvar` writes to the
  caller frame are ignored.** `proc reset {} { uplevel 1 {set counter 0} }; proc
  run {} { set counter 10; reset; return $counter }` folds `return $counter` →
  `return 10` (O100) and deletes `set counter 10` (O126). Root cause: a user-proc
  call lowers to `Statement::Call`, not a `Barrier`, so SCCP never widens
  `counter` at the call site. Acknowledged in `docs/rust-optimiser-parity.md` as a
  deferred miscompile; still live. Needs interprocedural uplevel-write analysis.
- **(also independently reproduced) O103** — folding a conditionally-returning
  pure proc to its constant return ignores the implicit fall-through `""` (the
  CFG/dataflow section's MID-H2); `tcl opt` turns `set y [f $arg]` → `set y 42`.

That is **five distinct confirmed miscompiles in the user-facing
`optimiseDocument` / `tcl opt` surface** — see *Cross-cutting theme D*.

### Findings — codegen + other

- **OPT-C1 (High) — `[incr ::g 200]` value-position: 1-byte operand overflow +
  phantom-variable read.** `codegen/cmd_subst.rs:882` emits `INCR_STK_IMM`
  (1-byte signed operand) without the range-check the proc-local branch (`:846`)
  and `values.rs:360` apply; `[incr ::g 200]` overflows the operand, and an amount
  `> i32::MAX` falls to `load_var("3000000000")` — reading a variable *named* after
  the number. The correct reference sits 30 lines up.
- **OPT-C2 (Medium) — `end+N` constant index mis-encoded in range emitters.**
  `emit_inline_lrange`/`string range`/`string replace` emit `LIST_RANGE_IMM`/
  `STR_RANGE_IMM` without the `idx <= INDEX_END` guard `lindex` applies, so
  `lrange {a b c} 0 end+1` encodes a garbage index; `string replace … i32::MAX`
  also does `last_int + 1` (overflow). **Confirmed in source.**
- **OPT-H5 (High) — `inline_uplevel` (the production-wired inliner) lacks a
  redefinition/rename gate and under-approximates frame-reach** (`body_has_frame_
  reach` misses `namespace upvar`, `eval`-wrapped bodies). `inlining/mod.rs`'s
  value-inliner has the gate; the wired `inline_uplevel.rs:242` doesn't. Impacts
  the analysis/bytecode path.
- **OPT-L (Low/Style)** — the ADCE fixpoint doesn't re-apply the LHS suppression
  sets (compounds M3/M4); `scan_scope_aliases`'s `variable x y 1` parse misses a
  trailing no-value name; pervasive banner comments; `lrange`/`incr`/`array`
  emitters duplicated between value- and statement-position files (so C1/C2 exist
  in two copies).

### What is correctly preserved (substantial credit)

The agent verified — empirically — that many miscompile classes are handled
**correctly**: globals through opaque calls are not folded (SCCP forces `::`/
escaping names Overdefined); `expr "$a == $b"` with string values is not folded;
`$x * 0`/`+ 0` identities fire only when the operand is provably numeric; braced
words `{$x}` are never propagated; side-effecting RHS is never removed or sunk;
RMW targets keep their feeding `set`; integer-overflow folding **declines** via
`checked_*` rather than wrapping or panicking; floored division semantics are
correct; and codegen never folds through variables. The *engine* is sound; the
defects are in specific transforms' liveness/quoting/gating.

---

## Cross-cutting theme A — unbounded recursion is the workspace's dominant correctness risk

The single most important finding of this review is not in any one subsystem: it
is that **the recursive-descent and tree-walking paths have no depth discipline,
and several are confirmed-reachable stack-overflow crashes.** A stack overflow is
a SIGABRT — it **cannot** be contained by `catch_unwind` or the LSP's
`spawn_blocking` panic firewall — so each one crashes the whole worker/process.

The team clearly *knows* the hazard: the lexer is iterative, the VM is a
trampoline, the SSA/CFG/memory-SSA graph walks were all deliberately made
iterative with explicit work-stacks "to avoid stack overflow on huge generated
CFGs", and `param_traits.rs` already ships a `MAX_DEPTH = 8` guard. But that
discipline didn't reach the *parsing and lowering* paths. Confirmed/identified
sites:

| Site | Status | Anchor |
|---|---|---|
| Analyser body walk | **Reproduced** (`tcl diag` aborts ~600 deep) | `analyser/commands.rs:82/230/732` |
| `expr` parser / eval | **Reproduced** (`tcl diag` on deep `expr`, rc=134) | `tcl-syntax/expr/parser.rs:155,203` |
| Regex parser + matcher | **Reproduced** (`(`×4000, exit 134) | `tcl-regex/parser.rs:1288`; `exec.rs` |
| CFG builder `lower_*` | Identified | `cfg_builder/mod.rs:470` |
| bigip-query parser + eval + jsonfmt | Identified | `tcl-bigip-query/{parser,eval,jsonfmt}.rs` |
| interprocedural / var_escape walks | Identified | `interprocedural.rs`; `var_escape/` |

**Recommendation:** thread an explicit depth budget through each recursive
descent / tree walk and bail conservatively at a cap (the analyser already has
the pattern to copy), or run the analyser on a thread with a large explicit
stack as a stop-gap. Add adversarial-nesting regression tests asserting "returns
an error / diagnostic, does not abort." This one class of fix closes reachable
crashes in the LSP diagnostics path, the `tcl` CLI, the regex engine, and the
`f5` query tool at once.

## Cross-cutting theme B — unchecked integer arithmetic on untrusted numbers

A second, smaller theme: user-controlled integers are operated on with raw `+`/
`*` (panic in debug — the release profile sets no `overflow-checks`, so they
**silently wrap** in production). Confirmed in `tcl-cmd-core` (`clock`/`binary`/
`scan`), `tcl-bigip-query` (evaluator arithmetic + integer-literal lexing), and
the `format_double` `.0`-loss edge. The codebase already knows the fix —
`lseq.rs`/`intervals.rs` use `i128`/`checked_*` — so these are omissions. Tcl's
own behaviour is a clean overflow error, which `checked_*` → error would match.

## Cross-cutting theme C — rebuild-don't-reuse (performance), and house-style drift

The prior review's structural theme holds workspace-wide: the registry is rebuilt
3–4× per `analyse`, taint 2–3× per document, the CST per `segment_commands*`
call, and `LineIndex` at dozens of sites. Separately, **banner-style `// ----`
comments** (explicitly forbidden by the house style) are the most common style
violation — 78 lines in the middle-end alone, plus the foundation, regex, VM, and
F5 crates — alongside a few US-spelling identifiers (`finalize`, `Color`,
`unnormalised`) and stale doc-comments. None affect correctness; all are
mechanical cleanups that a `make check-all` lint pass could enforce.

## Cross-cutting theme D — the optimiser's source-rewrites have multiple confirmed miscompiles

Distinct from the recursion (A) and overflow (B) themes, and arguably the most
*user-visible*: **`optimiseDocument` / `tcl opt` can silently change a program's
behaviour.** Five confirmed cases (OPT-M1–M4 + O103), three reproduced first-hand
during this review. The common shape is a liveness/effect under-approximation
(a write that escapes the analysed frame — via `::`-qualification, callee
`uplevel`, or a conditional return — is treated as dead or constant) or, for
OPT-M1, an output-quoting bug. Because these transforms *rewrite the user's
source* (not just emit a hint), each is a correctness defect a user would hit by
running a one-click "optimise". The mitigations already in the codebase (the
`::`-guard in the manager coupling, the `redefined_procedures` gate in O103, the
SCCP Overdefined widening) show the team knows every one of these shapes — the
gaps are specific passes that don't consult the same guards. **Recommendation:**
treat semantic-preservation of the O-code rewrites as a release gate; the
existing differential harness (Python + `tclsh`) should grow before/after
*execution* equivalence checks for each O-code on an adversarial corpus (renamed
builtins, `::`-writes, `uplevel` callees, conditional returns, multi-arg tail
recursion), not just disassembly/segmentation parity.

---

## Consolidated findings (by severity)

Severity reflects reachability × blast radius. **[R]** = independently reproduced
during this review; **[r]** = reproduced by the subsystem reviewer on the built
crate.

### Critical — reachable crashes and silent miscompiles

| ID | Finding | Anchor | Status |
|---|---|---|---|
| AN-C1 | Analyser body-walk stack overflow (~600 deep) → SIGABRT crashes LSP/CLI | `analyser/commands.rs:82/230/732` | **[R]** |
| FN-H2 | `expr` parser recursion → stack overflow | `tcl-syntax/expr/parser.rs:155` | **[R]** |
| RT-C1 | Regex parser recursion → stack overflow (`(`×4000) | `tcl-regex/parser.rs:1288` | **[r]** |
| RT-C2 | Regex backref engine exponential backtracking (ReDoS) | `tcl-regex/exec.rs:584,612` | **[r]** |
| RT-C3 | Regex "regular" reach core is O(n²)/O(n³), not linear | `tcl-regex/exec.rs:227` | **[r]** |
| F5-C1 | bigip-query parser + evaluator recursion → stack overflow | `tcl-bigip-query/{parser,eval}.rs` | reviewer |
| OPT-M1 | O122 tail-call emits braced `lassign` → breaks multi-arg tail recursion | `optimiser/tail_call.rs` | **[R]** |
| OPT-M2 | Builtin const-fold trust gate dead in production (folds renamed builtins) | `optimiser/manager.rs:625` | **[R]** |
| OPT-M3 | `::`-qualified global writes removed as dead stores | `optimiser/elimination.rs:482` | **[r]** |
| OPT-M4 | Callee `uplevel`/`upvar` caller-frame writes ignored → bad fold/DCE | `optimiser/elimination.rs`,`sccp.rs:322` | **[r]** |
| MID-H2 | O103 folds conditionally-returning pure proc, ignoring fall-through `""` | `interprocedural.rs:1882`,`propagation.rs:1359` | **[R]** |
| LSP-F1 | Diagnostics worker retries a panicked query forever (livelock) | `tcl-lsp-server/src/lib.rs:416,2817` | companion |

### High

| ID | Finding | Anchor |
|---|---|---|
| FN-H1 | Glob O(2ⁿ) backtracking DoS (`string match`/`switch -glob`/fold) | `tcl-syntax/src/glob.rs:52` |
| FN-C2/C3 | `clock`/`binary`/`scan` integer overflow (panic debug / wrap release) | `tcl-cmd-core/src/{clock,binary,scan}.rs` |
| FN-C1 | `format_double` drops `.0` ≥1e16 / `-0.0`→`"0.0"` | `tcl-syntax/src/number.rs:107` |
| MID-H1 | CFG builder `lower_*` unbounded recursion | `cfg_builder/mod.rs:470` |
| MID-H3 | var-escape dynamic-`upvar` local alias stays LOCAL (latent WASM miscompile) | `var_escape/handlers.rs:57` |
| MID-H4 | `writes_global` not transitive → taint false-negative (security) | `interprocedural.rs:905` |
| F5-C2/C3 | bigip-query `i64` arithmetic + integer-literal `.expect()` overflow | `tcl-bigip-query/{eval,lexer}.rs` |
| F5-H1 | Stale field-slot offsets on rename+edit → OOB/corruption | `tcl-bigip-query/edit_plan.rs:477` |
| F5-H2 | jsonfmt serialisation unbounded recursion | `tcl-bigip-query/jsonfmt.rs:51` |
| OPT-C1 | `[incr ::g N]` 1-byte operand overflow + phantom-var read | `codegen/cmd_subst.rs:882` |
| OPT-H5 | `inline_uplevel` lacks redefinition gate; misses `namespace upvar` | `inline_uplevel.rs:182,242` |
| AN-H1 | No panic firewall around `CompilationUnit::build_for` in CFG/SSA diags | `analyser/diagnostics.rs:~5491` |
| LSP-F2/F3/F6/F13/F15 | minify panics; sig-help encoding; salsa firewall bypass; GIL; CI gap | companion doc |

### Medium / Low (selected)

Config-source divergence on the cold analysis path (LSP-F4); `cached_analysis`
deep-clone (LSP-F7); workspace-index deep-clone + O(n) retain per keystroke
(LSP-F9); registry rebuilt 3–4× per analyse (AN-M2); taint rebuilt 2–3×
(MID-M1); `canonical.rs` nondeterministic JSON key order via feature unification
(F5-M1); semantic-tokens delta nominal (LSP-F11); the regex memo pointer-identity
fragility (RT-H1) and lookahead capture leak (RT-H2); `dict`/`string map` O(n²)
(FN-P); fuzzer silent no-op without `tclsh` (CLI-M1); plus the pervasive
banner-comment style violations and a few US-spelled identifiers and stale
doc-comments across most crates.

---

## Prioritised roadmap

**Correctness — reachable crashes and miscompiles (do first)**

1. **Recursion depth discipline (theme A).** Add a depth budget to the analyser
   body walk, the `expr` parser, the CFG builder, the regex parser/matcher, and
   the bigip-query parser/eval/jsonfmt — bail conservatively at a cap (the
   analyser's `MAX_DEPTH = 8` is the pattern). Closes AN-C1, FN-H2, RT-C1,
   MID-H1, F5-C1/H2 — several confirmed SIGABRT crashes — at once. Add adversarial-
   nesting regression tests.
2. **Optimiser semantic-preservation (theme D).** Fix OPT-M1 (emit `lassign
   [list …]`), OPT-M2 (populate `command_mutations` in `optimise_unit`), OPT-M3
   (`::`-guard in O109/O126/ADCE), MID-H2/O103 (require all-exits-return), and
   schedule OPT-M4 (interprocedural uplevel writes). Add before/after *execution*
   equivalence to the differential gate for every O-code.
3. **Regex resource bounds (theme A + ReDoS).** Cap parser/matcher depth; add a
   step budget to the backref engine (RT-C2); and re-architect the reach core to a
   single state×position worklist so the regular subset is genuinely linear
   (RT-C3). All three are reachable from `regexp`/`regsub`.
4. **Integer overflow (theme B).** Convert `clock`/`binary`/`scan` and the
   bigip-query evaluator/lexer to `checked_*`/`i128` → clean error (matching
   `tclsh`); fix `format_double`.
5. **LSP robustness.** Split cancellation from panic in the diagnostics worker
   (LSP-F1); firewall the CFG/SSA build in the analyser (AN-H1); fix the
   `minify`/`signature_help` issues (companion doc).
6. **Taint soundness.** Propagate `writes_global` transitively (MID-H4).

**Performance** — consolidate on `file_analysis_incremental`; stop deep-cloning
`AnalysisResult` and `WorkspaceIndex`; thread one registry / `CompilationUnit` /
`LineIndex` per document; `RwLock` + `Arc` the read-mostly maps (companion doc +
AN-M2/MID-M1).

**Safety net** — run `cargo test --workspace` on `main`; provision the
differential oracles so they fail-not-skip; add the execution-equivalence O-code
corpus and adversarial-nesting/overflow fixtures (the gaps that let the confirmed
miscompiles and crashes ship).

**Quality** — one mechanical pass for the banner comments and US-spelled
identifiers (a lint could enforce both); fix the stale doc-comments (notably the
`**`/unary comment, which is wrong while the code is right — FN-M1).

## Related

- [`lsp-server-deep-review-2026-06-22.md`](lsp-server-deep-review-2026-06-22.md)
  — the LSP-layer companion (18 findings, full detail).
- [`review-findings.md`](review-findings.md) — the earlier workspace review.
- [`../../rust-rewrite.md`](../../rust-rewrite.md) — SRV-LSP / SRV-ROPE tracking.
- [`../../rust-optimiser-parity.md`](../../rust-optimiser-parity.md) — already
  tracks OPT-M4; this review adds OPT-M1/M2/M3 and O103.
