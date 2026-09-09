# Review supplement: fact interfaces and registry authoring

Companion to the [adversarial review](value-transfers-review.md) and
[code coverage ledger](value-transfers-coverage-review.md). This is a
proposed contract, not implemented API or an approved SpecTcl grammar.
The examples deliberately show the desired ownership boundary. They must
not be copied into a pack and mistaken for supported syntax.

## Integrate the existing hooks; do not build a parallel type system

The existing dynamic return-type facility is a useful example of the right
ownership. `ReturnTypeHookId` selects algorithms in the **registry's**
[return_type.rs](../../../rust/tcl-registry/src/return_type.rs), not in the
compiler. `CommandSpec::return_type_for_call` is already shared by SSA type
inference, the registry's sanitiser query and representation-sensitive
consumers. Its `Regexp`, `Lsearch`, `Regsub`, `Scan` and `Pid` algorithms
must be incorporated, not superseded by another table in value transfer.

Two details make a naïve merger incorrect:

- These hooks guarantee an **internal representation**, not just membership
  in a semantic value set. An empty `regexp -inline` result can be a pure
  string even though it is a valid empty Tcl list. `lsearch -inline` can
  return a caller-owned element with its pre-existing representation.
- The current entry point selects a static subcommand return type for an
  ensemble, whereas the non-ensemble hook's `None` is authoritative: it
  does not fall back to the command's static type. A replacement must
  preserve that abstention and support per-form/subcommand hooks. It must
  not turn unknown into the convenient static default.

The existing string-array interface also cannot directly see expansion
provenance. Evolve it towards the already resolved invocation and typed
operand views, with a compatibility adapter during migration. Do not scan
`$` and `[` in decoded values to rediscover whether a word was substituted.

The compiler already owns rich `TypeLattice`/`TypeShape` domains, bounded
unions and aggregate shapes in
[types.rs](../../../rust/tcl-compiler/src/types.rs). The registry owns
`TclType`. Keep dependency direction intact: registry-neutral fact views
and typed answers below the compiler; adapters, lattice order, joins,
widening and solver state in the analysis owner. Do not make the registry
depend on compiler SSA types, and do not clone the compiler lattice in
SpecTcl. If a shared protocol needs a new crate, that is a small contract
crate, not a second analysis engine.

## One invocation, several independently available facts

“One semantic owner” does not mean one enormous callback that must compute
all facts before answering any query. It means one resolved meaning and
context, with composable queries and explicit dependencies. A known return
type must remain available when the concrete evaluator cannot run. A taint
transfer must remain available when the return value is unknown. A loop
body must remain analysable when its iteration count is unknown.

| Interface | Registry-owned answer | Generic consumer owns | What invalidates it |
|---|---|---|---|
| Invocation/form resolution | Argument roles, selected form, option semantics, expansion-sensitive uncertainty | Source-to-operand mapping and canonical resolved invocation | Word/expansion facts, binding, spec overlay, dialect |
| Structural plan | Bodies, scopes, binders, control/completion protocol, declared structural effects | AST/IR construction, CFG edges, SSA definitions and phi placement | Changed grammar, binding, body text, alias structure |
| Exact evaluation | Exact result and ordered store outcomes, or typed decline/error | Proven operand reads, execution budget, result validation and materialisation | Operand/storage versions and all declared environmental dependencies |
| Type and shape transfer | Semantic result possibilities, intrep guarantees, per-target/element relationships | Rich type lattice, joins, aggregate limits, consistency checking | Operand types/shapes, selected form, target semantics |
| Existence and aliases | Define/preserve/unbind/may-write, escape and alias descriptions | Place identity, reaching definitions, exceptional state, joins | Storage and alias epochs, trace/uplevel/unknown effects |
| Range and partial-value transfer | Bounds/lengths/prefixes or typed residual operations with prerequisites | Abstract interpretation, widening and bounded residual representation | Operand range/shape facts, arithmetic profile, effect facts |
| Effects and completion | Reads/writes, observable operations, possible error/return/break/continue/yield | Control flow, sequencing, deletion/motion proofs | Resolved implementation, callback/body summaries, traces and state |
| Taint and provenance | Source/sink/transform relationships, encoding/normalisation kind | Flow lattice, provenance paths, context-specific sink checks | Operand flow, unknown effects, sanitiser identity and dialect |
| Intrep and sharing | Guaranteed construction/conversion and alias/share relationships | Shimmer/use-site and mutation-copy analysis | Value provenance, representation transitions, escape/share facts |
| Protocol/resource state | Typed domain events such as collect/release, HTTP commit, widget ownership | Domain state machine and path joins | Event/body flow, command meaning, referenced resource world |
| Interprocedural summary | Declared external summary or analysable body with its contract | Call graph, recursive fixed point, context limits and dependencies | Body, callee/binding set, captured/global state and pack revision |
| Backend capability/lowering | Target operation, permitted forms, required semantic and safety facts | Backend IR construction, legality, allocation and verification | Language profile, ABI, helper/map capabilities, proof-changing rewrites |
| Validation | Typed constraint verdict and offending operand/member | Rule code, severity, source range, suppression and presentation | Invocation and relevant fact/context revisions |

