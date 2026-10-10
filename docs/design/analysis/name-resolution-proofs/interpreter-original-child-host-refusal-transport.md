# naming.interpreter.original-child-host-refusal-transport

Kind: `implementation-contract`

## Problem statement

A child evaluation can reach a host-only typed failure. Transporting it as a guest result/options pair would lose its cause and can overwrite the parent result despite having no guest completion receipt.

## Question

How does original child evaluation retain a reached typed host refusal and the existing parent result without manufacturing guest return options or cross-interpreter value/lookup identity?

## Conclusion

child_eval_original evaluates the selected original body in the actual child. Before guest completion capture it checks host_refusal_pending. A reached child refusal transports the actual admission/access failure fields through transport_host_refusal_from and returns Error without publishing a guest result/options pair. An existing parent refusal remains authoritative for each corresponding field; both interpreter states remain retained. The parent result stays unchanged. The fixed authored host-worker control demonstrates typed access-cause transport and prior-parent precedence, not a native child interpreter, guest error-options equivalence, value allocation, lookup identity or successful command execution.

The Runtime embedding adapter shares this host-only boundary. Its typed host refusal is uncatchable by guest catch, preserves effects already reached and leaves pending result/return/options state intact. Actual child transport retains an earlier parent cause and result. Explicit guest ScriptBytes errors and nonstandard completions retain a separate original options path; no host reason is fabricated as guest error options.

The central CmdCore refusal adapter preserves the first original typed ValueAccessRefusal independently of later failures. Its software control neither enters a child nor supplies Native command/value-operation availability; the original child-host public observation remains a separate measured scope.

The linked Wasm adapter control keeps an explicit host ExecutionRefusal outside guest catch and retains earlier guest effects. It observes the actual embedding API only when its required Wasm fixture is available; a skipped fixture supplies no executed outcome.

## Scope

One fixed Runtime source/API control uses an explicit authored host worker in a created model child, with and without an existing parent refusal. It compares the actual retained typed cause in both states and unchanged parent result. All seven native providers are not tested; no external process or native object/header/frame observation answers this API question.

One additional embedding software control binds this typed-host boundary and prior-parent precedence; it does not execute a native child or assert C/Jim guest exception equivalence. Exact counted result/resident facts are authored Rust inputs, not independently produced Native object observations.

One additional adapter-only current software control has no executed assertion receipt. Exact typed first-cause preservation does not issue child execution, Native invocation/frame or external provider identity.

This secondary source binding does not measure original child-interpreter creation, argv, callback or C/Jim frame behaviour and supplies no Native equivalence. Original provider answers remain tied to their existing captures; the independent embedding transport contract describes this software adapter. No assertion outcome is attached.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `naming-interpreter-original-child-host-refusal-transport-native_children.rs` (implementation): [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs). SHA-256 `8c881f13d44ad4f867fb1a888f488d2ad471a46d299332175dc43aa0c4fd4635`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-interpreter-original-child-host-refusal-transport-native_compilation.rs` (implementation): [runtime/rust/src/interp/native_compilation.rs](../../../../runtime/rust/src/interp/native_compilation.rs). SHA-256 `78abdaf067bb75527a8a4236e1582f702bb717d6a087c649d823350188af8ff9`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `Interp::child_eval_original`: Keep original child body selection and detect actual retained host-only refusal before guest completion capture.
- [runtime/rust/src/interp/native_compilation.rs](../../../../runtime/rust/src/interp/native_compilation.rs), `Interp::transport_host_refusal_from`: Transport reached typed child failure fields while preserving prior parent fields and parent result; no guest options or value identity.
- [runtime/rust/src/interp/native_compilation.rs](../../../../runtime/rust/src/interp/native_compilation.rs), `Interp::host_refusal_pending`: Keep retained admission/access refusal separate from guest completion.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `interp::native_children::tests::child_host_failure_keeps_its_typed_cause_and_parent_result` (linked): Explicit authored child host worker keeps its typed access failure and prior-parent precedence with unchanged parent result; no native execution observation.

A named test is a coverage binding, not a claim that it executed.

- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `fail`: Keep typed host execution refusal outside guest catch, retaining prior interpreter result/return state and effects; only explicit guest Script/ScriptBytes failures use the guest completion owner.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `capture_answer`: Detect retained host refusal before guest result/options capture; a child host cause does not replace prior parent refusal or result.
- [runtime/rust/src/engine.rs](../../../../runtime/rust/src/engine.rs), `engine::tests::host_refusal_is_uncatchable_and_retains_prior_effects_and_completion_state` (linked): Typed host execution refusal remains outside guest catch/completion, retains earlier effects and prevents later effects. The model keeps the prior result/resident bytes, pending return state/options and prior-parent host cause when transporting a child refusal.

These source/API bindings carry no executed assertion or Native provider result.

- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `completion_from_cmd_error`: Route original CmdCore typed refusal through the existing first-Host-cause owner; preserve typed identity instead of construct a String Host failure or guest message.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `command::tests::original_command_core_refusal_keeps_typed_first_host_cause` (linked): The shared CmdCore error adapter retains the first actual typed ValueAccessRefusal when a later distinct refusal is reported; it does not enter a child or establish native command/value availability.

These bindings are current software contracts without an executed assertion or external provider result.

- [rust/tcl-engine-wasm/tests/under_wasm.rs](../../../../rust/tcl-engine-wasm/tests/under_wasm.rs), `RefusingHost::invoke`: Supply an explicit host contract refusal to the Wasm adapter independently of Tcl guest error/completion metadata or child-interpreter creation.
- [rust/tcl-engine-wasm/tests/under_wasm.rs](../../../../rust/tcl-engine-wasm/tests/under_wasm.rs), `host_refusal_bypasses_guest_catch_and_keeps_prior_effects` (linked): The actual Wasm host adapter ExecutionRefusal bypasses guest catch and preserves a prior guest store; the following store is absent. Requires the Wasm build/toolchain; early unavailable return is not an executed pass. This is an adapter software control, not an original child-interpreter observation.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
