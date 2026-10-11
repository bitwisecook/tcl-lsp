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

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This record is a Rust implementation contract; no native or Rust execution is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This record is a Rust implementation contract; no native or Rust execution is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This record is a Rust implementation contract; no native or Rust execution is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This record is a Rust implementation contract; no native or Rust execution is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This record is a Rust implementation contract; no native or Rust execution is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This record is a Rust implementation contract; no native or Rust execution is claimed.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This record is a Rust implementation contract; no native or Rust execution is claimed.

## Exact evidence

- `e0` (implementation): [rust/tcl-runtime-api/src/native_substitution.rs](../../../../rust/tcl-runtime-api/src/native_substitution.rs). SHA-256 `25e8c49e680c9b5706488cd00fa8f52e8e3debbea654bf2ae28743e8a316dc3e`. Implementation source at the recorded digest. It is not native execution evidence.
- `e1` (implementation): [rust/tcl-compiler/src/codegen/native_substitution.rs](../../../../rust/tcl-compiler/src/codegen/native_substitution.rs). SHA-256 `d3843f2992315380b7255af10f3499034ae1632a7934e1d740d95aa2f5d12ec7`. Implementation source at the recorded digest. It is not native execution evidence.
- `e2` (implementation): [rust/tcl-vm/src/interp/native_substitution_entry_tests.rs](../../../../rust/tcl-vm/src/interp/native_substitution_entry_tests.rs). SHA-256 `d26f4995fe1178068ad587a6ba8567976d7131ef63f6350198dc390c3cd73133`. Implementation source at the recorded digest. It is not native execution evidence.
- `e3` (implementation): [runtime/rust/src/interp/native_body_artifact/native_substitution.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_substitution.rs). SHA-256 `52b13c74f652f327c77299c3834972a4fee13c119e8614954857cbebe2b44dae`. Implementation source at the recorded digest. It is not native execution evidence.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-runtime-api/src/native_substitution.rs](../../../../rust/tcl-runtime-api/src/native_substitution.rs), `NativeSelectedSubstitutionHandler::capture`: Unique actual selected handler row and interpreter before original getter callbacks.
- [rust/tcl-runtime-api/src/native_substitution.rs](../../../../rust/tcl-runtime-api/src/native_substitution.rs), `NativeSubstitutionCompilationEntry::from_selected_handler`: Same-interpreter native C compiled-template entry independent of script/procedure purpose.
- [rust/tcl-runtime-api/src/lib.rs](../../../../rust/tcl-runtime-api/src/lib.rs), `CompileService::compile_substitution_with_entry`: Explicit template compiler injection, with unavailable default and no script fallback.
- [rust/tcl-compiler/src/codegen/native_substitution.rs](../../../../rust/tcl-compiler/src/codegen/native_substitution.rs), `CodegenCtx::emit_substitution`: Shared actual native word parts plus native substitution completion branches.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::prepare_original_substitution`: Original source getter before current snapshot and independently validated substcode cache.
- [runtime/rust/src/interp/native_body_artifact/native_substitution.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_substitution.rs), `Interp::prepare_original_c_substitution`: Independent runtime original template artifact and strong borrowed local-name ownership.
- [rust/tcl-vm/src/interp/native_substitution_entry_tests.rs](../../../../rust/tcl-vm/src/interp/native_substitution_entry_tests.rs), `interp::native_substitution_entry_tests::substitution_entry_requires_selected_handler_and_independent_source_context` (linked): Genuine entry positive; opaque handler, foreign interpreter, unknown engine/string recipe, mismatched recipe, Document and script-only service negatives; legacy/Jim cannot acquire compiled purpose.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-vm",
  "interp::native_substitution_entry_tests::substitution_entry_requires_selected_handler_and_independent_source_context",
  "--",
  "--exact"
]
```

Run the listed selectors against the complete current API, Compiler, VM and runtime implementation. Native oracle captures do not execute or validate this Rust implementation. Native frame/refcount observations remain independent empirical questions.
