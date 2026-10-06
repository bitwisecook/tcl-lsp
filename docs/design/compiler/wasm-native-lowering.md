# WASM native lowering

The WASM compiler projects retained executable semantics into WebAssembly
through one pipeline. Optional native lowering uses machine values and explicit
Tcl framing where the available proof permits it. Unsupported instructions and
missing admission evidence retain runtime execution with a typed decline.

The public entry is `compile_wasm` in
`rust/tcl-compiler/src/codegen/wasm/pipeline.rs`. The runtime implementations
live in `runtime/rust`; intrinsic identities, target-neutral operations and ABI
imports have shared owners in `tcl-registry` and `tcl-runtime-api`.

## 1. Admission and semantic authority

A registry operation identifies a candidate implementation. It does not prove
that a call can bypass command lookup, callbacks, conversion, observers or
native compilation. Executable specialisation additionally requires the
original invocation's retained binding, operand layout and purpose-specific
proofs. A semantic type hint cannot authenticate an object's current cache or
its physical issuer.

Native compilation admission is retained on executable functions and NLIR.
`native_lowering::lower_function` declines a function whose admission requires
an actual native provider. The WASM pipeline separately records unavailable
original source and host-compilation requirements. Successful source analysis
alone does not discharge these obligations.

Source grammar, logical handler policy, physical object materialisation,
compiler-local layout and interpreter ownership are separate inputs. A logical
F5 provider does not authenticate a C compiler or its objects. Missing native
identity, frame or object evidence remains unavailable.

## 2. Compilation options and validation

### 2.1 Input modes

`WasmCompileOptions::hosted()` and `runtime_linked()` use the runtime's reserved
data window. `standalone(initialise_library)` creates a WASI entry point;
optional library bootstrap is represented as a packaging constraint.
`for_sealed_program()` is an explicit complete-interpreter-lifetime premise,
not a general host preference.

`native_tier()` enables `NativeLowering`, `RepresentationInference`,
`TraceBarrierElision` and `CellDemotion`. These passes are optional and can also
be selected independently. Default options do not enable them.
`LegacyAnalysisSpecialisation` independently controls analysis-derived leaf
specialisation. `for_eval_only_test_host()` restricts emission to the ABI its
isolated host implements and records a semantic decline.

Backend selection can choose a proved integer-add plan, generic prebuilt argv,
a guarded intrinsic, native lowering or general structured emission. These are
input plans for the same emitter. Failure to select a specialised plan retains
the original runtime operation.

### 2.2 Validation surfaces

`rust/tcl-compiler/tests/wasm_tiers.rs` compiles deterministic
`samples/wasm/` programs with default, analysis and native options. Real-link
tests compare stdout with committed Tcl oracle outputs. `TCL_REQUIRE_WASM_LINK=1`
makes missing runtime-link prerequisites a failure.

`samples/wasm/budgets.tsv` records calls to source evaluation, expression
execution and argv invocation, together with native numeric instructions.
Budget checks describe emitted framing; they do not establish semantic
correctness. Real runtime tests additionally cover completion ownership,
original source admission, guarded fallback and reached host refusal. A known
divergence table is checked in both directions so a newly passing case cannot
silently retain an obsolete expected divergence.

### 2.3 Procedure definitions

A compiled definition must retain the parameters and body Tcl actually
installed. `native_lowering::lower::definition_words_are_written_out` checks
that a definition registered by an emitted tier wrote the recorded definition
words. A substituted definition that cannot meet this contract uses runtime
invocation, where its operands are evaluated at the original call site.

Keeping a body for analysis does not authorise registering that body's written
substitution as executable source. Runtime introspection and declined native
entry must see the installed body and its proper frame.

## 3. Native lowered IR

### 3.1 Projection

`native_lowering::lower_function` consumes executable IR, validates it and
returns a `NativeFunction` with a `FunctionReport`, or a `FunctionDecline`.
Unmodelled executable instructions decline the function. Individual word or
statement failures retain `EvalSource` with a `NativeLoweringDecline`, such as
argument expansion, unsupported substitution or a computed cell name.

### 3.2 Runtime dispatch

