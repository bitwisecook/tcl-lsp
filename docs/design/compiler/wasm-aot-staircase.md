# WASM AOT staircase — phased plan

> Companion to `docs/design/runtime/memory-management.md` (runtime side, MM-A
> through MM-E). This doc is the **compile side**: how the AOT codegen walks
> from "every proc gets a Tcl frame" down to "small leaf procs are inlined into
> static WASM with no frame overhead".

> **Historical implementation record:** the staircase was executed while the
> retired Python compiler was still present. Detailed stage pages preserve
> file names, flags, and emitter classes from that implementation as evidence
> of what was measured; they are not current APIs. The production Rust
> architecture has one public entry, `codegen::wasm::compile_wasm`, documented
> in [`wasm-codegen.md`](wasm-codegen.md). Its private typed compatibility plan
> is not a second or selectable backend.

## North star

The Tcl-WASM AOT compiler should emit static WASM wherever it can prove the
behaviour matches reference Tcl, and fall through to the interpreter on the
exact set of constructs that resist proof. **Correctness comes first; speed is
won by tightening proofs, not by skipping work.**

The acceptable progression — each step strictly safer than the next — is:

1. **IR → unoptimised code with Tcl frames everywhere.** Every proc pushes a
   runtime frame. Every variable read/write hits the frame's hash table.
   Slow but trivially correct because the runtime already has working
   refcount discipline on the frame slot (`MM-B.3`).

2. **Reduce / remove Tcl frames where local escape analysis proves the proc
   has no var that escapes and no fallback path.** Those slots become WASM
   locals and the codegen owns the refcount.

3. **Interprocedural elision** — propagate "no escape" through statically
   resolvable call graphs, so a proc whose callees also can't escape can
   itself drop its frame.

4. **Inlining** — small leaf procs get pasted into the call site, removing
   the call-bridge and unlocking further escape proofs.

Each step is a strictly larger optimisation surface; rolling back one step
returns to the previous correct baseline.

## Stage skeleton

| Stage | Goal | Detail | Status |
|---|---|---|---|
| **S0** | Foundation — observability, contracts, repros | [s0](wasm-aot-staircase-s0.md) | landed (1edd39b0…db8188b4) |
| **S1** | "Frames everywhere" baseline — disable elision, prove correctness | [s1](wasm-aot-staircase-s1.md) | landed (f8d920ea, c6831243) |
| **S2** | Per-proc frame elision with refcount discipline | [s2](wasm-aot-staircase-s2.md) | landed (99c24c4c…43e8f14f), 0 double-frees |
| **S3** | Interprocedural escape-analysis tightening | [s3](wasm-aot-staircase-s3.md) | partial — S3.4 pure_leaf tag landed (48e885ba); S3.1/S3.2/S3.3 deferred (existing checks already mostly precise) |
| **S4** | Inlining of small leaf procs | [s4](wasm-aot-staircase-s4.md) | fully landed — S4.1 catalogue (e7480b4c), S4.2 v0 empty-body splice (2ce0fbe7), pipeline integration (d97e1c17), v1 single-call wrapper (5c21fd76) + frame-observation gate (9ff932fa), v2 multi-statement wrapper (bc5acddd), v3 parameterised inlining + α-renaming + dead-proc DCE (b2cefa5b), v3 eligibility completion (e0c1eddd), IRReturn-anywhere via while-break wrap + array-element writes (e6997712); S4.3 (profile-guided WASM-level inlining) closed as out-of-scope — needs profile infrastructure |
| **S5** | SSA-driven codegen optimisations | [s5](wasm-aot-staircase-s5.md) | fully landed — S5.1 first-write (acb851f4), S5.2 alias-skip (0cac52ce), S5.3 LICM for IRFor (099920da) + IRForeach (f111b3c3), S5.4 DCE adapter (0d531219), S5.4 SCCP const-prop reader for ``_emit_value`` (d6d01e52), S5.4 GVN for redundant IRAssignExpr (2fa82d23) |
| **S6** | Allocation + small-value representation | [s6](wasm-aot-staircase-s6.md) | landed — S6.1 free-list reuse (8fa4d064), S6.4 tagged-immediate small ints (33b968e9), S6.2 inline strings (58ada312), S6.3 v0 per-scope arena for subst scratch (c1702928), S6.3 v1 arena extended to regex + interp scratch (90d8c778) |

Each stage decomposes into numbered sub-plans (S0.1, S0.2, …). Sub-plans are
sized to land in 1–3 commits each. Each per-stage doc lists tasks, file
paths, test plan, rollback path, and acceptance gate.

