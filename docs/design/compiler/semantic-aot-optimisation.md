# Semantic AOT optimisation contract

> **Status:** implementation contract with two deliberately bounded WASM
> consumers: a live-guarded boxed `string length` intrinsic and the sealed
> constant-operand `add` demonstration. Every semantic AOT pass remains
> independently disableable and off by default. The rest of this document is
> the soundness contract for widening those first slices.

## Purpose

This contract defines when common compiler facts may authorise a specialised
execution plan for TclVM, WebAssembly (WASM), or a later native backend. C Tcl
9 is the semantic oracle unless the selected Tcl version or dialect specifies a
different rule.

The governing rule is:

> A static registry resolution is a candidate semantic operation, not proof of
> the command, frame, variable cell, object, or interpreter that will be live at
> execution time.

Command-specific knowledge belongs in `CommandRegistry`. Compiler passes may
combine registry facts, control flow, static single assignment (SSA), sparse
conditional constant propagation (SCCP), type, interval, alias, escape,
completion, and world-state analyses. They must not recognise a command by
spelling or duplicate its argument grammar, effects, callbacks, or state
transitions.

## Current bounded implementation

The implementation now exercises both guarded runtime specialisation and
sealed native lowering, but neither is a general Tcl AOT mode.

| Slice | Explicit controls | What is selected | Current limit and fallback |
|---|---|---|---|
| Guarded boxed intrinsic | `GuardedIntrinsic` | One executable-IR prebuilt-argv invocation resolved by the registry as `IntrinsicId::StringLength`. The common mixed-region plan retains the registry intrinsic identity, every dispatch-dependency domain, exact completion identity, and the original argv slow path. | The WASM emitter evaluates/builds argv once, asks the live Rust runtime to prepare and re-check a per-interpreter guard token, and invokes the boxed runtime intrinsic only on success. Rename/rebinding, execution traces, unsupported live policy, intrinsic refusal, and guard invalidation use `tcl_invoke_argv` with the same argv. Other intrinsics retain typed declines. |
| Sealed native i64 add | `DirectProc`, `MaterialisableSlot`, `FrameElision`, `NativeInteger`, and `SemanticOperationSpecialisation`, plus `for_sealed_program()` | The exact four-statement demonstration: one two-required-parameter procedure, two covered constant `set` actuals, and one registry-resolved channel-write boundary containing the direct call. Common proofs cover the procedure binding and body operations, caller/formal SSA identities, integer types and exact ranges, frame privacy, top-level statement coverage, and the boxed output boundary. WASM emits an exported `(i64, i64) -> i64` function using `i64.add`; only the result is boxed as a Tcl wide integer at the output boundary. | Selection requires overflow-impossible exact i64 operands, no extra top-level statement, no relevant mutation or trace, non-standalone packaging, and sealed-program policy. Any missing premise declines before emission to the existing generic/general path. This slice has no mid-function deoptimisation, checked-overflow branch, general materialisation protocol, default arguments, `args`, namespace-relative procedure dispatch, or TclOO support. |

### Tcl 8 supplementary-character boundary

The guarded `StringLength` slice must not be read as authorisation for general
native Tcl string operations. C Tcl 8 stores characters as 16-bit
`Tcl_UniChar` units. A supplementary-plane character therefore occupies two
units, and `string index` or `string range` can produce a Tcl value containing
one isolated surrogate. Rust `String` cannot represent that value. Counting a
Rust string's UTF-16 encoding is enough to implement Tcl 8 `string length`, but
it is not a representation from which exact Tcl 8 index, range, first, or last
semantics can be reconstructed.

The executable C Tcl oracle for the literal string `A😀B` is:

| Operation | Tcl 8.6 | Tcl 9.0 |
|---|---|---|
| `string length` | `4` UTF-16 units | `3` Unicode scalar values |
| `string index` at 1 and 2 | separate high- and low-surrogate Tcl values | the emoji, then `B` |
| `string range` 1 2 | the complete emoji | the emoji followed by `B` |
| `string first B` | `3` | `2` |

