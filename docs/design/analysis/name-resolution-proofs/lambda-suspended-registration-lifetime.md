# naming.lambda.suspended-registration-lifetime

Kind: `implementation-contract`

## Problem statement

A suspended apply activation owns a temporary implementation registration. Public command enumeration cannot measure that lifetime because its interned implementation holder is not a declared guest namespace.

## Question

Does the actual registration retained by a suspended apply frame survive suspension and retire when the coroutine is deleted or the lambda returns or errors?

## Conclusion

The test observes the actual Frame.lambda_registration lifecycle handle through the parked coroutine roster. It requires one attached handle while suspended, no public command in a declared tcl::apply namespace, and detachment after deletion, ordinary return or error. The fixture accessors are test-only; production cleanup continues through the existing actual lifecycle-key owner.

## Scope

VM implementation transport and a genuine selected C8.6 fixture. This is not a native command/cache/header/refcount observation or a claim that the Rust selector executed. No source-name label, public command glob or completed guest result grants a frame, body, preparation, object-method or Normal capability.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `Vm::retire_parked_lambda_registrations`: Retire each actual retained registration from the suspended activation stack, innermost first.
- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `Vm::retire_original_lambda_registration`: Retire only the still-attached lifecycle key; later same-name allocations are separate.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `build_lambda_proc`: Create and retain the actual implementation registration separately from lambda body namespace and native header/cache claims.
- [rust/tcl-vm/src/command/native_apply_original_tests.rs](../../../../rust/tcl-vm/src/command/native_apply_original_tests.rs), `command::native_apply_original_tests::parked_lambda_registration_retires_after_coroutine_deletion` (linked): An actual suspended frame retains exactly one attached lambda-registration handle. The private implementation holder yields no guest command enumeration; deleting the coroutine, normal lambda return and lambda error all detach the original handle and remove it from the parked roster.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact linked Rust selector exercises the independently retained registration lifecycle. No execution receipt is attached. Native commandless lambda storage remains outside this implementation-registration contract.
