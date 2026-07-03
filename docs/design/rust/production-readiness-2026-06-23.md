# Production-readiness assessment — retiring Python (2026-06-23)

> **Update (2026): superseded — the blockers below were resolved and
> Python is now fully retired on this branch.** The distribution blockers
> (CLI/MCP shipping only as `.pyz`, editors launching `python3 …pyz`,
> `install.sh` installing only zipapps, `docker create` emitting
> `apt install python3` + `python3 tcl.pyz`) are gone: `tcl`, `f5-query`,
> `tcl-lsp-server`, and `tcl-mcp` ship as native binaries, all editors
> launch the native server, and the Python engine has been deleted. This
> is a **dated snapshot** from 2026-06-23 (branch
> `claude/exciting-planck-q7rj94`); its "Short answer: not yet" verdict
> reflects that moment, not the current (completed) state. Retained as
> the readiness-audit record.

> **Question:** can Python be retired, leaving the Rust implementation as the sole
> production product? **Short answer: not yet — and the gap is larger than the
> parity audit implied.** This assessment grades the Rust workspace against a
> *production-sole-implementation* bar (not a parity bar): can it ship, can it
> stand alone, will it survive untrusted input at scale, and what protects it
> once the Python oracle is gone.
>
> Reviewed on branch `claude/exciting-planck-q7rj94`. Every headline claim is
> reproduced against built binaries; file:line anchors throughout. Companions:
> the [workspace deep review](workspace-deep-review-2026-06-22.md), the
> [LSP-server review](lsp-server-deep-review-2026-06-22.md), and the
> [parity audit](python-rust-parity-audit-2026-06-22.md).

## Verdict

**Three independent classes of blocker stand between today and "retire Python":**

1. **The shipped product still *is* Python for everything except the VS Code LSP.**
   The native `tcl-lsp-server` exists and VS Code launches it — but the entire CLI
   distribution (`tcl`, `f5-query`, `tcl-wasm`, explorer, MCP, AI) ships **only**
   as Python `.pyz`, the Rust CLI binaries are **never built or shipped in CI**,
   three editors (Zed, JetBrains, Sublime) launch `python3 …pyz`, and the native
   `tcl`'s own `docker create` **generates Dockerfiles that install and run
   Python**. This is mechanical and well-bounded, but it is real release
   engineering, not a flip of a switch.

2. **The Rust implementation is not yet robust enough to be the *only*
   implementation.** A **single malformed document crashes the whole LSP server**
   (confirmed: SIGABRT), `optimiseDocument` has confirmed miscompiles that corrupt
   user code, and there are no recursion/size/time limits anywhere. With Python
   gone there is no fallback and no cross-check.

3. **The safety net is largely Python.** The differential tests that protect
   correctness compare Rust against the live Python implementation; deleting
   Python turns most of them into no-ops, and CI doesn't run `cargo test` on
   `main` at all. The thing that would catch a regression disappears with the
   thing it protects against.

None of this means the Rust core is bad — the analyser, SSA/dominance, the VM
trampoline, the registry, and taint's lattice are genuinely strong. But
"production-ready as the sole implementation" is a higher bar than "at parity",
and the distance is concentrated in **robustness, distribution, and the safety
net** rather than in feature coverage.

The rest of this document is the blocker inventory, grouped by class and ordered
within each by severity, with a consolidated "path to production" at the end.

---

## A. Robustness — the server must not die on untrusted input

### A1 — CRITICAL: one malformed document kills the entire LSP server (confirmed)

Driving the real native `target/debug/tcl-lsp-server` with a single
`textDocument/didOpen` carrying a 700-deep `if {1} { … }` nesting **aborts the
whole process**:

```
server process exit code after deep didOpen: -6   (SIGABRT)
thread 'tokio-rt-worker' (…) has overflowed its stack
fatal runtime error: stack overflow, aborting
```

The recursive-descent paths (analyser body-walk, `expr` parser, CFG builder,
regex parser) have **no depth guard anywhere** in the workspace, and a stack
overflow is a SIGABRT — **`spawn_blocking` and `catch_unwind` cannot contain it**,
so the panic-firewall the server relies on does not apply. The blast radius is the
**whole editor session**: every open document loses hover, completion,
diagnostics, everything, until the user restarts. It is trivially reachable
(generated/minified Tcl, machine-emitted iRules, or a hostile file nest deeply)
and, with Python retired, there is no fallback server.

This is the single most important item in this document. Reproduced three ways
this review: `tcl diag` (CLI) aborts at ~600 deep; `tcl diag` on `expr
{((((…))))}` aborts (rc=134); and now the **live LSP server** aborts on
`didOpen`. The fix is a depth budget threaded through the recursive walks (the
analyser already ships the pattern — `param_traits.rs` `MAX_DEPTH = 8`), or
running analysis on a thread with a bounded, explicitly-sized stack and treating
exhaustion as "decline to analyse this document."

### A2 — CRITICAL: constant-folding allocation bomb OOM-kills the server (confirmed, new)

`fold_repeat` (`tcl-registry/src/commands/tcl/string_.rs:67`) caps the *multiplier*
(`n > 10_000 → None`) but does `s.repeat(n)` with **no bound on `s.len()` and no
`checked_mul`** — and the optimiser folds inner `[...]` first and recurses
(`literal_words`, `optimiser/propagation.rs:1552`), so the folds **nest and
compound**. A one-line file is a multi-terabyte allocation:

