# Value transfers — the consumer interface contract

What the command registry tells the analyser about one command invocation,
and what the analyser does with the answer. The registry owns the
command-specific meaning: what an invocation computes, which storage it
writes, in what order, and which bindings and target semantics the answer
depends on. The analyser owns the generic operations: proving a word's
value, resolving a storage place, reading the fact that holds there,
running the solver, joining, validating, and publishing facts with
provenance for every consumer. This is the design for
[issue #1943](https://github.com/bitwisecook/tcl-lsp/issues/1943). Read it
before touching a fold, a lattice transfer, or a constant-condition rewrite,
and before adding compile-time knowledge about any command to a consumer.

It is one of three pages. [value-evaluation.md](value-evaluation.md) is the
evaluation contract: the routes an answer is computed by, the shared cores
and engines behind them, state isolation, budgets, and caches.
[value-transfers-migration.md](value-transfers-migration.md) is the
migration plan: the inventory of hand-written command knowledge, the
delivery slices, the gate, and what changes for every pass and diagnostic.
[registry-consumer-contracts.md](registry-consumer-contracts.md) places the
value axis among the other axes and holds the runtime, package, and
C-extension contracts, none of which this one waits for.

> **Status — a proposal, not a description of what is built.** The
> `CommandSemantics` interface and every shape in § The interface —
> `AnalysisInputs`, `ResolvedInvocationView`, `OperandId`, `PlaceRef`,
> `TargetId`, `FactView`, `FactDomain`, `DomainFact`, `WordStructure`,
> `BodyRegion`, `EvaluationState`, `NestedPolicy`, `PlanAnswer`,
> `Binder`, `BodyPlan`, `Reconcile`, `CompletionProtocol`,
> `HandlerPlan`, `IterationPlan`, `IterableKind`, `ExitRule`,
> `SelectionContract`, `TemplateWordPlan`, `ScriptRegion`,
> `VariableRead`, `TransferAnswer`, `ExistenceTransfer`,
> `CompletionPath`, `ExistenceOutcome`, `Existence`, `BindingKind`,
> `RangeModel`, `SegmentFacts`, `TaintTransfer`, `SelectionFact`,
> `EvalAnswer`, `InvocationOutcome`, `CompletionOutcome`,
> `StoreOutcome`, `ExactValue`, `ExactValueOrUnavailable`,
> `RepresentationEvidence`, `ValueShape`, `TypeFacts`, `FactBounds`,
> `DependencyEvidence`, `RouteIdentity`, `AnalysisContext`, `Budget`,
> `BudgetLimit`, `AnalysisTier`, `DeclineReason` (whose `Axis` and
> `NoRouteReason` payloads the evaluation page defines) — and the
> section-local shapes `EdgeRefinement`, `TransferSummary`, `ParamRole`, and
> `LoopEnumeration`, with the migration plan's `folded_types` side map,
> name nothing in the workspace today; every Rust shape
> on this page is a sketch of capabilities and answer shapes, not
> compilable signatures. Every *existing* identifier cited here was
> checked against the tree at the revision named in the migration plan.
> The observed Tcl behaviours quoted below were run under Tcl 9.1b0,
> 9.0.4, 8.6.18, 8.5.19, and 8.4.20; a line names releases only where they
> differ, or where one of them lacks the feature.

## Rulings

These are the owner's decisions, and every section below fits inside them.

1. **Ownership boundary.** Command-specific specialisation lives in
   registry-owned code and data. The analyser exposes generic operations
   and applies validated answers; it acquires no new command-name or
   command-ID arms. A compiler-owned handler kept during migration is a
   delivery choice, not a different destination: it carries a migration
   ledger entry and an expiry.
2. **Executable backing is declared, never inferred.** A command's
   evaluator route — a direct shared core, the shared expression engine,
   or a declared implementation in the bounded engine — is an explicit
   registry capability with an implementation identity, supported target
   semantics, declared dependencies, and budgets. Purity classifies a
   command; it does not supply an evaluator, a cost bound, or its
   dependencies.
3. **Workspace-authored facts are authoritative.** Once a pack is loaded,
   its well-formed semantic declarations are inputs to analysis and
   optimisation on the same footing as a shipped spec: they may narrow
   values, prune executable edges, suppress findings in the pruned region,
   and justify code elimination. There is no advisory-only tier, no
   widen-only rule, no provenance-based precision cap, no separate opt-in
   for narrowing, and no certification requirement. Authors own the
   consequences of a false fact in their environment. Binding validity,
   structural validation of an answer, cache invalidation, and evaluator
   resource limits remain in force — they are correctness and execution
   contracts, not a disguised trust gate. Provenance is kept for
   explanation, binding selection, dependencies, and invalidation, never
   for ranking authority.
4. **Regexp owner.** Concrete matching goes through our own `tcl-regex`
   engine behind the existing `tcl_cmd_core::regex` command plumbing. The
   analyser gains no second matcher.
5. **Shipped specialisations stay Rust** over the shared cores; a SpecTcl
   calculation reaches the analyser through the same interface where
   authoring one is useful.
6. **First delivery:** direct values, `expr`, one private pack command,
   and one shared analysis context. Runtime manifests, C hosting, the
   engine's WASM sibling, and new optimisation-code numbering wait for
   their own phases and gate nothing here.

## The four motivating programs

Each of these is legal, common Tcl. What the compiler knows about it today
depends on which of the tree's independent constant evaluators —
[value-transfers-migration.md](value-transfers-migration.md) § *Where
per-command knowledge lives today* inventories them — happens to be
asked, and the answer is different for each consumer.

```tcl
set s [string range foobarbaz 3 6]      ;# (1) a pure result: barb
set h [binary format H* 414243444546]   ;# (2) a pure result with no evaluator: ABCDEF
set n 1; incr n; incr n 2               ;# (3) a read-modify-write chain: 4
set acc ""; append acc foo; append acc bar
switch -- $acc {                        ;# (4) a switch whose subject is known
    baz     { puts never }
    default { puts always }
}
```

| Program | Optimiser (O-codes) | Shared lattice the diagnostics read | Why they differ |
|---|---|---|---|
| (1) `string range` | O129 fires: `string range` carries `const_fold: Some(fold_range)` (`rust/tcl-registry/src/commands/tcl/string_.rs`) and the propagation pass re-runs SCCP with `BuiltinFoldInputs` | `s` is `Overdefined` — `FunctionUnit::build` calls `sccp_with_extra_escaping` with `folds = None` (`rust/tcl-compiler/src/compilation_unit.rs`) | the shared lattice's memo key does not carry the command-binding fact (`rust/tcl-compiler/src/sccp.rs`, the `BuiltinFoldInputs` doc) |
| (2) `binary format` | nothing: `rust/tcl-registry/src/commands/tcl/binary_.rs` declares `pure: true` on the `format` subcommand and no evaluator | `Overdefined` | the registry can say *whether* a command is pure but not *what* it computes; `tcl_cmd_core::binary::format` exists and the registry already depends on `tcl-cmd-core` |
| (3) `incr` chain | O100 forwards `4` into a subsequent `$n` (`sccp_value_literal`); the `Statement::Incr` arm of `evaluate_def_with_folds` does the arithmetic | `4` — the same arm runs in both lattices | `incr` is the one read-modify-write command with a typed IR node and a hand-written transfer; `append acc …` hits `_ => LatticeValue::Overdefined` and `acc` is unknown everywhere |
| (4) `switch -- $acc` | O112 would fire if `acc` were constant (`structure_elimination.rs` resolves `$acc` by name from its own `Env`); it is not, because of (3) | no `ConstantBranch`, no I231, no O107 — even with a constant subject, `switch_subject_operand` lowers a whole-variable subject to `ExprNode::Raw`, which the expression evaluator rejects before consulting the environment | three implementations of `switch` semantics (`cfg_lower.rs`, `structure_elimination.rs`, `analyser/handlers.rs`) with two notions of subject resolution |

The contract below makes one answer per program, computed once through the
registry's declaration, visible to every consumer, and reachable by a pack
for a command the registry has never heard of. The completion test is
stated at the end: a command that fits an existing analyser interface
changes only its registry declaration and, where needed, its registry-owned
evaluator or shared core, and never SCCP, the analyser walk, a diagnostic,
or the optimiser. These four are the shortest cases;
[value-transfers-examples.md](value-transfers-examples.md) has one program
for every optimisation and diagnostic the design touches, with the tool's
observed behaviour today and the declarations behind each in Rust and in
`.tclspec`.

## Vocabulary

- **Invocation** — one resolved call: the binding the head resolves to, the
  selected subcommand and form, the source words with their expansion and
  substitution kinds, and the storage places its argument coordinates name.
- **Value transfer** — the registry-owned answer for one invocation on the
  value axis: the command's result, the ordered outcomes for each storage
  place it may affect, the semantic types of both, and the evidence the
  answer depends on. Its projection onto the lattice is what SCCP applies.
- **Place** — a Tcl storage location as the analyser resolves it: a scalar,
  an array element, or a whole array, in a frame or a namespace. Names are
  resolved to places before any outcome is composed, so two spellings of
  one cell are one target.
- **Storage outcome** — for one place, one of *write* a value, *preserve*
  the prior state, *unbind*, or *may write* with bounded facts. `None` is
  never an outcome; silence is a decline.
- **Pending** — an input the solver has not reached yet: `LatticeValue::Unknown`,
  the optimistic bottom. Pending is not "unbound", not "dynamic", and not a
  reason to erase a previously joined fact.
- **Decline** — the evaluator could not establish an answer, for a recorded
  reason: an input is not exact, the binding is suspect, a place escapes
  or is traced, the target semantics are ambiguous, a budget was exhausted,
  or the answer would be approximate. A decline widens the affected
  definitions and is never a Tcl error, a no-match, or an absent variable.
- **Exact value** — a Tcl string, preserved byte for byte: whitespace, NUL,
  backslashes, leading zeros, signed zero, and Unicode as computed. Its
  numeric classification is an additional fact, never a replacement
  spelling.
- **Evaluator route** — the declared way an exact answer is computed: a
  direct shared core, the shared expression engine, or a declared
  implementation run in the bounded engine ([value-evaluation.md](value-evaluation.md)).
- **Analysis context** — the immutable identity every query and evaluation
  runs under: registry and overlay generation, command bindings and
  namespace context, target profile and grammar, trace and escape facts,
  seeds, and evaluator revisions.
- **Branch fact** — one of three distinct things: a *proven condition*, a
  *selected edge*, or *applied reachability* in `executable_blocks`. They
  are recorded separately because the CFG does not always contain the edge
  a selection names.
- **Completion path** — one way an invocation can complete: normally, with
  a completion code, or with an error after a known prefix of its ordered
  stores. Storage outcomes are indexed by it.
- **Edge refinement** — a fact that holds for one SSA version on one CFG
  edge and in the blocks that edge's target dominates, without a new
  definition: what a taken comparison, `switch` arm, or existence guard
  proves about its operand.
- **Transfer summary** — a procedure's caller-visible transfer: which
  caller places its name arguments denote and what happens to them, its
  global writes, its result shape, and its completion domain, derived from
  the callee's own analysis under no call-site seeds.

The lattice itself is unchanged: `LatticeValue::{Unknown, Const, ConstSet,
Overdefined}` over `ConstValue::{Int, Float, Bool, String}`
(`rust/tcl-compiler/src/analyses.rs`), joined by `sccp::join`, with
`MAX_CONSTSET_SIZE = 32`. What changes is who computes the value at each
statement, and what travels with it.

## The ownership boundary

| Owner | Responsibility | Must not acquire |
|---|---|---|
| Registry | Invocation semantics; specialisation implementations; operand and target selection; the evaluator route; a stable semantic identity per specialisation | SSA internals, LSP ranges, mutable analyser access, command algorithms copied from the shared cores |
| Shared syntax, numeric, and command cores | Parsing and exact value algorithms under explicit semantic policy (release, character model, numeral grammar) | Registry lookup or analyser policy |
| Analyser and compiler fact producers | Scope and place resolution; input facts; the solver; effect and completion validation; joins; source provenance | Command-specific argument grammars or arithmetic selected by name or command ID; diagnostic enable flags |
| Diagnostic rules and edit planners | Interpret shared facts; select codes and explanations; validate and anchor suggested edits | Private command evaluation, duplicated semantic proofs, dependence on which other diagnostics were displayed |
| Frontend presentation | Projection, suppression, severity overrides, protocol ranges, edit serialisation | New command semantics, or facts inferred from diagnostic messages and codes |
| Engine adapter | Bounded execution of a declared implementation over structured values in a fixed context | Ambient workspace execution, or a guess for an unknown input |
| Composition root | Install evaluator services and the immutable registry context for native, browser, CLI, MCP, and worker entry points | Hidden per-thread changes to what a canonical query means |

```mermaid
flowchart TB
    R["registry-owned plans<br/>invocation semantics · evaluator route · dependencies"] --> F
    E["shared engines<br/>tcl-cmd-core · expression engine · tcl-regex · declared implementations"] --> F
    C["immutable analysis context<br/>registry generation · bindings · profile · epochs"] --> F
    F["owned semantic facts<br/>values · storage · existence · types · effects · completion"]
    F --> D["diagnostic rules<br/>choose codes and explanations"]
    F --> O["optimisation and codegen proofs<br/>separate transformation proofs"]
    D --> P["frontend presentation<br/>projection · suppression · severity · ranges"]
    O --> P
```

The dependency direction already supports this: `tcl-registry` depends on
`tcl-syntax` and `tcl-cmd-core`, so a registry-owned specialisation can
call the cores directly. `tcl-engine-tclvm` depends on `tcl-compiler`, so
the concrete engine is injected by a composition root above the compiler;
the compiler and the registry see only the execution interface. Existing
hook installation (`pack_hooks::install_host`) shows the direction; its
thread-local, mutable availability is replaced by the context contract in
the evaluation page before it becomes a canonical lattice input.

## One invocation, one context

**Extend the resolver that exists.** `ResolvedInvocation` and
`InvocationSemantics` in `rust/tcl-registry/src/resolved_invocation.rs`
already select the subcommand and form, follow inheritance, keep the source
word kinds and argument offsets, and handle instance invocations. The
value axis is a projection of that resolution, not a second
command/subcommand/form resolver: a `value_transfer_for_call(&[&str])`
over bare strings would drop the evidence the resolver already retains.
`substitution_resolver` in `rust/tcl-registry/src/substitution.rs`, the
newest of the string-keyed resolvers beside `arg_role_resolver` and
`pattern_arg_resolver`, shows the cost: `subst $opt {hello $name}` answers
every kind even where the lattice proves `opt` is `-novariables`, because
a bare string carries no proof. The same contract over
`AnalysisInputs::operand` answers exactly.
There is one argument coordinate system — the resolved invocation's operand
indices — with an explicit mapping back to source words and to lowered
storage places. `Statement::Incr { name, amount, .. }` projects to the
invocation view `incr name ?amount?`; the synthetic loop header the CFG
builder emits for `foreach`, `lmap`, and `dict for` projects to its
declared iteration protocol, and its `container_arg: 0` refers to the
synthetic layout, never to the source var-list.

**Three declaration states.** For each specialisation on a spec, a
subcommand, or a form: *inherited* from the enclosing level, *declared*
explicitly, or *declined* explicitly — "no evaluator for this form". With
`Option<…>` meaning "derive when absent", a more specific form could not
suppress a parent's evaluator; the third state makes abstention a
declaration. Inheritance, declaration, and abstention are resolved once,
with the binding, argv shape, selected form, target profile, and overlay,
into one immutable identity that every axis query interprets.

**Derive only from a descriptor that states the same operation.**
`NativeLowering::CellReadModifyWrite(Increment)` states an operation;
deriving the value projection "read the cell, add, write it back, return
the new value" from it is sound, and a contract test pins the agreement
(`CellReadModifyWrite(u)` on a spec ⇒ the resolved value transfer is the
cell update `u` on the same target). `Traits::DESTROYS_VARIABLE` states
that the named places are unbound afterwards; an *unbind* outcome follows.
`VarWriteTyping::ElementsOf` states a type relationship and nothing about
ordering, padding, write conditions, or the result, so it derives nothing;
`foreach` and `lmap` carry that descriptor beside `LOOP_LIST_HEADER`, and
`HAS_LOOP_BODY` does not establish list iteration. Iteration, destructuring,
and dispatch semantics are explicit declarations.

**Cross-axis consistency is validated once.** A base declaration with a
pure evaluator, a selected form that writes a variable, a subcommand with a
different cell operation, and an overlay that changes argument roles can
each be individually legal and jointly impossible. A cell transfer needs a
matching read/write layout; a body plan must name real body operands; a
declared no-write effect cannot coexist with an applied write plan. An
inconsistent specialisation is quarantined with an author-facing reason and
the generic conservative behaviour is kept; the optimiser and the
diagnostics never pick different fallbacks. Structural validation catches
contradictory metadata; it does not certify that a consistent declaration
is true, and under ruling 3 it is not asked to.

**The analysis context is one value.** Its identity covers every fact that
can change an answer: the effective registry and overlay generation,
command bindings and namespace context, the target semantic profile and
grammar overrides, trace and escape facts, seeds, and any evaluator or
implementation revision not already fixed by the registry identity. It is
carried unchanged through lowering, unit construction, per-function
queries, optimiser consumers, and evaluator calls. Today `compilation_unit`
and `function_lattice` in `rust/tcl-lsp-db/src/lib.rs` resolve the
un-overlaid `db.registry` while the analyser receives `spec_pack_key` and an
overlay; adding the key at the outer query is insufficient if `FnLatticeKey`
or the registry lookup still drops it. The context is interned and carried
by the existing query infrastructure, not duplicated into uncoordinated
keys.

## The interface

The analyser owns seven generic operations, and every specialisation is
composed from them: obtain a proven word value; resolve a storage place;
query the fact that holds at a place before the invocation; enter a
described scope; register a described definition; apply a validated
transition; and emit a fact with provenance. The registry selects and
composes; it never receives `&mut Analyser`. Returned plans and facts are
preferred to callbacks because a plan can be validated, replayed, cached,
and consumed by more than one frontend; a restricted read-only input
interface is the one callback shape, for evaluators that need lazy access
to input facts. The ordered evaluation state the nested-script service
takes is a value the driver owns for one evaluation (§ `expr`: the first
demanding client), not analyser access.

```rust,ignore
// Rust-shaped pseudocode: capabilities and answer shapes, not signatures.
// Every variant says who produces it and who consumes it.

/// Read-only view of what the analyser has proven at this program point.
/// Implemented once by the transfer driver; read by every specialisation
/// and by the expression route's lazy services. Nothing here mutates.
trait AnalysisInputs {
    /// The resolver's projection of this call: binding, selected form,
    /// source words with their substitution kinds, and the operand ↔
    /// source-word ↔ place mapping. Produced from `ResolvedInvocation`.
    fn invocation(&self) -> &ResolvedInvocationView;
    /// One operand's fact in one domain: pending while the solver has
    /// not reached the input, `Top` with the reason when it never will.
    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView;
    /// The storage place a name operand denotes, resolved before any
    /// outcome is composed, or why it cannot be one.
    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason>;
    /// A variable read by *name* at this program point — the expression
    /// route's `var` service — through the same place owner as `place`.
    fn variable(&self, name: &str, domain: FactDomain) -> FactView;
    /// The fact that holds at a place before the invocation runs.
    fn prior_store(&self, place: PlaceRef, domain: FactDomain) -> FactView;
    /// A source word's substitution structure: braced or not, and its
    /// literal runs, `$name` reads, `[…]` regions, and backslash escapes
    /// with spans. What a template-word plan reads.
    fn word_structure(&self, id: OperandId) -> WordStructure;
    /// A body operand as a script region with its base offset and the
    /// frame it runs in. What a body plan and an iteration plan read.
    fn body(&self, id: OperandId) -> Result<BodyRegion, DeclineReason>;
    /// A nested `[…]` script evaluated under the ordered evaluation
    /// state — the expression route's `command` service (§ `expr`).
    fn nested(&self, script: &str, state: &mut EvaluationState) -> EvalAnswer;
    /// A math function with the binding evidence for what the analysed
    /// program calls — the expression route's `call` service.
    fn math_function(&self, name: &str) -> Result<CommandBindingIdentity, DeclineReason>;
    /// The immutable identity every answer is memoised under.
    fn context(&self) -> &AnalysisContext;
}

/// One answer from a fact domain. Produced by the solver that owns the
/// domain; consumed by the specialisation that asked.
enum FactView {
    /// The input is `LatticeValue::Unknown`: the answer stays pending.
    Pending,
    /// One exact value, with the SSA identity that correlates its uses
    /// (`None` for a literal word).
    Exact(ExactValue, Option<ValueKey>),
    /// A bounded set of exact values (`ConstSet`), correlated by identity.
    Finite(Vec<ExactValue>, Option<ValueKey>),
    /// A non-value domain's fact.
    Domain(DomainFact),
    /// The domain cannot answer for this input, for a recorded reason:
    /// `Overdefined`, an escaping place, a dynamic name, a tier that does
    /// not compute it.
    Top(DeclineReason),
}

/// The domains a specialisation can be asked about. One owner each.
enum FactDomain {
    /// `Const` / `ConstSet` over exact strings — the value lattice.
    ExactValue,
    /// Semantic type and shape (`folded_types`, the rich type lattice).
    Type,
    /// Bound, unbound, or may-bound, scalar or array — § Existence.
    Existence,
    /// Integer intervals — `intervals.rs`.
    Range,
    /// Exact prefix, suffix, or length bound of a partly known value.
    Segments,
    /// Colour and sanitiser identity — `taint.rs`.
    Taint,
    /// Internal-representation evidence — § Exact values, types, and
    /// representation.
    Representation,
    /// Which arm or edge a proven operand selects — § Branch facts.
    Selection,
    /// Reads, writes, and observable operations — the effect domains.
    Effects,
    /// The completion codes a construct can take — § `catch`, `try`, and
    /// completion.
    Completion,
}

/// A non-value domain's answer. Produced by the owning solver; read by
/// specialisations through `operand`, `variable`, and `prior_store`.
enum DomainFact {
    Existence(Existence),
    Type { intrep: Option<TclType>, shape: Option<ValueShape> },
    Range { lo: Option<i64>, hi: Option<i64> },
    Segments(SegmentFacts),
    Taint(TaintColour),
    Representation(RepresentationEvidence),
    Selection(SelectionFact),
    Effects(EffectFootprint),
    Completion(CompletionCodeDomain),
}

/// What a registry-owned specialisation supplies.
trait CommandSemantics {
    /// Bodies, scopes, binders, control and completion protocol,
    /// declared structural effects — consumed at construction time.
    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer;
    /// The abstract transfer for one fact domain: a delta the owning
    /// solver validates and applies. A transfer that evaluates — the
    /// `Selection` fact of a case list — charges the same budget as
    /// `evaluate`.
    fn transfer(&self, domain: FactDomain, input: &dyn AnalysisInputs, budget: &mut Budget) -> TransferAnswer;
    /// The exact evaluation over the declared route, under a budget.
    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer;
}

/// The structural plan for one invocation. Produced by `structure`;
/// consumed by lowering, the CFG builder, SSA construction, and the
/// solvers that need the shape.
enum PlanAnswer {
    /// A read-modify-write of one place, derived from
    /// `NativeLowering::CellReadModifyWrite`. Consumed by the solver's
    /// cell transfer, `chain_fold`, `static_loops`, `intervals`, and the
    /// removability and hidden-read rules in `elimination.rs`.
    CellReadModifyWrite {
        target: TargetId,
        operation: CellUpdate,
        amount: Option<OperandId>,
        /// The surfaces on which an absent cell is created
        /// (`safe_on_uninit`): every surface for `append` and `lappend`,
        /// 8.5 onwards for `incr`, none where the operation raises.
        creates_absent: Option<&'static [SpecSurface]>,
    },
    /// A body run in a described scope: produced for `dict with`, `dict
    /// update`, `catch`, `try`, `namespace eval`, `apply`, and every
    /// body-taking command; consumed by scope entry, definition
    /// registration, the generic body-analysis operation, and the
    /// completion protocol.
    Body {
        binders: Vec<Binder>,
        body: BodyPlan,
        reconcile: Reconcile,
        completion: CompletionProtocol,
    },
    /// An iteration protocol: produced for `foreach`, `lmap`, `dict for`,
    /// `for`, `while`, and a vendor loop; consumed by the CFG builder's
    /// loop nodes, the solver's per-element transfer, bounded-loop
    /// enumeration, W240–W242, and the vendor-loop fixture.
    Iterate(IterationPlan),
    /// A case list with its selection contract: produced from
    /// `CaseListSpec` for `switch` and for any command that declares the
    /// switch selection semantics; consumed by the `Selection` transfer,
    /// O112, `switch_body_is_selected`, `static_loops::exec_switch`, and
    /// I231. An arm is `(pattern, body)`; a `None` body is a `-` arm.
    CaseList {
        subject: OperandId,
        arms: Vec<(OperandId, Option<OperandId>)>,
        fallthrough: Option<&'static str>,
        selection: SelectionContract,
    },
    /// A template word — the final argument of a substituting command
    /// with the kinds that run over it: produced from `SubstitutionKinds`
    /// over proven switches; consumed by W102, the two `subst -nocommands`
    /// folders, extract-proc, and the dynamic-name barrier (§ The
    /// template-word plan).
    TemplateWord(TemplateWordPlan),
    /// An ordinary argv invocation with no structure of its own: the
    /// default; consumed by generic lowering.
    NoStructure,
    /// The structure is declared unsupported or cannot be read for this
    /// call: the generic conservative shape is kept.
    Declined(DeclineReason),
}

