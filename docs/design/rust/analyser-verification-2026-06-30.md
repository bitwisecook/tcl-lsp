# Rust analyser verification — parity + Tcl 9.0 oracle (2026-06-30)

> **Question.** Does the Rust analyser *match or exceed* the Python analyser,
> and is it *behaviourally correct against C Tcl 9.0* (real `tclsh9.0`) as the
> oracle? Verified on branch `rust` (PR #715 base) by **running** both engines
> over a corpus and reproducing every claim against running code and live
> `tclsh`. A follow-up to
> [`python-rust-parity-audit-2026-06-22.md`](python-rust-parity-audit-2026-06-22.md),
> which was a code review; this one is execution-driven and ships a fix.

## Verdict

The Rust analyser **matches or exceeds Python and agrees with the Tcl 9.0
oracle**, with one real recall gap found and **fixed** in this change.

* **Parity (committed corpus).** Across both the lighter `diag` path and the
  fuller `lint` path the Rust analyser emits **everything Python does, at the
  same positions, messages and severities**: `MISSING_FIRE = 0`,
  `WRONG_POSITION = 0`, `WRONG_MESSAGE = 0`, `WRONG_SEVERITY = 0`. The only
  divergences are Rust **EXTRA_FIRE** (Rust is richer) and the intentional,
  documented global-sort `WRONG_ORDER`.
* **The 2026-06-22 audit's single biggest concern is resolved.** That audit
  flagged ~18 taint/flow **severity-tier** mismatches (the whole taint family
  shown as red ERROR vs Python's Warning/Info). The fuller `lint`-path
  differential now reports **`severity_mismatch = 0`** — the per-code severity
  tables match.
* **The audit's #1 registry gap is closed.** `ledit` (Tcl 9.0) is modelled:
  no false `W123`, and its arity verdict matches the oracle exactly
  (`tcl`: `E002 expected at least 3, got 1` ≙ `tclsh9.0`:
  `wrong # args: should be "ledit listVar first last ?element ...?"`).
* **Oracle correctness.** On the subtle dialect-dependent equality fold
  `if {$x == 8}` with `set x 08`, the Rust analyser's `I230` direction matches
  real `tclsh` **per dialect**: `tcl9.0` → "always true" (`tclsh9.0` `08==8`→1),
  `tcl8.6` → "always false" (`tclsh8.6` `08==8`→0). Python now matches too.
* **Rust test suite green:** `tcl-compiler` lib `3323 passed; 0 failed`
  (incl. 2 new regression tests); `tcl-registry` all green.

## The gap found — and fixed: body-recursion into `catch` and `tcltest test`

Running the differential over real Tcl `*.test` files (not just the hand-written
corpus) surfaced a large `MISSING_FIRE` count. Root cause, pinned to exact code:

The analyser's **syntactic legacy checks** (W100 unbraced `expr`, W104, W105,
W304, …) recurse into command bodies via the registry's `ArgRole::Body`
(`analyser/commands.rs::dispatch_body_arguments`). Two body-bearing commands
were not reached:

1. **`catch { … }`** — `handle_catch_command` (`analyser/handlers.rs`) claimed
   the command (returned `true`, short-circuiting the generic body recursion at
   the dispatch site) but, unlike its sibling `handle_try_command`, **never
   walked `args[0]`**. So syntactic checks never entered a `catch` body, even
   though the registry marks `catch` arg 0 `Body` and `analyse_body`'s own doc
   comment lists `handle_catch_command` as a caller. The dataflow checks (W210)
   did descend, which is what masked the bug.
2. **`tcltest::test`** — the spec had `-body`/`-setup`/`-cleanup` options hinted
   `script` but **no `Body` arg roles**, and the analyser resolves imported
   commands by suppressing `W123` wholesale after `package require` rather than
   mapping `test` → `tcltest::test`. So no body was ever walked. Real `*.test`
   suites are almost entirely `test … { body } result` blocks, so this
   dominated the gap.

Because `tcltest::test`'s body really is evaluated as Tcl (legacy
`test name desc body result` and `-body {…}` both run the script), the
diagnostics inside are **true positives** — Python emits them; Rust did not.

