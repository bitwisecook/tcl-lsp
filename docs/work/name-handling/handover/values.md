# Hand-over: value transfers, escape, shimmer and types (`values`), 17 November 2026

The longer record of this area, with every fixture and trace, is the section "Value transfers, escape, shimmer and types area, 16 November 2026" in `docs/work/name-handling/current-state.md`. This file is what a successor needs to start; that section is the evidence behind it.

## Scope

All in `rust/tcl-compiler`.

Owned test groups:

| Group | Where (restructured tree) |
| --- | --- |
| `it_2 value_transfer_witnesses::*` | `tests/it_2/value_transfer_witnesses.rs` (harness) and `tests/it_2/value_transfer_witnesses/*.rs` |
| `it_2 var_escape_cfg`, `core_analyses`, `intervals` | `tests/it_2/var_escape_cfg.rs`, `core_analyses.rs`, `intervals.rs` |
| `it_3 var_escape_residual2`, `type_propagation` | `tests/it_3/` |
| `it_4 caller_frame_effects` | `tests/it_4/caller_frame_effects.rs` |
| `it_5 var_escape_cfg_catch`, `var_escape_cfg_tree_arms`, `var_escape_typeinfer` | `tests/it_5/` |
| `shimmer::*` | `src/shimmer/` |
| `type_infer::*` | `src/type_infer/`, tests in `src/type_infer/tests/` |
| `dynamic_names::*` | `src/dynamic_names.rs`, `src/dynamic_names/tests.rs` |
| `native_integer_proof::*` | `src/native_integer_proof.rs` |
| `cfg_builder::upvar_info`, `cfg_builder::global_write_info` | `src/cfg_builder/upvar_info.rs` and `upvar_info/`, `src/cfg_builder/global_write_info.rs` and `global_write_info/` |
| `unit_scope::*` | `src/unit_scope/`, tests in `src/unit_scope/tests.rs` and `src/unit_scope/tests/` |

Owned code: `src/value_transfer/` (the view is in `argument_words.rs`), `src/var_escape/` (the flow-sensitive walker is `cfg_propagation/walker.rs`), `src/shimmer/`, `src/type_infer/`, the two frame summaries above, `src/unit_scope/`, `src/interval_bounds/`, and the native store route in `src/interprocedural/transfer/source_context/native.rs`.

Not owned, although most of this area's failures are decided there:

- The execution walk, `src/command_binding/` (analyser and binding area). The summaries and analyses here consume its facts. When the walk withdraws the command world for a body, every plain call in the module reads as an unknown binding and both frame summaries answer opaque.
- SCCP and the lattice, `src/sccp.rs` and `src/sccp/` (dataflow and optimiser area). `sccp.values` is the strict execution lattice and stays strict.
- Which diagnostic is drawn (diagnostics area).
- The registry's variable projection, `rust/tcl-registry/src/registry/variable_projection.rs` (registry area).

## How to measure

Environment, from the repository root:

```bash
source scripts/dev/agent-build-env.sh
export CARGO_BUILD_JOBS=3 CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0 CARGO_PROFILE_DEV_INCREMENTAL=true
```

Whole crate (the figure every compiler area reports):

```bash
cargo nextest run -p tcl-compiler --no-fail-fast --status-level fail --final-status-level none --failure-output never 2>&1 | tee full.log
grep -E "^\s+(FAIL|TIMEOUT|SIGABRT|SIGSEGV|ABORT)" full.log | sed -E 's/^\s+\S+ \[[^]]*\] *\([ 0-9/]*\) *//' | sort -u > full.f
```

About 9 minutes to build from a warm target directory and 6 to 8 minutes to run on this host; nothing in `tcl-compiler` needs excluding (the 1,200-second test is in `tcl-spectcl`). Select `-p tcl-compiler` alone: adding a second package roughly doubles the build.

One group while iterating:

