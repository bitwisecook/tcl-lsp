# Test scrub: the unowned crates' passing tests

`tcl-lexer` (815 tests), `tcl-syntax` (926), `tcl-dialect` (217) and `tcl-core-types` (53): 2,011 tests, none failing, no session. Ledger: [`ledger/unowned-passing.tsv`](ledger/unowned-passing.tsv), 43 rows, written 18 November 2026. Nothing is deleted or edited; the ledger is for sampling.

## What was run

- The inventory (`tools/scrub_inventory.py`, 37,531 tests) gave 12 clusters touching these crates (5 exact, 7 overlap, 25 tests). Every body was read.
- A static scan of all 2,011 bodies for tests that assert nothing (17 after discounting helpers that assert) and for tests that return early (read case by case).
- The four crates' whole suites with the oracles required (`bash scripts/dev/ensure-jim-oracle.sh`, `TCL_LSP_JIMSH` exported, `TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1`): 2,008 run, 2,008 pass, 1 skipped (the `#[ignore]`d test below).
- Every value a test states as "tclsh: …" re-run on tclsh 8.4.20, 8.6.18 and 9.0.4, and the two oracle comments of the exact clusters on those and on Jim 0.84.

## Counts by verdict (43 rows)

| Verdict | Rows |
| --- | ---: |
| `KEEP-CONTRACT` | 22 |
| `KEEP-ORACLE` | 15 |
| `MERGE` | 5 |
| `VACUOUS` | 1 |

## The clusters, largest first

The clusters here are small: the largest has three members.

1. `vectors_match_real_tclsh` x3 (command, namespace and variable resolution conformance): three runners over three vector files. The repeated part is the renderer and runner, not the claim. Kept, `KEEP-ORACLE`; each asserts its release list is non-empty, so a missing tclsh fails.
2. `tagging_matches_whether_the_releases_disagree` x2 (exact copies of the same code over two vector files). Kept: different inputs.
3. `expand_prefix_before_braced` / `expand_newword_is_true_after_expand` (exact): `MERGE` into the second, which names the invariant.
4. `expr_string_is_opaque` / `expr_paren_string_interior_is_opaque` (exact): `MERGE` into the second, which names the oracle (`expr {"a(b"}` is `a(b` on 8.4, 8.6, 9.0 and Jim).
5. `line_at_matches_position_at_line_for_multibyte` (`line_index.rs`) / `line_index_line_at_is_encoding_independent` (`tests/it_4`) (exact): `MERGE` the integration copy into the owner unit test.
6. `command_substitution_node` / `command_substitution` (exact and overlap): `MERGE` the `it_4` copy into `expr_parser_coverage`.
7. `t3_6_braces_group_and_suppress_substitution` / `t14_braces_suppress_substitution` (overlap): `MERGE` t14 into t3_6, which asserts one more word; the oracle comment (`string index {$b} 0` is `$`, true on every release and Jim) moves with it.
8. `scanner_agrees_with_owner_over_corpora` / `…_deep`: kept; two findings below.
9. The two `fails_closed_…` tests, the two `ternary` render tests and the two `reparse_window` tests: kept; each pair differs in the gate, the path (hand-built node against parse then render) or the edit range.

## Findings

- **One `VACUOUS`: `word_parts::tests::oracle_sheet_is_recorded`.** Its body is `let () = ORACLE_SHEET;`: it can fail only at compile time. The claim "the oracle sheet is recorded" is a constant's existence. Either make the sheet a table whose rows run against tclsh, or retire the test once the constant is used.
- **One `#[ignore]`, against the scrub rules:** `differential_boundaries::scanner_agrees_with_owner_over_corpora_deep` ("run explicitly with `--ignored`"). Run explicitly on 18 November it passes: 969 files x 3 dialects, 778,294 commands, 111 dialect-divergent, 75 incomplete. The shorter sibling and this one also return silently when no corpus is present, so a missing corpus is a pass.
- **Seven crash guards** (`let _ = lex(..)`, 5,000-deep nests, the 60-input parser fuzz, the no-bignum-backend sweep) assert only that the call returns. They fail by panic or stack overflow, never by value; sound as guards, not behaviours; kept as `KEEP-CONTRACT`, cause `crash-guard`.
- **Silent skips are not in the crates' tclsh tests but in seven Jim tests** (`body_execution_conformance`, `execution_resolution_conformance` and five in `jim_name_resolution_conformance`): they return with an `eprintln` when no Jim oracle is configured, unless `TCL_LSP_REQUIRE_JIM_ORACLE` is set. With the standing rule's flags they ran and passed.
- **No expectation is contradicted by tclsh or Jim.** Every value the float-table tests state in their comments (`atan2`, `hypot`, `fmod`, `log10`, `sqrt`, the promotion arithmetic) is what tclsh 8.4.20, 8.6.18 and 9.0.4 print (8.4 prints 12 significant digits for `sqrt(2)` and `10.0/3`; the tests compare with a tolerance).

## Not done

The clusters file is a lower bound (the same check through a different helper is not caught). I read the 12 clusters, the 17 assertion-free tests and the early-return tests; the other 1,900 or so were not read one by one. The ledger says so in its scope, and a further pass would read the lexer's `lexer::tests` and `structural_index::tests` (1,500 tests between them) for tests that repeat one claim over inputs.