```
$ tcl opt   # on: set x [string repeat [string repeat [string repeat A 10000] 10000] 10000]
memory allocation of 1000000000000 bytes failed   # 1 TB → rust_oom → abort (rc=134)
```

Reached on the optimiser surface — confirmed via `tcl opt`, and the LSP exposes the
same optimiser through the `tcl-lsp.optimiseDocument` and `tcl-lsp.fixAllSafeIssues`
execute-commands (a user clicking "optimise"/"fix all" on a crafted file aborts the
server), with the diagnostics path (`compiler_check_diagnostics → optimise_unit`,
`tcl-lsp-db/src/lib.rs:660`) traced as a passive trigger. `fold_lrepeat`
(`const_fold.rs:248`) has the same shape. An abort inside `spawn_blocking` kills the
whole process. **Fix: cap the *output* size (`checked_mul` on `s.len() * n`, bail
above a few MB) on every allocating fold.**

### A3 — CRITICAL/HIGH: no resource limits of any kind

A production language server handling untrusted text needs backpressure; this one
has essentially none. Unbounded today:

- **Recursion depth** — A1 (analyser body-walk `commands.rs:82`, nested `[[…]]`
  `commands.rs:1500`, IR lowering `lowering/mod.rs:617`, `expr` parser
  `tcl-syntax/expr/parser.rs:155`). No `recursion_limit` / `stacker` anywhere.
- **Allocation** — A2 (string-fold bomb).
- **Analysis time** — no per-document timeout / deadline; `Instant` is used only
  for a timing log (`lib.rs:387,533`), never to bound work. A pathological document
  with no follow-up edit runs to completion or hangs with no cap.
- **Document size** — `did_open`/`did_change` accept arbitrary text (no
  `MAX_BYTES`); a 100 MB buffer is fully re-analysed per keystroke and amplifies A1/A2.
- **Diagnostic count** — the analyser pushes to an uncapped `Vec`
  (`analyser/state.rs:800,834`); a file engineered to emit one diagnostic per token
  grows the payload with input size.
- **Regex** — the `tcl-regex` engine is O(n²)/O(n³) on the regular subset and
  exponential on backreferences. **Scoped correction:** this is *not* reachable
  from the LSP analyser (which never compiles the engine — `diagnostics.rs:1259`
  bails on any metacharacter and uses `str::contains`); it is a DoS for the
  VM/CLI execution tools (`tcl-vm-cli`, debugger, fuzzer, irule-test) only.
- **VM** — no instruction/fuel budget (`exec.rs:469`); an inline `while {1} {}`
  hangs `tcl-vm-cli` forever (only proc-call depth is bounded at 1000). VM/CLI-only.

### A4 — HIGH: the diagnostics worker livelocks on a deterministic panic (F1, still present)

`run_diagnostics_core` treats a panicked salsa query identically to a cancellation
(`Ok(None) | Err(_) => return false`, `tcl-lsp-server/src/lib.rs:416,445`; the
comment even says "*or the worker panicked*") and the scheduler re-marks the slot
dirty and retries every 50 ms forever — a CPU pinned at ~20 failed analyses/second,
no diagnostics published, the panic payload swallowed (no log). For any document
that deterministically triggers a *catchable* panic in a query, this is a silent
hang. Distinct from A1/A2 (uncatchable abort).

### A5 — HIGH: two CPU handlers still run inline on the event loop, and observability is near-absent

- **`cross_document_incoming_calls`** (`lib.rs:1640-1647`) calls
  `Analyser::new().analyse(...)` **inline** over *every* workspace document (up to
  the 2000-file scan cap) on the LSP event loop — both a panic-containment gap (a
  parser panic/abort here unwinds the event loop) and head-of-line blocking (one
  `incomingCalls` request reads-and-analyses hundreds of files synchronously). Every
  sibling cross-document handler correctly uses the pre-built index / `spawn_blocking`;
  this one is the outlier. **`will_save_wait_until`** (`:3253`) runs the formatter
  inline too (unlike `formatting`/`range_formatting`).
- **Observability:** only 5 `client.log_message` calls in 8.8K LOC, **no panic
  hook** anywhere, and failure paths swallow errors (`spawn_blocking(...).await
  .unwrap_or_default()` → analysis silently becomes empty, indistinguishable from a
  clean file; salsa helpers `.ok().flatten()`). A field crash (A1) produces **no
  diagnostic record at all**. For a sole-implementation production server this is a
  serious operability gap.
- **Salsa interned-table growth** (`tcl-lsp-db/src/lib.rs:175,253`) — the DB is
  created once and never swept; every distinct proc-body edit interns a key that is
  never reclaimed, so RSS climbs monotonically over a long editing session.

---

## B. Correctness — wrong results ship to users with no Python cross-check

These were graded as "findings" in the parity audit; under a *sole
implementation* bar they are **ship blockers**, because `optimiseDocument` /
`tcl opt` rewrite the user's source and there is no Python to catch a bad rewrite.

- **B1 — O122 tail-call rewrite produces a *hard `tclsh` runtime error*** on
  multi-argument tail-recursive procs (braced `lassign` → malformed list,
  `list element in braces followed by "]"`). Confirmed via `tcl opt`. **Rust-only.**
