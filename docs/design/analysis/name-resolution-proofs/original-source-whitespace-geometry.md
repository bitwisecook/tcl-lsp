# naming.editor.original-source-whitespace-geometry

Kind: `implementation-contract`

## Problem statement

Raw delimiter and whitespace scans can treat nested data words as command trivia or infer a script depth for a partial range.

## Question

Which original source owners supply whitespace breaks, continuation gaps and a selected formatting range without changing data words?

## Conclusion

Whitespace geometry consumes complete lexer-owned NativeWord plans under the exact full configuration. Trimming is clamped outside original whole words, including nested quoted data and bare Unicode units. Line breaks and bracket separator continuation collapse use only gaps between actual command words; literal, variable-index and quoted data retain their original spelling. Expression breaks come from the shared expression lexer and preserve complete string, variable and command terms. Range indentation follows lexical bracket regions and the formatter's retained source schema through shared original script-body, case and lambda geometry. A range must contain complete words in that selected script; malformed input, cooked bodies and partial data-word selections produce no edit. Inline expansion thresholds count complete commands through that same full-configuration lexical plan; quoted separators and comments cannot supply extra statements, and unavailable syntax remains outside the inline threshold.

## Scope

Source whitespace and presentation geometry only. Native source bodies require the same retained complete source/configuration and actual ContextRegistry; explicit Logical compatibility layout retains its separate whole context. Existing structural and recursive formatting limits remain. No Native recipe, name key, runtime frame, selected handler execution, compiler admission, cell contents, Normal or general source-reflection equivalence follows. Expression and parameter-list value rewrites keep their separate original equivalence receipts.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-geometry invariant.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-geometry invariant.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-geometry invariant.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-geometry invariant.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-geometry invariant.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native interpreter observation establishes this Rust source-geometry invariant.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native interpreter observation establishes this Rust source-geometry invariant.

## Exact evidence

No evidence artifact is attached.

- `own-source347-software-outcomes` (observation): [docs/design/analysis/name-resolution-proofs/rust-validation/compiler-core-runtime-owned347-experiments.json](../../../../docs/design/analysis/name-resolution-proofs/rust-validation/compiler-core-runtime-owned347-experiments.json). SHA-256 `903b68fc411ccd67d42d0756077e0fe6df85453fcec7cc3d7619374e33cc9f4c`. JSON pointer `/exact_question_body_outcomes`. Exact original-source347 body/question joins, closed owning producers, strict actual inventories/admissions and per-selector results. Current implementation source and unselected/new controls receive no result from these images.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/formatting/source_layout.rs](../../../../rust/tcl-lsp-core/src/formatting/source_layout.rs), `trim_trailing_whitespace`: Clamp trimming to trivia outside complete original words under the full grammar and preserve unsupported source exactly.
- [rust/tcl-lsp-core/src/formatting/source_layout.rs](../../../../rust/tcl-lsp-core/src/formatting/source_layout.rs), `separator_spaces`: Project actual command-word separator gaps from complete root and independently owned bracket script plans.
- [rust/tcl-lsp-core/src/formatting/source_layout.rs](../../../../rust/tcl-lsp-core/src/formatting/source_layout.rs), `collapse_script_separator_continuations`: Apply the shared continuation decoder only to actual command separator gaps, retaining nested word data spelling.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `find_expr_break_points`: Select only genuine top-level Boolean operator tokens under the complete expression lexical grammar; comments and unavailable terms decline.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `presentation_children`: Consume the actual selected source schema and shared original Body/case/lambda descriptors for readonly script regions, with a separate explicit Logical compatibility route.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting_range_region`: Validate retained complete original image/configuration/Registry correspondence and the actual selected-script region before presentation indentation; range/source advice grants no executable rewrite or Native entry.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `count_body_commands`: Use complete full-configuration source command plans for presentation thresholds, retaining comments, quoted terms, nested scripts and unavailable syntax at their shared owner.
- [rust/tcl-lsp-core/src/formatting/source_layout.rs](../../../../rust/tcl-lsp-core/src/formatting/source_layout.rs), `formatting::source_layout::tests::trimming_retains_nested_quoted_data_and_bare_unicode_units` (linked): Nested quoted spaces, bare Unicode units, CRLF and opaque escape spelling survive; malformed input remains exact.
- [rust/tcl-lsp-core/src/formatting/source_layout.rs](../../../../rust/tcl-lsp-core/src/formatting/source_layout.rs), `formatting::source_layout::tests::separators_belong_to_full_original_words_and_child_scripts` (linked): Actual root/bracket word gaps are available while braced, quoted and array-index data spaces remain unavailable; malformed source declines.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::expression_wrapping_preserves_string_terms_and_declines_comments` (linked): Expression string and command-term whitespace remains exact; expression comments and missing quotes decline wrapping.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::bracket_continuations_preserve_nested_word_data` (linked): Only a bracket command's genuine separator continuation collapses; quoted and braced inner word continuations remain original.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::multiline_braced_data_keeps_its_original_continuation` (linked): Original braced data remains byte-spelled across selected C Tcl and Jim source grammars instead of borrowing a universal continuation rewrite.
- [rust/tcl-lsp-core/src/formatting/mod.rs](../../../../rust/tcl-lsp-core/src/formatting/mod.rs), `formatting::tests::range_depth_uses_selected_bodies_and_ignores_quoted_braces` (linked): Only selected nested script bodies supply indentation depth; an earlier quoted data brace cannot alter it.
- [rust/tcl-lsp-core/src/formatting/mod.rs](../../../../rust/tcl-lsp-core/src/formatting/mod.rs), `formatting::tests::partial_data_word_ranges_preserve_exact_crlf_and_opaque_spelling` (linked): Selections inside quoted, bracket-contained, braced and unknown-role data words preserve original CRLF and opaque spellings with no edit.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::count_body_commands_counts_top_level_statements` (linked): Root command count keeps quoted/braced/bracket separators and comment contents distinct; unavailable syntax declines.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::command_count_retains_complete_body_grammar` (linked): The same source distinguishes selected brace-line continuation grammar and strict malformed-source refusal; no frame or command execution follows.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::original_formatter_range_uses_the_selected_script_prologue_purpose` (linked): Finite source-defined control: original formatter range uses the selected script prologue purpose. Root-applied selected range carries actual parser purpose and preserves document/body prologue distinction; client source coordinate guards remain independent. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

A named test is a coverage binding, not a claim that it executed.

## Replay

Linked fixed Rust selectors are authored coverage, not execution. Full formatter and range suites are required to validate integration. No native or Rust execution is attached; source evidence hashes remain held until the reviewed final freeze.


## Exact original-source347 software outcomes

These results belong only to the retained source347 body/question pairs and their owning Compiler or Core image. They do not establish an outcome for current source, unselected controls or external providers.

| Original operation | Passed | Failed |
| --- | ---: | ---: |
| `integration-core-consumer-tests347` | 1 | 0 |

[Exact original body hashes, producer/list/admission receipts, raw logs and owned images](rust-validation/compiler-core-runtime-owned347-experiments.json) retain the finite per-selector results and independent failed Runtime producer.


## Exact owning Core source350 software outcomes

The owning Core image executes 1 original selections paired with this question and all pass. These are bounded Rust software assertions for the exact source350 map and executable. The process selects 14 tests and all pass, with no ignored tests and 2,953 other tests filtered out. Producer and inventory operations execute no assertions.

[Exact original body hashes and raw harness outcomes](rust-validation/frozen350/question-body-outcomes.json) join the full unchanged receipts, streams, source map, inventory/admission and owned executable bytes. Native provider answers and other source-image results remain independent.
