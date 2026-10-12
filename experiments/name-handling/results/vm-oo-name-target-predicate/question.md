# First actual OO target/facet predicate

Question: which original target capture/currentness predicate first fails in the retained C9.1 class-construction control, before its callback assertions?

[Typed receipt](../vm-oo-class-construction-diagnostic/control-01.log): `ValueAccessRefusal(CommandProtocolUnavailable("original OO Name target/facet"))`; one exact test failed at the unchanged-input class helper. This is not an expression-refusal diagnosis or semantic pass.

Read-only owner scope is exactly `cmd_oo.rs`, `cmd_oo/original_name_operands.rs`, `interp/native_namespace_names.rs`; source hashes accompany this question. No build or source edit was made. Both caches are leased to other work.

Earliest candidate in actual dispatch ([method-key selection](../../../../rust/tcl-vm/src/cmd_oo.rs#L1240), [factory](../../../../rust/tcl-vm/src/cmd_oo.rs#L1267)): `oo_dispatch_entered` selects the original `create` method through `native_method_key_for_target(..., Some((false, receiver)))` before the factory runs. That producer enters the Name scope, captures the existing receiver object and checks its currentness. The later factory separately selects `Some((true, class_key))`, adding the class-facet birth check. No written method name supplies either target's identity.

The exact purpose is emitted at [select_target](../../../../rust/tcl-vm/src/cmd_oo/original_name_operands.rs#L78) when capture is None, or [check_target](../../../../rust/tcl-vm/src/cmd_oo/original_name_operands.rs#L154) when its retained target is not current. The receipt alone does not distinguish these gates, an already retained bootstrap cause, or a later check. Currentness short-circuits object allocation life/presence/identity, creation, namespace identity, destroyed state, actual namespace ownership, and optional class-facet birth/presence/identity.

The [C namespace-owner term](../../../../rust/tcl-vm/src/interp/native_namespace_names.rs#L98) requires an arena path and a live name world, then actual declaration/intern identity or a dying/deferred/retained owner. Bootstrap roots call `install_object_helpers`; named class construction additionally calls `activate_namespace_written` before that helper. The helper implementation was outside the three-owner inspection, so this source difference is a candidate to measure, not proof that namespace ownership fails or justification to publish a namespace.

[instrumentation-plan.md](instrumentation-plan.md) and the mechanically prepared [instrumentation.patch](instrumentation.patch) preserve the actual original fixture/entry/arguments and every assertion. The first failing protocol predicate remains unmeasured until that separately coordinated diagnostic executes.
