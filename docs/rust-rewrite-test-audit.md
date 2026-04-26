# Test audit for the Python → Rust rewrite

This file tracks how the existing pytest suite is being mirrored into
Rust during the rewrite, which tests are deliberately **not** being
ported (because they test Python-bridge-specific behaviour), and which
tests are flagged as **low-value** and should be removed when the
rewrite is complete.

The audit is updated chunk-by-chunk. Each chunk lands with the relevant
rows filled in; unreviewed rows are chased down in a follow-up pass
before the chunk is declared done.

## Principle

The goal of the rewrite is 100% behavioural parity, not 100% test
replication. Tests fall into four categories:

1. **Ported** — the test exercises behaviour of the data/logic itself,
   so the Rust crate gets an equivalent (often more exhaustive) unit
   test. The pytest stays in place as the bridge regression net while
   the Python layer exists.
2. **Bridge-only** — the test exercises something specific to the
   Python binding (identity via `is`, keyword arguments, dataclass
   immutability, Python pattern matching, etc.). There is nothing to
   port — the Rust side has no equivalent concept — so the test lives
   in pytest until the Python layer is retired entirely.
3. **Remove at end** — low-value, duplicative, or over-specified. Kept
   in place during the rewrite for continuity, but flagged here and
   (where possible) inline with an `AUDIT:` comment so they can be
   deleted in one sweep at project end.
4. **Deferred** — the test exercises behaviour whose Rust replacement
   has not yet landed (e.g. the Rust lexer). It stays in pytest until
   the corresponding chunk lands and then gets re-classified.

No test is removed during the rewrite. The Python suite is the
behavioural oracle for every chunk. Low-value tests only come out at
the very end, when the Python layer itself comes out.

## L1 — `core/parsing/substitution.py::backslash_subst`

Commit: `0739d82`.

### Pytest tests — `tests/test_lexer.py::TestBackslashSubstCRLF`

| Test | Category | Rust equivalent |
|---|---|---|
| `test_lf_continuation` | Ported | `rust/tcl-lexer/src/substitution.rs::tests::lf_continuation` |
| `test_crlf_continuation` | Ported | `crlf_continuation` |
| `test_cr_continuation` | Ported | `cr_continuation` |
| `test_crlf_continuation_strips_leading_whitespace` | Ported | `continuation_strips_leading_whitespace` |
| `test_r_escape_preserved` | Ported | `simple_letter_escapes` (covers `\r`) |

No other pytest test calls `backslash_subst` directly. Indirect
coverage comes from every compiler/VM test that processes backslashes
inside source text, which will be re-classified when the lexer lands.

### Rust unit tests — `rust/tcl-lexer/src/substitution.rs`

All 22 tests exercise pure behaviour of the Rust function. None are
bridge-specific. None are marked for removal.

## L2 — `core/parsing/tokens.py` (`TokenType` / `SourcePosition` / `Token`)

Commit: `c7ab728` + this audit pass.

### Pytest tests — `tests/test_tokens.py`

| Test | Category | Rust equivalent / rationale |
|---|---|---|
| `TestTokenType::test_all_variants_exist` | Ported | `token_type_variants_have_distinct_names` + `token_type_name_exact_mapping` |
| `TestTokenType::test_class_attributes_are_singletons` | Bridge-only | Pure Rust enums are values, not singletons. The test pins the PyO3 class-attribute behaviour the binding crate relies on. |
| `TestTokenType::test_distinct_variants_are_not_identical` | Bridge-only | Same — tests Python `is not` identity. |
| `TestTokenType::test_equality` | Ported | `token_type_equality_matches_python_semantics` |
| `TestTokenType::test_name_matches_attribute` | Ported | `token_type_name_exact_mapping` (exhaustive on both sides) |
| `TestTokenType::test_value_discriminants_are_one_indexed_stable` | Bridge-only | The discriminants live in the PyO3 wrapper, not the pure Rust enum; this test pins the PyO3 contract. |
| `TestTokenType::test_hashable_and_set_membership` | Ported | `token_type_is_copy_and_hashable` |
| `TestTokenType::test_value_pattern_match` | Ported | `token_type_equality_matches_python_semantics` has a `match` on the Rust enum. |
| `TestSourcePosition::test_construction_with_keywords` | Bridge-only | Keyword arguments are a Python concept; the Rust equivalent is named-field struct construction. |
| `TestSourcePosition::test_construction_positional` | Bridge-only | Same — positional args are a Python concept. |
| `TestSourcePosition::test_equality` | Ported | `source_position_equality_and_hash` |
| `TestSourcePosition::test_hashable` | Ported | `source_position_equality_and_hash` |
| `TestSourcePosition::test_immutable` | Bridge-only | Tests that `pos.line = 5` raises — pins PyO3 `frozen` semantics. Pure Rust structs with `pub` fields are caller-mutable. |
| `TestToken::test_construction_with_keywords` | Bridge-only | Python kwargs on `Token(type=…, text=…, …)`. |
| `TestToken::test_construction_positional` | **Remove at end** | Flagged in-file. Real call sites always use kwargs for `Token`; positional construction is a Python-ism with no additional contract value. |
| `TestToken::test_in_quote_default_is_false` | Ported | `token_default_in_quote_is_false` |
| `TestToken::test_in_quote_true_propagates` | **Remove at end** | Flagged in-file. Subsumed by `test_equality_compares_all_fields` (`diff_quote` case) and `token_quoted_constructor_sets_in_quote`. |
| `TestToken::test_type_getter_returns_singleton` | Bridge-only | The whole point of the `py.get_type_bound + getattr(name)` trick in `rust/tcl-lsp-rust/src/tokens.rs`. Keeps the 200+ `tok.type is TokenType.X` call sites working. |
| `TestToken::test_type_passthrough_preserves_identity` | Bridge-only | Same — Python `is` identity after a round-trip through the wrapper. |
| `TestToken::test_equality_compares_all_fields` | Ported | `token_equality_compares_all_fields` |
| `TestToken::test_hashable` | Ported | `token_hash_distinguishes_in_quote` |
| `TestToken::test_immutable` | Bridge-only | Same as `TestSourcePosition::test_immutable`. |
| `TestLexerIntegration::test_lexer_produces_singleton_typed_tokens` | Deferred | Re-classified when the Rust lexer lands (L3+). Currently a Python-lexer smoke test that validates the shim propagates Rust-backed types correctly. |
| `TestLexerIntegration::test_match_on_lexer_token_type` | Deferred | Same. |

