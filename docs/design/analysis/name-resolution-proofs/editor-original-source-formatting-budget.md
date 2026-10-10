# naming.editor.original-source-formatting-budget

Kind: `implementation-contract`

## Problem statement

Semantic source analysis can consume unbounded time before the formatter reaches its existing recursive formatting depth guard.

## Question

How does formatting apply its source structural resource budget before semantic analysis?

## Conclusion

The existing Lexer braced-word scanner retains its actual maximum brace nesting under the supplied full config. Formatting checks the existing128 budget before creating semantic source owners. Withdrawal preserves the complete original input; range formatting emits no edits.

## Scope

Rust resource admission only. Literal braces in comments, quoted text, bare words and escape spans keep actual lexical treatment. Deep inert data conservatively preserves original source. No body/name/frame/evaluation or rewrite purpose is inferred. Other recursive caps, deterministic2,000-level test and default stack remain unchanged. Timing/pass results require actual execution.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust source resource invariant.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust source resource invariant.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust source resource invariant.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust source resource invariant.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust source resource invariant.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No interpreter observation is attached to this Rust source resource invariant.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No interpreter observation is attached to this Rust source resource invariant.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lexer/src/lexer.rs](../../../../rust/tcl-lexer/src/lexer.rs), `Lexer::braced_word_nesting_within`: Consume the existing selected scanner and apply a caller resource bound to actual braced-word nesting.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `source_within_formatting_budget`: Use the existing formatter depth limit and complete LexerConfig for source resource admission.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `format_tcl_impl`: Preserve complete original source before semantic owner creation when lexical resources are outside the bound.
- [rust/tcl-lsp-core/src/formatting/mod.rs](../../../../rust/tcl-lsp-core/src/formatting/mod.rs), `range_formatting_impl`: Withdraw edits before whole-document semantic analysis for an over-budget resource request.
- [rust/tcl-lexer/src/lexer.rs](../../../../rust/tcl-lexer/src/lexer.rs), `lexer::tests::brace_word_budget_uses_lexical_context_and_escape_boundaries` (linked): Actual nested braced words exceed the selected budget; escaped braces, comment text, quoted bodies and bare-word data do not acquire brace-word nesting.
- [rust/tcl-lexer/src/lexer.rs](../../../../rust/tcl-lexer/src/lexer.rs), `lexer::tests::brace_word_budget_retains_full_config_and_strict_errors` (linked): C and Jim word-separator geometry differs under the actual supplied grammar; strict malformed-word errors remain errors.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::formatting_budget_preserves_complete_source_before_analysis` (linked): Over-budget input preserves exact original CRLF/source bytes despite ordinary whitespace/final-newline options.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::deeply_nested_if_survives_formatting` (linked): The unchanged deterministic2,000-level formatter selector returns on the default stack.
- [rust/tcl-lsp-core/src/formatting/mod.rs](../../../../rust/tcl-lsp-core/src/formatting/mod.rs), `formatting::tests::range_formatting_withdraws_before_analysis_above_lexical_budget` (linked): A range request on over-budget original source emits no edits before semantic preparation.

A named test is a coverage binding, not a claim that it executed.

## Replay

Linked tests are not an execution claim. Existing2,000-level/defaultstack selector must be executed without changing depth, stack or timeout.
