# Original procedure argv transport

The scratch implementation is [implementation.patch](implementation.patch).
It is unapplied, uncompiled and unexecuted. It changes seven implementation
files; existing tests and ABI expectations listed below still need integration.
The proposed controls are [compiler-controls.rs](compiler-controls.rs).

`NativeOp::DefineProc` becomes `{entry_key: String, argv: Vec<NativeValueId>}`.
`lower_invoke` supplies its existing evaluated argv to `lower_definition`.
All current literal/vector/procedure-span/dispatch and native-compilation gates
remain in place; this does not broaden admission to a substituted name.
`argv[1]` is the already evaluated publication-name object, not a string rebuilt
from `Procedure.name` or `Procedure.qualified_name`. Internal table binding keeps
`entry_key`. The retained formals/body argument objects use the same single argv.

One additive `CodegenAbiImportId::ProcDefineOriginalArgv` imports
`tcl_codegen_proc_define_original_argv(argv, argc, entry, out) -> status`.
On wasm all four parameters are i32; `entry=0` supplies source-only publication.
Old `ProcRegister` and `ProcDefineNative` signatures and exports stay intact.
The descriptor is added to `ALL`; the existing derived ABI fingerprint changes
through that actual descriptor. Default host-refusal classification includes it.

Native emission reuses its borrowed argv frame and completion-adoption machinery.
The argv frame sizing walker must include DefineProc. It passes only the internal
key to `push_entry_index`, never to the runtime's name parser. The returned status
is treated with the same host-refusal checks as Invoke, and the exact completion
triple is adopted through the existing cleanup path.

The Runtime implementation copies the existing tcl_invoke_argv entry/argc/null/
BorrowedArgv/AbiActivation/CodegenOperation::settle structure, but calls the
shared procedure handler with an independently checked optional NativeProcEntry.
The stock handler and the additive ABI both enter OriginalArguments and call
`proc_cmd_original`. Its only semantic change is supplying that optional entry
at its existing install_proc_chosen_storage_with_original_key call. The actual
original name object, Jim key preparation, selected name validation, formals/body
order, result convention, body choice and operation checks remain shared.

There is no existing object-valued lower-level installer: install_proc_original_storage
accepts bytes. The existing object-aware owner is proc_cmd_original (cmd_proc.rs:105),
which obtains the original name from argv, checks native namespace/name grammar,
and retains the actual Jim key before its installer call at line 305. The new
helper exposes this owner instead of reimplementing the original name handling.
The actual namespace is the live current namespace under the entered Runtime
operation. A rendered source/QName cannot create or substitute that holder.
Current compile-time binding/context/admission obligations must remain independent;
a changed or unavailable source context continues its existing typed decline or
ordinary generic invocation. This patch creates no original object or holder from
source metadata and does not make a captured VM registration a Native compiler pass.

The general WASM emitter declines its direct ProcRegister specialization as a
whole, so its existing leaf plan evaluates the complete original argument vector
once and invokes the runtime normally. This stays source-only and avoids a second
literal-name facts structure or an argv reevaluation. The old import can remain
for compatibility while subsequent cleanup determines whether its unused generated
import needs removal. No root-colon special case is introduced.

Implementation owners in the scratch patch:
- native_lowering/ir.rs: DefineProc operand shape.
- native_lowering/lower.rs: evaluated argv argument and preserved admission gates.
- codegen/wasm/native_emit.rs: import, argv frame sizing, borrowed call/completion.
- codegen/wasm/backend.rs: general direct-definition decline; collect_defined_procs
  reads entry_key rather than the publication operand.
- tcl-runtime-api/src/codegen_abi.rs: additive ID, ALL and exact descriptor.
- runtime/rust/src/cmd_proc.rs: original handler's optional compiled-entry parameter.
- runtime/rust/src/codegen_abi.rs: additive borrowed-argv entry.

Current consumers and test updates still required (not covered by a build claim):
- native_lowering/tests.rs: the ordinary greet exact DefineProc struct assertion
  must instead verify entry key plus the existing evaluated argv values; retain
  existing redefinition/materialized-body/substituted-body decline tests.
- codegen/wasm/native_emit.rs: result_source already handles DefineProc as whole
  completion; can_fail already treats it as fallible. Neither needs weakened
  classification. Its direct variant match is in the scratch patch.
- codegen/wasm/backend.rs: collect_defined_procs is the other direct field consumer;
  table_install and entry maps keep their internal keys.
- codegen/wasm/pipeline.rs: new-import counters and the lazy-source definition
  allowance must include ProcDefineOriginalArgv; original admission-refusal test
  must assert zero new-import calls as well as the old import's zero calls.
- tests/wasm_codegen.rs: three ProcDefineNative counter sites at current lines
  1442/1708/1725 become the additive ID; the old generic ProcRegister expectation
  near line 201 becomes once-evaluated InvokeArgv behavior.
- tests/wasm_real_link.rs: compile_native_bound near line 1164 checks the additive
  definition call; its real table and runtime-entry checks remain mandatory.
- tcl-runtime-api/src/codegen_abi.rs: add four-i32-to-i32 descriptor and default
  refusal check assertions; preserve seven-i32 old-import compatibility assertions.
