# Phases 3, 5, 6 — design & deferral

Status of the algorithmic-improvement programme (branch
`claude/parser-compiler-algorithms-…`):

| Phase | State |
|---|---|
| 0 registry-aware green-tree descent | **shipped** (`descend_command`) |
| 1 shared loop-nesting forest | **shipped** (`compiler/loops.py`) |
| 2 semi-pruned SSA | **deferred** — `semi-pruned-ssa-deferred.md` |
| 3 interval/length domain + bounds | **shipped & complete** (`intervals.py` + guard narrowing + `interval_bounds.py` dynamic W230/W231/W232 + W233 divide-by-zero, all positions; `intervals`/`bounds` views + `types`-view ranges) |
| 4 interproc SCC condensation | **not needed** — existing worklist fixpoints already optimal |
| 5 emitter slot allocation | **deferred** — below |
| 6 incremental memoization + reparse | **deferred** — below |
| explorer views (P0 green-tree, P1 loops) | **shipped** |

3, 5 and 6 are each large, regression-prone efforts; they are specified here
rather than rushed. Each entry gives the approach, the exact seams, the risk,
and the validation gate that must pass before it ships.

---

## Phase 3 — interval / string-length abstract domain + bounds rewire

**Why deferred:** the *syntactic* bounds checks (`analyser/checks/_bounds.py`,
W230/W231/string) are already mature and Tcl-9.0.3-validated (76 tests in
`tests/test_checks_bounds.py` + `tests/test_bounds_vs_tcl903.py`); all verified
idiom-negatives (`lrange` clamp, `lset` append/`end+1`, `incr`-unset) already
stay silent. The remaining value is only the **dynamic (variable) index/length
cases that are currently *skipped*** (`test_lindex_dynamic_list_skipped`,
`test_lindex_dynamic_index_skipped`) plus off-by-one loop-bound detection. That
requires a numeric abstract domain flowing through SCCP — an output-changing
change with the same risk profile that blocked Phase 2.

**Approach (safest first):**
1. **Parallel, non-perturbing interval analysis** — do **not** add a kind to
   `LatticeValue`/`_join` (that risks perturbing existing CONST/CONSTSET folding
   and every consumer). Instead compute a *separate* per-`(name,version)`
   interval + list/string-length map **after** SCCP, seeded from SCCP consts and
   `llength`/`string length`/literal lists, propagated forward with **widening at
   loop headers** (use `compiler/loops.py` `build_loop_forest` headers) and a
   narrowing pass.

   **SHIPPED (core):** `compiler/intervals.py` — `Interval([lo,hi]` with
   `None`=±inf), sound `join`/`widen`/`add`/`sub`/`mul`/`negate`, and
   `compute_intervals(cfg, ssa, values)` doing a bounded RPO fixpoint seeded
   from SCCP consts, transferring through `IRAssignConst`/`IRAssignExpr`/`IRIncr`
   and widening at `LoopForest` headers. Exposed via the explorer `intervals`
   view (`tooling/explorer/cli.py:print_intervals`). It is computed *on demand*
   (explorer / future bounds check), **not** eagerly on `FunctionAnalysis`, to
   keep zero cost on the hot analysis path until a diagnostic consumes it.
   Tests: `tests/test_intervals.py`, `tests/test_compiler_explorer.py`.

   **REMAINING (todo, the high-value part):**
   - **Guard-based narrowing — SHIPPED** (`intervals.refine_interval`): refines a
     value's interval inside the true/false region of a *constant-bound*
     comparison branch (`if {$i < 10}` ⇒ `i ∈ [lo, 9]` on the true edge) by
     intersecting with the guards on every dominating branch edge reaching the
     block. Symbolic guards (`$i < [llength $l]`) yield no refinement (sound).
   - **List-length map — SHIPPED** (`interval_bounds._list_length_map`): seeds
     element counts from literal-list assignments (`set l {a b c}`) and
     `[list …]` command substitutions, per SSA version (a phi/`lappend` version
     is simply absent → the consumer does not fire). String-length tracking is
     still deferred (`string index` is skipped).