## Sub-plan inventory

### S0 — Foundation (gate for everything else)

- **S0.1** Document the runtime refcount contract (callee/caller ownership
  table for every WASM-exported runtime function).
- **S0.2** Add `-Dleak-check=true` debug build that counts allocs/frees and
  asserts zero residual at reactor exit (MM-C from runtime doc).
- **S0.3** Build a deterministic repro for the canonical "set var $other →
  drained between iterations" bug. This is the test we measure every later
  fix against.
- **S0.4** Add `make leakcheck` CI gate; baseline current sweep with leak
  detection on so future regressions surface as new leaked-byte counts.

### S1 — "Frames everywhere" baseline

- **S1.1** Add a codegen flag `--no-frame-elision` (defaults to off) that
  forces `wants_frame=True` for every proc.
- **S1.2** Run sweep + leakcheck under `--no-frame-elision`. Confirm the
  result matches today's baseline (because today, elision only fires when
  analysis says no escape AND no fallback — the runtime path is already
  exercised). Any divergence is a runtime-side bug to fix here.
- **S1.3** Audit any remaining gaps in runtime frame discipline that S0.2's
  leak counter surfaces. Fix runtime, not compile, in this stage.

### S2 — Per-proc frame elision with discipline

> The piece my failed attempt tried to do in one shot. Re-attempt as a series
> of smaller, individually verifiable changes.

- **S2.1** Refactor `_emit_value` to return an `Ownership` enum (`OWNED` |
  `BORROWED`) so call sites know whether the value on the stack carries a
  +1 they must transfer or a borrow they must retain to claim.
- **S2.2** Add `_emit_local_set_owned(idx, source)` and `_emit_local_tee_owned`
  primitives. Routed only when the slot is in `_owned_locals_set` AND the
  proc is frame-elided.
- **S2.3** Migrate every existing direct `_emit_local_set` to a Tcl-variable
  slot to use the owned variant — variables, foreach loop var, default-sub,
  lappend in-proc fast path, dict update, etc. Each migration is one commit
  with a sweep delta.
- **S2.4** Prologue: retain each param BEFORE default-substitution so the
  slot owns +1 of the caller's handle by the time default-sub's wrapped
  store releases the prior.
- **S2.5** Epilogue: release each owned slot before every `RETURN` and the
  natural fallthrough `END`, with a save-and-retain wrapper around the
  return value so the upcoming releases can't queue it for free.
- **S2.6** Re-enable runtime fast paths (`tcl_cmd_lappend` rc==1 in-place,
  `tcl_cmd_append`, etc.) under the new compiler discipline — verify the
  rc==1 check still matches by tracking "sole owner" via the ownership
  enum from S2.1, not raw rc.
- **S2.7** Land the two blocked fixes (`llength` unbalanced-brace error,
  namespace-aware `_emit_re_register_proc`) once S2.1–S2.6 prove the
  refcount foundation.

### S3 — Interprocedural escape-analysis tightening

- **S3.1** Tighten `info level` propagation: only pessimise to FRAME when a
  callee actually reads `info level`, not when its body merely contains
  the word.
- **S3.2** Tighten `has_fallback`: today any unrecognised IRCall sets it.
  Many fallback paths (eg. `puts`, `incr` with a known integer) actually
  resolve in the runtime without touching the compiled-frame's locals.
- **S3.3** Cross-proc upvar-source aggregation already exists; audit the
  fixpoint and add tests for the corner cases (recursive procs, dynamic
  dispatch through a const string).
- **S3.4** Tag "pure" leaf procs (no upvar/uplevel/info, no global mutation,
  no I/O) — they become candidates for inlining in S4.

### S4 — Inlining small leaf procs

- **S4.1** Catalogue inlining-eligible procs from the IR: small body
  (≤ N statements), no `upvar` / `uplevel` / `info` / `tailcall`, single
  static call site OR `pure` leaf.
- **S4.2** IR-level inlining: substitute the callee's IR into the caller's
  IRBlock, with α-renaming for body-locals so the caller's escape analysis
  sees the inlined ops.
- **S4.3** WASM-level inlining (post-codegen) for hot procs that the IR
  inliner couldn't handle — requires a "hot proc" marker, possibly from
  PGO or a static heuristic.

### S5 — SSA-driven codegen optimisations

- **S5.1** Skip the retain/release wrap when SSA proves the slot's prior
  value is null at the write site (eg. first write to a body local).
- **S5.2** Skip the wrap when SSA proves new and prior alias the same obj
  (eg. `set x $x` after `set x …`).
