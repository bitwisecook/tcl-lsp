# naming.diagnostics.original-variable-name-anchor

Kind: `implementation-contract`

## Problem statement

A diagnostic anchor can confuse an already resolved literal dollar with substitution syntax, or truncate a scalar at an unmatched parenthesis. Callback-shaped slot hints also need the same literal combined-name boundary instead of an independent split.

## Question

How do variable-read anchors retain the exact selected lexical source/configuration extent and literal resolved name, while callback-shaped slot hints share combined-name parsing without asserting a current variable read or installed callback?

## Conclusion

Analyser::narrow_to_read_var scans the actual statement source slice with its retained LexerConfig and delegates original substitution spelling/extent to scan_var_ref. The resolved target and scanner-returned name use split_element_ref without stripping a literal dollar; only a final closing parenthesis selects an array base. Unmatched-parenthesis scalars, braced literal-dollar names, Unicode, empty names and complete element closers retain their exact source spans. The bounded command-substitution descent uses the same config; braced literal words do not donate reads, unmatched lexical extent refuses and recursion beyond four levels supplies no narrow anchor. Selected C8/Jim versus C9 braced-name grammar and Jim bare-Unicode grammar remain distinct source parser controls. is_callback_array_slot delegates the same resolved combined-name split before its existing callback-shaped key hint; that hint establishes no occupancy, current callback, successful read, warning applicability, storage contents or Native pointer/frame purpose.

## Scope

Three fixed source/API controls verify exact variable-reference span/name correspondence across six selected lexical dialects and literal combined-name callback hints. The fixture supplies an actual selected analysis input and source/configuration to the private anchor helper; it does not execute a provider or prove a completed analysis warning/read. Nested command source, braced literal negatives, unmatched extents, Unicode/bare-braced distinctions and callback-shaped suffixes retain their own bounded grammar. All seven native providers are not tested.

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

- `naming-diagnostics-original-variable-name-anchor-dataflow.rs` (implementation): [rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs). SHA-256 `642cc6f4207c8194beb6a5e18daf948b450f0484d8c0173f7fa152bb95dae498`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-diagnostics-original-variable-name-anchor-var_command.rs` (implementation): [rust/tcl-compiler/src/analyser/diagnostics/var_command.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/var_command.rs). SHA-256 `501983691f47cdd87ddbf4d90f1496da3872179f217af0aacd5e2b42e9516630`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-diagnostics-original-variable-name-anchor-word_parts.rs` (implementation): [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs). SHA-256 `101e7e555853dd34bc2dd0307d2efa420387c1e45cf5c9fde6a3f0c76726b56d`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-diagnostics-original-variable-name-anchor-naming.rs` (implementation): [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs). SHA-256 `77c05067c4a55d218010c4225b664fbb07d16624293457299a52d96e3912c6fc`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs), `Analyser::narrow_to_read_var`: Use retained selected LexerConfig and actual source slice for bounded complete variable-reference geometry; resolved names remain literal.
- [rust/tcl-compiler/src/analyser/diagnostics/var_command.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/var_command.rs), `is_callback_array_slot`: Share already-resolved combined-name boundary before an existing callback-shaped key hint; no current callback/read fact.
- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `scan_var_ref`: Own original substitution syntax and complete selected lexical extent independently of literal resolved target parsing.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `split_element_ref`: Own literal scalar/element boundaries without stripping a resolved dollar or unmatched-parenthesis suffix.
- [rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs), `analyser::diagnostics::dataflow::original_variable_anchor_tests::variable_read_anchor_preserves_literal_name_and_complete_reference_geometry` (linked): Selected lexical inputs preserve complete scalar/dollar/Unicode/empty/element closer spans and nested source geometry; braced literals, unmatched extents and mismatched resolved targets refuse. This anchor control proves no W210 admission, variable existence or native read completion.
- [rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs), `analyser::diagnostics::dataflow::original_variable_anchor_tests::variable_read_anchor_uses_retained_braced_and_bare_name_grammar` (linked): Actual retained selected C8/Jim versus C9 braced-name grammar and Jim bare Unicode give their exact original reference spans. Grammar selection supplies no Native name, current cell or successful read.
- [rust/tcl-compiler/src/analyser/diagnostics/var_command.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/var_command.rs), `analyser::diagnostics::var_command::original_callback_slot_name_tests::callback_slot_hints_use_shared_resolved_element_name_boundaries` (linked): Literal resolved dollar, Unicode, empty array base and compound key geometry share final-parenthesis parsing; unmatched/trailing scalars and ordinary keys refuse the callback-shaped hint. No occupancy, callback registration or executed variable read is inferred.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