- runtime/rust/src/codegen_abi.rs: actual borrowed-object publication tests with
  current real namespace IDs, completion cleanup, no-entry and compiled-entry
  binding/replacement; retain existing old-import compatibility tests.
- runtime/rust/src/interp.rs comments mentioning the old import's sole provenance
  need wording encompassing both imports, without implying new identity evidence.
- Production WASM linking/export visibility must be checked on the actual build;
  no explicit hand-maintained export list was found for this import. runtime/rust/
  build.rs owns shared table linking; no extra table ABI change is proposed.

The fixed C86/C9 compiler test must require an actual DefineProc instruction;
a completely refused source cannot pass by comparing a QName formula. It checks
its evaluated argv[1] against NativeNameProtocol::command_publication_slot(root)
and separately proves that the entry key selects a different slot as fresh text.
If the existing actual entry/admission setup refuses this source, preserve that
failure and establish the applicable entry from its owner before changing the
control. Do not insert a default Native entry/frame to force a green assertion.
Dynamic counting, unknown read and shadowed-definer inputs separately keep generic
argv behavior; only real Runtime/WASM execution can establish once-only effects.
The current original CLI capture supplies public publication distinction only.


## Review evidence and compatibility limit

The general fallback is a real behavior/performance change, not a measured
no-op. Its old ProcRegister call (backend.rs:759) forwards entry=None
(Runtime codegen_abi.rs:1701), so that particular call never installed a
compiled NativeProcEntry. Native table entries are collected from native
DefineProc opcodes at backend.rs:2435; their separate entry_key and installation
remain. General procedure_plan/direct export generation is unchanged. Existing
direct_calls facts are populated/tested but are not read to invoke a compiled
callee in this current backend. The generic fallback now enters the original
handler with evaluated objects and may reach validation/admission that the old
registration shortcut omitted; actual lazy-body/frame tests must validate this.
No benchmark or all-provider equivalence is asserted. If general leaf formation
cannot establish the original full argv/context, keep its existing typed source
rung/Native admission requirement rather than force a lookup or default frame.

The dropped additive ABI status follows the existing Native Invoke sequence
(native_emit.rs:1642). NativeEmitter::call (line 775) checks
host_refusal_pending before returning to the emitter and branches to Label::Exit,
which releases owned locals and call-frame storage. A Host refusal leaves the
out triple unread. Only the reached non-Host path drops the transport status and
calls adopt_frame_completion (line 1256), which moves code/result/options into
owned completion locals. The new descriptor is not exempted by the transport
allowlist, and its new import index is not in NativeImports' transport-only list.
Thus its Host path retains the actual existing check, rather than treating a
Host status as a Tcl code. Runtime CodegenOperation::settle (line 438) captures
while activation owns Guest state, drops activation, validates currency and then
writes out. This is source inspection, not a run of the proposed ABI.

CODEGEN_ABI_VERSION is explicitly derived at tcl-runtime-api/codegen_abi.rs:600
from every ALL descriptor and layout. Adding this descriptor changes the hash.
ArtefactIdentityManifest::from_environment retains that version (manifest.rs:252);
its comparison at manifest.rs:277 records AbiVersion mismatch on strict inequality.
Runtime tcl_runtime_identity (codegen_abi.rs:580) reports the actual fingerprint;
existing wasm_real_link mismatched-manifest test near line 728 must remain.
Old import signatures/exports are retained, but artefacts bearing the old
fingerprint are refused by the existing identity gate. This proposal does not
waive the gate or claim acceptance of old fingerprinted modules. Compiler and
Runtime must be rebuilt together for new artefacts. Existing derived-table tests
verify added descriptors perturb the fingerprint; no manual version number or
compatibility table is proposed. No generated ABI report was located in the
bounded docs/generated + xtask search. If packaging generates one outside that
surface, regenerate it from the actual new descriptor, not a hand-written hash.

Runtime named-holder and missing-holder software control sources are in
runtime-controls.rs; they use actual Runtime native
core construction and current live NsIds, with no rendered holder reparse.
The colon-holder ordinary-name controls share scope with the C86/C9 original
publication proof; a Jim run needs its own selected original key/caller controls.


Scratch test-update diff: test-updates.patch.
It updates the original greet operand assertion, appends the fixed C86/C9
controls, requires zero new-definition calls in BOTH original compiler-admission
refusal cases, includes the lazy-body source-only allowance, adjusts the three
native-definition counters and real-link import check, and checks the additive
descriptor/refusal/fingerprint. The general analysed-add case now expects three
once-evaluated invocations (proc, nested add, puts), and zero ProcRegister calls.
The opaque rebound test already expects four generic invocations and stays so.
Other explicit generic counters in that file cover sources without a literal
procedure definition and are not mechanically incremented.

The seven implementation owners are prepared separately from those five existing
test files. Runtime controls are a separate proposed source, not a executed suite
or a complete replacement for existing callback/admission coverage. The shipping
wasm-codegen contract's old name/text definition call description near line 326
must be rewritten to describe original evaluated argv, internal entry keys,
original handler ownership and retained legacy imports. Existing native proc
comments in interp.rs should likewise reference both definition imports.
No hand-maintained ABI export report was found in the bounded current surfaces;
the actual Runtime build/export and manifest checks are still required.
