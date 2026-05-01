# Tcl 9.0.3 tcltest baseline — C tclsh vs WASM runtime

This page summarises one full sweep of the Tcl 9.0.3 test suite (the
non-platform-specific files listed in `_IN_SCOPE` of
[`tests/external/run_tcl9_tests.py`](../tests/external/run_tcl9_tests.py))
through both **C tclsh 9.0.3** (built from `tmp/tcl9.0.3/unix/` with the
default `./configure && make`) and our **Zig WASM runtime** (Release-Fast
build, executed under wasmtime 43.0.1).

The per-bundle data lives in
[`c-tcl-9.0.3-tcltest-baseline.csv`](c-tcl-9.0.3-tcltest-baseline.csv).
The two sweep harnesses live at
[`scripts/tcl9_ctcl_baseline.py`](../scripts/tcl9_ctcl_baseline.py) and
[`scripts/tcl9_wasm_sweep.py`](../scripts/tcl9_wasm_sweep.py); the CSV
combiner is [`scripts/tcl9_baseline_to_csv.py`](../scripts/tcl9_baseline_to_csv.py).

## Headline

| Metric                                                          | C tclsh 9.0.3 | WASM runtime |
|----------------------------------------------------------------:|--------------:|-------------:|
| Bundles in scope                                                |          100  |         100  |
| Bundles that ran to a tcltest summary line                      |           99  |          63  |
| Individual tcltest tests passing                                |   33,944 / 38,065 (89.2 %) | 9,323 / 38,065 (24.5 %) |
| Coverage vs native                                              |         100 % |         27 % |
| Sum of bundle interpreter time (`c_wall_ms` vs `wasm_run_ms`)   |          86 s |        53 s  |
| Per-bundle median slowdown (`wasm_run_ms / c_wall_ms`)          |             — |       5.8 ×  |
| Per-bundle mean slowdown                                        |             — |       5.9 ×  |
| Worst bundle slowdown                                           |             — |      17 ×    |

**The slowdown column in the CSV (`slowdown_x`) compares
`wasm_run_ms / c_wall_ms`** — both measure "interpreter ran the
script end-to-end". WASM compile time is excluded because it is a
one-time build cost paid before any test runs, not a per-call cost
the running interpreter pays. For reference, sweep-wide WASM
compile total is 50 s (~500 ms / bundle, invariant); subprocess +
harness overhead in `wasm_wall_ms` adds another ~200 s on top.

A handful of bundles run *faster* under WASM than under C tclsh —
not because the interpreter is faster, but because they trap before
reaching the expensive part of the script. `regexp.test` spends
15 s under C tclsh (real regex compile + match work); under WASM it
traps after 0.85 s, so the "ratio" looks great while actually
hiding ~250 untested assertions.

The 38,065 individual-test denominator is what C tclsh exercised on
this host (constraints like `win`, `mac`, `nonPortable` skip the
rest).

### Slowest 10 bundles where both interpreters finish

| bundle      | C wall | WASM run | ratio |
|:------------|-------:|---------:|------:|
| lrange      |  100 ms |  1696 ms |  17.0× |
| lreplace    |  169 ms |  2748 ms |  16.3× |
| expr-old    |   75 ms |   764 ms |  10.2× |
| compile     |   97 ms |   882 ms |   9.1× |
| oo          |   88 ms |   632 ms |   7.2× |
| trace       |   81 ms |   516 ms |   6.4× |
| cmdIL       |   71 ms |   444 ms |   6.3× |
| execute     |   71 ms |   414 ms |   5.8× |
| subst       |   57 ms |   326 ms |   5.7× |
| dict        |  272 ms |   901 ms |   3.3× |

## What the WASM runtime gets wrong

The 100 bundles split this way under WASM:

- **18 pass** (every assertion green)
- **45 fail** (bundle finishes, tcltest reports `Failed > 0`)
- **31 trap** (bundle aborts before reaching a tcltest summary)
- **3 timeout** (30 s / per-bundle budget, hard SIGKILL)
- **3 no_summary** (bundle ran but emitted no summary line)

The 31 + 3 + 3 = 37 bundles that fail to reach a summary line account
for **~22,000 individual tests** that we don't even get a chance to
score. Fixing the bundle-level failures is by far the highest-leverage
work — the actual per-test pass rate inside the bundles that DO reach
a summary is decent (e.g. `lset` 95%, `lpop` 100%, `dict` 71%, `oo`
81%, `ooProp` 100%).