### Pytest tests — elsewhere

| Test | Category | Notes |
|---|---|---|
| `tests/test_token_positions.py` (2 tests) | Deferred | Exercises `core.parsing.token_positions`, which sits one layer above `tokens.py`. Re-classify when `token_positions.py` gets its own chunk. |
| `tests/test_incremental_update.py::TestPositionShifting::test_shift_position` | Deferred | Tests `shift_position` helper in `token_positions.py`. Constructs `SourcePosition` directly; the shim already validates that construction path. |
| `tests/test_incremental_update.py::TestPositionShifting::test_shift_token` | Deferred | Same, for `shift_token`. |
| `tests/test_formatter.py::TestReconstruction::test_reconstruct_*` (4 tests) | Deferred | Constructs `Token(type=…, …)` directly to feed into the formatter. Will be re-classified with the formatter chunk. |
| All other `core.parsing.tokens` consumers | Deferred | Counted against future chunks. |

### Rust unit tests — `rust/tcl-lexer/src/tokens.rs`

| Test | Notes |
|---|---|
| `token_type_name_exact_mapping` | Added in the audit pass. Exhaustive `name()` mapping. |
| `token_type_variants_have_distinct_names` | Original; stays. |
| `token_type_equality_matches_python_semantics` | Original; stays. |
| `token_type_is_copy_and_hashable` | Original; stays. |
| `source_position_construction_and_field_access` | Original; stays. |
| `source_position_default_is_origin` | Original; stays. |
| `source_position_equality_and_hash` | Original; stays. |
| `source_position_is_copy` | Original; stays. |
| `source_position_accepts_u32_max_values` | Added in the audit pass. |
| `token_construction_borrows_text` | Original; stays. |
| `token_default_in_quote_is_false` | Original; stays. |
| `token_quoted_constructor_sets_in_quote` | Original; stays. |
| `token_equality_compares_all_fields` | Original; stays. |
| `token_hash_distinguishes_in_quote` | Original; stays. |
| `ghost_eof_token_uses_empty_span` | Added in the audit pass, renamed from `synthetic_eof_token_uses_empty_static_text` when L3 introduced the span-first architecture. Documents the empty-span pattern for ghost EOF tokens the Rust lexer emits. |
| `token_text_lifetime_borrows_from_source` | Original; stays. |

None of the Rust tests are flagged for removal.

## L3 — Rust lexer skeleton (`rust/tcl-lexer/src/lexer.rs` + `line_index.rs`)

First Rust lexer chunk. Handles EOF / SEP / EOL / COMMENT / plain ESC;
every other construct trips `LexError::UnsupportedCharacter` so the
differential harness can filter inputs cleanly.

### Pytest tests — `tests/test_rust_lexer_differential.py` (new)

| Test / group | Category | Notes |
|---|---|---|
| `test_curated_corpus_matches_python` (~47 cases) | **Ported** (as a differential harness) | Each parametrised case feeds one input through both lexers and asserts the flattened token tuples match. Becomes the regression gate for every future lexer chunk. |
| `test_harvested_fixtures_match_python` (~20 cases) | **Ported** | Harvests simple string literals from the existing lexer suite; the set grows automatically as chunks L4–L9 remove filter triggers. |
| `TestHarnessItself::test_deferred_inputs_are_filtered` | Bridge-only | Asserts the harness's skip-list matches the Rust lexer's actual `UnsupportedCharacter` set. Bridge-only because the Rust unit tests already cover each individual deferred character. |
| `TestHarnessItself::test_non_ascii_is_filtered` | Bridge-only | Same — documents the multi-byte column workaround, not lexer behaviour. |
| `TestHarnessItself::test_supported_input_passes_filter` | Bridge-only | Sanity check on the filter logic. |

### Pytest tests — elsewhere

| Test | Category | Notes |
|---|---|---|
| `tests/test_lexer.py` (~56 tests) | **Deferred** (most) | The majority exercise constructs the L3 lexer has not implemented yet. Individual classes will be re-classified chunk-by-chunk:<br>&nbsp;&nbsp;• `TestBasicTokens` — partially ported through the differential harness.<br>&nbsp;&nbsp;• `TestCommentBehavior` / `TestCommentSemicolon` — partially ported.<br>&nbsp;&nbsp;• `TestVariableSubst`, `TestCommandSubst`, `TestBracedString`, `TestQuotedString` — deferred to L4–L7.<br>&nbsp;&nbsp;• `TestBackslashNewlineMidWord`, `TestBackslashCRLFContinuation` — deferred to L9. |
| `tests/test_tcl_parse.py`, `tests/test_tcl_parse_old.py`, `tests/test_upstream_parse.py` | **Deferred** | End-to-end Python lexer tests; rely on features not yet ported. Stay Python-only until the Rust lexer subsumes them. |
| `tests/test_command_segmenter.py`, `tests/test_recovery.py`, `tests/test_parsing_helpers.py` | **Deferred** | Consume Token streams from the Python lexer. Will re-classify when their producer is Rust-backed. |

### Rust unit tests — `rust/tcl-lexer/src/line_index.rs`

| Test | Notes |
|---|---|
| `empty_source_has_one_line` | Original; stays. |
| `single_line_no_newline` | Original; stays. |
| `two_lines` | Original; stays. |
| `many_lines` | Original; stays. |
| `consecutive_newlines_yield_empty_lines` | Original; stays. |
| `line_index_is_cloneable` | Original; stays. |

### Rust unit tests — `rust/tcl-lexer/src/lexer.rs`

