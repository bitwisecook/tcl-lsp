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
- **One `#[ignore]`, sanctioned:** `differential_boundaries::scanner_agrees_with_owner_over_corpora_deep` (`; run explicitly with --ignored`) is the permanently expensive corpus tier that AGENTS.md lines 60 to 62 and `docs/design/contracts/test-tiers-and-ci-gates.md` rule 2 allow. Run explicitly on 18 November it passes: 969 files x 3 dialects, 778,294 commands, 111 dialect-divergent, 75 incomplete.
- **Corpus-absent skips:** the two corpus tests in that file (`scanner_agrees_with_owner_over_corpora` and `_deep`) return with an `eprintln` when no corpus is found, so a missing corpus is a pass. The corpus is the checked-in sources plus the `tmp/` Tcl trees; CI fetches the 9.0 tree in at least one job (`ci.yml:493`) and I have not shown that every job that runs these tests has the others. The non-deep test asserts `checked > 0` only after the early return. Recommended: make the early return a failure when `TCL_LSP_REQUIRE_ALL_TCL_ORACLES` is set.
- **Nine crash guards** (`let _ = lex(..)`, 5,000-deep nests, the 60-input parser fuzz, the no-bignum-backend sweep) assert only that the call returns. They fail by panic or stack overflow, never by value; sound as guards, not behaviours; each has its own row with the value it should also assert (none cites a fuzzer finding or an issue, so none is a KEEP-CONTRACT on that ground alone); kept as `KEEP-CONTRACT`, cause `crash-guard`.
- **Silent skips are not in the crates' tclsh tests but in seven Jim tests** (`body_execution_conformance`, `execution_resolution_conformance` and five in `jim_name_resolution_conformance`): they return with an `eprintln` when no Jim oracle is configured, unless `TCL_LSP_REQUIRE_JIM_ORACLE` is set. With the standing rule's flags they ran and passed. Each has a `REWRITE` row: onto one `tcl_test_support` helper (like `witness_tclsh`) so `TCL_LSP_REQUIRE_JIM_ORACLE` binds the same way in every file. The test bodies are unchanged but for the locator call; they wait for the owner.
- **No expectation is contradicted by tclsh or Jim.** Every value the float-table tests state in their comments (`atan2`, `hypot`, `fmod`, `log10`, `sqrt`, the promotion arithmetic) is what tclsh 8.4.20, 8.6.18 and 9.0.4 print (8.4 prints 12 significant digits for `sqrt(2)` and `10.0/3`; the tests compare with a tolerance).

## The 30 largest groups, five read from each

Method: the inventory's helper column, grouped by (crate, helper(s) used, class) over the four crates; the 30 largest groups hold 407 of the 2,011 tests. Five were read from each (evenly spaced), 150 bodies. The tclsh values the samples state were re-run on tclsh 8.4.20, 8.6.18 and 9.0.4 and on jimsh (`/tmp/o2.tcl`, 20 expressions): none contradicts a test. The other ~1,600 tests are in smaller groups and were not read.

