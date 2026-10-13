# Procedure publication and WASM entry keys

WASM procedure registration transports `Procedure.qualified_name` as fresh
original publication input. A constructed key for root simple name `:audit`
renders as `:::audit`; the shared publication owner interprets that new input
as simple name `audit`. The [fixed original control](question.md) demonstrates
why the representation change affects Tcl semantics.

## Actual software path

The frontend's [`lower_proc`](../../../../rust/tcl-compiler/src/lowering/mod.rs#L3782)
uses `qualify_proc_name` and registry `published_procedure_key`, then retains
both `Procedure.name` and `Procedure.qualified_name` in the procedure record.
[`qualify`](../../../../rust/tcl-syntax/src/naming.rs#L1186) joins a constructed
prefix without reparsing the result.

After registry `NativeLowering::Definition` selection and the per-site proof,
[`lower_definition`](../../../../rust/tcl-compiler/src/native_lowering/lower.rs#L991)
finds the original definition by span and checks literal name/formals/body.
It copies `procedure.qualified_name` into `NativeOp::DefineProc`.
[`native_emit`](../../../../rust/tcl-compiler/src/codegen/wasm/native_emit.rs#L1682)
uses that same key for two distinct purposes: the runtime name bytes and the
compiled-entry table lookup. The general WASM
[`try_emit_direct_operation`](../../../../rust/tcl-compiler/src/codegen/wasm/backend.rs#L735)
likewise sends `proc.qualified_name` through `ProcRegister`.

[`tcl_codegen_proc_define_native`](../../../../runtime/rust/src/codegen_abi.rs#L1745)
passes the received bytes to `install_proc_original_storage`.
The actual interpreter
[`publication call`](../../../../runtime/rust/src/interp.rs#L4973) selects the
current namespace and invokes
[`procedure_publication_at`](../../../../runtime/rust/src/namespace.rs#L1894),
which delegates to the shared release-selected `command_publication_projection`.
The owner receives a constructed report instead of the original operand.
`ProcRegister` delegates to the same ABI with no compiled entry.

The `EntryTable` map and `push_entry_index` are internal compiled-body selection;
their string key alone is not the publication defect. The concrete violation is
its reuse as the runtime name operand. The bytecode
[`emit_one_proc_def`](../../../../rust/tcl-compiler/src/codegen/emitter/proc_defs.rs#L59)
uses `proc_def.name` instead, so this finding does not establish the same defect
there or establish a complete audit of sidecars and symbol maps.

## Ingress and bounded repair

The original name is available before the faulty transport. Executable
[`emit_call`](../../../../rust/tcl-compiler/src/executable_ir.rs#L2642)
retains exact structured words and separately constructs evaluated argv.
[`GenericInvoke`](../../../../rust/tcl-compiler/src/executable_ir.rs#L203) carries
the original words, effective argument correspondence and point-specific source
binding, but explicitly forbids using source provenance to rerun substitutions.
The native lowerer retains the evaluated vector in its
[`BuildArgv` handling](../../../../rust/tcl-compiler/src/native_lowering/lower.rs#L825);
`lower_invoke` already reads that vector. For the currently admitted narrow
shape, `original_words[1]` is a `Literal` or `BracedLiteral` with its actual
text and source site, and structural evaluation has already produced its value.

A bounded transport repair can retain a separate publication-name operand from
that existing evaluated argument, leaving `qualified_name` solely as the
compiled-entry key. The current pointer/length ABI can carry an independently
checked literal value without changing table indices. It must retain the
actual publication namespace/context and must not synthesize a Native object,
holder or identity from the source record. Context or value correspondence
that cannot be checked keeps generic invocation with its already-built argv.

For the general emitter, structured words still exist at
[`record_operation`](../../../../rust/tcl-compiler/src/codegen/wasm/backend.rs#L1987).
Its facts currently reduce the literal-definition proof to a span set; retaining
the checked publication operand alongside that proof avoids reconstructing it
later from the surviving FQN. Both paths should use the same admitted operand
projection, preserving the separate formals/body and dispatch obligations.
Changing `Procedure.name` alone would not cover the native opcode or its emitter.

The existing
[`a_proc_statement_lowers_to_the_definition_shape`](../../../../rust/tcl-compiler/src/native_lowering/tests.rs#L653)
checks ordinary `::greet`, so it cannot detect the root-colon transport change.
Meaningful fixed-input validation should check the admitted root `:audit`
publication operand and separate entry key, then execute the emitted module
against the actual Runtime and compare both positive and absent call outcomes
with the retained originals. A literal absolute-name control and a supplied
non-root holder should verify context handling; missing owner/value metadata
should preserve the generic invocation.

Native compilation admission and word/dispatch proofs can refuse an input
before this path. The source inspection identifies the operand conflation when
the definition path is selected; it does not establish that every source state
reaches it. No compiler test, emitted module or Runtime execution result is
claimed by these artifacts. The root-colon distinction is established by the
original public capture and the existing shared-owner assertion separately.