/// A name a plan binds in the body's scope: from a var-list word
/// (`foreach`), from a value's keys (`dict with`), or declared by the
/// command (`try`'s message and options variables).
struct Binder { name: OperandId, kind: BindingKind }

/// A body operand and the frame it runs in: the enclosing frame, a new
/// procedure frame, a named namespace, or the caller's frame.
struct BodyPlan { body: OperandId, frame: FrameLevel }

/// What a body plan writes back when the body completes: nothing, the
/// bound keys into a dict operand, or the loop's binders.
enum Reconcile { None, WriteBackKeys(OperandId), Binders }

/// One iteration protocol. Produced by an `Iterate` plan; consumed by
/// the CFG builder, the solver, and bounded-loop enumeration.
struct IterationPlan {
    /// The names bound per iteration, in binding order; `foreach {a b}`
    /// is two binders over one iterable, padded with the empty string.
    binders: Vec<Binder>,
    /// A Tcl list operand, a dict operand, a counted loop (`init`,
    /// condition, `next`), a bare condition, or a vendor collection with
    /// its declared cardinality operand.
    iterable: IterableKind,
    body: BodyPlan,
    /// What ends the loop: exhaustion, a false condition, `break`, a
    /// non-normal completion of the body, or the enumeration cap.
    exit: ExitRule,
    /// Whether the binders stay unbound on the zero-iteration path
    /// (`foreach x {} {}` leaves `x` unbound in every release).
    zero_iterations_bind: bool,
    completion: CompletionProtocol,
}

/// The abstract transfer for one domain. Produced by `transfer`;
/// consumed by the solver that owns the domain.
enum TransferAnswer {
    /// No domain-specific model: the driver applies the generic
    /// conservative transfer derived from the declared effects (every
    /// written target widens, everything else is preserved).
    Generic,
    /// Result and per-target semantic types and shapes.
    Type(TypeFacts),
    /// Per-target existence outcomes, indexed by completion path.
    Existence(ExistenceTransfer),
    /// The registry-described operation the interval domain interprets
    /// (`IntegerAdd`, `Length`, `Point`, `Top`) — never the concrete
    /// evaluator applied to an interval.
    Range(RangeModel),
    /// Exact prefix, suffix, or length facts around an unknown segment.
    Segments(SegmentFacts),
    /// Colour flow through writes and the sanitiser identity.
    Taint(TaintTransfer),
    /// Guaranteed construction and conversion of the result's intrep.
    Representation(RepresentationEvidence),
    /// One selected-edge fact per member of a finite subject, joined.
    Selection(SelectionFact),
    /// The effect delta beyond the declared footprint.
    Effects(EffectFootprint),
    /// The completion codes this call can take.
    Completion(CompletionCodeDomain),
    /// This domain declines for this call, for a recorded reason.
    Declined(DeclineReason),
}

/// The exact answer. Produced by every route; consumed by the transfer
/// driver, which validates it before anything is published.
enum EvalAnswer {
    /// An input has not reached a usable fact; the answer stays pending.
    Pending,
    /// No exact fact, for a recorded reason. Never a speculative value.
    Declined(DeclineReason),
    /// The evaluated outcome on the path the exact inputs select.
    Evaluated(InvocationOutcome),
}

struct InvocationOutcome {
    /// How the call completes with these inputs; exact inputs select
    /// exactly one path, and every store below is ordered against it.
    completion: CompletionOutcome,
    /// The command result on that path; `Unavailable` when the route
    /// proves the completion but not the result text.
    result: ExactValueOrUnavailable,
    /// In execution order, over validated targets, aliases reconciled;
    /// on an error completion only the first `written` ran.
    ordered_stores: Vec<StoreOutcome>,
    /// Semantic type and shape facts for the result and each written
    /// place.
    types: TypeFacts,
    /// Every binding, math function, nested command, and context
    /// dependency the answer used.
    evidence: DependencyEvidence,
}

/// How an evaluated invocation completes. Produced by every route;
/// consumed by the solver (which facts publish on which edge), the
/// CFG's exception edges, and the `catch` and `try` plans.
enum CompletionOutcome {
    /// `TCL_OK`: every ordered store ran, the result is the command's.
    Normal,
    /// A non-error code a body plan completes with: `return` with its
    /// `-code` and `-level`, `break`, `continue`, or a numeric code.
    Code { code: CompletionCode, level: u32, result: ExactValueOrUnavailable },
    /// `TCL_ERROR` after the first `written` stores ran — the prefix
    /// rule of § `catch`, `try`, and completion. The message and the
    /// `-errorcode` are exact when the route proves them.
    Error { written: usize, message: ExactValueOrUnavailable, error_code: ExactValueOrUnavailable },
}

enum StoreOutcome {
    /// The place holds exactly `value` afterwards.
    Write { target: TargetId, value: ExactValue },
    /// The place is untouched: value, existence, and representation.
    Preserve { target: TargetId },
    /// The place is unbound afterwards (`unset`, a `DESTROYS_VARIABLE`
    /// command, a body plan's reconciliation).
    Unbind { target: TargetId },
    /// The place may have been written; what is known is bounded.
    MayWrite { target: TargetId, facts: FactBounds },
}

/// A Tcl string preserved byte for byte, with its representation
/// evidence carried separately. Produced by the routes' value ingress;
/// consumed by every value consumer.
struct ExactValue {
    bytes: Vec<u8>,
    /// Numeric classification as an additional fact, never a respelling.
    numeric: Option<ConstValue>,
    representation: RepresentationEvidence,
}

/// A value the route may prove, or may only bound.
enum ExactValueOrUnavailable {
    Exact(ExactValue),
    /// The route proved the path but not the text (an `errorInfo` with
    /// a line number, an options dictionary).
    Unavailable(FactBounds),
}

/// Semantic types and shapes for a result and each written place, in
/// registry vocabulary. Produced by the `Type` transfer and every
/// evaluation; consumed by `type_infer.rs` through `folded_types`.
struct TypeFacts {
    result: Option<TclType>,
    per_target: Vec<(TargetId, TclType)>,
    /// List length, dict key set, or element type when proven.
    shapes: Vec<(TargetId, ValueShape)>,
}

/// What a `MayWrite` or an `Unavailable` still knows: the bounds the
/// non-value domains keep when the exact value is lost.
struct FactBounds {
    existence: Existence,
    intrep: Option<TclType>,
    shape: Option<ValueShape>,
    segments: Option<SegmentFacts>,
    taint: Option<TaintTransfer>,
}

/// Every assumption an answer rests on. Produced with the answer;
/// consumed by the memo key, by re-proof before emission
/// (`require_command_binding`), and by the Explorer's explanation.
struct DependencyEvidence {
    /// `ResolvedConstSubst::command_bindings`, transitively: the head,
    /// every nested command, every math function.
    bindings: Vec<CommandBindingIdentity>,
    /// The places read, with the store version read.
    reads: Vec<(PlaceRef, u32)>,
    /// The route, its implementation identity, and its revision.
    route: RouteIdentity,
    /// The profile axes the answer depended on.
    numerals: Option<NumberSyntax>,
    characters: Option<StringCharacterModel>,
    release: Option<TclVersion>,
}

/// The immutable identity every query and evaluation runs under.
/// Produced once by the composition root; consumed by every memo key.
struct AnalysisContext {
    registry_generation: u64,
    overlay_generation: Option<u64>,
    /// Command bindings as `CommandTrustSnapshot`, and the namespace the
    /// head resolves in.
    bindings: CommandTrustSnapshot,
    namespace: String,
    /// The target profile and its grammar overrides.
    profile: &'static DialectProfile,
    grammar: LexerGrammar,
    /// `TraceInputs` as a value, and the escaping set.
    traced_variables: BTreeSet<String>,
    has_dynamic_variable_trace: bool,
    escaping: BTreeSet<String>,
    /// Call-site seeds and the transfer-summary revision.
    seeds_revision: u64,
    /// Evaluator and implementation revisions the registry does not fix.
    evaluator_revision: u64,
}

/// The request-wide and per-evaluation limits every route charges.
/// Produced by the request; consumed by every route and the cache.
struct Budget {
    fuel: u64,
    depth: u32,
    result_bytes: usize,
    allocation_bytes: usize,
    request_remaining: Duration,
    cancelled: AtomicBool,
}

