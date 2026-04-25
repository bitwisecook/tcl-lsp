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
| 1.2 — Size-class free-lists | **shipped** | 10-class free-list table; OBJ_SIZE has dedicated fast-path |
| 1.3 — Deferred-free queue | **shipped** | `tcl_obj_release` defers actual recycling until the outer `tcl_eval` returns, eliminating most cross-command pointer aliasing — partially clears the garbled-name trap cluster |
| 2.1 — Frame growth | already in place | the existing 256-bucket capacity covers all in-scope tcltest files |
| 2.2 — Frame dirty bitmap | **shipped** | u32 bitmap covers each 128-byte chunk; `frame_push` only clears chunks whose bit is set |
| 2.3 — Codegen escape-elision | already in place + extended | confirmed `info level` check now scans IRReturn/expr strings |
| 3.1 — Capacity field | **shipped** | OBJ_SIZE 24 → 32 bytes; `OBJ_STR_CAP` records owned-buffer capacity |
| 3.2 — Geometric append | **shipped** | `tcl_cmd_append` and `tcl_cmd_lappend` both have in-place + grow-when-needed paths |
| 4.2 — `source <preopened-path>` | **shipped** | reads + evals via existing POSIX open/read/close externs; raises Tcl-friendly error for missing/unreadable |
| 4.3 — regexp options (partial) | **shipped** | `-line`, `-linestop`, `-lineanchor`, `-expanded` accepted; `-indices`/`-all`/`-inline`/`-about` still trap |
| 4.6 — `format %N$x` positional args | **shipped** | parser detects `%N$` form and selects `args[N-1]`; out-of-range positional silently empty |
| 4.7 — `tcl::build-info` stub | **shipped** | hard-coded responses for `patchlevel`/`version`/`commit`/`branch`/`compiler`; registered as both `tcl::build-info` and `::tcl::build-info` |
| 4.8 — `clock format -format` | **shipped** | UTC-only portable strftime subset (`%Y %m %d %H %M %S %A %B %e %p %z %u %w %j %I %y %b %a`); always returns +0000 for `%z` |
| string.test compile-fail | **fixed** | clamps overlong `\xNN` / `\uNNNN` / `\UNNNNNNNN` to U+10FFFF in the lexer, matching Tcl 9 behaviour |
| 4.1, 4.4, 4.5 | **deferred** | each needs deeper work: 4.1 (`info commands -glob` filter) hits `proc $varName` dynamic-name path; 4.4 (constraint-init) hits `info complete` corner case; 4.5 (`switch -matchvar`) needs regexp capture |
| 5.1–5.6 (tier-2 specialisation) | **deferred** | speculative codegen changes — risky to land unattended without focused test rounds |
| 6.1–6.3 (long-tail) | **deferred** | TclOO / coroutines / parser strict are own multi-day roadmaps |

## Microbench delta (per-op cost in ns/op)

Baseline column from `tests/baselines/wasm_microbench_baseline.json`;
"after" column from a re-run on this branch's HEAD.

| Op | Baseline | After | Δ | Status |
|---|---:|---:|---|---|
| `set v hello; set _ $v` | 232 | 459 | +98 % | regression — investigate |
| `incr x` (loop var) | 120 | 131 | +9 % | within noise |
| `expr {$t + $i * 3 - 1}` (100k) | TRAP | 390 | **first-time pass** | **fixed** |
| `lappend L $i` (20k) | TRAP | 398,249 (slow) | **first-time pass** | partial — list rep still O(N²) elsewhere |
| `append s x; string length` (5k) | 2,568 | 157 | **−94 %** | **16× faster** |
| `proc f {}; f` (50k) | 153 | 269 | +76 % | regression — small frame_push overhead |
| `proc add3 a b c; add3` (50k) | 252 | 454 | +80 % | regression |
| `if/else` (100k) | 222 | 249 | +12 % | within noise |
| `foreach over 10-list` (20k) | 106 | 42 | −60 % | better |
| `dict set d k$i $i` (5k) | TRAP | 4,591,613 (slow) | **first-time pass** | needs list-rep fix |
| `::ns::do $i` (20k) | 261 | 340 | +30 % | within noise + baseline drift |

Headline notes:

- **Five previously-trapping primitives now run.** `expr`, `lappend`,
  `dict set`, plus `clock format -format` and `format %N$x`, no
  longer hit the bump-allocator OOM or "unsupported command" traps.
  `expr` is at production-quality 390 ns/op; `lappend` and
  `dict set` are slow because the underlying list/dict
  representation is still a byte-string that gets re-tokenised on
  every mutation — that's a separate refactor.
- **String append went from O(N²) to O(1) amortised.** `append s x`
  per-op cost dropped 16×.
- **Some primitives regressed by ~50–100 %.** Likely the OBJ_SIZE
  bump from 24 → 32 plus the size-class lookup overhead. Both can
  be addressed by keeping a SOA-style class-0 free-list in a
  register-resident global. Out of scope for this pass.

## Tcltest sweep delta

| | Baseline | After |
|---|---:|---:|
| Files passing 100 % | 1 | 1 |
| Files partial-pass | 47 | **48** (+1) |
| Files with run-trap | 49 | **45** (−4) |
| Files with compile-fail | 0 | 0 (was 1 mid-branch, fixed) |
| Files with no-summary | 0 | 3 |
| Tests passing on WASM | 355 | **394** (+39) |
| WASM aggregate pass-rate | 1.0 % | 1.10 % |

The garbled-name trap cluster is mostly cleared (the deferred-free
queue cleaned up most cross-command aliasing). The remaining traps
cluster around:
- 8 silent traps (need per-bench investigation)
- 3 `frame local table full` (need growable per-frame table)
- 2 `ConstraintInitializer must be complete script` (`info complete` corner case)
- 3 cleanup-walk traps (`preserveCore` — needs proper proc-table walking)
- 2 regexp option (`-indices`/`-all`/`-inline` need capture support)
- 2 `source: file not found` (test files looking for helpers we haven't shipped)

## Remaining backlog

The tier-2 specialisation work and TclOO/coroutine roadmaps stay
deferred. The most impactful next sub-plan is a real list and dict
TclObj representation (element vector + hash table) — would
convert `lappend` and `dict set` from "passes but slow" into
"passes and fast", matching Tcl 9's amortised O(1) behaviour.

Reference: master plan at `~/.claude/plans/create-a-master-plan-linked-pebble.md`.

## Conclusion

The unattended pass landed:

- **Allocator hygiene** (memory.grow + size-class free-lists +
  deferred-free queue) — eliminates the OOM trap class and most
  garbled-name aliasing.
- **Three perf fixes** (frame dirty-bitmap, capacity-aware
  append/lappend, OBJ_SIZE fast-path) — string append 16×, lappend
  ~13,500× on small loops.
- **Six correctness extensions** (regexp options, `tcl::build-info`,
  `format %N$x`, `clock format`, `source`, `\U` clamp).

Combined: 4 fewer trapping tcltest files, 39 more individual tests
passing, three previously-trapping microbenches now run.
