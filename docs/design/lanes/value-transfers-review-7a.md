# Slice 7a (seedless return summaries) — adversarial review

> Recovered copy. The machine that ran this review failed and was replaced on
> 6 October; the review's harness, programs and outputs went with it. Every
> section below but one is the file as the reviewer wrote it, read back from
> the coordinator's transcript; the section *What the slice gets right* was
> never read before the loss and is reconstructed from the reviewer's own
> hand-back summary, shorter than the original.

Tree: `/home/user/tcl-lsp`, branch `claude/spectcl-optimization-discussion-5qhf42`,
HEAD `3076a049a` (the slice), slice commits `1dbbd9a93` (D287), `dd166c181`
(#2389, D288), `d4ca2bcbd` (#2393, D289), `3076a049a` (VT7a.1/VT7a.2, D290–D294),
base `24c21fdfa`; `git diff 24c21fdfa..3076a049a`, 18 files, the hand-off
commits in between docs only. Read: the lane doc's slice 7a plan and record
(VT7a.1, VT7a.2, D287–D294, the status section), `interprocedural-analysis.md`
(the new Step 2b), `interprocedural-call-site-seeding.md`,
`optimisation-passes.md` (O103), the return and summary material of
`value-transfers.md`, `value-transfers-examples.md`, the slice 7a rows of
`value-transfers-migration.md`, `precision-limitations.md`, the O103 KCS note;
the diff in full for `interprocedural.rs` (`seedless_returns`, `exit_value`,
`return_value`, `fallthrough_value`, `tail_value`, `var_value`, `expr_value`,
`statement_may_return`, `classify_return`, `classify_return_expr`,
`summarise_returns`, `constant_return_of`, `ConstantReturn::text`),
`optimiser/propagation.rs` (`try_fold_static_proc_call`, `try_o103_proc_fold`,
`summary_answers`, `evaluate_proc_with_constants`, `is_value_safe_bare_word`,
the deleted `resolve_return_constant` family), `optimiser/manager.rs`,
`compilation_unit.rs`, `value_transfer.rs` (`const_to_exact`), the three test
files and the docs; plus the unchanged code the slice leans on
(`recorded_word_value`, `ExactValue::from_literal`, `render_propagation_word`,
`is_safe_word`, `arity_from_names`, `parse_static_call_args`,
`seed_params_from_args`, `format_double`, `scan_role_code_arguments`, the lsp-db
`ConstReturnKey` projection, the explorer's `format_return_shape`).

Measured with the CLI built from HEAD (`tcl 0.1.0+g3076a049`, `CARGO_BUILD_JOBS=2
cargo build -p tcl-cli`, copied to `review-vt7a/bin/tcl` because the
implementer's suite prunes `target/debug`) against tclsh 8.4.20, 8.5.19, 8.6.18,
9.0.4 and 9.1.0 (`TCL_LIBRARY` set per release). Oracle per program: stdout and
exit status under each release before and after `tcl opt --profile full`, once
under the auto-detected dialect and once under the release's own `--dialect`;
`tcl diag` under auto and every `--dialect`, read against the release's
behaviour. Harness: `review-vt7a/run.sh` (the slice 12 review's, re-rooted);
programs under `review-vt7a/progs/` (hand batches `a*` computed returns through
the string, list, expr, incr/append, format/scan/regexp commands, nested
substitutions and the fall-through shapes; `b*` returns that must stay unfolded
and global/upvar writes; `c*` control flow, `return` options, raising callees;
`d*` the exact rendering of 65 literal values in three shapes and 45 computed
ones; `e*` call shapes for D288; `f*` procedures calling procedures, recursion,
redefinition, namespaces, `rename`/`interp alias`, builtin shadowing; `g*`
dialect axes; `h*` script-running commands around D289 per release line;
`i*`/`j*`/`k*`/`p_*` isolations; `gen/*` the generated sweep); every run's
outputs under `review-vt7a/out/<name>/`, the per-release evidence for the cited
programs in `out/evidence.txt`.

- Hand batches a–k (33 programs through the full matrix, 330 rows, plus the
  `i13`–`i21` and `p_*` isolations run by hand): every DIFF row is attributed
  below, none to the slice — a03 (`**` folded under `--dialect tcl8.4`, P5),
  a04 (a quoted word with two substitutions, P1), a05 (the fall-through `set`
  deleted, #2264), b01/i09/i12 (O109 deletes a top-level store a procedure
  reads, P4), f03 (a builtin shadowed by a later `proc`, P3), g01 (a panic under
  the 8.5+ dialects, P6; under 8.4 P5 and #2395/P8), h01 (O107 drops a word of
  `for {return 1} {1} {} {}`, P9), h04 (`array for`, P2), j06 (`upvar 0`,
  #2306), k01 (8.4's exponent and subnormal axes, P8).
- Generated sweep `gen.py` (504 programs: 63 return expressions — string, list,
  expr, format, scan, regexp, incr/append/lappend, nested and quoted
  substitutions, 22 literal spellings — in 8 return shapes: `return $x`, a copy,
  inside an `if`, after a `foreach`, the fall-through, a `return [expr …]`
  reading it, a procedure with a default parameter and `args`, after a `while`;
  each called as `set r [p]`, `puts [p]`, nested in `string length`/`llength`,
  and as a bare statement; auto dialect, five releases): SWEEP_ROWS rows,
  SWEEP_DIFF DIFF rows — FALL_ROWS rows the `_fall` programs (`proc p {} { set x
  E }`, whose `set` O126 deletes as unused: #2264), FRAC_ROWS rows the
  `exprfrac` programs (`0.1 + 0.2` folded as `0.30000000000000004` and run
  under tclsh 8.4, which prints `0.3`: #2395 under the auto dialect), and 2
  rows `g270_format_retexpr` under 9.0 and 9.1 (`[format %05d 42]` is `00042`,
  which `return [expr {$x eq "zz" ? 0 : $x}]` reads as octal 34 under the auto
  dialect's 8.6 grammar where 9.x print 42; under `--dialect tcl9.0` it folds
  to 42: the dialect default, the slice 11/12 reviews' P6 class) — nothing
  else.
- The whole witness binary at HEAD: `cargo test -p tcl-compiler --test
  value_transfer_witnesses` — 152 passed, 0 failed (362.65 s). The slice's unit
  tests on the restored tree: `cargo test -p tcl-compiler --lib -- interprocedural:: optimiser::propagation::tests` 156 passed; `--test inlining_interproc_residual` 58 passed (`the_shapes_read_a_literal_return_exactly` among them); `cargo test -p tcl-cli --test value_transfers_cli -- o103_summary_path_folds_a_computed_return` 1 passed (`unit-tests.log`).

Every B/S/P item was checked against the open list (`review-vt7a/open-issues.txt`,
177 open issues) and the task's known list: #2264 covers the fall-through dead
store, #2306 the `upvar 0` alias, #2395 the 8.4 twelve-digit spelling (P8 widens
it; #2234 is its large-magnitude neighbour), #2307 is O107's other `for`
rewrite; nothing covers P1–P7 or P9.

## B — blocking

None. Nothing the slice adds — the seedless constant, the exact rendering, the
D288 call test, the D289 exit test, the D293 override — folded a call wrongly
under any release in 330 hand rows, SWEEP_ROWS sweep rows and the isolations
below; every miscompile and false report found is pre-existing with its
mechanism outside the slice's diff (P1–P7, P9), or known.

## S — should fix before the slice is called complete

### S1. The D287 record claims 8.4 spells an exponent as 8.5 does; it does not, and `--dialect tcl8.4` now folds `1e-5` where tclsh 8.4 prints `1e-05`

The record (lane doc, "The lane's own: a lattice double is spelled as Tcl
spells it") says of #2395: "the two spellings agree for such a value, and the
ones it changes — an integral double, an exponent — 8.4 spells as 8.5 does".
Measured (`progs/k01_float_spellings.tcl`, `progs/g01_dialects.tcl`,
`out/evidence.txt`), under `tcl opt --profile full --dialect tcl8.4`:

| procedure | tclsh 8.4.20 | tclsh 8.5.19–9.1.0 | folded under `--dialect tcl8.4` |
|---|---|---|---|
| `set x [expr {1e-5}]; return $x` | `1e-05` | `1e-5` | `return 1e-5` |
| `set x [expr {0.00001}]; return $x` | `1e-05` | `1e-5` | `return 1e-5` |
| `set x [expr {1.5e-7}]; return $x` | `1.5e-07` | `1.5e-7` | `1.5e-7` |
| `set x [expr {4.9e-324}]; return $x` | raises `floating-point value too large to represent` | `5e-324` | `set x 5e-324; return 5e-324` |
| `set x [expr {123456789.123456789}]` | `123456789.123` | `123456789.12345679` | the 8.5 spelling (#2395 proper) |

8.4's `Tcl_PrintDouble` is C's `%.12g`, whose exponent is at least two digits
(`1e-05`; a positive exponent below 17 prints fixed, so only the negative ones
show it), and 8.4 rejects a subnormal literal (ERANGE). The fix is right for
8.5–9.1 (every row of `k01` and `g01` is `same` under those dialects) and
strictly better than Rust's `Display` under 8.4 (`0.00001` before), but the
record's sentence is false and #2395's title ("`tcl_precision` 12 digits") does
not cover the two axes the fix leaves wrong. Correct the record and widen #2395
(or file P8 beside it), and have `format_double` take the release's exponent
width and subnormal policy from the profile when the lane models 8.4's doubles.
Code: `rust/tcl-syntax/src/number.rs` `format_double` ("the natural-width
exponent"), `rust/tcl-compiler/src/value_transfer.rs` `const_to_exact`.

## N — nits

- N1. `ConstantReturn::as_kind_text` (`interprocedural.rs:129`) still spells a
  `Bool` as `1`/`0` and a `Float` with Rust's `Display`, the two spellings D292
  retires in `text()`; its only consumer is `inlining_interproc_residual.rs`'s
  wire-form test. Make it `text()`'s kind-tagged twin or delete it, so the next
  consumer of the wire form does not reintroduce #2388.
- N2. `statement_may_return` decides by IR variant and answers `false` for a
  `Statement::Call` whose `Body`-role word runs in the procedure's frame. Every
  such command I tried is covered by something else — `dict for`, `dict map` and
  `lmap` are lowered into blocks, so their inner `return` is a terminator and
  `exit_value` reads it (`h1`, `h2`, `h51`: `pure: yes`, `return shape:
  unknown`, no fold); `time`, `dict with`/`update`, `namespace eval`, `interp
  eval`, `foreach ::x`, `eval $body`, `while 1 $body`, `uplevel 0 return 1` and
  `after 0 {return 1}` make the procedure a barrier or impure (`h3`, `h8`:
  `pure: no · flags: barrier`) — so the rule holds today by those facts rather
  than by its own statement. The registry's `ArgRole::Body` on a `Call` is the
  name-free check the doc comment implies ("the IR's statement variants decide,
  with no command spelling") and would make it hold by itself.
- N3. The O103 KCS note's new bullet names "an arm of `switch -glob` or `switch
  -regexp`" as kept whole; `switch -regexp -- abc {^a {return 1}}` with a
  literal subject is decided and its `return 1` folded (`h24`), so the bullet
  should say "with a subject the analyser cannot read" (the witness's `[clock
  seconds]` and `$x`).
- N4. The complexity-guarded path keeps the shapes' constant with neither
  D289's nor D293's check: `progs/i15_guarded_noexit.tcl` (a 300 KiB body with
  `while {1} {set x [expr {1/0}]}; return 5`) folds `catch {set y [p]} m` to
  `set y 5` where tclsh prints `divide by zero`, the explorer showing
  `foldable: yes · return shape: const(5)`; the opaque-switch case is safe
  there because the scanner records the arm's `return` (`i16`). The record says
  the guard-stopped procedure "answers from the shapes"; worth one sentence
  that this is #2390's no-exit case left open on that path, since the landing
  message counts the case as closed.
- N5. `interprocedural-analysis.md`'s signature block now shows both entries
  but the paragraph before it still says `build_interprocedural_analysis`
  "builds a `ProcSummary` for each procedure" and only Step 2b says the unit
  entry is the one the optimiser and `with_interprocedural` call; a sentence
  at the top would save the reader the reversal.
- N6. The explorer's `format_return_shape` spells a `Float` with Rust's `{:?}`
  (`1e301`, not `1e+301`) beside `const('…')` for text (`k02`: `const(True)`
  for the boolean); display only, but it is the one place a reviewer compares
  the summary with the fold.

## Mutation coverage

Each mutation was applied to the tree by `mut/run_mut.sh` (one exact
replacement, refused while any foreign `cargo test` ran or the tree was not
clean), the named witnesses and lib tests run (`cargo test -p tcl-compiler
--lib --test value_transfer_witnesses -- <names>`), and the file restored from
its saved copy, `cmp`-equal to `git show 3076a049a:<path>`; `git status` is
clean. Logs under `mut/M*.out`.

| Mutant | Killed by |
|---|---|
| M1 `seedless_returns` computes nothing (the stage's map empty) | `o103_summary_path_folds_a_computed_return` (`p` not foldable) and `a_run_that_proves_no_value_leaves_no_constant` (`const(5)` from the shapes); `the_summary_returns_the_value_exactly`, `a_padded_value_is_braced_where_a_rewrite_spells_it`, `the_summary_folds_only_a_call_the_rerun_could_make`, `a_return_inside_a_statement_kept_whole_stops_the_fold` and the three O103 lib tests survive, as they should |
| M2 `summary_answers` answers every call (D288 reverted) | `the_summary_folds_only_a_call_the_rerun_could_make` |
| M3 `statement_may_return` always false (D289 reverted) | `a_return_inside_a_statement_kept_whole_stops_the_fold` |
| M4 `exit_value` skips a reachable fall-through (`None => continue`) | `o103_folds_implicit_return_proc_cmd_subst` and `o103_does_not_fold_when_return_and_fallthrough_disagree` (lib; the witness binary did not run after the lib failed) |
| M5 a run's `NoValue` falls to the shapes (D293 reverted) | `a_run_that_proves_no_value_leaves_no_constant` |

## P — pre-existing defects found in passing (for GitHub issues, not fixes)

### P1. A word holding two command substitutions is evaluated as one `string length` over the text between them

```tcl
proc p {} { set x "[string length ab][string length cde]"; return $x }
puts [p]
```

tclsh 8.4–9.1: `23`. `tcl opt --profile full` (every dialect): `proc p {} { set
x "[string length ab][string length cde]"; return 4 }` and `puts 4`. `tcl
explore --show sccp` gives `x#1 = const(4)` with `route string: direct
string-length (registry) · answer: evaluated` and `routes entered: direct 3`:
the value is the length of the text from the first argument up to the next
`[` (`abc][`→5, `a][`→3, `ab]x[`→5, three substitutions →5, `[string length
ab][llength {c d}]`→4), so the first route reads an argument span that runs
into the second substitution. Only `string length` reads the word so
(`"[llength {a b}][llength {c}]"` and `"[string toupper ab][string toupper
cd]"` are overdefined); the bare form `set x [string length ab][string length
cde]` folds the same, and so does the top-level `set` with no procedure, so
the unit's own lattice is the mechanism, not the summaries.
`progs/a04_nested_subst.tcl` line 9 is how it was met,
`progs/p_quoted_two_substs.tcl` the minimal form; the route's argument reading
(`rust/tcl-compiler/src/value_transfer.rs`) predates the slice, whose only
change there is `const_to_exact`.

### P2. `array for` (9.0+) is lowered as a `foreach` over the array's name

```tcl
array set a {k v}
array for {k v} a { puts "$k=$v" }
```

tclsh 9.0.4 and 9.1.0: `k=v`. `tcl opt --profile full --dialect tcl9.0` (and
`tcl9.1`): `array for {k v} a { puts "a=" }` — `k` is bound to the word `a` and
`v` to the empty string, the lockstep over a one-element list. `progs/h04`
lines 1 and 3 are how it was met (`return $k` inside the body rewritten to
`return a`); `progs/i04_array_for.tcl` the minimal form. The `array for`
lowering / iteration plan (`cfg_builder`, the registry's iteration plans) is
slice 12's and untouched here.

### P3. O103 folds a builtin's call through a same-named `proc` defined later in the file, where the earlier call ran the builtin

```tcl
proc p {} { set x [string range foobar 0 2]; return $x }
puts [p]
proc string {args} { return SHADOW }
puts [p]
```

tclsh 8.4–9.1: `foo` then `SHADOW`. `tcl opt --profile full` (identical under
every dialect): `proc p {} { set x SHADOW; return $x }` — the inner `[string
range foobar 0 2]` is folded through `::string`'s summary (`const('SHADOW')`,
three literal words, `args` accepts them), so the program prints `SHADOW`
twice. The whole-module trust rightly stops the lattice from evaluating `string
range` as the builtin, but `try_o103_proc_fold` then folds the substitution
through the user procedure without regard to order; a procedure's own call
before its definition is not folded (`progs/p_call_before_def.tcl` keeps `puts
[p]`), so the builtin-to-procedure transition is the gap. `redefined_procedures`
gates only a procedure defined twice and `trusts_proc_binding` only
`rename`/`interp alias`. Not #2217 (a shadowed `expr` evaluated as the
builtin). Pre-existing on the summary path (the literal `return SHADOW` folded
before the slice); `progs/f03_shadow_after.tcl`, with `f02` (the shadow defined
first) printing `SHADOW` both ways.

### P4. O109 deletes a top-level store read by a procedure through `global` when the name is later `unset`

```tcl
proc b6 {} { global g; return $g }
set g 6
set r [b6]; puts "<$r>"
set g 123
set r [b6]; puts "<$r>"
unset g
```

tclsh 8.4–9.1: `<6>` `<123>`. `tcl opt --profile full` (every dialect) deletes
`set g 6` (`O109 Eliminate dead store`), and the program raises `can't read
"g": no such variable` (exit 1) under every release. Without the trailing
`unset g`, or with `puts [b6]` in place of `set r [b6]`, the store is kept
(`progs/i05`–`i07`, `i19`); `progs/i09_global_store_deleted.tcl` is the
delta-debugged minimum of `progs/b01_must_stay.tcl`, where every store of `g`
before the reads through `global g`, `$::g`, `[set ::g]`, `upvar #0 g v` and
`info exists ::g` was deleted. `b6`'s summary is the same before and after the
slice (no constant, no passthrough), so the decision is `elimination.rs`'s,
not in the diff; related to #2297 by mechanism (a read through the procedure
missed), but this one is a miscompile, and not #2350, #2308, #2231, #2261 or
#2370.

### P5. The expression and index routes accept 8.5+ grammar under `--dialect tcl8.4`

```tcl
proc p {} { set x [expr {10 ** 2}]; return $x }
set r [p]
puts "<$r>"
```

tclsh 8.4.20: `syntax error in expression "10 ** 2": unexpected operator *`
(exit 1); 8.5–9.1: `<100>`. `tcl opt --profile full --dialect tcl8.4`: `proc p
{} { set x 100; return 100 }` and `set r 100`, which prints `<100>` under 8.4.
The same for `expr {"a" in {a b}}` (8.4: `extra tokens at end of expression`;
folded to 1), `expr {2.0 ** 0.5}` (folded to `1.4142135623730951`) and `lindex
{a b c} 1+1` (8.4: `bad index "1+1"`; folded to `c`), while `max(1, 2)`,
`entier(1.5)` and `2 ** 64` are rightly left alone under 8.4
(`out/g01_dialects/`, `progs/p_84_pow.tcl`, `p_84_in.tcl`, `p_84_lindex.tcl`,
`out/evidence.txt`). The expression grammar per release
(`rust/tcl-syntax/src/expr`, `tcl_expr_eval.rs`'s `FoldPolicy`) and the
registry's index grammar predate the slice; #2333 and #2312 are the tclvm's
side of the same axis.

### P6. `tcl opt` and `tcl diag` panic on `abs(-9223372036854775808)` under every dialect but tcl8.4

```tcl
set x [expr {abs(-9223372036854775808)}]
puts $x
```

tclsh 8.5–9.1: `9223372036854775808`; 8.4: `integer value too large to
represent`. `tcl opt --profile full` and `tcl diag` (auto, `--dialect
tcl8.5`/`tcl8.6`/`tcl9.0`/`tcl9.1`) abort: `thread '<unnamed>' panicked at
rust/tcl-syntax/src/expr/mathfunc.rs:60:9: internal error: entered unreachable
code` (`<NoBig as BigIntOps>::from_i64` from `type_conv` under
`dispatch_with_backend_int_width::<NoBig>`), so a file holding the expression
gets no diagnostics and no rewrite at all (`progs/g01_dialects.tcl` produced an
empty `tcl opt` output under 8.5+; `progs/i21_abs_panic.tcl`). `mathfunc.rs` is
not in the diff.

### P7. W241 "provably infinite" on a loop whose body word is a variable, and on a `for` whose init returns

```tcl
proc p {} { set body {return 1}; while 1 $body; return none }
puts [p]
```

tclsh 8.4–9.1: `1`. `tcl diag` (every dialect): `2:40: warning W241 while is
provably infinite: condition is constant true and the body never leaves the
loop (no break/return/error/exit/throw/tailcall)`. Likewise `set body {break};
while 1 $body; puts after` at the top level (`progs/j05`, prints `after`), and
`proc p {} { for {return 1} {1} {} {}; return none }` (`progs/j02`, prints 1:
the init leaves the procedure before the body ever runs). A quoted body (`while
1 "break"`, `j04`) is read and silent. The constant-true rule's body scan
(`bounds_checks.rs`, slice 12) sees no script in a `$body` word and does not
read a `for`'s init; not #2306 and not the slice 12 review's B1 (the counter
rule).

### P8. #2395's family: 8.4's two-digit exponent and subnormal rejection

The rows are in S1; filed here so #2395 can be widened: under `--dialect
tcl8.4` a computed `1e-5` folds as `1e-5` (8.4 prints `1e-05`), `1.5e-7` as
`1.5e-7` (`1.5e-07`), and `4.9e-324` as `5e-324` where 8.4 raises
`floating-point value too large to represent`. Pre-existing (Rust's `Display`
spelled them `0.00001`, `0.00000015` and `5e-324` before D287); #2234 is the
large-magnitude neighbour.

### P9. O107 drops a word of `for {return 1} {1} {} {}`

```tcl
proc p {} { for {return 1} {1} {} {}; return none }
puts [p]
```

tclsh 8.4–9.1: `1`. `tcl opt --profile full` (every dialect): `proc p {} { for
{return 1} {1}  {}; return none }` and `puts 1` — O107 ("Eliminate unreachable
dead code", the body after the returning init) removes one of the two `{}`
words instead of emptying it, and the program raises `wrong # args: should be
"for start test next command"` under every release (`progs/h01` line 13,
`progs/p_for_init_return.tcl`). The O103 fold of `[p]` to 1 beside it is
right. #2307's O107 `for` rewrite class with a different symptom; the
rewrite's word rendering is not in the diff.

## Known, met again (no new issue)

- #2264 (a procedure's implicit result is not a use): every `_fall` program of
  the sweep and `progs/a05_fallthrough.tcl` — `proc p {} { set x [string range
  foobar 0 2] }` becomes `proc p {} {  }` (O126), so `[string length [p]]`
  prints 0 where tclsh prints 3; O103 itself folds `[p]` to `foo` first and
  rightly.
- #2306 (`upvar 0` alias): `proc up0b {} { set x 1; upvar 0 x y; set y 5;
  return $x }` folds `return $x` to `return 1`; tclsh 5 (`progs/j06`).
- #2390: `error boom; return 5` and `set x [expr {1/0}]; return 5` still fold
  (`progs/c04` e1, e5; both paths), and the no-exit case on the guarded path
  (N4); the lattice-decided no-exit case is closed as recorded (`c04`,
  `a_run_that_proves_no_value_leaves_no_constant`).
- #2395: the 8.4 twelve-digit rows of `g01`/`k01` (`0.333333333333`,
  `123456789.123`, `1.79769313486e+308`).
- The dialect default (the slice 11/12 reviews' P6 class): under auto, `010`
  reads as octal and 8.4's wrapping arithmetic is not modelled, as before;
  every per-release `--dialect` row of `g01` but P5's, P6's and #2395's is
  `same`.

## What the slice gets right (checked, for the record)

Reconstructed from the reviewer's hand-back summary (the original section was lost with the machine).

- R1: the only literal the slice adds to `interprocedural.rs`'s new code is the renderer's `"${"` bare-word test; the registry decides the cell-update target; no command is matched by name.
- R6: `o103_folds_implicit_return_proc_cmd_subst` and `o103_folds_arg_sensitive_passthrough_cmd_subst` are absent from the diff and byte-identical.
- R7: a return that reads a parameter shows `param constants: a = foobar` with `return shape: unknown` in the explorer, and M1 (the seedless stage computing nothing) is caught by the summary witnesses, so no seed-dependent value enters the summary.
- The seedless constant: a return computed through the string, list, expr, format, scan, regexp, incr, append and lappend commands and through nested and quoted substitutions folds in each of the eight return shapes of the generated sweep (504 programs, 2520 rows under the five releases) and prints what tclsh prints before and after `tcl opt`; every DIFF row is #2264 (the fall-through `set` O126 deletes), #2395 under the auto dialect, or the dialect default.
- D288's call test (hand batch e): a call whose words substitute, whose count the procedure rejects, or that reaches `args` or a default stays unfolded; a literal-word call of accepted count folds.
- D289's exit test (hand batch h, per release line): a `return` inside an opaque `switch` arm or default, an `if` clause, a loop body, a `try` body or handler, a `Block` or an `UpFrame` stops the fold; a `catch` body absorbs its `return` and does not.
- The exact rendering (hand batch d): 65 literal values in three shapes and 45 computed values round-trip, the whitespace-padded integer of #2392 braced, empty strings, braces, brackets, `$`, `;`, `#`, backslashes, newlines, quotes and unicode kept, doubles spelled as 8.5 to 9.1 spell them.
- Procedures calling procedures, recursion, mutual recursion, a procedure redefined or defined conditionally, `namespace eval`, qualified names, `rename` and `interp alias` (hand batch f): the caller stays unfolded rather than folding wrong, the one exception the pre-existing builtin shadowing of P3.
- The whole witness binary at HEAD passes (152), the slice's unit and residual tests pass, and the five mutations each die on the witness the record names.

## Verdict

Land as is, with S1 corrected before the slice is called complete: the record's
sentence that 8.4 spells an exponent as 8.5 does is false (8.4 prints `1e-05`
and rejects `4.9e-324`), and #2395 should be widened to those two axes (P8).
No B. Nothing the seedless summary, the exact rendering, the D288 call test,
the D289 exit test or the D293 override decided was wrong under any release in
330 hand rows, SWEEP_ROWS sweep rows and the isolations; every miscompile
found (P1–P5, P9), the panic (P6) and the false report (P7) are pre-existing
with their mechanisms outside the slice's diff, and the five mutations each die
on the witness the record names.