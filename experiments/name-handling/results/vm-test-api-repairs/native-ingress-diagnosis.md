# Native compilation ingress diagnosis

Question: where do the retained real-init and legacy C9.1 controls first lack original compilation input or an implemented recipe?

The retained [real-init refusal](../vm-c86-mathfunc-bootstrap/real-init.stderr.txt) and [legacy C9.1 refusal](../vm-c86-mathfunc-bootstrap/derived-registration.log) both report `NativePreflightRequired`. Neither proves library-script or C9.1 test execution.

Legacy first missing input: [run_at](../../../../rust/tcl-vm/tests/mathfunc_derivation_e2e.rs#L59) calls `svc.compile(src)` before creating its VM. [compile](../../../../rust/tcl-compiler/src/compile_service.rs#L562) selects whole-module compilation with `entry=None`; a profile label supplies no original interpreter entry.

Existing genuine route: construct `Vm::with_native_core` using the selected profile, actual host and `NativeBootstrapInputs`; install `BytecodeCompileService::for_profile(profile)`; call [Vm::try_eval_source](../../../../rust/tcl-vm/src/interp.rs#L24588) on the unchanged source string. For explicitly retained channels, use [Vm::try_eval_source_image](../../../../rust/tcl-vm/src/interp.rs#L24620) with the original `SourceImage` channel; do not silently convert a Document fixture into Native.

The current [VM entry producer](../../../../rust/tcl-vm/src/interp.rs#L23609) captures the actual interpreter/frame/namespace, command registrations, lexical/string policies, tables and compiler-pass environment. [compile_module_bytes](../../../../rust/tcl-vm/src/interp.rs#L24377) passes `ScriptCompileTargetBytes { source, namespace }`, the current profile and that entry to [CompileService::compile_script_bytes_with_entry](../../../../rust/tcl-compiler/src/compile_service.rs#L698). No fabricated unit or manually labelled entry is necessary.

Real C8.6 init already uses that route: [retained runner](../vm-c86-mathfunc-bootstrap/real-init-runner.rs) constructs an actual native core, installs the matching compiler and calls [try_init_library](../../../../rust/tcl-vm/src/interp.rs#L4291). [init_library](../../../../rust/tcl-vm/src/interp.rs#L4240) evaluates the exact `source <selected init.tcl>` wrapper through `eval_source`. Replacing this with another generic entry API does not identify its missing recipe.

The exact real-init recipe remains unanswered: [validate_native_function](../../../../rust/tcl-compiler/src/compile_service.rs#L52) collapses unresolved provider/preflight obligations into the same admission error. The scalar retained receipt has no first-refusing source span or recipe subtype. Static inspection cannot establish that the wrapper or a later library command reached compilation.

Small next diagnostic: reuse the [RecordingCompiler](../../../../rust/tcl-vm/tests/native_mathfunc_bootstrap.rs#L24) adapter, retaining each actual source image/channel and captured entry before delegation. The existing [TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC gate](../../../../rust/tcl-compiler/src/codegen/mod.rs#L684) reports the first refusing emitter caller/span/source. Replay only the exact real-init recipe with those observations, preserve its return/refusal, and keep preflight-admission gaps explicit when no emitter diagnostic appears.

CLI is separate: [run_script](../../../../rust/tcl-vm-cli/src/main.rs#L260) already calls `vm.eval_source`; [build_vm](../../../../rust/tcl-vm-cli/src/main.rs#L183) constructs legacy `Vm::with_output`, then [configure_vm](../../../../rust/tcl-vm-cli/src/main.rs#L209) changes its profile. A native-core constructor change would alter that host contract and needs its own bounded review.

This diagnosis used source/log inspection only. No source edits, Cargo/cache access, execution, network or raw F5 reads. The 29 repaired VM controls remain the next authorised validation after cache release.
