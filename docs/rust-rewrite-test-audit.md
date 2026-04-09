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
| `synthetic_eof_token_uses_empty_static_text` | Added in the audit pass. Documents the `&""` pattern for EOF tokens the L3 Rust lexer will use. |
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
| `trailing_whitespace_still_emits_synthetic_eol` | Pins the `_at_command_start` preservation across SEP. |
| `trailing_newline_does_not_add_second_eol` | Guards against double-emitting the synthetic EOL. |
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
