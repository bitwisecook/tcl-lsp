# naming.interprocedural.original-transfer-summary-source-context

Kind: `implementation-contract`

## Problem statement

An interprocedural transfer summary can borrow a reported procedure name or detached equal child/body text, losing the actual declaration, selected effective argv, source lookup point, complete lexer configuration or formal binder. Conditional source summaries must retain those owners independently of runtime completion and physical effects.

## Question

How do transfer summaries and their lattice consumers retain the same actual Logical Module, original procedure declaration, selected effective argv and genuine child/body source points while keeping Native completion, frame and effect obligations independent?

## Conclusion

SourceSummaryContext joins complete current retained Logical Module input/configuration with the selected original source point. A procedure call shares the authentic original declaration/header issuer and effective captured/written argv. OriginalSummaryScript retains exact parent word ancestry and child tokens for substitutions and selected literal bodies; detached equal text supplies no such receipt. Procedure aliases, source operations, destruction and outer arguments use the same retained owner. Actual selected Tcl/Jim formal binding remains independent, and unused-formal source uses come from the existing metadata/configuration-aware SSA inventory. Missing/foreign/changed source or header, known target barriers, command observers and Native-only input retain refusal. Lattice call/operation/substitution/body consumers borrow the same source owner rather than rebuilding nominal context.

## Scope

