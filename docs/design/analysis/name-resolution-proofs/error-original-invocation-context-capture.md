# naming.error.original-invocation-context-capture

Kind: `implementation-contract`

## Problem statement

A diagnostic invocation-list view borrows original member headers. Keeping that view beyond a failed invocation cannot retain members after their owners release them; reached capture needs its own owning receipt and exact typed refusal path.

## Question

How does a reached original error-context capture retain original argv members beyond invocation release without assigning ownership to earlier diagnostic views or reviving retired inputs?

## Conclusion

Value::capture_native_error_context validates the actual live header and selected C error-stack protocol. A non-owning NativeListItems view becomes an owning List over the same genuine member headers only at reached capture; a context already owning its native List header retains that original header. Missing/foreign protocol and retired header/member access refuse. NativeErrorStack begin, restart and instruction adapters retain this typed failure and the first original host cause, without fabricating a guest message or reconstructing member strings. Four software controls distinguish before/after-capture owner counts, live original member identity, retired or missing protocol, and adapter settlement. The unchanged public dictionary345 observation and the separate cleanup398/399 counters remain different evidence purposes.

## Scope

These Rust header/reference-count and typed-error controls use authentic software fixture constructors and selected protocols only. All seven external providers are not tested for this implementation question. Original diagnostic argv views remain borrowed before capture; successful reached retention does not certify an external C private header, Normal result, active frame, compiler/instruction admission or Native execution. Retired inputs never regain ownership and later refusals cannot replace the first typed host cause. Serialized options and captured public text do not establish underlying object identity.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Source/API software contract only.

No external provider is executed for this software question. Rust fixture protocol/storage choices are not provider-behaviour observations.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Source/API software contract only.

No external provider is executed for this software question. Rust fixture protocol/storage choices are not provider-behaviour observations.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Source/API software contract only.

No external provider is executed for this software question. Rust fixture protocol/storage choices are not provider-behaviour observations.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Source/API software contract only.

No external provider is executed for this software question. Rust fixture protocol/storage choices are not provider-behaviour observations.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Source/API software contract only.

No external provider is executed for this software question. Rust fixture protocol/storage choices are not provider-behaviour observations.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Source/API software contract only.

No external provider is executed for this software question. Rust fixture protocol/storage choices are not provider-behaviour observations.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Source/API software contract only.

No external provider is executed for this software question. Rust fixture protocol/storage choices are not provider-behaviour observations.

## Exact evidence

- `naming.error.original-invocation-context-capture.definition.0` (definition): [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs). SHA-256 `3090f391be31fa5cf9433bfd1a0ec4e2a9334987f94788c793e489ab875270f1`. Current source definition of the named software owner/control; this digest is not an executed assertion or Native provider/build identity.
- `naming.error.original-invocation-context-capture.definition.1` (definition): [rust/tcl-vm/src/value.rs](../../../../rust/tcl-vm/src/value.rs). SHA-256 `07c92bd8dbce0e144a5bb7224b3ed1a3577debc826f978d0eb1d5da9455b9d6a`. Current source definition of the named software owner/control; this digest is not an executed assertion or Native provider/build identity.
- `naming.error.original-invocation-context-capture.definition.2` (definition): [rust/tcl-vm/src/native_list_backing.rs](../../../../rust/tcl-vm/src/native_list_backing.rs). SHA-256 `95c864b625e718e075a7c5caa4886b74ec9aaa82db8812397c3a20bff9153761`. Current source definition of the named software owner/control; this digest is not an executed assertion or Native provider/build identity.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/value.rs](../../../../rust/tcl-vm/src/value.rs), `Value::capture_native_error_context`: Validate live actual header and selected error-stack protocol; convert a non-owning original invocation member view into an owning List at reached capture, preserving original members and existing owned context headers.
- [rust/tcl-vm/src/native_list_backing.rs](../../../../rust/tcl-vm/src/native_list_backing.rs), `NativeListItems::owns_native_header`: Distinguish true native-header ownership from lifetime-only invocation views; a surviving diagnostic view alone does not acquire member references.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `NativeErrorStack::begin_inner`: Retain original context only after fallible reached capture; typed live-header/protocol/member refusal is preserved.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `NativeErrorStack::begin_instruction`: Use the same fallible reached ownership purpose for original instruction context, independently of instruction or frame admission.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `NativeErrorStack::restart_inner`: Retain the reached owning original context across restart without reviving retired inputs or bypassing typed refusal.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `interp::native_error_stack::tests::reached_error_capture_retains_original_argv_members_after_invocation_release` (linked): Check borrowed pre-capture reference counts, then reached owning capture retaining the same genuine software member headers after invocation release; this is an own-host ownership control.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `interp::native_error_stack::tests::reached_error_capture_declines_retired_argv_and_missing_protocol` (linked): Refuse retired original argv and missing/foreign native error-stack protocol without reviving a header or constructing a substitute source string.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `interp::native_error_stack::tests::reached_capture_adapters_preserve_typed_retired_foreign_and_first_causes` (linked): Keep exact typed retired/foreign capture refusals through actual adapters and preserve the first original host cause independently of later failures.
- [rust/tcl-vm/src/interp/native_error_stack.rs](../../../../rust/tcl-vm/src/interp/native_error_stack.rs), `interp::native_error_stack::tests::reached_instruction_capture_preserves_missing_owner_refusal` (linked): Keep reached instruction capture missing-owner/protocol refusal typed rather than manufacture a guest completion or instruction-entry grant.

A named test is a coverage binding, not a claim that it executed.

## Replay

Current source definitions and canonical named controls describe this software interface. No native process, Rust build or assertion is run by publication. Actual artifact/source/receipt coverage belongs to its separately retained validation record; selected fixture protocols do not execute the external provider or supply Native entry/Normal/frame/private header authority.
