# naming.invocation.known-native-byte-values

Kind: `implementation-contract`

## Problem statement

An original operand can have exact native bytes that have no logical Unicode projection. Collapsing that value to a dynamic or opaque Registry word loses its ordinary argv position and prevents selected command or object transitions from retaining the real operand. Treating the same bytes as logical text would instead grant an unsupported encoding or naming interpretation.

## Question

How do shared Registry invocation and transition projections retain exact byte-only values and effective ordinals while separating them from logical literals, unresolved control operands and execution authority?

## Conclusion

KnownBytes retains one ordinary argv position and an exact native byte facet. LocatedNativeBytes retains that facet and the effective ordinal without a logical literal. Only a direct selected identity operation representing the same positioned subject discharges dynamic identity widening; unresolved operations and control operands remain conservative. Legacy logical pack-hook caches cannot reuse a byte-only value as a literal.

## Scope

Fixed Rust metadata and transport contracts, including opaque 0xff, surrogate native units, counted NUL, exact ordinal rebasing and independently selected C/Jim alias-name purposes. No native execution, normal completion, installed command identity, physical object, source word, edit or CPP/cache admission follows from these value facets.

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

- `implementation-0` (implementation): [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs). SHA-256 `befa88ea71c58d74cdb9df91ac3c64980a371d6622a970cdd00d7126c3ba5297`. Separate byte-only native values and argv shape from logical literals.
- `implementation-1` (implementation): [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs). SHA-256 `91b58b75d1d92dd7d8b34a04de0ee708034df527e5e156b0e04f20ccbe4a85d2`. Retain exact native payload and effective operand ordinal as independent metadata.
- `implementation-2` (implementation): [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs). SHA-256 `91b58b75d1d92dd7d8b34a04de0ee708034df527e5e156b0e04f20ccbe4a85d2`. Discharge only represented identity obligations without validating control operands or execution.
- `implementation-3` (implementation): [rust/tcl-registry/src/pack_hooks.rs](../../../../rust/tcl-registry/src/pack_hooks.rs). SHA-256 `61439494e3681407f7d7f6f35a1d0ae7a4e0843bca16332b0be13d23ec8a2029`. Prevent byte-only values from colliding with logical values in the legacy hook shape cache.
- `implementation-4` (implementation): [rust/tcl-compiler/src/registry_invocation.rs](../../../../rust/tcl-compiler/src/registry_invocation.rs). SHA-256 `3b07b5a68afee36b6854c1903362aed45f0da1b2ba46db527a8ebcc8de2991dd`. Preserve byte-only evaluated values at the shared Compiler/Registry seam.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs), `InvocationWord::native_bytes`: Separate byte-only native values and argv shape from logical literals.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `TransitionSubject::from_argument`: Retain exact native payload and effective operand ordinal as independent metadata.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `StateTransition::represents_native_identity_subject`: Discharge only represented identity obligations without validating control operands or execution.
- [rust/tcl-registry/src/pack_hooks.rs](../../../../rust/tcl-registry/src/pack_hooks.rs), `shape_key`: Prevent byte-only values from colliding with logical values in the legacy hook shape cache.
- [rust/tcl-compiler/src/registry_invocation.rs](../../../../rust/tcl-compiler/src/registry_invocation.rs), `EffectiveInvocationWord::as_registry_word`: Preserve byte-only evaluated values at the shared Compiler/Registry seam.
- [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs), `invocation_words::tests::known_native_bytes_keep_argv_shape_without_a_logical_value` (linked): Exact native bytes preserve two ordinary argv positions; logical values remain unavailable and an earlier expansion still makes later argv positions indeterminate.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `state_transition::tests::native_byte_subjects_preserve_values_and_project_each_ordinal` (linked): Equal payloads at different effective ordinals remain distinct; rebasing preserves bytes and underflow withdraws the whole projection.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `state_transition::tests::native_identity_values_do_not_close_unknown_control_operands` (linked): Only the same direct identity subject closes dynamic widening; unresolved command operands, recursion limits, package versions and different ordinals do not.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `state_transition::tests::native_byte_alias_names_use_the_selected_name_purpose` (linked): C and Jim select their existing independent alias-name recipes, with no-local and missing-policy cases preserved.
- [rust/tcl-registry/src/commands/tcl/oo_class.rs](../../../../rust/tcl-registry/src/commands/tcl/oo_class.rs), `commands::tcl::oo_class::tests::class_creation_keeps_opaque_native_target_and_conditional_commit` (linked): Selected OO creation retains native target bytes and ordinal under OnOkOnly; empty CString names and unknown operands do not emit a named Define.
- [rust/tcl-registry/src/pack_hooks.rs](../../../../rust/tcl-registry/src/pack_hooks.rs), `pack_hooks::tests::native_byte_hook_kind_cannot_reuse_a_logical_literal_cache_entry` (linked): The DSL shape is known-bytes, logical fold admission stays unavailable and the legacy two-bit cache declines byte-only entries.
- [rust/tcl-compiler/src/registry_invocation.rs](../../../../rust/tcl-compiler/src/registry_invocation.rs), `registry_invocation::tests::decoded_native_byte_words_preserve_cardinality_and_checked_text` (linked): Compiler ByteLiteral transports exact native bytes to Registry KnownBytes without inventing a logical string.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-registry",
  "native_byte"
]
```

The listed Registry, Compiler and pack-hook selectors are Rust coverage obligations for this contract. No native provider replay or Rust execution receipt is attached.