Until the shared Tcl value layer can retain unpaired UTF-16 surrogates (for
example through an exact UTF-16, WTF-8, or equivalent lossless internal
representation) and defines conversion and shimmering at every boundary, Tcl
8 string index/range/search specialisation must decline to generic runtime
dispatch. The representation belongs below both TclVM and WASM code generation;
an emitter or command-specific compiler branch must not approximate it with
Unicode scalar iteration or replacement characters.

#### Current support status: unsupported, not approximated

Only `string length` is versioned today. Every character-*indexed* string
operation — `index`, `range`, `first`, `last`, `replace`, `insert`, `reverse`,
`wordstart`/`wordend`, and the ranged case converters — addresses Unicode
scalars in both runtimes and in the compile-time folds, whichever dialect is
selected.

The consequence is deliberate and stated rather than hidden: on a Tcl 8
dialect, a string holding a supplementary character has a `string length` that
does not agree with the indices those operations use. That case is **not
supported**. It is also not approximated — addressing UTF-16 code units and
decoding a split surrogate pair to U+FFFD was implemented and then reverted,
because a replacement character is a wrong answer that looks like a right one,
and the oracle above is what it would have to meet.

Strings outside the supplementary planes are unaffected: both models agree on
every index, so ordinary Tcl 8 scripts see exact behaviour. Closing the gap
means the lossless value representation described above, below both runtimes,
as its own change.

`WasmCodegenPlan::GenericInvoke` remains the top-level record for the guarded
intrinsic because the operation's exact slow path is still generic argv
dispatch; `regions[].selectedKind` records `guarded-intrinsic`. The native
demonstration has the distinct top-level `native-i64-add` plan.

These consumers do not enable themselves. `SemanticOptimisationConfig::new()`
and `WasmCompileOptions` defaults contain no semantic AOT pass. Generic argv
dispatch and general structured lowering remain available with that empty
configuration.

## Default-off and independent controls

Every new code-changing semantic AOT pass is disabled by default until its
oracle and differential matrix is complete. Analysis may run while a transform
is disabled, because facts are also useful to diagnostics and source
optimisations. Producing facts must not implicitly select a fast path.

The current control surface is `SemanticOptimisationPassId`:

| Control | Meaning |
|---|---|
| `LegacyAnalysisSpecialisation` | retain the pre-common-proof structured-WASM compatibility tier behind an explicit opt-in |
| `GuardedIntrinsic` | select the bounded boxed intrinsic plus its live guard and exact argv fallback |
| `CachedBoxedSlot` | reserve authorisation for a future cached boxed Tcl-object slot; no emitter consumes it today |
| `MaterialisableSlot` | authorise common materialisable-slot evidence; currently consumed only as one premise of the sealed native add |
| `DirectProc` | authorise common direct-procedure evidence; currently consumed only by the sealed native add |
| `NativeInteger` | authorise native-integer proof; currently consumed only by the exact i64 addition |
| `FrameElision` | authorise common frame-elision evidence; currently consumed only for the exact add procedure |
| `SemanticOperationSpecialisation` | retain registry-resolved operation and boundary proofs; currently required by the sealed native add |

A later transform that is not independently ablated by one of these controls
must add its own pass identifier. In particular, general single-
representation storage, deoptimising materialisation, guarded native
procedures, and interprocedural region rewriting are not silently authorised
by the existing bits.

Each control must be independently disableable. An aggregate experimental
profile may enable several controls, but it is only shorthand for the explicit
set. Packaging mode, an editor optimisation profile, or availability of an
analysis result must never enable a semantic AOT transform implicitly.

The established O100–O130 source-optimisation profiles are a separate user
surface. Their default editor profile remains governed by the generated
optimisation catalogue. New backend transforms must not silently reuse those
profile defaults.

A complete semantic-AOT implementation records, per region:

- which pass was considered;
- whether it was disabled, selected, or declined;
- the typed proof or decline reason;
- the guards and materialisation obligations selected; and
- the exact slow path.