/// Why an answer is not exact. Each group names the lane of the lift
/// diagram in § The lift over the lattice that records it.
enum DeclineReason {
    // before step 1 · availability
    /// The fact is not computed at this tier, or the function is over
    /// the complexity ceiling: not a negative, and every consumer that
    /// would need it stays silent.
    Unavailable(AnalysisTier),
    // step 1 · resolve
    /// No structural or transfer declaration, or an explicit abstention
    /// (`semantics none`).
    NoSemantics,
    /// A declared `EvalRoute::None`: the specialisation classifies the
    /// command and evaluates nothing, and the evaluation page's
    /// `NoRouteReason` says why.
    NoRoute(NoRouteReason),
    /// Binding validity: the head, a nested command, or a math function
    /// is renamed, aliased, redefined, or in an opaque namespace.
    RebindingSuspected,
    /// The command has structure but no value (`switch`, a body).
    NotAValue,
    // step 2 · inputs
    /// A word is not an exact value at this use: multi-token, `{*}`, an
    /// unresolvable variable, JimTcl `$(…)`.
    NotExact,
    /// The place is `::`-qualified or in the escaping set.
    EscapingPlace,
    /// The place is in `Module::traced_variables`, or every place is
    /// (`has_dynamic_variable_trace`).
    TracedPlace,
    /// The name is computed (`DynamicNameBarrier`): a permanent miss.
    DynamicName,
    /// Duplicate or aliased targets, an array-element base write, or a
    /// trace-visible target: a precision limit, not a soundness rule.
    OverlappingTargets,
    // step 4 · finite sets
    /// More than one distinct SSA value is a set.
    CorrelatedSets,
    /// Combinations or result bytes over the cap.
    TooManyMembers,
    // step 5 · evaluate
    /// The route does not model this form, option, or operation.
    Unsupported,
    /// The operation needs a bound place and existence is not proven,
    /// or no release in the profile completes normally on an absent one.
    UnboundPlace,
    /// The prior value has the wrong shape for the operation: the
    /// program errors at run time, and an error is never a value.
    WrongRepresentation,
    /// The value's bytes are not text where a core needs a string
    /// (`ConstOps::as_str` on the evaluation page); never U+FFFD.
    NotText,
    /// A nested substitution's effects reach an observed input and the
    /// ordered evaluation state cannot own them (§ `expr`).
    StatefulNested,
    /// The answers differ between the profile's releases on one axis:
    /// `Axis` is the evaluation page's name for one `Needs` bit or for
    /// the availability of a command, form, or option.
    ReleaseAmbiguous(Axis),
    /// Fuel, depth, bytes before allocation, the request budget, or
    /// cancellation — distinct from `Unsupported`, never a negative.
    Budget(BudgetLimit),
    /// A regexp search cut short, or an approximate capture.
    Approximate,
    /// A nested query would demand the query being computed.
    Cycle,
    /// The host is unavailable or quarantined; never cached as
    /// `Unsupported`.
    Transient,
    // step 6 · validate
    /// Cardinality, index, overlap, effect, dependency, or limit check
    /// failed: a pack or implementation failure with a notice.
    MalformedAnswer,
}
```

`TargetId` is an invocation-scoped operand that the structural plan has
validated as a place-bearing target; it is never a hook-selected SSA id.
The `OperandId` and target index types are adequate for arbitrary-length
Tcl argv (`usize`, or a decline on overflow), never a truncating `u8`.

**Validation before publication.** A read-only input interface does not
make a returned plan trustworthy. Before any fact is published the driver
checks cardinality (one outcome per declared target, no more), indices,
overlap (duplicate names and aliases resolved to places, with an explicit
join rule for a repeated target), permitted effects against the declared
effect facts, dependency identity, and value limits. A malformed answer is
a pack or implementation failure — an actionable load or evaluation notice
plus a conservative result — and is never an apparently valid partial
lattice update. Slot exhaustion and hook reload are visible for the same
reason: the fixed-slot machinery must not silently become the definition of
the supported catalogue size.

```mermaid
flowchart LR
    I["resolved invocation<br/>+ analysis context"] --> S["registry-owned semantics<br/>structure · transfer(domain) · evaluate"]
    S --> T["generic transfer driver<br/>proven operands · places · prior facts · budget"]
    T --> R1["direct core"]
    T --> R2["expression engine"]
    T --> R3["declared implementation<br/>bounded engine"]
    R1 --> A["answer<br/>Pending · Declined(reason) · Evaluated(outcome)"]
    R2 --> A
    R3 --> A
    A --> V["validate, then join<br/>result · ordered stores · types · evidence"]
    V --> K["consumers<br/>diagnostics · optimiser · editor"]
```

Registry-owned plans, by example:

| Invocation | Plan | Route |
|---|---|---|
| `string range S I J` | resolve the value operands; apply the target-aware shared string and index owners; return the exact value | direct |
| `incr N A` | resolve the cell and the amount; read the proven old value and existence; apply the shared numeric owner; return the new value and one write | direct |
| `expr {E}` | assemble the expression arguments as the command specifies; invoke the shared expression evaluator with lazy, read-only input services | expression engine; a declared implementation only for execution the engine does not model |
| `scan S F A B` | parse the format through its owner; compute the partial conversions; return the count plus ordered write and preserve outcomes | direct, where the core supports the form |
| `regexp P S A B` | the shared regexp command core over our engine; return the count or captures and per-target write or preserve outcomes | direct engine call, no engine setup |
| a private Tcl helper | resolve its declared implementation and dependencies; invoke with structured arguments | bounded engine, when explicitly declared |
| `dict with D BODY` | describe the key binding and the reconciliation around a generic body-analysis operation | structural plan; concrete execution only for a fully supported closed case |
| `subst -novariables {T}` | read the switch operands as proven values and derive the kinds that run; return `PlanAnswer::TemplateWord`: the operand, its `SubstitutionKinds`, whether it is braced, the `[…]` regions inside a braced template that execute in the caller's frame, and the `$name` reads that remain (§ The template-word plan) | structural plan; the direct route materialises the template only when every read it keeps is proven and every script region is closed |

The API succeeds when a command spec chooses these plans while the consumer
stays unchanged. Descriptors stay small and declarative for the common
cases; registry-owned native functions carry irreducible algorithms; a
SpecTcl body is one more way to supply an evaluator through the same
interface, never a second programming language inside descriptors.

## Three permissions

A known result authorises less than it seems to. Knowing that
`[incr n]` is `2` permits propagating `2` into subsequent uses; it does not
permit replacing the substitution with the literal, because the increment
is a required write:

```tcl
set n 1
set result [incr n]
puts $n                    ;# must still print 2
```

Nor does a dead target with pure argument words make a command removable:

```tcl
set unused "\{"
lappend unused value       ;# raises "unmatched open brace in list"
```

Purity and referential transparency do not imply totality. Malformed
arguments, absent variables, traces, and non-OK completion are observable.
The contract therefore names three permissions, each with its own proof:

1. **Record** a proven result or state fact on the normal path.
2. **Substitute** a subsequent use of the value while preserving the producing
   operation and the binding and trace assumptions it was proven under.
3. **Remove or replace** the operation only after proving equivalent
   effects, completion, evaluation order, and result use, including
   implicit returns.

The current `Statement::Incr` deletion predicate in
`rust/tcl-compiler/src/optimiser/elimination.rs` is not generalised to
every transfer; removal of a read-modify-write needs the old value proven
well-formed for the operation, the place proven bound, and no trace — the
totality proof the third permission names.

A computed result carries the binding evidence that produced it.
`ResolvedConstSubst` already retains `command_bindings`; a side map of
types does not replace that. When a constant travels through SSA copies,
joins, calls, or branches and is then emitted into code or into a source
edit, its dependencies are retained and combined, or re-proven before
emission. Codegen's existing `require_command_binding` sites are the shape
of that re-proof for bytecode; a source edit has no accompanying runtime
guard, so a guardable runtime optimisation is not automatically a safe
code action.

## The lift over the lattice

The solver's vocabulary is preserved, and each state forbids a specific
inference:

| Knowledge | Meaning | What must not be inferred |
|---|---|---|
| `Unknown` (pending) | the solver has not obtained usable input evidence yet | runtime dependence, an absent variable, or a reason to erase a previously joined fact |
| `Const(v)` | one exact value under the recorded assumptions | permission to delete its producer, or immutability of the source variable |
| `ConstSet(S)` | a bounded collection of possible values | an arbitrary member, correlation with another independent set, or ordered loop iterations |
| `Overdefined` | this domain cannot represent a sufficiently precise value | proof that no subsequent definition can be constant, or that other domains know nothing |
| decline | this route could not establish an answer, for a recorded reason | a Tcl error, a no-match, a missing cell, or inherently dynamic behaviour |

Determinism of an evaluator establishes repeatability for the same concrete
input; it is not a monotonicity proof. The lift is stated explicitly:

- A pending input keeps the answer pending, which is what optimistic
  iteration needs.
- Exact inputs evaluate through the declared route.
- A finite set is evaluated per member and the answers joined, when exactly
  one *distinct SSA value* is a set: two uses of the same SSA value are
  correlated, and a repeated target is correlated too. Multiple
  independently varying sets decline as a precision limit, not a soundness
  rule; total combinations and result bytes are bounded.
- An unsupported case is top.
- Storage outcomes and auxiliary facts join with their previous information;
  no consumer re-narrows a place already widened by aliases or traces.

An interval transfer cannot call a concrete evaluator on an interval. It
needs a sound abstraction of the operation: a generic interval domain can
interpret a registry-described integer add under the target's overflow
rules, and returns top for an operation with no abstract model even when
exact evaluation exists. Bounded loop simulation can reuse the concrete
evaluator while keeping termination, trip count, and side effects as
separate reasoning. A correlated loop shows why a per-variable set is not a
loop result:

```tcl
set x 0
foreach {a b} {1 10 2 20} {
    incr x [expr {$b / $a}]
}                          ;# x is 20 in every tested release
```

The pairs are `(1,10)` and `(2,20)`; the sets `{1,2}` and `{10,20}` have
lost that relationship, and pairing members arbitrarily would be unsound.
An exact loop result needs ordered iteration simulation or a relational
fact; § Bounded loops and finite sets states both rules.

```mermaid
flowchart LR
    S1["1 · resolve<br/>one invocation view,<br/>binding validity, selected form"]
    S2["2 · inputs<br/>operands and target places<br/>at this program point"]
    S3["3 · pending?<br/>an input still ⊥ → the answer stays ⊥"]
    S4["4 · finite sets<br/>one distinct SSA value:<br/>evaluate per member, join"]
    S5["5 · evaluate<br/>declared route, budget,<br/>exact string values"]
    S6["6 · validate<br/>cardinality · indices · overlap ·<br/>permitted effects · limits"]
    S7["7 · join<br/>result · ordered stores · types · evidence;<br/>never re-narrow a widened place"]
    S1 --> S2 --> S3 --> S4 --> S5 --> S6 --> S7
    S1 -. NoSemantics · NoRoute · RebindingSuspected · NotAValue .-> W
    S2 -. NotExact · EscapingPlace · TracedPlace · DynamicName .-> W
    S2 -. OverlappingTargets .-> W
    S4 -. CorrelatedSets · TooManyMembers .-> W
    S5 -. Unsupported · UnboundPlace · WrongRepresentation · NotText .-> W
    S5 -. StatefulNested · ReleaseAmbiguous · Budget .-> W
    S5 -. Approximate · Cycle · Transient .-> W
    S6 -. MalformedAnswer .-> W
    W["decline lane<br/>affected defs → Overdefined<br/>reason recorded with the fact"]
```

Acceptance for the lift: bottom → constant → set → top progressions, joins
in different predecessor orders, loop back edges, correlated operands,
budget declines, and consistency of value, type, and provenance facts, all
as deterministic fixed-input tests; generator-driven campaigns stay in the
manual tier.

## Bounded loops and finite sets

Two rules of the lift need their own statement, because each is where a
consumer is tempted to read more than the lattice holds: a finite set is
not a relation, and a loop body is not a post-loop fact until every
iteration has run.

### The correlated finite-set limit

Per-member evaluation (step 4 of the lift) is admitted when exactly one
*distinct SSA value* among an invocation's inputs — its operands and the
prior values of its targets — is `FactView::Finite`. Two operands that
read the same `ValueKey` are one distinct value and stay correlated:
`expr {$a * $a}` with `a ∈ {1, 2}` answers `{1, 4}`, never `{1, 2, 4}`.
A repeated target is one place. Two distinct finite inputs decline with
`CorrelatedSets` and the affected definitions widen: the lattice holds no
pairing between the sets, so no member of one may be paired with a member
of the other, and the cartesian product — sound, and bounded by
`MAX_CONSTSET_SIZE` squared — is refused as a precision limit on
combinations and result bytes, not as a soundness rule. The witness is
the loop the lift quotes, and its mirror image:

```tcl
set x 0
foreach {a b} {1 10 2 20} { incr x [expr {$b / $a}] }   ;# 20
set x 0
foreach {a b} {1 20 2 10} { incr x [expr {$b / $a}] }   ;# 25
```

Both loops give `a` the set `{1, 2}` and `b` the set `{10, 20}`. Pairing
members by position answers `{10, 10}` for either loop, where the real
quotients of the second are `20` and `5`; the cartesian product
`{5, 10, 20}` is sound for both and exact for neither; only ordered
enumeration answers `20` and `25`. The rule is in force from slice 2, the
first slice that evaluates over lattice inputs, and slice 3's finite-set
condition tests pin it.

### Bounded-loop enumeration

`static_loops.rs` simulates a `for` loop concretely today:
`summarise_for_statement` runs `init`, the condition, the body, and `next`
through `exec_statement` with its own `StaticValue` lattice, its own
`Incr` arm, and its own `parse_literal_value`, and
`sccp::loop_summary_decision` folds a branch after the loop from the
summarised environment. The design keeps the simulator and makes it a
consumer of concrete semantics, as `LoopEnumeration`:

```rust,ignore
/// Ordered execution of one iteration plan over exact state. Produced by
/// the solver at a loop's pre-header when the plan and the state admit
/// it; consumed by the post-loop lattice, `loop_summary_decision`,
/// W240–W242, and `intervals.rs`.
struct LoopEnumeration {
    plan: IterationPlan,
    /// Every place the loop reads or writes, with its exact value and
    /// existence at the pre-header; an unbound binder stays unbound.
    entry: Vec<(PlaceRef, ExactValueOrUnavailable, Existence)>,
    /// Iterations run before the exit, the exit taken, and the state on
    /// the exit edge: exact values and existence for every place touched.
    iterations: u64,
    exit: ExitRule,
    state: Vec<(PlaceRef, ExactValueOrUnavailable, Existence)>,
    evidence: DependencyEvidence,
}
```

- **Semantics come from the interface.** `exec_statement` applies each
  statement's registry-owned `evaluate` over the enumeration's state —
  the cell update, `expr` through the expression route, an assignment, a
  call with a declared route and no world effect — and `exec_switch`
  consumes the `Selection` fact; the simulator's private `Incr`
  arithmetic, its `parse_literal_value`, and `resolve_switch_subject`
  retire, and the state is exact values with existence rather than
  `StaticValue`, so `foreach x {} {}` leaves `x` unbound and
  `foreach {a b} {1 2 3} {}` leaves `b` the empty string.
- **Bounds.** Iterations are capped at `DEFAULT_MAX_STATIC_LOOP_ITERS`,
  every iteration's statements are charged to the request `Budget`, and
  result bytes are bounded before allocation. A cap is a `Budget`
  decline: nothing is published and the ordinary widened lattice stands.
  A partial enumeration is never a post-loop fact.
- **What is closed.** Every statement in the body, the condition, and
  the `next` script evaluates through a declared route; no barrier, no
  world effect, no write to an escaping or traced place, no dynamic name.
  One unsupported statement declines the enumeration.
- **Exit conditions.** Exhaustion of the iterable, a false condition
  evaluated per iteration through the expression route, `break`, or a
  non-normal completion of the body. `break` and `continue` are
  completion codes the iteration plan absorbs; `return` and an error end
  the enumeration with that completion and the state so far, published on
  that path only — an error in the third iteration leaves the counter at
  `2` on the error edge and nothing on the normal exit.
- **Join.** The enumeration publishes the exit state on the loop's exit
  edge as exact values; inside the loop the header phis widen exactly as
  today, so a body statement still sees `ConstSet` or `Overdefined`, and
  a value that is exact after the loop is not exact within it. A nested
  loop is enumerated as one statement of the outer body under the same
  budget.

Witnesses, identical in every tested release:

```tcl
for {set i 0} {$i < 5} {incr i} {}                            ;# i is 5
set t 0; for {set i 0} {$i < 4} {incr i} {incr t $i}          ;# t is 6
for {set i 0} {$i < 10} {incr i} {if {$i == 3} break}         ;# i is 3
set t 0; for {set i 0} {$i < 4} {incr i} {if {$i == 1} continue; incr t}   ;# t is 3
for {set i 0} {$i < 2.5} {incr i} {}                          ;# i is 3
set i 0; for {} {$i < 3} {} {incr i 2}                        ;# i is 4
set n 0; for {set i 0} {$i < 3} {incr i} {set i [expr {$i + 1}]; incr n}   ;# i is 4, n is 2
catch {for {set i 0} {$i < 5} {incr i} {if {$i == 2} {error x}}}   ;# i is 2 on the error path
foreach x {} {}                                               ;# x stays unbound
foreach {a b} {1 2 3} {}                                      ;# a is 3, b is the empty string
set n 0; foreach x {1 2 3} {if {$x == 2} break; incr n}       ;# n is 1, x is 2
```

Today `tcl opt --profile full` folds `if {$i == 5}` after the first loop
through `loop_summary_decision` and leaves `if {$x == 20}` after the
correlated `foreach` undecided; the enumeration decides both. Consumers:
`sccp::loop_summary_decision`; `bounds_checks.rs`, whose W240–W242
seeding reads `set v INT` and `incr v ?INT?` textually and reads the
iteration plan's bound and step instead; `intervals.rs`, whose
loop-header widening (`MAX_ITERS`) is bypassed for an enumerated loop by
the exact exit state; and the argument-sensitive O103 path, whose callee
re-run enumerates the callee's loops under the call's seeds. Tests: the
`summarise_*` tests in `static_loops.rs` and
`sccp_folds_post_loop_branch_via_static_summary` pin today's answers, and
the eleven witnesses above are the fixed additions, each under every
release found on `PATH`. Migration: slice 12, sequenced after slices 2,
3, and 6, whose exit criterion is that `static_loops.rs` performs no
arithmetic of its own and the eleven witnesses fold.

## Exact values, types, and representation

**Ingress preserves the exact result.** `parse_literal_value` in
`rust/tcl-compiler/src/sccp.rs` begins with `text.trim()`, and the string
fallback trims too; `string range { a } 0 end` returns the three-character
string ` a `, and passing that through the helper would store `a`. A
computed value is a value, not a source token, so the value-ingress API
preserves leading and trailing whitespace, NUL, backslashes, leading-zero
numerals, signed zero, and Unicode exactly. Numeric classification adds a
fact; it never replaces the observable spelling without proof that the
value has that canonical spelling.

**Three separate facts, three joins.** The exact value, the semantic type
or shape, and the representation evidence are distinct, and `TclType` alone
cannot carry the second two:

- A list type does not give its elements; element facts still need the
  exact value.
- Per-target types differ (`scan %d %s`), so one written type cannot
  describe every destructured target.
- A runtime byte array, its Tcl string representation, and the UTF-8 bytes
  of that representation are three things; a computed `binary format` needs
  a lossless materialisation contract before it is emitted anywhere.
- After a join of identical strings produced with different internal
  representations the string stays exact while the representation is
  uncertain; "last written type wins" is not a sound join.
- Replacing a byte-array-producing call by a source literal can change
  representation costs even when the string is equal; the original return
  type does not prove the replacement has the runtime's representation.

The existing dynamic return-type hooks (`ReturnTypeHookId::{Regexp,
Lsearch, Regsub, Scan, Pid}` in `rust/tcl-registry/src/hooks.rs`, resolved
in `rust/tcl-registry/src/return_type.rs`) guarantee an *internal
representation*, and their `None` is authoritative rather than a fall-back
to the static type. They are adapters into the same form semantics, kept
with those guarantees, not superseded by a value table.

**Provenance is never fabricated.** A folded value enters
`value_provenance` with `literal_span: None`, rename abstains on it, and a
computed pattern is explained as computed rather than painted at a token
range it does not have. Existing literal mappings are retained; no rename
edit or token range is invented for computed characters.

## Decline conditions, stated once

The current `incr` arm re-derives its own widening rules; `append` would
re-derive them again; a pack would forget one. The driver states them once,
and every specialisation inherits them:

| Condition | Who decides | Answer |
|---|---|---|
| a required input is pending | the solver | pending |
| a word is not an exact value at this use (multi-token, `{*}`, unresolvable variable, JimTcl `$(…)`) | the driver | decline (`NotExact`) |
| the head's binding is suspect: renamed, aliased to an unknown target, redefined, or in an opaque namespace (`ModuleCommandMutations::trusts`, `trusts_proc_binding`, `redefined_procedures`, `opaque_namespaces`) | binding validity — not an author-trust check | decline (`RebindingSuspected`); a consumer with no whole-module view uses `distrust_all()` |
| the invocation has no semantics declaration, or declares a route of none (`evaluate none`) | the resolver, at step 1 | decline (`NoSemantics`, or `NoRoute` with the evaluation page's `NoRouteReason`); the generic conservative transfer applies, and a declared plan or transfer still answers its own domain |
| the place is `::`-qualified, escaping, or the function has a dynamic trace (`is_externally_mutable` over the `var_observability` escaping set) | the solver, before any transfer runs | the def is `Overdefined` and no transfer re-narrows it (`EscapingPlace`) |
| the place is named in `Module::traced_variables` (`TraceInputs`) | the solver | same (`TracedPlace`) |
| the target is an array-element base write, or the targets overlap, or a target is trace-visible | the driver | decline (`OverlappingTargets`), stated as a precision limit |
| a dynamic key (`incr a($i)`) | `DynamicNameBarrier` | decline (`DynamicName`), not pending: the miss is permanent and `join(prev, Unknown) = prev` would launder a stale element constant |
| the prior value has the wrong intrep for the operation, or the place is unbound and the release's uninitialised behaviour is not proven | the evaluator | decline (`WrongRepresentation`, `UnboundPlace`) — the program errors at run time, and an error is never a value; the error is a completion fact (§ `catch`, `try`, and completion) |
| the answer differs between target releases and the profile names none | the evaluator, comparing all relevant semantic cases | decline (`ReleaseAmbiguous`, naming the axis: `NumberSyntax::unanimous`, `StringCharacterModel`, the leading-zero numeral rule) |
| a core needs a string and the value's bytes are not text | the evaluator (`ConstOps::as_str`) | decline (`NotText`), never a U+FFFD substitution |
| a nested substitution writes a place the ordered evaluation state cannot own (§ `expr`: the first demanding client) | the expression route | decline (`StatefulNested`) |
| the command has structure but no value (`switch`; a body command asked for a result it does not have) | the specialisation | decline (`NotAValue`); the structural plan and the other domains still answer |
| a nested query would demand the query being computed | the driver | decline (`Cycle`), never a depth cap that makes the answer meaningful |
| the fact is not computed at this tier, or the function is over the complexity ceiling | the tier | unavailable (`Unavailable`): not a negative, and every consumer that would need it stays silent |
| a resource cap is hit: output bytes, allocation before it happens, fuel, depth, request budget, cancellation | the route and the budget | decline (`Budget`), distinct from an unsupported case (`Unsupported`) and from a transient host failure (`Transient`), and never an exact negative |
| a regexp search was cut short or a capture is approximate | the regexp owner | decline (`Approximate`), never "no match" |
| the answer fails validation | the driver | decline (`MalformedAnswer`) with a load or evaluation notice; the generic conservative result is kept |
| the statement is a `Barrier` or `UpFrame` | the solver | every tracked value widens, as today |

`incr` of `010` is the release row in one line: it answers 11 under
Tcl 9.1 and 9.0 and 9 under 8.6, 8.5, and 8.4, so a profile that names no
release declines.

## Storage-writing commands

A transfer is a result *and* a change to storage, and the two are
independent. The concrete shapes, observed under every tested release:

```tcl
set a before; set b before
regexp {(x)(y)} zz a b       ;# result 0; a and b remain before
scan {12 nope} {%d %d} a b   ;# result 1; a = 12, b remains before
lassign {first second extra} a a
                             ;# result extra; a = second