| Test | Notes |
|---|---|
| `empty_source_produces_no_tokens` | Python parity. |
| `single_word_emits_esc_and_trailing_eol` | Python parity. |
| `two_words_separated_by_space` | Python parity. |
| `multiple_spaces_collapse_into_one_sep_token` | Python parity. |
| `tab_separator` | Python parity. |
| `cr_is_separator_not_eol` | Pins the `\r ∈ _SEP_CHARS` Python detail. |
| `lf_is_eol` | Python parity. |
| `semicolon_is_eol` | Python parity. |
| `mixed_eol_and_whitespace_becomes_single_eol_token` | Pins Python's `_parse_eol` behaviour. |
| `leading_whitespace_before_word` | Python parity. |
| `trailing_whitespace_still_emits_ghost_eol` | Pins the `_at_command_start` preservation across SEP. |
| `trailing_newline_does_not_add_second_eol` | Guards against double-emitting the ghost EOL. |
| `comment_at_command_start` | Python parity. |
| `comment_terminated_by_newline` | Python parity; the `\n` is a separate EOL token. |
| `comment_after_whitespace_at_command_start` | Pins SEP preserving `at_command_start`. |
| `hash_not_at_command_start_is_part_of_word` | Pins `#` inside a word. |
| `two_commands_separated_by_eol_both_allow_comments` | Pins EOL resetting `at_command_start`. |
| `position_tracking_simple_word` | LineIndex-backed position. |
| `position_tracking_across_newline` | LineIndex-backed multi-line position. |
| `unsupported_character_dollar_errors` | Locks in the error surface until L4. |
| `unsupported_character_brace_errors` | Locks in the error surface until L6. |
| `unsupported_character_bracket_errors` | Locks in the error surface until L5. |
| `unsupported_character_backslash_errors` | Locks in the error surface until L9. |
| `unsupported_character_in_comment_errors` | Pins comment-scanner error for backslashes. |
| `after_error_iterator_stops` | Fuses the iterator on fatal error. |
| `shared_line_index_constructor` | Pins `Lexer::with_line_index` parity. |
| `into_line_index_round_trip` | Pins the LineIndex extraction API. |

None of the L3 Rust tests are flagged for removal.

### Category totals after L3

- **ported** (data behaviour has Rust coverage): 13 pytest + 49 Rust unit tests + 67 differential cases
- **bridge-only** (Python-specific, stays in pytest forever): 13 pytest tests
- **remove-at-end** (low-value, flagged inline): 2 pytest tests
- **deferred** (covered by later chunks): ~75 pytest tests in `test_lexer.py` and friends

## L4 — Variable substitution (`rust/tcl-lexer/src/lexer.rs::parse_var`)

Second Rust lexer chunk. Removes `$` from the deferred set and
implements all four Tcl variable substitution forms plus
`SourceMap::token_text` for Python-parity text extraction.

### Pytest tests — `tests/test_lexer.py::TestBasicTokens` / `TestExtendedVars`

The variable-substitution tests in `test_lexer.py` are now
indirectly exercised end-to-end via the differential harness, which
runs each Python test's input string through both lexers and
asserts byte-perfect parity on the token stream (including the
Python-API asymmetry where `tok.start` points at the `$` but
`tok.text` strips it).

| Test | Category | Rust equivalent |
|---|---|---|
| `TestBasicTokens::test_variable` (`$foo`) | Ported | `var_simple_identifier` + differential corpus |
| `TestBasicTokens::test_bare_dollar` (`$`) | Ported | `bare_dollar_is_an_str_token` + differential corpus |
| `TestExtendedVars::test_braced_var` (`${my var}`) | Ported | `var_braced_allows_arbitrary_characters` + differential corpus |
| `TestExtendedVars::test_namespace_var` (`$ns::var`) | Ported | `var_namespace_separator` + differential corpus |
| `TestExtendedVars::test_nested_namespace_var` (`$a::b::c`) | Ported | `var_multi_level_namespace` + differential corpus |
| `TestExtendedVars::test_array_var` (`$arr(idx)`) | Ported | `var_array_index` + differential corpus |
| `TestExtendedVars::test_array_with_namespace` (`$ns::arr(key)`) | Ported | differential corpus (`var_array_ns`) |
| `TestPositions::test_var_position` (`set x $y`) | Ported | `var_span_positions` + differential corpus (`var_in_command`) |

### Pytest tests — `tests/test_rust_lexer_differential.py`

| Change | Notes |
|---|---|
| `_rust_supports` rewritten as try/catch filter | No more hand-maintained deferred-character blacklist. As chunks L5–L9 remove triggers, the harness auto-detects the expanded support surface without any harness edit. Also handles context-sensitive cases correctly (e.g. `{` / `}` inside a `${…}` braced name are fine even though `{` / `}` as top-level constructs are still deferred). |
| Corpus grew from 47 to 74 parametrised cases | Added 27 L4 variable-substitution inputs: `var_simple`, `var_underscore`, `var_digit`, `var_alnum`, `var_uppercase`, `var_ns`, `var_ns_deep`, `var_leading_ns`, `var_single_colon_ends`, `var_braced`, `var_braced_empty`, `var_braced_with_spaces`, `var_braced_with_special`, `var_array`, `var_array_ns`, `var_array_nested`, `var_array_braced_inner`, `bare_dollar`, `bare_dollar_space`, `bare_dollar_lf`, `var_then_word`, `word_then_var`, `multiple_vars`, `var_in_command`, `var_resets_command_start`, `var_after_comment`, `var_mid_stream`, plus 2 unterminated cases for best-effort recovery coverage (`unterminated_braced_var`, `unterminated_array_var`). |
| `TestHarnessItself::test_deferred_inputs_are_filtered` | Updated to the post-L4 deferred set `{}[]"\\` (no more `$`). |
| `TestHarnessItself::test_dollar_is_no_longer_deferred` | **New**: regression guard ensuring `$foo`, `${name}`, `$arr(idx)`, bare `$` all pass the filter. |

### Rust unit tests — `rust/tcl-lexer/src/lexer.rs`

