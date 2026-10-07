# Slice 13 (proc-level transfer summaries) — adversarial review

Tree: `/home/user/tcl-lsp`, branch `claude/spectcl-optimization-discussion-5qhf42`,
HEAD at review time `163f3712d` (the hand-off page after the landing, docs only:
`docs/design/lanes/handoff.md` +8/−6), the landing `bfb4e083b`, slice commits
`5eac6d247`, `3060659ba`, `bfb4e083b`, base `e355162ca`;
`git diff e355162ca..bfb4e083b`, 64 files, +6291/−968. Read: the lane doc's
slice 13 plan, record and status (VT13.1–VT13.8, D296–D307), the slice 7a and
slice 12 reviews, `value-transfers.md` § *Proc-level transfer summaries*,
`interprocedural-analysis.md`, `interprocedural-call-site-seeding.md`,
`value-evaluation.md`, `pass-fact-ownership-matrix.md`,
`downstream-pass-contracts.md`, `optimisation-passes.md`,
`value-transfers-migration.md`, `precision-limitations.md`,
`docs/generated/value-transfers.md`, the KCS notes for O103, O109, W210 and
W211 (none of the four is in the diff); the diff in full for
`interprocedural/transfer.rs` (read whole, 2070 lines), `interprocedural.rs`,
`value_transfer.rs`, `sccp.rs`, `compilation_unit.rs`, `cfg_builder/mod.rs`,
`ir.rs`, `ssa.rs`, `optimiser/{propagation,elimination,branch_folding}.rs`,
`analyser/diagnostics/{dataflow,helpers}.rs`, `analyser/param_traits.rs`,
`var_escape/slot_resolution.rs`, `signature_scan/arity.rs`,
`command_binding.rs`, `tcl-lsp-db/src/lib.rs`, the registry's
`scope_alias.rs`, `frame_effect.rs`, `registry.rs`, `answers.rs`, `inputs.rs`,
`route.rs` and the five specs, the explorer, xtask, the tests and the docs.

Measured with the CLI built from the tree (`tcl 2.2.5-731+g163f3712`,
`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 cargo build -p tcl-cli`, copied to
`review-vt13/bin/tcl`) against tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0
(`tmp/tcl<ver>/unix/tclsh`, `TCL_LIBRARY` per release). Oracle per program:
stdout and exit status under each release before and after `tcl opt --profile
full`, once under the auto-detected dialect and once under the release's own
`--dialect`; `tcl diag --dialect …` read against the release's behaviour. To
settle pre-existence, the base `e355162ca` was also built
(`tcl 2.2.5-724+ge355162c`, `review-vt13/bin/tcl-base`) from a scratch
worktree under the scratchpad (`git worktree add --detach … e355162ca`, built
into the existing `target/`, removed afterwards); every B/P row below carries
its base-binary verdict (`out/basecmp/`). Harness: `review-vt13/harness/run.sh`
(full oracle), `sweep.sh` (chunked `tcl opt`, one release per dialect),
`basecmp.sh`, `mut.sh`; programs under `review-vt13/programs/`: `a` the seven
witnesses and call shapes (30), `b` purity, completion, rebinding, recursion
and `info default` (44), `c` the word-effects pairing and the #2050/#2141/#2132
programs (25), `d` consumer diagnostics (15), `g` callee bodies that write the
link through `eval`, `uplevel`, binders and loops (30), `h` condition shapes
(13), `i` namespaces, `rename`, `interp alias`, `args` links, loops (20), `j`
qualified and aliased `Name` arguments (10), `k` kept-whole `switch`/`catch`/
`try` headers (14), `w` W210 after a callee's unbind (8), `r` an iRule with
`sharedvar` (2), `gen/` the generated sweep (1470); every run's outputs under
`review-vt13/out/`.

- Hand batches a, b, c, d, g, h, i (full matrix, 1630 rows): DIFF rows on
  a11, a13, a22, b01, b02, b22, b26, c14, c22, d08, d11, h01, h04, h09, h10,
  h11 and i06 (159 rows) — each classified below; nothing else.
- Generated sweep (`gen.py`: 21 callee bodies over `upvar 1 $name v` — `incr`,
  `incr $by`, `set`, `unset`, `append`, `lappend`, a conditional `set`, an
  `ensure`, `[expr {$v * 2}]`, a read-only `return $v`, `catch {incr v}`,
  `incr v; error`, `set v 1; return -code return`, `unset -nocomplain`, `set v
  $name`, `array set`, an element write, a global `incr` beside the link, a
  global read, `unset; set`, a call to a second link — × 5 place states —
  unset, `1`, `foo`, an array, a list — × 7 call shapes — a statement, `set r
  [f n]`, an `expr` operand, an `if` condition, a `catch` body, a `puts` word,
  a `foreach` body — × top level and procedure; each followed by an `info
  exists` read and a `puts` of the place; 1470 programs under tcl8.4, tcl8.6
  and tcl9.0 against their own tclsh, 4410 rows): 227 DIFF rows, every one the
  `if {[f n] ne {}} {set r 1}` shape with a store before it (P2), under all
  three dialects alike; 0 DIFF rows in the other six shapes for every body and
  state.
