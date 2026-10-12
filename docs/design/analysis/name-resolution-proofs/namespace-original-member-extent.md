# naming.namespace.original-member-extent

Kind: `implementation-contract`

## Problem statement

Jim namespace canonicalisation preserves a trailing separator, while C namespace parsing consumes colon separator runs. Using the selected parent prefix itself as a member range includes Jim's :: pair and removes it during rename. A separate fixture can also incorrectly invoke the C-only command slot projector for Jim's flat command table, confusing a purpose refusal with unsupported namespace membership.

## Question

How do original namespace component ranges preserve separators and opaque parent geometry, and which independently selected command naming purpose supplies Jim namespace membership?

## Conclusion

The shared original member-range owner excludes Jim's exact separating :: pair after selecting the retained parent object; C colon-run projection remains independently selected. Operand replacement then reselects the complete expected namespace before returning bytes, preserving root markers and literal colons in the Jim parent. Jim membership and retained lookup-home selection consume the independently selected root-flat current/global command candidates; the C slot projector still refuses Jim. These are geometry/rewrite contracts, not namespace existence, binding, source edit or Native authority.

## Scope

Original immutable byte operands and independently retained C namespace components or Jim namespace objects. Fixed ranges, omitted components, opaque distinct inputs, literal parent colons, full result roundtrip and multi-component/raw-zero replacement negatives are covered. No native interpreter, object header, callback, entered frame or BIG-IP engine is executed for this implementation contract.

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

- [rust/tcl-syntax/src/naming/namespace_spans.rs](../../../../rust/tcl-syntax/src/naming/namespace_spans.rs), `native_written_namespace_member_extent`: Select one original component range with the exact independently selected retained parent and engine separator geometry.
- [rust/tcl-syntax/src/naming/namespace_spans.rs](../../../../rust/tcl-syntax/src/naming/namespace_spans.rs), `native_namespace_operand_rename`: Replace only that original member and verify the expected complete namespace projection before returning bytes.
- [rust/tcl-syntax/src/naming/namespace_spans.rs](../../../../rust/tcl-syntax/src/naming/namespace_spans.rs), `native_namespace_rename_target`: Construct a collision proposal through the same replacement/roundtrip owner without namespace existence or edit permission.
- [rust/tcl-syntax/src/naming/namespace_spans.rs](../../../../rust/tcl-syntax/src/naming/namespace_spans.rs), `native_command_slot_is_under_namespace`: Project namespace membership from the selected C holder or Jim root-flat command key without including the simple command tail.
- [rust/tcl-syntax/src/naming/namespace_spans.rs](../../../../rust/tcl-syntax/src/naming/namespace_spans.rs), `native_command_lookup_context_for_slot`: Join an independently selected called slot with the original operand's actual C or ordered Jim current/global candidate geometry, supplying no existence or binding proof.
- [rust/tcl-syntax/src/naming/namespace_spans.rs](../../../../rust/tcl-syntax/src/naming/namespace_spans.rs), `naming::namespace_spans::tests::namespace_spans_do_not_recover_display_colliding_scope_components` (linked): Reject display-colliding C retained parents; exclude Jim pair separator and retain a literal colon belonging to its selected parent object.
- [rust/tcl-syntax/src/naming/namespace_spans.rs](../../../../rust/tcl-syntax/src/naming/namespace_spans.rs), `naming::namespace_spans::original_namespace_rename_tests::original_namespace_rename_preserves_colon_runs_and_omitted_components` (linked): Preserve written root markers/separator runs, including Jim literal parent colon; leave omitted inherited components unchanged and refuse multi-component/raw-zero replacement.
- [rust/tcl-syntax/src/naming/namespace_spans.rs](../../../../rust/tcl-syntax/src/naming/namespace_spans.rs), `naming::namespace_spans::operand_namespace_purpose_tests::namespace_membership_excludes_command_and_variable_simple_tails` (linked): Use C namespace slots and Jim flat command lookup candidates independently; retain C-purpose refusal on Jim and refuse donating command namespace partitions to Jim variable roots. Retain the current Jim object for its selected candidate, the root object for genuine global fallback, and reject a mismatching flat slot.
- [rust/tcl-syntax/src/naming/namespace_spans.rs](../../../../rust/tcl-syntax/src/naming/namespace_spans.rs), `naming::namespace_spans::namespace_rename_target_tests::namespace_rename_targets_replace_one_opaque_component_and_reject_multiple_components` (linked): Roundtrip replacement of exact opaque C components or retained Jim namespace objects and reject empty/multiple/raw-zero tails.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-syntax",
  "--lib",
  "namespace_spans"
]
```

Run from rust/. These meaningful fixed-input geometry tests are Rust implementation contracts. Test bindings do not claim execution here and supply no native engine outcome or edit authority.