An analyser-local fact not listed here is not exempt. Its owner must declare
its input dependencies, monotone transfer/join or non-dataflow evaluation
phase, unknown policy, cache key and downstream consumers. The
[ledger](value-transfers-coverage-review.md) is the concrete migration
checklist; a generic “other facts” escape hatch is not an implementation.

### Construction and solving are different phases

1. Resolve invocation and structural semantics. Build scopes, IR, CFG and
   SSA using validated plans. Record conservative effects for unresolved
   calls and dynamic bodies.
2. Run the analysis domains over that graph. Hooks query immutable input
   facts and return deltas; the owning solver validates and applies them,
   schedules dependent queries, joins and widens.
3. If improved binding/alias/structural information changes the graph,
   invalidate the affected construction query and rebuild its dependent
   analyses. Do not let an arbitrary hook insert SSA definitions halfway
   through an SCCP worklist.
4. Publish an immutable, revision-labelled fact snapshot. Diagnostic rules,
   optimisers, code generators and editor queries consume it according to
   their required precision tier. Rewrites get their own proof obligation.

These are logical phases, not a requirement to eagerly compute the entire
pipeline for every editor request. A demand-driven query graph can realise
them. Cycles such as type inference asking for an exact result whose
evaluator asks for the same type need an explicit SCC/fixed-point or
conservative cycle result, not recursive callback execution.

Cross-domain reduction also needs rules. An exact value may refine a
semantic type or range; it does not prove a runtime representation that the
evaluator fabricated while transporting the value. A numeric type does not
erase arbitrary taint. A failure of exact evaluation does not erase a
separately established length bound. Contradictory hook claims must be
rejected/quarantined with conservative fallback, not resolved by whichever
callback ran last. Validation catches structural contradictions; it does
not certify semantic truth. The owner accepts well-formed workspace-authored
facts as authoritative without a separate trust gate; authors own mistakes
in their model, including resulting diagnostic suppression or misoptimisation.

### A small proposed ABI shape

The following is **Rust-shaped pseudocode**, with deliberately proposed
types. It illustrates capabilities and answer shapes, not compilation-ready
signatures or a demand to allocate every field on every call.

```rust,ignore
trait AnalysisInputs {
    fn invocation(&self) -> &ResolvedInvocationView;
    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView;
    fn prior_store(&self, place: PlaceRef, domain: FactDomain) -> FactView;
    fn context(&self) -> &SemanticContext; // includes language, binding, epochs
}

trait CommandSemantics {
    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer;
    fn transfer(&self, domain: FactDomain, input: &dyn AnalysisInputs)
        -> TransferAnswer;
    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget)
        -> EvalAnswer;
}

enum EvalAnswer {
    Exact(InvocationOutcome),
    Declined(DeclineReason), // input unknown, budget, unsupported, host absent...
}

struct InvocationOutcome {
    // Includes Tcl errors and other completion kinds, not just success.
    completion: CompletionOutcome,
    result: ExactValueOrUnavailable,
    ordered_stores: Vec<StoreOutcome>,
    evidence: DependencyEvidence,
}

enum StoreOutcome {
    Write { target: DeclaredTargetId, value: ExactValue },
    Preserve { target: DeclaredTargetId },
    Unbind { target: DeclaredTargetId },
    MayWrite { target: DeclaredTargetId, facts: FactBounds },
}
```

