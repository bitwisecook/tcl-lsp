# naming.refactor.selected-if-scalar-source-syntax

Kind: `implementation-contract`

## Problem statement

IF equality and OR source consumers can parse variable labels independently, losing selected brace/Unicode rules and original reference spelling. They need one scalar fragment classifier without promoting a source fragment to a genuine document operand or edit permission.

## Question

How do IF equality and OR data-group advice and if-to-switch consumers identify exactly one scalar substitution under selected Tcl grammar and preserve its whole spelling without array, compound or Native edit authority?

## Conclusion

ScalarVariableSourceSyntax classifies one explicitly supplied source fragment through the shared lexer, whole variable-reference and selected root/closed-element owners. It retains the fragment name and unchanged complete reference spelling, while OriginalScalarVariableSubject independently keeps its genuine original NativeWord. Both use scalar_syntax; the fragment carrier supplies no complete document input or original operand. Actual LexerConfig reaches Core data-group, MCP equality/OR shape and if-to-switch consumers. Core proposals keep original reference spelling and independently check generated lexical completeness. Existing extraction eligibility and RewriteObligation remain separate, with Native control-flow edits disabled.

## Scope

Five marked source/API controls cover actual C first-close versus C9 nested braces, Jim Unicode grammar, scalar unmatched parentheses and closed tails, quotes/braces/literal dollars, and original reference spelling. Arrays, inert values, command/compound substitutions, malformed references and mixed or negated OR branches refuse the bounded advice. Core fixed-space equality and MCP whitespace/regex equality shapes remain finite source classification; they are not full expression evaluation. A fragment result grants no actual AnalysisResult, complete original word, Native receiver/read/dispatch, entered frame/body, runtime equivalence or edit permission. Generated source completeness is lexical only. All seven external providers are not tested for this implementation contract.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Consumer bindings

- [rust/tcl-lsp-core/src/refactor/datagroup/source_subject.rs](../../../../rust/tcl-lsp-core/src/refactor/datagroup/source_subject.rs), `ScalarVariableSourceSyntax`: Retain selected fragment name and unchanged spelling without original document/input or operand authority.
- [rust/tcl-lsp-core/src/refactor/datagroup/source_subject.rs](../../../../rust/tcl-lsp-core/src/refactor/datagroup/source_subject.rs), `scalar_variable_source_syntax`: Require one complete scalar substitution in an explicitly supplied fragment under its actual lexer configuration.
- [rust/tcl-lsp-core/src/refactor/datagroup/source_subject.rs](../../../../rust/tcl-lsp-core/src/refactor/datagroup/source_subject.rs), `scalar_syntax`: Share the whole-reference/root/closed-element predicate between fragment syntax and genuine original subject projection.
- [rust/tcl-lsp-core/src/refactor/datagroup.rs](../../../../rust/tcl-lsp-core/src/refactor/datagroup.rs), `parse_eq`: Apply the shared selected scalar classifier within the bounded Core equality shape and keep original reference spelling for proposals.
- [rust/tcl-lsp-core/src/refactor/equality_source.rs](../../../../rust/tcl-lsp-core/src/refactor/equality_source.rs), `scalar_string_equality`: Retain the selected scalar fragment and original reference syntax through the shared configured expression equality classifier; read and edit authority remain independent.
- [rust/tcl-mcp/src/datagroup.rs](../../../../rust/tcl-mcp/src/datagroup.rs), `parse_eq`: Use bounded equality/whitespace shape only; variable classification delegates to shared actual-grammar scalar syntax.
- [rust/tcl-lsp-core/src/refactor/datagroup/source_subject.rs](../../../../rust/tcl-lsp-core/src/refactor/datagroup/source_subject.rs), `refactor::datagroup::source_subject::tests::scalar_fragment_syntax_preserves_selected_reference_boundaries` (linked): Selected fragment grammar preserves scalar root and complete spelling across C brace boundaries, Jim Unicode and quotes/tails; arrays, inert data, malformed or compound substitutions refuse.
- [rust/tcl-lsp-core/src/refactor/datagroup.rs](../../../../rust/tcl-lsp-core/src/refactor/datagroup.rs), `refactor::datagroup::tests::selected_if_scalar_source_keeps_reference_spelling_in_proposals` (linked): Core bounded IF equality advice reproduces the unchanged complete scalar reference rather than a reporting root, and checks proposed source completeness separately.
- [rust/tcl-lsp-core/src/refactor/datagroup.rs](../../../../rust/tcl-lsp-core/src/refactor/datagroup.rs), `refactor::datagroup::tests::selected_if_scalar_conditions_keep_eq_and_or_shape_under_actual_config` (linked): Actual lexical configuration governs scalar selection inside finite equality/OR shapes; mixed/negated branches and arrays cannot borrow a homogeneous scalar advice result.
- [rust/tcl-lsp-core/src/refactor/if_to_switch.rs](../../../../rust/tcl-lsp-core/src/refactor/if_to_switch.rs), `refactor::if_to_switch::tests::selected_if_switch_scalar_syntax_preserves_reference_and_edit_permission` (linked): IF-to-switch source advice retains original scalar spelling and lexical proposal completeness, while the independent Native rewrite obligation still refuses control-flow edits.
- [rust/tcl-mcp/src/datagroup.rs](../../../../rust/tcl-mcp/src/datagroup.rs), `datagroup::tests::selected_if_suggestions_share_actual_scalar_reference_grammar` (linked): MCP bounded equality/OR suggestions use the same actual-grammar scalar classifier for brace/Unicode/tail distinctions; regex shape supplies no variable-name grammar, original operand or execution authority.
- [rust/tcl-lsp-core/src/refactor/datagroup.rs](../../../../rust/tcl-lsp-core/src/refactor/datagroup.rs), `refactor::datagroup::original_arm_parser_tests::original_datagroup_arm_parsing_keeps_nested_marks_and_literal_extents` (linked): Finite source-defined control: original datagroup arm parsing keeps nested marks and literal extents. Proposal shape only; selected source handlers and edits remain separate. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

## Evidence

- [rust/tcl-lsp-core/src/refactor/datagroup/source_subject.rs](../../../../rust/tcl-lsp-core/src/refactor/datagroup/source_subject.rs)
- [rust/tcl-lsp-core/src/refactor/if_to_switch.rs](../../../../rust/tcl-lsp-core/src/refactor/if_to_switch.rs)
- [rust/tcl-mcp/src/datagroup.rs](../../../../rust/tcl-mcp/src/datagroup.rs)