```bash
cargo nextest run -p tcl-compiler --test it_2 -E 'package(tcl-compiler) & test(/value_transfer_witnesses::/)' --no-fail-fast --status-level fail --final-status-level none --failure-output final
cargo nextest run -p tcl-compiler --lib -E 'package(tcl-compiler) & test(/^shimmer::/)' --no-fail-fast --status-level fail --final-status-level none --failure-output final
```

The area as one regular expression over a failing list (lines are `binary path`, as nextest prints them):

```
value_transfer_witnesses::|var_escape_cfg::|core_analyses::|intervals::|var_escape_residual2::|type_propagation::|var_escape_cfg_catch::|var_escape_cfg_tree_arms::|var_escape_typeinfer::|caller_frame_effects::|tcl-compiler shimmer::|tcl-compiler type_infer::|tcl-compiler dynamic_names::|tcl-compiler native_integer_proof::|cfg_builder::upvar_info|cfg_builder::global_write_info|tcl-compiler unit_scope::
```

Comparing with the baseline list: the list below was taken before the file split, which put a module segment into many test paths (`value_transfer_witnesses::existence::…`, `type_infer::tests::propagation::…`). Compare by binary and last path segment, for example `awk '{n=split($2,a,"::"); print $1, a[n]}'` on both lists, then `comm`. A last segment that occurs twice in one binary needs a look by hand.

Build times: an edit to the library rebuilds the unit-test binary in 10 to 12 minutes, and an integration-test binary in about 3.5 minutes; an edit to an integration test alone rebuilds in seconds. Reach a library change through an integration test where one exists.

## Current numbers

Last full run of `tcl-compiler`: 16 November 2026, on `8846c517` plus this area's two `unset` commits (`6983b2a5`, `bc044f8c`, pre-restructure ids): 13,121 run, 2,357 failing, 342 in this area, 0 new against the previous list.

| Group | Baseline `46775880` | Last full run |
| --- | --- | --- |
| `it_2 value_transfer_witnesses` | 110 | 110 |
| `shimmer` | 47 | 47 |
| `it_2 core_analyses` | 30 | 30 |
| `cfg_builder::global_write_info` | 34 | 26 |
| `unit_scope` | 30 | 26 |
| `cfg_builder::upvar_info` | 21 | 18 |
| `it_2 var_escape_cfg` | 22 | 15 |
| `dynamic_names` | 13 | 13 |
| `native_integer_proof` | 13 | 13 |
| `type_infer` | 14 | 12 |
| `it_5 var_escape_cfg_catch` | 14 | 8 |
| `it_2 intervals` | 7 | 6 |
| `it_3 var_escape_residual2` | 14 | 5 |
| `it_5 var_escape_cfg_tree_arms` | 7 | 4 |
| `it_5 var_escape_typeinfer` | 5 | 4 |
| `it_4 caller_frame_effects` | 7 | 3 |
| `it_3 type_propagation` | 3 | 2 |
| Total | 391 | 342 |

Not re-measured after that run: the dataflow area's `d994c601` (the duplicate `ConstantBranch` record; should clear five `core_analyses` `array_exists_parameter_is_false` tests), and the binding area's `f1eeddf4`, `d37ab705`, `247ac462` and `b358a517` (reported to clear several `upvar_info` tests, including `late_alias_to_relative_one_uplevel_reaches_past_an_enclosing_caller_frame`, and the two alias-installer tests of `global_write_info`). The binding area reported the whole crate at 13,126 run, 2,335 failing on `247ac462`. The first thing a successor does is a full run on the restructured tree.

## Baseline list

`docs/work/name-handling/lists/values-failing-2026-11-16.txt`: the 342 area lines of the full run above, sorted, one test per line, in the pre-restructure path form.

## Queue

In order. Items 1 to 6 are this area's own code or fixtures. Item 7 lists what waits on other areas, so nobody traces it twice.

1. **Full run on the restructured tree** and a new list, compared by last segment. Expect the area somewhat under 342 from the commits named above.