- **B2 — O109/O126 delete `::`-global writes** read by another proc (confirmed:
  `proc w {} { }` after eliminating `set ::counter 0`). **Rust-only.**
- **B3 — O129 folds renamed/shadowed builtins** (the trust gate is unwired in the
  production path). Confirmed. **Rust-only.**
- **B4 — minify panics** on adversarial input (char-boundary on `expr {[é}`;
  `line 0` underflow in `unminifyError`).
- **B5 — taint family severity is hardcoded to ERROR** — the whole taint/iRules-
  security family shows as red errors vs Python's warnings/info (confirmed on the
  live LSP); a security-diagnostic that cries wolf trains users to ignore it.
- **B6 — false `W123 Unknown command 'ledit'`** on valid Tcl 9.0 (the one missing
  registry command).

Each is small and localised (see the parity audit for fixes), but every one is a
*visible wrong answer in a shipped surface*, and the differential gate that would
catch a new one is being deleted (class C).

---

## C. The safety net is mostly Python and disappears with it

**Correction to the obvious fear:** the net does *not* wholesale collapse. Most
parity tests bake **committed goldens** into the repo (Python is provenance in a
doc comment, not a runtime dependency) — `rust/tcl-cli/tests/cli_parity.rs` (44
cases), the ~330 `*_parity.rs` across `f5-cli`/`bigip`, `registry_dump_parity.rs`
— all run `CARGO_BIN_EXE_*` against a `.golden` and **survive**. The tclsh- and
frozen-snapshot oracles also survive: `differential_fold.rs` (live `tclsh9.0`),
`differential_segment.rs` (frozen pre-CST oracle), `codegen_golden.rs` (Rust
goldens), the regex corpora (frozen `testregexp` TSV, 544+259 rows), the VM e2e
suites. **Exactly one** Rust test spawns Python at runtime
(`differential_codegen.rs`, `Command::new("python3")` at `:107`/`:141`) and it
**fail-opens** — Python absent → `eprintln!("skipped")` + `return`, i.e. it goes
**silently green**, not red.

**But the three things that die land squarely on the high-risk core:**

1. **The optimiser's only execution-equivalence guard dies.**
   `tests/test_optimiser_vm_equivalence.py` runs unoptimised vs optimised source
   through the VM and asserts identical output — the one test that would catch the
   confirmed miscompiles (B1–B3). The Rust optimiser has **292 inline tests**, but
   they assert the rewrite *fires* / the rewritten *text*, **not** that the
   rewritten program *behaves the same* — and the Rust VM **never executes
   optimiser output** (no `optimise` reference in `tcl-vm/`). So once the Python
   test dies, **nothing** checks optimiser semantics. This is why O122/O109/O129
   shipped, and it gets worse, not better.
2. **The bytecode emitter loses its only true reference oracle.**
   `differential_codegen.rs` (vs the Python emitter) goes silent, and
   `tests/test_bytecode_identity.py` (vs the **979 real-tclsh** `.disasm` files in
   `tests/bytecode_reference/{8.4,8.5,8.6,9.0}`) dies — and that corpus has **zero
   Rust consumers**, so it becomes orphaned data. What remains, `codegen_golden.rs`,
   is **self-blessed** (golden generated by the same Rust code): it catches churn,
   not a pre-existing miscompile.
3. **The registry has no Rust validation at all.** The baselines
   (`tests/baselines/registry/*.csv`, 5044 cmd rows) are **generated from Python**
   and the presence test drives the **Python** CLI (`_harness.py:73`) — so it
   never checks Rust (the `ledit` escapee proves it, B6). After Python the CSVs
   freeze, un-regenerable, and nothing validates the Rust registry.

Also dying: `scripts/dev/diag_parity/` (the diagnostics differential — no tclsh
oracle can replace it), the `KIND=python` LSP e2e path, and the Python-VM tcltest
gate.

**CI is genuinely broken for a Rust-only product.** `cargo test --workspace` and
the rust-backend `lsp_e2e` trigger **only on the `rust` branch**
(`rust-gate.yml:17`, `rust-lsp-e2e.yml:19`); `main`'s PR gate (`ci.yml` →
`make ci-fast`) runs **Python** lint + a `TCL_LSP_SERVER_KIND=python` e2e subset
and **never compiles the Rust workspace**. The moment `rust` merges to `main` and
Python is deleted, `main` CI would lint deleted Python and drive a non-existent
Python server, while nothing builds or tests the Rust product.

**Safety-net replacement work (must precede the cutover):**

- **P0a — rewire `main` CI to Rust:** run `cargo test --workspace --all-features`
  + the rust-backend `lsp_e2e` on `main` PRs/pushes; drop the Python lint/e2e gate.
- **P0b — build a Rust optimiser execution-equivalence harness:** run a corpus
  unoptimised vs `optimise`-applied through `tcl-vm`, assert identical stdout +
  result. The single most important new test (catches B1–B3).
- **P0c — Rust-native registry completeness gate:** drive native
  `tcl registry-dump` and assert `== tests/baselines/registry/*.csv`; port the
  baseline generator to read the Rust registry. Catches `ledit`-class drift.
