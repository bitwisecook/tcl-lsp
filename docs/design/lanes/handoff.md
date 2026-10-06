# Hand-off: resuming the value-transfer build

This page is the orchestrator's resume point for the work on branch
`claude/spectcl-optimization-discussion-5qhf42` (issue #1943: value
transfers, the diagnostic policy, the registry consumer contracts). It is
rewritten at every push. The two lane documents carry the detail; this page
says where each lane stands, what is queued, and how the work is run.

## Where the lanes stand

| Lane | Tracking document | Landed | In flight | Remaining |
|---|---|---|---|---|
| Value transfers | [value-transfers.md](value-transfers.md) | slices 1, 2, 3, 4, 5, 8, 6 (reviewed, reworked in three rounds, re-checked "land as is") and 9 (`f06639de`); slice 10 item 3 complete: items 1, 2, 3a to 3e and 3d's remainder (`ddb53a10`, `b9ab4993`, `7ee77e66`, `6f86956d`, `07633485`, `e0675133`, `22be2fd0`, `aa0cc76e`) with the SpecTcl catalogue fix `1d3a937f`; item 4 complete (`513196e5` the reconciliation note, `596a5d86` the registry's handler chain and `try`'s protocol, `2ebf9756` the CFG half carrying `rust`'s #2230 change and reading the chain, with two `try` lowering defects fixed on the way); item 5 complete (`2e4cf3b9`: the prefix rule in the faithful-exceptions build, a `try` body split per statement with every split point wired to its handlers and `finally`, closing D225's open case; 1033 programs differential-clean on tclsh 8.6, 9.0 and 9.1 apart from the pre-existing O125 and O126 defects); the item 3 fix `823e1287` (a `catch` body's store counts as run only where its target is proved writable, closing the lane's own miscompile where a store to a variable that may be an array was taken as succeeding; D244); item 6 (`2d40a40f`: a loop absorbs `break` and `continue`, the registry's loop rule stating each loop's result, `foreach` empty and `lmap` the collected list; D245); item 7 part 1 (`2b67421e`: O109 and O126 read a chain's uses through the running-use rule, a store a later definition preserved counting as read, closing the lane's own item 3d defect; D246); item 7 part 2 (`754baa7b`: `scan` goes on past a store it cannot make as tclsh does, a write in a substitution-run `catch` or `try` body is a may-definition, a `try` body that never rests gets its region entry; D247 to D249, correcting D232 and D224); item 8 (`613c2147`: the slice's witnesses in all four build shapes, the registry's routes against tclsh 8.4 to 9.1, three CLI witnesses); item 9 (`379982e3`: the docs sweep, with two precision gaps recorded in `precision-limitations.md`); the landing (`4e7b15f2`: "slice 10" in the `registry-axes` gate's LANDED list, the ledgers regenerated unchanged, the migration page, the lanes README bullet and the lane doc's status section with what slice 11 starts from). Slice 10 is landed: slices 1 to 6 and 8 to 10. The upstream merge then reshaped three of the lane's rules on the merged tree (`a1e32956` one marker for calls to code the module cannot see, correcting D195: an unnamed head widens the caller's locals held at the call once, a named head's frame effect comes from the registry, D250; W210's silence in a procedure is per name, for a name the callee is handed as a literal word and after `source`, D251; `bfca7a26` the raise proof reads the existence rung, which takes the registry's write class where a declared existence step is a may-bind or the generic widening, D252) and fixed one of the lane's own defects (`5b247801`: the deferred-writes scan reads an alias's command prefix as one command); the slices 9 and 10 review's first fix (`7f1d9c8b`: a raising store is never dead, the raise proof refusing a definition the solver answers Raised for and a preserved definition, the coupled constant dead-store removal refusing both, a command substitution in a value word qualifying only where the solver folds the definition; sixteen witnesses kept under tclsh 8.4 to 9.1; four tests that pinned the deletion of an unproven call's store restated with their oracle; D253); the second (`3d36d2bf`: one decoding of `return`'s options in the registry, read release by release as tclsh reads them and shared by every reader, a level-0 `return` lowered as the completion it names so the CFG reads an error as a throw point and `break` or `continue` as the jump, the rest staying the return barrier; closes #2357; D254); the third (`f1708203`: one helper finds each release's reference tclsh at its pinned patchlevel, a witness that cannot find one says so on stderr or fails where `TCL_LSP_REQUIRE_TCLSH` names the release, and CI requires 9.0, so the witnesses compare there); the fourth (`0e43c05e`: a stub states its frame effect, `-frame own|none|caller`, through the registry's own frame-effect field, so a plain-call stub so declared is bound beside the catalogue's names and widens nothing, a `caller` stub blinds the computed-name walk and the deferred-writes scan as argparse does, and W123 says in a procedure's own frame what the call widens and what a stub keeps; D255); the fifth (`1a4f44dc`: one vocabulary for a handler's completion code, a `try` `on` clause holding the registry's completion code that one decoder reads, the separate selector type gone); the sixth (`ae3e6af0`: W123 carries its widening sentence only where an unseen-call marker covers the head, the one fact the widening reads, so the top level and a `namespace eval` body get it and an `uplevel #0` body inside a procedure does not, and the stub pages say `tcl opt` does not read stub declarations yet, #2366; D256); the seventh and last (`2a1c7745`: a callback's lambda writes what its body writes, the deferred-writes scan reading an `apply` lambda as `apply` does and asking the procedure summary what the body writes in the global frame, the summary now counting a barrier's bindings and a structured statement's own bindings; the 384-run sweep of writer lambdas in callback forms against tclsh 8.5 to 9.1 goes from 72 mismatches to 4, all one program on the pre-existing #2367; D257). The slices 9 and 10 review is closed. Slice 11 checkpoint 1 (`0c02a60e`: the condition transfer's refinements recorded on executable edges, the registry's `string is` member type proving a type only for the integer and double classes under `-strict` and for `dict` as `tcl::unsupported::representation` reads under 8.6 to 9.1, the Explorer's SCCP view listing refinements on executable edges only; `tcl opt` and `tcl diag` byte-identical over 1231 files); slice 11 landed (`f608abe7`: the solver narrows each block by the refinements in force there, so a nested equality decides I230 and `tcl opt` folds it, and a definition in a refined arm takes the arm's value, witnessed unchanged under tclsh 8.4 to 9.1; the ranges read the condition transfer's Range rows with the ordered half-lines, a boolean word bounding nothing, closing #2369; W210 takes each existence guard from the condition transfer by dominance, the guard helpers deleted, a first form that read the rung caught by the differential on two tcllib files; the type lattice and the shimmer checks read the type refinements; D261 to D264; one intended difference over 1231 files, two S101s gone in `units.tcl`); the slice 11 review (B 0, S 5, N 6, P 7; land as is) and its first fix (`aeba2f45`: a Range refinement from a comparison is read only where the version is proved an integer, one rule in `refine_interval` covering the slice's `!=` false edge and compositions and the pre-existing `end` case, closing #2368); its second (`05180afc`: the witnesses read each arm's value through a definition O100 rewrites, so a mutant stating an exact fact for a numeric literal or the first member of `in` changes the output; `literal_operand` reuses `fixed_string_operand`; the guards-tier note); its third (`9c47ec58`, pages only: the I230 note names what is never narrowed and says a plain top-level name is narrowed on the constant propagation's terms and that a `::z` write in the same code is no write to `z`, #2370, which precision-limitations.md also records; the W210 paragraph of `sccp-core-analyses.md` against the landed `existence_guards`/`guarded` shape; the `&&`/`||` rows' purity caveat; ledger item 11 against the landed flow; the `-nocase`, `-glob` and fall-through `switch` rows marked not recorded; D266); and the lane's own opaque-catch W210 fix (`07e68cc8`: the host call of an `ArmWrites` marker drops its reads of the names the marker states, the free-reads rule an opaque `switch`'s arms already use, so `catch {set i 0; puts $i}` draws no W210 while a read after the catch of a name the body only may set still does; sixteen false reports gone and three real ones found over tcllib; blind to order inside the body as before D205; D267). Slice 11 is complete and reviewed. Slice 12 checkpoint 1 (`c1d24182`: the registry's iteration answers for `for` and `while` and the loop plans, the solver reading a loop's exit state, the old loop-summary decision gone so the cap program runs ten times faster, the enumeration storing only to places proved to hold a scalar or nothing, an opaque `catch` body run whole on the same engine, a loop's exit state narrowing only downwards, I230 finding a loop's own test structurally in place of the block-name heuristic; eleven witnesses under tclsh 8.4 to 9.1; three correct I230s gained over 1120 corpus files, `tcl opt` output unchanged; D268 to D274); the lane's own may-definition fix (`08d5acf3`: a may-written element's base refreshes with the same quoted read of its prior version the element gets, so an element write in an opaque `switch` arm or `catch` body no longer lets O109 delete the store to the array's base before it, and W210's may-definition map skips a refreshed base so a never-set array draws what it drew; both shapes witnessed under tclsh 8.4 to 9.1; D275); slice 12 checkpoint 2 (`fb2b3109`: the loop summaries answer from the enumeration over a driver on the summary's registry, the old simulator's statement, `if`, `switch` and `incr` executors and its environment inputs deleted from `static_loops.rs` with every `summarise_*` test passing unchanged; the analyser's static conditions read a variable's text through the engine's literal rule; a rebound math function declines the enumeration; the ranges take an enumerated loop's exact exit integer so W230 bounds a counter after its loop; O103's return fold reads the value at the return's block so a call into a counting `for` folds; nine crates' suites green, two mutations each failing one witness; D276 to D279); the W241 exit-scan fix (`c3af84d8`, closing #2381: the body scans behind W241's exit test and the counter's write test descend into every word the registry gives a Body role, braced, quoted or bare, a bare word read as the one command it names, so `if {$i < 0} break` leaves the loop and `if {$i < -5} "set i 20"` writes the counter; four programs witnessed under tclsh 8.4 to 9.1 before and after `tcl opt`; corpus `tcl diag` identical to the second checkpoint's; D280); the escape fix (`25ead4f1`, the lane's own from `c1d24182`: the enumeration cooks each literal word of a call and the `incr` amount through `literal_token_value` before handing it on, Str rules for a braced word and Esc rules for a bare or quoted one, the rule the lattice driver and `word_in_state` already read, so `lappend r a\x41` in a loop folds a later test as tclsh decides it; four programs witnessed under tclsh 8.4 to 9.1 before and after `tcl opt`; D281); the landing (`5bf6eb1b`: W240 to W242 take the loop counter from the iteration plan, the bound the condition's literal, the step a `for`'s step script or a `while` body's one top-level integer cell update, the start a `for`'s value-word write or, at the per-function pass, the integer the solver proves at the loop's start block, so `set i 5; while {$i < 10} {incr i -1}` draws W241 and the old `set v INT`/`incr v` text scan is gone, W241's message naming the loop's command; IRULE5003 reads its loop from the plan and its decrement from the registry's cell update at any depth of the body; a word with a substitution is read over the loop state and a `[…]` script runs as one effect-free command, so the correlated `incr x [expr …]` pairs decide; the registry-axes pins down to 32 and 7 with the report regenerated; five tests and the eleven loop witnesses in the compiler and the CLI under tclsh 8.4 to 9.1, three mutations each failing a test; `tcl diag` and `tcl opt --profile full` identical over 1120 corpus files between each of the slice's commits; D282 to D284). Slice 12 is landed: slices 1 to 6 and 8 to 12. The review's B1 fix (`1d5fbc1f`, its record filled by `b0173698`: the W241 and W242 counter test reads the loop's SSA definitions and the call-frame facts the deferred-writes scan and the unseen-call marker state, in place of a text walk, so a write in a `[…]` word, through a binder or through a procedure's `uplevel` or `upvar` declines the report; IRULE5003 falls back to the default registry; pushed during the disk failure and gated green afterwards in both the gate worktree and the implementer's tree, `make rust-check` all 83 steps, dialect-drift at 8, the nine crate suites, the corpus differential identical over 1120 files; D285); the review's S1 fix with N6 (`723ae0326`: a loop's state is stated only at a version the loop defines, never the version live where its passes start, so a temporary bound before a loop and dead after it no longer contradicts, and a contradiction drops only the contradicted loops' state, the solver's rounds moved into `drive` so a unit test exercises them; the `foreach` binder takes the same store rule as every other store, so `foreach a(k) …` declines; the compiler page's publication and closed-state rules; two witnesses and a unit test, three mutations each killed; nine suites green, the corpus differential identical over 1120 files; D286 amending D270); the review's docs commit (`4831e0e68`: the W241 note says what the counter path reads and its policy paragraph covers both proofs, the W242 example no longer leans on an undefined call, the I230 note says a command substitution the registry evaluates runs over the loop's values without effects and the compiler page's closed-state rule says the same, six stale names out of the file index, a loop's own test decided false now reads "Loop condition '…' is never true; the loop leaves at this test" with a test, comments on the four-empty-words sentinel and the counted plan, the W230 note's variable-index case with #2387's limits and the O103 note's bounded callee loop, the record's note on why R7's error-path half has no behavioural witness; gate and nine suites green, the corpus unchanged over 1120 files). Slice 12 is complete and reviewed. Slice 7a's first commit (`1dbbd9a93`: a lattice double is spelled as Tcl spells it, through `format_double` in `const_to_exact`, the lane's own defect from slice 1, so `string length` of a computed double matches tclsh 8.4 to 9.1; witnessed under the five releases, two mutations each failing the witness; D287; pushed on the implementer's green gate while the device was failing, the suite re-run at the commit following the tclsh 8.5 rebuild, 10186 passed); its second (`dd166c181`, closing #2389: O103's summary path folds only a call whose words after the head are literal and whose count the procedure accepts, the re-run's own condition, so a call whose argument words raise or have effects is left alone; witnessed, two mutations each failing the witness; `make rust-check` and the compiler suite green, 10187 passed; D288; pushed on the implementer's gate while the device failed, the other crates' suites passing whole afterwards); its third (`d4ca2bcbd`, closing #2393: the return reading stops at a statement the flow graph keeps whole whose scripts can run a `return`, the IR's statement variants deciding with no command spelling and a `catch` body not read since `catch` absorbs the `return`, so a `return` inside an opaque `switch` arm or an `if` body no longer lets O103 fold the fall-through's value; a `return` in a command substitution's script stays #2394's; one witness, two mutations each failing it, both O103 anchor tests unchanged; `make rust-check` whole, dialect-drift at 8 and the nine crate suites green, 10188 in the compiler; pushed on the implementer's gate while the device failed and gated green in the gate worktree afterwards; D289); and the slice (`3076a049a`, closing #2388 and #2392: `seedless_returns` runs each pure procedure's lattice once after the purity and effect fixed points, its parameters unknown and nothing seeded, under `FoldTrust::WholeModule` with no existence rung, a procedure the complexity guard stopped skipped, the composition over callers and the cycle bound left to slice 13; the return reading shared by the summary and O103's re-run as `interprocedural::exit_value`, answering an `ExactValue` and keeping the opaque-return guard, a literal word read through `recorded_word_value` and `ExactValue::from_literal` with no trim; `ConstantReturn` lossless, both O103 forms rendering its text; `classify_return` reading exactly, an `expr` literal operand its own value only as a canonical decimal integer, the syntactic shapes answering only passthrough and depends where the run proved no value, which also closes the no-exit case of #2390 (`proc p {} {while {1} {set x [expr {1/0}]}; return 5}` no longer folds to 5); the renderer's `is_value_safe_bare_word` without its trim; `with_interprocedural`, its memoised form and `optimise_raw_for_profile` through one `build_interprocedural_analysis_for_unit`; four witnesses in the compiler, one in the CLI and a residual test, eight mutations each failing a named test, both O103 anchors byte-identical, no command spelling in the new code; `make rust-check` whole and dialect-drift at 8 on the implementer's tree, the compiler's unit tests 6735 passed with its integration tests and the other crates' suites still running at the commit, the corpus `tcl diag` and `tcl opt` identical to the opaque-return fix's for the first 230 files compared; committed and pushed under the device ruling as the device failed reads again, and gated green in the gate worktree afterwards; D290 to D294). Slice 7a is landed, its review running: slices 1 to 6, 8 to 12 and 7a. | slice 7a, seedless return summaries, is landed (`3076a049a`, left column); its fable review is running, with the findings to be filed beside the slice 12 review's; its record's measurements (the nine suites, the corpus differential against the opaque-return fix, a timing comparison) follow in one docs-only commit; the finds left open are #2390 (its no-exit case closed by the slice), #2391, #2394 and #2395 (the slice 12 review, B 1, S 3, N 7, P 7, verdict "rework", nothing the enumeration decided wrong in 945 harness rows, every item now landed, is [value-transfers-review-12.md](value-transfers-review-12.md); its P items are #2383 to #2387, with P4 on #2323 and P6 on #2373) | slices 13 and 7, in that order |
| Consumer contracts | [consumer-contracts.md](consumer-contracts.md) | steps 1 to 9, each reviewed with fixes landed (the step 8 rework and step 9 review's fixes are `16e3a5a0`, `1b870f36`, `cdc7307a`); step 10 items 1 to 5 and item 6's first three parts (`69f5211b` the extension default, `ca5cc729` an extension described from three sources, `75b750df` the host load bridge, `141d30f0`, `2d570cf7` and `ecc1e4e2` one header for two hosts with its C-extension WASM gate, `599e7ac3`, `fb1c1dfe`, `0a562616` and `bfbac02f` the engine's completion, variable and package doors with the shim's side, its second test extension and the return options crossing the door; `56515003` the WASM runtime grown to a real extension's C API surface; `3d3956fd` `tcl spec import` describing one entry point at a time; `a3ce8d4c` the runtime as an engine with its own limits and confinement, every family run on both engines; `4f7d977a` the runtime compiled to `wasm32-wasip1` as an engine under in-process wasmtime in the new crate `rust/tcl-engine-wasm`, with the registry's extension-host seam; `1b4a5c78` on the main branch, the LSP e2e archive's committed closure taking `runtime/rust`, which the hook host's dev-dependency on the runtime pulled in and the merge gate caught; `c6e23a53` item 6 landed, its part 3 mutation run killing all twenty-one mutants after two tests were strengthened; `55feb617` step 10 landed: "step 10" in the `registry-axes` gate's LANDED list, the design page's status box, both design indexes, the lane doc's landing sections and the lanes README; `d8549edd` the step 10 review fixes: the C API's integer reads take C Tcl's ranges on every host, one `content_hash` helper for the artefact and implementation identities, a reused WASM engine granting its first-use fuel once, the extension host's evaluate path noted and the runtime's export list pinned, and two defects found on the way, an evaluation's epoch deadline outliving it and the side-module loader resolving any runtime export rather than the C API alone) | nothing: the lane is complete and reviewed | nothing |
| Diagnostic policy | (landed; see the design page `docs/design/compiler/diagnostic-policy.md`) | complete | | |

Branch heads at the last rewrite of this page: the main branch at the
commit that rewrites this page, above `3076a049a`, slice 7a's landing,
`d4ca2bcbd`, its opaque-return fix, `dd166c181`, its summary-path fix, `1dbbd9a93`, its float spelling fix, and `4831e0e68`, the slice 12 review's docs commit,
`723ae0326`, the review's S1 fix, `b0173698`, B1's record, and
`1d5fbc1f`, the review's B1 fix, itself above `5bf6eb1b`, slice 12's landing,
`25ead4f1`, the escape fix, `c3af84d8`, the W241 exit-scan
fix, `fb2b3109`, slice 12's second checkpoint, `08d5acf3`, the lane's
may-definition fix, `c1d24182`, slice 12's first checkpoint, `07e68cc8`, the last of the
slice 11 review fixes, and `f608abe7`, slice 11's landing (`0c02a60e` its
first checkpoint), `2a1c7745`, the last of the seven slices 9 and 10
review fixes, and `67d9ba5f`, the tip of the upstream merge (`086a3bc3` merges `origin/rust` at `aac0e0d5` above
`4e7b15f2`, then ten fix commits and the lane-doc record); `cc-step5` at
`d8549edd`,
merged into the main branch as `1cda89b6`, and receiving no further
commits now that its lane is complete. `cc-step5` is
merged into the main branch, gated and pushed after every one of its
commits, so nothing of either lane stays off GitHub for longer than one
gate cycle.