2. **A witness row for an unresolvable head** (owed by ruling). `an_unseen_call_ends_no_refinement…` in `tests/it_2/value_transfer_witnesses/existence.rs` had its expectation changed by the dataflow area in `19f88c91`, which this area agreed to; the original claim (a call nothing can resolve ends every refinement) now has no row. Add one whose head is computed from an unknown value, and one after a `rename`. Check the expected lattice against tclsh by running the fixture with the head bound to a procedure that unsets the variable.

3. **`global_write_info::tests::uplevel_caller_frame_is_not_a_global_write`** (`src/cfg_builder/global_write_info/tests.rs`). The binding projection for the body is exact since the binding area's later commits; the remaining fault is in this summary (`src/cfg_builder/global_write_info.rs`, `record_global_upframe_body` and the relative-frame route beside it). Not traced further. Apply the alias ruling (below) to the remaining alias tests in the same file as their walk causes clear.

4. **Object element typing in `type_infer`.** `lappend items [C new]` and `foreach o $items` leave their targets `OVERDEFINED`; `src/type_infer/container_elements.rs`, tests in `src/type_infer/tests/container_elements.rs`. Decides two LSP semantic-token tests the server session sent. Not started.

5. **Small single causes, not started:**
   - `interval_bounds::tests::bounds_follow_live_command_identity_and_literal_body_quoting` (`src/interval_bounds/tests.rs`): 0 bounds after `rename lindex pick`.
   - `unset_nocomplain_never_raises_bound_or_not` (`tests/it_2/value_transfer_witnesses/existence.rs`): under the `tcl` profile a second `unset -nocomplain x` declines `not-exact`. The release profiles pass. Look at the logical counterpart of `src/interprocedural/transfer/source_context/native.rs`.
   - `it_5 var_escape_cfg_catch::catch_body_incr_escapes_counter`: expects `n` to escape from `catch {incr n 2}` at top level. A unit flattens that body to a typed increment, which escapes nothing; the premise holds only for a CFG with no module context. Decide whether it moves to `opaque_catch_result` (the context-free builder kept for exactly one test) or its expectation changes. It was left failing on purpose rather than moved without a decision.
   - `catch {error boom} m; puts "<$m>"` leaves `m` with its entry version only; `set a [incr n]` in a procedure leaves `a#1` and `n#2` `Overdefined`. Not traced; the second is probably the walk item about a write in a substitution.

6. **Classify what is not classified.** Most of the 110 witnesses have not been read one by one; 18 panic with `the variable` (see the poisoned-function item below). `intervals` (five expect one out-of-range `lindex` finding and get none; two cannot find the symbol they ask for) and the `core_analyses` residue (eleven W210 narrowing tests whose fixtures read an unset name at file level, four `variable_shape` tests on qualified and array names, and single causes) are read from messages only. Collapsing the `var_escape_*` files into table-driven tests was asked for and not started.

