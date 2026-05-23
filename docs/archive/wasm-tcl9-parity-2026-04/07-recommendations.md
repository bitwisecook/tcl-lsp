# 07 — Recommendations, ranked

A prioritised list of changes ranked by **expected impact ÷
implementation effort**. Each item links to the analysis
sub-report that motivates it.

After folding in the in-scope tcltest sweep results, the
ranking has changed. **R1 (allocator)** is now both a
correctness blocker and a perf one; previously it was
"only" the latter.

## Tier 1 — must land before next perf push

### R1. Fix the bump allocator: stable addresses + grow + size-class free-lists

- **Why:** [`08-tcltest-suites.md`](08-tcltest-suites.md) shows
  9 tcltest files trap with `unknown command: <garbage bytes>`
  because the dispatcher reads `(ptr, len)` pairs that point
  into recycled memory. Plus the OOM at scale documented in
  [`05-correctness.md`](05-correctness.md). Plus the leaked
  memory documented in [`06-hotspots.md`](06-hotspots.md).
  Single biggest correctness + perf gate.
- **Effort:** ≈ 2–3 days. Two layers:
  1. Replace `heap_ptr += aligned` with a real allocator —
     keep the bump fast-path for first-touch but call
     `@wasmMemoryGrow` on overflow and trap with a
     Tcl-friendly "out of memory" instead of a raw wasm trap.
  2. Add 4–6 size-class free-lists (24, 32, 48, 64, 128, 256
     bytes; power-of-two beyond) so `obj_release` can recycle
     non-OBJ_SIZE allocations. Critically, **never hand out
     the same slab while a refcount is live** — any place
     that holds a `(ptr, len)` for use after the next
     `alloc()` call must take an extra reference.
- **Expected impact:**
  - Unblocks the 9 tcltest files trapping on garbled command
    names.
  - Unblocks 3 microbench cases (`expr`, `lappend`, `dict`).
  - Removes the heap-pointer ceiling on real workloads.
  - No perf regression on the OBJ_SIZE fast-path.
- **Acceptance:**
  - `lrange.test`, `lreplace.test`, `format.test`, `var.test`,
    `namespace.test`, `info.test` reach a tcltest summary
    line (no `unknown command` traps).
  - `expr arithmetic`, `lappend`, `dict set` microbenches
    complete without trapping.

### R2. Mutable / capacity-aware `append`

- **Why:** [`06-hotspots.md`](06-hotspots.md) — O(N²) text
  builders are the only category where wasm is qualitatively
  worse than tclsh. Hits `append.test` (33/52 fail) and every
  templating / log / report-building real workload.
- **Effort:** ≈ 2 days. Mirror `Tcl_AppendObjToObj` from
  `tcl9.0.3/generic/tclStringObj.c`: store
  `(length, allocated, bytes)` and double `allocated` on
  exhaustion.
- **Expected impact:** 100×+ speed-up on `append` loops;
  raises `append.test` and `appendComp.test` pass rates from
  ≈ 35 % to expected 80 %+ (the rest are deeper bugs).
- **Acceptance:** the `string operations` microbench
  drops from 2,568 ns/op to ≤ 100 ns/op.

## Tier 2 — high-value, no behaviour change

### R3. Fix `frame_push`: smaller default + dirty-bitmap + grow on demand

- **Why:** [`06-hotspots.md`](06-hotspots.md) — proc call is
  the one per-op category where wasm loses to tclsh, **and**
  3 tcltest files (`set`, `incr`, `execute`) trap with
  `frame local table full`. Today the table both wastes 4 KB
  per push and overflows on real procs.
- **Effort:** ≈ 1–2 days.
  1. Drop `FRAME_BUCKET_COUNT` default to 16; grow geometrically
     on collision/overflow instead of trapping.
  2. Add a `u8` `dirty` mask written per `local_set`; only
     clear set buckets in `frame_push`.
- **Expected impact:**
  - no-arg proc call drops from 153 → ≈ 50 ns (parity with
    tclsh).
  - 3 trap-failing tcltest files convert to partial / pass.
  - Per-frame memory falls 16× for the common case.
- **Acceptance:** `proc f {}; f` microbench ≤ 60 ns/op;
  `set.test`, `incr.test`, `execute.test` reach a summary.

### R4. Investigate `tcltest::cleanupTests` `preserveCore` trap

- **Why:** 5 tcltest files (`parse`, `subst`, `for`,
  `foreach`, `parseExpr`) run their tests fine and trap during
  cleanup. Almost certainly one missing `info commands` or
  `info procs` filter combination. Low effort, big leverage —
  five `run-trap` files become `partial` or `pass` in one go.
