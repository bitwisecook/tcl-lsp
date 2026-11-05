# naming.lambda.original-namespace-constructor-and-getter

Kind: `native-observation`

## Problem statement

A source-file binary-produced namespace value and a raw counted original String can have different resident bytes. Lambda namespace selection must retain the actual constructor and getter channel instead of inferring raw00 behavior from CLI rendering.

## Question

For the nine fixed original namespace inputs, what do direct String versus C ByteArray lambda elements retain in sampled resident/type fields, actual Apply completion/result and the held original namespace getter?

## Conclusion

C8.5–C9.1 raw counted String00 inputs select the prefix namespace/root while preserving raw00 in the held getter. C ByteArray00 materializes asC080 and its full suffix cannot select the existing namespace. Current Jim preserves all counted String00 bytes in its literal prefixed namespace result and getter; Jim ByteArray is not tested. C8.4 records constructor/getter fields but cannot enter Apply. Compiler namespace selection consumes the actual retained OriginalVariableInvocation input at the effective lambda ordinal. The native list owner proves exact two-element omission or an exact third namespace child; policy and original static whole-image/full-config/currentness remain checked. Counted namespace units feed the shared lambda namespace path directly and join an unambiguous retained namespace inventory, preserving opaque surrogate units without a displayed String. Foreign static source, altered full config and unknown selected namespace refuse. This is a source-list/inventory assertion; it does not replay SDK raw String00/ByteArray rows or establish entered Apply, primary fields, namespace existence at runtime, native frames or effects.

## Scope

Six actual compiled SDK processes retain six queried version rows and99 independently initialized constructor/case interpreters:90 C String/ByteArray cases and9 Jim String cases. The594 reached setup/input/before/Apply/after/getter rows remain distinct from9 Jim NOT_TESTED rows. The BEFORE/AFTER fields sample only bytes-pointer presence and type name on the same held namespace object, without a preliminary namespace getter. Actual Apply result getter runs before AFTER; the held namespace getter runs afterwards. C8.4 post-invocation materialization occurs despite unsupported Apply and does not establish an entered worker. No pointer/address, arbitrary cache/header ownership, native command token, source/Document word, compiler admission, physical frame correspondence, arbitrary Normal completion or BIG-IP behavior is proved. CLI binary-produced values and output encoding remain a separate native question. Independent linked implementation coverage: Each Runtime/VM selector performs 81 original object constructor comparisons: nine fixed raw String and native ByteArray namespace objects for each of C85, C86, C90 and C91, plus nine original Jim String objects. Actual native-core fixture and selected native List constructors retain the original third element. Assertions compare only captured APPLY completion/result bytes and the retained namespace object's subsequent GETTER bytes. Raw00, ByteArray materialised C080 and raw C3A9 versus ByteArray C383C2A9 stay separate. BEFORE/AFTER native field samples, source words, allocation/cache/token identity, entered frame correspondence and arbitrary operation closure are not compared or donated. Jim byte-array constructor is not simulated. C84 unavailable Apply remains a native observation outside these comparison loops. Compiler source-list and retained namespace-inventory controls are independent of the SDK constructor replay. They prove exact omission/child/input correspondence and policy/source/config refusal only; they do not compare raw String00/ByteArray SDK rows or supply entered Apply, native frame/effects or runtime namespace-existence proof.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Direct original object vectors through Tcl_EvalObjv/Jim_EvalObjVector; counted original String or C ByteArray third element inside an original SDK List lambda. Each actual constructor/case has its own fresh initialized interpreter and separately caught namespace setup. Version: tcl8.4 8.4.20.

Apply is unavailable for all eighteen actual String/ByteArray inputs. The held original String getter preserves its counted bytes; the C ByteArray getter maps00 toC080 and the two UTF8 input bytes independently through Latin1. ByteArray resident-string flags change from0 before the outer call to1 afterwards while the type remains bytearray. These fields follow a failed outer invocation and cannot be attributed to an entered Apply worker.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Direct original object vectors through Tcl_EvalObjv/Jim_EvalObjVector; counted original String or C ByteArray third element inside an original SDK List lambda. Each actual constructor/case has its own fresh initialized interpreter and separately caught namespace setup. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All nine raw counted String Apply controls complete0: relative/absolute colon variants select::ns; empty/leading00 select root::; Unicode colon selects::é. Raw00 suffixes remain in held getter bytes but do not select a suffix namespace. C ByteArray inputs materialize00 asC080 and UTF8 byte values through Latin1; those four whole-suffix/non-ASCII variants fail namespace lookup, while plain/colon/absolute/empty controls succeed. Absolute successful inputs retain the sampled nsName type; relative inputs retain their original none/bytearray type. Every exact field and getter remains in the stream.

### jim

Status: `observed`. Channel: Direct original object vectors through Tcl_EvalObjv/Jim_EvalObjVector; counted original String or C ByteArray third element inside an original SDK List lambda. Each actual constructor/case has its own fresh initialized interpreter and separately caught namespace setup. Version: jim 0.84-9-g5bac7c9.