| # | Group (crate, helper, n) | What it pins | Distinct inputs or restatements | Oracle values |
| --- | --- | --- | --- | --- |
| 1 | lexer `subst` (34) | one backslash/escape decode rule per test (`\x`, octal stop, surrogates, continuation) | distinct inputs of one table; KEEP | continuation case re-run: `a b`, CR kept, on 8.4/8.6/9.0/Jim. `\xg` -> `xg`, `\19`, `\78` match. `lone_high_surrogate`: tclsh holds U+D800 (`scan %c` 55296 on every release); the test expects U+FFFD because a Rust `String` cannot; a representation choice, not an oracle value |
| 2 | syntax parser, no helper, C-uncited (29) | `matches!(node, Binary{op: X})` for one operator or shape | the single-operator ones (`add sub mul div modulo pow shift_left equality str_eq logical_and logical_or in_operator`, 12) restate G20's `all_*_operators`, which asserts the same top-level op through the same parser; the structure ones (`mul_before_add`, `parens_override_precedence`, `right_associative_pow`, `nested_ternary`) are distinct. Candidate MERGE of the 12; not edited | none |
| 3 | syntax, no helper, D-internal (27 over 5 files) | AST `Display`, literal nodes, list splitting | distinct, KEEP. `binop_display` and `unary_not` pin names, not behaviour | none cited |
| 4 | lexer `cmd_token_rows` (22) | `[...]` command-token extent: nesting, quotes, escapes, comments | distinct inputs; KEEP | none |
| 5 | lexer `toks` (21) | expr token kind/text per operator or literal | distinct; `t1_4_empty_expression` is the only empty case | none |
| 6 | lexer `var_token_text` C-uncited (21) | `$name`, `${..}`, array index, `::` | distinct; KEEP | none |
| 7 | lexer `texts` (20) | word texts; several assert `any(..)` membership only | `t1_5`, `t3_3` assert presence only; they should assert the full word list | none |
| 8 | lexer `str_token_rows` (16) | braced-word extent | distinct; KEEP | none |
| 9 | lexer, no helper (15) | first token of bare-`$`, quoted word; `span_is_copy`, `token_type_equality`, `source_position_default` | `span_is_copy` and `token_type_equality` pin derive behaviour only (a `Copy` impl and `==`); weak, kept | none |
| 10 | lexer `lex` D-internal (15) | whole-token streams | distinct; KEEP | 3 cite tclsh: `$$foo`, `a\;b` one word, `{{}}` re-run: `$BAR`, `1`, true |
| 11 | syntax `is_raw`,`p` (13) | inputs the parser must reject (dangling/leading operator, `)`, `max(1 2)`) | tables of distinct inputs; KEEP | `max(1 2)`, `1 +` fail on 8.6, 9.0, Jim (8.4 too) |
| 12 | lexer `ops` (12) | operator extraction per precedence level | distinct; several use `contains(..)` plus one `assert_eq!`; the `contains` lines restate the `assert_eq!` in the same test | none |
| 13 | lexer `quoted_rows` D-internal (12) | when `"` starts a word | distinct; KEEP | none |
| 14 | lexer `close_quote_offset` C-uncited (12) | quote closer with comments and substitutions | distinct bug-by-bug inputs; KEEP | none |
| 15 | syntax `p` D-internal (12) | node shape for strings, calls | distinct; `nested_function_calls` cites `min(5, abs(-2))` = 2: true on 8.6, 9.0; **8.4 has no `min`** (`unknown math function`) and Jim rejects it; the test asserts shape, so it is fine, the comment should say 8.5+ | `ternary_is_right_associative` = 2 on all |
| 16 | lexer `close_quote_offset` D-internal (10) | same, other cases | distinct; KEEP | none |
| 17 | syntax `p` C-uncited (10) | Var/Command/Call node shapes | `command_substitution` is the known MERGE; the rest distinct | none |
| 18 | dialect, no helper (9) | catalogue invariants | distinct; `every_escape_grammar_is_listed` is a compile-time exhaustiveness check plus `len == 4` | `escape_grammar_splits_85_from_86` cites tclsh, grammar table not re-run per release |
| 19 | lexer `tokenise_expr` (9) | release-dependent numeral/bareword boundaries | distinct; `deeply_nested_expr_array_index` is a crash guard (row exists) | re-run: `1.0_2` is one numeral on 9.0, `invalid character "_"` on 8.6; `1 lt_ 2` splits on 9.0, `invalid bareword` on 8.6; `1_000` 1000 on 9.0. All match |
| 20 | syntax `binop`,`p` (9) | one table per operator family | tables of distinct inputs; KEEP | `lt le gt ge` comment re-run: 9.0 and Jim accept, 8.6 `invalid bareword`, as stated |
| 21 | syntax `render_expr` D-internal (9) | hand-built node renders; precedence parens | distinct; verbose, no overlap | none |
| 22 | syntax `diagnose` (9) | exact error text and code per fault | distinct; text matches 8.6/9.0 (`missing operand at _@_`, `missing operator ...`) | re-run: `1 +` and `max(1 2)` messages identical on 8.6 and 9.0 |
| 23 | lexer `BraceIndex::build` (8) | `unterminated_count() == 0` for balanced inputs | restatements in shape: each asserts only the count is 0; the **unbalanced** twin is elsewhere. A table of (input, count) would hold them; no overlap in inputs | none |
| 24 | lexer `var_token_text` D-internal (8) | bare `$`, digit names | distinct; KEEP | none |
| 25 | lexer `word_closer_offset` (8) | closer offset for empty and escaped words | distinct; KEEP | none |
| 26 | lexer `group` (8) | word kinds, weld flags, expand markers | distinct; KEEP | none |
| 27 | syntax `tcloo_linearise` (8) | TclOO method resolution order | distinct; `c3_inconsistent_hierarchy` cites `tclsh9.0 info class call`: re-run on 8.6 and 9.0, `::Z ::B ::A` for the method-bearing classes, consistent with `Z X Y B A` | cited and re-run |
| 28 | dialect `all_with_fallback` (7) | one invariant per profile field | each is a loop over the whole catalogue; distinct fields; KEEP | none |
| 29 | lexer `script_is_complete` (7) | complete or incomplete scripts | distinct; KEEP | none |
| 30 | lexer `lex` C-uncited (7) | literal `$`, escaped brackets, quoted substitution | distinct; `t15_5` is a crash guard (row exists) | none |

### What this says for the crates

- **Tables of distinct inputs, no overlap worth removing**: groups 1, 4, 6, 8, 10 to 14, 16, 18 to 22, 24 to 30. The restatement the scan found is inside the named test files, not across them.
- **One restatement list**: group 2's twelve single-operator parser tests against group 20's `all_*_operators` (same op, same parser). I did not diff each pair; a MERGE needs the owner.
- **Weak assertions to strengthen, not delete**: group 7 (`any(..)` membership, `t1_5`, `t3_3`), group 12 (`contains` lines that restate the `assert_eq!`), group 23 (count only, no row that names the input), group 9 (`span_is_copy`, `token_type_equality`).
- **Oracle values**: the 15 comments that cite tclsh or Jim in these samples were re-run; none is contradicted. Three notes: `lone_high_surrogate` is a representation choice; `nested_function_calls` needs "8.5+" in its comment (8.4 has no `min`); `dispatch_*` float rows already carry tolerances.
- **No test in the sample asserts the opposite of an oracle, and none is vacuous beyond the one `VACUOUS` row.**

## Not done

About 1,600 tests in smaller groups (fewer than 7 per helper shape) were not read, and group membership is by helper name, so a test calling a differently named helper for the same check is not grouped. The clusters file is a lower bound for the same reason.
