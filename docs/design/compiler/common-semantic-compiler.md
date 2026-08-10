# Common semantic compiler and target-family lowering

> **Status:** implementation contract. The migration is additive: existing
> bytecode, WASM, and BPF output remains the behavioural baseline until each
> consumer has moved onto the common facts described here.

## Purpose

The compiler must understand Tcl once. Command resolution, substitutions,
completion, traces, variable cells, mutable interpreter state, and optimisation
legality belong above code generation so the LSP, TclVM, WASM, eBPF, and future
targets consume the same facts.

This does not require one universal low-level IR. After common semantic
analysis, each target family lowers eligible regions into an execution-model
specific IR:

```text
CST and command tokens
    -> common semantic IR
    -> executable semantic CFG
    -> value SSA + cell SSA + world/effect SSA
    -> common analyses and semantic transforms
    -> target eligibility and region selection
       -> runtime-capable IR (TclVM, WASM, native CPUs)
       -> verifier IR (eBPF)
       -> parallel kernel IR (GPU)
       -> spatial/dataflow IR (FPGA)
```

The common compiler proves meaning and legality. Target-family lowerers choose
representations and execution mechanisms. Machine emitters only encode an
already-selected target plan.

## Current implementation boundary

This document is the destination contract, not a claim that every layer in the
diagram is connected. The current additive implementation has these deliberate
limits:

- the function-owned semantic sidecar builds an executable graph from retained
  source-faithful IR and attaches registry resolution plus executable
  world-state SSA;
- exact calls and barriers become staged word evaluation, argv construction,
  and generic invocation. A sequence may contain more than one statement;
- already-lowered assignments, increments, expressions, and returns retain a
  registry-owned `LoweringHookId`, their exact `Statement`, `NodeId`, and source
  provenance as `ExecuteLowered` operations. Their precise completion and
  effects have not yet been projected, so common analyses treat them
  conservatively;
- `Block`, `UpFrame`, `If`, `For`, `While`, `Foreach`, `Catch`, `Try`, and
  `Switch` currently become typed `ExecuteOpaqueRegion` operations. This
  preserves the enclosing sequence and any structural descriptor that survived
  lowering, but it does not claim an exact CFG for the body. Each such region is
  an all-world use/clobber barrier;
- the sidecar complements the existing scalar SSA and optional memory SSA. It
  does not yet replace every legacy CFG, SSA, SCCP, GVN, lattice, optimiser, or
  LSP consumer; and
- backend contracts, representation types, and proof tokens are legality
  scaffolding. They do not by themselves enable specialisation.

No framing removal, single-representation variable lowering, or other AOT
optimisation is enabled by this work. The current WASM consumer declines the
lowered and opaque operations above, and accepts only its bounded generic
invocation shape. The current eBPF semantic bridge is a conservative auditor;
the established BPF-Tcl frontend and emitter do not consume its eligibility
record.

Safe and child interpreters, including the Safe Base, remain runtime concerns.
The common state model reserves interpreter-policy and visibility domains, and
generic invocation is required to use normal runtime dispatch. The compiler
does not yet prove a safe-interpreter domain closed, compile the Safe Base as an
AOT region, or provide a Safe-Base-specific backend plan. Runtime support and
runtime tests must therefore not be presented as end-to-end AOT coverage.

## Registry boundaries

### Command registry

`CommandRegistry` is the sole owner of command-level knowledge. A command spec
and its resolved subcommand or form provide:

- argument grammar and roles;
- a target-neutral semantic operation identifier;
- completion and suspension behaviour;
- state reads, writes, callbacks, and re-entrancy;
- constant, type, taint, and effect transfer descriptors;
- variable-cell, frame, command-table, namespace, trace, and TclOO effects.

Compiler and LSP consumers must not select behaviour by comparing a command
name. When an existing descriptor cannot express a command's semantics, extend
the registry descriptor rather than adding a consumer-local command branch.

### Common semantic registry

The common compiler may have a registry of implementations keyed by typed
semantic operation identifiers. It owns target-neutral lowering and abstract
transfer functions, not the association between a Tcl command spelling and an
operation. That association remains `CommandRegistry` data.

### Backend registries

Each backend registry maps a common operation or target-family operation to an
immutable target plan. Selection is non-mutating:

```text
Selected(plan)
Declined(reason)
```