`DeclaredTargetId` is validated against the structural plan. It is not a
hook-selected arbitrary SSA ID. Aliased targets are reconciled in execution
order. The full protocol needs conditional outcomes and effects on exceptional
completion; a vector of independent success-only writes is insufficient.
Unknown effects remain conservative even when another answer is exact.

The ABI should reuse existing descriptors where they already express a
relationship: `VarWriteTyping::ReturnValue`, fixed types, destructuring and
elements-of typing are useful **type relationships**, not executable transfer
algorithms. Keep their per-target interpretation; do not broadcast one
return type to every definition of a multi-output command.

## Worked native and SpecTcl declarations

Every snippet below is a **proposed authoring sketch**. Today’s actual
`const_fold {words ctx} {...}` surface and native `CommandSpec` fields remain
the compatibility baseline. Names such as `semantics`, `evaluate`, `facts`,
`iterate` and the `analysis::` capability API below are not implemented
SpecTcl syntax. Adopting them requires the registry field, loader, exporter,
renderer, studio form, documentation and parity tests together.

The registry can register a typed native function or a descriptor composed
from generic operations. Consumers dispatch through the interface; they
must not acquire new `Expr`, `Regexp` or vendor-command match arms.

### `incr`: direct arithmetic, independent result and mutation

```rust,ignore
CommandSpec {
    name: "incr",
    semantics: registry_semantics!(incr::SEMANTICS),
    // Existing arity, roles, version and documentation fields retained.
    ..CommandSpec::DEFAULT
}

// In registry-owned incr specialisation, using the shared numeric owner:
fn evaluate(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
    let form = incr_form(input.invocation())?;
    let target = form.declared_target();
    let old = input.prior_store(target.place(), FactDomain::ExactValue);
    let step = form.increment_or_exact_default("1");
    let outcome = numeric_core::tcl_incr(old, step, input.context(), budget)?;
    exact_outcome(outcome.completion, outcome.result, outcome.ordered_stores)
}
```

```tcl
# Proposed equivalent declaration; native binding owns the algorithm.
command incr {
    semantics -native core.incr
    evaluate -direct core.incr
    facts -native core.incr
}
```

The structural plan declares a variable read/modify/write and its possible
failure. A missing scalar can be created; an unknown scalar is not silently
zero; an array/traced/aliased cell needs its corresponding effects. The
shared numeric operation implements the selected Tcl release's parsing and
bignum behaviour. `incr x` can produce both `result = 4` and `x = 4`, but
knowing the result does not permit deleting the write or its traces.

No VM is needed. The same is true of small direct string operations when
their existing core correctly supplies Tcl character, index and error
semantics. “Direct” does not mean unchecked Rust `+` or Unicode scalar count.

### `expr`: shared lazy expression engine, not a miniature new interpreter

```rust,ignore
CommandSpec {
    name: "expr",
    semantics: registry_semantics!(expr::SEMANTICS),
    ..CommandSpec::DEFAULT
}

fn evaluate(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
    let expression = expr_arguments::prepare(input.invocation())?;
    let ops = ProvenTclExprOps::new(input, budget);
    shared_expr_engine::evaluate(expression, ops)
}
```

```tcl
# Proposed: names our shared expression route, not arbitrary VM fallback.
command expr {
    semantics -native core.expr_arguments
    evaluate -expression tcl.expr
    facts -native core.expr_facts
}
```

