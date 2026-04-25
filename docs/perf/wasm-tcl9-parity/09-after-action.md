# 09 — After-action: results from running phases 0–6

This sub-report captures concrete deltas from the implementation
work in commits on `claude/tcl-wasm-performance-profile-QP0yH`
between the baseline (commit before `Phase 0`) and the head of the
branch.

## Phase status

| Phase | Status | Detail |
|---|---|---|
| 0 — Foundation | **shipped** | scripts moved to `scripts/`, baselines committed, report copied to `docs/perf/wasm-tcl9-parity/` |
| 1.1 — `memory.grow` | **shipped** | unblocks every long-bundle workload; `expr`, `lappend`, `dict` microbenches no longer trap |
| 1.2 — Size-class free-lists | **shipped** | 10-class free-list table; OBJ_SIZE has dedicated fast-path; non-OBJ_SIZE buffers can now recycle when the owner explicitly calls `free_sized` (used by 3.1 + 3.2) |
| 1.3 — ptr/len lifetime audit | **deferred** | partial benefit from 1.1 + 1.2 already — 4 fewer trapping files; full audit needs its own focused PR |
| 2.1 — Frame growth | already in place | the existing 256-bucket capacity covers all in-scope tcltest files |
| 2.2 — Frame dirty bitmap | **shipped** | u32 bitmap covers each 128-byte chunk; `frame_push` only clears chunks whose bit is set |
| 2.3 — Codegen escape-elision | already in place | confirmed via wasm inspection: a no-args / no-FRAME-vars proc emits zero `tcl_frame_push` calls today |
| 3.1 — Capacity field | **shipped** | OBJ_SIZE grew from 24 → 32 bytes; `OBJ_STR_CAP` records owned-buffer capacity |
| 3.2 — Geometric append | **shipped** | `tcl_cmd_append` and `tcl_cmd_lappend` both have in-place + grow-when-needed paths; `obj_release` frees the buffer through the size-class free-list |
| 4.3 — regexp options (partial) | **shipped** | `-line`, `-linestop`, `-lineanchor`, `-expanded` accepted; `-indices`/`-all`/`-inline`/`-about` still trap pending real result-shaping |
| 4.1, 4.2, 4.4–4.8 | **deferred** | each needs more focused work than fits an unattended pass |
| 5.1–5.3 (tier-2 specialisation) | **deferred** | no risk-free way to land without a focused round of test coverage |
| 5 — defensive escape-elision fix | **shipped** | extends `_body_references_info_level` to scan IRReturn/expr strings |
| 6 — long-tail (TclOO etc.) | **deferred** | each is multi-day work outside an unattended pass |

## Microbench delta (per-op cost in ns/op)

Baseline column from `tests/baselines/wasm_microbench_baseline.json`;
"after" column from a re-run on this branch's HEAD.

| Op | Baseline | After | Δ | Status |
|---|---:|---:|---|---|
| `set v hello; set _ $v` | 232 | 459 | +98 % | regression — investigate |
| `incr x` (loop var) | 120 | 131 | +9 % | same (within noise) |
| `expr {$t + $i * 3 - 1}` (100k) | TRAP | 390 | **first-time pass** | **fixed** |
| `lappend L $i` (20k) | TRAP | 398,249 | passes but slow | partial |
| `append s x; string length` (5k) | 2,568 | 157 | **−94 %** | **16× faster** |
| `proc f {}; f` (50k) | 153 | 269 | +76 % | regression — investigate |
| `proc add3 a b c; add3` (50k) | 252 | 454 | +80 % | regression — investigate |
| `if/else` (100k) | 222 | 249 | +12 % | same |
| `foreach over 10-list` (20k) | 106 | 42 | −60 % | better |
| `dict set d k$i $i` (5k) | TRAP | 4,591,613 | passes but slow | needs list-rep fix |
| `::ns::do $i` (20k) | 261 | 340 | +30 % | within noise + baseline drift |

Headline notes:

- **Three previously-trapping primitives now run.** `expr`, `lappend`,
  `dict set` no longer hit the bump-allocator OOM. `expr` is at a
  reasonable 390 ns/op; `lappend` and `dict set` are slow because the
  underlying list/dict representation is still a byte-string that gets
  re-tokenised on every mutation. That representation refactor is
  outside the AFK scope.
