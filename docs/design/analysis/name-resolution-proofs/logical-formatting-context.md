# naming.editor.logical-formatting-context

Kind: `implementation-contract`

## Problem statement

Heterogeneous last-registered Registry descriptors can replace the documented compatibility formatter context.

## Question

How does the compatibility formatter retain its documented lenient modern-Tcl metadata when a supplied Registry contains multiple engine descriptors?

## Conclusion

The compatibility layout retains one complete ResolvedAnalysisInput with the actual lenient context, supplied Registry semantic key and full lexer configuration. Its shared positioned source-transition schema selects authored roles, traits and case presentation under that whole context. A positively retained Logical input distinguishes compatibility edits from Native readonly layout. Complete original words retain source geometry without acquiring Native inputs. Logical lambda fields use the same word’s full source and list grammar. A last-registered descriptor cannot choose another engine's layout. Changed configuration or Registry declines the projection; absent runtime policy remains absent.

## Scope

Explicit compatibility source metadata and existing Logical formatting behavior. Actual original-source formatting continues to consume its whole retained analysis input and independent original operands/equivalence receipts. No native string recipe, original name key, handler identity, compiler admission, activation or Normal follows from the compatibility projection.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation establishes this Rust metadata selection invariant.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation establishes this Rust metadata selection invariant.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation establishes this Rust metadata selection invariant.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation establishes this Rust metadata selection invariant.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation establishes this Rust metadata selection invariant.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No interpreter observation establishes this Rust metadata selection invariant.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No interpreter observation establishes this Rust metadata selection invariant.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `FormattingSourceLayout::new`: Retain one whole context, full config and supplied Registry generation for compatibility and original-source layouts.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `identify_body_args`: Select authored source roles, traits and case presentation through the exact actual context; original operands remain separate.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `keyword_rewrites_for`: Retain that same Logical context query for candidate metadata and decline unavailable full-config correspondence.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `case_list_body_index`: Consume the selected contextual case presentation instead of last-registered metadata.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `reconstruct_command`: Consume the same selected source traits without native rewrite grants.
- [rust/tcl-compiler/src/lambda_literal.rs](../../../../rust/tcl-compiler/src/lambda_literal.rs), `split_original_lambda_literal_lexical_decoded`: Decode only a genuine complete Document/braced/non-expanded word with its retained full source/list grammar; no Native value, name key or cooked source extent.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::logical_default_formatting_retains_its_whole_context_over_mixed_specs` (linked): Mixed descriptors retain the intended Tcl-family authored schema with genuine original words and without Native inputs/recipes; changed full config declines context and keyword projection.
- [rust/tcl-compiler/src/lambda_literal.rs](../../../../rust/tcl-compiler/src/lambda_literal.rs), `lambda_literal::tests::original_lexical_lambda_decoding_retains_word_config_and_channel` (linked): Whole original Document/braced/non-expanded words retain literal-versus-folded continuation grammar; NativeValue channel refuses this lexical view.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact Rust coverage is linked without execution. Existing compatibility procedure and expression formatter tests provide additional consumer checks; native behavior cannot establish the formatter's own retained-context invariant.
