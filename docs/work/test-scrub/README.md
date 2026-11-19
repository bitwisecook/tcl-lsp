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
- **A diagnostic in a body is not raised from one caller's argument.** `proc f {d} { dict update d k v { puts $v } }` with the one call `f {}` raises at that call, but W210 in `f`'s body would be a claim about every caller, so it is silent there (the body's lattice is not seeded from in-file callers, and a body-level fact holds for every entry). A test of the SCCP path is re-pointed at a constant that arrives from an earlier block of the same body; a proved-raising call is reported at the call word if a diagnostic for that exists, and is otherwise written up as a gap, not invented in a test repair (18 November, 05:30 UTC).
- **A hint that claims absence needs the absence proved.** IRULE5001 reports an ungated `log`; a gate variable counts when an event assigns it. Where an event body holds a statement lowering could not lay out (under 8.4 rules `switch $mode {…}` has no single layout, tclsh 8.4 reads a `-`-leading subject as an option), the flag index is open and the hint accepts the gate. The check changes, not switch lowering; a `switch -- $mode` twin pins the proved case and a control keeps the hint firing where nothing is unlowered. Whether TMM reads the subject as 8.4 does is a BIG-IP check (18 November, 05:35 UTC).
- **A proved load is a proof, lexical advice is not, and neither borrows the other.** `allows_lexical_declaration_advice` is the lenient `tcl` domain only and stays so. Under a pinned release a `source` edge the index resolves is a proof (tclsh agrees the override family is then real), but nothing yet carries it to cross-file method references: a missing fact, behind every wrong answer. Server rows about sibling files with no load move to lenient `tcl`; two or three stay in the `source`-proved pinned form as witnesses of the gap and the rest merge into them; the gap is written up under `outstanding/` (18 November, 05:50 UTC).
- **A witness whose oracle is tclsh does not run under f5-irules with a source that is not a rule.** A rule's top level admits only `when`, `proc`, `priority` and `timing`; a fixture with a top-level call is not an iRule, and a rule procedure has no proved execution target at load. Such witnesses take the list of dialects whose oracle is a Tcl core (narrowed at the witness, row by row), and rule-procedure summaries get iRule-shaped tests, a `proc` invoked from a `when` body, asserted as hosted advice. A top-level call in an iRule is not modelled (18 November, 07:55 UTC).
- **An instance is published only where its factory's package is proved present.** Without the owning package ambient in the environment or loaded by an unconditional `package require` (or another proved load) earlier on the walk, the identity of a factory word such as `ttk::treeview` is unproved: tclsh 8.4.20 to 9.1.0 raise `invalid command name`, and if the next statement is reached something else supplied the name. No instance is published; W120 and its fix are unchanged; a conditional or caught require is not a proved load; the call is not treated as a proved error. This is the compiler agreeing with the display tier's existing gate. The rule lands together with the repair of every fixture that used Tk without loading it. The same principle for all package-owned commands is written up, not built (18 November, 09:10 UTC).
- **A `classmethod` and a `method` of one name are two slots, and the order decides.** tclsh 9.0.4 and 9.1.0: with `method foo` then `classmethod foo`, both `[C new] foo` and `C foo` reach the classmethod and the method declaration is superseded; with `classmethod foo` then `method foo`, the instance reaches the method and the class the classmethod, two live members with distinct reference sets. A superseded declaration keeps a lens marked as superseded and has no reference set; tests carry both orders with the transcript; a rename runs the source on tclsh before and after and asserts the same output at each renamed site, or the product refuses (18 November, 09:25 UTC).
- **A scaling bound is the reference's own shape.** Scanning nested bodies is bounded at 4.0× counted steps per doubling of depth (quadratic): the source has that much body text, and tclsh 8.6.18 and 9.0.4 evaluate nested `if {1} {` at 4× per doubling (5 ms at 500, 19 ms at 1,000) and refuse depth 2,000 with `too many nested compilations`. Cubic was the fault. A survival test may exceed the oracle's limit on purpose and then asserts survival only (18 November, 09:30 UTC).
- **A captured alias word gets no anchor at the call, and a real error is reported at its producer.** This amends `diagnostic-registry-source-ownership.md`, whose provider answers were `not-tested` and whose text had captured slots decline every source diagnostic. tclsh shows five of six such calls raise (`interp alias {} selected {} string indez; selected abc 0`: `unknown or ambiguous subcommand "indez"`). The slot at the call still receives no anchor and no fix; the finding is anchored on the written word in the `interp alias` definition, once per producer word, only where a proved call reaches it, with no fix. Each test changes only on its own transcript, per dialect; the contract text and its provider answers are amended in the same commit. Confirmed by the owner on 19 November.
- **Review each proposed refactor under the selected dialect.** For the nine historical `switch` to `dict` rows in `refactor-original-source-rewrite-permissions.md`, the owner directs lsp-features to amend the contract and test the strongest provably correct action per fixture. A dictionary covering all possible subjects discharges the missing-key obligation. A provably unobservable final store can relax the value obligation. Preserve observable completion, errors, traces, evaluation effects and results. In particular, a reachable `dict get` error is observable even for a dead destination. Enable the edit where equivalence is proved and witness it before and after on the applicable oracle. Where a proof is missing, expect a disabled action with its reason and keep the previous expectation in the ledger (owner, 19 November).

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
| The ordering fixture reports more warnings than expected. On the clean baseline each procedure has exactly one T100 at its `eval` operand. On C 8.4–8.6 and Jim its separate `exec` operand has both T100 (code execution) and T102 (option injection). The C 9.0/9.1 selections emitted only the two `eval` diagnostics across the fixture. The C 9 `exec` coverage question remains open. Check order and multiplicity by code, operand and range in the selected public modes. | taint, reclassified 19 November |
| IRULE3101 fixture used a top-level fragment and strict SCCP inputs. In the selected public `HTTP_REQUEST` analysis, a proved `/safe` emitted zero warnings. A literal `unsafe`, an unknown result and a branch choosing between them each emitted one. Exercise these at the legal handler entry with the document’s semantic facts. | taint, fixture reclassified 19 November |
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
| The top-level timeline fix above landed as `29547433` (a later unknown command no longer makes earlier calls unseen) | tk-oo, fixed |
| `switch -exact -- $x {…}` (and `-glob`, `-regexp`) read as a possible-option subject, lowered to a call and withdrew the command world, so every `if`/`while` after it lowered to a call too; fixed in `f6d942f3`, 48 `tcl-compiler` tests | class-facts, fixed |
| An `if`'s else body span stopped before its closing brace, so a constant-false `if` with an `else` never folded (tclsh 8.4 to 9.1 print `bye here`); fixed in `72eb66a1` | dataflow-optimiser, fixed |
| Scanning a `when` body is about quadratic in nesting depth (`find_enclosing_when_event` / `scan_file_events`: depth 200 3.3 s, 400 9.2 s, 800 26 s, 1,200 56 s per call), so the 2,000-deep test does not finish. Not a regression: the same on every commit tried back to 17 November 13:41 | analysis-performance, after #2543; a count-based scaling witness first |
| Arm bodies of a `switch`/`case` written as separate words stopped before the closing brace, so their selection never folded; fixed in `019cdef1` | dataflow-optimiser, fixed |
| Under 8.4 a runtime-raised coded error (`expr {sqrt(-1)}`, NaN operands) reached `catch` with `errorCode` `NONE`, where tclsh 8.4.20 gives `ARITH DOMAIN {domain error: argument not in valid range}`; fixed in `f03e110a` | execution, fixed |
| IRULE5001 reported a `log` gated by a flag set in the arms of a `switch` lowering could not lay out | taint |
| The authored F5 expression parser reads `abs(077)` as 77; under the owner's 8.4 expression ruling it is 63 (tclsh 8.4.20) | execution |
| O103 folds `[answer]` to 42 inside the uncalled `f` in `proc answer {} {return 42}; proc f {} {set y [answer]}; unknown_future_entry`, although the unknown command may rebind `answer` before `f` runs | dataflow-optimiser |
| A stray `nop` before an invoked command in `foreach`/`while`/`if` bodies, where tclsh 8.6.18 and 9.0.4 compile no instruction; fixed in `2be09036` | execution, fixed |
| `proc p {} {if {$a} {$c}}` lacks the conversion `nop` tclsh 8.6 emits before `jumpFalse1` | execution |
| An authored F5 source-word provider seals words with a native 8.4 string protocol inside a C9 host engine, so `concat` and list copy refuse them (25 `tcl-irule-test` tests) | execution |
| Under tcl8.4 the optimiser rewrites `proc f {args} {return $args}; puts [f # a]` to `puts {{#} a}`; tclsh 8.4.20 prints `# a` for the original and `{#} a` for the rewrite (`args` was joined without the release's quoting at three sites); fixed in `ff48bc25`, held by `release_optimiser_safety` (69 of 69) | dataflow-optimiser, fixed |
| A class is recorded from a statement after a proved error (`error boom; oo::class create ::Wrong {}`; also after a `proc` into a missing namespace); fixed in `c0daade7` | class-facts, fixed |
| Incoming Calls reported a call made inside a class method as coming from the top level; fixed | lsp-features, fixed |
| `proc add` → `return 7`, cause found: the declared-frame collector skips a body that already runs in its own frame, and a single call binds the procedure there with that call's arguments; fixed in `17abcf5d` (a body's expression is read in its declared frame; tclsh 8.4.20, 8.6.18 and 9.0.4 return 2 for `add 1 1`) | analyser-binding, fixed |
| With `proc h {p p(key)} {return $p}`, latest Jim (0.84-15-g2f8e155) raises at `h 1 2`. Suppress W210 on an unreachable following read. The actual argument matters. `h {} 2` returns `key 2` and `h {a b} 2` returns `a b key 2`, so a following read is reachable in both controls. | diagnostics |
| After `rename C M`, a `C create b` inside `catch` is still bound to the class (tclsh 8.6.18 and 9.1.0: `invalid command name "C"`); the constructor receipt at the same site rightly says no | analyser-binding |
| The declared-frame view drops entry-independent facts (nested `uplevel 1` reach, `uplevel #0` global writes, alias cells, interprocedural taint sinks), so switching called bodies to it breaks 66 tests; the `info exists` fold waits on it | `dataflow` issue session |
| Two `tcl-compiler` tests fail only in the parallel run and pass alone (`command_binding::source_callable_static_tests::static_formals_update_one_retained_slot_across_actual_calls`, `source_command_world::tests::original_opaque_jim_callable_name_does_not_gate_ascii_static_capture`): state shared between analyses in one process, which in the server would be one document answering for another | analysis-performance finds the cache and the missing key |
| A procedure's `if {$a}` / `while {$a}` condition jumps without the conversion tclsh compiles (8.5 `tryCvtToNumeric`, 8.6+ `nop`); fixed 95cac8dc | execution |
| A script ending in `return $x` lacks the final `done`, and 8.6+ procedures end `done done` where tclsh trims one; fixed 8c24d37c | execution |
| 8.4/8.5 top-level scripts reach a literal scalar through `storeStk`/`loadStk`/`incrStkImm` where tclsh uses the scalar stack forms; fixed c9a0e5b6 | execution |
| An iRule `expr` reads numerals by the host engine's 9.0 rules (`expr {077}` gives 77, tclsh 8.4.20 63) because the framework's `::expr` override evaluated under the host catch; fixed f474416f | execution |
| 8.5 top-level script: no `startCommand` before a compiled `return` after another command (`set y 1; return $x`) | execution, open |
| 8.6+: an unreachable `jump` after a `done` stays a jump where tclsh fills the jump width with no-ops (two on 8.6 and 9.0, five on 9.1) (`if {$a} {return 1}; return 2`) | execution, open |
| 8.6+: `if {!$a}` compiles `not; jumpFalse` where tclsh gives `nop; jumpTrue` | execution, open |
| 8.4 errorInfo for `expr {0b10 + 1}` depends on entry. C Tcl 8.4.20 says “while executing” for a script file, `source`, `eval $src` and `uplevel #0 $src`. It says “while compiling” for the literal inside `catch`. On the normal VM `source` gives “while compiling”. Direct-file, literal catch, eval and uplevel controls matched their corresponding C Tcl entries. | execution, open |
| O129 rewrites `set x [split ""]` to `set x {{}}` and `puts [llength $x]` to `puts 1`; tclsh 8.4.20 to 9.1.0 print 0. #2418 back, through the optimiser's constant-fold engine. Witness `procedure_summaries::split_of_the_empty_string_is_the_empty_list` | dataflow-optimiser. The combined public optimiser now rewrites the exact incident to `set x {}; puts 0` on all five C releases and `set x {}; puts [llength $x]` on Jim. The originals and rewrites print `0` with normal completion on all six. The procedure variant also agrees. On `8d6ae9adde` the named `tcl-compiler::it_2` regression passed with every oracle required. |
| Strict local rename goes past a computed command head that may read the local by name: `proc m {w dir} { $w ${dir}view }` renames `dir` although `proc peek {args} {upvar 1 dir d; return $d}` then fails (tclsh 8.6.18, 9.0.4) | diagnostics (`analyser/types/variable_renames.rs`) |
| I230 "condition is always true" for `if {$g == 5}` after a stubbed body-role call that sets `g` (`db_eval {select 1} {set g 6}`); tclsh prints `other`. Witness `unseen_code::a_stub_that_states_its_frame_effect_is_a_named_head` | diagnostics / dataflow. The combined public analysis now drops the false claim for the script-bearing procedure and preserves it for the bodyless control under `own` and `none`. Strict reachability remains unchanged across all eight cases. On `8d6ae9adde` the named integration test got past the original I230 assertion, then failed in its later fixture lookup with `fixture body "::p" absent`. Keep that case open. |
| `puts [lrepeat 0 x]` folds to `puts {}` on C Tcl 8.5, which raises. The same folder accepts `08` and reads `010` as ten on 8.5/8.6. Both releases reject `08` and read `010` as eight. Reproduced through the production optimiser on `079afe5230`. | dataflow-optimiser, open |
| Direct `puts [list # a]` and `puts [split {#a} {}]` acquire braces around the leading hash on C Tcl 8.4.20. The earlier green `set` fixture covers a different call path. | dataflow-optimiser, open |
| Jim `puts [dict merge {a 1 a 2} {}]` returns `a 2`. The rewrite preserves the original duplicate-key text. C Tcl 8.5–9.1 preserve that text. | dataflow-optimiser, open |
| Registry folders also change 8.6 non-BMP list ordering, `string repeat x 010` and `08`, `format %f 010`, and default `string trim` on vertical tabs. Rewrites and release controls are in the [parser audit](../name-handling/parser-audit.md#product-reproductions-to-fix-first). | dataflow-optimiser |
| Definition misses a braced semicolon command. BIG-IP document symbols drop a quoted name and its sibling. `regsub` and `scan` hints, format/binary hovers and a quoted-string comparison action read different grammars from the selected command. | lsp-features |
| Spec Studio publishes a false closed vocabulary and invents an option from constant text. The SpecTcl loader drops a supported domain when an inner list element has redundant braces. A generated venv wrapper splits a single library path containing a space into two elements. | irules-registry-tooling |
| VM Thread pool wait accepts an invalid job ID or malformed list. Shared-variable increment misreads hex and octal. Verified with Thread 2.8.13 on C Tcl 8.6.18. | execution |
| Public analysis skips unknown Tk options containing literal dollars or brackets in braced words and offers a W110 comparison edit past an invalid leading non-breaking space in an expression. | diagnostics, tk-oo |
| The auto-path evaluator returns a joined path for a two-command script and reads `$a:b` as the scalar `a:b`. Both disagree with the public compiler API versus Tcl comparison. | analyser-binding |

### Additional public witnesses from the parser migration

| Public entry and observation | Owner and current verification |
| --- | --- |
| The native Rust runtime accepts malformed or unknown `glob -types` on `/dev/null`. Its filesystem glob also misses alternatives for `glob -nocomplain -- {{alpha,beta}}` and an escaped literal in `{alp\ha}` with files `alpha` and `beta`. | Execution. All five C releases reject the invalid type input. Jim rejects the unsupported `-types` option. All six match the brace and escape filesystem controls. Keep string-pattern semantics and pathname expansion separate. |
| Malformed runtime `try` handlers produce the right message but lose the trap failure's structured error code. | Execution. Reproduced via the real Rust runtime ABI. Native C 8.6, 9.0 and 9.1 supply `TCL OPERATION TRY TRAP EXNFORMAT`. The candidate delegates to the registry's existing cause. |
| Native Thread 2.8.13 rejects a list append to a cell holding an unmatched brace and preserves the cell. Appending `#foo` to the list `a` gives `a #foo`. The VM previously appended to the malformed cell and quoted the leading hash as the first element of a fresh list. | Execution. A candidate parses and writes the complete selected list. Keep the normal VM regression open until the candidate run. |
| The public optimiser previously changed Jim's output spelling for `dict create x {a] b}`. Duplicate keys and merge spelling exercise the same selected dictionary writer. | Dataflow. The constructor candidate passed 24 public optimiser executions across four cases and all six interpreters. Other producers, including Jim `dict keys`, `dict values`, `split`, `lreverse` and `lsort`, still need their own output checks. |
| The public list optimiser changes original padding on some empty/singleton `lreverse` and `lsort` inputs. | Dataflow. Reproduced for C release-specific `lreverse` and latest Jim `lsort`. Retain the selected original spelling wherever the native command does. |
| In the proposed `if`→`switch` rewrite, a literal last comparison with `"default"` becomes the fallback. With keys `a`, `default` and `z`, the selected source returns `one`, `two` and empty. The unsafe switch returns `two` for `z`. | LSP features. All six interpreters agree on these results and on the explicit trailing empty fallback control. Check the actual code action, a literal dash body, preserved body bytes and selected command bindings before enabling a rewrite. |
| The later I230 callback control claims `$done` is always false after `after 10 {db_bind {done}}`, `after 50` and `update` with a caller-frame stub. | Diagnostics and dataflow. All five C Tcl releases and latest Jim print 1 for `proc db_bind {spec} {upvar 1 done d; set d 1}` and 0 for a no-op definition. The fixture now retains its real authored procedure. Keep the callback wrong answer open separately from the synchronous body-role case. |
| Three SpecTcl state-transition fields accepted a valid prefix before an unterminated list quote and published partial transitions. | Tooling. The selected pack grammar applies to the entire descriptor. All five C releases reject the malformed lists. Jim accepts them under its own list grammar. The updated pack loader and its public controls await execution. |
| The public optimiser rewrites `string compare -- a b` and `string equal -- a a` to values where all six native commands raise. Other wrong results include C 8.x `-length 010` and `08`, C 8.5/8.6 `string is double 08`, and Jim `string is boolean t`. | Dataflow. The 108-case differential found 21 changed outcomes across six case shapes. Use the selected command and scalar owners, then check each rewrite through the optimiser. |
| Public member rename edits a TclOO method declaration and direct call, leaving a saved command prefix with the old method spelling. The issue reproduces for `{m;n}`, `{m n}` and `{m(n)}`. | LSP features. Reproduced on combined `8d6ae9adde` for C Tcl 8.6, 9.0 and 9.1. The same saved prefix with an unpunctuated name correctly blocks the edit. |
| Spec Studio inferred a compulsory option operand from text like `{lindex arbitrary}` or a comment containing `incr x` in a switch arm. | Tooling. The selected branch and the missing-argument outcome must prove arity. The coordinator's ruling and public controls are recorded in current-state. |

<a id="waiting-on-the-owners-approval"></a>

## Owner-approved edits

On 19 November the owner approved every tooling edit listed below and the live-inlining switch and guard change. The table records their scope. The owning session records implementation and verification in its ledger. The [owner decisions](../name-handling/current-state.md#owner-decisions-and-restart-19-november-2026) govern the restart.

| What | Session | Why it is right |
| --- | --- | --- |
| Two `rule_declaration` tests re-pointed from a proved target to the declaration-inventory record | irules-registry-tooling | Nothing is proved at rule load (analyser-binding's analysis, ruling of 17 November 23:55) |
| The second assertion of `an_open_pack_keeps_its_notices_while_it_is_edited` re-pointed from W123 to the pack's own notice on the buffer | irules-registry-tooling | A pack's root words belong to its grammar |
| `tcl-spectcl` `a_hover_key_written_outside_the_block_is_still_unknown` re-pointed from W123 to the pack's own notice | irules-registry-tooling | A pack's root words belong to its grammar (the same ruling) |
| The fixture of `exact_custom_return_codes` replaced: its second `try` never runs, because `try {return -code 42} finally {}` leaves the procedure (tclsh) | irules-registry-tooling | The source is not what the test says it is |
| `LIVE_REFERENCE_INLINING` switched on, with the guard test `the_live_compile_splices_no_reference_body` changed to assert that the differential is the gate | live-inlining | The differential is identical to tclsh 8.4 to 9.1 (ruling of 17 November 23:52). Measured on `2e8b3814`, switch off against on, eight packages (`tcl-codegen`, `tcl-spectcl`, `tcl-vm`, three engines, `tcl-cshim`, `tcl-spec-hooks`): the only new failure is the guard itself, the three `reference_bodies` tests are fixed, every other failing set is identical. Re-measured 05:27 UTC on 18 November at `1a99ba86` with every oracle required: `tcl-codegen` 110 off, 111 on of 1,488; `tcl-spectcl` 21 off, 18 on of 625; the same test-by-test result. Jim units splice nothing (`build_inlinable_map` refuses a module whose parameter grammar is not Tcl's). The fuzz run so far loads no pack; a pack-loading campaign is being added |
| Six merges, test into survivor: e2e `diagnostic_matrix::expr_not_redefined_still_folds` into `constant_expr_folds`; e2e `dialect_gating::issue_968_version_gated_math_function_dual_fires_end_to_end` into `undotted_vscode_language_ids_resolve_to_the_same_dialect_end_to_end`; e2e `hover::var_hover_shows_refs` into `hover::var_hover` (the survivor gains the `reference` assertion first); `tcl-irules` `walker::tests::extracts_refs_nested_in_body_and_command_substitution` into `object_refs::extract_irules_refs_nested_in_body_and_command_substitution`; `tcl-cli` `inspection_verbs::smoke_explore_reports_summary_for_tiny_snippet` into `explore_summary_lists_views`; e2e `recovery::c5_break_then_fix` into `c5_rapid_toggling_stays_consistent`. One rewrite: the seven `tcl-registry` `native_jim_lookup::namespace_helper_source_tests` `*_helper_descriptor_keeps_…` tests become one table over `NativeJimNamespaceHelperKind::ALL` that keeps every per-kind row | irules-registry-tooling | Sampled and signed off by the coordinator (rows in `ledger/irules-registry-tooling-passing.tsv`); that session's permission check asks the owner before it edits a test body, and the edits are not routed through another session |
| Eight `tcl-diagram` fixtures repaired: dynamic words read unset variables (`$code`, `$options`, `$status`, `$pattern`, `$ready`, `$items`; tclsh 8.6.18 and 9.0.4: `can't read "code": no such variable`, the rest of the script is dead) or statements follow a certain exit, so the code under test is never reached (names in `ledger/irules-registry-tooling.tsv`) | irules-registry-tooling | The source is not what the test says it is |
| In the four crates no session owns (`tcl-lexer`, `tcl-syntax`, `tcl-dialect`, `tcl-core-types`; 2,008 run, all pass with every oracle required, no expectation contradicted): 16 merges into named survivors (five exact copies and eleven single-operator parser tests the `expr_parser_coverage` tables already state), one vacuous test (`word_parts::tests::oracle_sheet_is_recorded`), seven Jim tests that return silently without the oracle moved onto the shared locator, four weak-assertion groups rewritten to assert a value, nine crash guards given a value to assert (rows in `ledger/unowned-passing.tsv`; about 1,600 tests in small groups were not read) | irules-registry-tooling | Sampled and signed off by the coordinator; test-body edits |
| `tcl-cli` `spec_test_presence_keeps_literal_colons_and_wildcards`: its Jim leg uses a `pkgIndex` layout Jim does not load, which showed once the Jim oracle was required; the fixture takes Jim's own package layout | irules-registry-tooling | A test that returns early for want of its oracle is not a pass |

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

## Status of the rows, 19 November 2026

On clean `a6c66517c7`, `tcl-compiler` integration target `it_2` ran 501 cases in isolated processes with every oracle required. There were 359 passes, 142 failures and zero skips. The two named O129 and I230 regressions are among the failures. The compiler library ledger reconciliation retained all 402 analyser-binding failures, found two passing class-facts rows among 193, and retained all three Tk/OO failures. The three Tk/OO rows in core integration target `it_5` still fail. The separate compiler library ran 10,322 cases on that head with 8,863 passes, 1,457 failures and two skipped incremental/full fuzz controls. The completed ten-binary package run listed 13,166 cases, ran 13,160, passed 11,289, failed 1,871 and skipped six in 1,347.419 seconds. The complete class-facts ledger has four passes and 227 failures across 231 rows. All 404 analyser-binding ledger rows failed, including the two codegen cases. [Exact failing selectors](../name-handling/lists/compiler-it_2-failing-2026-11-19.txt)

Recounted from the committed ledgers at clean `origin/work` head `2ea0ea7fd5` with `tools/ledger_status.py --check`. These are recorded dispositions. Fresh oracle-required runs verify which tests pass at the restart. The 18 November pause table contained 3,397 rows.

| Ledger | open | repaired | exposed | fixed | deleted | blocked | routed | Rows |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| analyser-binding | 404 | 5 | 0 | 9 | 0 | 0 | 0 | 418 |
| analysis-performance | 54 | 0 | 0 | 0 | 0 | 0 | 0 | 54 |
| class-facts | 231 | 8 | 2 | 6 | 0 | 0 | 0 | 247 |
| dataflow-optimiser | 323 | 39 | 40 | 18 | 0 | 2 | 2 | 424 |
| diagnostics | 232 | 62 | 64 | 13 | 19 | 1 | 9 | 400 |
| execution | 190 | 11 | 0 | 13 | 56 | 0 | 0 | 270 |
| irules-registry-tooling | 120 | 0 | 0 | 4 | 0 | 28 | 0 | 152 |
| live-inlining | 18 | 0 | 0 | 0 | 0 | 9 | 0 | 27 |
| lsp-features | 473 | 29 | 54 | 52 | 10 | 32 | 33 | 683 |
| server-e2e-db-flaky | 3 | 3 | 0 | 0 | 0 | 0 | 1 | 7 |
| server-e2e-db | 164 | 14 | 1 | 19 | 0 | 11 | 25 | 234 |
| taint | 132 | 5 | 26 | 10 | 0 | 11 | 1 | 185 |
| tk-oo | 61 | 9 | 0 | 3 | 0 | 0 | 0 | 73 |
| values | 6 | 1 | 23 | 11 | 1 | 0 | 178 | 220 |
| **all** | 2411 | 186 | 210 | 158 | 86 | 94 | 249 | 3394 |

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
