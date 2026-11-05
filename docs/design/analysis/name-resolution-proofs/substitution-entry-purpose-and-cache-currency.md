# naming.substitution.entry-purpose-and-cache-currency

Kind: `implementation-contract`

## Problem statement

An original native substitution template is neither an ordinary Tcl script nor a procedure body. Calling the ordinary CompileService path can change template parsing, completion handling, source-object cache type and borrowed local-name ownership. A same-spelled command or foreign interpreter must not manufacture the selected handler receipt; source getters may mutate the command table after genuine selection.

## Question

Does the substitution API preserve genuine selected-handler/interpreter provenance and independent template/cache purpose without script or procedure fallback?

## Conclusion

The API captures a unique actual stock handler row before original source getters, then requires the same interpreter and physical C86/C90/C91 release at template compilation. Template ingress is NativeValue with the independently retained namespace and full lexical configuration. Services without the template-entry purpose explicitly refuse. The Compiler emits native template parts/completion handling; VM and Runtime retain separate substcode caches with flags, current interpreter/namespace/compiler context and borrowed local-name table, without procPtr or ordinary script cache admission. This contract is asserted by the linked discriminators; no Rust execution result is inferred from the native captures.

## Scope

C86/C90/C91 compiled original-template entry and independent Runtime implementation. C84/C85 retain their existing direct implementation and Jim its existing original token path; they do not acquire this compiled C purpose. Unknown/foreign handler/world/string context, Document template and missing service purpose refuse. The contract does not grant arbitrary original-object class, namespace existence, variable Normal, procedure body ownership or editor coordinates.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This record is a Rust implementation contract; no native or Rust execution is claimed.

## Consumer bindings

- [rust/tcl-runtime-api/src/native_substitution.rs](../../../../rust/tcl-runtime-api/src/native_substitution.rs), `NativeSelectedSubstitutionHandler::capture`: Unique actual selected handler row and interpreter before original getter callbacks.
- [rust/tcl-runtime-api/src/native_substitution.rs](../../../../rust/tcl-runtime-api/src/native_substitution.rs), `NativeSubstitutionCompilationEntry::from_selected_handler`: Same-interpreter native C compiled-template entry independent of script/procedure purpose.
- [rust/tcl-runtime-api/src/lib.rs](../../../../rust/tcl-runtime-api/src/lib.rs), `CompileService::compile_substitution_with_entry`: Explicit template compiler injection, with unavailable default and no script fallback.
- `rust/tcl-compiler/src/codegen/native_substitution.rs`, `CodegenCtx::emit_substitution`: Shared actual native word parts plus native substitution completion branches.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::prepare_original_substitution`: Original source getter before current snapshot and independently validated substcode cache.
- [runtime/rust/src/interp/native_body_artifact/native_substitution.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_substitution.rs), `Interp::prepare_original_c_substitution`: Independent runtime original template artifact and strong borrowed local-name ownership.
- [rust/tcl-vm/src/interp/native_substitution_entry_tests.rs](../../../../rust/tcl-vm/src/interp/native_substitution_entry_tests.rs), `interp::native_substitution_entry_tests::substitution_entry_requires_selected_handler_and_independent_source_context` (linked): Genuine entry positive; opaque handler, foreign interpreter, unknown engine/string recipe, mismatched recipe, Document and script-only service negatives; legacy/Jim cannot acquire compiled purpose.

## Evidence

- [rust/tcl-runtime-api/src/native_substitution.rs](../../../../rust/tcl-runtime-api/src/native_substitution.rs)
- [rust/tcl-vm/src/interp/native_substitution_entry_tests.rs](../../../../rust/tcl-vm/src/interp/native_substitution_entry_tests.rs)
- [runtime/rust/src/interp/native_body_artifact/native_substitution.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_substitution.rs)