All nine counted original String Apply controls complete0. The returned namespace current bytes are the literal :: prefix plus the complete original input, including raw00/suffix and all colon spellings. Held getters preserve every counted input byte; resident/type fields stay1/none. All nine ByteArray constructors are explicitly NOT_TESTED because no native Jim ByteArray constructor is supplied. These results do not use the C namespace recipe.

### bigip

Status: `not-tested`.

No appliance observation of these exact original lambda namespace constructors and getters.

## Consumer bindings

- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `apply_native_c`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `lambda_namespace`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [runtime/rust/src/value_ops.rs](../../../../runtime/rust/src/value_ops.rs), `Interp::native_object_string_bytes`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::native_name_operand_bytes`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNamePurpose`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::lambda_namespace_input`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::lambda_namespace_path`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `walk_lambda_invocation`: Retain exact original whole operand, input/configuration and selected native namespace purpose independently of presentation names and runtime entry.
- [rust/tcl-compiler/src/command_binding/namespace_slots.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_slots.rs), `namespace_for_lambda_operand`: Retain exact original whole operand, input/configuration and selected native namespace purpose independently of presentation names and runtime entry.
- [runtime/rust/src/cmd_proc/native_apply_original_tests.rs](../../../../runtime/rust/src/cmd_proc/native_apply_original_tests.rs), `cmd_proc::native_apply_original_tests::original_lambda_namespace_objects_match_constructor_and_getter_controls` (linked): Each Runtime/VM selector performs 81 original object constructor comparisons: nine fixed raw String and native ByteArray namespace objects for each of C85, C86, C90 and C91, plus nine original Jim String objects. Actual native-core fixture and selected native List constructors retain the original third element. Assertions compare only captured APPLY completion/result bytes and the retained namespace object's subsequent GETTER bytes. Raw00, ByteArray materialised C080 and raw C3A9 versus ByteArray C383C2A9 stay separate. BEFORE/AFTER native field samples, source words, allocation/cache/token identity, entered frame correspondence and arbitrary operation closure are not compared or donated. Jim byte-array constructor is not simulated. C84 unavailable Apply remains a native observation outside these comparison loops.
- [rust/tcl-vm/src/command/native_apply_original_tests.rs](../../../../rust/tcl-vm/src/command/native_apply_original_tests.rs), `command::native_apply_original_tests::original_lambda_namespace_objects_match_constructor_and_getter_controls` (linked): Each Runtime/VM selector performs 81 original object constructor comparisons: nine fixed raw String and native ByteArray namespace objects for each of C85, C86, C90 and C91, plus nine original Jim String objects. Actual native-core fixture and selected native List constructors retain the original third element. Assertions compare only captured APPLY completion/result bytes and the retained namespace object's subsequent GETTER bytes. Raw00, ByteArray materialised C080 and raw C3A9 versus ByteArray C383C2A9 stay separate. BEFORE/AFTER native field samples, source words, allocation/cache/token identity, entered frame correspondence and arbitrary operation closure are not compared or donated. Jim byte-array constructor is not simulated. C84 unavailable Apply remains a native observation outside these comparison loops.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::tests::original_lambda_namespace_preserves_counted_object_and_lookup_purposes` (linked): Pure C8.5–C9.1 original getter-byte prefix and subsequent independent CString lookup projection retain count/zero/colon/Unicode boundaries. Unsupported C8.4/Jim C-specific purpose refuses. This is Rust API correspondence only, with no actual namespace existence, object/header identity, entered frame or native CLI replay.
- [rust/tcl-compiler/src/command_binding/namespace_slots.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_slots.rs), `command_binding::namespace_slots::tests::original_lambda_namespace_uses_native_list_children_and_proved_omission` (linked): Compiler source selection uses the retained OriginalVariableInvocation and native list owner for exact two-element omission or the third counted namespace child, preserving opaque units through shared path/inventory selection without display conversion.
- [rust/tcl-compiler/src/command_binding/namespace_slots.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_slots.rs), `command_binding::namespace_slots::tests::original_lambda_namespace_refuses_foreign_source_and_configuration` (linked): The static lambda source namespace projection refuses changed full source/configuration or an unknown selected namespace; no SDK constructor replay or entered Apply result is inferred.

## Evidence

- [rust/tcl-registry/tests/data/native_lambda_object_getter](../../../../rust/tcl-registry/tests/data/native_lambda_object_getter)
- [rust/tcl-registry/tests/data/native_lambda_object_getter/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_object_getter/8.4.20/stdout.tsv)
- [rust/tcl-registry/tests/data/native_lambda_object_getter/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_object_getter/8.5.19/stdout.tsv)
- [rust/tcl-registry/tests/data/native_lambda_object_getter/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_object_getter/8.6.18/stdout.tsv)
- [rust/tcl-registry/tests/data/native_lambda_object_getter/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_object_getter/9.0.4/stdout.tsv)
- [rust/tcl-registry/tests/data/native_lambda_object_getter/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_object_getter/9.1.0/stdout.tsv)
- [rust/tcl-registry/tests/data/native_lambda_object_getter/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_object_getter/jim/stdout.tsv)
