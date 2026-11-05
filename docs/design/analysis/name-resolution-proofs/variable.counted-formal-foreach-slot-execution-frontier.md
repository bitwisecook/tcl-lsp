# naming.variable.counted-formal-foreach-slot-execution-frontier

Kind: `native-observation`

## Problem statement

Two original formal objects k\x00a and k\x00b have equal lengths and CString prefixes. Their formal-list storage and invocation arity can prevent the foreach body from running at all; observing a failed call must not be used to infer which loop variable cell was selected.

## Question

When a procedure is defined with those two counted original formal objects, does invocation with ONE TWO reach the body, and if it does, what static/dynamic foreach and later write results are produced?

## Conclusion

All five C builds accept definition but reject the two-argument call with wrong # args for p k, so these controls establish no C foreach-body cell correspondence. The manifested Jim build reaches the body and returns {ONE CHANGED ONE CHANGED} ONE DYNAMIC ONE DYNAMIC. Original formal installation, body execution and source/static/runtime cell ownership remain independent.

## Scope

Two retained windows per six manifested native builds: original definition via object argv and one original invocation. Formal objects are counted k00a and k00b, exact body bytes in body.hex/probe.c. Jim revision/version is not recorded beyond build hashes. No C loop success, compiler LVT or observer/release capability is inferred.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 1 | "wrong # args: should be \"p k\"" |

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 0 | "{CHANGED CHANGED CHANGED TWO} CHANGED CHANGED CHANGED DYNAMIC" |

### jim

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "p" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 0 | "{ONE CHANGED ONE CHANGED} ONE DYNAMIC ONE DYNAMIC" |

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `compiled_foreach_matches_twelve_original_native_slot_records`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `interp::native_compiled_locals::tests::compiled_foreach_matches_twelve_original_native_slot_records` (linked): Compares all twelve original definition/invocation tuples, including every C arity failure; it does not claim twelve successful loop bodies.

## Evidence

- [rust/tcl-vm/tests/data/native_foreach_slots](../../../../rust/tcl-vm/tests/data/native_foreach_slots)