This record is required for Explorer output, ablation tests, and bug reports.
The current `MixedRegionPlan` satisfies it for guarded intrinsic candidates,
including `pass-disabled` and typed proof declines. Common direct/slot/native
analyses also retain typed decisions, and a selected native add is serialised,
but a failed native-add composition currently falls through to the ordinary
top-level generic/general plan without serialising every rejected native
premise. That observability gap must close before widening native selection.

## Plan shape and mixed lowering

The target design treats specialisation as a region decision, not a
whole-module backend mode. `MixedRegionPlan` already represents guarded
runtime intrinsics, generic prebuilt-argv invocation, retained lowered
operations, and opaque compatibility regions. It does not yet contain a native
region variant: the first native-add consumer is a separate top-level
selection that deliberately requires exact closed coverage of its whole
four-statement script. The intended progression is:

```text
native operation
    -> runtime intrinsic
    -> generic invocation with an already-built argv
    -> source evaluation for a genuinely dynamic or opaque script
```

A future mixed native plan must have explicit conversion edges. A native value
crossing into a Tcl region is materialised according to its representation
plan. A Tcl value crossing into a native region is converted with the selected
version and dialect's conversion rules. Conversion failure follows the
ordinary Tcl error path; it does not trap or manufacture a backend-specific
diagnostic. The current guarded intrinsic stays boxed, and the sealed add has
only its one proved output-boxing boundary, so neither implements that general
protocol.

Selection completes before emission begins. An emitter receives an immutable
plan and may decline only for a target encoding limitation. It may not invent a
weaker proof, recognise a command spelling, or partially emit a fast path before
choosing its fallback.

## Required common proofs

No single lattice authorises AOT specialisation. The selector consumes a proof
bundle whose missing or overdefined member causes a typed decline.

### Dispatch identity

Direct or intrinsic dispatch requires either a closed-world proof or live
guards covering every `InvocationFacts.dispatch_dependencies` domain. A proof
identifies:

- the interpreter, including a non-reused generation;
- the resolved command object or binding identity, not only its name;
- the current namespace and namespace lookup context;
- imports, exports, namespace path, and namespace-specific `unknown` state;
- command, execution, and active step-trace state;
- safe-interpreter policy, hidden commands, and child-interpreter visibility;
- TclOO receiver and dispatch-chain state where applicable; and
- the selected dialect profile, Tcl runtime version, registry fingerprint, and
  semantic ABI version.

An epoch without an interpreter and subject identity is insufficient. Epoch
wrap must not permit an old compiled plan to match new state. Implementations
must use a non-reused generation, invalidate all compiled dependants before
wrap, or make wrap unreachable by construction.

Word evaluation precedes the dispatch guard because substitutions and `{*}`
expansion can mutate bindings, namespaces, traces, interpreters, or objects.
The guard therefore runs after argv construction and immediately before the
specialised operation. On guard failure the slow path reuses that argv; it must
not evaluate any word twice.

### Trace and re-entrancy state

Trace proof is not one Boolean. It distinguishes:

- variable traces attached to a resolved variable cell;
- command rename and delete traces attached to a command object;
- execution `enter` and `leave` traces;
- registered `enterstep` and `leavestep` traces; and
- step traces already active because an enclosing traced command is running.

Variable traces observe reads, writes, and unsets, can replace state, and can
return abrupt completion. Command and execution traces observe command
identity, invocation text, result, and completion order. An active step trace
requires each Tcl command boundary in the compiled region to remain visible.
The legal choices are a trace-visible plan with exact callbacks or deoptimising
the whole affected activation before skipping any boundary.

Trace callbacks are re-entrant. Before a possible callback, compiled state must
be runtime-observable and ownership-balanced. After it returns, cached state is
invalidated or reloaded according to the callback's effect and completion.

### Frame observability

Frame elimination is not a Boolean. A plan records the least representation
that preserves every observer:

| Frame plan | Required state |
|---|---|
| full | named variable cells, metadata, caller links, namespace, and command context |
| cells and metadata | observable names/cells and introspection without ordinary interpreter execution |
| metadata only | call level, procedure, namespace, and error-stack identity |
| materialisable | native state plus an exact recipe and deoptimisation point |
| absent | no reachable observer, alias, trace, suspension, or fallback requires a frame |

The proof includes `upvar`, `uplevel`, `global`, `variable`, `namespace upvar`,
TclOO variable linking, dynamic `eval`/`subst`/`source`, frame-inspecting `info`
forms, execution traces, error-stack construction, and coroutine suspension.
Var-escape analysis supplies one component; `Local` does not prove that frame
metadata, dispatch, completion, or representation is unobservable.

Procedure formal parameters retain Tcl's list grammar and binding order.
Required parameters, `{name default}` parameters, a final `args` parameter,
malformed parameter lists, wrong-argument errors, and default-value string
representations must match the selected C Tcl oracle. A native entry point is
legal only after the common formal-parameter binder proves the exact form.

### Variable cells and materialisation

Value SSA identifies values. Cell or place SSA identifies Tcl storage that can
be observed by name, aliased, or traced. A native variable plan therefore
records both its authoritative storage and its materialisation recipe.

The supported storage states are:

- frame-resident;
- native cache backed by an authoritative Tcl cell;
- native authoritative with a materialisable Tcl cell; and
- native-only within a closed region.

At an unknown, generic, re-entrant, traced, suspending, or deoptimising
boundary, a plan performs the required ordered protocol:

1. materialise current values and named cells;
2. publish frame and namespace metadata;
3. retain owned Tcl objects needed by either path;
4. invoke or suspend;
5. propagate the complete Tcl completion;
6. invalidate native caches that the boundary could change;
7. reload values on the continuing edge; and
8. release each owned value exactly once.

Top-level variables, namespace variables, and registered procedures normally
outlive a compiled `::top` call. They cannot disappear merely because no read
is present in the compilation unit. A whole-program/sealed-interpreter policy
may prove otherwise; hosted compilation must materialise externally observable
state before returning.

### Tcl object representation and sharing

C Tcl values can carry a string representation and an internal representation
simultaneously. Converting to an integer does not normally discard an existing
string representation. Mutation invalidates the appropriate cached
representation, and shared mutable values use copy-on-write.

A single-representation plan proves all of:

- no observer requires the omitted representation before it can be rebuilt;
- rebuilding uses the selected Tcl version's canonical rules;
- original spellings such as `02`, `+1`, hexadecimal input, signed zero, NaN,
  or a list's precise element rendering are not observable;
- aliases and containers cannot observe mutation through a shared object; and
- reference ownership and copy-on-write transitions are exact on every normal,
  abrupt, callback, guard-failure, and deoptimisation edge.

Registry representation effects declare command-specific mutation or sharing
behaviour. Generic representation and ownership passes interpret those facts;
an emitter must not maintain its own list of shimmering commands.

### Native numeric lowering

`TclType::Int` does not mean WASM `i32` or `i64`. Tcl 9's integer domain
includes arbitrary-precision bignums. Native integer lowering requires an
interval or equivalent proof that every operand and result remains in the
chosen machine domain, or a guard and exact slow path.

Checked arithmetic must transfer to the slow path before any visible side
effect when overflow, division edge cases, shift bounds, or conversion failure
would leave the native domain. The slow path receives the original operand
values or exactly materialised equivalents. It performs the operation once.

The numeric proof also preserves:

- Tcl's integer-versus-double promotion rules;
- bignum growth and comparisons;
- division, modulo, exponentiation, and shift semantics;
- floating-point NaN, infinity, signed-zero, and domain-error behaviour;
- boolean conversion and error wording where observable; and
- the original or canonical string representation required at the next Tcl
  boundary.

Numeric rules are profile-dependent. The proof key includes the dialect's
runtime base rather than assuming that command availability and runtime
behaviour use the same Tcl version.

### Completion, errors, and suspension