- **P1 — restore the emitter's reference oracle:** a tclsh-only codegen
  differential (drive `tcl::unsupported::disassemble` live like
  `differential_fold.rs`, or wire a Rust consumer for the orphaned
  `bytecode_reference/` corpus); freeze the Python-generated goldens/baselines as
  Rust-owned fixtures with a `BLESS=1` re-bless path; port `diag_parity` to a
  committed Rust diagnostics-corpus snapshot test.

---

## D. Distribution & packaging — the shipped product is still Python

*(From the distribution audit — complete.)* **Retiring Python is not mechanically
possible today.** The Rust *build* is Python-free (all Python-generated Rust data
is committed; `cargo build -p tcl-lsp-server` and the VSIX need no Python), and
VS Code (+ all VS Code-compatible editors) is fully native. But:

| Surface | State | Blocker |
|---|---|---|
| VS Code LSP | ✅ native `tcl-lsp-server` | none — the migration reference |
| CLI `tcl` / `f5-query` | ❌ ships **only** as `.pyz` | Rust `tcl-cli`/`f5-cli` bins exist (`[[bin]]`) but **CI never builds/signs/ships them**; `install.sh` installs only `.pyz` |
| `tcl-wasm` | ❌ no standalone Rust binary; the in-tree Rust WASM emitter is a degraded **eval-fallback** (1.4K LOC vs Python's 20.6K) — confirmed: `tcl compwasm` on `fib` emits a 540-byte module that **can't instantiate** (`unknown import tcl::tcl_obj_new_string`, no `--link`) | port or drop |
| `tcl-explorer` (CLI+GUI) | ❌ Rust crate is **library-only** (no `[[bin]]`); GUI is Flask+Pyodide | CLI covered by `tcl explore`; GUI has no native replacement |
| MCP server, AI/Claude skills | ❌ Python-only, no Rust port | port or drop |
| Zed / JetBrains / Sublime | ❌ launch `python3 …pyz`, bundle a `.pyz` | migrate launchers to the native binary (VS Code is the template) |
| `tcl docker create` | ❌ the **native** `tcl` emits Dockerfiles that `apt install python3` + `python3 tcl.pyz …` (`tcl-pkg/src/docker.rs:208,246,262`) | rewrite to install the native CLI |
| `tcl-vm` / debugger | ❌ unshipped on both sides | decide scope |
| Installer `install.sh` | ❌ installs **only `.pyz`**, zero native binaries | rewrite for native assets |
| `tcl` CLI package resolution | ❌ "package resolution is not yet implemented in the Rust port" (`tcl-cli-support/src/input.rs:184`) | port |

The good news is this class is *engineering*, not research: every gap has a clear
shape (build the Rust binary in CI, repoint the launcher, rewrite the installer).

---

## E. Compiler — strong engine, but it ships noise and one degraded back-end

The compiler is the most impressive part of the workspace (a real optimizing
pipeline: CST → IR → CFG → SSA → memory-SSA → SCCP/GVN → taint/interprocedural →
bytecode), and core codegen/lowering is genuinely solid — the compiler review's
~70-case differential vs `tclsh9.0` matched on every common-to-moderate construct,
and the emitted **bytecode, diagnostics order, and JSON/graph/registry output are
deterministic** (the code carefully sorts HashMap-order results). One exception:
**`tcl dis` / `asm` *text* is nondeterministic** — label-comment and switch
JUMP_TABLE iteration are unsorted (`tcl-bytecode/src/format.rs:219,261`), giving
25/25 distinct outputs across runs on a switch; a flaky-golden hazard once Python's
reference is gone (one-line fix, the sort already exists in
`tcl-explorer/src/asm.rs:136`). The crashes (A1/A2) and optimiser miscompiles (B,
E1b) are the correctness blockers; the remaining compiler-specific gaps:

### E1 — HIGH: the analyser regresses on the Python precision contract, and that contract is unguarded for Rust

The Python analyser earned its precision through a large, deliberate false-positive
campaign, recorded in [`../compiler/FP.md`](../compiler/FP.md) — a **113-entry,
8 346-line catalog** (families RBS read-before-set, DS dead-store, OBJ/W307,
SH shimmer, STY, TNT taint, NAB, BND, INJ, RCH), ~92 marked FIXED, with **177
paired TP+FP regression tests** swept over real corpora (tcllib 2.0, Tcl 9.0.3
stdlib, tklib, tdom, SpiceGenTcl) against real tclsh. This is the precision
*contract* a production analyser must meet. **The Rust port does not fully meet it,
and nothing guards the gap:**

- **Measured regression.** On a real, well-reviewed Tcl 9 stdlib file (`http.tcl`,
  5 475 lines), the **Rust analyser emits 193 diagnostics vs Python's 130 — ~48%
  more**, the excess dominated by false positives (W210 54 vs 40, W220 22 vs 11,
  W213 10 vs 1, W214 5 vs 1, W307 5 vs 0). Confirmed by inspection: W213 fires on
  `unset state(socketcoro)` where `state` is an `upvar #0 $token state` alias
  (Rust doesn't model the aliasing); W214 over-reports params used via
  call-by-name; W220/W211 over-fire on the patterns the missing
  `ProcArgTrait::DynamicNameLocal` (parity audit §3) is meant to suppress.
- **Partial contract violation — FP-DS-04 (traced variables), scoped to the
  cross-scope case.** FP.md FP-DS-04 ("traced variables excluded from dead-store /
  unused", FIXED in Python). The *same-scope* suppression **is** wired in Rust —
  `scan_scope_aliases` (`optimiser/elimination.rs:1031`) has a `"trace" =>` arm
  (`:1077-1098`) feeding the W211 skip (`analyser/diagnostics.rs:6131`) and W220 skip
  (`:5942`), so `trace add variable x write cb; set x 1` in one proc is correctly
  silent. But the reproducer that still fires on the shipped binary is **cross-scope**:
  `trace add variable ::w write h; proc s {} { set ::w 1 }` makes Rust emit **W211
  "unused"** (Python: silent) — because `scan_scope_aliases` runs per-function CFG and
  does not see a *top-level* trace declaration when analysing a *different* proc's body
  (the namespace-global aliasing the parity audit's missing `DynamicNameLocal` /
  `::`-global liveness covers). So this is narrower than "traced variables are
  unguarded" — it is the cross-scope / namespace-qualified trace specifically. The same
  cross-scope gap drives the optimiser to *delete* the store (E-cross-ref below).
  **Action: a Rust regression test on this exact reproducer should pin the boundary**
  (the architecture-and-quality review §F flags the reconciliation).
- **The contract is unguarded for Rust.** The 177 paired FP tests live in
  `tests/test_fp_{rbs,ds,nab,obj,bnd}.py` — **Python tests of the Python analyser.**
  The Rust port references only **22 of the 113** FP-ids (in comments/asserts), and
  has none of the paired tests. So once Python is retired, the entire precision
  contract stops protecting anything, and the existing regressions ship.

For a *sole-implementation* production LSP this is the quiet killer: false-positive
fatigue trains users to ignore diagnostics, and there is no cross-check to reveal
the noise is wrong. **Before cutover this needs (a) the FP.md corpus re-run against
the Rust analyser to enumerate every regression, (b) the var-scoping / `upvar` /
`DynamicNameLocal` / traced-var fixes, and (c) the 177 paired FP tests ported to
Rust so the contract is guarded.** This is the most important *precision* item, and
it is the direct answer to "use the FP doc to find gaps."

### E1b — CRITICAL: the optimiser deletes stores to namespace-qualified globals on *real shipping code*

The known `::`-global dead-store deletion (B2) is **broader and worse than a
contrived example**: it covers namespace-qualified vars, qualified array elements,
*and* cross-scope traces (violating FP-DS-04 again), and it fires on real tcllib.
On shipping `practcl.tcl` (8 463 lines), `tcl opt` applies **32× O126 + 7× O109**
and *deletes* `set ::practcl::LOCAL_INFO $result` (read in another proc),
`set ::clay::idle_destroy {}`, `set ::make($name) 0`, `set ::target($name)
$filename`, … — a minimal faithful repro changes program output from `linux` to
`unknown` (a **silent wrong result**, masked by an `info exists` guard rather than
erroring). Root cause: `emit_dead_stores_and_unused` (`optimiser/elimination.rs:524`)
gates only on same-function `global`/`upvar`/`trace` declarations and scans
liveness per-function, with no `var.contains("::")` guard. Reachable via `tcl opt`
**and** the LSP `optimiseDocument` command. **Fix: ~10–20 lines** (guard qualified
writes; make liveness module-wide).

### E1c — CRITICAL: the taint/injection diagnostics are not wired into the `tcl` CLI security gate

`tcl diag` / `lint` / `validate` use `Analyser::analyse`, whose emit set contains
**no taint codes** — the taint family (T100–T106) is produced only by
`compiler_checks::run_all_checks`, which the `tcl-cli` crate **never calls**.
Confirmed: `gets stdin u; eval $u` → `tcl diag` emits only the syntactic `W101`
(which also false-positives on `set safe "puts hi"; eval $safe`), `tcl validate`
says "validation ok", and only `tcl dataflow` shows `T100`. So if the Rust `tcl`
CLI becomes the CI/security gate after Python, **no injection diagnostic ever
fires** — directly defeating the "sole security analysis" goal. The analysis
exists and works; it is a wiring gap (route `run_all_checks` into `Analyser::analyse`
or the CLI diag path). **Fix: small–medium.** (The LSP path *does* run the taint
checks, so this is CLI-specific — but the CLI is what a CI security gate would use.)

### E2 — HIGH: WASM codegen is a broken stub, and its parity gate is meaningless for Rust

`tcl compwasm` is worse than "degraded": it is an eval-fallback that boxes every
leaf command's *source text* and calls a runtime `tcl_eval` (`codegen/wasm/
backend.rs:180`), and its one real feature is broken — `if`/`while`/`for` lower to
WASM structure but call `tcl_expr_bool`, which is **hardwired to return `0` on
wasm32** (`runtime/rust/src/codegen_abi.rs:166`, because tommath is off for wasm),
so in the real artifact **every `if` takes the else branch and every loop body
never runs.** Output isn't standalone-runnable (`unknown import
tcl::tcl_obj_new_string`; no `--link`), procs are emitted but never called, and
data is placed at offset 0 (collides with the runtime stack). The Rust emitter is
~1.4K LOC vs Python's ~20.6K. **And the safety gate is illusory:**
`make check-wasm-parity` compares the *Python* registry against the *Zig* runtime —
zero references to any Rust crate — so it says nothing about `tcl compwasm` (and
both sides it checks are being retired). WASM is **not production-usable** Rust-only:
finish the backend (large), keep it Python-backed (contradicts the goal), or
descope it. (Out of the LSP/analysis path.)

### E3 — MEDIUM: the security analysis is the sole oracle, with known soundness gaps

Taint is the only thing standing between a user and an injection bug, and once
Python is gone there is no second opinion. Known gaps that ship: the
interprocedural `writes_global` is **not transitively propagated** (a global
written only via a callee is invisible → missed taint flow / false-negative,
parity audit §3), and the whole taint family's **severity is hardcoded to ERROR**
(B5) so it cries wolf. The `var_escape` analysis (which would sharpen several
results) is **built but unwired** — stranded behind the inliner that has no
`PassId::Inline` (parity audit §3). None of these is a crash, but "trust this as
your only security analysis" requires closing the soundness holes and calming the
severity.

### E4 — HIGH: three O(n²) blowups make large files unusable per-keystroke

This is more than the known "full recompute" latency — there are genuine
**super-linear** algorithms on the hot path (the compiler review measured the O(n²)
signature: 2 000→4 000 lines = **4.1× time**; a flat 8 000-line file takes **47 s**
for `tcl dis`):

- **Codegen** `span_line`/`span_text` (`codegen/mod.rs:177,164`) re-scan the source
  from offset 0 to count newlines **per emitted instruction** (and allocate a fresh
  `String` each) — O(n²); dominates `dis`/`opt`/`compwasm`. Fix: one shared
  `LineIndex` (already exists in `tcl-lexer`).
- **Interprocedural taint** (`taint_interproc.rs:552`) has **no worklist** —
  `while changed` re-infers *every* proc every pass, and runs `params × 15` full
  intra-procedural fixpoints per proc per pass regardless of whether the proc
  touches a taint source/sink (~17 000 fixpoint runs per outer pass on practcl).
  On the per-keystroke `compiler_check_diagnostics` path.
- **Analyser W216** rebuilds the whole-document `SourceMap` **once per command**
  (`analyser/diagnostics.rs:3113`).

The genuinely-good salsa per-proc-body memoisation sits on top of a whole-module
super-linear tail: `build_cfg` and the IPA summary are rebuilt whole-module on
every edit (`tcl-lsp-db/src/lib.rs:330`; `did_change` re-analysis is still
whole-document, `lib.rs:3034`), so the O(n²) taint solve runs per keystroke. The
maintainers' **SRV-ROPE** track addresses the incrementality, but the O(n²)
constants are independently fixable (worklist + line-index) and should be — a
language server that takes seconds per keystroke on an 8 000-line file is not
production-usable.

### E5 — MEDIUM: taint propagation drops common idioms (false-negatives in the security analysis)

Even where taint *is* surfaced (the LSP / `tcl dataflow`), it misses real flows:
`upvar` alias write-back is treated as disjoint (`var_resolve.rs:98`,
`place.rs:316`) so the standard return-by-reference idiom
(`upvar 1 x y; gets stdin u; set y $u`) reports **0** taint; computed/indirect
sinks (`set c eval; $c $u`) are rejected (`interprocedural.rs:251`); and tainted
*array keys* are dropped. Combined with E1c (not wired into the CLI at all) and B5
(severity hardcoded to ERROR so T101/T106 look like T100), the security analysis is
both under-reaching and miscalibrated — which matters because it is meant to be the
*only* one once Python is gone.

**What's solid (and load-bearing for a sole implementation):** core codegen/lowering
(~70/70 differential cases vs tclsh), the SSA/dominance machinery (CHK idoms,
cross-validated), the analyser's MRO (faithful TclOO), taint's conservative
*lattice* (sound formulation even where inputs have gaps), the const-fold overflow
discipline (`checked_*` on numeric folds — only the string-allocating folds (A2)
are uncapped), the text-scanner panic-safety (zero reachable panics from untrusted
text outside the recursion class), and the salsa per-proc memoisation. The engine is
genuinely good; the production gaps are the noise/precision regression (E1), the
optimiser miscompiles (E1b/B), the security wiring + soundness (E1c/E5), the O(n²)
hot paths (E4), one broken back-end (E2), and the unguarded resource limits (A).

---

## F. Tooling — closer to standalone than the docs say, gated by a few concrete blockers

**Good news first:** verb/feature *coverage* is far more complete than the
project's own `docs/rust-cli-port.md` claims — that doc is materially **stale**
(its ⛔ "stub" markers for `dis`, `compwasm`, `explore`, `pkg`/`venv`/`docker`,
`f5 explain-flow`, `f5 irule lint/context/trace` are all contradicted by working
binaries). The `tcl` CLI (26 verbs, zero genuine stubs), the `f5` query DSL
(244/244 builtins, byte-parity), the compiler explorer, the fuzzer, and the
debugger are production-usable Rust-only today. The real blockers are **not
missing verbs** — they are build-time couplings, the VM, and a few genuine stubs.

### F1 — HARD BLOCKER: deleting `tooling/` breaks the Rust build (compile-time coupling)

`tcl-irule-test/src/embedded.rs:19` embeds the orchestrator Tcl at compile time:

```rust
include_str!(concat!("../../../tooling/irule_test/tcl/", $name))
```

So a literal "delete the Python tree" **fails to compile** `tcl-irule-test`. Two
sibling couplings: `editors/zed/build.rs:8-9` `include_bytes!`s the Python
`tcl-lsp-server.pyz` / `tcl-lsp-mcp-server.pyz` into the Zed extension, and the
compiler-explorer **web-GUI static assets live under `tooling/explorer/static/`**
(`index.html`/`explorer-core.js`/`worker.js`, referenced by `make explorer-wasm`
and VS Code). (The `tcl-bigip` `include_str!`s point at `samples/`, which is fine
unless samples are also deleted.) **Fix: S** — relocate `tooling/irule_test/tcl/*`
and `tooling/explorer/static/*` under their consuming crates and repoint; drop the
Zed `.pyz` embed when the launcher goes native. But it must be done *before* the
tree is deleted or the build breaks.

### F2 — HARD BLOCKER: `tcl docker create` bakes Python into users' containers

(Also class D.) Reproduced — `tcl docker create` emits a Dockerfile that
`apt install python3`, `curl …tcl-<v>.pyz`, and `python3 …tcl.pyz pkg install`
(`tcl-pkg/src/docker.rs:60-62,215,246,262`), a faithful port of the Python
original. Any project using `tcl docker` ships Python at *its* runtime. **Fix: M**
— rewrite the recipe to install a native `tcl` binary.

### F3 — LONG POLE: the VM is missing whole language subsystems

`tcl-vm` is a real, well-built bytecode VM (NRE trampoline, namespaces, traces,
upvar/uplevel, catch/try, file I/O, loads the genuine Tcl 9 `tcltest.tcl`) — but
it is milestone-staged and **whole subsystems are absent**, confirmed against the
`tclvm` binary:

| subsystem | status | evidence |
|---|---|---|
| TclOO (`oo::class`, `oo::define`) | **missing** | `invalid command name "oo::class"` |
| coroutines | **missing** | `invalid command name "coroutine"` |
| event loop (`after`/`vwait`/`update`) | **missing** | `invalid command name "after"` |
| `socket` / networking | **missing** | `invalid command name "socket"` |
| child interpreters (`interp create`) | **missing** | `command.rs:317` "only the current interpreter is supported" |
| channels / real I/O | partial | file ok; no pipes (`cmd_chan.rs:51`), no `socket`, stdin always EOF |
| command/execution traces | partial | accepted but never fired (`cmd_trace.rs:9`) |
| `return -level` countdown | missing | `command.rs:699` "M2 simplification: skip the countdown" |

The VM backs `tclvm`, the debugger, the fuzzer, and iRule-test. Impact: the REPL
and debugger break on any modern OO/coroutine/event Tcl; the fuzzer **deliberately
steers around the gaps** (`tcl-fuzz/generator.rs:3-8`), so those subsystems get
**zero differential coverage**; only iRule-test (iRules don't use them) is a clean
fit. Python's `tooling/vm/` implements all of it. **Fix: L (large, ongoing — TclOO
is the biggest item).** This is the long pole for any *general-purpose runtime* use
of the Rust tools, though not for the LSP/analysis product (which never runs the VM).

### F4 — Genuine verb stubs and CLI-compat regressions (medium)

- `f5 irule pgo` — exit 2, "not yet ported (requires the compiler-VM engine)"
  (`f5-cli/commands/irule.rs:424`).
- `f5 registry-dump --section commands|all` — exit 2 (`registry_dump.rs:32`); only
  `profiles`/`objects`/`events` work.
- `tclvm` REPL: inline flag is `-c` not Python's `-e/--eval` (breaks scripts/CI),
  missing `--disassemble`/`--optimise`/`--enable-test-support`, no history/completion.
- `tcl explore`: missing 3 Python flags; terse default output not contract-compatible.
- `tcl pkg outdated`/`update`: registry version lookup "not yet wired" — always
  reports up-to-date (`commands/pkg.rs:698,766`).
- No standalone `irule test <file>` assertion runner (only the embedded
  `f5 explain-flow --simulate`).
- SSH transport deferred (`f5-cli/.../ssh.rs:11`) — **not a regression** (Python
  defers it too).

### F5 — Unverified byte-parity / contract risk (medium)

Several outputs are explicitly "not asserted byte-for-byte yet": the engine-gapped
`tcl` verbs (`diag`/`opt`/`symbols`/`callgraph`/`dataflow`/`registry-dump commands`),
`f5 diff` object-list display, and `tcl-pkg` lockfiles/manifests have **no golden
test vs Python** (only self-referential round-trips), so the "byte-for-byte" claim
is unverified. Editors/CI must not pin goldens on these until closed (ties to
class C: freeze the goldens as Rust-owned fixtures).

**Net tooling verdict:** the CLIs, query DSL, explorer, fuzzer, and debugger are
genuinely close to standalone; the cutover is gated by **F1 (build coupling — do
first)**, **F2 (docker)**, **F3 (the VM, if general runtime use is in scope)**, and
the F4/F5 cleanup. And `docs/rust-cli-port.md` plus several crate headers must be
de-staled *first*, since the cutover plan is being made against wrong status.

---

## Path to production — the ordered work to retire Python safely

Grouped by gate. Nothing here is research; it is bounded engineering, and the
estimates are rough relative sizes (S/M/L).

**Gate 1 — robustness (must fix; reachable from untrusted input in a long-lived,
sole-implementation server):**

1. **Recursion-depth guard** across the analyser body-walk, `expr` parser, CFG
   builder, lowering, and regex parser (A1). Thread a depth budget (the analyser
   ships `MAX_DEPTH=8` already) or run analysis on a bounded, explicitly-sized
   stack and decline past it. **The single most important fix** — it closes a
   trivial whole-server-kill DoS. *M.*
2. **Output-size caps on allocating folds** (`fold_repeat`/`fold_lrepeat`,
   `checked_mul` on `s.len()*n`, bail above a few MB) (A2). *S.*
3. **Document-size limit + per-document analysis timeout** (A3); split the
   panic-vs-cancel arms so a deterministic panic doesn't livelock (A4); move
   `cross_document_incoming_calls` and `will_save_wait_until` onto `spawn_blocking`
   (A5). *S–M.*
4. **A panic hook + real logging** so field crashes are diagnosable (A5/H6). *S.*
5. **Kill the O(n²) hot paths** — a shared `LineIndex` in codegen, a worklist (+
   taint-free-param skip) in interprocedural taint, and a cached `SourceMap` in
   W216 (E4). A server that takes seconds per keystroke on an 8 000-line file is
   not usable; these are bounded, independent fixes. *M.*

**Gate 2 — correctness (must fix; wrong results ship with no Python cross-check):**

5. **The optimiser miscompiles** — O122 `[list …]`; O109/O126 add the
   `var.contains("::")` guard + module-wide liveness (this is **E1b** — it
   corrupts real tcllib code, including traced and qualified-array writes); O129
   wire the trust gate (B1–B3); the minify panics (B4). *S each.*
6. **Wire taint into the `tcl` CLI** (`run_all_checks` → `diag`/`lint`/`validate`)
   — otherwise the sole CLI security gate reports **zero** injection diagnostics
   (E1c). *S–M.* Then the taint soundness gaps (`upvar`/computed-sink/array-key,
   E5), the per-code severity map (B5), and transitive `writes_global` (E3). *M.*
7. **Drive down the precision regression against the FP.md contract** — re-run the
   113-entry / 177-test FP catalog against the Rust analyser, fix the regressions
   (FP-DS-04 traced vars, the `upvar`/call-by-name/`DynamicNameLocal` families
   behind the 48% http.tcl excess), and **port the 177 paired FP tests to Rust** so
   the precision contract is guarded once Python is gone (E1). *M–L — the single
   biggest precision item.*
8. **Add `ledit`** + the registry-completeness gate (B6 / C). *S.*

**Gate 3 — the safety net (must exist before deleting Python):**

9. **Rewire `main` CI to Rust** (`cargo test --workspace` + rust-backend `lsp_e2e`)
   (C-P0a). *S.*
10. **Optimiser execution-equivalence harness** through `tcl-vm` (catches B1–B3
    going forward) and a **tclsh-only codegen differential** (replaces the dying
    Python/disasm oracles); freeze the goldens/baselines as Rust-owned fixtures
    (C-P0b/P1). *M.*

**Gate 4 — distribution (must exist before users get a Rust-only product):**

11. **Build, sign, and ship native `tcl` / `f5-query`** (and decide `tcl-vm`) in CI;
    rewrite `install.sh` for native assets (D). *M.*
12. **Migrate Zed / JetBrains / Sublime launchers** to the native server (VS Code is
    the template); de-`.pyz` the Zed build embed (D/F1). *M.*
13. **Rewrite `tcl docker` / `tcl-pkg::docker`** to install the native CLI, not
    `python3 tcl.pyz` (D/F2). *M.*
14. **Relocate the compile-time `tooling/` couplings** (`tcl-irule-test`'s
    `include_str!`, the explorer web assets) so deleting `tooling/` doesn't break
    the build (F1) — **do this first in the cutover, before any deletion.** *S.*
15. **Resolve the no-Rust-replacement items** — `tcl-wasm` (E2), the explorer GUI
    host, MCP/AI tooling: port or descope (D). *L / decision.*

**Gate 5 — scope decisions (not blockers, but decide before committing to
"Rust-only"):**

16. **The VM's missing subsystems** (TclOO, coroutines, event loop, sockets,
    child interp — F3): a blocker only if the REPL/debugger must handle modern Tcl;
    descopable if the VM stays an iRules/analysis-support runtime. *L.*
17. **De-stale the tracking docs** (`rust-cli-port.md`, the registry/optimiser/
    pipeline-parity docs, crate headers) — the cutover plan is being made against
    wrong status (F, parity audit). *S, do early.*

### Bottom line

The Rust core is good enough to be the foundation — but "retire Python" today would
ship a language server that **a single malformed file can kill**, an
`optimiseDocument` that **corrupts correct code**, an analyser that is **48% noisier
than Python on real files**, **no Rust CI gate and a dissolving safety net**, and a
**distribution that is still Python** for every surface except VS Code. None of it
is research and none of it is deep — it is Gate 1–4 above, perhaps 6–10 focused
work items on the critical path — but it is real, and the robustness and safety-net
gates (1 and 3) must land *before*, not after, Python is removed as the fallback.
