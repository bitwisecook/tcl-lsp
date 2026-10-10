# naming.source.original-refactor-command-selection

Kind: `implementation-contract`

## Problem statement

A refactor selecting body commands through String-only Registry queries can retain the outer declaration instead of the inner command. Reconstructing a body from a decoded value can also invent source coordinates.

## Question

Can a refactor select the innermost command from authentic current source body geometry without acquiring execution or edit authority?

## Conclusion

The original command finder delegates body, lambda, substitution and command geometry to the shared source structure and Registry source-schema owners under the actual retained analysis and full current source configuration. It selects the innermost unambiguous command and rejects a changed owning source. Reporting maps do not supply missing body roles or lexical extents. Decoded quoted body values do not become original inner source commands. The selected command is readonly source geometry; callers must independently establish frame, current cells, Normal execution, store preservation, motion and edit eligibility.

## Scope

Rust readonly refactor selection contract. Six selected C/Jim source profiles exercise an actual procedure body with erased reports; separate lambda and command-substitution controls retain their own extents. This selection supplies no physical frame, executed script, native object, runtime success or appliance context.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/refactor/source_command.rs](../../../../rust/tcl-lsp-core/src/refactor/source_command.rs), `find_original_command_at`: Select innermost unambiguous authentic current command geometry without interpreting its head as a name or donating an edit permission.
- [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs), `SourceStructure`: Own bounded Body, Lambda, expression and bracket source extents through the retained source-role schema and actual parser configuration.
- [rust/tcl-compiler/src/registry_invocation/source_structure.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_structure.rs), `source_registry_words`: Keep source schema, complete original operand producers, actual context and conditional applicability separate from execution facts.
- [rust/tcl-lsp-core/src/refactor/source_command.rs](../../../../rust/tcl-lsp-core/src/refactor/source_command.rs), `refactor::source_command::tests::original_command_cursor_uses_actual_body_geometry_after_reports_are_erased` (linked): Selects the actual nested setter after procedure, invocation and variable reports are cleared across six profiles, and rejects a full-source change elsewhere at the same cursor.
- [rust/tcl-lsp-core/src/refactor/source_command.rs](../../../../rust/tcl-lsp-core/src/refactor/source_command.rs), `refactor::source_command::tests::original_command_cursor_keeps_lambda_and_substitution_extents_separate` (linked): Keeps lambda-body and command-substitution positions distinct and does not treat a decoded quoted body value as original inner command geometry.

A named test is a coverage binding, not a claim that it executed.

## Replay

Run the exact linked Rust selectors through the maintained workspace test setup. No native provider observation or executed Rust result is attached to this implementation record.