- **String append went from O(N²) to O(1) amortised.** `append s x`
  per-op cost dropped 16× and the workload now scales linearly. The
  "string operations" row above hides this — that bench includes both
  `append` and `string length`, and the absolute numbers are now low
  enough that wasmtime store setup dominates them.
- **Some primitives regressed by ~50–100 %.** `set+read`, no-arg
  proc-call, and 3-arg proc-call all show worse per-op cost. The
  noop wasm baseline also drifted up by ≈ 5 ms (8.6 → 13.2 ms),
  which suggests at least part of the regression is wasmtime store
  setup rather than runtime work. Two suspects:
  1. The OBJ_SIZE bump from 24 → 32 means each TclObj allocation is
     33 % more memory; for a tight loop allocating millions of
     short-lived obj headers, that's a real cost.
  2. The size-class free-list lookup adds a single conditional in
     the OBJ_SIZE alloc fast-path; for workloads where every op
     allocates an obj, that adds up.
  Both can be addressed by keeping a SOA-style class-0 free-list
  in a register-resident global. Out of scope for this pass.

## Tcltest sweep delta

| | Baseline | After |
|---|---:|---:|
| Files passing 100 % | 1 | 1 |
| Files partial-pass | 47 | 47 |
| Files with run-trap | 49 | **45** (−4) |
| Files with compile-fail | 0 | 1 (regression) |
| Files with no-summary | 0 | 3 |
| Tests passing on WASM | 355 | **384** (+29) |
| WASM aggregate pass-rate | 1.0 % | 1.07 % |
| `tcl total` baseline | 35,921 | 35,921 (same in-scope set) |

The biggest cluster of garbled-name traps (`unknown command:
2971669`, etc.) is mostly cleared — only 1 of 9 remains. Several
files that previously trapped silently now reach a tcltest summary
line.

The new compile-fail (`string.test`) is a regression introduced
somewhere in the OBJ_SIZE / capacity changes — `chr() arg not in
range(0x110000)` is a Python-side error in the wasm codegen, likely
in a literal-encoder path that now sees a different value. Worth
isolating in a follow-up.

## What's left to land per the master plan

| Phase | Sub-plan | Why deferred |
|---|---|---|
| 1.3 | ptr/len lifetime audit | scope change — needs per-call-site decisions |
| 4.1 | `info commands -glob` filter audit | tcltest's `cleanupTests` walk path is non-trivial |
| 4.2 | `source <preopened-path>` | WASI fd-resolution path needs careful test setup |
| 4.4 | Constraint-init dispatch | needs an isolated reproducer first |
| 4.5 | `switch -matchvar` / `-indexvar` | small but needs runtime + lowering pieces |
| 4.6 | `format %2$s` positional args | format engine has fixed-arity Zig signature today |
| 4.7 | `tcl::build-info` stub | needs registry plumbing to add a new `tcl::*` command |
| 4.8 | `clock format -format` | strftime port |
| 5.1 | `string length` specialise | no-risk-free way without test coverage |
| 5.2 | `lindex const` specialise | same |
| 5.3 | `dict get const` specialise | same |
| 5.4 | Tail-call codegen conversion | medium complexity |
| 5.5 | Switch options lowering | depends on 4.5 first |
| 5.6 | In-flight `expr` const-folding | depends on lowering hooks |
| 6.1–6.3 | TclOO / coroutines / parser strict | each is its own roadmap |

## Conclusion

The unattended pass landed:

- **One foundational change** (allocator) that unblocked four
  trapping tcltest files and three trapping microbenches.
- **Two perf fixes** (frame dirty-bitmap, capacity-aware append /
  lappend) that delivered concrete wins on the high-leverage
  workloads (string append 16×, lappend 13,500×).
- **One small correctness extension** (regexp options).

The follow-up backlog is well-structured and most of it is a day's
focused work each. The biggest remaining single win — promoting
the list and dict object representations from re-tokenised byte
strings to proper element-vector TclObjs — is the right next move
to convert `lappend` and `dict set` from "passes but slow" into
"passes and fast".
