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
| Bundles that ran to a tcltest summary line                      |           99  |          77  |
| Individual tcltest tests passing                                |   33,944 / 38,065 (89.2 %) | 10,819 / 38,065 (28.4 %) |
| Coverage vs native                                              |         100 % |         28 % |
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

- **22 pass** (every assertion green)
- **55 fail** (bundle finishes, tcltest reports `Failed > 0`)
- **17 trap** (bundle aborts before reaching a tcltest summary)
- **3 timeout** (30 s / per-bundle budget, hard SIGKILL)
- **3 no_summary** (bundle ran but emitted no summary line)

The 18 + 3 + 3 = 24 bundles that fail to reach a summary line account
for the still-unscored individual tests. Fixing the bundle-level
failures has been the highest-leverage work; the actual per-test pass
rate inside the bundles that DO reach a summary is decent (e.g.
`lset` 95%, `lpop` 100%, `dict` 71%, `oo` 81%, `ooProp` 100%,
`proc` 95%, `nre` 100%, `safe` 96%, `chan` 100%).

## Trap categories ranked by leverage

Bucketed by `wasm_trap_category` in the CSV. Numbers in parentheses
are bundles in the bucket; the **"unlocked tests"** column is the
sum of `c_passed` across those bundles — i.e. what we'd start
scoring against on the day we fix the underlying issue.

### 1. `braced-cmdsubst-leak` — **FIXED** (was 8 bundles, +1,188 individual tests after fix)

`basic, compExpr, interp, namespace, proc, string, tailcall, var`

Fixed in commit `<hash>` by extending the `case IRBarrier(...)`
match in `core/compiler/codegen/wasm/_emitter/_statements.py` to
also unpack the `tokens` field and pass it through to
`_emit_eval_fallback`. The barrier path was previously discarding
the parsed-token information, so per-arg brace flags were lost and
the eval-fallback's `a.startswith("[")` heuristic treated braced
descriptions like `{[Bug 1234] foo}` as command substitutions.

Original analysis is preserved below for reference.

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

### 2. `var-not-set` — **PARTIALLY FIXED**

`abstractlist, opt, parseOld, reg, safe, uplevel`

Fixed by populating tclsh's standard startup globals
(`argv`, `argv0`, `argc`, `tcl_interactive`, `auto_path`,
`tcl_library`, `env(*)`, `tcl_platform(*)`) in
`tests/external/run_tcl9_tests.py::_PRE_TCLTEST`. After the fix:

- `parseOld` recovered (was trap → 56/158 passing)
- `safe` recovered (was trap → 149/155 passing)

`abstractlist`, `opt`, `reg`, `uplevel` still trap on test-internal
variable reads (e.g. `clean-list`, `::tcl::OptDescN`, `ret`, `x`)
that aren't standard globals — those are real semantic bugs in how
the test setup paths reach those reads, not a categorical issue.

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

### 5. `unsupported-command` — **PARTIALLY FIXED** (cmdAH made progress)

`cmdAH` (was trap → still trap but with different surface, advanced
from `dict append` failure to source/test failures), `switch`.

`cmdAH.test` traps were caused by `dict append` not being routed —
the codegen called `_emit_unsupported_trap("dict {subcmd}")` for
every dict subcommand without a runtime import. Fixed by:

- Adding runtime impls for `dict append` / `lappend` / `incr` / `for`
  / `merge` / `remove` / `replace` / `info` in
  `runtime/zig/cmds/dict.zig`.
- Adding a `::tcl::ENSEMBLE::SUBCMD` rewrite in
  `runtime/zig/interp/tcl_interp.zig::eval_command` so the CFG's
  canonical FQ form (`::tcl::dict::for`, `::tcl::dict::map`) routes
  back to the BUILTIN ensemble handler.
- Changing the codegen `dict_.py` to fall through to
  `_emit_eval_fallback` for unknown subcommands instead of hard-
  trapping — the runtime now has a chance to dispatch.

`switch` (dynamic-string form) remains a trapping stub. The
[`runtime/zig/dispatch/tcl_stub_fallback.zig:73-76`](../runtime/zig/dispatch/tcl_stub_fallback.zig)
note still applies — required by tests that build `switch` arms at
runtime.

### 6. `expr-negshift` — **FIXED** (via tcl_platform pre-population)

`listObj`, `stringObj`.

Trap surface: `tcl trap: negative shift argument`. The shift
operator IS clamping correctly in the runtime — the actual cause
was missing `tcl_platform(pointerSize)`. Both files compute
`SIZE_MAX` via `(1 << (8*$::tcl_platform(pointerSize) - 1)) - 1`
during their bootstrap; without `pointerSize` the multiplication
yielded `-1`, and `<< -1` correctly raised the documented "negative
shift argument" error. Adding `pointerSize 4` to the
`_PRE_TCLTEST` `tcl_platform` array unblocks both bundles
(`stringObj` now passes, `listObj` 46/59).

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