- **S5.3** Hoist invariant retain+ssaload pairs out of loops (loop-invariant
  code motion for refcount ops).
- **S5.4** Constant-fold across statements where SCCP already proves the
  result — many existing optimiser passes apply, just need the codegen
  hook to read their output.

### S6 — Allocation + small-value representation

> Mostly already covered by `docs/design/runtime/memory-management.md`'s
> MM-D. Listed here so the staircase shows the full picture.

- **S6.1** Re-enable size-class free-lists for the four most-common classes
  (32, 48, 64, 96) — recover most of the libc-malloc cost.
- **S6.2** Inline-string optimisation: ≤ 8-byte strings (``MAX_INLINE_STR``)
  live in the TclObj header's ``OBJ_INT_CACHE`` slot via the
  ``TYPE_INLINE_STRING`` tag, no separate buffer.  (Original design
  spec'd ≤ 23 bytes; the landed cap is 8 — see s6.md S6.2 for the
  rationale.)
- **S6.3** Per-statement arena for parser scratch + regex intermediates,
  reset on `eval_command` boundary.
- **S6.4** Tagged-immediate small ints (high bit set on the i32 means
  "this is the int value, not a pointer") — eliminates the alloc for
  every `set i 0`-style hot literal.

## Acceptance gates

| Stage | Gate |
|---|---|
| S0 | Leak-check CI green; canonical bug reproduces deterministically with leak counter |
| S1 | `--no-frame-elision` sweep == baseline (96 files, 10746 tests, 0 regressions) |
| S2 | Each S2.x sweep is net-zero or net-positive vs the previous; S2.6 lets S2.7's two fixes land net-positive |
| S3 | More procs tagged frame-elidable than today; sweep stays net-positive |
| S4 | Inlined procs show measurable wall-time reduction in `perf_microbench`; sweep stays net-positive |
| S5 | Retain/release call count drops measurably (counted via S0.2 instrumentation) |
| S6 | `set` / `incr` / `expr` per-op stay under the post-S6 absolute thresholds table below |

### S6 acceptance thresholds

The original "within 10 % of pre-MM-A bump-allocator numbers" gate
is unmeasurable: the bump allocator was retired before per-op
microbench numbers were captured, and reverting the runtime to the
pre-MM-A state to re-baseline is a multi-day task with limited
return.  The gate is restated as **absolute thresholds** anchored
to the post-S6 microbench, with a 20 % budget for noise and
incidental future regressions.

Thresholds are per-op nanoseconds with `--no-frame-elision=false`,
captured by `scripts/dev/perf_microbench.py` against the production
runtime build.  A run is green when each row stays under its
threshold.

| Bench               | Post-S6 measured | Gate threshold |
|---------------------|-----------------:|---------------:|
| `set+read variable` |        ~3 200 ns |      4 000 ns  |
| `incr loop`         |        ~2 400 ns |      3 000 ns  |
| `expr arithmetic`   |        ~3 000 ns |      3 600 ns  |

These rows make up the S6 gate.  When a future change pushes any
of them over its threshold, treat it as a regression and either:

* fix the regression before landing, or
* explicitly raise the threshold in this doc with a justification
  comment, then bump the committed
  `tests/baselines/wasm_microbench_baseline.json` row.

`scripts/dev/perf_microbench.py --baseline tests/baselines/wasm_microbench_baseline.json
--regression-pct 20` is the CI-runnable form: it red-flags any row
that drifts >20 % from the captured baseline.  This doubles as a
*proxy* for the threshold gate — the baseline rows already encode
the "post-S6" numbers above, and a 20 % drift is the same budget
the threshold table allows.

## Sequencing rules

- **No skipping S0.** Every later stage relies on the leak detector to catch
  regressions that the sweep alone would miss.
- **S2 must complete before S3, S4, S5.** Without correct per-proc discipline
  the upstream optimisations can't be measured because they'll trigger
  use-after-frees that confuse the metrics.
- **S6 is independent of S1–S5** and can land in parallel. Improvements
  there are felt by every stage.

## Where to flesh out next

This is the skeleton only. Each sub-plan needs its own detailed task list
(file paths, code snippets, test plans, rollback) before implementation
starts. Recommended fleshing-out order, given the failed S2 attempt:

1. **S0.1 + S0.2 + S0.3** first — without these, every later attempt is
   blind.
2. **S1.1 + S1.2** — confirms our floor.
3. **S2.1** before any of S2.2–S2.7 — the ownership enum is the keystone.
4. Everything else in numerical order.