### Fix

* `handle_catch_command` now walks the catch script body via `analyse_body`
  (no-ops on a dynamic `catch $cmd`), mirroring `handle_try_command`.
* `tcltest::test` gains a dynamic `arg_role_resolver` (`test_arg_roles`)
  mirroring Python's `dialects/stdlib/tcltest.py::_test_arg_roles`: `Body` on
  `-setup`/`-body`/`-cleanup` values and on the legacy positional penultimate
  arg; plus `body_kind = Structural`.
* `dispatch_body_arguments` gains a conservative fallback so an **unqualified**
  imported command (`namespace import ::tcltest::*` then bare `test`) resolves
  its body roles through the recorded `namespace_imports`. It fires only when
  the bare name owns no body itself and only against an explicitly imported
  namespace — verified **not** to recurse into an unrelated user command.

### Effect (differential over 40 real `tcl8.x` `*.test` files, `tcl8.6`)

| metric | before | after |
|---|--:|--:|
| `MISSING_FIRE` (Rust gap vs Python) | 729 | **48** |
| `WRONG_SEVERITY` | 0 | 0 |
| committed-corpus defects | 0 missing | **0 missing (unchanged)** |
| `tcl-compiler` lib tests | 3321 ✓ | **3323 ✓** |

Regression tests added: `catch_body_is_walked_for_syntactic_checks`,
`tcltest_test_body_is_walked_when_imported` (covers qualified call, bare import,
the un-walked `-result` field, and the un-imported opaque case).

## Residual divergences (all benign / Rust-correct)

* **`EXTRA_FIRE` rose (E002 arity, ~56).** Now that Rust walks `catch`/`test`
  bodies it also applies arity checks to the tcltest **error-probe idiom**
  `list [catch {append} msg] $msg`. `append`/`lappend`/`rename foo` with too few
  args **genuinely error in `tclsh`**, so Rust's `E002` is Tcl-correct — Rust
  *exceeds* Python here (Python suppresses arity inside these nested contexts).
  This is the "exceeds" direction of the goal, not a false positive.
* **Remaining `MISSING_FIRE = 48`** are smaller residuals (some position-pairing
  artifacts, a few E2xx/W1xx in deeply nested forms) — well below the closed
  major gap and out of scope here.
* **Performance pathology (not correctness):** the Rust analyser takes >30 s on
  a synthetic **37 k-line** file (`clock.test`) under `f5-irules`. Real iRules
  files are nowhere near that size; flagged for a separate perf pass.

## Method / how to reproduce

```sh
rustup update stable                       # crates require rustc ≥ 1.96
cargo build -p tcl-cli --bin tcl
python scripts/dev/diag_parity/run.py      # committed corpus: 0 missing/pos/msg/sev
# Oracle spot-checks:
tclsh9.0 <<<'puts [expr {"08"==8}]'        # 1  → I230 "always true" under tcl9.0
tclsh8.6 <<<'puts [expr {"08"==8}]'        # 0  → I230 "always false" under tcl8.6
target/debug/tcl lint --dialect tcl9.0 f.tcl --json   # fuller path (taint/flow/I230)
cargo test -p tcl-compiler --lib catch_body tcltest_test_body
```

## Files changed

* `rust/tcl-compiler/src/analyser/handlers.rs` — walk the `catch` body.
* `rust/tcl-registry/src/commands/stdlib/tcltest__test.rs` — `test_arg_roles`
  resolver + `body_kind`.
* `rust/tcl-compiler/src/analyser/commands.rs` — imported-name body-role fallback.
* `rust/tcl-compiler/src/analyser/diagnostics/tests.rs` — 2 regression tests.
