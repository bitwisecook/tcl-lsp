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
3. `RETIRE` and `MERGE`, in commits of their own that name the rows; a merge only after the survivor asserts at least as much.
4. `REWRITE`, re-pointed at the named observable, by cause.
5. The `KEEP` causes, ranked by tests blocked.

The ledger gains a `status` column (open, repaired, fixed, deleted) and is kept current, so the number of distinct failing behaviours can always be read from it.

## Rulings made during the scrub

- *Tests of `inlining::inline_module`* (23, no production caller): read per test by the live-inlining session. "The inlined program answers as the original" is `KEEP-ORACLE`; an assertion on the inliner's internal shape is `REWRITE` to the differential or `MERGE` into a `live_differential` row; an assertion that names the renamed locals is `FIX-EXPECTATION`, since they become anonymous slots.
- *A procedure body is never rewritten from call-site facts.* The procedure stays callable with other arguments from another file, a later `source`, a callback or an interactive session. The call site may fold; the body may not. The product does this today (`proc add {a b} {...}` becomes `return 7` from its one in-file call): a wrong rewrite, top of the dataflow fix list.
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

## Progress

Updated at the hourly check from the ledgers on `work`.

| Session | Failing tests to triage (about) | Rows in ledger | Sampled by the coordinator |
| --- | ---: | ---: | --- |
| diagnostics | 405 | 0 (started 21:00 UTC) | |
| analyser-binding | 330 | 0 | |
| class-facts | 250 (with the 44 class-factory tests) | 0 (started 20:45 UTC) | |
| dataflow-optimiser | 423 | 423, complete (FIX-FIXTURE 142, KEEP-CONTRACT 103, KEEP-ORACLE 82, REWRITE 50, FIX-EXPECTATION 30, MERGE 10, RETIRE 5, KEEP-F5 1) | signed off 20:55 UTC |
| values | 218 | 218, complete (KEEP-ORACLE 123, KEEP-CONTRACT 32, KEEP-F5 30, FIX-FIXTURE 20, FIX-EXPECTATION 6, REWRITE 6, RETIRE 1) | signed off 21:07 UTC |
| taint | 190 | 0 | |
| analysis-performance | 30 | 0 | |
| lsp-features | 500 | 0 | |
| server-e2e-db | 216 (plus 214 routed: 181 navigation to lsp-features, 33 memo to analysis-performance) | 216 (KEEP-CONTRACT 151, UNDECIDED 50, KEEP-F5 9, KEEP-ORACLE 6) | part 1 read 21:00 UTC; lower-level twins, tclsh runs and the never-publishing cause to do |
| execution | 286 | 261 kept, 25 routed (KEEP-ORACLE 108, MERGE 48, RETIRE 36, REWRITE 32, KEEP-CONTRACT 28, FIX-EXPECTATION 4, FIX-FIXTURE 3, KEEP-F5 2) | G1 to G15 accepted; the 31 receipt-ordering retirements rejected (26 of 36 RETIRE rows cite a contract in the test) and sent back |
| irules-registry-tooling | 151 | 151, complete (KEEP-F5 48, KEEP-CONTRACT 47, KEEP-ORACLE 40, MERGE 15, FIX-FIXTURE 1); BIG-IP checks TS-01 to TS-10 | signed off 21:00 UTC |
| tk-oo | 60 | 0 | |
| live-inlining | 29 (23 inliner tests routed from execution) | 23 routed rows, unread | |