2. **Bounds rewire — SHIPPED** (`compiler/interval_bounds.py` +
   `analyser/_analyser/_diag_interval_bounds.py`): a **new SSA-path check** (the
   intervals are not available in the syntactic `_bounds.py`) consults the
   interval + length maps for the *dynamic* shapes the syntactic check skips —
   a plain `$var` index on `lindex`/`lset` against a known length. Fires
   **only** when the *whole* index interval is out of range (sound; an interval
   over-approximates). W230 (smell) for `lindex` (silent empty in tclsh), W231
   (error) for `lset` (`index == length` append slot stays silent). The
   syntactic literal-index path is untouched → no double-fire. Reachable
   positions: top-level `lindex`/`lset` IRCalls, `set x [lindex …]`, **and
   `lindex`/`string index` substitutions nested in **any argument value, a
   `return` value/expr, a branch/loop condition, or embedded in an `expr {…}`
   body** (`puts [lindex $l $i]`, `lappend out [lindex $l $i]`,
   `return [lindex $l $i]`, `set u [expr {[lindex $l $i] + 1}]`,
   `while {[lindex $l $i]} …`) — parsed via `_parse_command_substitution` /
   `_index_subs_in_expr`, var versions resolved from the statement's `uses` (or
   the block's `exit_versions` for a terminator). **String index (W232)** is
   covered via `_string_length_map` (character length of literal string
   assignments). Only remaining gap: `lset` nested in a substitution (its first
   arg is a name needing reaching-version resolution, handled only top-level) —
   rare. Explorer: a `bounds` view (`cli.print_bounds`) renders each W230/W231/
   W232/W233 finding; the `types` view annotates each value with its interval.
   Tests: `tests/test_interval_bounds.py`, `tests/test_compiler_explorer.py`.

   **Corpus result (836 files): 0 new findings.** Well-tested libraries
   (tcllib/Tcl stdlib/tklib/tdom) contain no *provably* out-of-range dynamic
   accesses — confirming soundness (no false positives) rather than absence of
   capability; the check fires on the synthetic positives and on real buggy
   user code. (A +3 `S101`/`S102` shimmer delta on `optimize.tcl` `$denom`
   appeared *only* in full-corpus order — `0` in isolation and in math-dir
   order — i.e. the pre-existing cross-file shimmer cache-determinism artifact,
   not a finding of this check.)
3. **Deep finding — SHIPPED (W233 divide-by-zero)** — `find_divide_by_zero`
   flags a `/` or `%` whose divisor's guard-narrowed interval is exactly
   `[0,0]`, in an SCCP-**reachable** block. The `!= 0`-guard case is sound for
   free: a `!= 0` guard cannot narrow an interval, but a provably-zero divisor
   makes `$d != 0` constant-false, so SCCP prunes the guarded division's block
   unreachable and the executability filter excludes it. Verified against tclsh
   (`expr {1/0}`/`expr {5%0}` raise; guarded division stays silent;
   `expr {10/($x-$x)}` with `x` constant fires via interval arithmetic). New
   code W233 (warning/control_flow). Corpus: 0 (clean code has no provable /0).

**Validation gate (mandatory):**
- `bench/phase0_descend.py` diagnostics **byte-identical** for everything that
  was already flagged (only *new* dynamic-case findings may appear).
- Every new finding tclsh-verified; the verified negative table in the plan
  (`lrange` clamp, `incr`-unset, `lset` append, `2**70` bignum) must stay silent.
- `make test-opt` (VM equivalence) green; `make test-py` green.
- Explorer: RANGE in the `types` view + a `bounds` reasoning overlay (both shipped).

**Risk:** medium-high (output-changing). Ship the interval analysis + explorer
view first (output-equivalent), then the bounds rewire behind tclsh-audited
deltas.

---

## Phase 5 — emitter slot allocation + cross-block peephole (WASM + opt-in only)

**Status: allocator CORE shipped** (`compiler/slot_allocation.py` +
`tests/test_slot_allocation.py`); emitter wiring is the risk-gated remainder.

**Approach:**
1. Thread SSA `live_in`/`live_out` (already on `FunctionAnalysis`) into
   `compiler/codegen/.../codegen_function`/`codegen_module` (currently CFG-only).
2. Build an interference graph over locals from liveness; **linear-scan** colour
   to reuse `LocalVarTable` slots — applied in the **WASM** emitter
   (`compiler/codegen/wasm/`) and the `optimise=True` bytecode mode.
   **SHIPPED:** `slot_allocation.py` — `build_interference(cfg, ssa, analysis)`
   computes an *instruction-granular* name-level interference graph (backward
   walk per block seeded from `live_out` + terminator reads, so straight-line
   locals with disjoint ranges interfere correctly — block granularity misses
   them), and `coalesce_slots(...)` greedily colours it (params pinned to stable
   low slots, never shared). Verified: 3 sequential locals → 1 slot; overlapping
   → distinct. This is the hard algorithm; it is emitter-free and unit-tested.