7. **Waiting on other areas** (fixtures sent; each is a real Tcl 8.6 unit; re-test after their fixes rather than work round them):
   - Walk: a list command over a value the walk has not proved ends the walk for the body. `proc f {lst} { set x [llength $lst]; lindex $x 0 }` with no caller lowers both statements behind `<caller-frame-opaque>` and `<unseen-call>`. Decides the `shimmer`, `native_integer_proof` and `type_infer` fixtures that are an uncalled procedure over a list parameter (the largest single cause in the library groups).
   - Walk: the command world is withdrawn for `proc ::shout {} { uplevel #0 [list puts hi] }` (with the issue-1797 session), `proc p {} { set y [incr ::hits] }`, `namespace upvar`, and `proc p {} { global g; catch {incr g} }`. The fix for the middle two is the held patch `docs/work/name-handling/patches/walk-stock-object-types.patch`, which is the binding area's and waits on a coordinator ruling.
   - Walk: a scope declaration (`global`, `variable`, `upvar`) inside a `for`, `while` or `try` body raises the caller-frame barrier. The remaining escape tests of that shape wait on it; the `foreach` and `switch` ones pass.
   - Walk: a procedure defined after an undecided `if` that rebinds an alias has no summary row (`interp alias {} g {} upvar #0; if {$replace} { interp alias {} g {} puts }; proc ::p {} { g x local; set local 1 }`). Two `global_write_info` conditional-alias tests.
   - Walk: `proc ::p {} { uplevel 1 { ::set n 1 } }` reports `opaque_variable_frame` from `variable_write_projection_with_metadata_context`.
   - Walk: the representation an `expr` leaves is not captured. `set v 5; expr {$v + 1}; lindex $v 0` at file level under Tcl 8.6 gets no S100 for the second conversion, because `SsaSourceView::read_word_representation_advice` (`src/ssa/source_view/word_reads.rs`) returns none at the `lindex` and `CommitState::at_captured_representation` (`src/shimmer/commit.rs`) by design discards a replayed commitment nothing captured confirms. Thirteen server tests on S100, S101 and S102 wait on this and on the uncalled-parameter item; re-run them when it lands.
   - Registry: `set a($i) 1`, `incr a($i)` and `set fixed {*}$n` mark the whole frame opaque, because `CommandRegistry::variable_write_projection` has no case for a word whose array root is literal and whose index substitutes. Three `dynamic_names` tests.
   - Dataflow: a statement after a proved error poisons the function. `proc p {} {set n 3; set x 1; incr x {$n}; return $x}` gives `Overdefined` for `n#1` and `x#1`. `a_braced_increment_amount_is_its_text` and others of the 18 witnesses that panic with `the variable`.

## Held work

No local commits, stashes, side branches or uncommitted edits. Everything this area changed is on `work`.

One patch, a tool and not a fix:

- `docs/work/name-handling/patches/handover-values-probe-test.patch`. Adds `tests/it_2/zz_probe.rs` and its `mod` line. The test reads a Tcl source from the file named by `PROBE_SRC`, builds a unit under `PROBE_DIALECT` (default `tcl8.6`) and prints, for the top level and each procedure, the SSA statements with uses and definitions, reachability, `sccp.values`, the explanations and the constant branches. Against the restructured tree at `da039ed1`; `git apply --check` passes there; not compiled there (it compiled and was used before the split, and it only uses public API). Apply it locally, build `it_2` once, copy the binary aside and run `PROBE_SRC=x.tcl ./binary --exact zz_probe::zz_probe --nocapture`. Never commit it. The variant that also printed both frame summaries and the escape summaries was lost with the container; they are a few lines over `cfg_builder::global_write_info`, `upvar_info` and `var_escape` to add back.

## Open questions and rulings

Rulings in force (coordinator):

- `sccp.values` stays strict. The witness harness reads it, not `FunctionUnit::semantic_values`.
- Alias tests of `global_write_info`: the binding at the call's execution point decides, not a module-wide may-state. A fixture whose installer never runs is case 2 (fixture not what it claims): make it call the installer, and keep one as-written row pinned to the sound, non-silent answer with the tclsh `invalid command name` error in a comment.
- `upvar_info::nested_relative_one_reaches_past_the_caller`: `uplevel 1 { uplevel 1 { set x 1 } }` in an uncalled procedure stays opaque (`caller_frame_opaque_writes`), with the frame two out still named. The binding area confirmed no query proves a closed caller set for an uncalled procedure. tclsh evidence is in the test's comment.
- `an_unseen_call_ends_no_refinement…`: the dataflow area's changed expectation stands; this area owes the replacement row (queue item 2).
- The possibly-empty loop rule for W210 (silent over an unknown list) is the diagnostics area's, applied there as a table.

Unanswered:

- `walk-stock-object-types.patch`: whether the walk may trust stock object types across a foreign command (the foreign-object-type hazard). Asked by the binding area; this area only waits on it.
- Whether a qualified or array name gets an entry in the type table (`::ns::x`, `a(k)`): four `core_analyses::variable_shape` tests and two `type_propagation` tests. Recorded in the dataflow section as a policy question; no answer yet.
- `catch_body_incr_escapes_counter` (queue item 5): not yet asked; ask before changing the expectation.