set d {a 1}
dict with d {incr a; set result done}
                             ;# result done; d = {a 2}
```

- **Preserve is a first-class outcome.** No match means *preserve*, and a
  partial `scan` writes some targets and preserves the rest. A vector of
  optional values cannot say "leave this alone", and cannot deliver the
  precision W210 needs for the no-match case. The shape to copy exists:
  `tcl_cmd_core::regex::RegexpResult::Count` carries an independent count
  and `assign: Option<Vec<…>>`, where `None` means the match variables are
  untouched.
- **Result and cell are separate values.** A single "new value" cannot
  describe a list pop that returns the removed element and writes the
  remainder, or a body command whose result is the body's and whose
  write-back is a reconciliation.
- **Duplicate targets resolve to places first.** `lassign … a a` writes
  `a` twice in order; the outcomes are composed after resolution, in
  execution order.
- **Body commands are structural plans.** `dict with` and `dict update`
  bind keys, run a body, and reconcile; their implementation in
  `rust/tcl-vm/src/cmd_dict.rs` is those three steps. Initial key binding
  is a useful projection of a constant dict; it never stands for the whole
  command.
- **Pending is not unbound.** `incr` on an absent variable behaves by
  release (8.4 raises `can't read "x": no such variable`; from 8.5 the
  cell is created and the result is the amount), while `append` and
  `lappend` create it in every release. The uninitialised case therefore
  needs an existence proof and, for `incr`, the selected release; no old
  value is manufactured from bottom. § Existence owns the proof, and the
  route — step 5 of the lift — is what declines with `UnboundPlace` when
  the proof is absent; the input step never manufactures one.
- **An interpreter error is not an atomic rollback.** With `a` a scalar
  and `b` an array, `catch {lassign {new second} a b}` fails with code 1
  and leaves `a` equal to `new`. A transfer that declines on error and
  preserves all incoming state would be wrong if that state reaches a
  handler; one that treats all writes as successful would be wrong too.
  Storage outcomes are therefore indexed by completion path under the
  prefix rule of § `catch`, `try`, and completion: the ordered stores
  before the failing step ran, the rest did not, and no exact fact
  escapes a path the analyser does not model. Applying a *validated
  analysis result* atomically says nothing about the *analysed command*
  having transactional semantics.

## Existence

Existence is the third lattice rung: a flow-sensitive bound/unbound fact
per place, owned by the solver, fed by storage outcomes, and consumed by
W210, W211, W213, W214, O108, O109, I230, O101, and S100. Today the fact
is two
whole-body scans — `scan_defined_and_unset` in `sccp.rs` collects every
assigned name and every name a literal `unset` names, recognising `unset`
by its spelling — and `existence_constant_branches` folds `[info exists
X]` from them as a post-pass outside the fixed point, so a name assigned
anywhere in the body never folds, and W210's read-after-`unset` and
W213's `unset`-of-a-killed-version answers come from def-use chains
(`whole_unset_names`, `phi_can_undef`) rather than from a fact the solver
holds.

**The lattice.** Per place, per SSA version of the binding:

```rust,ignore
/// The existence fact at one place. Produced by the solver from storage
/// outcomes; consumed through `FactDomain::Existence`.
enum Existence {
    /// ⊥: the solver has not reached the place.
    Pending,
    /// Provably no binding.
    Unbound,
    /// Provably bound, with the kind when it is proven.
    Bound(BindingKind),
    /// ⊤: bound on some path and not on another, or unknowable here.
    MayBound,
}

/// What a bound place is. `array exists` distinguishes them; a parameter
/// is always `Scalar`; `Either` is the join of the other two.
enum BindingKind { Scalar, Array, Either }

/// The per-target existence delta a specialisation returns, indexed by
/// completion path (§ `catch`, `try`, and completion). Produced by
/// `transfer(FactDomain::Existence)` or derived from the storage
/// outcomes of an evaluation; consumed by the existence solver.
struct ExistenceTransfer { paths: Vec<CompletionPath> }

/// One completion path's storage consequences.
struct CompletionPath {
    /// Normal, a completion code, or an error — the kind, not an instance.
    completion: CompletionCodeDomain,
    /// Per target, in execution order; a target absent here is preserved.
    outcomes: Vec<(TargetId, ExistenceOutcome)>,
}

enum ExistenceOutcome {
    /// The place is bound afterwards — a `Write`.
    Bind(BindingKind),
    /// Untouched — a `Preserve`.
    Preserve,
    /// Unbound afterwards — an `Unbind`.
    Unbind,
    /// A `MayWrite`: joined with the prior fact.
    MayBind(BindingKind),
}
```

The join is the lattice's: `Pending` is the identity; equal facts stay;
`Bound(Scalar) ⊔ Bound(Array)` is `Bound(Either)`; `Unbound ⊔ Bound(_)`
and anything with `MayBound` are `MayBound`. The transfer per storage
outcome: a `Write` binds — the scalar place for a scalar write, and for
an element write the element as `Scalar` and its base as `Array`; a
`Preserve` leaves the fact; an `Unbind` makes the named place `Unbound`
and, for an element, leaves the base bound; a `MayWrite` joins the prior
fact with `Bound`, so an `Unbound` place becomes `MayBound` and a bound
one stays bound. An unbind of an absent place is an error, not a
transition: `unset x` on `Unbound` completes with `Error` and writes
nothing; `unset -nocomplain x` completes normally and leaves `Unbound`.

**Entry state.** A procedure's parameters enter `Bound(Scalar)`; every
other local enters `Unbound`. A scope-alias local (`global`, `variable`,
`upvar`, `namespace upvar`, `Traits::CREATES_SCOPE_ALIAS`) enters
`MayBound` at its declaration, because its existence tracks the linked
cell; a `TclOO` method's instance variables enter `MayBound`; in the
document's initial global frame a registry special variable enters
`Bound(Scalar)` when `startup_read_facts` says it is initially bound and
`MayBound` otherwise, a name another procedure may write
(`scan_module_global_names`) enters `MayBound`, and the rest enter
`Unbound`; a `::when::*` handler in iRules enters every cross-event
variable (`ConnectionScope::cross_event_defs`) as `MayBound`, which is
the rule `drop_cross_event_existence_folds` applies to the post-pass's
output today. A `Barrier` or `UpFrame` statement sets every place to
`MayBound`, as it sets every value to `Overdefined`. The dynamic-name
barrier is flow-sensitive here: a dynamic write
(`DynamicNameBarrier::writes`) turns every `Unbound` place `MayBound`
from that statement on, and a dynamic destroy (`destroys`) turns every
`Bound` place `MayBound` from that statement on; today both blind the
whole function.

**The release rule for an absent cell.** A cell update declares the
surfaces on which an absent cell is created (`safe_on_uninit`, the plan's
`creates_absent`): `append` and `lappend` create it in every release;
`incr` creates it from 8.5 and raises under 8.4. Under every tested
release:

```tcl
incr fresh                 ;# 8.4: can't read "fresh": no such variable
                           ;# 8.5, 8.6, 9.0, 9.1: 1, and fresh is bound
incr fresh 2               ;# the same split: 2 from 8.5, and bound
incr arr(k)                ;# the same split for an element; arr is bound too
append s foo               ;# every release: foo, and s is bound
lappend l foo              ;# every release: foo, and l is bound
regexp {(x)(y)} zz a b     ;# every release: 0; a and b stay unbound
scan {12 nope} {%d %d} a b ;# every release: 1; a bound, b unbound
unset nosuch               ;# every release: can't unset "nosuch": no such variable
unset -nocomplain nosuch   ;# every release: no error, still unbound
set p 1; set q 2; unset p nosuch q   ;# every release: error; p unbound, q still 2
set arr(k) 1; unset arr(k) ;# every release: arr bound, arr(k) unbound
set x 1; unset x; set x 2  ;# every release: x bound
foreach x {} {}            ;# every release: x unbound
```

On the normal path of `incr fresh` the place is `Bound(Scalar)` and the
value is the amount, because only the releases that create the cell
complete normally; under a profile whose every release raises, the
statement has no normal successor and the value declines with
`UnboundPlace`; the possible error under 8.4 is a completion fact of the
statement in every profile that includes it. The `unset p nosuch q` line
is the prefix rule for unbind outcomes.

**`info exists` and the guard narrowing.** `[info exists x]` and
`[array exists x]` are the intrinsics `IntrinsicId::InfoExists` and
`IntrinsicId::ArrayExists`, recognised through the resolved invocation
(`existence_query.rs`) and evaluated through the expression route's
`nested` service as a read of `FactDomain::Existence`: `Bound(_)` answers
`1` to `info exists` and `Bound(Array)` answers `1` to `array exists`,
`Unbound` answers `0` to both, `Bound(Scalar)` answers `0` to `array
exists`, and `Bound(Either)` and `MayBound` leave the condition
undecided. The condition then decides inside the fixed point like any
other proven condition, so the branch fact carries applied reachability
and `executable_blocks` drops the dead arm: the post-pass, its second run
in `emit_existence_constant_branch_diagnostics`, and the reachability
gate of `emit_constant_branch_diagnostics` become one path. Where the
fact is `MayBound` the condition refines its edges instead: the true edge
of `[info exists x]` carries `Bound(Either)` for `x`, the false edge
`Unbound`, and `![info exists x]` swaps them — the edge refinement of
§ Predicate refinement in the existence domain, which
`collect_existence_guards` and `existence_exempt` implement today by
dominance and which stays byte-identical in effect.

**Consumers.**

- **W210** (`emit_read_before_set_diagnostics`,
  `record_chain_w210_uses`, `emit_return_phi_undef_w210`,
  `emit_provably_unset_w210`): a value read at a place whose fact is
  `Unbound` reports; a read at `MayBound` reports, as today's
  may-undefined chain does; a read at `Bound(_)` is silent. The version-0
  origin, `whole_unset_names`, `phi_can_undef`, the existence guards, and
  the private `regexp` / `scan` no-match prover are all readings of this
  one fact: no match is a `Preserve`, so the prior fact stands. A use
  that safely initialises (`use_site_safe_initialises`) is a cell update
  whose `creates_absent` admits the release.
- **W213** (the `command == "unset"` site in `record_chain_w210_uses`):
  an `unset` of an `Unbound` place reports as definite, of a `MayBound`
  place as "may not exist", of a `Bound` place not at all; the
  `-nocomplain` form never reports, because its completion domain has no
  error.
- **W211** (`emit_unused_variable_diagnostics`): an existence read —
  `info exists`, `array exists`, an `unset`, a `DESTROYS_VARIABLE`
  command — is a use of the binding. Today
  `proc p {} { set x 1; if {[info exists x]} { puts yes } }` reports W211
  on `set x 1`, and `tcl opt --profile full` deletes the store, so the
  optimised procedure prints nothing where the original prints `yes`.
- **O108 and O109** (`elimination.rs`): a store is removable only when
  no value read *and no existence read* of its version remains; an
  unbind statement is never removed, because the error on an absent
  place and the binding's disappearance are its effects. Today
  `proc p {} { incr n; if {[info exists n]} { puts yes } }` loses its
  `incr n` to O109 under `--profile full`.
- **I230 and O101**: the existence branch fact is an ordinary
  `ConstantBranch` with applied reachability; the post-pass extension in
  `FunctionUnit::build` and its cross-event retention retire with it.
- **S100** (`shimmer/`): an unbind is not a typed value, so a phi that
  merges a bound version with an unbound one is not a representation
  merge; today `set x 1; if {$c} { unset x }; puts $x` reports S100
  beside its W210.

