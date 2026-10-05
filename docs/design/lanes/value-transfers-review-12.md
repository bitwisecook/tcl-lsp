# Slice 12 (bounded-loop enumeration) — adversarial review

Tree: `/home/user/tcl-lsp`, branch `claude/spectcl-optimization-discussion-5qhf42`,
HEAD `5bf6eb1b` (the landing), slice commits `c1d24182`, `08d5acf3`, `fb2b3109`,
`c3af84d8`, `25ead4f1`, `5bf6eb1b`, base `ce22c374`; `git diff ce22c374..5bf6eb1b`,
51 files. Read: the lane doc's slice 12 plan and record (VT12.1–VT12.7, D268–D284),
the loop material of `value-transfers.md`, `sccp-core-analyses.md`,
`value-transfers-examples.md`, `value-transfers-migration.md`,
`precision-limitations.md`, the KCS notes for W240, W241, W242, IRULE5003, I230,
W230 and O103 (the last two carry no loop material at all, see N5); the diff in
full for `static_loops.rs` (read whole, it is a rewrite), `value_transfer.rs`,
`sccp.rs`, `bounds_checks.rs`, `tcl_expr_eval.rs`, `intervals.rs`, `ssa.rs`,
`analyser/{handlers,diagnostics/dataflow,diagnostics/helpers,irules_event_checks}.rs`,
`optimiser/propagation.rs`, `cfg.rs`, `cfg_builder/{cfg_lower,mod}.rs`, the
registry's `iteration.rs` / `for_.rs` / `while_.rs` / `answers.rs`, the witness
tests and the docs.

Measured with the CLI at `target/debug/tcl` (`tcl 0.1.0+g5bf6eb1b`, built with
`CARGO_BUILD_JOBS=2 cargo build -p tcl-cli`) against tclsh 8.4.20, 8.5.19,
8.6.18, 9.0.4 and 9.1.0 (`TCL_LIBRARY` set per release). Oracle per program:
stdout and exit status under each release before and after `tcl opt --profile
full`, once under the auto-detected dialect and once under the release's own
`--dialect`; `tcl diag` under auto and every `--dialect`, read against the
release's behaviour. Harness: `review-vt12/run.sh`; programs under
`review-vt12/progs/` (hand batches `a*` enumeration semantics, `b*` and `d/*`
W240–W242 and I230 shapes, `c*` state semantics, `f*` list/string/switch/catch
shapes, `e*` isolations, `gen/*` the generated sweep); every run's outputs under
`review-vt12/out/<name>/`, the per-release evidence for the cited programs in
`out/evidence.txt`, the W241/W240 probe table in `out/d/diag.txt`.

