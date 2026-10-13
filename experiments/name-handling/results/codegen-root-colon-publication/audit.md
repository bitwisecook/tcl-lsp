# Procedure publication and WASM entry keys

A constructed reporting key is a different input from an original procedure
name. Root simple name `:audit` renders as `:::audit`; using that display text
as fresh written publication input selects simple name `audit`. The
[original control](question.md) retains the C5/Jim public observations that
establish this distinction.

## Software path

The frontend's [lowering owner](../../../../rust/tcl-compiler/src/lowering/mod.rs)
retains `Procedure.name`, `Procedure.qualified_name`, the definition span and
original structured words. The qualified key supplies internal procedure/body
selection; it cannot replace the original publication operand.

[Executable IR](../../../../rust/tcl-compiler/src/executable_ir.rs) separately
retains source provenance and the already evaluated argv vector. Native
[definition lowering](../../../../rust/tcl-compiler/src/native_lowering/lower.rs)
uses that vector in `NativeOp::DefineProc { entry_key, argv }`, subject to its
original declaration/span, exact argument correspondence, literal-word,
registry operation, dispatch and compilation-admission checks. Unproved or
substituted definitions retain existing generic invocation or typed refusal.

The [native emitter](../../../../rust/tcl-compiler/src/codegen/wasm/native_emit.rs)
uses only `entry_key` for compiled-table selection and sends the evaluated
objects to `tcl_codegen_proc_define_original_argv`. Its existing borrowed argv
frame and completion owner retain cleanup and Host refusal checks. The
[general emitter](../../../../rust/tcl-compiler/src/codegen/wasm/backend.rs)
uses ordinary dispatch with the complete evaluated original argv for procedure
declarations.

The [Runtime ABI](../../../../runtime/rust/src/codegen_abi.rs) borrows those
objects and enters the same [original procedure handler](../../../../runtime/rust/src/cmd_proc.rs)
as stock `proc`, with a separately proven optional compiled entry. The handler
owns current-namespace selection, original C/Jim name publication, conversion
order, body/formal handling and result semantics. It grants no new Native
identity merely because input spelling matches source metadata.

The direct ABI requires an independently proved live stock definer in the
current invocation context. Its retained command head is not dispatched;
replacement or shadowing therefore requires the generic route. The optional
compiled entry, original-name ownership and selected-definer authority remain
separate obligations.

Legacy byte-definition ABI exports remain available. The descriptor/layout
fingerprint changes with the import table, and actual Runtime/module identity
must agree before linking. See the [current WASM contract](../../../../docs/design/compiler/wasm-codegen.md#binding-a-compiled-body-to-its-procedure)
and [original argument contract](../../../../docs/design/runtime/original-command-arguments.md#compiled-procedure-definitions).

## Evidence and limits

The [original CLI results](question.md) establish the bounded publication/call
semantics. [Evaluated argument probes](evaluated-argv-oracles/results.json)
separately record counted name/body construction, inert body text, missing
variables, replaced definers and provider-specific missing-holder observations.
The original Jim missing-holder orchestration cannot complete because
`namespace exists` is unavailable there; its failure stays recorded.

[Runtime validation](runtime-validation-01/README.md) executes six actual Runtime
software controls, including object borrowing, namespace holders, missing-holder
rejection, replacement and Host refusal. Its compiled callable controls use a
Rust software ABI function; they establish no original Native compiler authority
or emitted-WASM/table result. Compiler admission, genuine linked execution and
F5 acceptance require their own measurements.

Procedure maps and entry tables still use reporting FQNs in parts of the
compiler. Distinct holder/simple-name geometries can render the same string.
Namespace restrictions and conservative fallbacks remain in force; this
transport contract does not establish complete structured identity coverage.