`ProvenTclExprOps` adapts the existing
[ExprOps](../../../rust/tcl-syntax/src/expr/eval.rs) contract: resolve a
variable when reached; evaluate a supported command or math function only
when reached; retain ordering and completion. `expr {0 && [expensive]}`
does not require `expensive` to be calculable. `expr {$x + [incr x] + $x}`
must not read all three operands from the initial state. A first delivery
may decline reached stateful substitutions instead of modelling them, but
must not evaluate them against stale inputs.

The result is a Tcl value, not necessarily a number: a bare variable or
selected ternary arm can return a string. Braced and concatenated/unbraced
arguments have different evaluation stages. Math-function and nested-command
binding dependencies belong in evidence and cache keys.

Partial simplification is a separate query returning a bounded residual
expression and proof prerequisites. `2 + 3 + $x` can fold its closed subtree;
`$x + 1 + 2` cannot unconditionally regroup to `$x + 3`. Keep the existing
floating-point counterexample from R15 as an ordinary fixed regression.

### `regexp`: our engine, with typed precision and per-target outcomes

```tcl
# Proposed declaration; no diagnostic codes or compiler callback IDs.
command regexp {
    semantics -native core.regexp_forms
    evaluate -direct core.regexp
    facts -native core.regexp_facts
}
```

The native specialisation delegates to shared regexp command plumbing backed
by **our `tcl-regex` engine**, as the owner requested. It resolves flags once
from the canonical invocation. Direct matching returns typed exact match,
exact no-match, Tcl pattern error or analysis decline; fuel exhaustion and
approximate capture results must not masquerade as exact answers (R13).

For `regexp {a(b)} $subject whole capture`, an exact match produces the
integer result and ordered writes to `whole`/`capture`. No match preserves
their previous values and existence. An unknown subject yields bounded
conditional writes and a known numeric result type without inventing
captures. `-inline`, `-indices`, `-all` and `-about` select different forms;
the existing dynamic return-type hook is an adapter into those same form
semantics, not a competing switch parser. Index units follow the target Tcl
release, and copied substrings retain exact spelling.

Pattern taint and ReDoS findings consume the registry's pattern-role and
engine/flow facts. They do not run a second matcher inside diagnostic
generation. A warning about pattern cost is not an engine timeout, and a
timeout is not proof of a vulnerability.

### A private executable command, without granting the pack analyser mutation

Consider a pack command `tenant::label NAME` whose real runtime implementation
returns `string cat "tenant:" $NAME`. It can expose an executable evaluation
route and a partial-value relationship without extending SCCP.

```tcl
# Entire block is proposed API, including capability names and fact syntax.
command tenant::label {
    arity 1
    semantics {
        effects {no_store_writes no_external_io}
        result -semantic string
    }
    evaluate -implementation tenant.label.v1 -host bounded_tcl {
        inputs {arg 0 exact}
        depends {tcl_profile implementation_identity}
        body {name} {
            return [string cat "tenant:" $name]
        }
    }
    facts {
        result -string_segments {{constant "tenant:"} {operand 0}}
        taint -result_from {arg 0}
    }
}
```

If the argument is unknown, do not invoke the body with a placeholder.
Return an unknown exact value plus the proven prefix/segment and taint
relationship. A prefix does not sanitise the unknown suffix. The author
declares which implementation the evaluator models; no independent
certification of that claim is required. Resolved binding and implementation
revision still determine when the model applies: a later redefinition cannot
silently reuse the old answer merely because the spelling is unchanged.

The bounded host denies ambient files, network, clock, randomness and
undeclared globals, accounts for aggregate request cost, and isolates or
resets mutable state. A persistent global counter cannot affect successive
answers. These are execution/correctness contracts, not an author-trust gate.
The owner confirmed that loaded workspace pack facts may narrow analysis,
prune branches and justify code elimination without another opt-in or
provenance cap. A well-formed but false assertion is the author's
responsibility, not something the analyser must independently certify.

## EDA: own loops and opaque collections