Six marked source/API controls comprise four original call/child/body/owner-barrier controls and two independently scoped formal/unused-use controls. They are authored conditional Logical summary assertions with no Rust execution result or external provider process attached. Original scalar-binder, definition-reach, trace/mutation, purity/observer and edit gates remain required separately. Source names, relative source positions and possible transfer facts grant no Native entered frame, actual argument value, physical cell/read/effect, Handler/Normal completion, compiler instruction, safe deletion or inlining equivalence. The record links current source interfaces without refreshing held mutable implementation pins.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No original external provider process answers this conditional Logical source-summary contract. Selected Tcl/Jim grammar controls retain authored source ownership and binder premises only; Native frame/completion/effects remain independent.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No original external provider process answers this conditional Logical source-summary contract. Selected Tcl/Jim grammar controls retain authored source ownership and binder premises only; Native frame/completion/effects remain independent.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No original external provider process answers this conditional Logical source-summary contract. Selected Tcl/Jim grammar controls retain authored source ownership and binder premises only; Native frame/completion/effects remain independent.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No original external provider process answers this conditional Logical source-summary contract. Selected Tcl/Jim grammar controls retain authored source ownership and binder premises only; Native frame/completion/effects remain independent.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No original external provider process answers this conditional Logical source-summary contract. Selected Tcl/Jim grammar controls retain authored source ownership and binder premises only; Native frame/completion/effects remain independent.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No original external provider process answers this conditional Logical source-summary contract. Selected Tcl/Jim grammar controls retain authored source ownership and binder premises only; Native frame/completion/effects remain independent.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No original external provider process answers this conditional Logical source-summary contract. Selected Tcl/Jim grammar controls retain authored source ownership and binder premises only; Native frame/completion/effects remain independent.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `SourceSummaryContext::for_module`: Require complete actual current retained Logical Module source/input/configuration and authentic registry correspondence.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `SourceSummaryContext::procedure_call`: Join original declaration/header allocation, genuine point metadata and selected effective captured/written argv.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `SourceSummaryContext::substitution`: Project only an authentic original substitution child under its genuine parent word and selected grammar.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `SourceSummaryContext::body`: Retain original selected literal-body ancestry and each child source point; detached equal text refuses.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `OriginalSummaryScript::command_at`: Resolve actual child tokens from the sealed original script/source inventory.
- [rust/tcl-compiler/src/interprocedural/transfer.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer.rs), `ModuleProcedures::original_call_arguments`: Share original selected call arguments with lattice consumers without rebuilding a name-based nominal issuer.
- [rust/tcl-compiler/src/interprocedural/transfer.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer.rs), `ModuleProcedures::with_original_operation`: Borrow the same actual source point/metadata for conditional operation effects and transfer facts.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `interprocedural::transfer::source_context::tests::original_transfer_call_keeps_selected_procedure_and_captured_operands_together` (linked): Shares complete retained Logical Module/source-point/header/child issuers for transfer call graphs, aliases, destruction, outer arguments and effects. Selected effective argv including captured prefix values and Tcl/Jim formal binding remain one original owner. The lattice direct procedure, original operation, substitution and selected literal-body consumers carry that same owner; detached text has no original receipt. Conditional summary facts establish no Native Normal completion, entered frame, observer, physical effect or edit permission. Independent original scalar-binder and trace/mutation guards remain. Formal unused-use inventory comes from existing actual metadata/config SSA, not text matching.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `interprocedural::transfer::source_context::tests::original_transfer_child_and_body_require_the_genuine_parent_word` (linked): Shares complete retained Logical Module/source-point/header/child issuers for transfer call graphs, aliases, destruction, outer arguments and effects. Selected effective argv including captured prefix values and Tcl/Jim formal binding remain one original owner. The lattice direct procedure, original operation, substitution and selected literal-body consumers carry that same owner; detached text has no original receipt. Conditional summary facts establish no Native Normal completion, entered frame, observer, physical effect or edit permission. Independent original scalar-binder and trace/mutation guards remain. Formal unused-use inventory comes from existing actual metadata/config SSA, not text matching.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `interprocedural::transfer::source_context::tests::original_transfer_source_refuses_missing_foreign_config_and_changed_headers` (linked): Shares complete retained Logical Module/source-point/header/child issuers for transfer call graphs, aliases, destruction, outer arguments and effects. Selected effective argv including captured prefix values and Tcl/Jim formal binding remain one original owner. The lattice direct procedure, original operation, substitution and selected literal-body consumers carry that same owner; detached text has no original receipt. Conditional summary facts establish no Native Normal completion, entered frame, observer, physical effect or edit permission. Independent original scalar-binder and trace/mutation guards remain. Formal unused-use inventory comes from existing actual metadata/config SSA, not text matching.
- [rust/tcl-compiler/src/interprocedural/transfer/source_context.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer/source_context.rs), `interprocedural::transfer::source_context::tests::original_transfer_shadow_and_command_observers_do_not_borrow_catalogue_closure` (linked): Shares complete retained Logical Module/source-point/header/child issuers for transfer call graphs, aliases, destruction, outer arguments and effects. Selected effective argv including captured prefix values and Tcl/Jim formal binding remain one original owner. The lattice direct procedure, original operation, substitution and selected literal-body consumers carry that same owner; detached text has no original receipt. Conditional summary facts establish no Native Normal completion, entered frame, observer, physical effect or edit permission. Independent original scalar-binder and trace/mutation guards remain. Formal unused-use inventory comes from existing actual metadata/config SSA, not text matching.
- [rust/tcl-compiler/src/interprocedural/transfer.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer.rs), `interprocedural::transfer::tests::transfer_formals_keep_selected_tcl_and_jim_binders_separate` (linked): Shares complete retained Logical Module/source-point/header/child issuers for transfer call graphs, aliases, destruction, outer arguments and effects. Selected effective argv including captured prefix values and Tcl/Jim formal binding remain one original owner. The lattice direct procedure, original operation, substitution and selected literal-body consumers carry that same owner; detached text has no original receipt. Conditional summary facts establish no Native Normal completion, entered frame, observer, physical effect or edit permission. Independent original scalar-binder and trace/mutation guards remain. Formal unused-use inventory comes from existing actual metadata/config SSA, not text matching.
- [rust/tcl-compiler/src/interprocedural/transfer.rs](../../../../rust/tcl-compiler/src/interprocedural/transfer.rs), `interprocedural::transfer::tests::transfer_unused_formals_use_actual_variable_roots_and_metadata` (linked): Shares complete retained Logical Module/source-point/header/child issuers for transfer call graphs, aliases, destruction, outer arguments and effects. Selected effective argv including captured prefix values and Tcl/Jim formal binding remain one original owner. The lattice direct procedure, original operation, substitution and selected literal-body consumers carry that same owner; detached text has no original receipt. Conditional summary facts establish no Native Normal completion, entered frame, observer, physical effect or edit permission. Independent original scalar-binder and trace/mutation guards remain. Formal unused-use inventory comes from existing actual metadata/config SSA, not text matching.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact marked source definitions and current owner interfaces are linked. They must be compiled, listed and executed under their own independent software receipt before any assertion result is claimed. No Native process, original private frame/object/header, external provider semantics or erasure permission is supplied by these source bindings; mutable implementation pins remain held.