- Every B/S/P item was checked against the issue list the task names (titles
  fetched by number, `review-vt13/known-issues.txt`): #2390 is the base of B1,
  #2262 covers a13, #2327's class covers c14/c22 (see P8); nothing covers
  P2–P7.

## B — blocking

### B1. O103 folds a caller to its constant return through a callee whose re-run never completes normally under the call's seeds — or whose parameters reject the call's word count

The slice makes a caller pure when its calls name only its own plain locals
(D305, `fixpoint_pure` / `names_own_places`, `interprocedural.rs`), so `proc c
{} {set n foo; bump n; return 0}` is now a fold candidate on O103's summary
path, and `proc c {x} {set n $x; bump n; return 0}` on the argument-sensitive
one. Both paths read the callee's exit value through the caller's lattice, in
which the driver answers the call statement (`procedure_answer`,
`value_transfer.rs`) with `waiting(defs, Overdefined)` when the re-run reports
`completes: false`, and with the generic answer when `call_transfer`
(`transfer.rs`) returns `None` for a word count the callee's arity rejects.
Neither answer ends the run — the block runs on to `return 0` — so
`exit_value` reads a constant where tclsh raises inside the callee. The
record's R7 ("a callee that does not complete normally under the seeds keep[s]
the generic answer") covers the call's places only; the caller's exit was
never guarded. Base binary: `puts [c]` left as written in every program below.

| program (`progs/b/…`) | tclsh 8.4.20 … 9.1.0 | `tcl opt --profile full --dialect tcl8.4` and `tcl8.6` |
|---|---|---|
| b01 `proc bump {name {by 1}} {upvar 1 $name v; incr v $by}; proc c {} {set n foo; bump n; return 0}; puts [c]` | `expected integer but got "foo"`, exit 1 | `puts 0` (O103 summary path) |
| b02 `proc c {name} {bump name; return 0}; puts [c foo]` | same error | `puts 0` |
| b36 `proc c {} {set n foo; set r [bump n]; return 0}; puts [c]` (the embedded form) | same error | `puts 0` |
| b35 `proc half {name} {upvar 1 $name v; set v [expr {$v / 0}]}; proc c {} {set n 1; half n; return 0}; puts [c]` | `divide by zero` | `puts 0` |
| b39 `proc c {x} {set n $x; bump n; return 0}; puts [c 1]; puts [c foo]` | `0` then the error, exit 1 | `puts 0` twice (argument-sensitive path) |
| b32 / d11 `proc c {} {bump n; return 0}; puts [c]` | 8.4: `can't read "v": no such variable`; 8.5+: `0` | `--dialect tcl8.4`: `puts 0` — a release-specific miscompile; 8.5+ right |
| b37 `proc c {} {set n 1; bump n 1 extra; return 0}; puts [c]` | `wrong # args: should be "bump name ?by?"` | `puts 0` (`call_transfer` → `None`, the generic answer, the caller still pure) |

The exact output for b01:

```
$ tcl opt --profile full --dialect tcl8.6 b01_callee_raises.tcl
proc bump {name {by 1}} {upvar 1 n v; incr v $by}
proc c {} {set n foo; bump n; return 0}
puts 0
# O100  Propagate constant into command argument
# O103  Fold pure-proc call to '::c' to its constant return
```

The control b31 `proc c {} {set n foo; incr n; return 0}; puts [c]` — no
procedure call — folds to `puts 0` under base and HEAD alike: the exit reading
ignoring a certain raise is #2390 (open), and the slice widens it to every
caller of a `Name`-linking callee, which D305 newly makes foldable, with the
callee's own raise, its arity error and the 8.4 `incr` of an absent place as
the new routes to it. Fix in the lane: a callee whose re-run does not complete
normally, and a call its callee's arity rejects, must leave the call statement
as a raise — the solver has `DefValues::Raised` and `keep_prior` for exactly
that — so the exit reading sees no executable return; or, until #2390 is
fixed, D305's purity must not extend to a caller whose re-run of the callee
did not complete. Mutation M6 (below) shows which tests cover the word-count
route.

### B2. A call that omits a `Name` parameter whose default names a caller place is a barrier that widens nothing, so the place keeps a stale value the next read or re-run folds

