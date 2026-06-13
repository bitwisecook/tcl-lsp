# 05 — Correctness gaps found while running the samples and the tcltest sweep

These are not perf issues but they shape the perf story (a sample
that traps doesn't tell us anything about its inner loop) and
several are easy enough to fix that they should land before the
next perf pass.

The first half of this report covers the sample scripts; the
second half adds the new evidence from the
[in-scope tcltest sweep](08-tcltest-suites.md) — 97 files,
35,921 individual tests.

## Stdout disagreements

### `04_substitution_and_quoting.tcl`

```diff
  user=JIMD
  $who is not substituted here
- time=06:58:14
+ time=0
```

`clock format $now -format %H:%M:%S` returns `0` on wasm — the
runtime registers `clock format` as a sub-command but the
formatter is a stub that doesn't honour the `-format` argument.
Code: `runtime/zig/io/tcl_clock.zig`.

### `05_warning_examples.tcl`

The script intentionally writes pathological idioms; tclsh
**rejects** the unbraced expression on line 8 with

```
invalid bareword "foo" in expression "foo"; should be "$foo" or
"{foo}" or "foo(...)" or ...
```

WASM **accepts** it and prints `matched`, then mis-parses the
final string concatenation `"/tmp/"$a"/data.txt"` as
`/tmp/foo"/data.txt"` (stray quote retained).

These are two separate parser permissiveness bugs — both worth
fixing because they let sloppy code execute that real Tcl would
reject.

## Wasm traps that tclsh would also error on

### `07_arity_and_subcommand_errors.tcl`

Both backends fail (intentional). Different message:

| | message |
|---|---|
| WASM | `tcl trap: site=1 unsupported in WASM: string (no subcommand)` |
| tclsh | `wrong # args: should be "string subcommand ?arg ...?"` |

`string` with no subcommand should produce the standard `wrong # args`
error so `catch` users see the same failure category. Today it
looks like a runtime-completeness gap.

### `09_long_code.tcl`

```
tcl trap: site=3 unsupported command: oo::class
  in eval-script at offset 0: oo::class create Counter { … }
```

TclOO is not implemented in the runtime. This sample exercises
classes, ensembles, dict deep ops, `try / on error`, etc — it's a
parser exerciser by design. Worth keeping as a "what's missing"
checklist rather than a perf candidate.

### `10_format_strings.tcl`

```
tcl trap: unsupported command: format %2$s
```

Positional `format` arguments (`%1$s`, `%2$d`, …) are not
implemented in `runtime/zig/valtypes/tcl_format.zig`. The
sample also references undefined variables (`$name`, `$age`,
`$score`) — tclsh stops at the first such reference; wasm
reaches the format call before tripping. The latter is a
diagnostic-completeness gap (we should also raise on undefined
read).

## Wasm runtime traps in long-running benches

These three appeared in the microbench (see
[`03-microbench.md`](03-microbench.md)) and are all the same
root cause:

| Bench | trap site |
|---|---|
| `expr arithmetic (braced)` × 100 k | `tcl_string.tcl_expr_order_cmp → obj_new_int → write_i64 → __stack_chk_fail` (Debug) / out-of-bounds (Release) |
| `lappend L $i` × 20 k | `tcl_list.tcl_cmd_lappend` → out-of-bounds at `0x1040000` (16 MB ceiling) |
| `dict set d k$i $i` × 5 k | `tcl_dict.dict_set` → out-of-bounds at `0x376b2033` (≈ 928 MB — heap_ptr ran past linear-memory size) |

The bump allocator in `runtime/zig/valtypes/tcl_obj.zig:44`
unconditionally bumps `heap_ptr` and never calls `memory.grow`.
Once heap_ptr exceeds the WASM linear-memory page count we trap.
This is the single biggest blocker to running real workloads.

## Sample 06 must be skipped

`06_security_smells.tcl` runs `source $argv0` which sources the
script recursively forever. tclsh and wasm both spin until OOM /
stack overflow. The harness lists it as `NON_TERMINATING` and
skips both backends.

## Recap (samples only)

| Issue | Location | Severity |
|---|---|---|
| Bump allocator never grows / never frees big objects | `valtypes/tcl_obj.zig:44` | **blocker** |
| `clock format` ignores `-format` | `io/tcl_clock.zig` | medium |
| `format %N$x` positional args missing | `valtypes/tcl_format.zig` | medium |
| TclOO (`oo::class`, `oo::define`, …) missing | not implemented | low (own roadmap) |
| Unbraced expression accepted in `if`/`while` | parser | low |
| Stray quote in unbraced concatenation | parser | low |
| `string` (no sub) gives wrong error class | dispatch fallback | low |
| Reading undefined var should error | scope handling | low |

## Additional issues surfaced by the in-scope tcltest sweep

(See [`08-tcltest-suites.md`](08-tcltest-suites.md) for the full
trap-signature table — 49 of 97 files trap before the test
summary line is printed.)

### `unknown command: <garbage bytes>` — 9 files affected

Affected suites: `listObj`, `listRep`, `lrange`, `format`, `var`,
`namespace`, `trace`, `info`, `rename`. Sample garbage strings
seen in trap stderr: `2971669`, `tConstraintsHookam`, `gleFile`,
`\nds`, `\xc2\xa0Sb`. The pattern — random-looking ASCII or raw
bytes interpreted as a command name — is unmistakable: the
dispatcher receives a `(name_ptr, name_len)` pair pointing into
storage that has been overwritten between the time the parser
recorded it and the time dispatch reads it.

Root cause is the same bump allocator referenced in the samples
section: when `obj_release` returns memory to the free-list, an
in-flight `(ptr, len)` reference into the original payload is
never invalidated, so a subsequent `alloc()` re-uses the same
bytes for unrelated data and the dispatcher reads the
overlapping write. **This is now the highest-severity
correctness issue** — it gates 9 fundamental tcltest files and
will gate any non-trivial workload long before the obvious OOM
path triggers.

### `frame local table full` — 3 files affected

`set.test`, `incr.test`, `execute.test` — the per-frame local-
variable table is fixed at 256 buckets. These tcltest helpers
have procs with > 256 locals (or hash-collision chains that
fill the buckets). Bug **and** perf issue: today every frame
allocates 4 KB (16-byte buckets × 256), and the table is still
fixed-capacity, so deep procs both waste memory and overflow.

Fix: shrink default to 16 buckets, grow geometrically on
collision/overflow.

### `tcltest::cleanupTests` traps — 5 files affected

`parse`, `subst`, `for`, `foreach`, `parseExpr` all run their
test bodies fine but trap during the standard `cleanupTests`
post-amble. The trap message is uniformly `preserveCore` — the
helper that walks the master command table to flag any test
that registered a stray command.

This means **the runtime probably has a missing `info commands`
filter combination** (e.g. `-glob` + `::*`) that `cleanupTests`
relies on. Five `run-trap` files would convert to `partial`
or `pass` if this single helper worked.

### `unsupported command: source` — 3 files affected

`regexp`, `get`, `cmdIL` — these load helper data files via
`source helpers.tcl`. Our `source` is implemented but only for
literal in-bundle strings; the WASI preopen-fd path that would
let `source helpers.tcl` resolve against the test directory
isn't wired yet.

### `regexp: unsupported or unknown option` — 3 files affected

`lseq`, `lrepeat`, `reg.test` — bundle-internal `regexp` calls
fail because of an option flag (likely `-line`, `-indices`, or
the new `-command` form) that our regex shim doesn't accept.
The Spencer regex engine is vendored from Tcl 9 sources but
the option parser around it doesn't track the full set.

### `unsupported command: switch` — 1 file

`switch.test` — our `switch` implementation doesn't handle
the form the test exercises (probably `-matchvar` or
`-indexvar`).

### `ConstraintInitializer must be complete script` — 2 files

`parseExpr`, `dict` — the bundled `tcltest::testConstraint`
dispatch fails the constraint initialiser. tcltest 2.5 lets a
constraint be defined as a script that returns 0/1; our
dispatch path probably mistakes the script for a single
command word.

### `tcl::build-info` unknown — 1 file

`format.test` references `tcl::build-info patchlevel`. We
don't implement this internal accessor. Trivial to stub.

## Updated severity ranking

| Issue | Files affected (samples + tcltest) | Severity |
|---|---:|---|
| Bump allocator returns recycled / overlapping memory | 9 tcltest + microbench OOMs | **blocker** |
| `cleanupTests` `preserveCore` trap | 5 tcltest | high |
| `frame local table` fixed-capacity overflow | 3 tcltest | high |
| `tcl_cmd_append` is O(N) per call | sample 11 + `append`, `appendComp` partial fail | high |
| `source filename` (preopen-fd) | 3 tcltest + sample 6 | medium |
| `regexp` option set incomplete | 3 tcltest | medium |
| `clock format -format` returns 0 | sample 4 | medium |
| `format %N$x` positional args | sample 10 | medium |
| `switch -matchvar` / `-indexvar` | 1 tcltest | low |
| `tcltest` constraint initialiser dispatch | 2 tcltest | low |
| `tcl::build-info` stub | 1 tcltest | low |
| TclOO (`oo::class` …) | sample 9 + 4 tcltest | low (own roadmap) |
| Unbraced expression accepted in `if`/`while` | sample 5 | low |
| Stray quote in unbraced concatenation | sample 5 | low |
| `string` (no sub) wrong error class | sample 7 | low |
| Reading undefined var should error | sample 10 | low |
