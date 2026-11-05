# naming.invocation.known-native-byte-values

Kind: `implementation-contract`

## Problem statement

An original operand can have exact native bytes that have no logical Unicode projection. Collapsing that value to a dynamic or opaque Registry word loses its ordinary argv position and prevents selected command or object transitions from retaining the real operand. Treating the same bytes as logical text would instead grant an unsupported encoding or naming interpretation.

## Question

How do shared Registry invocation and transition projections retain exact byte-only values and effective ordinals while separating them from logical literals, unresolved control operands and execution authority?

## Conclusion

KnownBytes retains one ordinary argv position and an exact native byte facet. LocatedNativeBytes retains that facet and the effective ordinal without a logical literal. Only a direct selected identity operation representing the same positioned subject discharges dynamic identity widening; unresolved operations and control operands remain conservative. Legacy logical pack-hook caches cannot reuse a byte-only value as a literal. Pack alias transition facts retain complete known bytes and original effective ordinals for level/target/local operands. A selected C/Jim name policy may project the local extent; missing dialect retains an unknown local projection while preserving the counted target.

## Scope

Fixed Rust metadata and transport contracts, including opaque 0xff, surrogate native units, counted NUL, exact ordinal rebasing and independently selected C/Jim alias-name purposes. No native execution, normal completion, installed command identity, physical object, source word, edit or CPP/cache admission follows from these value facets. These transition facts grant no installed alias, current variable value, reached caller selection, physical link or entered Native frame. A byte carrier and equal rendered names are insufficient execution or identity premises.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs), `InvocationWord::native_bytes`: Separate byte-only native values and argv shape from logical literals.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `TransitionSubject::from_argument`: Retain exact native payload and effective operand ordinal as independent metadata.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `StateTransition::represents_native_identity_subject`: Discharge only represented identity obligations without validating control operands or execution.
- [rust/tcl-registry/src/pack_hooks.rs](../../../../rust/tcl-registry/src/pack_hooks.rs), `shape_key`: Prevent byte-only values from colliding with logical values in the legacy hook shape cache.
- [rust/tcl-compiler/src/registry_invocation.rs](../../../../rust/tcl-compiler/src/registry_invocation.rs), `EffectiveInvocationWord::as_registry_word`: Preserve byte-only evaluated values at the shared Compiler/Registry seam.
- [rust/tcl-registry/src/pack_hooks.rs](../../../../rust/tcl-registry/src/pack_hooks.rs), `pack_transitions`: Keep counted transition subjects, original effective ordinals and selected name-purpose projection without issuing a physical alias or caller-frame receipt.
- [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs), `invocation_words::tests::known_native_bytes_keep_argv_shape_without_a_logical_value` (linked): Exact native bytes preserve two ordinary argv positions; logical values remain unavailable and an earlier expansion still makes later argv positions indeterminate.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `state_transition::tests::native_byte_subjects_preserve_values_and_project_each_ordinal` (linked): Equal payloads at different effective ordinals remain distinct; rebasing preserves bytes and underflow withdraws the whole projection.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `state_transition::tests::native_identity_values_do_not_close_unknown_control_operands` (linked): Only the same direct identity subject closes dynamic widening; unresolved command operands, recursion limits, package versions and different ordinals do not.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `state_transition::tests::native_byte_alias_names_use_the_selected_name_purpose` (linked): C and Jim select their existing independent alias-name recipes, with no-local and missing-policy cases preserved.
- [rust/tcl-registry/src/commands/tcl/oo_class.rs](../../../../rust/tcl-registry/src/commands/tcl/oo_class.rs), `commands::tcl::oo_class::tests::class_creation_keeps_opaque_native_target_and_conditional_commit` (linked): Selected OO creation retains native target bytes and ordinal under OnOkOnly; empty CString names and unknown operands do not emit a named Define.
- [rust/tcl-registry/src/pack_hooks.rs](../../../../rust/tcl-registry/src/pack_hooks.rs), `pack_hooks::tests::native_byte_hook_kind_cannot_reuse_a_logical_literal_cache_entry` (linked): The DSL shape is known-bytes, logical fold admission stays unavailable and the legacy two-bit cache declines byte-only entries.
- [rust/tcl-compiler/src/registry_invocation.rs](../../../../rust/tcl-compiler/src/registry_invocation.rs), `registry_invocation::tests::decoded_native_byte_words_preserve_cardinality_and_checked_text` (linked): Compiler ByteLiteral transports exact native bytes to Registry KnownBytes without inventing a logical string.
- [rust/tcl-registry/src/pack_hooks.rs](../../../../rust/tcl-registry/src/pack_hooks.rs), `pack_hooks::tests::pack_aliases_preserve_original_known_bytes_and_selected_name_policy` (linked): Pack alias transition facts retain counted known bytes and selected local-name extent independently; a missing dialect retains the target bytes but refuses to select a C/Jim local-name projection. This is the current software/API definition; no assertion outcome or new original-provider observation is attached to this binding.
- [rust/tcl-registry/src/pack_hooks.rs](../../../../rust/tcl-registry/src/pack_hooks.rs), `pack_hooks::tests::pack_caller_alias_keeps_counted_operands_and_effective_ordinals` (linked): Caller alias source facts preserve complete counted level, target and local operands plus each effective ordinal without selecting or entering the caller frame. This is the current software/API definition; no assertion outcome or new original-provider observation is attached to this binding.

## Evidence

- [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs)
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs)
- [rust/tcl-registry/src/pack_hooks.rs](../../../../rust/tcl-registry/src/pack_hooks.rs)
- [rust/tcl-compiler/src/registry_invocation.rs](../../../../rust/tcl-compiler/src/registry_invocation.rs)