## Queued on the running implementers

- Value transfers: slice 7a (seedless return summaries) is landed at
  `3076a049a` and under review; slice 12 (bounded-loop enumeration) is landed at `5bf6eb1b`,
  reviewed, and its fixes are pushed, the last `4831e0e68`; slice 11
  (predicate refinement) is landed at `f608abe7`, reviewed, and its fixes
  are pushed. The review's pre-existing findings are #2370
  to #2376. Open from slice 11: #2368 (a range from a comparison
  holds only for an integer operand, so W230 still fires on `end`) and a
  precision gap on the examples page (in an exact `switch` arm a read of
  the subject itself is not rewritten, so `[string length $x]` there is
  not folded). A stub
  states its frame effect since `0e43c05e`; a stub with a
  body, expression, command-prefix or variable role stays unseen whatever
  its `-frame`, because the flow graph reads roles off the catalogue and
  not the document's declarations, and naming it would lose its body's
  writes; reading the document surface there is a follow-up if wanted.
  Item 4
  (`try`) was shaped to reconcile with `rust`'s PR #2230
  (the try/finally reachability fix for #2142, absent from this branch's
  base) at the upstream merge: one implementation per fact, with a
  "reconciling with #2230" paragraph in the lane doc naming the files and
  facts the merge has to settle. The slices 9 and 10 review is done: its
  findings are the fixes above, and the pre-existing defects it found are
  issues.