## Hand-offs

Sent and waiting: everything in queue item 7, to the areas named there.

Received and not started: the object element typing (item 4), the interval bounds test, the `unset -nocomplain` row under `tcl`, the table-driven collapse of the `var_escape_*` files, and the re-run of the thirteen server shimmer tests once the walk records representations.

Received and done: `unset` left every place `MayBound` (from diagnostics; fixed in `6983b2a5` and `bc044f8c`).

## Hazards

- A shallow clone cannot rebase cleanly: `git rebase origin/work` after `git fetch --depth=50` replayed 50 upstream commits and conflicted. Use `git reset --hard origin/work` and cherry-pick your own commits, or deepen first.
- `work` did not compile once after a rebase (`247ac462`, fixed upstream in `fcbe6f2d`). Build before measuring, and report a broken head instead of fixing another area's code.
- Hypotheses that looked right across a whole group and fixed nothing; do not repeat them: reading `semantic_values` in the witness harness (fixed 0, broke 4); building the `global_write_info` fixtures as real units (fixed 0, twice); rewriting the CFG escape walker to fall back to boundary sites (contradicts `original_condition_substitutions_require_the_current_retained_operand`); loosening the guard in `shimmer` `cost_at_read` (fixed 0, and `captured_physical_representation_supersedes_replayed_commitment` pins the design).
- Fixtures that looked wrong and were right: the eleven `core_analyses` narrowing tests read an unset name at file level, which is the code under test. Fixtures that looked right and were wrong: escape fixtures built on a CFG with no module context retain no source site, so no condition can be read; they now analyse a unit's top level.
- The crate forbids `unsafe`, so a trace switch cannot set an environment variable from a test; use a thread-local `Cell` or read the variable. Existing switches: `TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE=1` (prints `ORIGINAL_WORLD_FINISH … operations=["withdraw"]` when the walk withdraws the world; needs no rebuild), `TCL_LSP_TRACE_N17_FOLD=first-gate-v1`.
- Test binaries are under `target/debug/build/tcl-compiler/<hash>/out/`, not `target/debug/deps`.
- Disk: the container allows 39 GB and one target directory with the test profile takes most of it; about 5.5 GB was free at the end. Do not build a second profile or a second worktree.
- A stop hook complains about uncommitted changes when probes are in the tree. Remove probes before ending a turn; never commit them.
- Every hand-written Rust file stays at or under 2,000 lines (`cargo xtask file-sizes --check`); new code goes into a named module, with no `use super::*`.

## Oracle habits

- `tclsh8.4`, `tclsh8.5`, `tclsh8.6`, `tclsh9.0` and `tclsh9.1` are on `PATH` (`/root/.local/bin`), installed by the session hook; sources are under `tmp/tcl8.4.20`, `tmp/tcl8.5.19`, `tmp/tcl8.6.18`, `tmp/tcl9.0.4` and `tmp/tcl9.1.0`. Jim is not installed in this container; no expectation here was checked against it, and none of this area's tests names a Jim profile.
- One fixture against every release: `for v in 8.4 8.5 8.6 9.0 9.1; do echo "== $v"; tclsh$v x.tcl 2>&1; done`. Wrap the statement under test in `catch {…} m; puts $m` to see the error text, and print the variable after the call to see which frame was written.
- Internal representation, for the shimmer tests: `tcl::unsupported::representation $v` on 8.6 and later prints the value's type (`string`, `int`, `list`) after each command.
- No Tcl core is evaluated on an F5 dialect's behalf. A question only a device can answer goes to `docs/work/name-handling/bigip-name-checks.md`; this area added none.
- Each commit says which of the four cases it is (code regressed, fixture not what it claims, expectation wrong for the dialect, two tests contradict) and quotes the tclsh result that decided it.
