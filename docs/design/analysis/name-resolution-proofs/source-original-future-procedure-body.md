# naming.source.original-future-procedure-body

Kind: `implementation-contract`

## Problem statement

A helper procedure can be declared before a Registry-created widget or source class and called after its factory appears. Scanning only at declaration time loses the later original factory. Rebuilding an analysis from the body discards its original parent, full dialect input and source-order replacement information. Reusing parent variable handles inside a procedure also mistakes globals for local variables.

## Question

Can a later actual source call retain its original procedure declaration and body, then join body source occurrences against the current conditional factory graph through the shared invocation walker without issuing an entered frame or publishing body effects to the caller?

## Conclusion

The conditional source graph retains the genuine selected procedure definer, original formal-list topology, purpose-specific declaration slot and borrowed original script body. Later source calls and aliases retain declaration and original move lineage. A separate finite inventory also retains uncalled original procedure bodies against the authentic final source graph with explicit FutureOriginalProcedureSourceApplicability; it does not invent a written call. Earlier actual source body scans are preserved. The private branch reuses the shared original invocation walker with the complete image/configuration/context and authentic body parent. Selected C declaration namespaces and Jim flat source naming remain separate. Known procedure or factory replacement/deletion, invalid formals, missing geometry and conflicting joins refuse. Procedure-local handle assistance starts independently of parent/global handles and branch effects never update the caller. Recursion and total work have finite limits. Body-parent applicability supplies no call, frame, value, Normal or executed body effect.

## Scope

Four deterministic source-contract controls cover helpers declared before or after a factory, authentic parent bodies and stale-image rejection; independently selected C Tcl 8.4–9.1 and Jim source grammar; known replacement/deletion and malformed formal refusal; and isolation of local handles from parent values and caller state. These do not execute native Tcl, prove body entry, materialise formals or variables, establish successful factories or physical receivers, donate Native admission, prove normal completion or permit edits. Dynamic procedure/factory names, unresolved global/upvar aliases, and incomplete original source geometry remain unavailable to this source projection.  Original formal topology is owned by [procedure-original-formal-count-shape.md](procedure-original-formal-count-shape.md). Its native observations do not turn this source-body contract into a new native execution proof. C Tcl 8.4–9.1, JimTcl and BIG-IP have no new process result for this contract.

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

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs), `OriginalSourceProcedureBody`: Retain original selected declaration/formal/body source ancestry independently of installed or entered state.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs), `AdviceInvocationContext::source_procedure_declaration`: Issue only genuine selected procedure/formal/body source shape with independent C namespace and Jim source-name recipes.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs), `AdviceInvocationContext::retain_original_procedure_call`: Walk the original future body in a private current source graph with local-handle isolation and finite depth/work; do not publish body effects to caller.
- [rust/tcl-compiler/src/command_binding/source_transition_advice.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice.rs), `OriginalSourceCommandTransitionAdvice::original_procedure_source_body`: Expose authentic source parent applicability without a frame or completion grant.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/registered_instance.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/registered_instance.rs), `OriginalSourceRegisteredInstanceWords::source_body`: Retain the genuine parent source body alongside the conditional instance receipt.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs), `AdviceInvocationContext::retain_future_original_procedure_bodies`: Inventory uncalled selected original procedure bodies against the authentic final source graph, preserving earlier actual body scans.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs), `AdviceInvocationContext::retain_original_procedure_body`: Reuse the same authentic source body walker and private branch for actual and future source purposes.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs), `command_binding::source_transition_advice::source_procedure::tests::original_future_procedure_joins_later_current_factory_with_authentic_parent_body` (linked): A helper declared before or after its Registry factory retains the authentic whole body parent and later current source factory receipt; a changed whole image refuses it.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs), `command_binding::source_transition_advice::source_procedure::tests::original_future_procedure_source_roles_retain_independently_selected_c_and_jim_grammar` (linked): C8.4–9.1 and Jim source controls retain their own selected grammar, full config/context and genuine original procedure parent for nested if roles.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs), `command_binding::source_transition_advice::source_procedure::tests::original_future_procedure_refuses_known_replacement_deletion_and_malformed_formals` (linked): Known replacement, deletion and malformed formal topology prevent the original future body from borrowing instance source advice.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs), `command_binding::source_transition_advice::source_procedure::tests::original_future_procedure_local_handle_does_not_borrow_parent_value_or_escape_branch` (linked): A procedure cannot borrow a parent/global variable handle; a locally produced handle can supply its own body source advice but cannot escape to the caller.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure.rs), `command_binding::source_transition_advice::source_procedure::tests::original_uncalled_procedure_uses_authentic_final_source_graph_without_call_or_frame` (linked): An uncalled helper before or after the class declaration retains its exact source body and FutureOriginalProcedureSourceApplicability; procedure/class deletion or redefinition blocks the join without fabricating a call or frame.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