Emission starts only after selection succeeds. A backend may impose stronger
legality restrictions than the command semantics, but it must not weaken the
registry-declared effects or completion behaviour.

## Common semantic identity

Every semantic node has a stable function-local `NodeId`, a `SourceSite`, and
rewrite provenance. Synthesised nodes retain the identity of the source node
that caused them. Absolute source offsets are presentation data and are not
part of a position-independent analysis cache key.

Tcl words are represented once as structured `WordExpr` values. The structure
must preserve literal, template, expansion, variable, command-substitution,
and backslash-substitution components in evaluation order. Backends must not
independently reparse argument strings.

A `ResolvedInvocation` retains both the original words and the registry's
semantic resolution:

- the source spelling and evaluated command-head expression;
- command, subcommand, and form resolution kind;
- target-neutral semantic operation;
- registry-derived effects and transfer descriptors;
- any static command-identity proof and its mutable-state dependencies;
- the original invocation needed by a generic slow path.

`canonical_command: String` alone is not an executable identity proof. Runtime
commands, imports, aliases, ensembles, object commands, and renamed procedures
require identity and interpreter-domain information.

## Ordered invocation semantics

The executable graph preserves this order:

1. evaluate each word and its substitutions;
2. parse and expand `{*}` words;
3. resolve the resulting command head in the current interpreter and
   namespace;
4. test any command-identity, policy, or trace guard;
5. invoke the selected operation;
6. propagate completion code, result, and return options.

Substitution may mutate command bindings or trace state. Therefore an identity
guard occurs after argv construction. A guard's slow path reuses the already
constructed argv and must not evaluate the words a second time.

The fallback ladder is:

```text
native operation
    -> runtime intrinsic
    -> generic argv invocation
    -> eval of a genuinely dynamic script
```

Generic invocation, not source reparsing, is the normal compatibility path.

## Shared runtime contract

There is one Tcl runtime semantic surface. It is not duplicated per compiler
backend. The Rust runtime and `tcl-runtime-api` define operations such as
command dispatch, namespace lookup, frame access, variable access, traces, and
the complete `(code, result, return-options)` result. A target chooses only how
it transports those operations:

- an in-process TclVM or native backend calls the Rust traits directly;
- WASM imports a narrow C-compatible handle ABI over the same implementation;
- a future native object can use an ordinary platform calling convention over
  the same semantic entry points;
- a closed eBPF, GPU, or FPGA region is legal only when common analysis proves
  that no runtime operation is required inside that region.

The generic invocation boundary accepts the full, already-evaluated argv,
including the command head. The runtime then performs its ordinary namespace,
alias, ensemble, `unknown`, safe-interpreter, and TclOO dispatch. It does not
parse source or repeat substitution. The boundary returns the exact completion
triple, including custom integer completion codes and owned return options.
Concrete ABIs may use pointers, handles, or explicit output storage, but those
are transport details below the shared semantic contract.

The WASM transport allocates an individual call frame from the runtime for each
prebuilt argv invocation. The frame holds only transient argv handles and the
completion output triple; immutable literal bytes may live in a separately
reserved data region, but no call frame may use fixed linear-memory scratch.
Before it releases the private output triple and frame, a generated callee
retains the result and return-options handles for its completion successor.
Consequently a nested command callback can re-enter generated code without
overwriting an outer argv or losing an arbitrary Tcl completion code.

This separation also keeps runtime compilation portable. The same runtime
source can be compiled for the host, WASM, or another supported target without
moving Tcl semantics into an emitter. Backend intrinsics are optional fast
paths into that runtime, not alternate implementations of Tcl commands.

## Shared Tcl primitives

Pure Tcl algorithms belong in common crates below both the interpreter and
compiler-facing runtime adapters. Tcl list scanning/quoting, numeric and
boolean conversion, string indexing and case rules, option/argument grammar,
completion option construction, and object type conversions must not acquire
independent backend copies. The layering is:

1. byte/string/list/argument algorithms with no interpreter or backend state;
2. runtime object adapters that apply Tcl dual-representation, ownership, and
   error-result rules;
3. registry descriptors that declare which grammar or semantic operation a
   command uses;
4. common compiler transfer functions keyed by semantic operation;
5. target plans and emitters.

When an implementation cannot yet share a pure primitive, its target registry
must select generic invocation or a runtime intrinsic. An emitter must not
reimplement Tcl list parsing, option matching, or string semantics as a
command-specific shortcut.