- Consumer contracts: complete, every step reviewed and its fixes
  landed. The extension evaluation route's vocabulary (`HostKind::WasmExtension`, the `extension FILE PREFIX` row,
  the artefact hash in the memo key) is value-transfer slice 7's to add
  (its item VT7.11); item 6 built the host side behind the registry's
  `ExtensionHost` seam, which slice 7 binds.

## After the lanes

1. Done at `086a3bc3` to `67d9ba5f`, with the reconciliation recorded in the
   lane doc's slice 10 Record: merge `origin/rust` (at `aac0e0d5`, 254 commits above this branch's
   base `08bceb36`: the dialects/tclspec editor-config generation, the
   jimtcl updates, the Rust 1.99.0 baseline this branch already carries,
   PR #2230, and PR #2196's handler-barrier series, which lands in the
   same compiler code as slice 10) into the branch right after the slice
   10 landing commit, before the slices 9 and 10 review and before slice
   11, so that the review and the later slices build on the merged tree.
   A dry run at `2d40a40f` showed 76 conflicting files: fifteen in
   `rust/tcl-compiler/src` (`cfg_builder/mod.rs` and `cfg_lower.rs`,
   `sccp.rs`, `ssa.rs`, `optimiser/elimination.rs` and `propagation.rs`,
   `ir.rs`, `ir_helpers.rs`, `unit_scope.rs`, `lowering/mod.rs`,
   `tcl_expr_eval.rs` among them), ten in `rust/tcl-registry/src`,
   twenty-four in `rust/tcl-spectcl/tests`, and single files in
   `tcl-dialect`, `tcl-cmd-core`, `tcl-vm`, `tcl-syntax`, the studio, the
   MCP server, the language server, the CLI tests, the shard table, the
   Makefile, the glossary and three design pages. The lane doc's
   "reconciling with #2230" paragraph names the files and facts the merge
   has to settle, #2196's included. Reconcile through the registry
   interface (one implementation per fact), regenerate every generated
   artefact with the repository's tooling, rerun both lanes' suites and
   the gate, push. Repeat before the end if `rust` moves.