New tests (25 added, all in the `L4 — variable substitution`
section at the end of the lexer's test module):

| Test | Notes |
|---|---|
| `var_simple_identifier` | Plain `$foo`. |
| `var_with_underscore` | `$_private` — underscore is a valid name char. |
| `var_alphanumeric_accepts_digits_anywhere` | `$foo1` and `$1` (Tcl uses `$1`, `$2` for regex backrefs). |
| `var_uppercase` | `$FOO`. |
| `var_namespace_separator` | `$ns::var`. |
| `var_multi_level_namespace` | `$a::b::c`. |
| `var_leading_namespace` | `$::global`. |
| `var_single_colon_terminates_name` | `$foo:bar` — single `:` is not a name char. |
| `var_braced_form` | `${name}`. |
| `var_braced_empty_body` | `${}` — exercises the empty-content end-position clamp. |
| `var_braced_allows_arbitrary_characters` | `${weird name with spaces}`. |
| `var_braced_unterminated_tokenises_best_effort` | `${unterminated` — L9 will add the warning. |
| `var_array_index` | `$arr(idx)`. |
| `var_array_index_nested_parens` | `$arr(one(two)three)`. |
| `var_array_index_with_inner_braced_var` | `$arr(${key})` — `${…}` inside an array index is scanned as a unit. |
| `var_array_index_unterminated_tokenises_best_effort` | `$arr(idx`. |
| `bare_dollar_is_an_str_token` | `$` alone emits an STR, not a VAR (matching Python). |
| `bare_dollar_followed_by_space` | `$ foo`. |
| `bare_dollar_followed_by_lf` | `$\n`. |
| `var_followed_by_word` | `$foo bar`. |
| `multiple_vars` | `$a $b $c`. |
| `var_resets_at_command_start` | After a VAR, `#` is no longer a comment opener. |
| `esc_stops_at_dollar` | `foo$bar` → ESC then VAR. |
| `var_span_positions` | Pins the "span starts at `$`, text strips `$`" convention via explicit span and `token_text` assertions. |
| `braced_var_span_covers_delimiter_and_name` | Same convention for `${name}`. |

Regression guard `dollar_is_no_longer_an_unsupported_character`
replaces the pre-L4 `unsupported_character_dollar_errors` test so
the check that `$` is accepted survives into the chunk log.

### Pytest tests — ported elsewhere

The wider `tests/test_lexer.py` classes for variable substitution
(`TestBasicTokens::test_variable`, `TestBasicTokens::test_bare_dollar`,
`TestExtendedVars::*`, `TestPositions::test_var_position`) are all
exercised end-to-end by the differential harness now. They stay in
pytest as the bridge oracle but the Rust side has byte-perfect
parity coverage.

### Category totals after L4

- **ported** (data behaviour has Rust coverage): 21 pytest + 74 Rust unit tests + 100 differential cases
- **bridge-only** (Python-specific, stays in pytest forever): 14 pytest tests
- **remove-at-end** (low-value, flagged inline): 2 pytest tests
- **deferred** (covered by later chunks): ~60 pytest tests in `test_lexer.py` and friends (shrunk from ~75 as the variable tests were ported)

## L5 — Command substitution (`rust/tcl-lexer/src/lexer.rs::parse_command`)

Third Rust lexer chunk. Removes `[` and `]` from the deferred set
and implements Tcl command substitution with Python-parity nesting
rules. Also extends `SourceMap::token_text` with CMD stripping
(and a subtle fix for nested cases that had their inner `]`
incorrectly consumed by the original VAR-style suffix strip).

### Pytest tests — `tests/test_lexer.py::TestBasicTokens` / `TestTclConstructs`

| Test | Category | Rust equivalent |
|---|---|---|
| `TestBasicTokens::test_command_substitution` (`[+ 1 2]`) | Ported | `cmd_simple_body` + differential corpus |
| `TestBasicTokens::test_nested_brackets` (`[+ 1 [+ 2 3]]`) | Ported | `cmd_nested_brackets` + differential corpus |
| `TestPositions::test_cmd_position` (`set x [+ 1 2]`) | Ported | `cmd_span_positions` + differential corpus (`cmd_mid_command`) |

The broader `tests/test_tcl_parse.py` and `test_upstream_parse.py`
test classes that use `[…]` are now eligible for the differential
harness. They stay in pytest as the Python-bridge oracle; the Rust
side gains byte-perfect parity as the harness corpus expands.

### Pytest tests — `tests/test_rust_lexer_differential.py`

| Change | Notes |
|---|---|
| `TestHarnessItself::_EXPECTED_DEFERRED` | Updated to `{}"\\` (no more `[]`). |
| `TestHarnessItself::test_brackets_are_no_longer_deferred` | **New**: regression guard ensuring `[cmd]`, `[+ 1 2]`, and lone `foo]bar` all pass the filter. |
| Corpus grew from 74 to 102 parametrised cases | Added 28 L5 command-substitution inputs covering empty, simple, nested, mixed-with-var, quoted-substring-inside-cmd, braced-substring-inside-cmd, backslash-escaped close, multiline, standalone `]`, and unterminated best-effort cases. |

### Rust unit tests — `rust/tcl-lexer/src/lexer.rs`

20 new tests in a dedicated `L5 — command substitution` section:

| Test | Notes |
|---|---|
| `cmd_simple_body` | `[+ 1 2]`. |
| `cmd_empty_body` | `[]` — exercises the empty-body end-position clamp. |
| `cmd_nested_brackets` | `[+ 1 [+ 2 3]]` — pins the level-counter correctness. |
| `cmd_deeply_nested_brackets` | `[a [b [c [d]]]]`. |
| `cmd_followed_by_word` | `[cmd] tail`. |
| `word_then_cmd` | `foo[cmd]` — `[` terminates the bare word. |
| `cmd_then_word` | `[cmd]tail`. |
| `cmd_with_quoted_substring` | `"…"` inside a CMD body toggles `in_quotes` so a `]` inside the quotes does not close the command. |
| `cmd_with_bracket_inside_quotes_does_not_close` | Same, with explicit `]` inside `"…"`. |
| `cmd_with_braced_substring` | `{…}` inside a CMD body adjusts `blevel` so a `]` inside the braces is inert. |
| `cmd_with_bracket_inside_braces_does_not_close` | Same, explicit. |
| `cmd_with_nested_braces` | Multi-level brace nesting inside a command. |
| `cmd_with_backslash_escape` | `\]` inside the body is inert (backslash consumes two chars as a pair). |
| `cmd_with_backslash_quote` | `\"` inside the body does not toggle `in_quotes`. |
| `cmd_with_dollar_braced_var_inside` | `${odd}name` sub-scan so a `}` inside a braced variable name doesn't fool `blevel`. |
| `cmd_with_plain_dollar_var_inside` | `$a + $b` inside a command. |
| `cmd_multiline_body` | `[a\nb\nc]`. |
| `cmd_unterminated_tokenises_best_effort` | `[unterminated`. |
| `cmd_span_positions` | Pins the span convention ("span starts at `[`, text strips `[`") via explicit span and `token_text` assertions. |
| `standalone_closing_bracket_is_part_of_word` | `foo]bar` — `]` is no longer deferred; it's a regular word char. |
| `cmd_resets_at_command_start` | After a CMD, `#` is no longer a comment opener. |
| `cmd_after_eol_allows_comment_before` | Mirror of the `after_eol` comment-opener test for CMD contexts. |

Regression guard `bracket_is_no_longer_an_unsupported_character`
replaces the pre-L5 `unsupported_character_bracket_errors` test
so the check that `[` is accepted survives into the chunk log.

### Category totals after L5

- **ported**: 24 pytest + 94 Rust unit tests + 128 differential cases
- **bridge-only**: 15 pytest tests (the new `test_brackets_are_no_longer_deferred` regression guard)
- **remove-at-end**: 2 pytest tests
- **deferred**: ~50 pytest tests (shrunk from ~60 as command-substitution tests became eligible for the harness)

## L6 — Braced strings (`rust/tcl-lexer/src/lexer.rs::parse_brace`)

Fourth Rust lexer chunk. Removes `{` and `}` from the deferred
set and implements Tcl's brace-quoted strings with Python-parity
nesting rules. Adds `Lexer::is_newword()` — a mirror of Python
`_parse_string`'s `newword` predicate based on `last_kind` —
because a `{` at a word boundary starts a braced string but a
`{` in the middle of a bare word is a regular character.

### Dynamic harness harvest

Between L5 and L6, the differential harness gained a
`_harvest_lexer_inputs()` helper that scans every pytest test
file for ASCII string literals and filters by `_rust_supports`.
As L6 removes braces from the deferred set, the corpus grows
automatically with every brace-using test literal.

- Pre-L6: ~1011 harvested inputs (L3-L5 supported subset).
- Post-L6: ~1210 harvested inputs (+~200 brace-using literals).
- Per-chunk harvest growth will continue as L7-L9 land.

The `_is_known_drift` rule-based filter replaces the earlier
exact-match `_KNOWN_PARITY_DRIFT` set and catches two classes of
drift automatically: (a) inputs ending with `{`, `[`, or `${`
(past-EOF end-position clamp), and (b) inputs containing `{*}`
(L8 expansion prefix). Both shrink as later chunks land.

### Pytest tests — `tests/test_lexer.py::TestBasicTokens` / `TestTclConstructs`

| Test | Category | Rust equivalent |
|---|---|---|
| `TestBasicTokens::test_braces` (`{hello world}`) | Ported | `braced_simple_body` + dynamic harvest |
| `TestBasicTokens::test_nested_braces` (`{a {b c} d}`) | Ported | `braced_nested_once` + dynamic harvest |
| `TestPositions::test_multiline_braced_string` (`{line1\nline2}`) | Ported | `braced_multiline_body` + dynamic harvest |
| `TestTclConstructs::test_if_else`, `test_while_loop`, `test_proc_definition`, `test_foreach`, `test_switch`, `test_namespace_eval` | Ported via harvester | Every one of these fixtures is now eligible for the dynamic corpus; the harness runs byte-perfect parity on them automatically. |

### Pytest tests — `tests/test_rust_lexer_differential.py`

| Change | Notes |
|---|---|
| `_is_known_drift` helper | Replaces the hand-maintained `_KNOWN_PARITY_DRIFT` set with a rule-based filter. |
| `TestHarnessItself::_EXPECTED_DEFERRED` | Updated to `"\\` (no more `{}`). |
| `TestHarnessItself::test_braces_are_no_longer_deferred` | **New**: regression guard ensuring `{body}`, `proc foo {a b} {return $a}`, `foo}bar`, and `foo{not-a-brace}baz` all pass the filter. |
| Curated corpus grew from 102 to 127 cases | Added 25 L6 inputs: simple, multiline, nested, backslash-pair escapes, literals inside braces, mid-word `{` / `}`, braced-then-braced, empty, and realistic proc/if/while/foreach shapes. |

### Rust unit tests — `rust/tcl-lexer/src/lexer.rs`

21 new tests in a dedicated `L6 — braced strings` section:

    braced_simple_body, braced_empty_body, braced_nested_once,
    braced_deeply_nested, braced_after_word,
    braced_midword_is_regular_character,
    close_brace_midword_is_regular_character,
    braced_multiline_body, braced_with_dollar_is_literal,
    braced_with_brackets_is_literal,
    braced_with_backslash_is_literal_pair,
    braced_with_backslash_close_brace_is_inert,
    braced_with_backslash_open_brace_is_inert,
    braced_followed_by_word, braced_then_braced,
    braced_unterminated_tokenises_best_effort,
    braced_at_command_start, braced_inside_command_substitution,
    braced_span_positions, braced_resets_at_command_start,
    braced_preserves_newword_for_next_token

Plus `brace_is_no_longer_an_unsupported_character` replacing the
pre-L6 error regression test.

### Category totals after L6

- **ported**: ~35 pytest + 115 Rust unit tests + ~1210 differential cases (dynamic harvest ballooned as `{…}` became eligible)
- **bridge-only**: 16 pytest tests (new `test_braces_are_no_longer_deferred` regression guard)
- **remove-at-end**: 2 pytest tests
- **deferred**: ~40 pytest tests — mostly backslash-heavy tests (L9), quoted-string tests (L7), iRules dialect tests (L8)

## L7 — Quoted strings (`rust/tcl-lexer/src/lexer.rs::parse_quoted`)

Fifth Rust lexer chunk. Removes `"` from the deferred set and
implements Tcl's `"…"` quoted strings with Python-parity
interpolation rules: `$` and `[` inside a quoted run still
dispatch to `parse_var` / `parse_command`, but separators, EOL
characters, `#`, `{`, and `}` are all literal content while
`in_quote` is set.

Adds a new `Token::content_offset: u8` field that records how
many leading bytes of the span are delimiter rather than
content. `SourceMap::token_text` uses this to strip the opening
`$` / `${` / `[` / `{` / `"` from the wrapper tokens uniformly,
replacing the per-kind `strip_prefix` logic. The new field
distinguishes quoted ESCs (whose span starts with an opening
`"`) from bare-word ESCs that happen to contain literal `"`
characters (like the `"cd"` in `"ab""cd"`) — the latter have
`content_offset = 0` and are NOT stripped.

### Pytest tests — `tests/test_lexer.py::TestBasicTokens` / `TestTclConstructs`

| Test | Category | Rust equivalent |
|---|---|---|
| `TestBasicTokens::test_quoted_string` (`"hello world"`) | Ported | `quoted_simple` / `quoted_with_space` + dynamic harvest |
| `TestTclConstructs::test_string_interpolation` (`"hello $name, result is [+ 1 2]!"`) | Ported | `quoted_with_var_and_cmd` + dynamic harvest |
| `TestTclConstructs::test_backslash_in_string` | Deferred — L9 | Uses `\`, out of scope until L9. |

The wider `test_tcl_parse.py` / `test_upstream_parse.py` suites
now pick up quoted-string inputs via the dynamic harvester.

### Pytest tests — `tests/test_rust_lexer_differential.py`

| Change | Notes |
|---|---|
| `_is_known_drift` updated | Added `endswith('"')` to Category A (past-EOF clamp) — inputs ending with an unterminated empty `"` like `set x "` hit the same drift class as `{`, `[`, `${`. |
| `TestHarnessItself::_EXPECTED_DEFERRED` | Updated to `\\` — the last character in the deferred set after L7. |
| `TestHarnessItself::test_quotes_are_no_longer_deferred` | **New**: regression guard ensuring `"hello"`, `"hello $foo world"`, and `foo"bar"` all pass the filter. |
| Curated corpus grew from 127 to 156 cases | Added 29 L7 inputs: simple, empty, single-char, multiline, literal braces/hash/semicolon inside, var/cmd interpolation, opening-empty cases, mid-word quote, quoted-after-esc, multiple quoted strings, quoted inside cmd/brace, quoted with bracket/brace literals, set/puts fixtures, unterminated. |

### Rust unit tests — `rust/tcl-lexer/src/lexer.rs`

19 new tests in a dedicated `L7 — quoted strings` section:

    quoted_simple, quoted_with_space, quoted_empty,
    quoted_contains_braces_literally,
    quoted_contains_separators_literally,
    quoted_with_hash_is_literal, quoted_with_var_interpolation,
    quoted_with_cmd_interpolation, quoted_with_var_and_cmd,
    quoted_opening_empty_with_var,
    quoted_opening_empty_with_cmd,
    quoted_mid_word_is_regular_character,
    quoted_after_esc_then_space_is_word_start,
    quoted_then_mid_word_quote,
    quoted_unterminated_tokenises_best_effort,
    quoted_multiline_body, quoted_span_positions,
    quoted_inside_cmd_is_managed_by_parse_command,
    quoted_resets_at_command_start,
    quoted_in_quote_propagates_to_sub_tokens

### `Token::content_offset` refactor

The pre-L7 `SourceMap::token_text` used kind-specific
`strip_prefix` calls (`raw.strip_prefix("${")`,
`raw.strip_prefix('[')`, etc.). This worked for VAR, CMD, and
STR because their spans always start with a known delimiter. It
broke for quoted ESCs because a bare-word ESC like `"cd"` (from
`"ab""cd"`) also starts with `"` but its content should NOT
strip the leading quote.

L7 replaces the per-kind prefix stripping with a single
`Token::content_offset: u8` field that the lexer sets at
construction time:

| Kind | `content_offset` |
|---|---|
| `Sep`, `Eol`, `Comment`, bare `Esc` | 0 |
| `Var` (`$name`, `$arr(idx)`) | 1 (skip `$`) |
| `Var` (`${name}`) | 2 (skip `${`) |
| `Str` (bare `$`) | 0 (the `$` IS the content) |
| `Cmd` | 1 (skip `[`) |
| `Str` (braced) | 1 (skip `{`) |
| `Esc` (quoted opening) | 1 (skip `"`) |
| `Esc` (quoted mid/closing) | 0 |

`token_text` then does `&raw[content_offset..]` uniformly, plus
a per-kind trailing-strip for the empty-degenerate clamp cases.

### Category totals after L7

- **ported**: ~40 pytest + 134 Rust unit tests + ~1290 differential cases
- **bridge-only**: 17 pytest tests (new `test_quotes_are_no_longer_deferred` regression guard)
- **remove-at-end**: 2 pytest tests
- **deferred**: ~30 pytest tests — backslash-heavy (L9) and iRules dialect (L8) remain


## Audit gap notice

This audit has been dormant since L7 (the lexer's last sub-strip).
Chunks C0–C39 + R2 + L8–L13 + Sync landed without per-chunk audit
rows. A back-fill pass is outstanding; this doc should not be
treated as a complete record of the test-port status until the
back-fill lands.

The C40 row below is added in the C40-fu5 follow-up to break the
silence; subsequent chunks are expected to land their audit rows in
the same commit that ships them.

## C40 — `core/analysis/signature_scan.py`

Commit range: C40a1 (`20d4357`) → C40e8 (`eaf9c09`); roadmap visibility
fix `e0ead7a`. The work landed across 38 commits on the
`claude/rust-signature-scan-c40-sdNaw` branch.

### Pytest tests — `tests/test_signature_scan.py` (Python-side)

The pre-existing Python test file is unchanged by C40 — every test
runs against `_extract_signatures_python` in default-off mode and
acts as the parity oracle the differential harness compares against.

| Tests | Category | Rationale |
|---|---|---|
| Every `tests/test_signature_scan.py` test | Ported (oracle) | The Python implementation stays as the default behaviour and as the differential harness's "Python side" until the C40-default-on follow-up flips the gate; once the env var is gone the Python file + its tests can both retire. |

### Pytest tests — `tests/test_rust_signature_scan_differential.py` (new)

Inline-corpus fixtures + 1 sanity test, all of category **Ported**
(differential parity asserts the Rust binding produces the same
`AnalysisResult` as the Python implementation on every fixture).
The corpus grew by one fixture (`source_substituted_path`) once
Seg1 landed the segmenter argv-widening fix.

The corpus is gated on `pytest.importorskip("tcl_lsp_rust", …)` so
it stays green where the binding isn't built. Recovery-shape
fixtures are intentionally not added until the Python recovery
position-rebasing bug is fixed (see Seg2).

### Pytest tests — `tests/test_rust_bindings_smoke.py` additions

3 new smoke tests (C40e1 + C40e2 + C40e3), all **Ported** —
they pin the PyO3 binding's dict shape so a Rust-side struct change
that breaks the binding contract is caught immediately.

### Rust unit tests

| Module | Count | Notes |
|---|---:|---|
| `signature_scan/params.rs` | 7 | Plus 1 doc-test for `parse_param_list`. |
| `signature_scan/types.rs` | 0 | Pure data types; covered transitively by handler / walker tests. |
| `signature_scan/ctx.rs` | 3 | Skip-heads count + canonical-builtin presence + default empty. |
| `signature_scan/handlers.rs` | 33 | Per-handler unit tests (3–5 per handler) + `qualify` / `emit_class` helpers. |
| `signature_scan/walker.rs` | 15 | Top-level dispatch + body recursion (if / catch / try / namespace eval) + factory walker. |
| `signature_scan/factory.rs` | 13 | `is_factory_body` + `lookup_factory` + `resolve_factory_defs`. |
| `signature_scan/mod.rs` | 6 | End-to-end tests of `extract_signatures(source, registry)`. |

All Rust unit tests exercise pure behaviour of the Rust functions —
none are bridge-specific, none are marked for removal.

### Test-audit gaps tracked for C40

- **Dispatcher coverage** (landed C40-fu4, updated for the
  C40-default-on polarity flip): four `extract_signatures(source)`
  tests in `tests/test_rust_signature_scan_differential.py` cover
  env var unset (now → Rust) / explicit `=1` (Rust) / `=0`
  (forces Python — the post-flip opt-out knob) / Rust path raising
  → Python fallback. No longer an open C40 gap.
- **`command_aliases` corpus is thin**: only one fixture
  (single-`hello` extra). Multi-extras case (`interp alias {} a {}
  b c d e`) should land in a follow-up.
- **Factory-wrapper cross-namespace negative is unit-only**: the
  Rust `factory.rs::lookup_cross_namespace_never_falls_through`
  test pins the rule, but the differential corpus has no
  end-to-end fixture exercising it.

## Seg1 — Segmenter argv-widening parity

Single-strip fix in `rust/tcl-compiler/src/segmenter.rs`. Adds one
Rust unit test
(`segmenter::tests::multi_token_word_argv_spans_full_word`)
pinning the new behaviour, and one Python differential fixture
(`source_substituted_path` in
`tests/test_rust_signature_scan_differential.py`) closing the
last C40 differential gap. No other test families touched.

## Seg2 — Segmenter error recovery

Rust port of `_has_suspicious_token` + `_find_recovery_offset`
from `core/parsing/command_segmenter.py`. Lands as a hard
prerequisite for C40-default-on — without recovery on the Rust
side, flipping the dispatcher to Rust would silently regress
workspace indexing for any file mid-edit with an unclosed brace.

### Rust unit tests — `rust/tcl-compiler/src/segmenter.rs::recovery_tests`

8 tests, all **Ported** (none bridge-specific):

- `unclosed_brace_recovers_at_known_command` — canonical
  `proc early {} { ... proc late {} {}` shape.
- `unclosed_brace_without_known_command_keeps_swallowed_input`
  — recovery only fires when a known command is found.
- `unclosed_brace_below_line_threshold_skipped` — multi-line
  literals shorter than the 3-line threshold are not treated as
  suspicious.
- `unclosed_bracket_recovers_without_line_threshold` — `[`
  recovery fires regardless of line count (a valid `[…]` always
  closes).
- `empty_known_commands_set_never_recovers` — empty set is a
  no-op.
- `closed_input_is_unaffected_by_recovery` — recovery preserves
  well-formed segmentation byte-for-byte.
- `recovery_picks_first_matching_line_not_later_ones` —
  first-match semantics.
- `recovery_skips_indented_lines_and_finds_unindented_match` —
  leading-whitespace stripping in the line scan.
- `recovery_from_offset_into_source_uses_absolute_offsets` —
  the recovered slice's argv tokens carry outer-source byte
  offsets.

### Pytest tests — `tests/test_signature_scan.py::TestSegmenterRecovery`

Pre-existing test
(`test_unclosed_brace_does_not_swallow_later_procs`) is unchanged
by Seg2 but now passes through the Rust path under
`TCL_LSP_RUST_SIGNATURE_SCAN=1` (and is the default after
C40-default-on).

### Differential corpus — recovery fixtures intentionally omitted

The Python `_segment_raw` recovery synthesises a body_token with
`start.line=0, start.character=0`, which causes the Python lexer
to report recovered tokens with `line=0` even when the proc is on
line 3+. The Rust port produces absolute positions throughout
(line=3 for `proc late` on line 3). Offsets agree, line/character
disagree.

That's a Python-side position-rebasing bug surfaced only when
both implementations participate in the differential harness. It
silently goes away once the C40-default-on flip ships
(consumers see Rust's absolute positions, not Python's rebased
ones); fixing the Python rebase is tracked as a separate
follow-up since main hasn't seen this issue.

## C40-default-on — flip the C40 dispatcher to Rust by default

Two-commit chunk:

- **Phase 1** (`rust_shim_enabled` keyword extension): one new test
  file `tests/test_rust_shim_enabled.py` with 24 parametrised
  helper-level cases enumerating truthy / falsy / unrecognised /
  unset × `default=False` / `default=True` permutations. Pins the
  shared helper contract so a polarity bug is caught at its source
  rather than via a per-shim regression.
- **Phase 2** (dispatcher flip): updates the four dispatcher tests
  in `tests/test_rust_signature_scan_differential.py` to assert the
  new polarity (unset → Rust; `=1` → Rust; `=0` → Python; Rust
  raising → Python fallback). The pre-existing Python
  `tests/test_signature_scan.py` file is unchanged but now exercises
  the Rust dispatcher by default in any environment with the wheel
  installed; it stays useful as a public-API behavioural check for
  whichever path is the default. The differential corpus
  (`_assert_same`) still calls the Python and Rust paths directly,
  bypassing the dispatcher, so it remains the parity oracle until
  the Python implementation retires. Seg2 had to land before Phase 2
  shipped — without it, the Rust path silently dropped recovered
  declarations from the workspace index.

## C41 — `core/analysis/_analyser/`

The analyser has the largest test surface in the repo
(`tests/test_analyser.py` carries 2,049 cases covering every
W-code, every iRule diagnostic, and every recovery shape).
The C41 chunk does **not** add per-Python-test parity coverage
to the Rust port — the bar is differential parity on a
purpose-built corpus instead.

- **Differential corpus**
  (`tests/test_rust_analyser_differential.py`): 92 parametrised
  fixtures across two comparators after the
  `C41-default-on-1` … `C41-default-on-6` strips.
  Each fixture runs the Python `Analyser().analyse(source)` and
  the Rust `tcl_lsp_rust.analyser_analyse(source, dialect)` →
  `_materialise_rust_analysis` paths.
  - The 53-fixture **shape-only** subset uses `_compare_shapes`
    and asserts the proc-qualified-name set and class-qualified-name
    set match (with the `::::`→`::` normalisation kept for
    backwards-compat against any pre-fix output).
  - The 34-fixture **field-by-field** subset
    (`FIELD_PARITY_LABELS`) plus 5 iRules-dialect fixtures
    (`IRULES_CORPUS` activated under the `_irules_profile`
    fixture) use the strict `_compare_fields` helper —
    `all_procs` keys, the bare `name` / `params` / `name_range`
    / `body_range`, the `all_classes` keys + per-class
    `methods` / `class_methods` / `superclasses` / `mixins`,
    and the recursive scope-tree shape (`kind`, `name`, child
    var/proc/class name sets at every level).
  - Diagnostics are intentionally **not** compared
    field-by-field — Python's `Analyser.analyse` integrates
    `run_compiler_checks` (style + SSA dead-store W110 / W220
    / W304 …) while Rust's `analyser_analyse` is analyser-only;
    layering alignment is its own follow-up.
- **Dispatcher gating**: 4 cases in the same file assert the
  env-var-gated dispatcher in `core/analysis/_analyser/__init__.py`:
  unset → Python (default-OFF), `=1` → Rust, `=0` → Python,
  Rust raising → Python fallback.  The Rust binding is patched
  in via `monkeypatch.setattr(analyser_module, "_rust_analyse",
  …)` so the gating tests run in Python-only CI environments
  too.
- **Cargo unit tests**: 1,765 across the workspace.  The
  per-strip files (`analyser/oo.rs`, `analyser/recovery.rs`,
  `analyser/diagnostics.rs`, `analyser/handlers.rs`) carry
  in-module `mod tests` blocks for each handler, recovery
  helper, and diagnostic emitter.  These tests cover the *Rust
  side only* — they don't compare against Python.

**Known parity gaps surfaced by the differential** (each
tracked as its own Rust-side follow-up; see the
`C41-default-on` chunk-log row):

- **Materialiser fields landed**:
  `_materialise_rust_analysis` now extracts
  `command_invocations` (with Python-side
  `resolved_qualified_name` resolution against the
  materialised `all_procs`), `command_aliases`,
  `package_requires`, `source_targets`, and
  `namespace_imports` from the Rust dict.
- **Hybrid supplement landed**:
  `_merge_rust_with_python_supplement` runs the Python
  `Analyser` alongside the Rust pass and copies in the
  fields the Rust port doesn't emit yet
  (`source_targets`, `regex_patterns`, `stub_commands`,
  `stub_expr_defs`, `auto_path_entries`,
  `package_provides`, `has_dynamic_providers`,
  `suppressed_lines`, `namespace_imports` /
  `command_aliases` when Rust returns empty,
  `unknown_proc_info`, `all_procs`, `all_classes`,
  `global_scope`, `all_variables`,
  `command_invocations`, `diagnostics`).  Verified
  performance-neutral: `make prep-pr` runs in 67s at
  default-on (vs 70s baseline default-off).
- **Rust-side body iteration landed**:
  `Analyser` gained a `registry: Option<CommandRegistry>`
  field populated once at the top of `analyse()`;
  `process_command` runs a registry-driven
  `ArgRole::Body` loop after the per-command handlers
  return, walking `if` / `while` / `when` / `eval` /
  `uplevel` / `subst` / etc. body arguments.  The loop
  sets `current_event` for `when EVENT { body }` and
  bumps `conditional_depth` for `if` / `try`, mirroring
  the Python `iter_body_arguments` block in
  `_AnalyserCommandsMixin._process_command`.  `for` /
  `foreach` got dedicated body recursion in their
  handlers as well.
- **Rust-side gaps still absorbed by the hybrid
  supplement** (each is its own future port chunk that
  shrinks the supplement list):
  - **Class body parsing**: `oo::abstract` /
    `oo::singleton` / `oo::configurable` metaclasses
    unrecognised; `class_def.variables` + per-method
    `params` not extracted.
  - **Proc doc**: `ProcDef.doc` not extracted from a
    preceding comment.
  - **Per-scope `Scope.classes`** not threaded back
    (`result.all_classes` is populated; the per-scope
    map is empty).
  - `unset xs` argument not recorded as a per-scope
    variable.
  - The post-pass equivalent of `run_compiler_checks`
    (W110 / W220 / W304 emission) is not ported.
  - `unknown_proc_info` extraction lowers the user-defined
    `unknown` proc body to IR; tests patch
    `lower_to_ir` to assert the failure-suppression
    path, so Python's value is taken to honour those
    patches.
- Diagnostic-code-set deltas (`W113` dialect-label
  wording, `W214` over-emit on `[expr {$param}]`
  patterns) — exposed by the audit but absorbed by the
  hybrid (Python's diagnostics list is the superset);
  the field-by-field comparator does not assert on
  diagnostics.

## C41-default-on — flip the C41 dispatcher to Rust by default

Mirrors the C40-default-on shape.  Two phases:

- **Phase 1**: tighten the differential corpus's
  `_compare_shapes` helper from set-equality to field-by-field
  equality (per-proc params, per-class methods, scope-tree
  shape, `all_variables`, diagnostic-code parity).  Each
  fixture that fails the tightened check goes through one of
  three resolutions: Rust-side fix, Python-side fix (filed as
  a separate change), or removal from the corpus with an
  explanatory comment.  Lands when every fixture is
  field-by-field green.
- **Phase 2**: flip the dispatcher polarity in
  `core/analysis/_analyser/__init__.py::analyse` — change
  `rust_shim_enabled("TCL_LSP_RUST_ANALYSER", default=False)`
  to `default=True`.  Update the four dispatcher tests in
  `test_rust_analyser_differential.py` to assert the new
  polarity.  Once a release cycle has soaked, retire the
  Python `_AnalyserBase` mixin set and the env var.