**Availability across tiers.** The fact is a deep-tier fact. A fast-tier
request, a function over the complexity ceiling (`FunctionUnit`'s
trivial lattices), and a consumer without SSA read
`FactView::Top(DeclineReason::Unavailable)`, which is neither `Unbound`
nor `MayBound`: every consumer above stays silent on it. An empty
reachable-block set has no universal meaning outside its producing
analysis — missing or deferred analysis, no normal successor, and a
proved unreachable branch are different answers, and each is typed as
such. On the exception
edges of the faithful-exceptions build a handler's entry state is the
join of the throw sources' prefix states; in the default build a `catch`
body's summarised defs are `MayBind` outcomes and its result and options
variables `Bind(Scalar)`. Across a call, the callee's transfer summary
(§ Proc-level transfer summaries) supplies the outcomes for the caller
places it names; an unknown callee is a barrier.

**Tests.** The `info_exists_*` and `emit_cfg_ssa_diagnostics_w210_*` /
`w213_*` tests in `rust/tcl-compiler/src/analyser/diagnostics/tests.rs`
and the `existence_fold_abstains_*` and `upframe_body_models_*` tests in
`sccp.rs` pin today's answers and stay byte-identical; the fixed
additions are the release table above, `set x 1; unset x; info exists x`
deciding `0`, `set x 1; if {$c} { unset x }` giving W210 and a definite
W213 on a following `unset x`, the two O109 refusals, and the S100
silence. Migration: slice 8, sequenced after slice 5 and before slice 6,
whose exit criteria are that `sccp.rs` recognises no command by
spelling, `existence_constant_branches` and `scan_defined_and_unset` are
deleted, and `emit_provably_unset_w210` reads the fact.

## The template-word plan

A substituting command reads through its final argument: `subst {hello
$name}` reads `name` out of a braced word that every other command would
treat as literal text. Which kinds run is a per-call answer —
`SubstitutionKinds` in `rust/tcl-registry/src/substitution.rs`, computed
by `subst_substitutions` over the switch words and answered as `ALL` for
a call it cannot read — and four consumers derive the rest by their own
bracket walks: W102 (`emit_w102_subst_injection` and
`substitution_narrowing_switches` in `analyser/diagnostics/security.rs`),
the two template folders (`eval_subst_nocommands_body` in
`lowering/mod.rs` and `extract_subst_nocommands_template` in
`specialise_factories.rs`, both gated on `SUBST_NOCOMMANDS_KINDS`),
extract-proc's literal cut and same-frame regions (`literal_word_holes`
in `rust/tcl-lsp-core/src/refactor/extract_proc.rs` and
`push_substituted_commands` in `refactor/mod.rs`), and the dynamic-name
barrier (`template_word_is_substituted` in `dynamic_names.rs`, which
reads only the trait). The plan is the one fact they share:

```rust,ignore
/// The final argument of a substituting command, with what runs over
/// it. Produced by the registry's `subst` specialisation from its option
/// rows and the proven switch operands; consumed by W102, the two
/// template folders, extract-proc, and the dynamic-name barrier.
struct TemplateWordPlan {
    /// The template: the call's final argument, as the resolver defines
    /// it; every earlier word is a switch.
    operand: OperandId,
    /// The kinds that run, from the switches' proven values: a `Const`
    /// switch reads exactly as its literal spelling; a `ConstSet` of
    /// switches joins per member, a kind on in any member being on; an
    /// unproven switch answers `SubstitutionKinds::ALL`.
    kinds: SubstitutionKinds,
    /// Whether the template is a braced word: only then are its `$name`
    /// and `[…]` source text the analysis can read. Any other word
    /// reaches the command already substituted once, and `dynamic` says
    /// so.
    braced: bool,
    dynamic: bool,
    /// The `[…]` regions of a braced template, in template order, when
    /// `kinds.commands`: each is a script that runs in the caller's
    /// frame with its own resolved invocations, whose ordered stores
    /// apply in that order under the ordered evaluation state.
    script_regions: Vec<ScriptRegion>,
    /// The `$name`, `${name}`, and `$arr(k)` reads of a braced template
    /// outside its script regions, in order with spans, when
    /// `kinds.variables`; a read inside a region belongs to that
    /// region's script and substitutes even when `kinds.variables` is
    /// off.
    reads: Vec<VariableRead>,
    /// The backslash escapes that materialise, when `kinds.backslashes`.
    escapes: Vec<Span>,
}

struct ScriptRegion { span: Span, script: BodyRegion }
struct VariableRead { span: Span, name: String, element: Option<String> }
```

The plan is produced through `AnalysisInputs::operand` for the switches
and `AnalysisInputs::word_structure` for the template, so a computed
switch with a proven value reads exactly as the literal spelling does:
`set opt -novariables; subst $opt {hello $name}` has `kinds.variables`
off, where the bare-string resolver answers every kind. The kinds and
the option grammar stay the registry's: the negated family
(`-nobackslashes`, `-nocommands`, `-novariables`) in every release, the
positive family (`-backslashes`, `-commands`, `-variables`) from Tcl 9.1,
and mixing them an error. Under every tested release, except where a
line says otherwise:

```tcl
set b 5
subst -novariables {a$b[set b]}       ;# a$b5 — the bracket still runs
subst -nocommands {a$b[set b]}        ;# a5[set b]
subst -novariables {x[expr {$b+1}]}   ;# x6 — $b inside the region substitutes
subst -novariables {a[string length $b]}          ;# a1
subst -novariables -nocommands {a$b[set b]\x41}   ;# a$b[set b]A
subst -nobackslashes {a\tb}           ;# a\tb, four characters
subst {a\$b[set b]}                   ;# a$b5 — the escape protects the $
set opt -novariables; subst $opt {hello $name}    ;# hello $name
set t {a$b}; subst -nocommands $t     ;# a5 — a dynamic template
proc p {} {set c 1; subst {[set c 2]}; return $c}; p   ;# 2 — the caller's frame
proc q {} {set c 1; set r [subst -novariables {[incr c]}]; list $r $c}; q   ;# 2 2
subst -variables {a$b[set b]}         ;# 9.1: a5[set b]; 9.0 and 8.x: bad option
subst -backslashes {a$b[set b]\x41}   ;# 9.1: a$b[set b]A; 9.0 and 8.x: bad option
subst -nocommands -variables {a$b}    ;# 9.1: cannot combine positive and negative options; 9.0 and 8.x: bad option
```

Under a profile that does not reach 9.1 the positive family is a
completion fact — the call errors — and under a profile that spans both
sides of 9.1 the plan declines with `ReleaseAmbiguous`, so W102 falls
back to every kind, as it does for an unreadable call today.

Each consumer reads the fields it needs and nothing else:

| Consumer | Reads | What it stops doing |
|---|---|---|
| W102 | `kinds`, `dynamic`, and the narrowing advice from the option rows | asking the bare-string resolver, so a proven switch word narrows the message |
| `eval_subst_nocommands_body`, `extract_subst_nocommands_template` | `kinds == SUBST_NOCOMMANDS_KINDS`, `braced`, `reads` — every read must be in the const map — and `escapes` | re-segmenting the `[subst …]` text and matching the head `subst` by spelling |
| extract-proc's literal cut and same-frame regions | `kinds.variables` to keep or cut the braced word; `script_regions` as the same-frame regions | `push_substituted_commands`' own `[` walk over the braced word |
| the dynamic-name barrier | `dynamic && kinds.variables` to set `reads`; `script_regions` for the region scan | reading only `PERFORMS_SUBSTITUTION`, so `subst -novariables $t` stops blinding every read |

The direct route materialises a template only when the plan is closed:
every read is proven, every script region evaluates through a declared
route with no world effect, and the kinds are exact; `subst_nocommands`
in `rust/tcl-compiler/src/subst_nocommands.rs` is that materialisation
for the commands-off case, and it consumes `reads` rather than scanning
the template again. Tests: the `tp_*` and `fp_*` tests in
`substitution.rs` stay the kinds oracle; `w102_*` in
`analyser/diagnostics/tests.rs`, `proc_subst_nocommands_*` in
`lowering/mod.rs`, `detects_*` and
`rejects_factory_with_computed_subst_switch` in
`specialise_factories.rs`, and
`tp_a_substituting_call_can_switch_its_variable_reads_off` with the
`tp_a_substituted_bracket_*` tests in `extract_proc.rs` pin today's
behaviour; the fixed additions are the fourteen witnesses above as plan
fixtures and the proven-switch W102 narrowing. Migration: slice 5, with
the structural plans, whose exit criterion gains "the four consumers read
`TemplateWordPlan`; none walks a template word".

## `expr`: the first demanding client

`expr` is the first client that cannot be reduced to a suffix of literal
operands, so it validates the interface early. There are three layers, and
the seam for the third already exists:

1. **Word evaluation** obtains the argument values under the source's
   braced, quoted, bare, or expanded substitution rules.
2. **The registry-owned `expr` specialisation** assembles those arguments
   into one expression as the command specifies: a braced argument is the
   expression text; a quoted argument has had substitutions performed
   before the engine sees it; several arguments concatenate.
3. **The shared expression engine** — `tcl_syntax::expr::eval` with its
   `ExprOps` (`var`, `command`, `call`) in `rust/tcl-syntax/src/expr/eval.rs`
   — evaluates lazily, using analysis services for proven variable values
   and supported nested calls.

The compiler's `FoldOps` in `rust/tcl-compiler/src/tcl_expr_eval.rs`
already adapts that engine: it supplies an environment, rejects command
substitutions, and calls the shared math-function dispatcher. Two limits
of the adapter are corrected rather than inherited. Its `eval_with_config`
ends in `to_number`, and its public `TclValue` has numeric variants only,
so it cannot fold `expr {"x"}` — which is the string `x` — or a
string-valued ternary; the analysis result boundary carries the engine's
full value. And it declines a command substitution it reaches, so
`expr {[string length $s] * 2}` never folds even when `s` is known; the
`command` service resolves the nested invocation through the registry's
semantics instead. What the adapter already does right is kept: it stops
at a short-circuit, so the substitution in the first line below is never
reached, and the contract preserves that laziness.

```tcl
expr {0 && [error never]}   ;# 0: the right operand is never reached
set a alpha; set b beta
expr {$a == $b}             ;# 0
expr "$a == $b"             ;# error: invalid bareword "alpha"
```

Resolving words as structured values avoids building a script by
interpolating values that contain Tcl syntax.

**Nested substitutions run under an ordered evaluation state.** Under
every tested release:

```tcl
set x 1
expr {$x + [incr x] + $x}    ;# 5, and x becomes 2
set x 1
expr {0 && [incr x]}         ;# 0, and x remains 1
set x 1
expr {$x + [set x 10] + $x}  ;# 21, and x becomes 10
set x 1
expr {[incr x] + [incr x]}   ;# 5, and x becomes 3
set x 1
expr {$x ? [incr x] : [incr x 10]}   ;# 2, and x becomes 2
set x 1
expr "$x + [incr x]"         ;# 3, and x becomes 2: the word substituted first
set x 1
catch {expr {[incr x] + [error mid]}}   ;# 1, and x is 2 on the error path
```

A callback that reads every variable from one incoming map is wrong for
the first and third; one that evaluates all substitutions first is wrong
for the second. The state that makes them right is a value the driver
owns for one evaluation:

```rust,ignore
/// The ordered evaluation state of one expression or one word
/// substitution. Produced by the driver at the start of the evaluation;
/// consumed by the `variable` and `nested` services; discarded at the
/// end, its writes becoming the invocation's ordered stores.
struct EvaluationState {
    /// Writes applied so far, in order: a read of a place consults these
    /// before the analysis inputs at the program point.
    writes: Vec<(PlaceRef, StoreOutcome)>,
    /// The evidence every nested outcome contributed.
    evidence: DependencyEvidence,
    /// Which nested operations this evaluation admits.
    policy: NestedPolicy,
}

enum NestedPolicy {
    /// Only effect-free nested invocations (`string length`, a pure
    /// route): the policy of a branch condition and of slice 3.
    EffectFreeOnly,
    /// Nested invocations whose ordered stores name only places the
    /// state can own — local, not escaping, not traced, not dynamic —
    /// and whose completion is exact.
    LocalWrites,
}
```

The state's scope is one evaluation — one `expr` invocation, one branch
condition, or one word whose substitutions run before its command — and
the ordering is the shared walker's: `tcl_syntax::expr::eval` visits
operands left to right and owns `&&`, `||`, and `?:`, so which services
run and in what order is never decided here, and a quoted argument has
had its substitutions performed by word evaluation, under the same
state, before the engine parses it. A nested invocation is admitted when
the policy admits it: an effect-free route always; under `LocalWrites`, a
route whose outcome's `ordered_stores` all name places the state can
own, applied to `writes` in order so that the next `variable` read sees
them, with the outcome's evidence merged. Order is proven by
construction — every read goes through `variable`, which consults
`writes` first, and every write enters `writes` through a validated
outcome — and the driver rejects any nested outcome whose evidence names
a place outside the admitted set, which is the `StatefulNested` decline.
An error completion inside the expression ends the evaluation with that
completion and the writes so far as the statement's stores, so the last
witness above leaves `x` at `2` on the error path and never assigns the
result. The evaluation declines when a nested invocation has a world
effect or an unknown completion domain, when a read names a place another
admitted write may alias, when the nested depth or the request budget is
exhausted, and when a nested query would need the lattice being computed
(`Cycle`). What the state is not: the mutable analyser, the program's
interpreter, or a store that outlives the evaluation. Tests:
`short_circuit_logical` in `rust/tcl-syntax/src/expr/eval.rs` pins the
walker's ordering; `command_substitution_is_none` in `tcl_expr_eval.rs`
pins today's refusal and flips when the `nested` service lands; the seven
witnesses above are the fixed additions. Migration: `EffectFreeOnly`
lands in slice 3; `LocalWrites` is slice 9, sequenced after slices 3 and
5, whose exit criterion is the seven witnesses through `tcl opt` and the
memoised path.

**Math functions are bindings.** Binding validity covers every command
implementation actually used, transitively:

```tcl
rename ::tcl::mathfunc::abs ::tcl::mathfunc::saved_abs
proc ::tcl::mathfunc::abs {x} {return 99}
expr {abs(-2)}             ;# 99 from 8.5
```

Checking only `expr`, or retaining a builtin `abs` in a private engine,
does not prove what the analysed program calls. A successful answer carries
evidence for the math functions and nested commands it used, alongside
variable observability and the target's numeric and word rules; a
user-defined math function or an ambiguous binding is a
`RebindingSuspected` decline.

**Three operations, not one.** Exact evaluation, partial simplification,
and algebraic regrouping are different operations with different proofs:

| Input | Useful reduction | Required justification |
|---|---|---|
| `expr {2 + 3 + $x}` | `expr {5 + $x}` | fold the closed `2 + 3` subtree; keep the remaining operation |
| `expr {$x + 2 + 3}` | `expr {$x + 5}` only when proven safe | reassociation across an unknown term is not valid for arbitrary doubles |
| `expr {(2 + 3) * ($x + (4 + 5))}` | `expr {5 * ($x + 9)}` | fold two independent closed subtrees without regrouping the dynamic term |
| `expr {$x * 0}` | usually retain | dropping `$x` can remove errors and effects or change numeric semantics |
| `string cat {prefix:} {abc} $x {:} {suffix}` | merge the two constant runs around `$x` | registry-declared concatenation semantics; exact strings, word evaluation, quoting preserved |

```tcl
set x 10000000000000000.0
expr {$x + 1 + 2}  ;# 10000000000000002.0
expr {$x + 3}      ;# 10000000000000004.0
```

`instcombine_expr_typed` in
`rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs` rewrites
`$x + 1 + 2` to `$x + 3` even with a present, empty `OperandTypes` context,
because `reassociate_node` does not receive the numeric context; the
production expression pass calls it. Reassociation must consume the
relevant type and target proofs, and integer-only reasoning must still
account for overflow and bignum behaviour, errors, effects, and evaluation
order. The partial rewriter consumes the same semantic facts as the
evaluator and returns a typed residual with its dependencies and
transformation proof; `Residual(expr)` is not `Const(rendered_expr)`, and
the dynamic operand is never replaced by a dummy to run an engine — that
yields one sample, not a symbolic result. Partial simplification extends
`expr_simplify.rs` and `optimiser/propagation.rs`, which already substitute
known operands and simplify; it is not a second partial evaluator inside
the registry driver.

**Partial facts after the exact value is lost.** A non-constant integer can
still have an interval; a non-constant list a known length and element
type; a string assembled around an unknown segment an exact prefix, suffix,
or length bound. Those facts support diagnostics and further reduction
without pretending the whole value is known, and a constant textual result
does not prove a representation or the absence of effects. Existing
domains carry them where they suffice; bounded segment and residual facts
are added only for concrete consumers, with caps, joins, invalidation, and
loop widening specified, and on complexity exhaustion residual precision is
dropped without changing semantic truth.

**Constant → unknown → constant.** With no aliases, traces, or external
observability:

```tcl
set x 3
incr x 2
set x $request_value
set x 7
```

The definitions have exact values 3, then 5, then an unproven value, then
7. The unknown input does not retroactively invalidate the earlier
definitions, and the final assignment is a new constant definition, not a
violation of monotone convergence. An alias write, trace, callback,
namespace mutation, or opaque call may invalidate knowledge without an
explicit `set`; that loss uses the existing place, effect, and observability
owners, and with today's whole-function conservative barriers precision can
be lost earlier than the runtime mutation. A query over an SSA use or a
place *at the requested program point* returns the value-domain answer,
the relevant type, shape, and range facts, dependencies, and bounded
explanation evidence — why a fact ceased to be exact: dynamic input,
conflicting incoming values, alias write, trace, binding uncertainty,
unsupported semantics, release ambiguity, or resource decline — linked to
the responsible statement or edge where known. Explorer explanations and
rules consume that; they never reconstruct it from warning messages or by
comparing solver iterations, and no append-only log of solver events is
kept.

