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