3. Cross-block redundant-load elimination + jump threading/block reordering in
   `layout.py`, gated to the optimise/WASM path; shared table-driven peephole.

**Wiring finding (raises the risk vs the original plan):** the WASM emitter's
`_intern_local` slot is **not** a pure name→index map — it also seeds
retain/release refcounting (`_owned_locals`), the frame-sync map
(`_tcl_var_locals`), and escape routing. Remapping two names onto one slot
therefore requires releasing the previous occupant's `TclObj` before the next
live range reuses the slot, i.e. it is entangled with the runtime's memory
management, not a drop-in `coalesce_slots()` remap. So the WASM wiring must
insert release/retain at coalesced-slot boundaries and be validated against the
WASM runtime (`make runtime-rust-test` + `make test-opt`) for leaks/use-after-free. The
opt-in **bytecode** mode is lower-risk (the VM owns locals, no manual
refcounting) and is the recommended first wiring target.

**Validation gate:** `tests/test_bytecode_identity.py` **must stay green**
(default path untouched — the guardrail). New `optimise`-mode tests assert slot
reduction; WASM runtime (`make runtime-rust-test`) + `make test-opt` confirm semantics.
Explorer: slot allocation/interference in `asm`/`wasm` views.

**Risk:** low for the default path (guardrail), medium for the WASM/opt path.

---

## Phase 6 — incremental reparse persistence + document-level query memoization

**Why deferred:** largest and most architectural; touches `server/workspace/`
and the analyser orchestration. ~84% of per-edit cost is document-level work
that survives the existing per-proc `FunctionUnit` cache (measured;
`bench/phase0... / tmp/algo_incremental.py`).

**Approach:**
1. **Query memoization** — memoise per-proc *diagnostic results* keyed by the
   existing proc-body hash (`_proc_cache_key`) plus a small **dependency
   fingerprint** (external symbols/commands the proc references). Recompute only
   procs whose hash or fingerprint changed; merge cached per-proc diagnostics in
   source order. Cross-proc-inherent diagnostics (interproc, taint, iRules
   connection scope, duplicate-proc) recompute over cached *summaries*.
   Owner: `server/workspace/document_state.py` (already owns `_proc_cache`).

   **Fingerprint foundation (experimental — NOT wired into any cache):**
   `compiler/proc_fingerprint.py` `dependency_fingerprint(proc)` — the
   over-approximate set of external symbols a proc references (invoked command
   names, top-level and inside `[...]` substitutions in every nested
   control-flow body, plus `::`-qualified global/namespace vars from words and
   expr ASTs); locals/params excluded. Order-independent, stable across
   same-dependency body edits, distinct when a callee/global changes. Tests:
   `tests/test_proc_fingerprint.py`. **No cache references it today** — the
   per-proc `FunctionUnit` reuse that actually shipped keys on `(proc source,
   stub fingerprint, CFG-context fingerprint, start line/char/offset,
   known-classes fingerprint)` (`document_state._build_proc_cache` /
   `compilation_unit._proc_cache_key`), and cross-proc invalidation rides the
   context fingerprint. **Remaining (risk-gated):** wiring `dependency_fingerprint`
   into a body-local diagnostic cache in `document_state.py` (key = body-hash +
   fingerprint-hash; merge cached per-proc diagnostics in source order; recompute
   cross-proc diagnostics over summaries), behind the byte-for-byte
   golden-equivalence gate. The over-approximation bias means a missing dep can
   only force recomputation, never staleness — but the golden test is the hard
   contract and must pass before this ships.
2. **Incremental reparse persistence** — persist the green-tree scope across
   edits; extend `compiler/parsing/incremental.py` reuse below top-level chunks.

**Validation gate (mandatory):** the snapshot/restore golden contract
(`tests/test_incremental_update.py`) — incremental diagnostics must equal a full
rebuild **byte-for-byte**. Re-run `tmp/algo_incremental.py`; target single-proc
edit well below today's ~86%-of-cold. Explorer: per-proc cache-hit/recompute
overlay.

**Risk:** high (correctness of incremental ≡ full is the whole game). Land query
memoization behind the golden-equivalence test before reparse persistence.

---

## Cross-cutting (still open)

- Explorer **audit/cleanup**: consolidate `asm`/`asm-opt`, `wasm`/`wasm-opt`,
  `opt`/`callouts` into diff toggles; add P3/P5/P6 views as those land. Removal is
  deferred (avoid breaking editor integrations that reference view names) until a
  focused pass can update `pipeline.ALL_VIEWS`, renderers, and tests together.