Acceptance for `expr`: multi-argument forms, braced versus quoted
arguments, short-circuit operators and ternaries, strings that look like
code, math-function rebinding, nested pure substitutions, errors, bignums,
and target release ambiguity, with direct-expression and engine entries
measured separately.

## Branch facts

Three things are recorded separately, because the CFG does not always
contain the edge a selection names:

```mermaid
flowchart LR
    P["proven condition<br/>operand values and the registry's<br/>selection semantics"] --> E["selected edge<br/>which successor or which arm"]
    E --> A["applied reachability<br/>executable_blocks · executable_edges,<br/>only when the CFG has the edge"]
    P -. explanation .-> D["I230 · I231 · O100 hints"]
    E -. structured-arm fact .-> S["O112 · switch_body_is_selected ·<br/>static_loops::exec_switch"]
    A -. reachability .-> R["O107 · W210 · taint · shimmer"]
```

Today `FunctionUnit::build` appends existence-derived constant branches to
`sccp.constant_branches`, but those post-pass facts do not update
`executable_blocks`, so `emit_existence_constant_branch_diagnostics` calls
the same semantic helper again to learn which kind it has. Sharing a helper
prevents algorithm drift; it does not give consumers one complete stored
fact. The stored fact says which of the three it is, and emission never
reruns the proof. With the existence rung (§ Existence) the existence
condition decides inside the fixed point, so the post-pass and its second
run retire and the three kinds are the only distinction left.

### `if`, `elseif`, `while`, `for`

Conditions are parsed expressions, so `evaluate_branch` already binds
`$var` operands from the lattice. Two gaps close through the same
interface:

- **Command substitutions inside a condition.** `ExprNode::Command` is
  rejected by the evaluator, so `if {[string length $acc] == 6}` never
  decides even when `acc` is constant. The engine's `command` service
  resolves a nested invocation through the registry's semantics, lazily,
  under the branch's use versions and the nested-substitution policy
  above; this reaches every pure specialisation at once — `info exists`,
  `string is integer`, `dict exists`, `lsearch`, `regexp` without match
  variables.
- **Finite-set operands.** `env_from_uses` binds only a single `Const`.
  When exactly one distinct SSA value is a `ConstSet`, the condition is
  evaluated per member: all true → taken, all false → not taken, mixed →
  open.

Loop-carried values still widen at the header phi. The three disagreeing
`incr` models — `static_loops::exec_statement`, `intervals::transfer`, and
the SCCP arm — become consumers of one registry-described integer add: the
simulator applies it concretely, the interval domain applies its abstract
model, and both answer the leading-zero and overflow questions through the
same target rules.

### `switch`

The CFG builder flattens only an exact, case-sensitive, no-fall-through
`switch` into a dispatch chain of `StrEq` branches; glob, regexp,
`-nocase`, and any fall-through arm stay one opaque `Statement::Switch`,
and `lower_opaque_switch` stores that structured statement in one block —
it creates no CFG block per arm. Even in the flattened form a
whole-variable subject lowers to `ExprNode::Raw`, deliberately, because
`Raw` is the only operand that preserves a backslash-bearing `${…}` name
under both 8.x and 9.x close rules, and `Raw` cannot be evaluated.

| Form | O112 (structured IR) | O107 / I231 (CFG) |
|---|---|---|
| exact, literal subject | fires | fires |
| exact, `$var` subject | fires when `var` is constant | never — `Raw` |
| exact + `-nocase`, or a fall-through arm | fires | never — opaque |
| `-glob` | fires (`pattern_matches` is glob-aware) | never — opaque |
| `-regexp` | never (bails) | never |

```mermaid
flowchart LR
    SRC["set x b<br/>switch $x { a {A} b {B} default {D} }"] -->|lowers| C1["StrEq(Raw $x, &quot;a&quot;)<br/>Raw: unevaluable today"]
    C1 -->|true| A["arm A<br/>I231 · O107 after step 1"]
    C1 -->|false| C2["StrEq(Raw $x, &quot;b&quot;)<br/>→ Const(&quot;b&quot;) after step 1"]
    C2 -->|true| B["arm B · taken"]
    C2 -->|false| D["default body D<br/>not taken · O107"]
    OP["opaque forms: -glob · -regexp · -nocase · a - body<br/>Statement::Switch, one block today"] -->|subject Const or ConstSet| SEL["step 2 · selection facts<br/>tcl_cmd_core::switch::select over the arms:<br/>ordered first match, fall-through, default,<br/>captures, option parsing, match errors"]
    SEL --> CONS["consumers of one selected-edge fact<br/>O112 · analyser switch_body_is_selected ·<br/>static_loops::exec_switch · I231"]
    SEL -. only with real lowering or explicit arm blocks .-> CFG["step 3 · applied reachability<br/>executable_blocks · O107"]
```

The order of delivery, each step with its own contract:

1. **The exact whole-variable case.** `evaluate_branch` resolves a `Raw`
   operand from the lattice only when the existing word and variable-name
   owners prove the operand is exactly one variable reference; arbitrary
   `Raw` text never becomes executable because one synthetic operand uses
   that variant. The flattened form then decides per arm for a constant
   subject, `executable_blocks` drops the dead arm bodies, O107 and I231
   fire, and O101 stays suppressed on the synthetic chain as today.
2. **Selection facts for opaque forms.** For a `Statement::Switch` whose
   subject is `Const` or a `ConstSet`, arm selection is computed by the
   existing owner — `tcl_cmd_core::switch::{parse_options, select}` in
   `rust/tcl-cmd-core/src/switch.rs`, which already implements exact, glob,
   and regexp selection and capture construction over `RegexEngine` — and
   recorded as a structured-arm fact against the arm's `pattern_span`. The
   registry-declared selection semantics include ordered first-match
   behaviour, the final-default rule, fall-through to a following body, regexp
   capture writes through the same storage outcomes, option parsing, and
   match errors. A pattern that never matches can still supply the body
   reached through a preceding `-` arm, so deleting an arm and body pair
   can change behaviour; a `ConstSet` subject joins selected bodies and
   writes across every member and retains error possibilities when a
   pattern cannot be evaluated. O112, the analyser's
   `switch_body_is_selected`, and `static_loops::exec_switch` consume that
   one fact instead of three private matchers; the runtime adapters'
   remaining steps — fall-through body resolution and body execution — are
   modelled by the analysis adapter too.
3. **CFG integration and edits.** Applied reachability for an opaque form
   needs either real lowering support or explicit arm blocks; a post-pass
   cannot remove blocks that do not exist. Source edits that delete an arm
   are optional presentation work after the semantics are established, and
   no optimisation code is reserved for them until the ordered-matching,
   completion, source-edit mapping, and proof contracts are implemented.

A `case_list` descriptor establishes locations and grammar. A private
dispatch-table command receives Tcl `switch` execution semantics only when
its registry declaration names that semantic contract explicitly;
structural similarity is not proof of first-match execution. With the
contract declared, an Expect-like `case` or a vendor `switch` wrapper gets
the same selection facts from its `.tclspec`, as data.

### `catch`, `try`, and completion

