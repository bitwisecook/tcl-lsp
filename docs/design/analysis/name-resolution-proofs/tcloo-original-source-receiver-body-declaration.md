# naming.tcloo.original-source-receiver-body-declaration

Kind: `implementation-contract`

## Problem statement

A genuine original method body can be available for source analysis even when no native factory allocation or entered receiver is proved. Requiring those physical owners loses readonly source applicability; accepting computed/recooked bodies or mere containment would instead fabricate original invocation geometry.

## Question

Can a genuinely selected original method declaration retain a borrowed source body independently of native factory allocation or receiver entry?

## Conclusion

OriginalSourceReceiverBodyDeclaration selects one genuine original TclOO method declaration with complete parameter/body words under the full source image, configuration and input. Its borrowed ASCII body must be unchanged at the original content extent. Scope retains this source declaration independently of native deferred-body entry. owns_invocation revalidates current source/input, the exact body extent and the complete original vector against the shared original script parser; a contained partial vector does not qualify. Computed, recooked, conflicting and foreign body owners decline. This supplies direct original invocation/body applicability only, without method allocation, executing object namespace, entered receiver/frame, Native handler or Normal completion.  The visibility consumer uses the distinct selected original method-body declaration through its owns_invocation/full-input guard. This does not convert it into a Native preview-frame entry or class publication. Possible global-source fallback retains unknown executing receiver namespace and earlier local-binding obligations, and both diagnostic/source-edge subjects retain the same source receipt.

## Scope

Two marked C8.6 source controls retain the original method m and exact source invocation inside a safely created child-source class body, with no native entry required. A partial vector, root command outside that body and changed SourceImage refuse. Computed $script and quoted recooked tab bodies supply no borrowed source mapping. Complete original parameter/body words and one unchanged ASCII body are required; non-ASCII/cooked interior reconstruction and physical deferred execution remain independent.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original source method/body declaration and whole-vector applicability contract supplies no native deferred-body entry, method/object allocation, namespace, frame or completion observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original source method/body declaration and whole-vector applicability contract supplies no native deferred-body entry, method/object allocation, namespace, frame or completion observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original source method/body declaration and whole-vector applicability contract supplies no native deferred-body entry, method/object allocation, namespace, frame or completion observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original source method/body declaration and whole-vector applicability contract supplies no native deferred-body entry, method/object allocation, namespace, frame or completion observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original source method/body declaration and whole-vector applicability contract supplies no native deferred-body entry, method/object allocation, namespace, frame or completion observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original source method/body declaration and whole-vector applicability contract supplies no native deferred-body entry, method/object allocation, namespace, frame or completion observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original source method/body declaration and whole-vector applicability contract supplies no native deferred-body entry, method/object allocation, namespace, frame or completion observation.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/analyser/types/original_receiver_body.rs](../../../../rust/tcl-compiler/src/analyser/types/original_receiver_body.rs), `OriginalSourceReceiverBodyDeclaration::from_class_body`: Retain the dedicated immutable original source method/body declaration and current whole-vector applicability independently of Native deferred entry and executing receiver namespace.
- [rust/tcl-compiler/src/analyser/types/original_receiver_body.rs](../../../../rust/tcl-compiler/src/analyser/types/original_receiver_body.rs), `OriginalSourceReceiverBodyDeclaration::owns_invocation`: Retain the dedicated immutable original source method/body declaration and current whole-vector applicability independently of Native deferred entry and executing receiver namespace.
- [rust/tcl-compiler/src/analyser/oo.rs](../../../../rust/tcl-compiler/src/analyser/oo.rs), `Analyser::walk_method_body`: Retain the dedicated immutable original source method/body declaration and current whole-vector applicability independently of Native deferred entry and executing receiver namespace.
- [rust/tcl-compiler/src/analyser/types.rs](../../../../rust/tcl-compiler/src/analyser/types.rs), `Scope`: Retain the dedicated immutable original source method/body declaration and current whole-vector applicability independently of Native deferred entry and executing receiver namespace.
- [rust/tcl-compiler/src/analyser/types/original_receiver_body.rs](../../../../rust/tcl-compiler/src/analyser/types/original_receiver_body.rs), `OriginalSourceReceiverBodyDeclaration::matches_source`: Require current original body image, full configuration and complete retained analysis input before source applicability.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `Analyser::original_interp_source_receiver_declaration`: The visibility consumer uses the distinct selected original method-body declaration through its owns_invocation/full-input guard. This does not convert it into a Native preview-frame entry or class publication. Possible global-source fallback retains unknown executing receiver namespace and earlier local-binding obligations, and both diagnostic/source-edge subjects retain the same source receipt.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `ConditionalInterpreterVisibilitySubject::source_receiver_body_declaration`: The visibility consumer uses the distinct selected original method-body declaration through its owns_invocation/full-input guard. This does not convert it into a Native preview-frame entry or class publication. Possible global-source fallback retains unknown executing receiver namespace and earlier local-binding obligations, and both diagnostic/source-edge subjects retain the same source receipt.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `OriginalInterpreterSourceLoad::source_receiver_body_declaration`: The visibility consumer uses the distinct selected original method-body declaration through its owns_invocation/full-input guard. This does not convert it into a Native preview-frame entry or class publication. Possible global-source fallback retains unknown executing receiver namespace and earlier local-binding obligations, and both diagnostic/source-edge subjects retain the same source receipt.
- [rust/tcl-compiler/src/analyser/types/original_receiver_body.rs](../../../../rust/tcl-compiler/src/analyser/types/original_receiver_body.rs), `analyser::types::original_receiver_body::tests::original_source_receiver_body_retains_child_declaration_without_native_entry` (linked): A genuine C8.6 child-source class method retains its original m declaration and complete body invocation independently of Native entry; partial vector, root vector and changed image refuse.
- [rust/tcl-compiler/src/analyser/types/original_receiver_body.rs](../../../../rust/tcl-compiler/src/analyser/types/original_receiver_body.rs), `analyser::types::original_receiver_body::tests::original_source_receiver_body_refuses_computed_or_recooked_script_mapping` (linked): Computed script and quoted recooked tab body inputs refuse borrowed original source mapping.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `analyser::interp_visibility::tests::original_child_receiver_visibility_keeps_unknown_namespace_and_body_owner_explicit` (linked): The visibility consumer uses the distinct selected original method-body declaration through its owns_invocation/full-input guard. This does not convert it into a Native preview-frame entry or class publication. Possible global-source fallback retains unknown executing receiver namespace and earlier local-binding obligations, and both diagnostic/source-edge subjects retain the same source receipt.

A named test is a coverage binding, not a claim that it executed.

## Replay

Marked selectors bind direct original source body ownership. Exact Rust outcomes require independently retained source/executable receipts; this source contract supplies no native body/frame/namespace or allocation observation.