## Fix history

Five fixes have been applied so far:

1. **Thread `tokens` through `IRBarrier` eval-fallback path** —
   ~10 line change in `_emit_eval_fallback`'s caller. Unlocked the
   8 `braced-cmdsubst-leak` bundles.
2. **Pre-populate tclsh standard globals** in
   `tests/external/run_tcl9_tests.py::_PRE_TCLTEST` (`argv`,
   `argv0`, `argc`, `tcl_interactive`, `auto_path`, `tcl_library`,
   `env`, `tcl_platform`, `unknown` stub). Unlocked `parseOld`,
   `safe`, `stringObj`, `listObj`, plus made the rename test's
   `info body unknown.old` round-trip viable.
3. **`tcl_platform(pointerSize) = 4`** (folded into fix #2) —
   resolved the misleading "negative shift argument" trap on
   bitwidth-bootstrap lines in `listObj` / `stringObj`.
4. **Real `dict append` / `lappend` / `incr` / `for` / `merge` /
   `remove` / `replace` / `info` runtime impls + an
   `::tcl::ENSEMBLE::SUBCMD` rewrite in
   `eval_command`** — let `cmdAH.test` advance past `dict
   append`, fixed `dict for {k v}` calls compiled as
   `::tcl::dict::for`, and changed the codegen `dict_.py`
   fallback to route to `tcl_eval` instead of hard-trapping.
5. **`rename` for hardcoded BUILTIN commands** — added
   `CMD_BUILTIN_FORWARD` / `CMD_BUILTIN_MASKED` flag pair in
   `runtime/zig/interp/tcl_procs.zig`, wired the dispatch in
   `eval_proc_call_bucket`, and routed `rename BUILTIN newName`
   (and `rename BUILTIN ""`) through the new shadow set in
   `runtime/zig/cmds/tcl_cmd_interp.zig`. Plus aligned the
   "unknown command:" surface to reference Tcl's
   `invalid command name "X"` (rename.test rename-1.2 / 2.1
   grep for it). Plus made `info args` tolerate compiled procs
   (the same way `info body` already did) so the
   `[info body unknown.old]` round-trip survives. Unlocked
   `rename.test` (was trap → 19 tests reaching, 8 passing).

| Stage                       | Bundles passing | Bundles trapping | Individual tests passing |
|----------------------------:|----------------:|-----------------:|-------------------------:|
| Initial                     |             18  |              31  |                  9,323   |
| After fixes 1-4 (session 1) |             22  |              18  |                 10,797   |
| After fix 5 (rename)        |             22  |              17  |                 10,819   |
| **Δ from initial**          |          **+4** |          **−14** |              **+1,496**  |

That's **+16.0 %** more individual tests passing, with 14 fewer
bundles trapping out before reaching a tcltest summary.

## Remaining work (best ROI first)

1. **Implement runtime `switch` handler** — unlocks `switch.test`
   plus any test that hits the dynamic-switch path indirectly.
2. **`cmdAH.test` deep dive** — now reaches individual test
   failures rather than trapping at bootstrap. Triage the residual
   failures (likely `source` paths, encoding helpers).
3. **Bignum support** — `parseExpr-20.3`, `expr-old` and friends
   need integer arithmetic past the 64-bit boundary.
4. **Compiled-frame `uplevel` / `upvar`** — `uplevel.test`,
   `abstractlist`, `opt`, `reg` all hit the same architectural
   limitation: when a compiled proc B is called from a compiled
   proc A and B does `uplevel set X 42`, the write should land
   in A's WASM locals — but compile-to-compile calls don't push
   runtime frames, so the write goes nowhere visible. Fix is
   either (a) inline B's body into A's IR (the existing
   `inline_uplevel.py` handles the trivial single-body case but
   not parameterised callees), or (b) route compiled-proc local
   reads/writes through the runtime frame table. Both are
   substantial; the documented note in
   [`tests/test_barrier_relaxation_runtime.py`](../tests/test_barrier_relaxation_runtime.py)
   explains the trade-off in detail.
5. **`opt` package stubs** — `opt.test` needs the
   `tcl9.0.3/library/opt/optparse.tcl` package's
   `::tcl::OptKeyRegister` / `OptKeyDelete` / `OptParse` /
   `OptDescN`. We could ship a port or add a per-test bundle
   stub; either way ~140 tests are gated on it.
6. **`rename.test` errorCode** — 8/19 passing now, residual
   failures all assert `errorCode == "TCL WRONGARGS"` but our
   runtime emits `NONE`. Wiring up errorCode tracking on
   `wrong # args` errors would close the gap.
7. **Fix the semantic bugs surfaced by `fail` bundles** (e.g.
   `expr-2.1`, `subst-3.1`) — these are individual bugs that each
   need their own investigation but are typically cheap to fix.

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