- Hand batch a (20 programs, full matrix, 200 rows): 42 DIFF rows — `a06`
  (the nested-`catch` loss, P4), `a09` under 9.x auto (P6, 2 rows), and
  `a14`, `a15`, `a20` (each a single-pass `for` rewritten to `{0}`, #2382).
- Hand batch b (4 programs + 1 iRule, 40 rows): 12 DIFF rows — `b02` under 9.x
  auto (the dialect-default policy, P6) and `b03` (the O112 brace drop, P1).
- Hand batch c (10 programs, 100 rows): 21 DIFF rows — `c01` under 8.4 auto and
  `c08` under 9.x auto (P6), `c03` (#2382), `c07` (the qualified `foreach`, P3).
- Hand batch f (10 programs, 100 rows): 10 DIFF rows — all `f09` (the nested
  call's parameter seed, P5).
- W241/W240 probes `d/d01`–`d79` (79 single-loop programs, `tcl diag` under
  tcl8.6 and tcl9.0): the B1 table below.
- Generated sweep `gen.py` (360 programs: one `for` with a literal start, one
  `while`, or one `for` seeded from `set start N`, over starts {0, 1, 5, −3},
  `<`/`<=`/`>`/`>=`/`!=`, bounds {0, 3, 5, 10}, steps {1, 2, −1, −2, 3}, and 14
  bodies — accumulators, `break`, `continue`, a counter write, `lappend`,
  `switch`, `catch`, an escaped literal, a quoted word with a tab — each with a
  post-loop test; auto dialect, five releases; stopped after 101 of the 360
  programs, about an hour of harness time): 505 rows, 15 DIFF rows, every one
  a single-pass `for` rewritten to `{0}` (#2382: `g012` `$i <= 5` from 5,
  `g036` `$i >= 5` from 5 step −2, `g072` `$i != 3` from 0 step 3); no other
  difference. The remaining 259 programs are under `progs/gen/` for the
  harness to finish (`MODE=quick ./run.sh progs/gen/g1*.tcl …`).

Every B/S/P item below was checked for an existing issue against the open list
(`review-vt12/open-issues.txt`, 167 open issues) and the recent closed ones:
#2306 (same-frame `upvar 0` alias) and #2305 (`::name` at the top level) cover
two of B1's rows and are named there; #2323 covers P4's class; #2382 and #2381
are the known ones; nothing covers P1, P2, P3, P5 or the rest of B1.

## B — blocking

### B1. W241 "provably infinite" on `while` loops (and solver-seeded `for` starts) that tclsh ends: the counter's write scan misses most ways a body writes the counter

VT12.5 makes a conditional loop's one top-level increment its step and seeds
the counter's start from the solver (`LoopTerminationCandidate::seed`,
`loop_start_integer`), so `while` loops are now checked by the counter rule.
The rule's "nothing else in the loop writes the variable" is
`body_increment` → `command_writes` → `writes_first_arg` / `writes_the_name` /
`script_words` (`rust/tcl-compiler/src/analyser/bounds_checks.rs`), a text walk
that descends into braced words and `Body`-role words only. Every program
below prints its number and exits 0 under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4
and 9.1.0, and `tcl diag` (auto, `--dialect tcl8.6`, `--dialect tcl9.0`)
reports `W241 while loop is provably infinite: counter $i starts at 5, moves
by -1 per step, and compares < 10 (never reached)` on line 2
(`review-vt12/out/d/diag.txt`):

| program (`progs/d/…`) | body after `incr i -1;` | tclsh | miss |
|---|---|---|---|
| d16 | `set j [incr i 20]` | 24 | a write inside a `[…]` word |
| d35 | `set x [set i 100]` | 100 | same |
| d36 | `puts [incr i 20]` | 24 | same |
| d37 | `set l [list [incr i 20]]` | 24 | same |
| d38 | `expr {[incr i 20]}` | 24 | same, inside a braced `expr` |
| d39 | `if {[incr i 20] > 0} {}` | 24 | a write in an `if` condition (braced, but segmented as a command whose head is `[incr`) |
| d58/d59/d60/d61/d62 | `set x [regexp {(\d+)} 100 -> i]`, `set x "[set i 100]"`, `set x a[set i 100]b`, `if {[set i 100] > 0} …`, `expr {[set i 100]}` | 100 | same family |
| d09 | `foreach i {100} {}` | 100 | a `foreach` var-list rebinding the counter (no `VarWrite` role) |
| d56 | `lmap i {100} {}` (8.6+) | 100 | same |
| d57 | `dict for {i v} {100 1} {}` (8.5+) | 100 | same |
| d04 | `q`, with `proc q {} { uplevel 1 {set i 100} }` in the file | 100 | a defined procedure's `uplevel` write — the solver itself knows it (`i#3 = const(100)` in `tcl explore --show sccp` of the `for` form d53) |
| d05 | the same inside a procedure `s` | 100 | same |
| d54 | `foo i`, with `proc foo {name} {upvar 1 $name v; set v 100}` | 100 | the `upvar` helper idiom |
| d40 | `foo i` undefined | error | the note's own "a call to a command the file does not define … may set" |
| d01/d02 | `foo` undefined, top level / in a procedure | error | same |
| d24 | `upvar 0 i j; set j 100` | 100 | #2306 |
| d23 | `set ::i 100` at the top level | 100 | #2305 |

The exact `tcl diag` line for d16 (`set i 5\nwhile {$i < 10} {incr i -1; set j
[incr i 20]}\nputs $i`):

```
d16_incr_in_subst.tcl:2:7: warning W241     while loop is provably infinite: counter $i starts at 5, moves by -1 per step, and compares < 10 (never reached)
```

The same misses on a `for` with a literal start (`for {set i 5} {$i < 10}
{incr i -1} {…}`) predate the slice (the deleted `for_is_provably_infinite`
used the same `body_writes_var` / `body_may_exit` walk), and those now draw
the wrong W240 of P2 instead; the `while` form and the `set i $start` form are
the slice's, which is the standard the lane applied to #2381 (D280: "the
landing extends the counter to a `while` loop's increment and would otherwise
widen a false report to a new loop shape"). Three of the seven silent shapes
`w240_seeds_from_the_iteration_plan` pins are writes the walk does see
(`set i 20` in a braced `if` body, a second `incr`, a `break`); none of the
rows above is covered. Fix in the lane: `command_writes` must take a `[…]`
word as a script (segment the substitution's commands as `any_command_recursive`
segments a body), take a var-list or binder word (`foreach`, `lmap`, `dict
for`, `dict update`, `dict with`, `regexp`/`scan` targets — the registry's
roles beyond `VarWrite`) as a write, and treat a call to a command the registry
does not resolve — a procedure the file defines or not — as a possible write of
the counter, which is the policy the W241 note states for the solver path and
which `script_writes` can read from the unit's lattice (`loop_start_integer`
already reads it for the start); or, simpler and sound, seed only a loop whose
body the enumeration ran and declined for the iteration cap alone (a run past
4096 passes with the counter as the only moving part is the proof the text
walk tries to make).

## S — should fix before the slice is called complete

### S1. A name bound before a loop, rebound inside it and dead after it drops every loop's state in the unit

`enumerate_loops` (`rust/tcl-compiler/src/sccp.rs`) keys each place the loop
wrote by `ssa.blocks[end].entry_versions[symbol]`. For a name the loop writes
that is not live after the loop the pruned SSA places no φ at the exit block
(`tcl explore --show ssa` of `progs/e06_inner_binder_bound.tcl`: `foreach_end_4`
has φs for `r` and `x`, none for `y`), so the entry version is the pre-loop
one, the published fact (`y = 'd'`, `tmp = 2`) is stated about a version whose
settled value is the pre-loop constant, the second run's `exit_state_contradicted`
(D270) fires, and `Solved::Contradicted` drops the state of **every** loop in
the unit:

```tcl
set tmp 1
set n 0
foreach x {1 2} { set tmp $x; incr n }
if {$n == 2} {puts yes} else {puts no}
```

tclsh 8.4–9.1: `yes`. `tcl opt --profile full` (every dialect) leaves the
program byte-identical, `tcl diag` reports nothing, and `tcl explore --show
sccp` prints no `enumerated loop` line; delete `set tmp 1`, or add `puts $tmp`
after the loop, and the branch decides (`if {1} {puts yes} else {}`, I230). The
same with a `while` (`progs/s1_dead_temp.tcl`, the `while` and `for`-after
variants in the transcript), and the idiom the slice is for:

```tcl
proc p {} {
    set found 0
    set last ""
    foreach x {a b c} { set last $x; if {$x eq "b"} { set found 1 } }
    if {$found} { return yes }
    return no
}
puts [p]
```

tclsh 8.4–9.1: `yes`; `tcl opt` keeps `if {$found} { return yes }` (O109
deletes `set last ""` and `set last $x`, so the dead store is seen as dead —
the φ is the only thing missing); without `set last ""` it folds to
`if {1} { return yes }`. A later unrelated loop pays too: append
`for {set i 0} {$i < 3} {incr i} {}; if {$i == 3} {puts three}` to the first
program and `$i == 3` stays undecided (0 enumerated loops), where the same two
lines alone decide. A `for` whose body writes such a name was decided in my
probes (its exit block carried the φ), so the `foreach`/`while` lowering's
exit block is where the φ is pruned. Fix: publish a place only for a version
the loop defines — a φ at `end`, or a definition the loop's blocks dominate —
and never the version live before the loop; and `exit_state_contradicted`
should drop the one loop whose fact contradicts, not all of them. Sound as it
is (nothing wrong is decided), but it silently switches the slice's feature
off for the most ordinary loop shape there is.

### S2. The W241 note promises what the counter path does not do

`docs/kcs/codes/kcs-diagnostic-w241-loop-provably-infinite.md`: "The proof is
about the value the condition's variable holds, so a write the analyser cannot
place leaves the loop undecided and draws no W241: … one a call to a command
the file does not define, a call whose command is computed and a call inside a
`catch` body may set … A procedure the file defines is read for what it
writes". True of the header-fact path, false of the counter path for every row
of B1 (d01, d02, d04, d05, d40, d54). Whichever way B1 is fixed, the note's
counter bullets ("If the body assigns the counter itself (`set v ...`, nested
`incr v`, `lset`, …) the analyser backs off") should say what the scan reads
— and the `## Why` paragraph should not claim a general policy the counter
path does not apply.

### S3. The I230 note's "A test after a loop" is stale against D283

`docs/kcs/codes/kcs-diagnostic-i230-constant-existence-check.md` (added by the
slice): "A loop is not run … when it … runs a command the analyser does not
evaluate (`puts`, a procedure, a command substitution other than `set v [expr
…]`)". Since D283 any `[…]` word whose one command has a registry route runs
over the state: `incr n [string length $s]`, `incr x [expr {$b / $a}]`,
`foreach x [lrange $l 1 end]`, `for {…} {$i < [llength $l]} …` all decide
(`progs/a04`, `f01`); `[string toupper $x]` does not (no direct route). Say
"a command substitution the registry evaluates" and name the effect-free
rule (a script that stores declines).

## N — nits

- N1. `docs/design/compiler/value-transfers.md` line 2923 (the file index the
  slice edited to add `enumerate_loops`, `start_state`, `catch_body_answer`)
  still lists `env_from_uses` and `existence_constant_branches`, deleted in
  slices 11 and 8 (the slice 11 review's N4/S4).
- N2. `irules_event_checks.rs`: the IRULE5003 check now returns silently when
  `self.registry` is `None`; the substring scan it replaced ran without one.
  `bounds_checks.rs` falls back to `default_registry()` for the same case;
  do the same here, or document that an analyse without a registry has no
  IRULE5003.
- N3. A loop test decided false still carries the generic I230 wording
  ("Branch condition '$i < 010' is constant; one branch is unreachable",
  `progs/b02` line 5, beside W240). D274 now knows the branch is a loop's test,
  so it could say so ("loop condition is never true"); pre-existing wording.
- N4. `summarise_for_statement` gives a statement built without words four
  empty words so the counted plan resolves (`vec![String::new(); 4]`); the
  registry cannot tell that sentinel from four genuinely empty words. The
  `summarise_*` boundary is accepted (D276); a comment on the plan's side that
  the words are never read for their text would keep it honest.
- N5. The W230 and O103 KCS notes carry none of the slice's claims (W230 bounds
  a counter after its loop, D278; O103's re-run runs the callee's loop and reads
  the return at its block, D279), though `value-transfers.md` and the ledger
  state both; the task's reading list expected them there.
- N6. The `foreach` binder write (`run_loop`, `static_loops.rs`) goes through
  `write` directly and never through `apply`'s `PlaceKind::Scalar` check, so a
  binder spelled `a(k)` with an exact element in the state is written as an
  element (`progs/a18`: `set a(k) 0; foreach a(k) {1 2} {}` decides
  `$a(k) == 2`, which is what Tcl does) while D269 says "a store to an element
  declines". Right by accident; make the rule one rule.
- N7. `the_eleven_loop_witnesses` and its CLI twin are the slice's behavioural
  gate, but no witness observes a loop that leaves by `return` or `error`
  outside a `catch` with a post-loop read (R7's "an error path publishes its
  prefix only"); see the mutation table (M4) — such a loop's normal exit is
  dead when the enumeration is exact, so the rule cannot be witnessed
  behaviourally, which is worth a sentence in the record.

## Mutation coverage

Each mutation was applied to the tree, the targeted witnesses run
(`cargo test -p tcl-compiler --test value_transfer_witnesses -- <names>`),
and the file restored from a copy before the next; `git status` is clean.

| Mutant | Killed by |
|---|---|
| M1 `Enumerator::source_word` hands every word on as written (D281 reverted) | `a_loop_word_is_its_value_with_its_escapes_decoded` (8.4 row `[false]` vs `[true]`); `the_eleven_loop_witnesses` and `the_correlated_pairs_decide_by_enumeration` survive |
| M2 `refined_value` keeps a constant a loop's exit state rules out (D270 reverted) | `the_state_a_loop_leaves_decides_the_branch_after_it` (`[true]` vs `[true, true]`); the eleven and `a_may_written_element_reads_the_store_to_its_array` survive |
| M3 `Enumerator::exec_script` runs past a non-normal completion (the error-path prefix) | `the_eleven_loop_witnesses` (line 7854, the eighth program); `the_prefix_rule_holds_in_both_builds`, `an_enumeration_runs_only_over_places_it_proves`, `a_may_written_element_reads_the_store_to_its_array` survive |
| M4 `enumerate_loops` publishes on a `NonNormalCompletion` (reasoned) | nothing can: a deterministic loop that leaves by `return`/`error` never takes its normal exit, so the stated fact narrows a dead block |
| M5 `write` without the `state.get(name).is_none()` check (reasoned; the record's own) | `an_enumeration_runs_only_over_places_it_proves`, `the_prefix_rule_holds_in_both_builds` |
| M6 `refine_interval` ignores the loop point (reasoned; the record's own) | `an_enumerated_loop_bounds_its_counter_after_it` |
| M7 `loop_counter` without `body_may_exit` (reasoned) | `w240_seeds_from_the_iteration_plan` (the bare `if {$i < 0} break` row), `a_bare_or_quoted_body_word_leaves_the_loop` |

The whole witness binary at HEAD: `cargo test -p tcl-compiler --test
value_transfer_witnesses` — 143 passed, 0 failed (484.6 s). The slice's unit
tests, after the tree was restored: `cargo test -p tcl-compiler --lib --
bounds_checks static_loops irules_event_checks tcl_expr_eval::tests::evaluate_expr
sccp::tests` 261 passed, 0 failed; `cargo test -p tcl-registry --test
value_transfers -- the_loop_plans_name_their_bound_and_step
several_lists_step_in_lockstep route_stamps_match_the_pinned_set
shipped_builtins_stay_on_the_direct_route` 4 passed; `cargo test -p
tcl-explorer sccp_text_prints_each_enumerated_loop` 1 passed. The four test
binaries (96, 80, 78 and 52 MB) were deleted afterwards; `git status` is clean.

## P — pre-existing defects found in passing (for GitHub issues, not fixes)

### P1. O112 drops a closing brace when the folded `if`'s body ends with a braced word

```tcl
set c 1
if {$c} {for {set j 0} {$j < 3} {incr j} {}}
puts $j
```

tclsh 8.4–9.1: `3`. `tcl opt --profile full` (auto and every `--dialect`):

```
set c 1
for {set j 0} {$j < 3} {incr j} {
puts $j
```

which raises `missing close-brace` (exit 1) under every release. The same with
`if {$c} {set j {}}`, `{set j {a}}`, `{set j { }}`, `{foreach j {1 2} {}}`,
inside a `while`/`for`/`foreach` body or in a procedure (`progs/b03` line 18 is
how it was met; `progs/p1_o112_brace.tcl` is the minimal form); a body ending
in `[list]` or a bare word is rewritten right. O112 "Eliminate constant if"
(`rust/tcl-compiler/src/optimiser/branch_folding.rs`), whose only slice change
is the `loop_enumerations: Vec::new()` test fixtures; the fold of `$c` is a
plain constant branch the lattice decided before the slice.

### P2. W240 "body never executes" for a rotated `for` that runs exactly once

```tcl
for {set i 0} {$i < 3} {incr i} {set i 5}
puts $i
```

tclsh 8.4–9.1: `6`. `tcl diag` (every dialect): `1:15: warning W240 for
condition is constant false; body never executes.` and no I230. The lowering
rotates a `for` whose condition is true on entry (`cfg_lower.rs`, `rotate`:
the header becomes a synthetic always-true guard with `span: None` and the
step re-tests the condition), and `header_fact`
(`analyser/diagnostics/dataflow.rs`) finds the loop's branch fact by condition
span, so the step's test — decided false because the body leaves `i` at 5 —
is read as the entry test. Any body that the lattice can see sets the counter
past its bound draws it: `set i 100`, `incr i 50`, `set j [incr i 5]`,
`foreach i {100} {}`, a call to `proc q {} { uplevel 1 {set i 100} }`
(`progs/d/d51`, `d52`, `d53`, `d74`, `d75`, `d78`, `d79`; tclsh prints 24,
100, 99, 99, 54, 6, 6). The rotation and `header_fact` predate the slice;
`header_fact` should read the entry test (the header's fact) and treat a
decided-false latch as "runs once", not "never".

### P3. A `foreach` with a `::`-qualified binder is an opaque call whose body writes are invisible

```tcl
set i 0
foreach ::x {1 2} { incr i $::x }
puts $i
```

tclsh 8.4–9.1: `3`. `tcl opt --profile full` (every dialect):
`; foreach ::x {1 2} { incr i $::x }; puts 0` — O109 deletes `set i 0` and
O100 inlines `puts 0`. `lower_foreach_dispatch`
(`rust/tcl-compiler/src/cfg_builder/mod.rs`, `has_qualified_vars`) keeps the
statement as a `Statement::Call` whose defs do not include the body's writes
(`tcl explore --show cfg`: one block, `call foreach ::x 1 2 incr i $::x`);
from commits a1e32956/49a38574/fbbc8b94, untouched by the slice. Not #2305
(which is `::name` versus `name`); not #2262.

### P4. A `[catch {…}]` body nested in a substitution records only its top-level writes (#2323's class)

```tcl
set i 0
set rc [catch {if 1 {set i 7}} msg]
puts $i
```

tclsh 8.4–9.1: `7`. `tcl opt --profile full`: `; set rc [catch {if 1 {set i 7}}
msg]; puts 0` (every dialect). A plain `set i 5` in the body is recorded
(`set rc [catch {set i 5} msg]; puts $i` → `puts 5`), but a write inside
`if`, `for` (`{for {set i 7} {0} {} {}}`), `foreach i {7} {}`, `while {$i < 7}
{incr i}` or `eval {set i 7}` is not, so `i` keeps `const(0)` (`tcl explore
--show sccp`: no `i#2`; `route catch: declined: unsupported`, so the slice's
`catch_body_answer` is not involved). `progs/a06` line 23 is how it was met:
`set i 0; set rc [catch { for {set i 0} {$i < 5} {incr i} { if {$i == 2} {
error x } } } msg]; if {$i == 2 && $rc == 1 && $msg eq "x"} {puts yes} else
{puts no}` prints `yes` under every release and `no` after `tcl opt` (I230
"always false"). #2323 names the class ("Writes inside a `catch` body nested
in a command substitution are invisible to the value lattice"); the remaining
gap is the nested control structure, which `precision-limitations.md`'s
"recorded where its text is known" does not mention.

### P5. A nested call site seeds the callee's own unit with the argument's braced spelling

```tcl
proc p {l} { set t 0; foreach x $l { incr t $x }; return $t }
puts [p {1 2 3}]
```

tclsh 8.4–9.1: `6`. `tcl opt --profile full` (every dialect) rewrites the
body to `foreach x "{1 2 3}" { incr t $x }`, and the program raises `expected
integer but got "1 2 3"` (exit 1) under every release. `tcl explore --show
sccp`: `::p`'s `l#0 = const('{1 2 3}')` — the braced word's text, braces
included — where the plain call `p {1 2 3}` seeds `l#0 = const('1 2 3')` and
rewrites to `foreach x "1 2 3"`. Also `proc p {l} { return $l }; puts [p {1 2
3}]` → `return {{1 2 3}}`, and `proc p {l} { while {[llength $l]} { set l
[lrange $l 1 end] }; return $l }; puts [p {1 2 3}]` → `set l {}` in the body
and `return {{1 2 3}}` (tclsh prints an empty line, the optimised program
`{1 2 3}`). The caller-uniform-literal seeds (`compilation_unit.rs`,
`encode_param_constants`) are not in the slice's diff; `progs/f09` is how it
was met, `progs/e07`/`e08`/`e09`/`e10` the isolations.

### P6. The auto-detected dialect reads release-dependent loop semantics under the 8.6 grammar

`progs/c01` (`catch {for {set i 0} {$i < 3} {incr i} { incr n }}; if {[info
exists n]} …`): tclsh 8.4 raises inside `incr n` and prints `unbound`, 8.5+
print `n=3`; `tcl opt` without `--dialect` folds to `puts "n=$n"`, which
raises under 8.4. `progs/c08` (`set x 010; … incr x …`) folds `ten` where 9.x
print `twelve`; `progs/b02` (`set i 9; while {$i < 010} {incr i -1}`) is
removed as dead where 9.x loop. Under the release's own `--dialect` each is
right (the per-release rows are all `same`, and the enumeration declines `incr
n` on an unbound name under tcl8.4, reads `010` as octal under 8.6 and decimal
under 9.0). The slice 11 review's P4 class; the dialect-default policy, not the
slice's classification.

### P7. W230 does not read a `lindex` nested in a quoted word, and `lrange` past the end draws nothing

`set l {a b c}; for {set i 0} {$i < 5} {incr i} {}; puts "<[lindex $l $i]>"`
draws nothing where `set r [lindex $l $i]` and a bare `lindex $l $i` draw W230
(`progs/e03_w230.tcl` lines 3, 5, 6); `set r [lrange $l $i end]` with `i` 5
draws nothing. Scope of the checker, not of the slice's interval point, which
is right in every form it reaches (5, 3, −2 after `incr i -4`, 3 after a
`foreach`, in a procedure).

## What the slice gets right (checked, for the record)

- The eleven loop programs (`progs/a01`): every post-loop branch decides for
  I230 and `tcl opt` under every dialect, and each optimised program prints
  what tclsh 8.4–9.1 print; the Explorer prints `enumerated loop: 5
  iterations, false condition · exit block: for_end_5 · i: const(5)` for the
  plan's program.
- Escapes and substitutions in loop words (`a02`, `a03`, `f07`): `a\x41`,
  `"a\x41 b"`, `{a\x41}`, `\$x`, `"\[$x\]"`, `"$r\\$x"`, `é`, a
  backslash-newline, `\{`, `"a b"` as one element, `$x;$x`, `$x#$x`,
  `"$r{$x}"`, `$x\n`, `"$x\""`; list words `{a\x41 {b\x41} "c\x41" d\ e}`,
  `"a\tb c"`, `{a;b c}`, `[list 1 2 3]`, `[split "a b"]`, `[lrange $l 1
  end]`, `[lsort -integer …]`, `{*}$l` (8.5+) — all byte-identical before and
  after `tcl opt` under every release, the decided ones decided right.
- `[…]` words in bodies (`a04`, `f01`): `incr x [expr {$b / $a}]` (20 and 25),
  `[string length $s]`, `[set k]`, `[llength {a b}]`, a condition reading
  `[llength $l]` while the body grows `l` (`i` 4, `l` 4 elements); `[incr k]`
  as an amount declines (stateful nested) and the program still prints right.
- `switch` in loops (`a05`, `f04`): `--`, `-glob`, `-nocase`, `-exact`, a
  fall-through arm, `-regexp`, a `break`/`continue` inside an arm inside a
  nested loop — every branch decided right.
- `catch` in and around loops (`a06` but line 23, `a17`, `c04`, `f08`): a
  `catch {break}` that absorbs the `break` (the loop runs on), `catch
  {continue}`, an error part-way through a `for`/`foreach` inside an opaque
  `catch` (the prefix state: `i` 2, `n` 2), `unset` then `set` in the body, a
  `return` inside a `catch` in a procedure, `return -code break`/`continue`
  from a called procedure (declined, program right).
- Bodies written bare and quoted (`a07`): `if {$i < 0} break`, `"break"`,
  `"set i 20"`, `"incr i 20"`, `continue else "incr t"` — no W241, every
  branch right.
- Nested loops (`a08`, `a16`, `e04`): an inner loop's exit state decides an
  `if` inside the outer body (`$j == 2` always true), an inner loop writing
  the outer counter is modelled right (`set i 100` in the inner body), 60×60
  passes under the cap, 4096 decided and 4097 not.
- Loops in sequence (`a13`): a loop whose start reads an earlier loop's exit
  is not enumerated (D268) and the program stays right; two `while` loops in
  a row decide `$i == 10`.
- Numerals and dialects (`a09`, `a15`, `c08`, `b02`): `010` is octal under
  8.4–8.6 and decimal under 9.x in a condition, an `incr` amount and a start
  value; `0x10`, `" 2"`, `-0`, `+3`, `1.5` (an error), `1_0`, `0b11`, `0o17`,
  `08`, `0d19`, `1e2`, `true` all decline or decide as the release does, and
  `set i 9; while {$i < 010} {incr i -1}` is W240 under 8.6 and W241 under 9.0
  — both right.
- Big integers (`a20`): `incr` past 2^63, `$x * 2` seventy times, `<< 1`
  and `**` are left alone where 8.4 wraps and 8.5+ go bignum (the one DIFF
  in `a20` is a single-pass `for` with `$x * $x`, #2382); the floats of `a14`
  (`+ 0.1` ten times, `/ 2.0`, `1 2.0 3e0`) likewise, its two DIFF lines
  being #2382 as well.
- The state is closed (`a18`, `c06`, `c07`, `c10`): a traced `n`, `::n`, a
  `global` in a procedure, an `upvar` write by a callee, `clock seconds`,
  `rand()`, `pid`, `puts` in the body, `namespace` variables — declined or
  modelled right, every row `same`.
- `foreach x {1 2 3} {unset x}` leaves `x` unbound, `{set x 10}` leaves 10,
  `{incr x}` 4; `foreach x {} {}` leaves `x` unbound (I230 on `[info exists
  x]`); lockstep lists pad with the empty string (`c01`, `a12`).
- W230 after `for`/`while`/`foreach` and in a procedure (`e03`): 5 past the
  end, 3 past the end, "negative" after `incr i -4`.
- O103 (`a10`, `f09`): `[f 3]` folds to 3 through the callee's `for`, `[h 5]`
  to 6 through a `while` with step 2, `[fact 5]`/`[fact 20]` to their values
  and `[fact 21]` left alone (bignum), a callee that `return`s from inside its
  loop is not folded.
- I230 (`b03`, `a19`): `while 1`, `while {1}`, `for {} {1} {}` decided true are
  silent; an `if {1}` inside a `foreach` body is reported (D274); `while {0}`
  draws W240 and I230; `[incr i] < 3`, `$i < 2.5`, `$s ne "aaa"`, `!($i >=
  3)`, `$go`, `"$i" < 3`, `$i in {0 1 2}` conditions all decide the branch
  after the loop right.
- IRULE5003 (`b04_irules.irul` under `f5-irules`): `incr count -2`, `::incr`
  in a nested `if`, `0 != $c`, `$c ne 0`, `${c}`, `"-1"`, `{-1}`, inside a
  `catch` all reported; `log local0. "incr c3 -1"`, `set c4 [expr {$c4 - 1}]`
  and `incr c5 -$step` silent.
- W241/W240 seeds that hold (`d/d03`, `d71`–`d73`, `b05`): a defined
  procedure that does not write the counter, an empty body, `puts x`, `set j
  1` beside `incr i -1` — truly infinite and reported; `set j 0; while {$j <
  10} { puts $j }` W241 through the header fact; a counter reset in a braced
  `if` body, a second increment, `lassign`, `regexp`/`scan` targets, `eval
  {set i 100}`, `eval "set i 100"`, `uplevel 0`, `namespace eval`, `apply`, an
  `after` script, a trace, `source`, `exit`/`error`/`throw`/`return -code
  break`, `unset`/`array set` — all silent.
- Every witness in the slice's record reruns green (143), and the three
  mutations the record names die on the witnesses it names.

## Verdict

Rework. B1 is a false W241 the slice makes reachable on every `while` loop
and seeded `for` whose body writes the counter in a `[…]` word, a `foreach`
var-list or through a procedure — the shapes the task named to hunt — and the
lane's own ruling on #2381 is the precedent for fixing it here. S1 silently
disables the enumeration for the most common loop idiom (a temporary
initialised before the loop and dead after it) and takes every other loop in
the unit with it. S2 and S3 are note edits. Nothing the enumeration decides
was wrong under any release in 945 harness rows (440 hand-written, 505
generated) and 79 single-loop diagnostic probes; every miscompile found is
pre-existing (P1–P5) with its mechanism outside the slice's diff, and every
other DIFF row is #2382 or the dialect default (P6).
