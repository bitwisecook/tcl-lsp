# naming.optimiser.original-definition-resurrection-guard

Kind: `implementation-contract`

## Problem statement

A final rewrite can expose a variable reference after a proposed defining statement removal. The removal guard needs selected genuine source operands and canonical place owners; whitespace-split reporting text loses quoted/qualified/captured names or invents absence when original ownership is missing.

## Question

How does optimiser finalisation retain authentic original definition operands and cell projections when checking whether another surviving replacement resurrects a reference?

## Conclusion

elim_target_vars selects an actual original CFG statement and retained source binding under matching module source, lexer configuration and command store, and, for an empty-replacement O112 structure removal (including a constant-false `if`, `while` or `for`, or a `switch` with no selected body), only among the statements the unit's solver proves executable (a complexity-guarded unit keeps every statement): a body the structure never enters defines nothing, so its removal withdraws no definition. Other removals, O107 included, keep every statement in the span. normal_definition_reference_names consumes the selected normal definition/output arguments and canonical variable place owners, keeping quoted names, qualified heads, captured alias operands, literal dollar bytes and scalar-parenthesis values as selected values rather than reparsing reporting argv. Missing/foreign source, Registry/configuration, binding or required literal/output geometry declines definition-removal advice. drop_def_elims_resurrected_by_replacements excludes same atomic-group replacements from resurrection; a declined or resurrected deletion withdraws its complete atomic group. This guard protects final source removal proposals only; lexical source advice grants no Native rewrite, variable cell, executed frame, compiler instruction entry or end-to-end optimiser emission result.

## Scope

Two marked fixed source/API controls exercise quoted/qualified/captured operands, genuine unknown/rebound target refusal, unrelated replacement, and missing/foreign source/configuration/Registry/carrier refusal. Atomic group handling is part of the current owner contract. The manufactured proposals test this final safety gate independently of complete optimiser production. All seven providers are not tested; actual Rust receipts remain separate.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Consumer bindings

- [rust/tcl-compiler/src/optimiser/manager.rs](../../../../rust/tcl-compiler/src/optimiser/manager.rs), `elim_target_vars`: Select authentic original CFG source and retained binding under matching module/store/configuration before definition removal.
- [rust/tcl-compiler/src/optimiser/manager.rs](../../../../rust/tcl-compiler/src/optimiser/manager.rs), `normal_definition_reference_names`: Consume selected normal output/definition arguments and canonical place owners while preserving actual selected value units.
- [rust/tcl-compiler/src/optimiser/manager.rs](../../../../rust/tcl-compiler/src/optimiser/manager.rs), `drop_def_elims_resurrected_by_replacements`: Withdraw unsafe or unowned deletion proposals and their atomic groups without granting runtime/edit authority.
- [rust/tcl-compiler/src/optimiser/manager.rs](../../../../rust/tcl-compiler/src/optimiser/manager.rs), `optimiser::manager::tests::original_definition_guard_keeps_quoted_qualified_and_alias_names` (linked): Manufactured final proposals retain genuine quoted/qualified/captured names, literal-dollar and scalar-parenthesis values; unknown/rebound targets decline removal and unrelated replacements remain distinct.
- [rust/tcl-compiler/src/optimiser/manager.rs](../../../../rust/tcl-compiler/src/optimiser/manager.rs), `optimiser::manager::tests::original_definition_guard_declines_foreign_source_registry_and_missing_carrier` (linked): Actual selected definition names refuse foreign module image/store/configuration or missing retained binding. Final source safety advice grants no Native worker/frame or successful complete optimisation.

## Evidence

- [rust/tcl-compiler/src/optimiser/manager.rs](../../../../rust/tcl-compiler/src/optimiser/manager.rs)
