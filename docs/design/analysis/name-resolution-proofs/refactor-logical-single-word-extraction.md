# naming.refactor.logical-single-word-extraction

Kind: `implementation-contract`

## Problem statement

A multiword selection such as helper {a and b} $x cannot be appended to a setter as one value; doing so emits a wrong-arity command. An operator substring guess can also interpret ordinary prose as an expression. Line-zero insertion into a one-line procedure moves the new variable to another frame. Applicability depends on a complete value word and the original body position.

## Question

Does explicit lexical variable extraction preserve one complete value word and actual body placement while refusing unsupported expression construction?

## Conclusion

The explicit lexical branch requires current complete source/configuration, a safe authored scalar identifier, actual Registry setter metadata and an original whole argument word. It inserts inside the original command body when a line prefix is not whitespace. Multiple value words, invalid names and absent independently installed Logical expression parser/evaluator refuse. This bounded authoring branch supplies no Native name, cell, Normal, store or motion authority.

## Scope

Lexical-only Core and MCP variable extraction compatibility. Single braced/quoted or command-substitution words retain their original source spelling; general multiword expression extraction remains unavailable without the separately installed checked Logical parser/evaluator.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This Rust consumer contract has no native execution receipt.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This Rust consumer contract has no native execution receipt.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/refactor/extract_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_variable.rs). SHA-256 `e20a036e18d1e32eba6c203704facbe9224c40c349cfbfe8f256d8a55eec29b2`. Current shared owner, consumer and fixed validation assertions. No executed Rust or native result is implied.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/refactor/extract_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_variable.rs), `logical_value_word`: Require a complete lexical value word without operator substring inference.
- [rust/tcl-lsp-core/src/refactor/extract_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_variable.rs), `unique_logical_command`: Select authored setter metadata from the actual Registry rather than hardcoding a default Registry.
- [rust/tcl-lsp-core/src/refactor/extract_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_variable.rs), `refactor::extract_variable::tests::logical_complete_value_words_keep_explicit_compatibility_behaviour` (linked): Valid complete literal and substitution words preserve explicit lexical behaviour.
- [rust/tcl-lsp-core/src/refactor/extract_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_variable.rs), `refactor::extract_variable::tests::logical_invalid_multiword_value_never_becomes_a_four_argument_set` (linked): Invalid multiword values, absent parser and unsafe names refuse.
- [rust/tcl-lsp-core/src/refactor/extract_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_variable.rs), `refactor::extract_variable::tests::logical_one_line_body_insertion_remains_inside_the_original_body` (linked): One-line procedure insertion stays inside its actual body.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named fixed Rust selectors bind validation obligations. No Rust execution receipt is attached to this implementation contract. No native interpreter experiment or overall passing suite is claimed. Missing authority and unsupported source purposes remain explicit in the scope and conclusion.