Every specialised region preserves `(code, result, return-options)`, including
custom integer completion codes. `return`, `error`, `break`, and `continue`
cannot be reduced to a trap or Boolean status. Error paths preserve `-errorinfo`,
`-errorcode`, `-errorstack`, `-level`, source location, and any state committed
before the abrupt completion.

Suspension is independent of completion. A potentially yielding operation
requires an explicit suspension plan that materialises the frame, spills live
native values, retains owned objects, and restores them on resume. TclOO
receiver/next-chain state, namespace context, trace scopes, and interpreter
identity travel with the suspended flow.

## Difficult Tcl surfaces

### Namespaces, aliases, and interpreters

Namespace resolution includes current namespace, namespace path, imported
commands, global fallback, and the namespace-specific unknown handler in the
order defined by the selected Tcl version. Namespace deletion can recursively
destroy commands, variables, traces, and TclOO objects, and can run callbacks
partway through mutation.

`rename` moves command identity and attached traces. `interp alias` can cross
parent, child, sibling, safe, and ordinary interpreters. Safe interpreters have
hidden-command and policy state that is not implied by the source dialect.
Every transition comes from registry facts and invalidates the corresponding
runtime proof domain.

### TclOO

A TclOO direct call requires more than the receiver's source spelling. Its
proof identifies the object and class generations, object-private namespace,
per-object and class methods, superclasses, mixins, filters, forwards, export
and private visibility, unknown-method handler, and the exact current method
chain used by `my`, `next`, and `nextto`.

Object/class configuration, rename, copy, destruction, namespace deletion,
and callbacks invalidate relevant proofs. A saved `my` command or callback can
retain receiver identity across a rename. No direct TclOO plan is selected
until the common object graph can express these identities and transitions;
otherwise normal runtime dispatch remains the slow and default path.

### Dialects and versions

The semantic cache and guard key contains the interned `DialectProfile`, its
availability/signature base, runtime base, expression grammar base, selected
Tcl patch-level policy, registry fingerprint, and runtime ABI version. A
dialect is not merely a set of available commands: vendor commands, safe
surfaces, event context, numeric behaviour, namespace rules, and callbacks may
differ.

Command-level differences remain registry data. Common algorithms may branch
on a typed version/profile property, never on a command spelling or an ad hoc
dialect-name list.

## Pass ownership

Optimisation should be lifted as high as its proof permits:

1. **Registry:** command/subcommand/form grammar, semantic operation, effects,
   transitions, representation behaviour, result stability, callbacks, and
   dispatch dependencies.
2. **Common compiler:** identity, cell/frame, trace, type/range,
   representation, ownership, completion, suspension, and region-selection
   proofs. Source O-code optimisations consume the same facts where applicable.
3. **Shared runtime/codegen contract:** materialisation, generic argv dispatch,
   completion transport, guard identities/domains, deoptimisation metadata,
   and ownership protocol available to TclVM and WASM-capable code generation.
4. **Target plan:** machine representation, calling convention, guard encoding,
   and legal target instructions.
5. **Emitter:** serialisation of an already-selected immutable plan.

Pure list parsing and quoting, Tcl string and index operations, argument and
option parsing, formal-parameter binding, numeric conversion, and completion
option construction belong in shared utilities below the registry and
backends. A target that cannot share the required primitive selects a runtime
intrinsic or generic invocation.

The current implementation crosses those layers as follows:

| Layer | Shipping role in these slices |
|---|---|
| `tcl-registry` | Owns `StringLength`, `ChannelWrite`, `Set`, `Expr`, and `Return` semantic identities, completion/effect descriptors, and dispatch dependencies. Compiler and emitter code do not select by command spelling. |
| Common compiler | `MixedRegionPlan` records NodeId-keyed generic, guarded, lowered, and opaque regions with exact prebuilt-argv slow-path identity. `CommonAotProofPlan` and `native_integer_proof` retain direct-call, materialisable-slot, frame, boundary, SCCP, type, and range evidence with typed declines. |
| `tcl-runtime-api` | Owns the stable guard identity/domain/token vocabulary and code-generation ABI descriptors. |
| Rust Tcl runtime | Re-resolves evaluated argv using the live dialect, mints and checks per-interpreter guard tokens without epoch wrap, executes the boxed `StringLength` intrinsic, and supplies generic argv and wide-integer/channel-write boundaries. |
| WASM selector/emitter | Consumes the common immutable evidence. It emits the guarded boxed intrinsic and exact sealed native-add shapes described above. |
| TclVM | Remains a semantic and differential-test target. It does not yet consume `MixedRegionPlan`, `CommonAotProofPlan`, or the native i64 selection, so this implementation must not be described as TclVM AOT support. Future VM integration should consume these common identities and proofs rather than recreate them in bytecode selection. |

