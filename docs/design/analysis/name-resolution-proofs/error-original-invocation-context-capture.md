# naming.error.original-invocation-context-capture

Kind: `implementation-contract`

## Problem statement

A diagnostic invocation-list view borrows original member headers. Keeping that view beyond a failed invocation cannot retain members after their owners release them; reached capture needs its own owning receipt and exact typed refusal path.

## Question

How does a reached original error-context capture retain original argv members beyond invocation release without assigning ownership to earlier diagnostic views or reviving retired inputs?

## Conclusion

Value::capture_native_error_context validates the actual live header and selected C error-stack protocol. A non-owning NativeListItems view becomes an owning List over the same genuine member headers only at reached capture; a context already owning its native List header retains that original header. Missing/foreign protocol and retired header/member access refuse. NativeErrorStack begin, restart and instruction adapters retain this typed failure and the first original host cause, without fabricating a guest message or reconstructing member strings. Four software controls distinguish before/after-capture owner counts, live original member identity, retired or missing protocol, and adapter settlement. The unchanged public dictionary345 observation and the separate cleanup398/399 counters remain different evidence purposes. An options-presence query reads owned software shape without forcing child materialisation and preserves original child identity and owning references. Borrowed, retired, foreign or missing-protocol shapes remain refused. Procedure activation retains its actual original invocation references until exit; reached trace callbacks read those live activation words rather than a retired lifetime view.

## Scope

These Rust header/reference-count and typed-error controls use authentic software fixture constructors and selected protocols only. All seven external providers are not tested for this implementation question. Original diagnostic argv views remain borrowed before capture; successful reached retention does not certify an external C private header, Normal result, active frame, compiler/instruction admission or Native execution. Retired inputs never regain ownership and later refusals cannot replace the first typed host cause. Serialized options and captured public text do not establish underlying object identity. These four controls test software shape and activation ownership only. They confer no original Native header/refcount/cache/frame observation, public stack-byte equivalence, completed error semantics or assertion outcome. The independently recorded dictionary345 public comparator and its own pinned result are separate from activation-reference retention; no repair prerequisite is inferred from these source links.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No external provider is executed for this software question. Rust fixture protocol/storage choices are not provider-behaviour observations.

## Consumer bindings

- [rust/tcl-vm/src/value.rs](../../../../rust/tcl-vm/src/value.rs), `Value::capture_native_error_context`: Validate live actual header and selected error-stack protocol; convert a non-owning original invocation member view into an owning List at reached capture, preserving original members and existing owned context headers.
- [rust/tcl-vm/src/native_list_backing.rs](../../../../rust/tcl-vm/src/native_list_backing.rs), `NativeListItems::owns_native_header`: Distinguish true native-header ownership from lifetime-only invocation views; a surviving diagnostic view alone does not acquire member references.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `NativeErrorStack::begin_inner`: Retain original context only after fallible reached capture; typed live-header/protocol/member refusal is preserved.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `NativeErrorStack::begin_instruction`: Use the same fallible reached ownership purpose for original instruction context, independently of instruction or frame admission.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `NativeErrorStack::restart_inner`: Retain the reached owning original context across restart without reviving retired inputs or bypassing typed refusal.
- [rust/tcl-vm/src/value.rs](../../../../rust/tcl-vm/src/value.rs), `Value::native_return_options_nonempty`: Query the owned original options shape without forcing child getters; preserve missing/foreign/borrowed/retired protocol and ownership refusals.
- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `Vm::run_activation`: Keep the actual activation frame and its original procedure invocation references through execution and frame exit; error capture remains an independent reached-owner operation.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `interp::native_error_stack::tests::reached_error_capture_retains_original_argv_members_after_invocation_release` (linked): Check borrowed pre-capture reference counts, then reached owning capture retaining the same genuine software member headers after invocation release; this is an own-host ownership control.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `interp::native_error_stack::tests::reached_error_capture_declines_retired_argv_and_missing_protocol` (linked): Refuse retired original argv and missing/foreign native error-stack protocol without reviving a header or constructing a substitute source string.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `interp::native_error_stack::tests::reached_capture_adapters_preserve_typed_retired_foreign_and_first_causes` (linked): Keep exact typed retired/foreign capture refusals through actual adapters and preserve the first original host cause independently of later failures.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `interp::native_error_stack::tests::reached_instruction_capture_preserves_missing_owner_refusal` (linked): Keep reached instruction capture missing-owner/protocol refusal typed rather than manufacture a guest completion or instruction-entry grant.
- [rust/tcl-vm/src/value.rs](../../../../rust/tcl-vm/src/value.rs), `value::tests::return_options_presence_uses_owned_shape_without_materialising_children` (linked): An owned software options shape reports empty/nonempty without materialising child strings or changing original child identity/refcounts. Logical and selected sealed C86/C90/C91 list/dictionary shapes are separate from original Native cache observations.
- [rust/tcl-vm/src/value.rs](../../../../rust/tcl-vm/src/value.rs), `value::tests::return_options_presence_preserves_retired_foreign_and_borrowed_refusals` (linked): A shape query refuses a foreign protocol, borrowed nonowning argv view, missing original protocol, retired header and unsupported scalar shape; it cannot revive ownership merely to answer options presence.
- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `exec::tests::procedure_activation_retains_original_invocation_references_until_exit` (linked): The actual software procedure frame retains the same original invocation head/argument references throughout its lifetime, then releases them at frame exit; lifetime-lease views carry no owning reference.
- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `exec::tests::original_trace_procedure_heads_remain_live_during_their_actual_activation` (linked): Software C86/C90/C91 trace callbacks inspect the actual activation argv while the original procedure head remains live with positive owning references. This is a software lifetime control and has no original Native process comparison.

## Evidence

- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs)
- [rust/tcl-vm/src/value.rs](../../../../rust/tcl-vm/src/value.rs)
- [rust/tcl-vm/src/native_list_backing.rs](../../../../rust/tcl-vm/src/native_list_backing.rs)