A `catch` body is one opaque `Call` in the default CFG build
(`emit_opaque_catch` in `cfg_builder/mod.rs`: the body's defs, the result
variable, and the options variable become the call's defs), a `try` with
handlers is deferred the same way (`lower_try_dispatch`), and only the
faithful-exceptions build (`with_faithful_exceptions`, `lower_try`,
`push_try_handler_exception_edges`) gives handlers blocks and exception
edges. The interface does not change which build a tier uses; it gives
both the same plan and the same completion protocol.

**The protocol.** Every invocation completes one way. The registry
declares the static domain (`CompletionDescriptor::codes`,
`CompletionCodeDomain::Exact` or `Any`) and the evaluated outcome is the
instance: `CompletionOutcome::Normal`; `Code { code, level, result }` for
`return`, `break`, `continue`, and a numeric `-code`; or `Error {
written, message, error_code }`. A body-taking command's plan names how
the body's completion becomes its own:

```rust,ignore
/// How a body plan's completion becomes the command's. Produced with
/// the `Body` or `Iterate` plan; consumed by the CFG builder's edges and
/// by the solver when it publishes facts per path.
enum CompletionProtocol {
    /// The body's completion is the command's (`if`, `namespace eval`).
    TclBody,
    /// Loop bodies: `break` and `continue` are absorbed, the rest pass.
    Absorb(&'static [CompletionCode]),
    /// `catch`: every completion is absorbed; the code is the result,
    /// the result or message goes to `result_var`, and the options
    /// dictionary to `options_var` (from 8.5).
    CatchAll { result_var: Option<TargetId>, options_var: Option<TargetId> },
    /// `try`: the first handler whose `on CODE` or `trap PATTERN` matches
    /// the body's completion runs with the message and options bound;
    /// a `-` handler shares the next body; `finally` runs on every path.
    Handlers { handlers: Vec<HandlerPlan>, finally: Option<OperandId> },
}

/// One `try` handler: how its pattern word selects it, the pattern
/// operand, the message and options binders, and the body — `None` for
/// a `-` handler that shares the next body.
struct HandlerPlan {
    matches: HandlerMatch,
    pattern: OperandId,
    binders: Vec<Binder>,
    body: Option<OperandId>,
}
```

`HandlerPlan::matches` is the clause-grammar descriptor's `HandlerMatch`
([registry-consumer-contracts.md](registry-consumer-contracts.md)
§ *The clause-grammar descriptor*): whether the pattern selects by
completion code (`on`) or by `-errorcode` prefix (`trap`), stated once
for the grammar and read here per handler.

**Write order across the error edge — the prefix rule.** An outcome's
`ordered_stores` are in execution order, and `Error { written, .. }` says
how many of them ran; the targets after that index are preserved. Under
every tested release (`lassign` from 8.5, `try` from 8.6):

```tcl
set a old; array set b {k keep}
catch {lassign {new second} a b} m   ;# 1; a is new; b(k) is keep
set a old; set c old
catch {lassign {x y z} a b c}        ;# 1; a is x; c is old
set a old
catch {foreach {a b} {new second} {set inside 1}}   ;# 1; a is new; inside unbound
set a old
catch {scan {1 2} {%d %d} a b}       ;# 1; a is 1
set a old
catch {regexp {(x)(y)} xy a b}       ;# 1; a is xy
set p 1; set q 2
catch {unset p nosuch q}             ;# 1; p is unbound; q is 2
set x 1
catch {append x 2 [error boom]}      ;# 1; x is 1: the word failed before the command ran
catch {set r [expr {1 + [error mid]}]}   ;# 1; r stays unbound
set x 1
catch {expr {[incr x] + [error mid]}}    ;# 1; x is 2
```

So an error in an argument word is `written = 0` for the command, an
error in the command's own step `k` is `written = k`, and an error
inside a nested substitution carries the ordered evaluation state's
writes so far. When the inputs are not exact enough to place the failing
step — which target is an array is an existence fact — the `Existence`
transfer lists the paths with their prefixes: on the error path every
target before the first whose kind is not proven `Scalar` is a `Bind`,
that target and the rest are `MayBind`, and the normal path lists every
target. The messages differ by release (`couldn't set variable "b"`
under 8.4 and 8.5 for `scan` and `regexp`, `can't set "b": variable is
array` from 8.6), so `message` is exact only when the route proves it
under every profile release, and `error_code` likewise; the code is exact
in every case.

**What `catch` exposes.** The result is the completion code; under every
tested release `catch {return 5}` is `2` with result `5`, `catch {break}`
is `3`, `catch {continue}` is `4`, `catch {error boom} m` is `1` with `m`
equal to `boom`, `catch {set v 1} r` is `0` with `r` equal to `1`, and
`return -code 5 custom` inside a procedure is `5`. The options dictionary
(from 8.5; 8.4 accepts no third argument) carries `-code` and `-level`
exactly, `-errorcode` exactly when the route proves it (`NONE` for a
plain `error`), and `-errorinfo`, `-errorline`, and `-errorstack` as
`Unavailable` with type `dict`; on success it holds `-code 0 -level 0`.
An error also writes the globals `::errorCode` (exact when proven) and
`::errorInfo` (`Unavailable`) through the effect domain. `catch {incr
absent}` is the release split of § Existence in completion form: `1`
under 8.4 with `absent` unbound, `0` from 8.5 with `absent` equal to `1`.
`catch {expr {1/0}}` is `1` with the message `divide by zero` in every
release: the expression has no value, and its completion is exact. `try`
(from 8.6) runs the matching handler with the message and options bound
and runs `finally` on every path, so `catch {try {error a} finally {set f
1}}` is `1` with `f` equal to `1`.

**What stays opaque, and why.** A body is evaluated concretely only when
it is closed — every statement has a declared route, no barrier, no world
effect, exact inputs — which is what `catch {expr {1/0}}` and `catch
{incr absent}` are. Otherwise the body's facts are the tier's: in the
default build the body is one call whose defs are `MayWrite`, whose
result variable is a `Write` of an unavailable value, and whose inside is
`Unavailable`; in the faithful-exceptions build each statement transfers
as usual and every handler's entry state is the join of its throw
sources' prefix states. Opaque in every build: an error raised inside an
invocation with `CompletionCodeDomain::Any` (every target of that
invocation is `MayWrite` on the error edge), traces that run during the
body, the text of `errorInfo`, a computed `-code`, and `bgerror`. The
exception edge remains a completion fact, which the DSL excludes for the
reason the spec-pack rules give; `error`, `throw`, `exit`, and `tailcall`
promote to `Return` terminators through `TERMINATES_BLOCK`, and a decided
`switch` arm that ends in one keeps that promotion.

Consumers: the CFG builder's exception edges, the solver's per-path
publication, the existence rung, W210 in handlers, O109 (today `tcl opt
--profile full` deletes `set a old` ahead of `catch {lassign {new second}
a b} msg`, the #2051 shape through `catch`), and the Explorer's per-path
view. Tests: `try_finally_creates_finally_block` and `try_with_handler`
in `cfg_lower.rs` pin the faithful shape; the nine witnesses above and
the `catch` code table are the fixed additions. Migration: slice 10,
sequenced after slices 5, 8, and 9, whose exit criteria are the prefix
rule through both builds and the O109 refusal.

### Predicate refinement

Inside the taken arm of `if {$x eq "a"}` or the `a` arm of an exact
`switch $x`, `x` is `"a"` even when it was `Overdefined` before the test.
The existence-guard narrowing (`collect_existence_guards`,
`block_dominated_by`) is the precedent, and the general mechanism is an
edge fact:

```rust,ignore
/// A fact that holds on one CFG edge and in every block that edge's
/// target dominates, for one SSA version. Produced by the `Selection`
/// transfer of a branch condition or a case list; consumed by the
/// block-qualified lookup of every domain it names.
struct EdgeRefinement {
    edge: (BlockId, BlockId),
    key: ValueKey,
    domain: FactDomain,
    /// The refined fact: an exact value, a finite set, an existence, a
    /// type class, a range point, or a segment.
    fact: DomainFact,
    evidence: DependencyEvidence,
}
```

**What refines, per shape.** The expression route's `Selection` transfer
reads the condition's tree and answers per edge; the variable must be one
plain local operand of the comparison and the other side a literal or a
proven constant:

| Shape | True edge | False edge |
|---|---|---|
| `$x eq LIT`, `LIT eq $x` | `ExactValue` is `LIT` | nothing |
| `$x ne LIT` | nothing | `ExactValue` is `LIT` |
| `$x == LIT`, `LIT == $x`, with `LIT` non-numeric under every profile release | `ExactValue` is `LIT` — the comparison is a string comparison | nothing |
| `$x == LIT` with `LIT` numeric | `Range` is the point `LIT` and `Type` is numeric; never the string | nothing |
| `$x != LIT` | nothing | as `==`, on this edge |
| `$x in {L1 L2 …}` (from 8.5) | `ExactValue` is the finite set | nothing |
| `$x ni {…}` | nothing | the finite set |
| `[string is CLASS ?-strict? $x]` | `Type` is the class; never a value | nothing |
| `[info exists x]`, `[array exists x]` | `Existence` is `Bound(Either)` / `Bound(Array)` | `Unbound` for `info exists`; nothing for `array exists` |
| `!C` | the false-edge answer of `C` | the true-edge answer of `C` |
| `C1 && C2` | both true-edge answers | nothing |
| `C1 \|\| C2` | nothing | both false-edge answers |
| `switch -exact` arm `LIT` (the default mode) | `ExactValue` is `LIT` at the arm's entry; a body reached through `-` arms gets the finite set of their patterns | nothing at `default` |
| `switch -nocase` (from 8.5) | `Type` is "case-insensitively equal to `LIT`"; never a value | nothing |
| `switch -glob` | `Segments`: the literal prefix of the pattern | nothing |
| `switch -regexp`, `$x`, `$x eq $y` | nothing | nothing |

The numeric rows are why `==` is not `eq`. Under every tested release,
except where a line says otherwise:

```tcl
set x 1.0; if {$x == 1} {string length $x}     ;# 3: the string is still 1.0
set x " 1"; if {$x == 1} {string length $x}    ;# 2
set x 01; if {$x == 1} {set x}                 ;# 01
set x 1.0; if {$x eq "1"} {puts yes} else {puts no}   ;# no
set x a; if {$x eq "a"} {set x}                ;# a
string is integer " 12 "                       ;# 1
string is integer {}                           ;# 1; 0 with -strict
string is integer 0x10                         ;# 1
set x yes; if {$x} {set x}                     ;# yes: truth is not a value
set x 08; if {$x == 8} {puts yes} else {puts no}   ;# 8.4, 8.5, 8.6: no; 9.0, 9.1: yes
set x A; switch -nocase -- $x { a {set x} }    ;# 8.5 onwards: A; 8.4: bad option "-nocase"
set x b; if {$x in {a b c}} {set x}            ;# 8.5 onwards: b; 8.4: syntax error
```

The leading-zero line does not make the `==` row release-ambiguous: on
its true edge `x` is numerically `8` under whichever release took it, and
the string is untouched either way.

**The block-qualified lookup.** A refinement never creates an SSA
version; it overrides the version's lattice value in the blocks the
edge's target dominates, until a new definition of the name. The
per-value map (`SccpResult::values`, keyed by `ValueKey`) gains a second
key, `(BlockId, ValueKey)`, consulted first by `env_from_uses`,
`evaluate_branch`, and `evaluate_def_with_folds` through the block they
run in, and by every other domain's lookup for the domains it names. At a
merge the edge's target does not dominate the override is not consulted,
so the join is with the unrefined version — two arms refining `x` to `a`
and `b` meet as the version's own value, and a `switch` whose arms all
refine is enumerated only through the finite set its case list supplies.
A place that is externally mutable (`is_externally_mutable`) is never
refined: the version could change between the test and the use, and
that is the one sense in which a widened place is not re-narrowed.

**What it feeds.** The value lattice, so a nested `if {$x eq "b"}`
inside `if {$x eq "a"}` decides false and I230 and O112 fire — today
`tcl diag` and `tcl opt` report nothing for it; the type lattice through
`string is`; the existence rung through the guard, byte-identical with
today's narrowing; the range domain; O100, under the same presentation
gate as any constant. Taint never reads values and is untouched. Tests:
`info_exists_guard_narrows_read_in_then_arm`,
`info_exists_negated_guard_narrows_false_arm`, and
`info_exists_read_outside_guard_still_flags_w210` pin the precedent; the
fixed additions are the twelve witnesses above, the nested-`if` I230, a
merge that drops the refinement, and a traced variable that is never
refined. Migration: slice 11, sequenced after slices 6 and 8, whose exit
criterion is the nested-`if` program deciding through `tcl diag` and
`tcl opt` with `collect_existence_guards` deleted.

## Proc-level transfer summaries

A private procedure needs no spec: the argument-sensitive O103 path
(`evaluate_proc_with_constants` in `optimiser/propagation.rs`) re-runs
SCCP on the callee under the call's constants, so every specialisation
the callee's body uses evaluates through it, and `proc pad {s} {append s
"!!"; return $s}` folds `[pad hi]` to `hi!!` once `append` has a route.
What the callee does to the *caller's* places is the summary's business:

```rust,ignore
/// The caller-visible transfer of one procedure, derived from its own
/// analysis under the seedless lattice. Produced by the interprocedural
/// pass beside `ProcSummary`; consumed by the caller's transfer driver
/// at every call site, by O103, and by the existence rung.
struct TransferSummary {
    params: Vec<ParamRole>,
    /// Places outside the callee's frame it may write, with the outcome
    /// kind: `set ::counter 5`, `incr ::hits`, a `global` alias.
    globals: Vec<(PlaceRef, ExistenceOutcome)>,
    /// `Literal`, `Passthrough(param)`, or computed — the shape
    /// `summarise_returns` derives, over the seedless lattice.
    result: ReturnKind,
    completion: CompletionCodeDomain,
    effects: EffectFootprint,
    evidence: DependencyEvidence,
}

enum ParamRole {
    /// The argument is a value the body reads.
    Value,
    /// The argument names a caller-frame place the body binds through
    /// `upvar LEVEL $name v` and applies these outcomes to, in order;
    /// `ProcArgTrait::VarRead` and `VarWrite` are its two projections.
    Name { level: FrameLevel, outcomes: Vec<ExistenceOutcome> },
    /// Never read.
    Unused,
}
```

- **The seedless lattice.** The summary is computed from the callee's
  unit with its parameters `Overdefined` — no call-site seeds — so it
  holds for every caller; a return that is constant under the seedless
  lattice is a constant return for all callers, which is what
  `summarise_returns` consumes (slice 7). A value that is exact only
  under a seed belongs to the argument-sensitive re-run, never to the
  summary, and the re-run's seeds (`seed_params_from_args`) gain the
  caller-frame place values for the `Name` parameters: `bump n` re-runs
  `bump` with `v` seeded from the caller's `n`.
- **Application at a call site.** The caller's driver resolves each
  `Name` argument to a place in the frame `level` selects, applies the
  outcomes in order — a cell update's value from the re-run when the
  O103 path runs, otherwise a `MayWrite` with the summary's bounds — and
  applies `globals` to the module's places; a summary with no `Name`
  parameters and no `globals` leaves the caller's places alone, which is
  what the synthetic `<upvar-invalidate>` def in `cfg_builder/mod.rs`
  approximates today for every caller local an `upvar` callee could name.
- **Context limits.** One summary per procedure, context-insensitive;
  summaries compose bottom-up over the call graph, so `twice`'s `Name`
  outcome is the composition of two `bump` cell updates; a cycle resolves
  by the staged fixed point the evaluation page requires of
  `summarise_returns`, and a cycle that does not converge within
  `MAX_INTERPROCEDURAL_WALK_DEPTH` yields `MayBind` for every `Name`
  place, `Any` completion, and a computed result. An unknown callee, a
  `has_unknown_calls` body, or a callee reached through a suspect binding
  is a barrier at the call site.
- **Invalidation.** A summary depends on the callee's body, its bindings
  (`ModuleCommandMutations`), its callees' summaries, the seeds'
  revision, and the pack revision; the analysis context carries the
  summary revision, so a `rename` or a redefinition anywhere in the
  module invalidates every summary in it, the sensitivity `FnLatticeKey`
  already has for traces.

Under every tested release, except where a line says otherwise:

```tcl
proc bump {name {by 1}} {upvar 1 $name v; incr v $by}
set n 1; bump n; bump n 2; set n              ;# 4
proc reset {name} {upvar 1 $name v; unset v}
set m 1; reset m; info exists m                ;# 0
proc twice {name} {upvar 1 $name w; bump w; bump w}
set t 1; twice t; set t                        ;# 3
proc g {} {set ::counter 5}; g; set ::counter  ;# 5
proc rec {n} {if {$n <= 0} {return 0}; expr {$n + [rec [expr {$n - 1}]]}}
rec 4                                          ;# 10: the argument-sensitive path
proc ctr {} {incr ::hits}; ctr; ctr; set ::hits   ;# 2 from 8.5; 8.4: can't read "::hits"
bump absent                                    ;# 8.4: can't read "v"; from 8.5: 1, and absent is bound
```

Today `tcl opt --profile full` leaves `if {$n == 2}` after `bump n`
undecided. Consumers: O103; the caller's value lattice and existence
rung; W210 and W211 in the caller (a `Name` write defines the caller's
place); `param_traits.rs`, whose two-command copy tracker reads the
summary's `Name` outcomes instead; taint's interprocedural pass; and the
`<upvar-invalidate>` def, which the summary replaces. Tests:
`o103_folds_implicit_return_proc_cmd_subst` and
`o103_folds_arg_sensitive_passthrough_cmd_subst` in `propagation.rs` pin
the two O103 paths; the seven witnesses above are the fixed additions.
Migration: slice 13, sequenced after slices 7 and 8, whose exit criteria
are `bump n` deciding through both O103 paths and `param_traits.rs`
matching no command by name.

## Diagnostics consume facts

`CompilationUnit` and the per-function lattice queries supply reusable
analyses; compiler checks emit a protocol-independent `Diagnostic`;
`CompilerDiagnostics` retains checks and optimisation findings
independently of display-time gates; the server's lifts convert spans and
severities and apply tags and overrides. Those boundaries are strengthened,
not replaced, under six rules:

1. **Display policy cannot change semantic truth.** Disabling W210, W100,
   I230, or the optimiser presentation must not change values, storage or
   existence facts, reachability, or summaries consumed elsewhere. A
   disabled expensive rule may skip its own private calculation; it cannot
   withhold a shared fact an enabled consumer needs. Semantic context
   inputs are separated from diagnostic and style settings even while one
   configuration object carries both.
2. **Facts carry evidence; findings carry policy.** A branch fact
   identifies the condition, the proof context, and its edge or
   reachability status. A rule chooses I230, I231, or an optimisation
   finding and an explanation. A value specialisation never chooses a
   severity, a message, or an editor range.
3. **A diagnostic is not a rewrite certificate.** An unreachable-arm
   observation can be useful without a safely editable region. Edit
   planning additionally checks original syntax, comments, substitutions,
   effects, source revision, and the exact replacement range, keeps the
   distinction between hints and actionable replacements, and never gives a
   fabricated span to a value to obtain a code action.
4. **Aggregation does not recover semantics.** Lifts filter, map ranges,
   attach tags, and serialise edits. Workspace refinement belongs to its
   semantic owner. The publication path never parses messages, evaluates
   expressions, or infers a fact from whether another diagnostic survived
   suppression; intentional overlap policy such as W110 / O120 precedence is
   explicit and separate from fact production.
5. **Availability and revision are part of the contract.** A fast-tier
   request can lack a deep fact without that fact being false; the
   workspace-refinable policy and the lightweight structure queries stay.
   `file_token_facts` in `rust/tcl-lsp-db/src/lib.rs` deliberately uses
   structure-only analysis, and its source records the reduction that
   choice bought on an 883-file workload; "every consumer shares the
   contracts" never becomes "every consumer eagerly computes every fact". A
   result or edit from an old source or context revision is not attached to
   the current document; canonical relative spans are rebased by the
   source-mapping owner, never guessed by consumers.
6. **Policy has one owner below every surface.** Suppression directives,
   disabled codes across the documented scopes, default-off seeding,
   severity overrides, the optimiser switch and profile, overlap
   precedence, and encoding abstention are applied once, by a function
   every surface calls, and a suppressed finding stays in the report with
   its reason. Today each surface assembles its own subset: the server's
   publish paths, `tcl diag`, `tcl opt`, the MCP tools, and the code-action
   lifter differ on which steps they apply, which is the parity gap #2089
   tracks. The `# noqa` directive has one parser and one predicate; the
   pipeline around them is the remaining half: one `Finding` shape, one
   `Policy` holding every documented scope, one pure `apply` that pairs
   every finding with an outcome, and adapters that render rather than
   decide, designed in [diagnostic-policy.md](diagnostic-policy.md).

Three producers move onto the interface in the first slices:

- `emit_provably_unset_w210` in
  `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` recognises
  `regexp` and `scan` by name, parses their forms, and computes no-match
  consequences inside a diagnostic producer, with a second traversal for
  embedded conditions; its own comment says the registry lacks these
  per-form semantics. The owner is the registry transfer's *preserve*
  outcome plus the existence rung (§ Existence): no match preserves, so
  the prior cell fact is necessary, and W210 consumes the resulting proof. This
  is the end-to-end acceptance test for storage outcomes and the regexp
  owner, not a name-dispatch cleanup.
- The existence constant-branch fact is stored once with its kind
  (proven, selected, applied), as above.
- `append_brace_expr_perf_hints` in `rust/tcl-lsp-server/src/lib.rs`
  creates O111 by searching already-lifted diagnostics for W100, so O111's
  production depends on another diagnostic surviving presentation policy.
  Both rules consume the same unbraced-expression fact, or the product
  encodes a shared rule-group policy; the same file's encoding abstention,
  which uses byte-decode evidence rather than whether W109 is displayed, is
  the pattern to follow.

Not every predicate needs a globally stored lattice: a rule-specific
analysis may compute its finding from shared facts, as the interval
emitters delegate to the canonical interval-bounds owner. The boundary is
that rules do not reimplement command semantics, and reusable conclusions
are not available only as a side effect of emitting a warning.

Existing registry-owned validation stays, and is the model for
command-specific validity: `LiteralArgumentValidator` in
`rust/tcl-registry/src/literal_validation.rs` returns `Valid`, `Invalid`
with a typed issue, or `Abstain`, with an argument index and a semantic
reason independent of diagnostic types; `ConstraintReport` in
`rust/tcl-registry/src/spec.rs` lets a hook report a slot and an explanation
while the analyser owns code and span policy, and deliberately permits
authored message text. A value query never emits W146 as a side effect, and
disabling W146 never prevents another consumer from learning the operand's
value; when a constant-derived argument can be validated, its exact value
is fed through the validation interface with honest provenance, and its
source token is not relabelled as a literal. A newly declared
command-specific constraint must not require a new diagnostic
command-name arm; registry authorship of the constraint and consumer
ownership of policy are complementary.

## The other domains

"One semantic owner" means one resolved meaning and context with
composable queries and explicit dependencies, not one callback that must
compute every fact before answering any query. A known return type stays
available when the evaluator cannot run; a taint transfer stays available
when the value is unknown; a loop body stays analysable when the iteration
count is unknown.

| Interface | Registry-owned answer | Generic consumer owns | What invalidates it |
|---|---|---|---|
| Invocation and form resolution | argument roles, selected form, option semantics, expansion-sensitive uncertainty | source-to-operand mapping; the canonical resolved invocation | word and expansion facts, binding, overlay, dialect |
| Structural plan | bodies, scopes, binders, control and completion protocol, declared structural effects | AST and IR construction, CFG edges, SSA definitions, phi placement | grammar, binding, body text, alias structure |
| Exact evaluation | exact result and ordered store outcomes, or a typed decline | proven operand reads, budget, validation, materialisation | operand and storage versions; every declared environmental dependency |
| Type and shape transfer | semantic result possibilities, intrep guarantees, per-target and per-element relationships | the rich type lattice, joins, aggregate limits, consistency | operand types and shapes, selected form, target semantics |
| Existence and aliases | define, preserve, unbind, may-write; escape and alias descriptions | place identity, reaching definitions, exceptional state, joins | storage and alias epochs; trace, `uplevel`, and unknown effects |
| Range and partial value | bounds, lengths, prefixes, or typed residual operations with prerequisites | abstract interpretation, widening, bounded residual representation | operand range and shape facts, arithmetic profile, effects |
| Effects and completion | reads and writes, observable operations, possible error, return, break, continue, yield | control flow, sequencing, deletion and motion proofs | resolved implementation, callback and body summaries, traces, state |
| Taint and provenance | source, sink, and transform relationships, with the call shapes that qualify a transform (`taint_transform_when`); encoding and normalisation kind | the flow lattice, provenance paths, context-specific sink checks | operand flow, unknown effects, sanitiser identity, dialect |
| Intrep and sharing | guaranteed construction and conversion; alias and share relationships | shimmer, use-site, and mutation-copy analysis | value provenance, representation transitions, escape and share facts |
| Protocol and resource state | typed domain events (collect and release, HTTP commit, widget ownership) | the domain state machine and path joins | event and body flow, command meaning, the referenced resource world |
| Interprocedural summary | a declared external summary, or an analysable body with its contract | the call graph, recursive fixed point, context limits, dependencies | body, callee and binding set, captured and global state, pack revision |
| Backend capability and lowering | the target operation, permitted forms, required semantic and safety facts | backend IR construction, legality, allocation, verification | language profile, ABI, helper and map capabilities, proof-changing rewrites |
| Validation | a typed constraint verdict and the offending operand or member | rule code, severity, source range, suppression, presentation | the invocation and the relevant fact and context revisions |

An analyser-local fact not in this table is not exempt: its owner declares
its input dependencies, its monotone transfer and join or its non-dataflow
evaluation phase, its unknown policy, its cache key, and its consumers. The
migration plan's ledger is the checklist.

**Construction and solving are different phases.** First, resolve the
invocation and its structural semantics and build scopes, IR, CFG, and SSA
from validated plans, with conservative effects for unresolved calls and
dynamic bodies. Second, run the domains over that graph: a specialisation
queries immutable input facts and returns deltas; the owning solver
validates, applies, schedules dependent queries, joins, and widens. Third,
if improved binding, alias, or structural information changes the graph,
invalidate the affected construction query and rebuild its dependants — a
hook never inserts SSA definitions halfway through a worklist. Fourth,
publish an immutable, revision-labelled fact snapshot that rules,
optimisers, code generators, and editor queries consume at their precision
tier, with rewrites carrying their own proof obligation. These are logical
phases a demand-driven query graph can realise; a cycle such as type
inference asking for an exact result whose evaluator asks for the same type
gets an explicit fixed point or a conservative cycle result, never
recursive callback execution, and a callback reads the supplied current
state rather than demanding the completed query being computed. Summary
dependencies stay acyclic: `summarise_returns` consulting a lattice that
depends on those summaries needs a staged or explicit fixed-point protocol,
especially under recursion.

**Cross-domain reduction has rules.** An exact value may refine a semantic
type or a range; it does not prove a runtime representation the evaluator
fabricated while transporting the value. A numeric type does not erase
taint. A failed exact evaluation does not erase an independently
established length bound. Contradictory claims from two hooks are rejected
or quarantined with conservative fallback, never resolved by whichever ran
last.

**Dependency direction.** Registry-neutral fact views and typed answers sit
below the compiler; adapters, lattice order, joins, widening, and solver
state sit in the analysis owner. The registry does not depend on compiler
SSA types, and SpecTcl does not clone the compiler's `TypeLattice` and
`TypeShape` (`rust/tcl-compiler/src/types.rs`). If a shared protocol needs a
new crate, it is a small contract crate, not a second analysis engine. The
existing string-array return-type entry point cannot see expansion
provenance; it evolves towards the resolved invocation and typed operand
views with a compatibility adapter, and never rediscovers substitution by
scanning `$` and `[` in decoded values.

## Vendor loops and BPF

**A vendor collection is not a Tcl list.** The bundled SDC pack declares
`foreach_in_collection` (`specs/sdc_base.tclspec`) with `VarWrite` and
`Body` roles, `LOOP_LIST_HEADER`, and `analyser_hook -native Foreach`; the
native `Foreach` handler in `rust/tcl-compiler/src/analyser/handlers.rs`
can split a braced literal iterable as a Tcl list and simulate selected
definition effects per element. Reusing a loop hook imports more than body
walking. Vendor iteration is its own declared protocol — binder grammar,
iterable kind, yield type, cardinality source, completion contract,
zero-iteration behaviour — checked per vendor pack, and the analyser
applies it generically:

- A constant handle is not constant contents, membership, order, or
  length; the vendor object and collection types are modelled separately
  from Tcl list representation, and collection facts depend on the vendor
  design database revision and command side effects.
- The body is analysed with a typed unknown yielded object even with no
  vendor runtime; the zero-iteration edge, the back edge, and the binding
  definitions come from the declared protocol, and prior variables are
  preserved on the zero-iteration path where required.
- Known cardinality can help reachability and range analysis without
  exposing elements; known elements are used only with adequate identity
  and snapshot evidence; the printable handle is never split.
- Break, continue, return, error, nested loops, and implicit results follow
  the vendor's completion rules; termination is never inferred from a body
  argument or `CONTROL_FLOW`.
- Mutation of the vendor database invalidates affected collection
  summaries and object-property facts while Tcl-local propagation stays
  valid; an unknown external command needs a declared world-effect boundary
  or conservative invalidation.
- Vendor filter strings are their own language unless the pack declares
  Tcl expression semantics for them; they get a domain owner with a typed
  query contract, never a trip through our engine.
- Lowering may keep a generic runtime call even when analysis understands
  the loop: body analysis, exact enumeration, and executable backend
  support are three capabilities.

The acceptance fixture defines a new vendor loop spelling in a pack with no
consumer edits and exercises W210 and W211, type and taint flow, zero and
multiple iterations, break, continue, and error, collection invalidation,
and conservative lowering, including a handle whose printed text is a valid
multi-element Tcl list.

**BPF-Tcl is a different language.** [ebpf-backend.md](ebpf-backend.md)
defines it as a statically typed language using Tcl syntax with its own IR
and restricted operations, and the registry already carries `BpfOpSpec`
(`rust/tcl-registry/src/bpf_op.rs`; `commands/bpf/loop_.rs` declares
`BpfOpSpec::structural(BpfOpKind::LoopMacro)`). Signed division truncates
towards zero in BPF, so `-7 / 2` is `-3`, where Tcl floors to `-4`.
Constant evaluation therefore selects a *language-semantic profile* —
width, overflow, signedness, division and remainder, shifts — as a cache
dependency, and reuses the expression parser and engine only with the BPF
arithmetic adapter; a Tcl engine result is never a BPF constant. Packet and
map handles keep pointer kind, offset and range, nullability, map identity,
and lifetime as typed facts; a constant-looking address never erases
provenance or manufactures access permission. The existing
`loop N VAR {body}` unrolls a literal bound, at most `MAX_UNROLL = 64` in
`rust/bpf-tcl-ir/src/unroll.rs`, before CFG construction; accepting a
proven bound instead of a literal would be an explicit language change
with resource and verifier tests, not a consequence of better SCCP. Subset
acceptance, packet bounds, map access, scalar legality, bounded execution,
stack and calling convention, handler completion, and target capability
each remain a separate proof that a computable output does not supply, and
backend legality is never gated by diagnostic enablement.

## The completion test

Migrated command names are not the proof. The proof is the experiment:

1. Author one private command with a supported existing transfer,
   validation, and body pattern. Rename it, and add a subcommand form with
   different operand positions. The only semantic edits are in its
   registry-owned declarations and implementation. Exercise analysis, a
   relevant diagnostic, the optimiser, incremental overlay changes, export,
   renderer, and studio preservation, the CLI, and the applicable runtime
   fallback. No consumer changes.
2. Author a command that needs a genuinely new semantic capability.
   Extending the generic interface is legitimate; disguising a new command
   ID as a family-neutral operation is not, and the review rule for the
   distinction asks for two unrelated clients of the operation where
   practical.
3. Audit command-specific ID arms as well as string matches, and keep a
   ledger of the remaining command-specific consumer handlers rather than
   calling them irreducible by default.

"No analyser change ever" is neither achievable nor the goal; the goal is
that catalogue growth stops duplicating semantics the interface already
expresses. No finite review proves the architecture complete for arbitrary
Tcl extensions; it makes the supported boundary complete and falsifiable,
and "not supported yet" is never mistaken for "proved impossible".

## The issue's three questions

**Is the IR node one registry fact with the transfer, or two?** Two facts,
one operation. `LoweringHookId::Incr` describes IR *shape*, and the typed
`Statement::Incr` node is consumed for codegen, native lowering,
α-renaming, span rebasing, and liveness; it stays. The value projection is
derived from the same `CellUpdate` the native lowering declares, so a new
read-modify-write command adds one `CellUpdate` variant and gets both
consumers, the same relationship `SemanticOperationId::StructuredLowering`
has to `LoweringHookId`. The `Statement::Incr` sites that encode
*semantics* — the transfer in `sccp.rs`, the simulator arm in
`static_loops.rs`, the interval arm in `intervals.rs`, the removability and
hidden-read arms in `elimination.rs`, the tail fold in `propagation.rs`,
the global-write rule in `interprocedural.rs` — become consumers of the
resolved semantics; the sites that encode shape do not change.

**Is this a new hook or `const_fold` with an environment parameter?**
A new interface, of which today's `const_fold` is the result-only
projection and the compatibility baseline. Giving `const_fold` an extra
`old` parameter would silently change the contract of every existing
folder; the interface instead declares which operands and target values an
evaluator reads, and returns a result with storage outcomes rather than one
optional string.

**Do the analyser's `analyser_hook` handlers belong on this axis?** No.
`AnalyserHookId` is scope and definition structure — what a `proc`, a
`namespace eval`, or a `dict for` *declares* — and is the structural plan
in the table above. Its constant-string store (`Analyser::const_strings`)
is a consumer of constants, not a producer of transfers. `DictWith` is a
structural plan around a body plus a projection of a constant dict's keys;
keeping the axes apart is what lets the analyser's isolated per-item pass
stay sound with `ModuleCommandMutations::distrust_all()` while the
unit-level lattice evaluates.

## File-path anchors

- `rust/tcl-registry/src/resolved_invocation.rs` — `ResolvedInvocation`, `InvocationSemantics`, the resolver the value axis extends
- `rust/tcl-registry/src/spec.rs` — `CommandSpec`, `SubCommand`, `CommandForm`, `CaseMatchMode`, `case_list`, `ConstraintReport`, `run_const_fold`
- `rust/tcl-registry/src/native_lowering.rs` — `NativeLowering`, `CellUpdate`, the operation the value projection derives from
- `rust/tcl-registry/src/hooks.rs`, `return_type.rs` — `ReturnTypeHookId` and the intrep-guaranteeing return-type algorithms
- `rust/tcl-registry/src/literal_validation.rs` — `LiteralArgumentValidator`, the validation pattern
- `rust/tcl-registry/src/bpf_op.rs`, `commands/bpf/loop_.rs` — `BpfOpSpec`, the BPF descriptor
- `rust/tcl-compiler/src/sccp.rs` — the transfer function, `evaluate_def_with_folds`, `evaluate_branch`, `env_from_uses`, `existence_constant_branches`, `scan_defined_and_unset`, `ExistenceFrame`, `loop_summary_decision`, `parse_literal_value`, `TraceInputs`
- `rust/tcl-compiler/src/const_subst.rs` — `ConstSubstCtx`, `ResolvedConstSubst` and its `command_bindings`
- `rust/tcl-compiler/src/analyses.rs` — `LatticeValue`, `ConstValue`, `MAX_CONSTSET_SIZE`
- `rust/tcl-compiler/src/command_binding.rs` — `ModuleCommandMutations`, `CommandTrustSnapshot`, binding validity
- `rust/tcl-compiler/src/var_observability.rs`, `dynamic_names.rs` — the escaping and dynamic-name gates
- `rust/tcl-compiler/src/tcl_expr_eval.rs`, `rust/tcl-syntax/src/expr/eval.rs` — `FoldOps`, `eval_with_config`, `ExprOps`
- `rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs`, `optimiser/propagation.rs` — `instcombine_expr_typed`, `reassociate_node`, the partial-simplification owners
- `rust/tcl-compiler/src/cfg_builder/cfg_lower.rs` — `lower_switch`, `switch_subject_operand`, `lower_opaque_switch`, `lower_try`, `push_try_handler_exception_edges`
- `rust/tcl-cmd-core/src/switch.rs`, `regex.rs` — `parse_options`, `select`, `RegexpResult::Count`
- `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` — `emit_provably_unset_w210`, `emit_existence_constant_branch_diagnostics`, `emit_read_before_set_diagnostics`, `record_chain_w210_uses`, `emit_unused_variable_diagnostics`, `existence_query_vars`, `existence_exempt`
- `rust/tcl-compiler/src/analyser/diagnostics/helpers.rs` — `collect_existence_guards`, `block_dominated_by`, `whole_unset_names`, `phi_can_undef`
- `rust/tcl-compiler/src/analyser/diagnostics/security.rs` — `emit_w102_subst_injection`, `substitution_narrowing_switches`
- `rust/tcl-registry/src/substitution.rs` — `SubstitutionKinds`, `subst_substitutions`
- `rust/tcl-compiler/src/lowering/mod.rs`, `specialise_factories.rs`, `subst_nocommands.rs` — `eval_subst_nocommands_body`, `SUBST_NOCOMMANDS_KINDS`, `extract_subst_nocommands_template`, `subst_nocommands`
- `rust/tcl-lsp-core/src/refactor/extract_proc.rs`, `refactor/mod.rs` — `literal_word_holes`, `push_substituted_commands`, `same_frame_regions`
- `rust/tcl-compiler/src/dynamic_names.rs` — `DynamicNameBarrier`, `template_word_is_substituted`
- `rust/tcl-compiler/src/static_loops.rs` — `summarise_for_statement`, `exec_statement`, `exec_switch`, `DEFAULT_MAX_STATIC_LOOP_ITERS`
- `rust/tcl-compiler/src/intervals.rs` — `transfer`, `widen`, `MAX_ITERS`
- `rust/tcl-compiler/src/interprocedural.rs` — `ProcSummary`, `ProcArgTrait`, `ReturnKind`, `summarise_returns`, `MAX_INTERPROCEDURAL_WALK_DEPTH`
- `rust/tcl-compiler/src/optimiser/propagation.rs` — `evaluate_proc_with_constants`, `seed_params_from_args`
- `rust/tcl-compiler/src/analyser/param_traits.rs`, `bounds_checks.rs` — the two-command copy tracker and the W240–W242 loop-bound readers
- `rust/tcl-compiler/src/cfg_builder/mod.rs` — `emit_opaque_catch`, `lower_try_dispatch`, `with_faithful_exceptions`, the `<upvar-invalidate>` def
- `rust/tcl-registry/src/completion.rs` — `CompletionDescriptor`, `CompletionCodeDomain`, `CompletionCode`
- `rust/tcl-registry/src/frame_effect.rs` — `FrameLevel`
- `rust/tcl-compiler/src/connection_scope.rs`, `compilation_unit.rs` — `cross_event_defs`, `drop_cross_event_existence_folds`
- `rust/tcl-compiler/src/compilation_unit.rs`, `compiler_checks.rs` — `FunctionUnit::build`, the diagnostic envelope
- `rust/tcl-lsp-db/src/lib.rs` — `FnLatticeKey`, `compilation_unit`, `file_token_facts`, `spec_pack_key`
- `rust/tcl-lsp-server/src/lib.rs` — `append_brace_expr_perf_hints`, the lifts
- `rust/tcl-compiler/src/analyser/handlers.rs` — the native `Foreach` handler, `switch_body_is_selected`
- `specs/sdc_base.tclspec` — `foreach_in_collection`
- `rust/bpf-tcl-ir/src/unroll.rs` — `MAX_UNROLL`

## Test anchors

- `rust/tcl-compiler/src/sccp.rs` — `evaluate_def_incr_*`, `evaluate_def_assign_value_folds_*`, `evaluate_def_foreach_*`: today's arms, and the byte-identity gate for the first slice
- `rust/tcl-compiler/src/optimiser/branch_folding.rs` — `switch_dispatch_branches_are_skipped`
- `rust/tcl-compiler/src/optimiser/propagation.rs` — `o103_folds_implicit_return_proc_cmd_subst`, `o103_folds_arg_sensitive_passthrough_cmd_subst`
- `rust/tcl-registry/tests/differential_fold.rs` — every fold against a real `tclsh`, the shape the storage-outcome witnesses extend
- `rust/tcl-registry/tests/analyser_hooks.rs` — the pinned-set shape
- `rust/tcl-compiler/src/analyser/diagnostics/tests.rs` — `info_exists_*`, `emit_cfg_ssa_diagnostics_w210_*`, `emit_cfg_ssa_diagnostics_w213_*`, `w102_*`: the existence, read-before-set, and template-word answers the rungs keep byte-identical
- `rust/tcl-compiler/src/sccp.rs` — `existence_fold_abstains_*`, `upframe_body_models_*`, `sccp_folds_post_loop_branch_via_static_summary`
- `rust/tcl-compiler/src/static_loops.rs` — `summarise_*`: today's bounded `for` simulation, the enumeration's baseline
- `rust/tcl-syntax/src/expr/eval.rs` — `short_circuit_logical`: the walker's ordering the evaluation state relies on
- `rust/tcl-compiler/src/tcl_expr_eval.rs` — `command_substitution_is_none`: today's refusal of a nested script
- `rust/tcl-compiler/src/cfg_builder/cfg_lower.rs` — `try_finally_creates_finally_block`, `try_with_handler`: the faithful-exceptions shape
- `rust/tcl-registry/src/substitution.rs` — `tp_*`, `fp_*`: the kinds oracle
- `rust/tcl-compiler/src/lowering/mod.rs`, `specialise_factories.rs`, `rust/tcl-lsp-core/src/refactor/extract_proc.rs` — `proc_subst_nocommands_*`, `detects_*`, `rejects_factory_with_computed_subst_switch`, `tp_a_substituting_call_can_switch_its_variable_reads_off`, `tp_a_substituted_bracket_*`: the four template consumers
- fixed witnesses to add: `regexp` no-match preserve, partial `scan`, `lassign … a a`, `dict with` result versus write-back, `expr {0 && [error never]}`, quoted versus braced `expr`, `abs` rebinding, `string range { a } 0 end` exactness, the `$x + 1 + 2` floating-point regrouping refusal, `incr` of `010` per release, the failing dead `lappend`, the correlated `foreach` pairs and their mirror image, the existence release table, the prefix-rule programs and the `catch` code table, the seven ordered-state expressions, the twelve refinement programs, the eleven loop programs, the fourteen template programs, and the seven summary programs — each under every release found on `PATH`, with the release recorded

## Related docs

- [value-evaluation.md](value-evaluation.md) — the evaluation contract behind `evaluate`
- [value-transfers-examples.md](value-transfers-examples.md) — one program per optimisation and diagnostic, and the declarations in Rust and `.tclspec`
- [value-transfers-migration.md](value-transfers-migration.md) — the inventory, slices, gate, and per-consumer changes
- [registry-consumer-contracts.md](registry-consumer-contracts.md) — the other axes and the runtime, package, and extension contracts
- [sccp-core-analyses.md](sccp-core-analyses.md) — the lattice, the drivers, and the existence post-pass
- [constant-folding-type-inference.md](constant-folding-type-inference.md) — the fold-versus-rewrite separation and the type lattice
- [command-registry.md](command-registry.md) — the `CommandSpec` field reference and the hook catalogues
- [lowering-dispatch.md](lowering-dispatch.md) — why `Statement::Incr` exists and stays
- [pass-fact-ownership-matrix.md](pass-fact-ownership-matrix.md), [downstream-pass-contracts.md](downstream-pass-contracts.md), [diagnostics-integration.md](diagnostics-integration.md), [diagnostics-calculation.md](diagnostics-calculation.md) — the ownership, consumer, and diagnostic contracts each implementing slice updates
- [ebpf-backend.md](ebpf-backend.md) — BPF-Tcl as a language
- [optimisation-passes.md](optimisation-passes.md), [precision-limitations.md](precision-limitations.md) — pass ownership and recorded imprecision
- [interprocedural-analysis.md](interprocedural-analysis.md), [interprocedural-call-site-seeding.md](interprocedural-call-site-seeding.md) — the summaries and the seeds
- [../registry/spec-packs.md](../registry/spec-packs.md) (including § *Authoring rules for SpecTcl 2.0 (design E)*), [../spec-dsl-examples/README.md](../spec-dsl-examples/README.md) — the DSL and its hook contract
- [../contracts/vm-compiled-artifact-provenance.md](../contracts/vm-compiled-artifact-provenance.md) — how a folded value in bytecode is admitted and invalidated
- [compiler design index](README.md), [design docs index](../README.md)
