# Original procedure argv transport

Status: **UNAPPLIED / UNCOMPILED / UNEXECUTED**. The implementation and controls
are proposed software changes. They establish no Runtime/compiler pass or
emitted-WASM result. Original CLI evidence remains in the parent directory.

## Semantic transport

`NativeOp::DefineProc` becomes `{entry_key: String, argv: Vec<NativeValueId>}`.
`lower_invoke` supplies its existing evaluated argv to `lower_definition`.
The actual name argument is `argv[1]`; it is not reconstructed from `Procedure.name`
or `Procedure.qualified_name`. The internal entry key only selects the compiled
body's table slot. Original formals and body use the same complete argument vector.

Current literal/vector/procedure-span/dispatch and native compilation admission
checks remain independent. Substituted names do not gain definition eligibility.
The fixed C86/C9 compiler control must require a real `DefineProc` instruction;
a declined source cannot pass by merely comparing a QName formula. If the actual
retained entry/admission setup refuses it, that failure stays explicit. A default
Native frame or entry cannot force the control through.

One additive `CodegenAbiImportId::ProcDefineOriginalArgv` imports
`tcl_codegen_proc_define_original_argv(argv, argc, entry, out) -> status`.
All four parameters are i32 on wasm; `entry=0` supplies source-only publication.
Both old `ProcRegister` and `ProcDefineNative` signatures and exports stay intact.
The descriptor is in `ALL` and retains default Host refusal checks.

The native emitter reuses its borrowed argv frame and completion-adoption code.
Frame sizing includes `DefineProc`; only the internal key reaches `push_entry_index`.
The Runtime entry reuses `tcl_invoke_argv`'s input validation, `BorrowedArgv`,
`AbiActivation` and `CodegenOperation::settle` conventions, but invokes the shared
original procedure handler with the independently checked optional compiled entry.

The object-aware owner is `cmd_proc::proc_cmd_original`, not the lower-level byte
installer. The stock handler and additive ABI both enter `OriginalArguments`.
The shared handler retains actual name objects, Jim key preparation, selected
namespace/name validation, formals/body order, body choice, result convention and
operation checks. Its existing `install_proc_chosen_storage_with_original_key`
call receives the optional compiled entry. No header or holder is recreated from
source metadata or reporting text.

The publication namespace is the actual live current namespace under the entered
Runtime operation. A rendered source/QName cannot substitute for it. Independent
source binding/context/admission requirements remain mandatory; absent or changed
ownership keeps the existing generic invocation or typed refusal.

## General backend and cleanup

The general emitter invokes the existing complete once-evaluated leaf argv path
for procedure declarations. The former `ProcRegister` shortcut always forwarded
`entry=None`; it did not install a compiled NativeProcEntry. Native installation
continues through the separate `DefineProc` set and internal table key.
General procedure planning and exports remain; the generic path changes dispatch,
argument-object creation and parser work. It can expose original validation and
admission that the byte-registration shortcut omitted. Performance and full
provider equivalence remain unmeasured.

The always-false direct-definition emitter, its private operation/literal maps,
recording helper and span-to-Procedure copies are removed within the backend owner.
The independent native `definition_words_are_written_out` check stays unchanged.
Existing backend tests now inspect the actual leaf argument plan: a substituted
body is a scalar read; a literal body stays inert data; a root-colon name remains
its original operand. Actual real-link controls separately check materialized
body text, once-only effects and inert body text. No dead-code suppression is added.

## Lifetime and completion boundary

Native `Invoke` already drops the transport status after its checked call.
`NativeEmitter::call` checks `host_refusal_pending` before returning to adoption
and branches to `Label::Exit`, which releases owned locals and call-frame storage.
The additive import is not exempted by either transport-only allowlist. Host
refusal therefore leaves the completion out triple unread. Only the non-Host path
adopts code/result/options into owned locals through `adopt_frame_completion`.
Runtime `settle` captures while activation owns Guest state, drops activation,
validates currency and publishes out. These are inspected code paths, not a run
of the proposed ABI.

## ABI compatibility

`CODEGEN_ABI_VERSION` is derived from all descriptors and transport layout.
`ArtefactIdentityManifest` records that fingerprint and strictly compares it with
the actual Runtime's identity. Old import symbols remain callable, but a module
with the prior fingerprint is refused. The proposal does not widen this gate.
New compiler and Runtime artifacts must use the same descriptor table.

[abi-layout.json](abi-layout.json) retains the exact source preimage, descriptor
order and layout values. Its source-derived fingerprints are 0x034bab19 for the
63-descriptor table and 0x4dc7e147 after the proposed additive descriptor. This
arithmetic is not an executed old Runtime/module result. The proposed real-link
control checks actual loader refusal for that prior fingerprint, accepts the
current manifest and executes the new four-i32 import signature. Existing
arbitrary-foreign-ABI refusal and old-signature checks remain.

No generated ABI report was found in the bounded `docs/generated` and `xtask`
surfaces. Actual Runtime export/build/manifest checks remain required. A generated
report outside those surfaces must come from the actual table, not a manual hash.

## Owners and controls

The seven implementation owners in [implementation.patch](implementation.patch):

- `native_lowering/ir.rs`: evaluated argv and internal entry key.
- `native_lowering/lower.rs`: existing evaluated vector and preserved admission.
- `codegen/wasm/native_emit.rs`: additive import, borrowed frame and completion.
- `codegen/wasm/backend.rs`: general argv route, private cleanup, internal table key.
- `tcl-runtime-api/src/codegen_abi.rs`: additive descriptor and `ALL`.
- `runtime/rust/src/cmd_proc.rs`: original handler's optional compiled entry.
- `runtime/rust/src/codegen_abi.rs`: additive borrowed-argv entry.

[test-updates.patch](test-updates.patch) updates existing definition assertions,
private backend-plan tests, native import counters and actual link-import checks.
Both original compilation-admission refusal controls require zero new-definition
calls. The lazy source-body allowance includes the new import. The general add
case now expects three generic invocations; its opaque rebound case remains four.

[compiler-controls.rs](compiler-controls.rs) requires actual C86/C9 definition
selection and compares the evaluated original operand through the shared
publication owner, independently of the compiled entry key. Counting substitutions,
unknown reads, replaced definers and missing vectors must keep the correct fallback.

[runtime-controls.rs](runtime-controls.rs) uses actual Runtime construction and
live namespace IDs for root, named, colon and missing holders, caller-owned
argument objects and completion cleanup. Its `Some(stub_body)` callable is
explicitly Rust software ABI coverage; it grants no Native provenance.

[wasm-controls.rs](wasm-controls.rs) uses the actual linked Runtime/Wasmtime
harness. The root-colon test requires two installed compiled bodies, two actual
native dispatches, distinct results and absence of the plain name. Additional
controls check strict manifest refusal/current signature, a body substitution
executed once, stored materialized body text and inert braced body text. None is
executed here, and no oracle conformance result is attributed to these sources.

Before acceptance, integrate and run these controls with the actual Runtime,
retaining registration, replacement, lazy-body, frame, callback and cleanup checks.
The shipping WASM contract must describe evaluated original argv and internal keys,
with both legacy imports retained. Source preimages and artifact hashes are in
[provenance.json](provenance.json); preparation scripts produce temporary diffs
and never apply them to production files.
