# naming.expression.selected-reference-root

Kind: `implementation-contract`

## Problem statement

Expression analytical read enumeration must preserve whole scalar names, selected brace-close grammar and genuine array roots. Text stripping can truncate unmatched parentheses or literal dollar bytes and cannot retain opaque/counting source units.

## Question

How does a whole original expression variable reference select its analytical root under actual lexer grammar across permissive/checked Pratt, Jim term trees and Raw read enumeration without donating Native name/read authority?

## Conclusion

Syntax naming::variable_reference_root_bytes consumes exact source bytes plus actual LexerConfig and delegates whole_var_ref. Separate index syntax retains the lexical name; a braced combined name uses split_element_ref_bytes only with a final closing parenthesis, so an unmatched parenthesis or scalar tail remains part of the exact scalar name. Compound text returns None and malformed syntax returns the retained scanner error. The three linked permissive Pratt APIs, checked NativeSyntax byte-source parsing, Jim term trees and Raw analytical read enumeration share this root selection under their admitted grammar. The profile entry independently applies the fixed Tcl 8.4 preflight and can conservatively return Raw for a nesting override that the separate grammar/context topology entries accept. Raw collection calls the selected scan_var_ref at the original lexical token offset and requires its complete source_span to equal that token span before borrowing the whole original substitution; stripped token presentation cannot supply this purpose. UTF-8, opaque bytes, counted NUL and literal dollar source units remain source roots; they grant no native input extent, physical cell/read, current storage, cache/header identity, entered frame, compiler/SSA contents or execution admission. Independent native expression public values remain a separate question. The authored source normalise_var_name_for_style delegates to the same selected braced-name/closed-element partition. Unmatched opens and scalar tail text keep their whole root, while a complete closed element supplies its array root; this sigil-aware authored-source purpose must not replace a literal resolved-name owner. The SourceTopology ingress reuses the shared Pratt variable worker with a separate source-only admission purpose; its existence does not extend these root-control definitions to every ingress or certify Native syntax. Its admission boundary and topology controls are described by naming.grammar.checked-expression-substitution-context.

## Scope

Six linked source definitions cover shared naming roots, three named permissive Pratt APIs, checked byte-source parsing under selected NativeSyntax contexts, Jim analytical term roots and Raw read enumeration. The permissive control calls parse_expr_for_profile, parse_expr_with_grammar and parse_expr_with_syntax_context; it does not call the SourceTopology ingress. The checked-byte control calls parse_expr_bytes_checked_with_context under C/Jim contexts and selected brace styles. SourceTopology uses the same Pratt variable worker, but this record attaches no positive scalar/array/brace-root control for that ingress. Its separate admission/refusal controls belong to naming.grammar.checked-expression-substitution-context. All six named syntax controls passed in the separately retained follow-up receipt; the earlier failed permissive control and diagnostic probe remain retained separately. No other execution result follows. All seven provider observations remain not tested for this source/API contract; public binary-format expression originals do not prove analytical AST/private object behaviour.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Consumer bindings

- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `variable_reference_root_bytes`: Retain complete whole-reference source/actual lexical grammar and shared closed combined-name partition; reject compound or malformed references without Native authority.
- [rust/tcl-syntax/src/expr/ast.rs](../../../../rust/tcl-syntax/src/expr/ast.rs), `collect_raw_vars_with`: Require the actual selected scanner whole-reference extent to match the original lexical token span before borrowing complete source; no reconstructed sigil or closer.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `naming::tests::selected_reference_roots_keep_whole_scalar_and_combined_element_bytes` (linked): Complete scalar unmatched/tail parentheses, literal dollars, UTF-8/opaque/NUL source bytes remain exact; only genuinely closed combined elements partition the analytical array root, with compound/malformed refusal.
- [rust/tcl-syntax/src/expr/parser.rs](../../../../rust/tcl-syntax/src/expr/parser.rs), `expr::parser::tests::all_expression_ingress_keeps_selected_reference_roots_and_close_rules` (linked): The three named APIs preserve the fixed scalar/element roots; grammar and context entries follow the selected FirstClose/Tcl9Nesting styles, while the Tcl 8.4 profile entry retains its independent fixed preflight and returns Raw for the nested override case. This control does not invoke parse_source_expression_checked or prove every ingress. The corrected named software control passed in the separately retained scoped receipt; no provider outcome follows.
- [rust/tcl-syntax/src/expr/checked.rs](../../../../rust/tcl-syntax/src/expr/checked.rs), `expr::parser::checked::native_byte_tests::checked_reference_roots_keep_selected_grammar_and_opaque_source` (linked): Checked byte-source NativeSyntax parsing through parse_expr_bytes_checked_with_context retains scalar/array roots, opaque/counting units and selected brace rules under C/Jim contexts. SourceTopology is not invoked, and physical Native read/header authority is not supplied. This named syntax control passed in the separately retained follow-up receipt; no provider outcome is attached.
- [rust/tcl-syntax/src/expr/jim_function_tree.rs](../../../../rust/tcl-syntax/src/expr/jim_function_tree.rs), `expr::jim_function_tree::tests::jim_reference_tree_shares_selected_utf8_and_opaque_roots` (linked): Jim analytical term trees use shared selected root partition for UTF-8/opaque scalar and combined array syntax, without executing a Jim provider.
- [rust/tcl-syntax/src/expr/ast.rs](../../../../rust/tcl-syntax/src/expr/ast.rs), `expr::ast::tests::raw_expression_reads_share_selected_scalar_and_array_roots` (linked): Raw analytical reads retain complete original scalar tails, literal dollar roots and closed combined-element names under actual lexical grammar; collection admits only scanner/token source-span correspondence, without physical Native read/header proof.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `naming::tests::scalar_parentheses_share_the_closed_element_partition` (linked): Both selected brace-close styles retain complete unmatched/tail scalar roots, repeated closed index parentheses and literal-dollar authored names consistently through normalise/split source projection. No Native cell/read/header or resolved-name sigil reinterpretation is granted.

## Evidence

- [rust/tcl-syntax/src/expr/ast.rs](../../../../rust/tcl-syntax/src/expr/ast.rs)
- [rust/tcl-syntax/src/expr/checked.rs](../../../../rust/tcl-syntax/src/expr/checked.rs)
- [rust/tcl-syntax/src/expr/jim_function_tree.rs](../../../../rust/tcl-syntax/src/expr/jim_function_tree.rs)
- [rust/tcl-syntax/src/expr/parser.rs](../../../../rust/tcl-syntax/src/expr/parser.rs)
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs)