- **Effort:** ≈ half a day to localise + fix.
- **Acceptance:** at least 3 of those 5 files reach a summary.

### R5. Pin runtime build mode in CI + session-start hook

- **Why:** [`04-debug-vs-release.md`](04-debug-vs-release.md)
  — perf gates need ReleaseFast or numbers swing 6×;
  correctness gates need Debug or pointer bugs hide.
- **Effort:** half a day in
  `.claude/hooks/session-start.sh` plus a CI matrix entry.

## Tier 3 — correctness gaps that block tcltest files

### R6. Wire `source <preopened-path>`

- **Why:** 3 tcltest files (`regexp`, `get`, `cmdIL`) load
  helper data with `source helpers.tcl`. Sample 6 also needs
  it (recursive — would still loop, but for the right reason).
- **Effort:** ≈ 1 day in the WASI fd-resolution path —
  resolve the path against the preopened root, read it as a
  bundle.

### R7. Round out `regexp` option parser

- **Why:** 3 tcltest files trap with
  `regexp: unsupported or unknown option`. Spencer engine
  is already vendored; the option parser in `cmds/regexp.zig`
  is what's incomplete.
- **Effort:** ≈ 1 day to add `-line`, `-indices`, `-command`,
  and audit the rest against `tcl9.0.3/generic/tclCmdMZ.c`.

### R8. `tcltest` constraint initialiser dispatch

- **Why:** `parseExpr.test`, `dict.test` trap with
  `ConstraintInitializer must be complete script` while
  loading bundled tcltest.
- **Effort:** ≈ 1 day. Check how our dispatch stringifies a
  constraint-script body before passing it to `eval`.

### R9. Implement `clock format -format`

- **Why:** sample 4 fails the stdout match purely because of
  this.
- **Effort:** ≈ 1 day. WASI `clock_time_get` plus a portable
  strftime port.

### R10. Positional-arg `format` (`%2$s`)

- **Why:** sample 10 + format.test trap on it.
- **Effort:** ≈ half a day in `valtypes/tcl_format.zig`.

### R11. Wrong-error-class for `string` (no sub)

- **Effort:** trivial — rewire the trap site to return
  `wrong # args` instead of `unsupported`.

### R12. `tcl::build-info` stub

- **Why:** `format.test` references it.
- **Effort:** trivial.

### R13. `switch` `-matchvar` / `-indexvar` forms

- **Why:** `switch.test` traps.
- **Effort:** ≈ half a day.

## Tier 4 — bigger swings

### R14. Strict-parser pass for unbraced expressions

- Sample 5 shows wasm runs code tclsh refuses. Aligning on
  tclsh's `invalid bareword` rejection is a parser change.
- **Effort:** medium; touches parse / lowering paths.

### R15. TclOO

- Sample 9 + 4 tcltest files (`oo`, `ooNext2`, `ooProp`,
  `ooUtil`) need it.
- **Effort:** large — own roadmap.

## Suggested sequencing

```
R1 ─┬─ R2 ─┬─ R3 ─┬─ R4 ─┬─ rest in any order
    │      │      │      │
    │      │      │      └── unblocks 5 files
    │      │      └────────── unblocks 3 files + perf
    │      └─────────────── unblocks append.test family
    └─────────────────────── unblocks 9 files + microbenches

R5 in parallel.
```

R1 + R5 can land in parallel; R2 builds on R1's allocator
changes; R3 and R4 are independent of the rest. After R1 +
R2 + R3 + R4, **expect WASM tcltest pass rate to jump from
1.0 % to ~25 – 40 %**: nine garbled-name files plus three
frame-overflow files plus five cleanup-trap files plus the
append family come back online without further work.

## Summary of expected wins

| Item | Files unblocked (tcltest) | Microbench wins |
|---|---:|---|
| R1 — allocator hygiene | ~9 garbled-name + 10 silent-trap | `expr`, `lappend`, `dict` complete |
| R2 — capacity `append` | ~2 (append + appendComp pass% jumps) | string ops 25× faster |
| R3 — frame fix | 3 (set, incr, execute) | proc-call no-args 3× faster |
| R4 — cleanupTests | 5 (parse, subst, for, foreach, parseExpr) | — |
| R6 — `source` | 3 (regexp, get, cmdIL) | — |
| R7 — regexp options | 3 (lseq, lrepeat, reg) | — |
| R8 — constraint init | 2 (parseExpr, dict) | — |

Cumulative: ≈ 27 of the 49 currently-trapping files can reach
a tcltest summary line after Tier 1 + 2 + 3 land.