`proc bumpd {{name n}} {upvar 1 $name v; incr v}`: a call with no argument
links `v` to the caller's `n` through the parameter's default. `call_transfer`
(`transfer.rs`) returns `None` for an omitted `Name` parameter ("a word count
the callee's parameters reject" or a `Name` role past the words), which
`procedure_answer` turns into the generic answer — and the generic answer
widens only the call statement's own definitions, of which the CFG builder
recorded none (it maps arguments to `param_targets` and knows no default:
the pre-existing half, P9). So the call is no barrier at all: the caller's
lattice keeps `n`'s prior value across it, and the slice then reads it — a
later `puts $n` is inlined from it, and a later `bumpd n` re-runs the callee
seeded from it.

| program (`progs/i/…`) | tclsh 8.4.20 … 9.1.0 | base `tcl opt` | HEAD `tcl opt --profile full --dialect tcl8.6` (and 8.4) |
|---|---|---|---|
| i21 `set n 1; bumpd n; bumpd; puts $n` | `3` | `puts $n` | `puts 2` (`# O100 Inline the constant value of 'n'`) |
| i22 `set n 1; bumpd; bumpd n; puts $n` | `3` | `puts $n` | `puts 2` — the re-run of `bumpd n` seeded from the stale 1 |
| i06 `set n 1; bumpd; puts $n; bumpd n; puts $n; proc p {} {set n 1; bumpd; return $n}; puts [p]` | `2 3 2` | `1 3 1` (P9) | `1 2 1` — the middle line newly wrong |

`parameter_values` already computes the default an omitted argument binds,
so `call_transfer` can name the default's place as the call's `Name` place
and the CFG builder's `upvar_invalidated` give it a definition; until both
hold, an omitted `Name` argument must widen the caller's frame as a call to
unseen code does.

## S — should fix before the slice is called complete

### S1. `TransferSummary.globals` omits the outer places a callee writes through a qualified or aliased `Name` argument, so the summary states a false fact and `keeps_to_its_frame` answers true

`tcl explore --show interproc` on

```tcl
proc bump {name {by 1}} {upvar 1 $name v; incr v $by}
set g 1
proc c {} {bump ::g; return $::g}
proc d {} {upvar #0 g x; bump x}
proc e {} {global g; bump g}
puts [c]
```

prints `transfer: completes: ok or error` for `::c`, `::d` and `::e` — no
`globals` — though each increments `::g` (tclsh 8.4–9.1 print `2`).
`component_outer_writes` (`transfer.rs`) reads `GlobalWriteInfo.names`, which
never records a write made through a callee's link (the same gap that P3
rests on), and the links' places are applied only to the caller's own frame.
Consumers checked: `outer_steps` reaches only definitions the caller's SSA
already lacks for the same reason; `detached_procedure_result` consults
`keeps_to_its_frame` but runs inside an O103 fold that D305 refuses for `c`,
`d` and `e` (not plain locals; `global`-linked); the Explorer line and the
summary's `evidence` are what is wrong today. The summary is the fact the
runtime consumers of slice 7 are to share ("the registry readings D307 names,
which a runtime consumer can share"), so it should either carry the place a
qualified or aliased `Name` argument names among `globals`, or have no summary
for such a caller (D299's barrier), with the Explorer saying so.

### S2. After a callee's unbind or may-bind, W210 does not read the place "as the call leaves it"; the drafted diagnostics row overstates

The record drafts for `diagnostics-calculation.md`: "a caller's W210 and W211
read a place a `Name` argument names as the call leaves it". For a bind that
holds (`bump q; puts $q` draws nothing, d01). For an unbind it does not:

| program (`progs/w/…`) | tclsh 8.4.20 … 9.1.0 | `tcl diag --dialect tcl8.6` (and 8.4) |
|---|---|---|
| w01 `proc reset {name} {upvar 1 $name v; unset v}; proc p {} {set m 1; reset m; puts $m}; p` | `can't read "m": no such variable` | silent |
| w03 the same at the top level, w04 with `return $m` | same | silent |
| w05 `proc maybe {name c} {upvar 1 $name v; if {$c} {set v 9}}; proc p {c} {maybe k $c; puts $k}; p 0` | same, for `k` | silent (no "may be read before set") |
| w02 `proc p {} {set m 1; unset m; puts $m}; p` (the direct form) | same | `1:35: warning W210 Variable 'm' is read before it is set` |
| w07 `… reset m; if {[info exists m]} {puts $m} else {puts gone}; puts $m` | `gone` then the error | `I230 Condition '[info exists m]' is always false` — and nothing for the read |

The lattice holds `m` Unbound after `reset m` (I230 decides on it; the seven
witnesses' `reset m` decides the same); W210 never asks, because the
call-as-assignment suppression that predates the slice
(`build_undef_suppression`'s `may_defs`, `analyser/diagnostics/helpers.rs`)
takes any call whose definitions name `m` as its assignment before the fact is
read. Base binary: silent too. The slice did not introduce the silence, but it
claims the opposite in the row it drafts and in `value-transfers.md`'s
consumer list ("W210 and W211 in the caller (a `Name` write defines the
caller's place)"); either the suppression reads the summary's step — an
`Unbind` step is not an assignment — or the row says bind only.

## N — nits

- N1. The four KCS notes the task names are untouched by the slice
  (`git diff e355162ca..163f3712d -- docs/kcs` is empty). The O103 note's
  "Skipped when the proc body cannot be summarised" and its skip list say
  nothing of a caller that stays foldable through a callee's `upvar` write of
  its own local (D305), of the re-run seeding `Name` parameters from the
  caller's places, or of a recursion that now folds (`[fact 5]` → 120, `[rec
  4]` → 10, with the depth-32 and 4096-re-run bounds); the W210 note's
  "treats the call as the assignment" paragraph is the pre-existing rule S2
  rests on and now disagrees with the drafted row.
- N2. D303 says a `switch`, `catch` or `try` kept whole "had been paired with
  its header's word effects through a shared span, though the pair's
  evaluation never ran for such a host; it no longer is … and O109's
  hidden-read scan no longer credits it with the header's reads", as if that
  changed nothing observable. It changed nothing because the shape was already
  wrong: `proc p {} {set x 1; switch -- [incr x] {2 {return "two $x"} default
  {return other}}}; puts [p]` is sunk into the arm by O125 under base and HEAD
  alike (P8). The record should say so, and the shape belongs in the issue.
- N3. `transfer.rs`'s `callee()` comment says "a command the registry knows
  runs no code of the module's"; `eval`, `uplevel`, `apply`, `after` and
  `namespace eval` do. The summary stays right for them (g01–g05, g20, g21:
  `eval "incr v"`, `uplevel 0 {incr v}`, `apply {{} {uplevel 1 {incr v}}}`,
  `uplevel 1 [list incr $name]` all leave `puts $n` unfolded; `eval {incr v}`
  and `eval [list incr v]` fold to the right 2) because the frame facts
  (`frame_is_closed`), `evaluated_command_substitutions(...).opaque` and the
  lattice's treatment of a script-written local carry it, not that line; the
  comment should name what does.
- N4. `the_seven_summary_witnesses` reads `creates_absent = matches!(dialect,
  "tcl8.6" | "tcl9.0")` with the sentence "folded only under a release that
  creates the cell"; `DIALECTS` holds no tcl8.5 or tcl9.1, under which the CLI
  also folds `puts 1` (a07: `--dialect tcl8.5` and `tcl9.1` print `puts 1`,
  `tcl8.4` keeps `puts $absent`). A comment that names the dialect list would
  save the next reader the check.
- N6. No witness exercises the summary's step applied without a re-run
  (`place_answers` with `after == None`: a callee past the depth-32 or
  4096-re-run bound, or a recursion its seeds do not end), so a wrong
  composed outcome (M1) passes every witness and only
  `summaries_compose_through_two_callees` stands between it and the caller.
  A witness with a bounded recursion over an `upvar` link (`up z 40` in b22
  prints 40 and is left unfolded, so it is a candidate) would close that.
- N5. `RerunKey` carries the callee, the seeds and the trust but not the policy
  or the dialect; both are fixed per `ModuleProcedures` instance (one per unit
  build, one per O103 fold), which the type's comment could state.

## Mutation coverage

Each mutation applied with `harness/mut.sh` (backup, `sed`, build the one test
target, run the named tests, restore byte for byte, `git status` clean after
each; logs under `out/mut/<name>/`). The tree was clean and no `cargo test`
other than the review's was running before each.

| mutation (`out/mut/<name>/mutation.diff`) | target and tests run | outcome |
|---|---|---|
| M1 `outcomes_of`: a place unbound at every exit read as `Preserve` (`transfer.rs:1714`) | `tcl-compiler --lib`: `interprocedural::transfer` | dies: `summaries_compose_through_two_callees` (`reset` reads `[Preserve]` for `[Unbind]` under tcl8.4) |
| M1b the same | `value_transfer_witnesses`: `the_seven_summary_witnesses` | **survives**: the witnesses take each place's fact from the callee's re-run (`place_answers` prefers `after.places`), and the summary's step is applied only where no re-run can be made (the depth and count bounds, a recursion its seeds do not end), a path no witness exercises — the unit test is the only guard (N6) |
| M2 `word_effects_host`: a `Statement` host paired with no statement (`ssa.rs:3588`) | `--lib`: `word_effects`, `hidden_reads_are_what_the_ssa_does_not_record` | dies on both: `word_effects_stand_ahead_of_the_statement_whose_words_they_are`, `hidden_reads_are_what_the_ssa_does_not_record` |
| M2b the same | witnesses: `an_embedded_call_applies_its_summary`, `set_result_incr_keeps_its_increment`, `a_nested_caller_keeps_the_callee_whole` | dies on the first and third; #2050's `set_result_incr_keeps_its_increment` survives (the definition point's own use keeps the store alive without the pairing) |
| M3 `solve_rerun`: the links' values never seeded (`transfer.rs:672`) | witnesses: `bump_decides_through_both_o103_paths`, `the_seven_summary_witnesses`, `two_callers_share_one_summary` | dies on all three (no `const(2)`, no `puts 4`, no `2 11`): a place's value comes only from the seeded re-run (R7) |
| M4 `memo_key` ignoring `reads_module` (`tcl-lsp-db/src/lib.rs:2204`) | `tcl-lsp-db --lib`: `value_transfer_parity` | dies: `a_lattice_that_read_another_procedure_reaches_the_checks_and_rewrites`, `a_call_through_a_name_parameter_decides_in_the_editor` (15 of 17 pass) |
| M5 `names_own_places` always true (`interprocedural.rs:1664`) | `--lib`: `a_call_naming_the_callers_own_place_keeps_it_pure` | dies (`::g` with `bump ::n` and `h` with `global n` read as pure) |
| M6 `call_transfer` without the word-count check (`transfer.rs:490`) | witnesses: `the_seven_summary_witnesses`, `bump_decides_through_both_o103_paths` | **survives** both: no test calls a summarised procedure with a word count its arity rejects — B1's b37 route |

Every mutation's build and run log is beside its status; the tree was `git status`-clean after each restore and at the end of the chain (`out/suites/final-status.txt`, 0 lines).

## P — pre-existing defects found in passing (for GitHub issues, not fixes)

### P2. O109 deletes the store feeding a procedure call's `Name` argument inside an `if` or `elseif` condition, and W220 calls it never read

`set n 1; if {[bump n] == 2} {puts yes} else {puts no}; puts $n` (a11) prints
`yes` `2` under tclsh 8.4.20 … 9.1.0 and, after `tcl opt --profile full` under
every dialect, `no` `1`: `set n 1` is gone (`# O109 Eliminate dead store`), and
`tcl diag` says `2:5: hint W220 Assignment to 'n' is never read`. The SSA
(`tcl explore --show ssa`) has the `<cond>` statement with `defs {n#2}` and
`uses {}` — no form of a call records the callee's read of the place (`bump n`
and `set r [bump n]` show `uses: {}` too) — and the direct and value-word
forms survive only through `collect_rmw_hidden_reads`, which never covers a
`Branch` condition. Same for h01 (no `else`), h04 and h09 (in a procedure),
h10 (two calls, both stores), h11 (through `twice`); `while` and `for`
headers keep the store (the loop's φ reads it), as does a nested `[incr n]`
(h03). The sweep's 227 DIFF rows are all this shape. Base binary: identical.
`embedded_subst_extras` and the scan's condition gap are not in the diff. Fix:
record the `Name` argument as a read on the call and on the condition's
dispatch (the callee's `VarRead` trait, or the summary's `Name` role), which
makes the SSA right for all three forms instead of papering two of them.

### P3. A `::`-qualified `Name` argument, or a caller's `upvar #0` alias passed as one, writes a global the top level then forwards and deletes

`set g 1; proc c {} {bump ::g; return $::g}; puts [c]; puts $g` (a22) prints
`2` `2`; optimised: `puts [c]` then `puts 1`, with `set g 1` deleted (`O102
Forward literal load of 'g'`, `O109`) — under 8.4 the optimised program then
raises inside `bump`. Same for j01 (`c` as a statement), j03 (`bump ::g` at
the top level: `puts 1` for `2`), j07 (through `twice ::g`: `puts 1` for
`3`), j09 (`proc c {} {upvar #0 g x; bump x}`). Base binary: identical;
`GlobalWriteInfo` (unchanged) records no write made through a callee's link
to a qualified or aliased place, and the summary inherits it (S1). `global g;
bump g` in the caller (j08) and `bump ::ns::g` of a `namespace eval`
variable (j06) are right.

### P4. O122 turns a tail call inside an `if` with no `else` into `while {1}` with no exit; the optimised program never ends

`proc down {n} {if {$n > 0} {puts $n; down [expr {$n - 1}]}}; down 3` (b42)
prints `3 2 1`; `tcl opt --profile full` gives `proc down {n} {while {1} {if
{$n > 0} {puts $n; set n [expr {$n - 1}]}}}` (`# O122 Convert tail-recursive
'down' to iterative loop`), which loops forever once `$n` is 0 (the harness
timed b22 out, exit 124, under 8.5–9.1; 8.4's dialect gets no `lassign` form
on b22 but the same `set n` form on b42). Base binary: identical. b22's `up`
(a recursive `upvar` writer) gets the same loop plus `upvar 1 $name v` re-run
each iteration; its folds `puts 5` / `puts 3`, the slice's, are the right
values.

### P5. A bare write of a qualified `variable`'s tail is removed as an unused assignment, and W211 reports it

`proc init2 {} {variable ::cfg2; set cfg2 2}; init2; puts $cfg2` (d08) prints
`2`; optimised `proc init2 {} {variable ::cfg2; }` then raises `can't read
"cfg2"` (`# O126 Remove unused variable assignment`); `tcl diag` W211
`Variable 'cfg2' is set but never used`. Base binary: identical. The
read-before-set side already knows the tail (`collect_qualified_variable_alias_tails`,
the helper the slice rewrote to read the registry's `Namespace` alias frame
and `VarWrite` role); O126 and W211 do not.

### P6. iRules: a `sharedvar` variable's writes are deleted as unused or dead, with false W211 and W220

`progs/r/r01_sharedvar.irul`: `when CLIENT_ACCEPTED {sharedvar mode; set
mode 1; …}` and `when SERVER_CONNECTED {sharedvar mode; if {$mode == 1} {…};
set mode 2}` → `tcl opt --profile full --dialect f5-irules` removes both `set
mode …` lines (`O126`, `O109`), and `tcl diag` reports `3:9 W211 Variable
'mode' is set but never used` and `10:9 W220 Assignment to 'mode' is never
read`; r02 (two events) the same. Base binary: identical. The slice's D302
declares `sharedvar` a scope alias (`scope-alias:connection`) and retires its
`KNOWN_GAPS` row on the ground that "the escaping treatment of an aliased
name already gives the lattice" the answer; O126, O109, W211 and W220 do not
read the plan, so the connection-shared cell is a plain local to them. `my
variable` (d13) and `variable` (b15) keep their writes.

### P7. W220 reports a caller's local as never read when a callee writes a same-named namespace variable through `variable`

`namespace eval ns {variable n 0; proc inc {} {variable n; incr n}}; proc q
{} {set n 1; ns::inc; return $n}; puts [q]` (b44, b15) prints `1`; `tcl diag`
says `W220 Assignment to 'n' is never read` at `set n 1` (5:16 and 7:16 in
b44). The CFG builder widens the `ns::inc` call's definitions with the
callee's outer write `n` (`global_write_procs`) without knowing whether the
caller's `n` is linked, so the caller's own store looks overwritten. O109
does not fire (the program is unchanged), only the report is wrong. Base
binary: identical.

### P8. O125 sinks a store into a `switch` arm past the subject's `[incr x]` (#2327's class, header side)

`proc p {} {set x 1; switch -- [incr x] {2 {return "two $x"} default {return
other}}}; puts [p]` (k01; c14, c22, k02 at the top level, k07 unbraced arms,
k08 with `[bump x]`, k10 `-glob`, k14 `[catch {incr x}]` as the subject)
prints `two 2`; optimised `switch -- [incr x] {2 {set x 1; return "two $x"} …}`
prints `other` (`# O125 Sink 'x' into the branch(es) that use it`). Base
binary: identical; the MCP build of 2026-10-06 05:35 deletes the store
outright (O109) instead. #2327 names "an intervening write … in the target
body"; here the write is the header's, so the issue should name the header.

### P9. A `Name` parameter's default names a caller place no definition records: O102 forwards and O100 folds across the call

`proc bumpd {{name n}} {upvar 1 $name v; incr v}; set n 1; bumpd; puts $n;
proc p {} {set n 1; bumpd; return $n}; puts [p]` (i06) prints `2` and `2`;
base and HEAD alike print `1` and `1` (`# O102 Forward literal load of 'n'`,
`# O100 Fold return of constant variable`): the CFG builder's call-by-name
definitions come from the call's words, never from a parameter's default.
B2 is what the slice builds on top of it.

### Known, met again (no new issue)

- a13 `foreach x [list [bump n] [bump n]] {puts $x}; puts $n` → `puts 1` for
  `3`: #2262 (a `foreach` list word's substitution effects).
- b31 `proc c {} {set n foo; incr n; return 0}; puts [c]` → `puts 0`: #2390,
  the base of B1.
- b26 `set n foo; if {[catch {bump n} msg]} {puts err}; puts $n` → `set n foo` deleted and `puts foo` forwarded, so the optimised program prints `foo` for `err` `foo` (8.5+ `incr` creates the absent `n`, so the `catch` no longer catches): P2's class through a `[catch …]` header; base identical.
- `fumagic/filetypes.tcl`, #2404 and #2327's body-side shape were not
  re-measured.

## What the slice gets right (checked, for the record)

- The seven witnesses and their shapes: `bump n; bump n 2` → 4, `reset m` →
  `if {0} {} else {puts unbound}` with I230, `twice t` → 3, `g; set
  ::counter` and `ctr; ctr; set ::hits` read where the summary binds with no
  value, `[rec 4]` → 10, `bump absent` → 1 from 8.5 and kept under 8.4 — each
  printing what tclsh 8.4.20–9.1.0 print before and after `tcl opt`, under the
  auto dialect and the release's own (a01–a07, 70 rows SAME); the CLI's
  `explore --show sccp` prints `n#2 = const(2)` in `::p` (the exit evidence),
  and `--show interproc` prints each summary (`bump`: "names a place 1 up,
  binds a scalar · by: value", `reset` unbinds, `maybe` may bind, `up`
  settles on a may-bind, `far` "none — a call is a barrier").
- Call shapes: the call in a value word (a09), an `expr` operand (a10, left
  unknown as the recorded limit says, and `[s]` folding to 6 on the
  argument-sensitive path, c23), a `while` header (a12), `catch` (a14), a
  `switch` subject (a15), a `return` value (a16, c24), a variable `by` (a17),
  a substituted name (a18, barrier), `eval {bump n}` and `uplevel 0` (a19),
  `bump n [incr n]` and `bump n [bump n]` (a20, a21: left unfolded), an array
  element and an array place (a24, a25), levels 1, 2, `#0`, `$lvl`, an odd
  word count (a26), two names in one call and the same name twice (a27: `2
  11` folded right, `two p p` a barrier), a computed level (a28), 5000 and
  4200 calls in a loop (a30, i18): every row SAME.
- Completion and rebinding: `return -code return` and `-code break` in a
  callee (b03, b04, b07 unfolded), a callee that errors after its write under
  `catch` (b06: 2), `upvar 2` through a middle frame (b08: no fold), `namespace
  upvar` (b09), a global written after a read (b10, b11), a conditional unset
  and bind (b12, b13, with `info exists`), `global` and `variable` in and out
  of `namespace eval` (b14, b15), a module that rebinds `puts` (b16: no
  summary, no fold), a callee defined after its caller (b17: no fold, right
  output), a redefinition and a `rename` between calls (b18, b19: no fold),
  mutual recursion (b20: no fold), the depth bound (`sum 30` → 465 folded,
  `sum 40` and `down 33` not), `info default` with and without a default,
  with `args`, a missing procedure or parameter, after `rename`, from a
  namespace holding its own `f` and through `interp alias` (b23–b25: `ns::g`
  → 7, `g2` → 5, nothing folded once `f` is renamed), `unset -nocomplain`
  (b28), a read-only callee's store kept (b29, d04), `append`/`lappend` links
  (b30).
- Batch i: a namespace `bump` shadowing the global one and `::bump` beside
  it (i01: 11, 2, 11), a callee defined later inside `namespace eval` (i02),
  `rename a {}; rename b a` between calls and `interp alias` (i03, i04: no
  fold, right output), an `args`-tail link (i05: no summary), the place as a
  `foreach` variable (i07), `return [get n]` and `[expr {[get n] + 1}]`
  (i08), a `Name` beside `args` with braces in the rest (i09), a two-name
  swap (i10), the parameter read as a value beside its link (i11),
  `global`/`variable` then `bump` (i12, i13), an `upvar 0` place (i14),
  `catch` around the call with a raising callee (i15), a call in each arm
  and under a decided `if` (i16), `while` with the call in the body and the
  header (i17), 4200 calls (i18), a callee named before its definition and
  an unknown one under `catch` (i19), a callee that calls an unknown
  command under `catch` (i20): 190 rows SAME.
- Callee bodies writing the link by other means (batch g): `eval` braced,
  quoted and `[list …]`, `uplevel 0`, `apply`, `foreach`/`while`/`switch`/
  `catch`/`try` inside the callee, `lassign`, `scan`, `regexp`, a `foreach`
  variable, `dict set`, a same-frame `upvar 0` re-alias, `unset; set`, a bare
  `return`, an early `return` before the write, a conditional link, `subst`,
  a `$cmd` head, `info exists` beside the write, `array names`, a `global` of
  the caller's name beside the link, `namespace eval`, `after idle`: 300 rows
  SAME.
- The word-effects pairing: every #2050, #2141 and #2132 program (c01–c12)
  byte-identical to the base binary under tcl8.4 and tcl8.6 and SAME under
  every release; `[incr x] + $x` in an assignment, a `return`, a quoted word,
  an `incr` amount and an `if` header (c13, c17–c21), `catch` and `try`
  headers (c15, c16), a dead store beside word effects (c25).
- R1: no literal command name in the four consumer files or `transfer.rs`
  outside tests (the one `p == "args"` at `interprocedural.rs:67` is
  pre-existing and recorded); `cargo xtask value-transfers --check` passes
  ("27 file(s) clean, 19 site(s) waived, 69 site(s) pinned") and
  `registry-axes --check` passes ("18 file(s) clean, 37 waived, 858 pinned").
- R6: both O103 anchors and every #2050/#2141/#2132 witness pass unchanged
  (`optimiser` 105, `optimiser_coverage` 93, `inlining_interproc_residual`
  58, the witness binary 158 with `TCL_LSP_REQUIRE_TCLSH=1` and all five
  releases, `value_transfer_parity` 17 in `tcl-lsp-db`, `value_transfers_cli` 28 in `tcl-cli` with the seven witnesses and the nested caller under each release); the lib
  unit tests named by the record pass (`interprocedural::transfer` 4,
  `word_effects` 3, `param_traits::` 57, `slot_resolution::` 27,
  `global_alias` 11, and the singles).
- R7: a seed-dependent value never enters the summary (M3 below); `far`,
  `opaque`, `outer`, a computed level, `args` links (i05) and a procedure
  defined later are barriers; `up` is may-bound; #2134's nested caller keeps
  `upvar 1 $name v` (the witness, h10 with two names).
- D307's level rule: `upvar $lvl $a b` pairs `(a, b)` (d07, a26 `lx 1 n` and
  `lx #0 n` right under every release).

## Verdict

Rework. B1 and B2 are miscompiles the slice introduces, each with the base
binary leaving the program alone: D305 makes a caller of a `Name`-linking
callee foldable, and a callee whose re-run raises under the call's seeds (a
non-integer, an absent place under 8.4, a division by zero) or whose arity
rejects the call leaves the caller's exit reading a constant, so `puts [c]`
becomes `puts 0` where every tclsh raises, on both O103 paths and in the
embedded form (B1); and a call that omits a `Name` parameter whose default
names a caller place is a barrier that widens nothing, so `set n 1; bumpd n;
bumpd; puts $n` prints 2 for 3 (B2). S1 (the summary's `globals` states a
false fact for a qualified or aliased `Name` argument) and S2 (W210 does not
read a callee's unbind, though the slice's drafted row and
`value-transfers.md` say it does) should be fixed or restated before the
slice is called complete. Everything else the summaries decide was right
under every release in 1630 hand rows, 4410 sweep rows and the five suites
(the witness binary 158, `optimiser` 105, `optimiser_coverage` 93,
`inlining_interproc_residual` 58, `value_transfer_parity` 17,
`value_transfers_cli` 28, both xtask gates); every other miscompile found
(P2–P9) is pre-existing with the base binary's identical output as evidence;
six of the eight mutations die on the tests the record names, and M1b and M6
survive the witness binary (N6, and B1's word-count route), which is where
the lane's coverage should grow.

S items: S1 (the summary's `globals` for a qualified or aliased `Name`
argument), S2 (W210 after a callee's unbind, and the drafted row).

## Run log

- 06:25 gate run (`make rust-check`, the coordinator's) in `tcl-lsp-gate2` finished `exit=0`; reading done first.
- 06:55 `cargo build -p tcl-cli` (1m51s), binary copied to `bin/tcl`; harness and programs written.
- 07:00–08:30 hand batches a, b (600 rows), c, d, g (700 rows), h, i (330 rows) through `run.sh`; the sweep (4410 rows) through `sweep.sh` (07:03–07:35).
- 07:24 the whole witness binary: 158 passed (789 s); 07:30 base binary built from the scratch worktree (5m36s) and the base-vs-HEAD battery run.
- 07:38 lib unit tests; 07:45 the chained suites (`optimiser_coverage` 93, `optimiser` 105, `inlining_interproc_residual` 58, `value_transfer_parity` 17, `value_transfers_cli`) and the eight mutation runs, one at a time; `cargo xtask value-transfers --check` and `registry-axes --check` OK.
- No container restart during the review; every background run completed from its own log.
- Tree: `git status` clean throughout; every mutation restored byte for byte (`out/mut/*/status`); the scratch worktree removed and pruned at the end.