### Analysis reuse is not authorisation

Source optimisation and semantic AOT share retained compiler facts, not
source-edit decisions. The authoritative O100–O130 names, descriptions, and
profiles are the
[`generated optimisation catalogue`](../../generated/optimisation_codes.md);
the relationship below deliberately does not assign one implementation module
to each code.

| Common fact | Material semantic-AOT consumer | Boundary |
|---|---|---|
| SSA and def-use | Direct actual identities, native-expression operands, and materialisable-slot candidates | A dead or single-use source value is not proof that its Tcl cell or frame is unobservable. |
| SCCP | Constant values and interval seeds | O100/O101 propagation or folding output is not consumed, and an unreachable-code suggestion is not a region-deletion proof. |
| Type lattice and the central interval lattice | Direct-call actual types and native integer ranges | `Int` is not a machine-width proof; finite operands still require a proved result range or checked boxed fallback. |
| Command binding, registry invocation facts, and mutation/trace summaries | Direct procedure and semantic-operation identity, dispatch domains, and guards | O103/O129 folding eligibility and GVN purity are compile-time candidates, not live command identity. |
| Variable escape, observability, and place/alias facts | Frame-private and storage/materialisation premises | `Local`, unread, or untraced is necessary in some plans but never sufficient for frame elision. |
| Interprocedural summaries and binding-safe call sites | Joined caller types and ranges | Dynamic, unenumerated, rebound, or traced callers force a typed decline. |
| GVN/PRE/LICM findings | None today | O105/O106 results remain source-optimiser findings. Shared registry/effect legality may be reused independently. |
| DCE/dead-store findings | None today | O107–O109, O112, and O126 results cannot authorise storage or frame removal. |
| Liveness | None in the common AOT plan today | Existing SSA and name-level liveness serve diagnostics, source elimination, and slot coalescing. AOT still needs escape, observer, completion, suspension, and deoptimisation proofs. |

