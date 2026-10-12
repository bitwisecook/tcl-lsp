# First public OO refusal

Read-only scope: `cmd_oo/native_context/definition_exit/tests.rs`, `exec.rs`, `cmd_oo/original_name_operands.rs` only. No source edit, extra execution or Cargo.

The first six [exact controls](controls-results.json) panic at `exec.rs:9170`. [invoke_command](../../../../rust/tcl-vm/src/exec.rs#L9166) calls the genuine object-vector worker, then asserts that no neutral host refusal survived. Its expression-provider wording is not evidence that the retained cause is an expression refusal: the predicate covers every `execution_refusal` variant.

The [factory](../../../../rust/tcl-vm/src/cmd_oo/native_context/definition_exit/tests.rs#L26) already uses `native_fixture::interpreter` for C9.1, whose previously inspected implementation constructs `Vm::with_native_core` before attaching the matching compiler. The [class helper](../../../../rust/tcl-vm/src/cmd_oo/native_context/definition_exit/tests.rs#L29) invokes the actual `oo::class create` command on that VM. Replacing the factory or adding a manually labelled entry is unsupported by this evidence.

[try_invoke_command](../../../../rust/tcl-vm/src/exec.rs#L9184) uses exactly the same `invoke_host_object_vector` path and returns its full `NativeExecutionError` through `finish_host_execution`. The [host worker](../../../../rust/tcl-vm/src/exec.rs#L9197) constructs the actual native command-head bytes and forwards the original argument objects; it does not compile a fake unit or infer authority from a display name.

Small next repair/diagnostic: replace only the class helper's `invoke_command` with the existing `try_invoke_command(...).expect("actual C9.1 class construction")`. Keep its existing `Code::Ok`, actual object-resolution and every later callback/currency assertion. This exposes the complete real constructor refusal instead of the wrapper's generic assertion; it is not a proposed semantic fix and cannot justify a pass before the original assertions execute.

Independent [active controls 14–16](results.md) already expose `ValueAccessRefusal(CommandProtocolUnavailable("original OO Name target/facet"))` during actual setup. The [Name owner](../../../../rust/tcl-vm/src/cmd_oo/original_name_operands.rs#L87) emits that same purpose when either original target capture is unavailable or a retained target is no longer current ([check_target](../../../../rust/tcl-vm/src/cmd_oo/original_name_operands.rs#L154)). Currentness checks actual object allocation/cache identity, creation, namespace ownership, destroyed state and class-facet birth.

The logs do not distinguish failed capture from one failed currentness predicate, and the first six generic wrapper panics do not yet prove this is their cause. No safe production-forwarding change follows from the current receipt. Next production diagnosis must locate the first failed original target/facet predicate at the existing owner; preserve its complete first cause and actual selected object. Do not remove a check, refresh a target after selection or accept a logical substitute.

A `try_eval_source` migration is appropriate only when the intended input is the original Script/Document channel. This helper deliberately exercises actual object-vector class creation; retaining that entry is the smallest current API repair.
