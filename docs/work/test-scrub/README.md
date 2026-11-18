# Test scrub

Started 17 November 2026 at the owner's request: find what the tests hold, where they overlap, and
which of the failing ones are worth making pass. Value is counted in behaviours, not in tests.

## Inventory

`tools/scrub_inventory.py` reads every `#[test]` under `rust/` and `runtime/rust/` without compiling
and writes `data/` (about 20 seconds):

```bash
python3 docs/work/test-scrub/tools/scrub_inventory.py --root . --failing docs/work/name-handling/lists/*-2026-11-17.txt --out docs/work/test-scrub/data
```

- `data/summary.md`: counts by class, crate and family.
- `data/inventory.tsv` (not committed, 8 MB): one row per test with its file and line, whether the
  lists mark it failing, its class, the codes it names, how many Tcl sources it runs, and the groups
  it falls in.
- `data/clusters_exact.tsv`: tests whose bodies are the same text.
- `data/clusters_overlap.tsv`: tests that run the same Tcl sources through the same helpers against
  the same codes. A lower bound: the same check written through a different helper is not caught.
- `data/fixtures_shared.tsv`: Tcl sources used from more than one file, the wider net.

Classes are read from the test's text, so they sort the work; they are not verdicts. **A** runs Tcl
source and cites tclsh, Jim or a release. **B** runs Tcl source and cites an issue, a contract or a
KCS page. **C** runs Tcl source and cites nothing. **D** runs no Tcl source: it holds an internal
structure or API. A differential test that runs tclsh itself and says so nowhere reads as C.

Its own tests: `cd docs/work/test-scrub/tools && uv run --with pytest python -m pytest -q`.

## First reading, 17 November 2026

37,567 tests; 3,743 marked failing by the morning's lists (about 150 have been fixed since).

| Class | Tests | Failing |
| --- | ---: | ---: |
| A, cites an oracle | 4,045 | 564 |
| B, cites an issue or contract | 2,962 | 497 |
| C, runs Tcl, cites nothing | 18,182 | 2,333 |
| D, internal structure | 12,378 | 349 |

- Six failing tests in ten state an expectation about Tcl source with nothing cited behind it. Each
  is either a behaviour to confirm on tclsh, a contract to cite, or an expectation nobody can defend.
- 179 tests in 88 groups are word-for-word copies of another test.
- 844 tests in 385 groups repeat another test's sources, helpers and codes; 157 groups span files
  (the same check in a unit test and again in `tests/`).
- 4,313 Tcl sources are used from more than one file.
- 12 overlap groups have members that pass and members that fail on the same input.
- The families with the most failures: `tcl-compiler` `analyser` (408 in the library, 112 in `it_4`),
  `command_binding` (283), `optimiser` (147), `it_2/value_transfer_witnesses` (109).

## Triage

Each area session writes a ledger for the failing tests of its families before fixing anything else,
`ledger/<area>.tsv`, tab-separated:

```
crate	target	path	verdict	cause	evidence	survivor
```

`cause` is a short slug shared by every test that fails for the same reason; `evidence` is the tclsh
or Jim release and output, the contract or KCS path, the ruling, or the commit; `survivor` is the
test a merged one folds into.

| Verdict | Meaning | What happens to the test |
| --- | --- | --- |
| `KEEP-ORACLE` | It holds behaviour tclsh or Jim shows, confirmed on the releases named | Stays; the product is fixed |
| `KEEP-F5` | It holds iRules or iApp behaviour no Tcl core can show | Stays; a row goes in the BIG-IP check document |
| `KEEP-CONTRACT` | It holds product behaviour a contract, KCS page or ruling states | Stays; the product is fixed |
| `FIX-EXPECTATION` | The oracle or the contract contradicts what it expects | The expectation is corrected, with the evidence |
| `FIX-FIXTURE` | The source is not what the test says it is | The source is corrected |
| `MERGE` | It holds the same observable on the same input as the survivor | Deleted once the survivor asserts at least as much |
| `REWRITE` | The behaviour is real but held through an internal detail | Re-pointed at the observable |
| `RETIRE` | It holds an internal detail or a superseded design, and nothing observable or stated stands behind it | Deleted, with the reason |
| `UNDECIDED` | The session cannot tell | The coordinator rules |

Rules:

- No test is edited or deleted during triage. The ledger comes first.
- "It fails and is hard to fix" is never a reason to retire. A retirement says what the test asserted
  and why nothing observable or stated depends on it, or names the ruling that superseded it.
- A merge needs a named survivor that asserts at least what the merged test asserts.
- A class C test that is kept gets its citation: the tclsh rows or the contract path go into the test.
- The coordinator samples every ledger before anything is deleted. `RETIRE` and `MERGE` then land in
  commits of their own that name the ledger rows, never mixed with a fix.
- Every measurement runs with all oracles required: `TCL_LSP_JIMSH` exported from `scripts/dev/ensure-jim-oracle.sh` and `TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1` set. Until 18 November no session had `jimsh` on its path, so every Jim comparison was skipped and counted as a pass; tests that newly fail under the required oracles join the ledgers as `KEEP-ORACLE`.
- Oracles in the tree: tclsh 8.4.20 to 9.1.0 under `tmp/tcl*/unix/tclsh`; Jim through `scripts/dev/ensure-jim-oracle.sh` (prints `TCL_LSP_JIMSH`); tcllib 2.0 at `tmp/tcllib-2.0` (add it to `auto_path`). "No interpreter here" is not a reason for `UNDECIDED`.
- A test of an internal record with no cited observable is decided by its consumer: if the record can reach a diagnostic, a fold, a rewrite or an editor answer, write the Tcl source that shows the wrong answer and the verdict is `REWRITE` to that observable; if no path to a consumer answer can be shown, it is `RETIRE` with that argument written down.
- A cause that makes the product give a wrong answer (a fold tclsh contradicts, a rewrite that changes behaviour, an edit that breaks code) gets its own slug and goes to the top of the fix list: wrong answers before missing ones.
- A test that cites a proof document, a contract or a naming owner key is not retired on a group argument. It is `KEEP-CONTRACT` unless the document no longer says that (`FIX-EXPECTATION`, with the passage). A retirement's argument is specific to the test.
- Each area ends with `<area>.md`: counts by verdict, the causes ranked by tests blocked, and the list
  of distinct behaviours that are failing. That list is what is worth fixing.

The second pass, after the failing tests, takes the passing ones through the exact and overlap groups.

## After a ledger is signed off

1. The wrong answers it found, each with its tclsh witness as a new test.
2. The repairs: `FIX-FIXTURE` and `FIX-EXPECTATION`, by cause, one commit per cause naming the ledger rows and the evidence. A repaired test then passes or shows a real product fault; it is re-read in the ledger, never adjusted until green.
3. `RETIRE` and `MERGE`, in commits of their own that name the rows; a merge only after the survivor asserts at least as much. Where the survivor is itself failing, either wait until it passes or record in the survivor's note that it is the survivor of those merged tests: it then carries the only copy of the behaviour and cannot be retired, merged or have its expectation changed without re-reading what was folded into it.
4. `REWRITE`, re-pointed at the named observable, by cause.
5. The `KEEP` causes, ranked by tests blocked.

The ledger gains a `status` column and a free `note` column after it, and is kept current, so progress can be counted. A status is exactly one word:

| Status | Meaning |
| --- | --- |
| `open` (or empty) | nothing done yet |
| `repaired` | the test was wrong, was corrected, and passes |
| `exposed` | the test was corrected and now fails on a product fault (its verdict is re-read to a `KEEP` with the cause) |
| `fixed` | the product was fixed and the test passes |
| `deleted` | merged into its survivor or retired |
| `blocked` | waits on a named row or session (named in `note`) |
| `routed` | moved to another session's ledger |

Commit ids, what the repaired test now shows, and who it waits on go in `note`. `python3 docs/work/test-scrub/tools/ledger_status.py docs/work/test-scrub/ledger` prints the table; `--check` fails on any other word.

## Rulings made during the scrub