The current bundled SDC pack declares
[`foreach_in_collection`](../../../specs/sdc_base.tclspec) with `VarWrite`
and `Body` roles, `LOOP_LIST_HEADER` and native `Foreach` analyser dispatch.
The current
[foreach handler](../../../rust/tcl-compiler/src/analyser/handlers.rs)
parses a braced literal iterable as a Tcl list and can simulate selected
definition effects for each element. This is concrete evidence that reusing
an existing loop hook can import more assumptions than body walking alone.
It is not evidence that a vendor collection handle is a Tcl list, or that
this review executed a proprietary vendor runtime.

Separate **clause grammar**, **iteration protocol** and **enumeration**:

```tcl
# Proposed SpecTcl shape, not a statement of every vendor's exact contract.
command foreach_in_collection {
    arity 3
    semantics {
        iterate {
            binder -arg 0 -grammar vendor.single_variable
            iterable -arg 1 -kind vendor.collection
            body -arg 2 -scope enclosing
            yield -semantic vendor.object_handle
            cardinality -from vendor.collection_summary
            completion -contract vendor.collection_loop_completion
            zero_iterations -bindings preserve
        }
    }
}
```

The binder grammar, completion contract and zero-iteration behaviour must
be checked for each vendor pack, not copied uncritically from this sketch.
Some APIs use multiple variables, collection unions, destructive iteration,
implicit variables or callback prefixes. Those differences belong in the
registry's plan, not `if vendor_command_name` branches in the analyser.

Required consequences:

- A constant handle is not constant collection contents, membership, order
  or length. Model the vendor object/collection type separately from Tcl
  list representation. Collection query facts depend on the vendor design
  database revision and command side effects.
- Analyse the body with a typed unknown yielded object even when no vendor
  runtime or enumeration service is available. Build the zero-iteration
  edge, back edge and binding definitions from the declared protocol.
  Preserve prior variables on the zero-iteration path where required.
- Known cardinality can help reachability/range analysis without exposing
  elements. Known elements can be used only with adequate identity and
  snapshot evidence. Do not split the printable collection handle.
- Break, continue, return, error, nested loops and implicit results must
  follow the vendor's completion rules. Never infer termination merely
  because there is a body argument or `CONTROL_FLOW` trait.
- Mutation of the vendor design database invalidates affected collection
  summaries and object-property facts. Tcl-local constant propagation can
  remain valid independently. An unknown external command needs a declared
  world-effect boundary or conservative invalidation.
- Vendor filter strings are their own language unless the pack explicitly
  declares Tcl expression semantics. Use a domain owner with a typed query
  contract; do not feed every filter into our Tcl VM.
- Lowering may retain a generic runtime call even when analysis understands
  the loop. Body analysis, exact enumeration and executable backend support
  are three separate capabilities.

An acceptance fixture should define a **new vendor loop spelling** in a
pack, with no consumer edits, and exercise W210/W211, type/taint flow,
zero/multiple iterations, break/continue/error, collection invalidation and
conservative lowering. Include a collection handle whose printed text is a
perfectly valid multi-element Tcl list to expose accidental list parsing.

## eBPF: distinct semantics and target proof obligations

The [existing backend contract](ebpf-backend.md) defines BPF-Tcl as a
statically typed language using Tcl syntax, **not a Tcl runtime target**.
It has its own IR and restricted operations. The shared registry already
has [BpfOpSpec](../../../rust/tcl-registry/src/bpf_op.rs); the current native
[`loop` declaration](../../../rust/tcl-registry/src/commands/bpf/loop_.rs)
is a good example of concise registry-owned target metadata:

```rust
// Existing API shape from commands/bpf/loop_.rs, not the proposed API above.
const OP: BpfOpSpec = BpfOpSpec::structural(BpfOpKind::LoopMacro);
CommandSpec {
    name: "loop",
    surface: Some(SpecSurface::BPF),
    arity: Arity::exact(3),
    bpf_op: Some(&OP),
    ..CommandSpec::DEFAULT
}
```

Keep that integration. Evolve capabilities in the existing descriptor or
its shared protocol rather than introducing a parallel target registry.
The backend may own implementation of generic BPF operations; command
spelling/form recognition belongs in registry descriptors.

### A Tcl value evaluator is not a BPF constant folder