The full O-code overlap and current non-consumers are recorded in
[`optimisation-passes.md`](optimisation-passes.md#reuse-by-semantic-aot).
`optimiser::PassContext`, O-code findings, source spans, replacement groups,
and profile selection remain source-optimiser state. `common_aot_plan`,
`native_integer_proof`, and later target-neutral selectors own typed evidence
and declines. Target plans own representation and calling convention, and
emitters only serialise a selected plan.

### Consolidation boundaries

New AOT work should remove, rather than copy, the remaining utility seams:

- use one deterministic `CompilationUnit` function traversal;
- retain one CFG/SSA call-site carrier with executable-word provenance and
  exact actual-value identities, instead of rescanning flattened arguments;
- expose join, finite-bound, and checked-arithmetic operations from the central
  interval lattice rather than maintaining native-proof-local variants; and
- route interval literal seeding through the dialect-aware Tcl numeric parser.

The existing multiple liveness views are not one-for-one duplicates: the
Explorer detector reports unread SSA stores, O109 decides source deletability,
and slot allocation computes name-level interference. They may share lower
level primitives, but each must retain its distinct conservative contract.

## Implemented first tier and widening order

The first native procedure tier implements this deliberately narrow subset:

1. one ordinary procedure with required scalar parameters only;
2. exact live procedure, `expr`, `return`, `set`, and output-boundary operation
   proofs obtained through registry semantic identities;
3. no trace, alias, frame observer, callback, suspension, or opaque fallback;
4. two exact constant operands whose addition is proved to remain in i64;
5. exact coverage of the four top-level statements; and
6. boxing only the native result at the registry-proved channel-write
   boundary.

For the representative `add` script, parameter types may flow from constant
call-site arguments into `b` and `c`. That proves numeric candidacy. It does not
by itself prove that top-level `d` and `e` may disappear, that `add` need not be
registered, that its frame is unobservable, or that Tcl integer addition cannot
produce a bignum. Those are separate obligations above.

The common plan now proves those separate obligations for the exact sealed
demonstration only. A proof miss declines before native emission; there is no
guarded or deoptimising native activation in this tier. Widening should proceed
through checked boxed overflow fallback, non-constant machine-range operands,
general materialisation edges, default parameters and `args`, mixed
numeric/string values, namespace-relative procedures, safe/child interpreters,
and finally TclOO dispatch.

## Verification matrix

Every code-changing pass is tested with the pass off, that pass alone on, and
all experimental passes on. Compare C Tcl 9, applicable older C Tcl versions,
TclVM, and WASM. For dialects, compare the authoritative dialect runtime or its
ratified oracle fixtures as well.

The current guarded-intrinsic tests cover explicit off/on selection, real
runtime fast entry, rename/rebinding fallback, execution-trace fallback, exact
argv reuse, and guard-token ownership. The native-add tests cover individual
pass ablation, hosted-versus-sealed policy, extra-statement decline,
trace-induced decline, exact common/Explorer evidence, emitted WASM shape, and
real runtime linking. That is the acceptance set for these two slices, not a
claim that the full adversarial matrix below has passed for wider regions.

The result comparison includes:

- stdout and stderr bytes;
- completion code, result bytes, and return-options dictionary;
- error information, error code, error stack, and source location;
- variables, arrays, namespaces, procedures, commands, and object state left
  after execution;
- frame and command introspection;
- trace callback arguments and ordering; and
- allocation/ownership assertions in instrumented runtimes.

Required adversarial families include:

- redefine, rename, delete, namespace import/path/unknown, and aliases before
  entry, during word substitution, and from callbacks;
- variable, command, execution, and active step traces returning every
  completion class;
- dynamic `eval`, `uplevel`, `upvar`, `namespace upvar`, `source`, and `subst`;
- safe and child interpreters, hidden commands, deletion, and cross-interpreter
  aliases;
- TclOO methods, forwards, filters, mixins, unknown, `my`, `next`, `nextto`,
  rename, copy, and destruction;
- required/default/`args` parameters and malformed or wrong-arity calls;
- non-canonical numeric strings, machine-boundary values, bignums, division
  edges, NaNs, and copy-on-write containers;
- custom completion codes, partial state changes before error, and nested
  `catch`/`try`; and
- coroutine creation, yield, deletion while suspended, resume, and trace state
  across suspension.

Property and fuzz campaigns should generate mutations at every guard boundary
and minimise any divergence to a Tcl script plus pass configuration. A test is
not an AOT differential if both sides execute all semantically relevant
commands through the same eval-only compatibility path.

## Documentation and code ownership

The generated O-code tables are projections of `DiagCode` metadata and are the
published catalogue for code, category, description, and profile membership.
`optimiser::PassId` and the pass modules own execution order and implementation.
The same O-code can legitimately be produced by more than one implementation
site, so a second hand-maintained one-code-to-one-file table would be false
precision and another source of drift.

Per-code KCS pages explain user-visible rewrites. Design documents explain pass
contracts and shared proofs. New semantic AOT controls must not allocate an
O-code unless they also produce a source-level optimisation finding; backend
plan evidence uses its own typed pass and decline identities.

## Related documents

- [Common semantic compiler](common-semantic-compiler.md)
- [WASM code generation](wasm-codegen.md)
- [WASM extensions](wasm-extensions.md)
- [Optimisation passes](optimisation-passes.md)
- [Var-escape analysis](var-escape-analysis.md)
- [Command registry](command-registry.md)