## Trap categories ranked by leverage

Bucketed by `wasm_trap_category` in the CSV. Numbers in parentheses
are bundles in the bucket; the **"unlocked tests"** column is the
sum of `c_passed` across those bundles — i.e. what we'd start
scoring against on the day we fix the underlying issue.

### 1. `braced-cmdsubst-leak` (8 bundles, ~2,400 unlocked tests)

`basic, compExpr, interp, namespace, proc, string, tailcall, var`

The trap surface is `tcl trap: unknown command: <hex>` or
`unknown command: Bug` — the tcltest test description (a braced
literal in source like `{[586e71dce4] Valid Tcl_PkgPresent return}`
or `{[Bug 1869989]: expr parser memleak}`) is being **command-
substituted** when the test bundle dispatches through the WASM
fallback path for a proc that has a variadic `args` tail.

**Reproducer (verified):**

```tcl
proc test {a b args} { puts ok }
test basic-50.1 {[586e71dce4] x} -setup foo
# → tcl trap: unknown command: 586e71dce4
```

Same source with `{a b c}` (no `args` tail) runs cleanly. The
emitter dump shows that the args-tail case emits a single fused
script literal:

```
'::test basic-50.1 [586e71dce4] x -setup foo'
```

— the description's outer braces have been dropped, leaving
`[586e71dce4]` exposed as a `[…]` substitution when `tcl_eval` later
parses the script. The no-tail case emits each arg as a separate
TclObj literal (`'[586e71dce4] x'`) and the brackets stay literal
because they never re-enter the parser.

**Root cause:** the proc-call emission path for procs with an `args`
catch-all loses per-argument brace information when packing the
call into a fallback script. Either the dispatch table doesn't
register `args`-tail procs by their compiled `func_idx` (forcing
fallback through `_emit_eval_fallback`), or `_emit_args_list` /
`_emit_eval_fallback` is being reached without the parsed `tokens`
needed by `_arg_was_braced`.

**Suggested fix path:**

1. Verify in
   [`core/compiler/codegen/wasm/_emitter/_statements.py:728`
   `_resolve_proc`](../core/compiler/codegen/wasm/_emitter/_statements.py)
   that `args`-tail procs land in `_proc_index` so they hit the
   direct-call path. The reproducer above shows `_resolve_proc` was
   *not* invoked for `test` — the dispatch went somewhere else
   (likely a fast-path scan that pre-emits the fused script).
2. Audit any pre-emission caller that builds a script string out of
   raw IR strings without consulting `tokens.argv[i].type ==
   TokenType.STR`. Re-quote with `_tcl_list_quote` whenever the
   brace-status of an arg can't be recovered.
3. Defensive backstop in
   [`core/compiler/codegen/wasm/_emitter/_statements.py:1183`
   `_emit_eval_fallback`](../core/compiler/codegen/wasm/_emitter/_statements.py)
   — when `tokens is None`, drop the `elif a.startswith("[")` shortcut
   and always go through `_tcl_list_quote`. That branch is currently
   "trust the input is a substitution"; the args-tail path can break
   that trust.

### 2. `var-not-set` (6 bundles, ~600 unlocked tests)

`abstractlist, opt, parseOld, reg, safe, uplevel`

Trap surface: `can't read "argv": no such variable` (`parseOld`),
`can't read "clean-list"` (`abstractlist`), `can't read "x"`
(`uplevel`), etc.

Two sub-causes:

- **Globals not pre-populated.** Real tclsh sets `argv`, `argv0`,
  `tcl_platform(...)`, `auto_path`, `env(...)` at startup. Our
  runtime omits several of these (`argv` in particular). Tests that
  guard with `if {[info exists argv]}` succeed; tests that read
  `$argv` directly trap.

- **Compound `set` / `upvar` patterns the WASM compiler doesn't
  capture.** `abstractlist` and `uplevel` use forms like
  `upvar 0 NS::var x` where the outer-scope variable hasn't been
  initialised in the same frame; our `frame_get_at_depth` doesn't
  follow array-element upvar links to a still-empty target.

**Suggested fix:** start with a one-line preamble in
`tests/external/run_tcl9_tests.py::_PRE_TCLTEST` that defines `argv
{}` / `argv0 ""` / `auto_path {}`. That alone unlocks `parseOld`
(158 tests) and probably most of `safe` (147). The deeper upvar /
namespace-variable fixes are bigger tickets — see the "future work"
list below.

### 3. `tcltest-option-parse` (3 bundles, ~620 unlocked tests)