BPF-Tcl signed integer division truncates towards zero: `-7 / 2` is `-3`.
Tcl integer division floors: the result is `-4`. Constant evaluation must
therefore select a **language-semantic profile**, not merely a Tcl version
and deployment target. Reuse the expression parser/engine only with the
correct BPF arithmetic adapter. Include width, overflow, signedness,
division/remainder and shift rules in the profile and cache dependencies.
Do not invoke the Tcl VM to establish BPF arithmetic results.

Similarly, BPF packet/map handles are not Tcl values that happen to have a
numeric-looking spelling. Preserve pointer kind, offset/range, nullability,
map identity and lifetime through typed facts. A constant-looking address
must never erase provenance or manufacture permission for a memory access.

### Loop and safety contracts

The existing `loop N VAR {body}` unrolls a literal bound, currently limited
to 64, before CFG construction. This is a bounded macro protocol, not
Tcl `foreach`, an arbitrary vendor loop or permission to support native
Tcl `while`. Its induction bindings and source-origin map must survive
unrolling. If future support accepts a statically proved bound instead of
a literal, that is an explicit language change with resource and verifier
tests, not an accidental consequence of improved SCCP.

| Fact/proof | Where it comes from | Why knowing an output is insufficient |
|---|---|---|
| Subset/form acceptance | Registry operation and BPF frontend rules | An unsupported call can have a mathematically obvious result |
| Packet bounds | Dominating bounds checks and pointer/range facts | A constant byte offset may still exceed packet length |
| Map access | Map schema, helper capability, null check and handle lifetime | A key constant says nothing about lookup success or pointer validity |
| Scalar operation legality | Width/signedness and BPF arithmetic profile | Tcl bignum or floor-division results may not be BPF results |
| Bounded execution | Declared loop protocol and target resource limit | A computable loop count may exceed allowed expansion or proof budgets |
| Stack and calling convention | Backend liveness/allocation, ABI and helper contracts | Value/type inference does not establish a 512-byte stack bound |
| Handler completion | CFG path completion and composition order | One branch's constant verdict does not prove all paths terminate |
| Target capability | Program/attach type, profile allow/deny, helper/map support | A valid operation in one BPF program type can be forbidden in another |

Optimisation must preserve or regenerate these proofs. A motion/CSE pass
must not move a packet load above its guard, retain a stale pointer across
an invalidating helper or conflate accesses to different maps. Backend
legality is a separate consumer of shared facts; diagnostic enablement must
not control rejection of an illegal program.

There is also a separate diagnostic catalogue: `BpfDiag` has 18 codes,
including subset, typing, bounds/resource, profile and completion errors.
The rootless verifier has five uncoded `VerifyError` variants. Both are
included in the ledger rather than silently omitted because they are not
in `DiagCode::ALL`. The rootless verifier is not proof of acceptance by
every kernel verifier; this review runs no privileged load tests.

An illustrative future SpecTcl declaration could expose the same existing
BPF descriptor, without Tcl execution:

```tcl
# Proposed parity spelling; do not accept this in the loader until implemented.
command loop {
    surface bpf
    arity 3
    backend bpf {
        operation loop_macro
        bound -arg 0 -require literal -maximum 64
        binder -arg 1
        body -arg 2
        expand_before cfg
    }
}
```

The maximum belongs to one target contract, not independently authored
copies in a pack, lowering pass and verifier. The source authoring surface
may refer to a named contract instead of repeating `64` as shown here.

## Acceptance: centralisation demonstrated, not asserted

For every example, require native/SpecTcl expansion parity, cold/warm query
parity, overlay/target/binding invalidation and source mapping. The loader
must validate capabilities, form selection and per-target output plans.
Renderer/studio round trips must retain all semantics or explicitly report
an unsupported gap, never silently discard a hook.

Then add a new spelling with an existing contract and verify that generic
analysis, relevant diagnostics and supported backends see it without new
name matches. A genuinely new semantic operation may require extending a
generic interface and its owning solver/backend; “no analyser changes ever”
is neither achievable nor the goal. The goal is that command catalogue
growth does not repeatedly duplicate already expressible semantics.
