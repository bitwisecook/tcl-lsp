# naming.core.original-data-brace-classification

Kind: `implementation-contract`

## Problem statement

Variable cursor providers and caller-reference scans can read a variable-shaped substring inside literal data. A nominal command-name role lookup and private delimiter arithmetic neither retain the original effective operand nor distinguish missing ownership from actual source-code applicability.

## Question

How do all variable cursor and lexical caller-reference consumers distinguish original literal data, potential script/expr source and unavailable ownership under one current complete analysis?

## Conclusion

Core offset_in_data_brace_in_analysis accepts the complete retained AnalysisResult and returns Option<bool>. SourceStructure owns original nested regions; native_script_words_in owns whole word extents. The shared original Registry source schema and authentic effective argument origins preserve captured alias prefixes, shadows, case and lambda geometry. Shared expression terms distinguish expression string literals from potential substitution source. None preserves stale, partial, unknown and cooked geometry. Every variable cursor and lexical caller-reference gate consumes this one classification; Scope lookup requires independently enabled lexical declaration advice. Native providers retain their separate original variable naming owners. AuthoredSourceExpressionArguments owns expression ordinal/concatenation grammar. Multiple effective contributors retain unknown until an independently mapped joined original expression is available; no individual contributor is reinterpreted as its own expression or inert data.

## Scope

Original source applicability and lexical compatibility selection only. Conditional source metadata remains conditional. Some(false) does not attest an actual runtime variable substitution, entered frame, handler, complete reference inventory or edit permission. The existing comment classifier is unchanged and has its own narrower source premises.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust source-consumer invariant.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust source-consumer invariant.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust source-consumer invariant.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust source-consumer invariant.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust source-consumer invariant.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No interpreter observation is attached to this Rust source-consumer invariant.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No interpreter observation is attached to this Rust source-consumer invariant.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/inert_text.rs](../../../../rust/tcl-lsp-core/src/inert_text.rs), `offset_in_data_brace_in_analysis`: Classify source data under actual complete retained ownership and shared word/schema geometry; preserve unavailable outcomes.
- [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs), `SourceStructure::capture`: Own original bracket, Body, case, lambda and declaration source regions without runtime entry or frames.
- [rust/tcl-lsp-core/src/definition.rs](../../../../rust/tcl-lsp-core/src/definition.rs), `substituting_var_at_position`: Share the actual-analysis cursor classification and positive lexical Scope gate across definition, hover, references, highlights and rename.
- [rust/tcl-lsp-core/src/definition.rs](../../../../rust/tcl-lsp-core/src/definition.rs), `lookup_var_read_at`: Share the actual-analysis cursor classification and positive lexical Scope gate across definition, hover, references, highlights and rename.
- [rust/tcl-lsp-core/src/definition.rs](../../../../rust/tcl-lsp-core/src/definition.rs), `offset_is_inert`: Share the actual-analysis cursor classification and positive lexical Scope gate across definition, hover, references, highlights and rename.
- [rust/tcl-lsp-core/src/caller_frame.rs](../../../../rust/tcl-lsp-core/src/caller_frame.rs), `substituted_var_read_at`: Delegate cursor and lexical caller-reference classification to the same retained analysis gate; Native callers use independently owned original roots.
- [rust/tcl-lsp-core/src/caller_frame.rs](../../../../rust/tcl-lsp-core/src/caller_frame.rs), `substituted_read_spans`: Delegate cursor and lexical caller-reference classification to the same retained analysis gate; Native callers use independently owned original roots.
- [rust/tcl-lsp-core/src/inert_text.rs](../../../../rust/tcl-lsp-core/src/inert_text.rs), `inert_text::tests::a_dollar_ref_inside_a_data_brace_is_inert` (linked): Original top-level/nested/Unicode braced data is classified from complete original word geometry.
- [rust/tcl-lsp-core/src/inert_text.rs](../../../../rust/tcl-lsp-core/src/inert_text.rs), `inert_text::tests::a_dollar_ref_inside_a_script_brace_keeps_source_applicability` (linked): Shared Body, expression, bracket, case-arm and lambda source geometry supplies a source-code position; no runtime variable receipt is asserted.
- [rust/tcl-lsp-core/src/inert_text.rs](../../../../rust/tcl-lsp-core/src/inert_text.rs), `inert_text::tests::original_case_lambda_and_expression_literals_keep_data_separate` (linked): A case pattern and cooked lambda body preserve unknown; original lambda parameters and braced expression strings remain data, while quoted expression substitution source stays available.
- [rust/tcl-lsp-core/src/inert_text.rs](../../../../rust/tcl-lsp-core/src/inert_text.rs), `inert_text::tests::original_data_classification_keeps_alias_prefix_and_shadow_boundaries` (linked): Original written operands map through captured alias prefixes; known replacement and unknown braced head schema do not borrow nominal command metadata.
- [rust/tcl-lsp-core/src/inert_text.rs](../../../../rust/tcl-lsp-core/src/inert_text.rs), `inert_text::tests::original_data_classification_declines_stale_or_unowned_inputs` (linked): Changed full bytes, changed complete lexer configuration, default results and incomplete delimiters withdraw source classification.
- [rust/tcl-lsp-core/src/inert_text.rs](../../../../rust/tcl-lsp-core/src/inert_text.rs), `inert_text::tests::original_expression_contributors_do_not_borrow_independent_data_roles` (linked): First/later written operands and captured alias expression prefixes preserve unknown without joined source ownership; a single original expression still distinguishes actual string-literal and potential substitution source.

A named test is a coverage binding, not a claim that it executed.

## Replay

Linked Rust selectors require actual root validation. No native interpreter execution or target semantics are inferred from source geometry.