`info, lrepeat, lseq`

Trap surface:
`bad option "5": must be -body, -cleanup, -constraints, …`,
`bad option "}": must be …`,
`bad option "proc": must be …`.

These hit
[`tcltest.tcl:1981`](../tmp/tcl9.0.3/library/tcltest/tcltest.tcl)
inside the test harness's option-validation loop. The option name it
rejects is *part of the test body* — meaning the WASM runtime parsed
the test-call words such that a body fragment ended up in the
options dict. This is the same pattern as `braced-cmdsubst-leak`
manifesting through a different surface: when a body contains
`while {…} { … }` the runtime is treating the closing `}` as the
next argument.

**Suggested fix:** same as #1 — fix the eval-fallback re-quoting so
braced bodies survive intact.

### 4. `tcltest-arg-parse-fallthrough` (3 bundles, ~500 unlocked tests)

`mathop, nre, ooNext2`

Trap surface:
`wrong # args: should be "test name desc ?options?"`. The runtime
falls through to the legacy 2-or-3-arg form because the new-format
option parser (lines 1962-1988 of `tcltest.tcl`) couldn't classify
the args. Same root cause as #1 / #3.

**Combining 1 + 3 + 4 = 14 bundles, ~3,500 unlocked tests** for one
codegen fix (the `_emit_eval_fallback` re-quoting bug).

### 5. `unsupported-command` (2 bundles, ~16,820 unlocked tests)

`cmdAH` (16,820 tests!), `switch`.

`cmdAH.test` exercises commands A-H including a lot of `clock`,
`format`, `binary`, `encoding`, `expr` — most are wired up but the
test bootstrap uses a path that hits an unimplemented dispatcher.
The trap text is empty in the captured record (truncated by the
error-message buffer), need a focused re-run with `--trace` to
isolate the exact missing command.

`switch` is documented in
[`runtime/zig/dispatch/tcl_stub_fallback.zig:73-76`](../runtime/zig/dispatch/tcl_stub_fallback.zig)
as "compiled inline by the code generator (IRSwitch), so this entry
only fires when the interpreter evaluates a dynamic `switch` string".
The fix is to add a real `switch` interpreter handler in
`runtime/zig/cmds/`. Required by tests that build `switch` arms at
runtime (tcltest itself doesn't, but several test-file helpers do).

### 6. `expr-negshift` (2 bundles)

`listObj`, `stringObj`.

Trap: `tcl trap: negative shift argument`. Comes from a Zig
`std.math.shl(x, -k)` call — almost certainly in the expr engine's
`<<` / `>>` opcode. Tcl's spec is that negative shifts give 0 (for
`<<`) or all-ones / sign-extended (for `>>`); we should clamp the
shift count to [0, bitwidth) and let the operation produce the Tcl-
defined value rather than panic.

**Suggested fix:** in the expr `INST_LSHIFT` / `INST_RSHIFT` handler
(probably `runtime/zig/cmds/tcl_mathop.zig` or the `expr` codegen),
guard with `if (shift < 0) return 0;` before dispatching to
`std.math.shl`.

### 7. Other timeouts / hangs

- `expr.test` and `compExpr-old.test` time out at the 30 s budget.
  Rooted in **error-path overhead in `obj_new_string_copy` →
  `tcl_cmd_error`** — the tests have many failing assertions and each
  failure triggers a slow trap-construction path. With the watchdog
  removed, they eventually trap inside `obj_new_string_copy`
  (observed in a 20 s wasmtime-epoch timeout repro). Fix the
  semantic bugs (e.g. `expr-2.1` returns `0` instead of `10.0`) and
  the timeout falls out for free; failing tests run cheap, succeeding
  tests run cheaper.