- *Tests of `inlining::inline_module`* (23, no production caller): read per test by the live-inlining session. "The inlined program answers as the original" is `KEEP-ORACLE`; an assertion on the inliner's internal shape is `REWRITE` to the differential or `MERGE` into a `live_differential` row; an assertion that names the renamed locals is `FIX-EXPECTATION`, since they become anonymous slots.
- *A procedure body is never rewritten from call-site facts.* The procedure stays callable with other arguments from another file, a later `source`, a callback or an interactive session. The call site may fold; the body may not. The product does this today (`proc add {a b} {...}` becomes `return 7` from its one in-file call): a wrong rewrite, top of the dataflow fix list.
- *The strict lattice of a procedure body is never seeded from its in-file callers* (the cause of the body rewrite above, and of I230 on a condition over a formal in a library file). A body is analysed from a generic entry; a value the in-file calls pass may be shown by hover as advice from the advisory side, never from the strict lattice.
- *A pack document's root words belong to its grammar.* The SpecTcl document grammar abstains from W123 on them; the diagnostic is the pack's own notice ("unknown property … dropped"). An open, unsaved pack shows what the pack check says about the buffer as it stands; a notice computed from the saved image is dropped once the buffer differs and replaced by the buffer's own.
- *At rule load nothing is proved, by design.* With no execution-name policy no head is an original input, and a hosted entry binds heads through hosted metadata, which is advice. A top-level `proc` or `when` is a declaration-inventory record (name, body, namespace; a `priority` value is inventory-only; a replaced `when` declines the inventory). Tests that wanted a proved target there are re-pointed at the inventory record.
- *Store removal by liveness in a `when` body is not a licensed rewrite.* Handler variables are connection-scoped and what hosted commands read is advice; the finding may be offered as advice, never applied. The same-event case gets a BIG-IP check row.
- *A binding's target is the command as tclsh names it.* After `rename a b` it is `::b`; the defining declaration is the binding's definition record.
- *A proof nothing consumes is not behaviour.* Range joins that only the sealed four-statement add could use, on sources it can never select, are retired; the unconsumed code is noted for removal or a consumer.
- *An unknown head is bounded when the `unknown` handler in force is a proved document procedure* (and no `namespace unknown` applies): the call's effect is that procedure's.
- *Reads in a body the analysis did not enter are display facts:* the occurrence is recorded, with no version.
- *The Unavailable metadata context is a product state* (a missing overlay abstains); its tests are kept and reach it by the real route. A profile-less registry is a supported input only if production code builds one.
- *Facts inside a never-called procedure.* A fact that depends on a caller's values is a fixture fault (call it). A fact that holds for every entry is kept: an uncalled procedure is still callable, so the walk gives it a generic entry (formals unknown, the command world under the entry-independent conditions, caller frame unknown) and proves what holds for every entry. Tests of that kind are not "fixed" by adding a call. Design with analyser-binding.
- *A fold on the characters of a non-ASCII source literal* declines on 8.4 to 8.6 unless the script's encoding is proved (a script file is read in the system encoding there); on 9.0 and later the default is utf-8.
- *A full constant `[expr {…}]` in a command substitution folds as O101 on every release* (the KCS page stands; a value's internal representation is not observable to a script). A statement-position `expr {2+2}` becomes `expr 4`, which tclsh 8.4 to 9.1 run (verified; the KCS page's "Standalone expression limitation" was stale and is removed).
- *A passing test that cannot fail is vacuous.* Negative tests on a helper that refuses for want of its inputs (48 optimiser library tests build a pass context with no registry or module) check nothing; they are listed in the second pass.
- *O101 declines on 8.4 for a shift count outside the word width.* What one build of tclsh 8.4 printed for undefined C behaviour is not the release's behaviour.
- *Precision at a call site is sound; a procedure's own summary stays pessimistic.* A caller passing a written name to `upvar 1 $n a` has exactly that variable touched; with the interprocedural step off no caller is consulted and the answer is unbounded.
- *A diagnostic about one argument's value is anchored on that argument's word.* The statement is the anchor only when the fault is the command as a whole or the word cannot be located. One site reports once: the pass with the narrower anchor owns it.
- *W304 is silent on a value proved to be a constant that cannot be read as an option.* The dash-literal warning and the unpinned-value information are unchanged.
- *A switch-dispatch `proc unknown` bounds its calls:* a name its switch handles is known; a name that falls to a raising arm draws W123, instead of W123 being off for the whole file.
- *A resolver derived from a pack's frame effect does exactly what `upvar` does* for a computed level: the local exists as a link to an unknown frame.
- *A wall-clock budget in a unit test holds nothing.* It is re-pointed at a count the algorithm controls, bounded by a stated function of input size, or at a scaling check; a user-visible latency budget belongs in a benchmark with the number's source written down.
- *A statement after a proved error in an executed unit yields no facts for proof consumers or the call graph* (no object handles, call edges or definitions); it keeps the editor tier only. Uncalled procedure bodies are not affected.
- *A hardening action is offered only on a proof:* the inserted command is the release's builtin at that site, the registry declares its evaluation effect-free, and the single read stays in place. It is a single, not-preferred action, never in fix-all.
- *Branch conversion keeps its control-flow obligation only where a variable trace cannot be ruled out;* expression bracing beyond the literal subset and dictionary conversion keep theirs until they have evidence.
- *A red test is not turned green by expecting the product's refusal.* Where tclsh shows a minifier rewrite equivalent under conditions the fixture meets (keyword abbreviation over a stock ensemble), the test keeps expecting the shorter text and stays red until the product can issue the receipt; the loss is tracked in the minifier open item. Only a rewrite tclsh shows not equivalent has its expectation changed to the refusal.
- *A Logical class call publishes no Native class record;* the lenient environment's class facts live in the Logical domain, so they cannot reach proof consumers.
- *Whether the workspace may fill in what one document cannot prove depends on the environment.* After `namespace import ::A::*` from a namespace the document cannot see, the next command head is unproved. Under the lenient `tcl` environment the index may use `::A`'s exports from the indexed document (that environment's model is that workspace files are loaded) to order imports and refuse the link tclsh refuses; it stays a navigation fact. Under a pinned release an indexed file that is not proved loaded proves nothing: the fixture proves the load, or navigation lists conditional candidates without claiming an order.
- *Navigation answers under the display policy; edits need proof.* Go to Definition answers a written declaration where the strict world is unknown; inline and rename refuse there. Definition lists the sites that create a variable on some path to the cursor, not a later write to one that exists. A call-hierarchy item for a procedure carries its qualified name, as a member item does.
- *A rename that returns no edits and no reason is a wrong answer.* An expression function name resolving through `tcl::mathfunc` is a reference a rename must edit or refuse on.
- *A test that returns early for want of its oracle is not a pass,* and a hand-written parser beside a shared one is removed: the goal names both. (`substitution.rs` in the tooling crates parses `subst` options by hand beside `option_effect::substitution_kinds`.)
- *Same input, same result, to the byte, is a contract of the analysis.* Incremental reuse and reproducible diagnostics rest on it; a difference between two runs is a wrong answer.
- *Abstaining at `source -encoding` with a decode obligation recorded is sound* when nothing can decode; the command world after it must not stay proved.
- *The generic entry covers a procedure the body defines on every call:* the nested procedure's body gets its own generic entry. It covers that body's facts only; whether the nested procedure exists for an outside caller still depends on whether the outer one surely ran.
- *A `link` with written names puts exactly those names into the receiver namespace.* It refuses the generic entry of an uncalled method body only for those names; a computed name refuses it for every relative head.

