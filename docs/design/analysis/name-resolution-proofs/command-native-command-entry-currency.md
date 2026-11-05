# naming.command.native-command-entry-currency

Kind: `implementation-contract`

## Problem statement

A captured compiled command cannot borrow its command-entry validation rule from a profile label or continue with obsolete compiler/resolver epochs. Command selection and argument evaluation are separate boundaries.

## Question

Which actual engine and independent compiler/resolver epochs select compiled command-entry source revalidation, and why does that guard supply no captured opcode or argument execution proof?

## Conclusion

NativeCompilationGuard::for_native_dialect requires the actual retained NativeNameProtocol::C release. Plain Logical, Jim and hosted profiles issue no C guard. The selected C8.4 recipe retains ChunkEntry; C8.5 and later retain BeforeArguments. revalidate_source checks compiler and resolver epoch changes independently only at the selected BeforeArguments boundary. Runtime execute_body_instruction compares the actual captured/current epochs before preparing arguments and re-enters the original command source on a required revalidation. The guard authenticates no compiler hook, captured opcode, argument value, Native execution or successful world transition. Public mathop source/envelope observations remain independent; the pure selected recipe test does not turn their result into a private instruction or token claim. NativeCompilationGuard::requires_current_epochs distinguishes the original release boundary: C8.4 ChunkEntry keeps its captured selection while operands execute and does not require an epoch obtained from those operand mutations; C8.5 through C9.1 BeforeArguments independently requires current compiler/resolver epochs and revalidates when either changes. Actual engine selection, captured artifact identity and opcode/admission proof remain separate.

## Scope

One fixed Registry source/API control exercises each actual C recipe, independent changed compiler/resolver epochs and unchanged pairs, with Logical/Jim/hosted no-guard controls. All seven providers are not tested for this API question. No external process or current opcode/token/frame observation is added.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Consumer bindings

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationGuard`: Keep actual release-selected command-entry currency separate from captured opcode and argument execution.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationGuard::for_native_dialect`: Select the C guard only from actual retained engine/release purpose.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationGuard::revalidate_source`: Compare compiler/resolver epochs independently under the selected command-entry boundary.
- [runtime/rust/src/interp/native_body_artifact.rs](../../../../runtime/rust/src/interp/native_body_artifact.rs), `Interp::execute_body_instruction`: Retain actual source extent and validate selected command currency before argument preparation.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationGuard::requires_current_epochs`: Require current compiler/resolver epochs at BeforeArguments only, independently of captured C8.4 chunk-entry selection.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::native_command_currency_tests::original_command_currency_uses_actual_engine_and_independent_epochs` (linked): Actual native engine selects ChunkEntry versus BeforeArguments; only the latter requires current epochs, and independent compiler/resolver changes trigger revalidation. Other engine/profile-only inputs cannot borrow that guard.

## Evidence

- [runtime/rust/src/interp/native_body_artifact.rs](../../../../runtime/rust/src/interp/native_body_artifact.rs)
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs)