2. The final scrub before the pull request to `rust`: delete this
   directory and every link to it, remove slice, step, decision and lane
   references from every page and comment outside it, retire the
   `registry-axis-ok … until` clause and the `LANDED` list in
   `rust/xtask/src/registry_axes.rs`, keep the `value-transfers` ledger
   only in the form its gate needs, and review the result before pushing.
3. The pull request's first CI run is the first time the output witnesses
   compare under the pinned Tcl 9.0 there (`f1708203` made CI require it
   through `TCL_LSP_REQUIRE_TCLSH=9.0`; before that the witnesses looked
   only on `PATH`, where CI's 9.0 is not), so read that run for a 9.0
   witness that never ran in CI before.
4. A last fable review of the whole branch, and the artifact mirror
   (`claude.ai/code/artifact/f1aa5423-0def-49ac-bdb9-30dc19f65830`, at
   Version 8 as of 1a67ca4f) brought up to date.

## How the work is run

- On 5 October at 16:12Z the session's machine began returning disk read
  errors (`I/O error, dev vda`, several sector ranges, growing). The damaged
  git pack was replaced by a fresh clone's, so the history reads; tracked
  files that had become unreadable were restored from it; `tmp/tcl8.4.20`,
  `tmp/tcl8.5.19`, `tmp/tcl9.0.4`, `tmp/tcllib-2.0` and the Tk trees of the same
  releases were mostly unreadable, which broke `make rust-check` (the runtime
  build reads `tmp/tcl9.0.4/libtommath`) and the witnesses under those releases
  until they were restored. A fresh
  session on a fresh machine resumes from this page: the session-start hook
  fetches the Tcl trees again. On the failed machine the damaged trees were
  restored from shallow clones at their tags, and `1d5fbc1f` was then gated
  green in both trees. At 21:55Z the device failed again: the tclsh 8.5.19
  oracle binary became unreadable and was rebuilt, static, from the restored
  sources into `tmp/tcl8.5.19/build85`; the five oracles answer again. On 6
  October at about 01:30Z the device failed again: 552 files under
  `tmp/tcl9.0.4` (the library's `tzdata` and six encoding tables, two test
  files, `manifest.uuid` and the build products) and the build products of
  the 8.4 and 8.5 trees became unreadable, while every tracked file, the
  corpus, the scratchpad and the five tclsh binaries stayed readable; the
  library and test files were restored from a fresh clone at `core-9-0-4`,
  and the build products are not needed.

- Four worktrees: the main worktree on the branch (value transfers, and
  the upstream merge), a second worktree on `cc-step5` (consumer contracts,
  complete; its `target/` was removed to free disk and is rebuilt only if a
  review needs it), and two detached worktrees with their own `target/` for
  gates and reviews (one can hold a running review while the other gates a
  push; the first one's `target/` is also removed).
- Every commit on the main branch is gated with `make rust-check` plus
  `cargo xtask dialect-drift` (eight pre-existing sites; a lane adds none)
  in a gate worktree, then pushed with
  `git push -u origin <hash>:refs/heads/claude/spectcl-optimization-discussion-5qhf42`.
  Nothing is pushed to any other branch. A docs-only commit above a gated
  head gets `make xtask-check` before its push.
- Implementers commit every green sub-item at once, and a sub-item that
  runs past about two hours commits a green intermediate state rather than
  keeping a long-lived draft. `make rust-check` compiles but does not run
  tests, so each lane runs its crates' suites before every commit; the
  value-transfer suites include `tcl-spectcl`, whose catalogue tables
  mirror the registry's id tables, and both lanes' suites include
  `tcl-cli`. `cargo test -p <crate>` stops at the first failing test
  binary, so a failure hides the binaries after it until it is fixed.
- Each landed slice or step gets a fable review in a gate worktree; its
  findings land as "review fixes" commits (blocking ones before the next
  item), and pre-existing defects found in passing become GitHub issues
  (#2253 to #2272, #2291 to #2297, #2299 to #2303, #2305 to #2316, #2323
  to #2328, #2330 to #2335, #2337 to #2358, #2365 to #2376, #2378, #2379, #2381 to #2395 so far), never fixes on this
  branch.
- Implementers ran on sonnet until the account's weekly sonnet limit was
  reached on 2 October (it resets on 6 October, 16:00 UTC) and run on opus
  until then; reviewers are fable. Commit messages are
  `wip(<lane>): <slice or step> — <phrase>` with the session's two trailer
  lines; files outside this directory describe current state only.
- The branch builds under Rust 1.99.0, the same way `rust` does: `rust`'s
  baseline commit (`ee4aeb7a` there) is cherry-picked as `81ffe5f3`, and
  `e96cb2b5` clears the one branch-local site it does not cover (the
  renamed `AtomicU64::try_update`). The Makefile's `CLIPPY_LINT_FLAGS`
  carries `-A clippy::assert_is_empty`, so a direct clippy run outside
  `make` needs the same flag:
  `cargo clippy --workspace --all-targets -- -D warnings -A clippy::assert_is_empty`.
  The repository pins `stable`; every worktree carries a rustup directory
  override to `1.99.0`.
- The machine has four CPUs and fifteen gigabytes: `~/.cargo/config.toml`
  pins `jobs = 2`, gates run with `CARGO_BUILD_JOBS=1`. Disk is tight: test
  executables over 20 to 30 MB and duplicate rlibs under
  `target/debug/deps` are deleted after each run, never while that tree's
  implementer is running; a gate worktree's `target/` can be removed
  whenever it is idle; the extracted registry sources under
  `~/.cargo/registry/src` can be removed whenever no dependency is
  compiling, since cargo re-extracts them from its crate cache. The
  consumer-contract tree holds several rlib variants of the large crates
  (one per feature set it builds), which is inherent to its feature matrix.
- Each agent keeps its scripts in a private directory under the scratchpad
  (two agents once collided on a shared `gates.sh`), and nothing is run with
  a gate worktree as its working directory except the gate itself. A review
  of a lane whose implementer is idle runs in that lane's worktree, whose
  `target/` is warm, read-only and with every experiment reverted; the
  implementer is resumed afterwards to land the fixes.
- The session has restarted several times; drafts survive on disk, so a
  restart is recovered by reading `git status` and `git log` in each
  worktree and relaunching the implementer on the draft.