## Wrong answers found by the scrub

These go ahead of missing facts once the ledgers are in.

| Finding | Area |
| --- | --- |
| A procedure body rewritten from its one in-file call (`proc add` becomes `return 7`) | dataflow-optimiser |
| A procedure summarised pure although it writes a global (`proc g {} {bump ::n}`) | dataflow-optimiser |
| A call through `interp alias {} via {} far x` not seen as a call of `far`; `return $x` folds to 1 where tclsh prints 2 | dataflow-optimiser |
| `rename list mylist` / `rename llength myll` still fold where tclsh raises | analyser-binding (mutation scan) |
| `string length` of U+1D11E folds to 2 under 8.6 where tclsh 8.6 prints 4 in an iso8859-1 locale | dataflow-optimiser |
| `try {set x [expr {1 / $d}]; error boom} on error {} {}` then `info exists x`: the rewrite drops the `yes` path | values |
| A write made by an `after` callback is folded away (original prints 1, rewrite 0; 8.4.20, 8.6.18, 9.0.4) | values |
| `expr {1 << 70}` folded to 0 under 8.4, where the shift is C's and a count beyond the word width differs by platform and build | values |
| I231 calls a `switch` arm always false when its pattern is written with a backslash escape (`a\nb`); every release runs that arm | diagnostics |
| I231 on a `switch` whose subject a release may read as an option, under a profile that names no release | diagnostics |
| W210 on a variable an existence guard has just set (false positive) | diagnostics |
| W210 at reads after an undefined command, which raises first | diagnostics, with dataflow |
| A procedure recorded under the raw text `::${c}` when its name is a substitution inside a `foreach` | diagnostics |
| Two `helper` procedures created in two child interpreters share one key (`@interp@${s}`); navigation crosses between them | diagnostics |
| False W001 on `dict is_dict` and false W214 after the `dict` ensemble's `-map` is reconfigured | diagnostics |
| A debug assertion in the lexer's variable scan fires on a backslash-continued parameter list (a caller passes an offset that is not at a `$`); a silent misread in a release build | diagnostics to name the caller |
| W304 (information) on a value proved to be a constant that cannot be read as an option | diagnostics |
| IRULE5001 fires where the setup event sets the flag (three tests) | taint |
| IRULE3102 reported inside an event body the rule loader rejects | taint |
| A duplicate eval-sink warning for each procedure | taint |
| IRULE3101 on a path proved to be `/` | taint |
| Nine in the class and object facts area, listed first in `class-facts.md`: a creation recorded after a `proc` in a namespace that does not exist (tclsh stops); a call in an uncalled procedure closed to the in-file callee although a later `proc` or `rename` decides it; a nested `proc` selected as structured, feeding rename's coverage; `struct::graph g` typed as an object with no provider; and five more | class-facts |
| Two analyses of the same file compare unequal (`aliases.tcl`): the analysis is not deterministic there | class-facts to name the field |
| The whole-file walk records `::${ns}::setdef` as a candidate for `${ns}::setdef` | analyser-binding |
| A namespace rename accepted while `after 0 {::old::tick}` and `[list ::old::tick]` keep the old name (tclsh: invalid command name after the rename) | lsp-features |
| The namespace-variable rename hazard check clears a branch-dependent constant that can spell the old name | lsp-features |
| The workspace index installs import links that tclsh refuses (15 tests; feeds definition and the linked references view) | lsp-features |
| Three go-to-definition answers that follow import ordering in a way tclsh contradicts; a property lens that counts `my size`, which raises on 9.0 and 9.1 | lsp-features |
| The stub's declared words are still given to a command after `rename check {}` | analyser-binding |
| The rename family: the lattice keeps a builtin after `rename string {}`; `mystr` reads as opaque after `rename string mystr`; `expr` or `::add` still trusted after a renaming procedure or `rename $command …` runs; a rename inside an evaluated body (`namespace eval`, `apply`, a procedure built from a variable) or behind an unknown-handler alias is not replayed | analyser-binding (with dataflow's rename scan) |
| Body advice inside `proc A::p` where tclsh raises unknown namespace; a `struct::graph` callback receipt with no `package require`; a constructor shape after `unexport new` | analyser-binding |
| Native lowering still emits a definition after `proc` is replaced; `unset` in a callback not counted as a write; a Jim entry lexed with the 8.6 configuration; `[dict size DATA]` inside an unknown command's braced word listed as a nested call | analyser-binding |
| `-force` imports followed to the deleted local (10 tests); an inner namespaced class does not shadow the global one; a mathfunc rename that returns no edits and no reason | lsp-features |
| A helper's condition concluded, and its body rewritten, from in-file callers although another file calls it differently (the body-seed cause above); W210 after a raising `unset x`; `namespace export *` in `namespace eval` scanned as no export boundary | analyser-binding |
| The top-level CFG the taint walk reads is replayed from the file's final state, so one later world-withdrawing statement turns every earlier call into an unseen-call marker | tk-oo writes, analyser-binding reviews |
| Under `tcl`, inside `namespace eval`, a dynamic variable name is resolved to one branch's constant (`set n other; if {$c} {set n v}; set $n 2` claims `other`) | analyser-binding |
| `tcl-lsp analyze` under the SpecTcl dialect reports W123 on a pack's root word where the analyser, rightly, abstains | lsp-features |

## Waiting on the owner's approval

Edits a session's permission classifier refused; the replacement text is in the area's summary.

| What | Session | Why it is right |
| --- | --- | --- |
| Two `rule_declaration` tests re-pointed from a proved target to the declaration-inventory record | irules-registry-tooling | Nothing is proved at rule load (analyser-binding's analysis, ruling of 17 November 23:55) |
| The second assertion of `an_open_pack_keeps_its_notices_while_it_is_edited` re-pointed from W123 to the pack's own notice on the buffer | irules-registry-tooling | A pack's root words belong to its grammar |
| `tcl-spectcl` `a_hover_key_written_outside_the_block_is_still_unknown` re-pointed from W123 to the pack's own notice | irules-registry-tooling | A pack's root words belong to its grammar (the same ruling) |
| The fixture of `exact_custom_return_codes` replaced: its second `try` never runs, because `try {return -code 42} finally {}` leaves the procedure (tclsh) | irules-registry-tooling | The source is not what the test says it is |
| `LIVE_REFERENCE_INLINING` switched on, with the guard test `the_live_compile_splices_no_reference_body` changed to assert that the differential is the gate | live-inlining | The differential is identical to tclsh 8.4 to 9.1 (ruling of 17 November 23:52). Measured on `2e8b3814`, switch off against on, eight packages (`tcl-codegen`, `tcl-spectcl`, `tcl-vm`, three engines, `tcl-cshim`, `tcl-spec-hooks`): the only new failure is the guard itself, the three `reference_bodies` tests are fixed, every other failing set is identical. A fuzz smoke with the switch on is still to run |

## Passing tests the oracle contradicts

Found while triaging the failing ones; they pass today and hold the product to a wrong answer.

| Test | What tclsh shows | Area |
| --- | --- | --- |
| `tcl-compiler` `it_4` `list_wrapped_namespace_unknown_installer_suppresses_w123` and the `…via_namespace_inscope…` twin | A `namespace unknown` set in `::target` does not cover a call from `::`; tclsh raises `invalid command name "mystery_cmd"`, so W123 is right there | diagnostics |

## Families by session

`tcl-compiler` is split by family (crate, target, first modules); a family not listed goes to the
session whose area note names it, and analyser-binding decides the rest.

| Session | Families |
| --- | --- |
| diagnostics | `tcl-compiler` `analyser::diagnostics`, `analyser::handlers`, `analyser::param_traits`, `analyser::state`, `analyser::types`, `analyser::bounds_checks`, `analyser::scope`, `compiler_checks`, `it_4/analyser`, `it_5/checks`, `it_3/fp_depth`, `it_2/core_analyses` |
| analyser-binding | `tcl-compiler` `command_binding`, `unit_scope`, `compilation_unit`, `inline_uplevel`, `dynamic_names`, `deferred_writes`, `word_subst`, `common_aot_plan`, `native_lowering`, `cfg_builder::upvar_info` |
| class-facts | `tcl-compiler` `registry_invocation`, `lowering`, `analyser::oo`, `analyser::original_members`, `object_types` |
| dataflow-optimiser | `tcl-compiler` `optimiser`, `sccp`, `ssa`, `cfg_builder::tests`, `cfg_builder::execution_regions`, `interprocedural`, `native_integer_proof`, `it_3/optimiser`, `it_3/optimiser_depth`, `it_3/side_effects_binding`, `it_5/optimiser_coverage`, `it_5/pipeline_coverage` |
| values | `tcl-compiler` `it_2/value_transfer_witnesses`, `cfg_builder::global_write_info`, `shimmer`, `type_infer`, `value_transfer`, `it_2/var_escape_cfg`, `it_5/var_escape_cfg_catch` |
| taint | `tcl-compiler` `taint`, `taint_interproc`, `irules_checks`, `analyser::irules_event_checks`, `uri_split`, `connection_scope`, `it_3/taint`, `it_4/taint_depth` |
| analysis-performance | `tcl-compiler` `analyser::per_item`, `per_item_corpus`, the performance tests |
| lsp-features | `tcl-lsp-core`; the navigation groups of `tcl-lsp-server` |
| server-e2e-db | `tcl-lsp-server` (end to end and library), `tcl-lsp-db` |
| execution | `tcl-codegen`, `tcl-vm`, `tcl-runtime`, the engines, `tcl-cshim`, `tcl-spec-hooks` |
| irules-registry-tooling | `tcl-cli`, `tcl-spectcl`, `tcl-diagram`, `tcl-explorer`, `tcl-mcp`, `tcl-irules`, `tcl-spec-studio`, the F5 crates |
| tk-oo | the tests its issues name |
| live-inlining | `tcl-spectcl` `codegen_stamps` |

## Where it stands, 17 November 2026, 22:40 UTC

All thirteen ledgers are signed off (the last, Tk and TclOO, at 23:06 UTC with 35 rows of its own): about 3,290 failing tests triaged. The table below is the first twelve.

| Verdict | Tests | Share |
| --- | ---: | ---: |
| `KEEP-ORACLE` | 942 | 29% |
| `KEEP-CONTRACT` | 920 | 28% |
| `KEEP-F5` | 140 | 4% |
| `FIX-FIXTURE` | 622 | 19% |
| `FIX-EXPECTATION` | 201 | 6% |
| `REWRITE` | 290 | 9% |
| `MERGE` | 94 | 3% |
| `RETIRE` | 35 | 1% |
| `UNDECIDED` (ruled, rows being updated) | 8 | |

What the failing tests are worth:

- **62% (2,002) hold behaviour worth making pass**: tclsh or Jim shows it, a contract states it, or it is F5 behaviour tied to a BIG-IP check.
- **25% (823) fail because the test is wrong**: the fixture is not what it claims, or the expectation contradicts the oracle. One fixture fault alone, a registry built with no dialect profile, accounts for well over a hundred across five areas; fixtures that read an unset variable, never install the procedure under test, or use a package without requiring it account for most of the rest.
- **9% (290) hold real behaviour through an internal detail** and are re-pointed at the observable.
- **4% (129) go**: merged into a named survivor or retired with an argument specific to the test.

The scrub also found about sixty places where the product gives a wrong answer today (list below; `analyser-binding.md` ranks 27 of them), and a handful of passing tests that hold the product to a wrong answer or pass for the wrong reason. Wrong answers are fixed before missing facts.

The failing behaviours are far fewer than the failing tests. The causes ranked highest across the ledgers:

1. A procedure body's strict lattice seeded from its in-file callers (wrong rewrites and wrong diagnostics; being removed).
2. `rename` not followed: a builtin still trusted after it is renamed away, in the lattice, the mutation scan and evaluated bodies.
3. Statements with substituted words in a procedure body define nothing (the `info exists` wrong fold; 37 tests).
4. The generic entry of a never-called procedure and the entry-independent binding (74 tests in the compiler, about 42 more in the editor features).
5. A `proc` or a TclOO creation under a namespace that does not exist is not taken as the error tclsh raises.
6. The VM's own compile refuses scripts tclsh runs (21).
7. A class made by a user metaclass publishes no factory (15).

The second pass (overlap among passing tests: 88 exact-copy groups, 385 overlap groups, the vacuous negatives on unwired helpers) follows the repairs.

## Status of the rows, 18 November 2026, 03:55 UTC

From `tools/ledger_status.py` (a cell that is not one of the seven words counts as open).

| Ledger | open | repaired | exposed | fixed | deleted | blocked | routed | Rows |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| analyser-binding | 412 | 5 | 0 | 1 | 0 | 0 | 0 | 418 |
| analysis-performance | 46 | 0 | 0 | 0 | 0 | 0 | 0 | 46 |
| class-facts | 236 | 8 | 2 | 0 | 0 | 0 | 0 | 246 |
| dataflow-optimiser | 365 | 24 | 24 | 7 | 0 | 1 | 2 | 423 |
| diagnostics | 332 | 18 | 38 | 4 | 0 | 0 | 5 | 397 |
| execution | 197 | 4 | 0 | 7 | 56 | 0 | 0 | 264 |
| irules-registry-tooling | 134 | 0 | 0 | 0 | 0 | 19 | 0 | 153 |
| live-inlining | 27 | 0 | 0 | 0 | 0 | 0 | 0 | 27 |
| lsp-features | 636 | 10 | 16 | 5 | 10 | 0 | 0 | 677 |
| server-e2e-db-flaky | 2 | 3 | 0 | 0 | 0 | 0 | 1 | 6 |
| server-e2e-db | 187 | 14 | 1 | 2 | 0 | 12 | 6 | 222 |
| taint | 136 | 5 | 26 | 6 | 0 | 0 | 1 | 174 |
| tk-oo | 58 | 9 | 0 | 3 | 0 | 0 | 0 | 70 |
| values | 195 | 18 | 0 | 4 | 1 | 0 | 0 | 218 |
| **all** | 2963 | 118 | 107 | 39 | 67 | 32 | 15 | 3341 |

## Progress

Updated at the hourly check from the ledgers on `work`.

| Session | Failing tests to triage (about) | Rows in ledger | Sampled by the coordinator |
| --- | ---: | ---: | --- |
| diagnostics | 397 | 397, complete (KEEP-ORACLE 143, KEEP-CONTRACT 94, FIX-FIXTURE 62, REWRITE 36, FIX-EXPECTATION 33, KEEP-F5 17, UNDECIDED 6 ruled, RETIRE 5, MERGE 1) | signed off 21:20 UTC |
| analyser-binding | 418 | 418, complete (KEEP-CONTRACT 140, FIX-FIXTURE 98, KEEP-ORACLE 94, REWRITE 50, FIX-EXPECTATION 22, KEEP-F5 12, RETIRE 1, MERGE 1); 27 wrong answers; one ranked list merging the routed causes | signed off 22:38 UTC |
| class-facts | 246 | 246, complete (KEEP-ORACLE 74, FIX-FIXTURE 67, KEEP-CONTRACT 43, REWRITE 40, FIX-EXPECTATION 14, UNDECIDED 4 ruled, KEEP-F5 2, RETIRE 1, MERGE 1); BIG-IP checks CF-01, CF-02 | signed off 21:30 UTC |
| dataflow-optimiser | 423 | 423, complete (FIX-FIXTURE 142, KEEP-CONTRACT 103, KEEP-ORACLE 82, REWRITE 50, FIX-EXPECTATION 30, MERGE 10, RETIRE 5, KEEP-F5 1) | signed off 20:55 UTC |
| values | 218 | 218, complete (KEEP-ORACLE 123, KEEP-CONTRACT 32, KEEP-F5 30, FIX-FIXTURE 20, FIX-EXPECTATION 6, REWRITE 6, RETIRE 1) | signed off 21:07 UTC |
| taint | 171 | 174, complete (FIX-FIXTURE 81, KEEP-CONTRACT 38, REWRITE 18, KEEP-ORACLE 18, KEEP-F5 13, MERGE 4, RETIRE 1, FIX-EXPECTATION 1); BIG-IP checks 049, 071, 072, 073 | signed off 21:50 UTC |
| analysis-performance | 46 | 46, complete (KEEP-CONTRACT 41, REWRITE 4, UNDECIDED 1 to diff) | signed off 21:33 UTC |
| lsp-features | 672 (492 core, 177 routed from the server, 3 preview tickets) | 672, complete (KEEP-ORACLE 220, KEEP-CONTRACT 155, FIX-FIXTURE 149, FIX-EXPECTATION 84, REWRITE 35, RETIRE 13, UNDECIDED 7 ruled, KEEP-F5 5, MERGE 4) | signed off 22:35 UTC |
| server-e2e-db | 216 (plus 214 routed: 181 navigation to lsp-features, 33 memo to analysis-performance) | 216, complete (KEEP-CONTRACT 163, KEEP-ORACLE 29, MERGE 10, KEEP-F5 10, REWRITE 2, FIX-FIXTURE 1, UNDECIDED 1 ruled) | signed off 21:12 UTC; three publication faults first |
| execution | 264 (23 routed to live-inlining) | 264, complete (KEEP-ORACLE 109, KEEP-CONTRACT 56, MERGE 49, REWRITE 33, RETIRE 8, FIX-EXPECTATION 4, FIX-FIXTURE 3, KEEP-F5 2) | signed off 21:20 UTC, after the receipt-ordering retirements were redone test by test (26 back to KEEP-CONTRACT) |
| irules-registry-tooling | 151 | 151, complete (KEEP-F5 48, KEEP-CONTRACT 47, KEEP-ORACLE 40, MERGE 15, FIX-FIXTURE 1); BIG-IP checks TS-01 to TS-10 | signed off 21:00 UTC |
| tk-oo | 63 (28 cross-listed from lsp-features and class-facts, not counted twice) | 35, complete (KEEP-ORACLE 20, FIX-FIXTURE 12, KEEP-CONTRACT 3) | signed off 23:06 UTC |
| live-inlining | 29 | 29, complete (REWRITE 13, KEEP-CONTRACT 8, FIX-EXPECTATION 4, KEEP-ORACLE 2, FIX-FIXTURE 1, MERGE 1); the splice matrix (1,449 VM pairs, 20 minutes) to become 142 cells holding the same shapes | signed off 21:45 UTC |
