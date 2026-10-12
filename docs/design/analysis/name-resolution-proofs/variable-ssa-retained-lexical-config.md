# naming.variable.ssa-retained-lexical-config

Kind: `implementation-contract`

## Problem statement

A source scanner can own grammar and lexical coordinates that differ from the Registry profile. Replacing its configuration changes body-local classification and nested expression heads, creating or losing SSA read dependencies.

## Question

How do SSA source ownership and nested substitution roles retain the scanner's actual lexical configuration?

## Conclusion

scan_command_words passes the retained scanner configuration to braced-word ownership classification. scan_nested_substitution_words uses that same configuration to recover nested calls and authentic entered or selected expression roles. Grammar, nested coordinate normalization and list parsing remain owned by the scanner; a Registry profile cannot replace a vertical-tab-containing Jim head with a Tcl expression command.

## Scope

Two fixed source-classification selectors use a Tcl9.0 grammar and an independently supplied Jim grammar with nondefault source coordinates and BOM policy. Unknown wrapper bodies distinguish vertical-tab command separation and adjacent list elements; a nested expression head distinguishes Tcl and Jim lexical roles. These classifications establish no wrapper execution, selected runtime handler, physical read/cell, Normal completion or compiler admission. Native provider answers remain independent.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No native provider execution is attached to this source classification contract.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No native provider execution is attached to this source classification contract.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No native provider execution is attached to this source classification contract.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No native provider execution is attached to this source classification contract.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No native provider execution is attached to this source classification contract.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No native provider execution is attached to this source classification contract.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native provider execution is attached to this source classification contract.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `scan_command_words`: Classify braced source ownership using the actual scanner lexer configuration.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `scan_nested_substitution_words`: Recover nested substitution boundaries and expression roles using retained scanner grammar.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `ssa::tests::source_ownership_classification_uses_the_retained_lexical_config` (linked): Two fixed unknown-wrapper bodies yield opposite quoted/substituted classifications under Tcl and Jim retained grammar; nondefault coordinates normalize only through the scanner owner.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `ssa::tests::nested_source_expression_roles_use_the_retained_lexical_config` (linked): A vertical-tab nested head carries a Tcl expression role but remains a distinct Jim command head; the Registry label cannot supply a substituted read.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named body-marked Rust selectors are authored coverage bindings. Executed outcomes require independent exact-source receipts; no native observation or Rust success is attached here.