Selected intrinsics and completions require dispatch proof. Otherwise NLIR can
invoke original prebuilt argv through runtime lookup. Source evaluation remains
available when the original operand evaluation cannot be projected safely.
The emitter consumes these decisions; it does not reconstruct command-specific
semantics from command names.

### 3.3 Operations and completion

NLIR uses SSA values, blocks, explicit cell operations, arithmetic, invocation
and completion operations. It mirrors executable block and completion
identities. A failing operation abandons the remainder of its statement and
passes the original completion to the block terminator.

`EntryProtocol::Script` enters the compiled script in the host's existing
frame. `EntryProtocol::ProcEntry` uses the procedure frame and compiled
activation already established by runtime native-procedure dispatch. Adding a
second procedure frame would change `namespace current`, `upvar` and
`info level`.

### 3.4 Values and cells

The representation lattice distinguishes bounded native integers, native
doubles, native booleans, boxed objects and unknown values. A boxed type shape
is only a fast-path hint. Integer interval proofs must establish operation
preconditions and absence of overflow; unchecked wrapping is not Tcl
arithmetic. Mixed integer/double comparisons require exact conversion, and
unproved numeric cases retain runtime arithmetic.

Cell access remains explicit. Shadow values and cell-demotion decisions are
recorded separately from variable names and inferred types. Address, lifetime,
alias and observer requirements continue to apply to any substituted storage.
A slot decision in a report must not be interpreted as proof that every
procedure variable is emitted as an indexed slot.

### 3.5 Framing decisions

`TraceLedger` records whether variable tracing requires a barrier. Unknown
trace targets keep the barrier; `incr` can use a runtime trace-bit guard for an
unknown ledger. Disabled elision passes retain framing with an explicit
reason. Removing a trace barrier supplies no independent command, frame or
object-conversion proof.

Per-statement reports record native operations, intrinsics, definitions,
generic invocation and source declines, along with representation and cell
access decisions. These reports explain selected emission without turning
unavailable evidence into an assertion of safety.

## 4. Runtime ABI

`CodegenAbiImportId` in `tcl-runtime-api` describes the imports implemented by
`runtime/rust/src/codegen_abi.rs` and `codegen_native.rs`. Native emission uses
this shared descriptor table.

Invocation ABI status and Tcl completion are distinct. An invocation that
writes `TclCompletionAbi` returns ABI success even when the Tcl completion is
an error. The completion owns its result and return-options objects. A guarded
intrinsic decline writes no completion and permits the exact generic argv
fallback.

A reached host refusal also writes no Tcl completion. Generated code checks
this separate state before adopting output or continuing through Tcl
`catch`, `try`, cleanup or read-modify-write recovery. It must not retry an
operation after its observable effects have already occurred.

Value-getter imports use the selected runtime conversion contract and leave
output storage untouched on failure. Cell, slot, activation, procedure-entry
and command-attribution imports preserve runtime ownership and Tcl framing.
Their presence in the ABI does not by itself make an optimisation admissible.

## 5. Intrinsic surface and limitations

`IntrinsicId::ALL` contains 34 stable operation identities, covering list and
dictionary operations, strings, regular expressions, selected introspection,
arrays, concatenation, channel output, procedure no-op and namespace current,
origin and code. The identities describe semantic operations, not an
unconditional native implementation for every engine, operand or invocation.
Release-specific protocol and original binding checks remain necessary.

Dynamic scripts, custom object effects, mutable command tables, tracing,
aliased cells and unsupported executable instructions can require runtime
execution. Native lowering does not close those requirements using assistance
metadata or a nominal class, variable or command name. A specialised result
must preserve observable object sharing, conversion, completion and compiler
entry effects as well as returned bytes.

## Related

- [WASM code generation](wasm-codegen.md)
- [Semantic AOT optimisation](semantic-aot-optimisation.md)
- [Common semantic compiler](common-semantic-compiler.md)
- [Dispatch stability proof](dispatch-stability-proof.md)
- [Evaluated Tcl semantics](evaluated-tcl-semantics.md)
- [WASM target surfaces](wasm-target-surfaces.md)
- [Sample programs](../../../samples/wasm/README.md)
