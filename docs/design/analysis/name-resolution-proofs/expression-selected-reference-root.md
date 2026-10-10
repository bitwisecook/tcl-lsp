# naming.expression.selected-reference-root

Kind: `implementation-contract`

## Problem statement

Expression analytical read enumeration must preserve whole scalar names, selected brace-close grammar and genuine array roots. Text stripping can truncate unmatched parentheses or literal dollar bytes and cannot retain opaque/counting source units.

## Question

How does a whole original expression variable reference select its analytical root under actual lexer grammar across permissive/checked Pratt, Jim term trees and Raw read enumeration without donating Native name/read authority?

## Conclusion

Syntax naming::variable_reference_root_bytes consumes exact source bytes plus actual LexerConfig and delegates whole_var_ref. Separate index syntax retains the lexical name; a braced combined name uses split_element_ref_bytes only with a final closing parenthesis, so an unmatched parenthesis or scalar tail remains part of the exact scalar name. Compound text returns None and malformed syntax returns the retained scanner error. Permissive and checked Pratt ingress, Jim term trees and Raw analytical read enumeration share this root selection and preserve their actual grammar. Raw collection calls the selected scan_var_ref at the original lexical token offset and requires its complete source_span to equal that token span before borrowing the whole original substitution; stripped token presentation cannot supply this purpose. UTF-8, opaque bytes, counted NUL and literal dollar source units remain source roots; they grant no native input extent, physical cell/read, current storage, cache/header identity, entered frame, compiler/SSA contents or execution admission. Independent native expression public values remain a separate question. The authored source normalise_var_name_for_style delegates to the same selected braced-name/closed-element partition. Unmatched opens and scalar tail text keep their whole root, while a complete closed element supplies its array root; this sigil-aware authored-source purpose must not replace a literal resolved-name owner.

## Scope

Five marked fixed Syntax/source controls cover complete scalar and closed-combined roots, selected C8/Jim first-close versus C9 nested-close grammar, malformed/compound refusal, checked opaque bytes, Jim UTF-8/opaque term roots and Raw reads. All seven provider observations are not tested for this source/API contract; the actual public binary-format expression originals do not prove analytical AST/private object behaviour.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `naming-expression-selected-reference-root-ast.rs` (implementation): [rust/tcl-syntax/src/expr/ast.rs](../../../../rust/tcl-syntax/src/expr/ast.rs). SHA-256 `413ac4dfb166e72b5dab1ae7db62c2a65f30e66afd82b8d7bbf9810904bce4b7`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-expression-selected-reference-root-checked.rs` (implementation): [rust/tcl-syntax/src/expr/checked.rs](../../../../rust/tcl-syntax/src/expr/checked.rs). SHA-256 `4a5b64ae4d0d306812314ac3bad4090cbb82d4adde0f599fdb23741a2dbc93c7`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-expression-selected-reference-root-jim_function_tree.rs` (implementation): [rust/tcl-syntax/src/expr/jim_function_tree.rs](../../../../rust/tcl-syntax/src/expr/jim_function_tree.rs). SHA-256 `c6f234bfda852253078f979efa2e904c07c147fb03c5ac708b5420f0afb860f4`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-expression-selected-reference-root-parser.rs` (implementation): [rust/tcl-syntax/src/expr/parser.rs](../../../../rust/tcl-syntax/src/expr/parser.rs). SHA-256 `81589fcdb23d65a7ea0b240e2e9ac4f4ac9663c01851411acf8c8c6932b3c4ad`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-expression-selected-reference-root-naming.rs` (implementation): [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs). SHA-256 `a4487cc75c01e586796348f4da4071687669ee8a703669e0ff5c36edd6edb373`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `variable_reference_root_bytes`: Retain complete whole-reference source/actual lexical grammar and shared closed combined-name partition; reject compound or malformed references without Native authority.
- [rust/tcl-syntax/src/expr/ast.rs](../../../../rust/tcl-syntax/src/expr/ast.rs), `collect_raw_vars_with`: Require the actual selected scanner whole-reference extent to match the original lexical token span before borrowing complete source; no reconstructed sigil or closer.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `naming::tests::selected_reference_roots_keep_whole_scalar_and_combined_element_bytes` (linked): Complete scalar unmatched/tail parentheses, literal dollars, UTF-8/opaque/NUL source bytes remain exact; only genuinely closed combined elements partition the analytical array root, with compound/malformed refusal.
- [rust/tcl-syntax/src/expr/parser.rs](../../../../rust/tcl-syntax/src/expr/parser.rs), `expr::parser::tests::all_expression_ingress_keeps_selected_reference_roots_and_close_rules` (linked): All permissive expression ingress uses actual selected brace-close grammar and shared analytical root selection; whole scalar tails and array roots remain distinct.
- [rust/tcl-syntax/src/expr/checked.rs](../../../../rust/tcl-syntax/src/expr/checked.rs), `expr::parser::checked::native_byte_tests::checked_reference_roots_keep_selected_grammar_and_opaque_source` (linked): Checked original byte-source expression references retain actual grammar, opaque/counting units and exact source roots without physical Native read/header proof.
- [rust/tcl-syntax/src/expr/jim_function_tree.rs](../../../../rust/tcl-syntax/src/expr/jim_function_tree.rs), `expr::jim_function_tree::tests::jim_reference_tree_shares_selected_utf8_and_opaque_roots` (linked): Jim analytical term trees use shared selected root partition for UTF-8/opaque scalar and combined array syntax, without executing a Jim provider.
- [rust/tcl-syntax/src/expr/ast.rs](../../../../rust/tcl-syntax/src/expr/ast.rs), `expr::ast::tests::raw_expression_reads_share_selected_scalar_and_array_roots` (linked): Raw analytical reads retain complete original scalar tails, literal dollar roots and closed combined-element names under actual lexical grammar; collection admits only scanner/token source-span correspondence, without physical Native read/header proof.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `naming::tests::scalar_parentheses_share_the_closed_element_partition` (linked): Both selected brace-close styles retain complete unmatched/tail scalar roots, repeated closed index parentheses and literal-dollar authored names consistently through normalise/split source projection. No Native cell/read/header or resolved-name sigil reinterpretation is granted.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