## Common state model

The common executable graph carries three related forms of SSA.

### Value SSA

Value SSA versions Tcl values assigned to symbols and continues to drive
def-use, SCCP, type, shape, interval, rendered-property, and taint analysis.

### Cell SSA

A `PlaceId` or `CellId` identifies observable variable storage independently
of a value SSA version. Cell regions cover locals, globals, namespace
variables, upvar targets, instance variables, array elements, and dynamic
wildcards. Variable traces and aliases attach to cells, not to scalar SSA
versions.

### World/effect SSA

Mutable interpreter state is versioned through typed regions. Initial regions
include:

- command bindings;
- namespace lookup and namespace-specific unknown handlers;
- execution, variable, and command traces;
- TclOO object, class, method, filter, and mixin dispatch;
- interpreter visibility, hidden-command, safe-interpreter, and limit policy;
- package state and host capabilities;
- variable-store wildcard state where cell precision is unavailable.

Operations declare an `EffectFootprint` containing reads, writes, re-entrant
callbacks, and clobbers. Unknown invocation widens to the appropriate wildcard
regions. State tokens are identities used by GVN and motion legality; they are
not Tcl values and do not enter the SCCP value lattice.

Registry-owned state-transition facts complement effect footprints. Effects
say which state may be observed or changed; transitions describe known
identity changes such as defining, moving, deleting, or aliasing a command, or
linking a local variable cell to a global, namespace, or caller-frame cell.
Literal operands retain precise identities. Dynamic operands and positional
expansion produce typed domain widenings. Missing transition metadata is an
unknown invocation, not an empty transition set; only an explicit empty
descriptor proves that no tracked identity changes.

Each transition also declares its completion-edge commit policy. An
`OK`-only transition is transferred only along normal completion. A command
that can expose pair-wise or re-entrant partial mutation before abrupt
completion requires the abrupt edge to join the unchanged and transitioned
states. This prevents an error edge from accidentally proving that a
definition happened, while preserving soundness for partial `upvar`, alias,
or trace-visible changes.

Closed targets may prove domains absent. BPF-Tcl, for example, has sealed
bindings, no traces, no generic invocation, no Tcl frame, and no heap Tcl
objects. Its unused world-state domains erase before BPF lowering.

### Shared versioning substrate

Cell SSA and world SSA share graph mechanics, not a location enum. A generic
versioned-state substrate provides `Use`, `Def`, `Phi`, and `Clobber`, reaching
versions, dominance placement, and stable node attachment. Its location type
supplies overlap, wildcard, and display behaviour.

Variable cells use the compiler's structured `Place` identity and its
conservative overlap relation. World locations use typed semantic regions and
normally compare by region identity; a domain wildcard overlaps every location
in that domain. Keeping these location types distinct prevents command-table
mutation from masquerading as a variable write while avoiding two independent
SSA implementations.

The existing `MemoryLocation` representation migrates towards `Place` through
an adapter. New command discovery must come from resolved registry effects,
not from adding more command recognisers to memory SSA.

## Completion and callbacks

Every invocation semantically produces a completion code, result value, and
return-options value. The completion lattice supports bounded exact sets and
an unknown top; Tcl permits custom integer completion codes.

The completed common graph must make `catch`, `try`, `finally`, loop control,
procedure return, trace callbacks, and fallback invocation real executable CFG
edges. Analysis-only exception reachability is not sufficient for executable
optimisation. At present, a plain retained `return` terminates the compatibility
sequence, while the structured constructs listed above remain typed opaque
regions with conservative completion. Their internal handler, loop, and
callback edges are not yet executable-IR edges.

Trace flow distinguishes:

- variable read, write, unset, and array traces;
- execution enter, leave, enter-step, and leave-step traces;
- command rename and delete traces.

Trace add and remove require a flow-sensitive exact-or-unknown lattice. A
module-wide additive set is a conservative compatibility fact, not an AOT
absence proof. The runtime remains authoritative for callback order,
re-entrancy, result replacement, active-trace suppression, and errors.

## Shared analyses and target refinements

Common analyses own facts that improve both tooling and code generation:

| Fact | Common owner | Target-family refinement |
|---|---|---|
| command/form resolution | semantic lowering | live identity guard or sealed proof |
| constants and reachability | SCCP | target constant encoding |
| Tcl value types and shapes | type/shape propagation | physical representation |
| integer ranges and known bounds | interval analyses | verifier, SIMD, or bit-width lattice |
| variable aliasing and observability | cell SSA and escape analysis | frame, stack, map, device, or fabric storage |
| mutable semantic state | world/effect SSA | target legality and guard implementation |
| completion set | completion analysis | verdict, status buffer, or native unwinding protocol |
| suspension | interprocedural summary | state machine, runtime fallback, or rejection |
| ownership and escape obligations | common liveness/escape facts | refcount, tracing GC, registers, or transfer buffers |

Target-only facts stay below this boundary. Examples are eBPF pointer kinds
and verifier state, CPU register allocation, GPU address spaces and SIMT
divergence, and FPGA scheduling and resource binding.

## Target contract

Target capability data is structured by execution, value, control, memory,
runtime, resource, and feature models. It must not grow into an unrelated list
of booleans. A target contract describes whether a whole operation or region
is legal and why.

Runtime-capable targets may use guarded fast paths with generic invocation or
interpreter deoptimisation. Targets without a Tcl runtime, including eBPF and
accelerator kernels, reject the region or retain it on the host; they do not
place an impossible per-operation fallback inside the target program.

## Representation and materialisation

Semantic type, variable storage, and physical value representation are
separate analyses. A value known to be an integer is not automatically proven
to be an unboxed integer: Tcl dual representations, string materialisation,
sharing, mutation, callbacks, and reflective frame observation still apply.

Before a representation optimisation is enabled, the common compiler must
determine frame observability, materialisation points, boxing boundaries,
suspension safepoints, and ownership obligations. A backend can then select
physical storage such as a Tcl frame, cached slot, materialisable native slot,
register, eBPF stack location, GPU buffer, or FPGA wire/memory.

The current representation module models dual representations, sharing,
copy-on-write, boundary materialisation, and proof obligations, but those types
are not an optimisation authorisation. Generated WASM continues to use runtime
objects and runtime call frames. It does not remove Tcl framing or assume that
a variable has only a native representation.

Guard failure or an unknown call must have an explicit materialisation and
reload plan. Future deoptimisation uses a `NodeId`-keyed continuation map rather
than reconstructing state from emitted instructions.

## Pass invalidation

Executable transforms mutate IR rather than rewriting source text. The common
pass manager therefore tracks an IR revision and the analyses preserved by
each transform. A control-flow change invalidates dominators, value SSA, cell
SSA, world SSA, SCCP reachability, GVN, liveness, and any affected summaries.

Analysis cache keys include the command-registry profile/fingerprint and
semantic ABI. Runtime mutation epochs are normally emitted guards, not static
compiler cache keys. A sealed-environment snapshot may be part of a cache key
when the compilation policy explicitly requests one.

## Migration stages

The numbered stages are sequencing constraints, not a completion ledger. The
current boundary is recorded above; later entries remain design work until a
consumer is connected and differential tests cover its Tcl surfaces.

1. Add stable node identity, source provenance, and structured words behind
   compatibility adapters.
2. Add target-neutral resolved invocation and registry-derived semantic/effect
   descriptors without removing existing backend hooks.
3. Introduce structured target contracts and machine-readable decline reasons.
4. Build exact executable completion flow and generic argv invocation.
5. Promote variable places to common cell SSA and add world/effect SSA.
6. Add flow-sensitive binding, namespace, TclOO, interpreter, and trace state.
7. Replace boolean-heavy interprocedural summaries with structured effects.
8. Move SCCP, GVN, motion, DCE, taint, and escape consumers onto the common
   operation and state facts.
9. Add guarded semantic specialisation and explicit materialisation, while
   keeping optimisation disabled until differential tests prove it sound.
10. Migrate TclVM and WASM selection to backend registries, preserving emitted
    bytecode and runtime behaviour during the transition.
11. Feed resolved operations and common facts directly into BPF-Tcl's existing
    typed BPF IR; retain its verifier-specific lattice and emitter.
12. Add native CPU lowering only after the runtime ABI, completion, ownership,
    and deoptimisation contracts are stable. GPU and FPGA work begins with
    host/device region extraction, not a full Tcl interpreter on the device.

Every stage adds registry drift tests, focused compiler tests, differential
runtime tests, and LSP consumer tests. No stage adds a command-name special
case outside registry data, and no stage adds a Clippy allowance.
