# naming.variable.observed-original-substitution-receiver

Kind: `implementation-contract`

## Problem statement

An original variable substitution can bypass a positively selected observed naming purpose and use a default receiver, losing the complete counted name or actual entered event-frame owner.

## Question

How do original object-passthrough and interpolated-byte substitutions retain the complete observed name input and current event-frame receiver without authorising separate index or trace purposes?

## Conclusion

observed_substitution_receiver accepts only the complete original combined name under the positively selected observed purpose and delegates the existing measured receiver at the actual current frame level. Runtime fire_read_trace and read_var use that owner for original substitution. A separately supplied root/index and a retired event frame refuse; this query grants no observer registration or new naming policy. Object-passthrough and interpolated-byte paths retain their independent result forms.

## Scope

One authored Rust selector checks counted scalar cells against their prefix values, original object-passthrough and interpolated-byte substitution, direct complete-name access, split index refusal and retired-frame refusal. The real Rust event frame and selected observed policy are implementation premises, not new appliance execution or physical receiver measurements. Native C/Jim inputs, compiler admission, observer callbacks, successful trace/upvar purposes, pointer/header/cache layout and Native Normal remain independent. Existing captured BIG-IP grammar observations supply only their own exact purpose limits.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this implementation question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No observation for this implementation question.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/execution_name_policy.rs](../../../../runtime/rust/src/interp/execution_name_policy.rs), `Interp::observed_substitution_receiver`: Require complete original combined observed name input and actual current event-frame level; separately supplied index refuses.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::fire_read_trace`: Delegate the positively selected observed substitution to its original measured receiver without permitting original trace registration.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::read_var`: Read original complete observed substitution bytes through the same actual current-frame owner.
- [runtime/rust/src/interp/execution_name_policy.rs](../../../../runtime/rust/src/interp/execution_name_policy.rs), `interp::execution_name_policy::tests::original_observed_substitutions_keep_counted_cells_and_input_forms_separate` (linked): A genuine authored Rust event frame keeps counted S-NUL-T separate from S through object-passthrough and interpolated-byte source substitutions. Complete-name trace-frontier/read queries retain the existing receiver; split root/index and retired-frame queries refuse. No native C protocol or observer registration is selected.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named selector binds internal Rust substitution receiver/purpose coverage. Actual prior failed Runtime receipts remain independent; this contract supplies no native execution observation or successful assertion claim.