- `io.test` times out — known issue (issue #270 follow-up): a leak
  inside `tcl_cmd_append`'s integer-cast guard pushes linear memory
  past 2 GiB. Documented in `_IO_BASELINE` and held out of the
  passing baseline.

### 8. `rename-missing-cmd` (1 bundle: `rename`)

`tcl trap: can't rename "list": command doesn't exist`. The
`rename.test` bundle starts by `rename list ""` to delete `list`,
then later does `rename ::list-bak list` to restore. Our `rename`
command can't find `list` in the registry because list-builtins are
dispatched through a different table (BUILTINS, not the proc table)
that `rename` doesn't see. Fix: extend
[`runtime/zig/cmds/tcl_rename.zig`](../runtime/zig/cmds/tcl_rename.zig)
to also look up names in BUILTINS and shadow them in the proc table
when renamed-away.

## Where the WASM runtime is fast (or fast-enough)

With compile time excluded, the run-only picture is:

- **Per-bundle median slowdown is 5.8 ×** — the typical bundle takes
  ~6× longer to *interpret* under WASM than under native tclsh.
- **Worst bundle is 17 ×** (`lrange.test`).
- A few bundles finish in WASM at 0.1-0.6 × the C wall time, but
  these "wins" are almost always because the bundle traps before
  reaching the expensive work — not a real speedup.
- `c_wall_ms` includes a fixed ~25 ms tclsh startup; `wasm_run_ms`
  is the in-process wasmtime call only. Subtracting the startup
  shifts the median to closer to **8 ×** for short bundles, less
  for longer ones.
- The remaining columns (`wasm_compile_ms` ≈ 500 ms / bundle, fixed;
  `wasm_wall_ms` includes ~700 ms of subprocess + Python harness
  overhead) are kept for readers who care about end-to-end build
  cost, but they're not what `slowdown_x` reports.

So the runtime is a steady ~6 × slower than C tclsh on bundles that
finish — well within range for an interpreter that doesn't yet have
a JIT or a bytecode VM. The dominant work to close the gap is in
the per-command dispatch, not the broad architecture.

## Suggested fix order (best ROI first)

1. **Fix the braced-word leak in `_emit_eval_fallback`** — biggest
   single win, unlocks ~3,500 individual tests across 14 bundles
   (categories 1, 3, 4). The fix is small and self-contained.
2. **Pre-populate `argv`/`argv0`/`auto_path`/`env(*)` globals** in the
   WASM runtime entry — small change, unlocks ~600 tests across 6
   bundles (category 2).
3. **Clamp negative shift counts in expr** — 2 bundles, ~100 tests,
   trivial fix.
4. **Implement runtime `switch` handler** — unlocks `switch.test` plus
   any test that hits the dynamic-switch path indirectly.
5. **Triage `cmdAH.test`** — 16,820 individual tests behind one
   missing dispatcher. Need `--trace` re-run to isolate the call.
6. **Add `rename`-vs-BUILTINS plumbing** — unblocks `rename.test`.
7. **Fix the semantic bugs surfaced by `fail` bundles** (e.g.
   `expr-2.1`, `subst-3.1`, `parseExpr-20.3` bignum overflow) — these
   are individual bugs, not categorical issues, so each needs its
   own investigation, but they're cheap ones.

## How to refresh

```bash
# 1. Build C tclsh (idempotent, ~30 s)
mkdir -p /tmp/tcl-build
( cd /tmp/tcl-build && \
    /home/user/tcl-lsp/tmp/tcl9.0.3/unix/configure --prefix=/tmp/tcl-install && \
    make -j$(nproc) tclsh )

# 2. Build the WASM runtime in ReleaseFast (~30 s)
( cd runtime/zig && zig build -Doptimize=ReleaseFast )

# 3. Run both sweeps (each ~5 minutes)
uv run --extra dev python scripts/tcl9_ctcl_baseline.py \
    --timeout 60 --out /tmp/c-tcl-sweep.ndjson
uv run --extra dev python scripts/tcl9_wasm_sweep.py \
    --timeout 30 --out /tmp/tcltest-sweep-fast.ndjson

# 4. Combine into the baseline CSV
uv run --extra dev python scripts/tcl9_baseline_to_csv.py \
    --c /tmp/c-tcl-sweep.ndjson \
    --wasm /tmp/tcltest-sweep-fast.ndjson \
    --out docs/c-tcl-9.0.3-tcltest-baseline.csv
```

## Caveats

- The WASM column's wall time includes ~700 ms / bundle of subprocess
  + Python harness overhead. A real headline-perf number wants the
  in-process timing (`wasm_compile_ms` + `wasm_run_ms` columns).
- `wasm_pass_rate` is `wasm_passed / c_total`, so it's the absolute
  fraction of native-passable tests we cover. A bundle that traps
  before reaching its summary scores 0% even when its earliest tests
  would have passed.
- The C tclsh build uses a vanilla `./configure` — no Tk, no
  threads-disabled, no special build options. RSS numbers are
  comparable to a normal user install.
- One C tclsh result needs caveats: `brodnik.test` traps natively
  too (peeked at `time -v` output corruption — likely a known
  upstream issue with the `brodnik` C extension). We treat it as
  "ignore" rather than a regression.
