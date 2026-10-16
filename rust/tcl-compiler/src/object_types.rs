// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Object-handle & object-collection → class-name provenance for the
//! `$obj method …` / `[dict get $objs $k] method …` patterns.
//!
//! [`object_handle_classes`] recognises a `set VAR [Factory new|create …]`
//! constructor assignment *and* any SSA value the type lattice typed
//! `OBJECT(class)`, including a handle retrieved from an object collection
//! (`set p [dict get $pins $k]`).
//! [`object_collection_classes`] maps a `List`/`Dict` variable to the class of
//! its elements, harvested from the lattice's container element-typing.
//!
//! The maps union across scopes.  For the syntactic constructor signal that is
//! merely a highlight-precision convenience; for the collection signal it is
//! also the *interprocedural bridge* that resolves the cross-method case — an
//! object built into an instance-variable collection in one method is dispatched
//! from it in another, which no intraprocedural lattice can connect
//! (`experiments/mro_eval/RESULTS.md` measured 99.8% ⊤ intraprocedurally on real
//! `TclOO` corpora, factory-return / cross-method dominating).  An
//! un-provenanced receiver is still left to the generic shape-based option
//! fallback rather than resolved with a wrong-or-abstain lattice.
//!
//! [`object_handle_facts`] is the widened entry point: the same union
//! **plus** a scope-keyed twin,
//! an owner-span index, the collection map, the factory-return fact, and the
//! `::`-qualified subset — one fact for all five dispatch consumers instead of
//! four maps that can disagree on the same document.
//! [`object_handle_classes`] is that entry point restricted to the union, so
//! every existing consumer is untouched.  Contract and consumer/soundness
//! tables: `docs/design/compiler/object-type-lattice.md`.

mod original_collection_sources;
pub use original_collection_sources::{
    OriginalCollectionElementSource, OriginalCollectionReceiverPath,
    OriginalObjectCollectionSourceCandidates, OriginalObjectCollectionSources,
    original_object_collection_sources_at,
};

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{MethodCoordinate, MethodKind, ProcedureId, Statement, WordExpr};
use crate::value_shapes::parse_command_substitution_with_config;
use tcl_core_types::BodyFrameId;

/// Receiver side of an advisory class-state declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectScopeReceiver {
    /// State declared for instances of the class.
    Instance,
    /// State declared for the class object.
    ClassObject,
}

/// Advisory scope coordinates, independent of runtime frames and object tokens.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ObjectFactOwner {
    /// One retained analytical body or its module top level.
    Frame(BodyFrameId),
    /// Shared declarations in one independently retained definition script.
    DefinitionScript {
        /// Independent retained definition-script owner.
        definition: ProcedureId,
        /// Class-object state and instance state remain separate.
        receiver: ObjectScopeReceiver,
    },
    /// A genuine original declaring class allocation.
    OriginalClass {
        /// Original declaring provider, separate from the current receiver.
        allocation: crate::command_binding::CommandAllocation,
        /// Advisory declared-state axis, without object allocation authority.
        receiver: ObjectScopeReceiver,
    },
}

fn frame_for_unit(cu: &CompilationUnit, fu: &FunctionUnit) -> Option<BodyFrameId> {
    if std::ptr::eq(fu, &cu.top_level) {
        Some(BodyFrameId::Top(cu.ir_module.procedure_ids.module_id()))
    } else {
        fu.body_id.map(BodyFrameId::Body)
    }
}

/// The source extent of one owning scope, plus the keys a
/// advisory [`ObjectHandleFacts::by_scope`] candidate lookup may use.
///
/// `unit` is the [`FunctionUnit`] key (`::proc`, `::Class::method`,
/// `::Class::<constructor>`, `::top`).  `class` is `Some` for a method body —
/// an *instance-variable* name is owned by the class, not by the method that
/// happens to write it, so a consumer resolving a receiver inside a method
/// tries `(unit, var)` first and `(class, var)` second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerSpan {
    /// Byte extent of the owning definition, absolute in the analysed source.
    pub span: Span,
    /// The [`FunctionUnit`] key that owns names written in this extent.
    pub unit: String,
    /// Independent retained body coordinate; labels cannot issue it.
    pub frame: BodyFrameId,
    /// Shared state scope supplied by the actual retained declaration owner.
    pub class_owner: Option<ObjectFactOwner>,
    /// The enclosing class qualified name, for a method/constructor body.
    pub class: Option<String>,
}

/// One object value proved at an original lexical variable read.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectHandleRead {
    /// Extent of the actual variable substitution.
    pub span: Span,
    /// Author-spelled variable name under its selected grammar.
    pub variable: String,
    /// Current class of the actual object allocation reaching this read.
    pub classes: HashSet<String>,
    /// Allocation, producing class incarnation, and current OO dispatch receipt.
    pub instance: crate::command_binding::SourceObjectInstanceProof,
}

/// Physical read of an SSA value carrying an inferred object class. The read
/// receipt does not track the object allocation or its OO dispatch generation,
/// so its class remains candidate evidence and cannot license specialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalObjectTypeReadEvidence {
    source: crate::ir::SourceSite,
    evaluation: crate::command_binding::SourceVariableEvaluationOwner,
    unit: String,
    frame: BodyFrameId,
    variable: String,
    place: crate::place::Place,
    value: crate::ssa::ValueKey,
    class: String,
}

impl PhysicalObjectTypeReadEvidence {
    /// Exact lexical read and its retained provenance.
    #[must_use]
    pub fn source(&self) -> &crate::ir::SourceSite {
        &self.source
    }

    /// Actual evaluation reaching this read, distinct from lexical containment.
    #[must_use]
    pub fn evaluation(&self) -> &crate::command_binding::SourceVariableEvaluationOwner {
        &self.evaluation
    }

    /// Function whose SSA arena represents the retained contents version.
    #[must_use]
    pub fn unit(&self) -> &str {
        &self.unit
    }

    /// Independent retained analytical body of this SSA read.
    #[must_use]
    pub const fn frame(&self) -> BodyFrameId {
        self.frame
    }

    /// Original variable reference normalised by its selected native grammar.
    #[must_use]
    pub fn variable(&self) -> &str {
        &self.variable
    }

    /// Closed physical address and lifetime selected at the read.
    #[must_use]
    pub fn place(&self) -> &crate::place::Place {
        &self.place
    }

    /// Represented SSA contents read at this physical boundary.
    #[must_use]
    pub const fn value(&self) -> crate::ssa::ValueKey {
        self.value
    }

    /// Object allocation and OO dispatch generation remain unproved.
    #[must_use]
    pub const fn object_dispatch_unknown(&self) -> bool {
        true
    }

    /// Inferred SSA class candidate; object reclassification may invalidate it.
    #[must_use]
    pub fn candidate_class(&self) -> &str {
        &self.class
    }
}

/// Owner-attributed object-handle provenance for one
/// [`CompilationUnit`], carried on
/// [`crate::analyser::types::AnalysisResult`].
///
/// Every map is **best-effort**: an absent key means *no evidence was found*,
/// never *proof that the name holds no object*. A consumer that needs a sound
/// "provably a different class" answer must read [`Self::proven_reads`] singletons
/// and treat every other shape as an abstention.
///
/// Soundness directions, by map:
///
/// | map | key | widening risk | safe for |
/// |---|---|---|---|
/// | [`Self::any_scope`] | bare name | same name in two procs collides | highlighting, navigation (labelled) |
/// | [`Self::by_scope`] | `(owner, name)` | all versions are unioned | scoped assistance |
/// | [`Self::proven_reads`] | source substitution | exact SSA contents at the read | edits, references, rename, refusal gates |
/// | [`Self::collections`] | bare name | cross-scope union (deliberate — the cross-method bridge) | highlighting, collection dispatch |
/// | [`Self::returns_object`] | retained body ID | candidate class, no allocation proof | result reporting |
/// | [`Self::global_object_cells`] | `::`-qualified name | all versions and documents may disagree | labelled cross-document assistance |
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObjectHandleFacts {
    report_source: Option<std::sync::Arc<report_comparison::ObjectReportSource>>,
    logical_source_candidates:
        Option<std::sync::Arc<logical_source_candidates::LogicalSourceCandidateOwner>>,
    /// The scope-blind union — **verbatim** what [`object_handle_classes`]
    /// returns, so a consumer can migrate one map at a time.
    pub any_scope: HashMap<String, HashSet<String>>,
    /// The same bindings keyed by `(owner_qualified_name, var)`, where the
    /// owner is the unit the binding edge actually binds in: an alias / proc
    /// return binds in the *assigning* unit, a proc-parameter edge in the
    /// *callee*, a constructor-parameter edge in the *constructor*, and a
    /// class instance variable in the *class* (unioned across its methods —
    /// that union is the interprocedural bridge).
    ///
    /// **Scoped propagation, not just scoped keying.**  Every variable *read*
    /// an edge resolves is resolved in the reading unit's own scope (falling
    /// back to its class for an instance variable, and to nothing otherwise) —
    /// never through [`Self::any_scope`].  Keying the output alone would be
    /// unsound: after `proc a {} { set x [Pin new] }`, the alias in
    /// `proc b {} { set x 0; set y $x }` would read `a`'s `x` and record
    /// `(::b, y) → ::Pin`, a candidate attributed to the wrong unit. Scoped
    /// candidates still union all contents versions and never authorise edits
    /// or a runtime class claim; those require [`Self::proven_reads`].
    /// Cross-*unit* edges are unaffected, because their source is not a scoped
    /// variable read: a proc-return edge's source is the callee's return type,
    /// and a parameter edge resolves its argument in the caller before binding
    /// the parameter in the callee.
    pub by_scope: HashMap<(ObjectFactOwner, String), HashSet<String>>,
    /// Source-positioned runtime proofs, independent of advisory scope unions.
    pub proven_reads: Vec<ObjectHandleRead>,
    /// Physical SSA read receipts with explicit object-dispatch uncertainty.
    pub physical_type_reads: Vec<PhysicalObjectTypeReadEvidence>,
    /// Owning-scope extents, sorted by start (then by end descending) for the
    /// binary search in [`Self::owner_at`].  One entry per procedure and
    /// method, plus a whole-file entry for the top level.
    pub owner_spans: Vec<OwnerSpan>,
    /// [`object_collection_classes`] verbatim: `List`/`Dict` variable → the
    /// classes of its elements.
    pub collections: HashMap<String, HashSet<String>>,
    /// Inferred object class candidate for each retained body.
    /// Duplicate reports retain separate rows; this supplies no object allocation.
    pub returns_object: HashMap<ProcedureId, String>,
    /// Normal result types at their actual implementation source, independently
    /// of class names, object allocations and method dispatch authority.
    normal_procedure_results: crate::type_infer::NormalProcedureResultTypes,
    /// The `::`-qualified subset of [`Self::any_scope`] — object handles that
    /// have a global spelling. Cross-document assistance may join these
    /// candidates; the spelling does not prove the cell's current contents.
    pub global_object_cells: HashMap<String, HashSet<String>>,
}

mod logical_source_candidates;
mod report_comparison;
pub use logical_source_candidates::LogicalSourceReceiverClassCandidates;

impl ObjectHandleFacts {
    /// Inferred normal result of the retained source procedure implementation.
    /// A matching qualified name cannot borrow another declaration's result.
    #[must_use]
    pub fn normal_procedure_result(
        &self,
        target: &crate::command_binding::SourceCommandTarget,
    ) -> Option<&crate::types::TypeLattice> {
        (target.kind == crate::command_binding::BindingKind::Proc).then_some(())?;
        self.normal_procedure_results
            .get(target.implementation_allocation.as_ref()?)
    }

    /// Select a physical type-read receipt at an exact source/evaluation boundary.
    /// This retains object-dispatch uncertainty and cannot prove a runtime class.
    #[must_use]
    pub fn physical_type_read_at(
        &self,
        frame: BodyFrameId,
        source: &crate::ir::SourceSite,
        evaluation: &crate::command_binding::SourceVariableEvaluationOwner,
    ) -> Option<&PhysicalObjectTypeReadEvidence> {
        let mut proofs = self.physical_type_reads.iter().filter(|proof| {
            proof.frame == frame && &proof.source == source && &proof.evaluation == evaluation
        });
        let first = proofs.next()?;
        proofs.all(|proof| proof == first).then_some(first)
    }

    /// The innermost owning scope containing `offset`, or `None` at a byte no
    /// tracked definition covers (top-level code outside every proc/method).
    ///
    /// [`Self::owner_spans`] is sorted by start ascending (then end
    /// *descending*), so the candidates are found by binary search and the walk
    /// back from the partition point returns the last entry that still contains
    /// `offset` — which for the properly-nested definition spans of a Tcl file
    /// is the innermost scope, including when a scope and its enclosing one
    /// begin at the same byte.
    #[must_use]
    pub fn owner_at(&self, offset: u32) -> Option<&OwnerSpan> {
        let upto = self
            .owner_spans
            .partition_point(|o| o.span.start() <= offset);
        self.owner_spans[..upto]
            .iter()
            .rev()
            .find(|o| o.span.end() >= offset)
    }

    /// The actual allocation and dispatch receipt at a retained receiver read.
    /// Missing or conflicting receipts provide no runtime object proof.
    #[must_use]
    pub fn instance_in_scope(
        &self,
        offset: u32,
        var: &str,
    ) -> Option<&crate::command_binding::SourceObjectInstanceProof> {
        let mut reads = self.proven_reads.iter().filter(|read| {
            read.variable == var && read.span.start() <= offset && offset <= read.span.end()
        });
        let first = reads.next()?;
        reads
            .all(|read| read.instance == first.instance)
            .then_some(&first.instance)
    }

    /// The runtime classes proved at the variable substitution containing
    /// `offset`. Missing or conflicting allocation/dispatch receipts remain unknown.
    #[must_use]
    pub fn classes_in_scope(&self, offset: u32, var: &str) -> Option<&HashSet<String>> {
        let instance = self.instance_in_scope(offset, var)?;
        self.proven_reads
            .iter()
            .find(|read| {
                read.variable == var
                    && read.span.start() <= offset
                    && offset <= read.span.end()
                    && &read.instance == instance
            })
            .map(|read| &read.classes)
    }
}

/// Map every variable that holds an object handle to the set of class names it
/// can hold, across the top level, procedures, and method bodies of `cu`.
///
/// Two signals are unioned:
/// - the syntactic `set VAR [Class new|create …]` constructor assignment
///   (reliable for registry-modelled factory commands); and
/// - any SSA value typed `OBJECT(class)` by the type lattice — which now
///   additionally covers a handle *retrieved from an object collection*
///   (`set p [dict get $pins $k]`, `set p [lindex $objs $i]`) via
///   `type_infer`'s container element-typing.
///
/// Keys are the handle text a `$VAR method` dispatch presents once its leading
/// `$` is stripped — a scalar name (`chart`) or an array element (`arr(key)`).
/// The map unions across scopes: a highlight-only consumer does not need
/// per-scope precision, and a variable named `chart` that is a
/// `ticklecharts::chart` in one proc is overwhelmingly one in another.
#[must_use]
pub fn object_handle_classes(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
) -> HashMap<String, HashSet<String>> {
    build_facts(cu, registry, FactMode::AnyScopeOnly)
        .0
        .any_scope
}

/// Original Logical source candidates under the actual analysis and Module.
/// These scope unions retain unknown contents and grant no runtime identity,
/// physical read, completed store, Native entry or executable transformation.
#[must_use]
pub fn logical_source_object_handle_classes(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    analysis: &crate::analyser::AnalysisResult,
) -> Option<HashMap<String, HashSet<String>>> {
    logical_source_candidates::logical_source_unit_input(analysis, cu, registry)?;
    Some(
        build_facts_gated(cu, registry, FactMode::AnyScopeOnly, true, Some(analysis))
            .0
            .any_scope,
    )
}

/// [`object_handle_classes`] **without** the empty-seed fast path — the
/// unconditional propagation walk.
///
/// Exists so the measurement harness (`examples/object_lattice_cost.rs`) and
/// the unit test below can pin the fast path as behaviour-preserving and
/// quantify what it saves on a file with no object seeds.  Never call this
/// from shipping code: it is strictly slower and, by construction, produces
/// the same map.
#[doc(hidden)]
#[must_use]
pub fn object_handle_classes_full_walk(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
) -> HashMap<String, HashSet<String>> {
    build_facts_gated(cu, registry, FactMode::AnyScopeOnly, false, None)
        .0
        .any_scope
}

/// The type-propagation edge a binding came from — the VTA edge taxonomy,
/// carried so the measurement harness can report bindings by edge kind.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectFlowEdge {
    /// `set A $B`.
    Alias,
    /// `set A [factoryProc …]`.
    ProcReturn,
    /// `set A [$obj make …]` — a method-return on an already-typed receiver.
    MethodReturn,
    /// `f $obj` → `f`'s parameter.
    ProcParam,
    /// `C new $obj` → `C`'s constructor parameter.
    CtorParam,
}

/// Shape/cost counters for one lattice build — the measurement gate M1's
/// per-file row (`examples/object_lattice_cost.rs`).  Not a shipping fact.
#[doc(hidden)]
#[derive(Debug, Clone, Default)]
pub struct LatticeStats {
    /// Handles the harvest seeded before any propagation.
    pub seeds: usize,
    /// Fixpoint rounds actually run (0 when the fast path fired).
    pub rounds: u32,
    /// Whether the empty-seed fast path skipped the propagation walk.
    pub fast_path_hit: bool,
    /// Bindings produced per [`ObjectFlowEdge`], summed over all rounds.
    pub alias_bindings: usize,
    /// See [`Self::alias_bindings`].
    pub return_bindings: usize,
    /// See [`Self::alias_bindings`].
    pub method_return_bindings: usize,
    /// See [`Self::alias_bindings`].
    pub param_bindings: usize,
    /// See [`Self::alias_bindings`].
    pub ctor_param_bindings: usize,
}

/// [`object_handle_facts`] plus the build's [`LatticeStats`].
#[doc(hidden)]
#[must_use]
pub fn object_handle_facts_instrumented(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
) -> (ObjectHandleFacts, LatticeStats) {
    build_facts_gated(cu, registry, FactMode::Full, true, None)
}

/// The full owner-attributed object-handle fact set for `cu` — the carrier
/// described on [`ObjectHandleFacts`].
///
/// [`object_handle_classes`] is this entry point restricted to
/// [`ObjectHandleFacts::any_scope`]; the union it returns is byte-identical
/// either way, so the scope-keyed maps are pure addition.  The extra work over
/// the union-only path is the owner attribution, the owner-span index, the
/// collection map, and the two exported sub-facts.
#[must_use]
pub fn object_handle_facts(cu: &CompilationUnit, registry: &CommandRegistry) -> ObjectHandleFacts {
    build_facts(cu, registry, FactMode::Full).0
}

/// Actual source interpretation supplies advisory constructor metadata only.
pub(crate) fn object_handle_facts_for_analysis(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    analysis: &crate::analyser::AnalysisResult,
) -> ObjectHandleFacts {
    build_facts_gated(cu, registry, FactMode::Full, true, Some(analysis)).0
}

/// Union-only candidates skip the scope-keyed maps; full builds retain them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactMode {
    /// Populate [`ObjectHandleFacts::any_scope`] only.
    AnyScopeOnly,
    /// Populate every map.
    Full,
}

fn build_facts(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    mode: FactMode,
) -> (ObjectHandleFacts, LatticeStats) {
    build_facts_gated(cu, registry, mode, true, None)
}

/// The one implementation behind both public entry points.
///
/// `empty_seed_fast_path` is `true` for every production caller; only
/// [`object_handle_classes_full_walk`] passes `false`, to run the
/// unconditional walk for the behaviour-equality test and the cost
/// measurement.
fn build_facts_gated(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    mode: FactMode,
    empty_seed_fast_path: bool,
    source_analysis: Option<&crate::analyser::AnalysisResult>,
) -> (ObjectHandleFacts, LatticeStats) {
    let owners = OwnerIndex::build(cu, mode);
    let context =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &cu.ir_module);
    let mut sink = FactSink {
        facts: ObjectHandleFacts::default(),
        owners: &owners,
        metadata_context: context,
        source_analysis,
        mode,
        saw_constructor_arg: false,
        stats: LatticeStats::default(),
    };
    let units = std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values());
    for fu in units {
        harvest_unit(fu, frame_for_unit(cu, fu), registry, &mut sink);
    }
    // Original source bodies supply factory candidates independently of
    // callable publication and SSA proof.
    for (name, procedure) in cu.ir_module.original_source_procedure_bodies() {
        if context.is_some_and(|context| context.permits_logical_source_names())
            || !cu.procedures.contains_key(name)
        {
            harvest_script_factories(
                &procedure.body,
                Some(BodyFrameId::Body(*name)),
                registry,
                &mut sink,
            );
        }
    }
    for (name, method) in &cu.ir_module.methods {
        if !cu.methods.contains_key(name) {
            harvest_script_factories(
                &method.body,
                Some(BodyFrameId::Body(*name)),
                registry,
                &mut sink,
            );
        }
    }
    sink.stats.seeds = sink.facts.any_scope.len();
    let returns = returning_proc_candidates(cu, registry, source_analysis);
    // Seeds, procedure-return candidates and nested constructor arguments can
    // create flow edges. `[Factory new]` arguments need no existing seed.
    let any_edge_can_fire =
        !sink.facts.any_scope.is_empty() || !returns.is_empty() || sink.saw_constructor_arg;
    sink.stats.fast_path_hit = !any_edge_can_fire && empty_seed_fast_path;
    if any_edge_can_fire || !empty_seed_fast_path {
        // VTA-lite object-flow propagation.  Having seeded the handles that are
        // locally provable (constructor assignments + SSA `OBJECT` values), push
        // those classes along the type-propagation edges of Variable Type
        // Analysis (Sundaresan et al., OOPSLA'00) to a finite fixpoint:
        //   - *aliasing*         `set A $B`            → A ⊇ classes(B)
        //   - *proc return*      `set A [make …]`      → A ⊇ return-class(make)
        //   - *proc parameter*   `f $obj`              → f's param ⊇ classes($obj)
        //   - *constructor param* `C new $obj`         → C's ctor param ⊇ classes($obj)
        // Nodes are name-keyed (field-based, object-insensitive) and joins are
        // set union — the economy VTA trades precision for.  Highlight-only,
        // matching the imprecision tolerance documented above.
        propagate_object_flow(cu, registry, &returns, &mut sink);
    }
    let stats = sink.stats;
    let mut facts = sink.facts;
    if mode == FactMode::Full {
        facts.owner_spans = owners.spans;
        (facts.proven_reads, facts.physical_type_reads) = proven_object_reads(cu, registry);
        facts.returns_object = returning_procs(cu);
        retain_normal_procedure_result_types(cu, registry, &mut facts);
        facts.collections = object_collection_classes(cu);
        facts.global_object_cells = facts
            .any_scope
            .iter()
            .filter(|(name, _)| name.starts_with("::"))
            .map(|(name, classes)| (name.clone(), classes.clone()))
            .collect();
    }
    (facts, stats)
}

fn proven_object_reads(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
) -> (Vec<ObjectHandleRead>, Vec<PhysicalObjectTypeReadEvidence>) {
    let mut candidates: HashMap<
        (Span, String),
        Option<crate::command_binding::SourceObjectInstanceProof>,
    > = HashMap::new();
    let mut physical = Vec::new();
    let document_origin = cu
        .ir_module
        .top_level
        .executed_source
        .as_ref()
        .map(|source| &source.origin);
    for fu in std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values())
    {
        for (&block, cfg_block) in &fu.cfg.blocks {
            for index in (0..cfg_block.statements.len()).chain(std::iter::once(usize::MAX)) {
                let source = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
                let Some(tokens) = source.source_tokens() else {
                    continue;
                };
                for access in &tokens.variable_accesses {
                    let Some(dialect) = access.variable_context.invocation_dialect else {
                        continue;
                    };
                    let name = tcl_syntax::naming::var_reference_for_style(
                        &access.original_spelling,
                        dialect.lexer_grammar.braced_var,
                    )
                    .to_owned();
                    let reference =
                        source.read_reference(&access.source, &access.original_spelling);
                    let classes = reference
                        .and_then(|read| fu.types.get(&(read.symbol, read.version?)))
                        .filter(|value| value.tcl_type() == Some(tcl_registry::TclType::Object))
                        .and_then(crate::types::TypeLattice::class_name)
                        .map(|class| HashSet::from([class.to_owned()]));
                    if let Some(read) = reference
                        && let Some(class) =
                            classes.as_ref().and_then(|classes| classes.iter().next())
                        && let Some(proof) = physical_object_type_read(
                            access,
                            read,
                            &fu.name,
                            frame_for_unit(cu, fu),
                            &name,
                            class,
                            registry,
                        )
                        && !physical.contains(&proof)
                    {
                        physical.push(proof);
                    }
                    // Numeric document offsets cannot select a read in an independently
                    // materialised script, even when its bytes/variable spelling agree.
                    if tokens
                        .source_binding
                        .as_ref()
                        .and_then(|binding| binding.source_origin())
                        != document_origin
                    {
                        continue;
                    }
                    let instance = tokens
                        .source_binding
                        .as_ref()
                        .filter(|binding| binding.retains_object_instance_at_dispatch(access))
                        .and_then(|_| access.proved_object_instance())
                        .cloned();
                    candidates
                        .entry((access.source.span, name))
                        .and_modify(|previous| {
                            if *previous != instance {
                                *previous = None;
                            }
                        })
                        .or_insert(instance);
                }
            }
        }
    }
    let mut reads = candidates
        .into_iter()
        .filter_map(|((span, variable), instance)| {
            let instance = instance?;
            Some(ObjectHandleRead {
                span,
                variable,
                classes: HashSet::from([instance.class_target().command.clone()]),
                instance,
            })
        })
        .collect::<Vec<_>>();
    reads.sort_by_key(|read| (read.span.start(), read.span.end(), read.variable.clone()));
    (reads, physical)
}

fn physical_object_type_read(
    access: &crate::command_binding::SourceVariableAccess,
    reference: crate::ssa::SsaReadReference,
    unit: &str,
    frame: Option<BodyFrameId>,
    variable: &str,
    class: &str,
    registry: &CommandRegistry,
) -> Option<PhysicalObjectTypeReadEvidence> {
    if access.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed
        || matches!(
            access.owner,
            crate::command_binding::SourceVariableEvaluationOwner::Unspecified
        )
    {
        return None;
    }
    let places = access
        .context_alternatives()
        .iter()
        .map(|context| {
            crate::var_resolve::resolve_substitution_access(
                &access.original_spelling,
                context,
                registry,
                tcl_registry::TraceOperation::Read,
            )
        })
        .collect::<Vec<_>>();
    let place = places.first()?;
    if place.dynamic
        || place.observed
        || place.cell.as_ref()?.generation == crate::place::CellGeneration::Unknown
        || places.iter().any(|other| other != place)
    {
        return None;
    }
    Some(PhysicalObjectTypeReadEvidence {
        source: access.source.clone(),
        evaluation: access.owner.clone(),
        unit: unit.to_owned(),
        frame: frame?,
        variable: variable.to_owned(),
        place: place.clone(),
        value: (reference.symbol, reference.version?),
        class: class.to_owned(),
    })
}

/// Which unit — or class — owns a name written inside a given
/// [`FunctionUnit`], plus the sorted span index consumers resolve an offset
/// against.
struct OwnerIndex {
    method_class: HashMap<BodyFrameId, ObjectFactOwner>,
    class_instance_vars: HashMap<ObjectFactOwner, HashSet<String>>,
    spans: Vec<OwnerSpan>,
}

impl OwnerIndex {
    fn shared_empty() -> &'static Self {
        static EMPTY: std::sync::OnceLock<OwnerIndex> = std::sync::OnceLock::new();
        EMPTY.get_or_init(|| Self {
            method_class: HashMap::new(),
            class_instance_vars: HashMap::new(),
            spans: Vec::new(),
        })
    }

    fn build(cu: &CompilationUnit, mode: FactMode) -> Self {
        let mut index = Self {
            method_class: HashMap::new(),
            class_instance_vars: HashMap::new(),
            spans: Vec::new(),
        };
        if mode == FactMode::AnyScopeOnly {
            return index;
        }
        for (id, method) in &cu.ir_module.methods {
            let frame = BodyFrameId::Body(*id);
            let receiver = if method.kind == MethodKind::ClassMethod {
                ObjectScopeReceiver::ClassObject
            } else {
                ObjectScopeReceiver::Instance
            };
            let class_owner = method
                .original_receiver_context
                .as_deref()
                .and_then(|context| context.declaring_allocation())
                .cloned()
                .map(|allocation| ObjectFactOwner::OriginalClass {
                    allocation,
                    receiver,
                })
                .or_else(|| {
                    method
                        .definition_scope
                        .map(|definition| ObjectFactOwner::DefinitionScript {
                            definition,
                            receiver,
                        })
                });
            if let Some(owner) = class_owner.as_ref() {
                index.method_class.insert(frame, owner.clone());
                index
                    .class_instance_vars
                    .entry(owner.clone())
                    .or_default()
                    .extend(method.instance_vars.iter().cloned());
            }
            if let Some(span) = method.span {
                index.spans.push(OwnerSpan {
                    span,
                    unit: method.reporting_label(),
                    frame,
                    class_owner,
                    class: Some(method.class_name.clone()),
                });
            }
        }
        for (id, procedure) in cu.ir_module.original_source_procedure_bodies() {
            index.spans.push(OwnerSpan {
                span: procedure.span,
                unit: procedure.qualified_name.clone(),
                frame: BodyFrameId::Body(*id),
                class_owner: None,
                class: None,
            });
        }
        index.spans.push(OwnerSpan {
            span: Span::new(0, u32::try_from(cu.source.len()).unwrap_or(u32::MAX)),
            unit: cu.top_level.name.clone(),
            frame: BodyFrameId::Top(cu.ir_module.procedure_ids.module_id()),
            class_owner: None,
            class: None,
        });
        index.spans.sort_by(|a, b| {
            (
                a.span.start(),
                std::cmp::Reverse(a.span.end()),
                &a.unit,
                a.frame,
            )
                .cmp(&(
                    b.span.start(),
                    std::cmp::Reverse(b.span.end()),
                    &b.unit,
                    b.frame,
                ))
        });
        index
    }

    fn owner_for(&self, frame: BodyFrameId, name: &str) -> ObjectFactOwner {
        if let Some(owner) = self.method_class.get(&frame)
            && self
                .class_instance_vars
                .get(owner)
                .is_some_and(|vars| vars.contains(name))
        {
            return owner.clone();
        }
        ObjectFactOwner::Frame(frame)
    }
}

/// Accumulator threaded through the harvest and the fixpoint so the union and
/// the scope-keyed map are filled from the same binding events.
struct FactSink<'a> {
    facts: ObjectHandleFacts,
    owners: &'a OwnerIndex,
    metadata_context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    source_analysis: Option<&'a crate::analyser::AnalysisResult>,
    mode: FactMode,
    /// Whether the harvest saw an argument that could be a bracketed registry
    /// constructor — the one type-propagation edge that fires without any
    /// seeded handle.  Over-approximate on purpose; see
    /// [`may_be_constructor_word`].
    saw_constructor_arg: bool,
    /// Cost/shape counters for the measurement gate.
    stats: LatticeStats,
}

impl FactSink<'_> {
    /// Record that `name`, written in `unit`, can hold `class`.
    fn bind(&mut self, frame: Option<BodyFrameId>, name: &str, class: &str) {
        self.facts
            .any_scope
            .entry(name.to_owned())
            .or_default()
            .insert(class.to_owned());
        if self.mode == FactMode::Full
            && let Some(frame) = frame
        {
            let owner = self.owners.owner_for(frame, name);
            self.facts
                .by_scope
                .entry((owner, name.to_owned()))
                .or_default()
                .insert(class.to_owned());
        }
    }

    /// Record whether any of `args` could be the seed-free constructor
    /// argument the fast-path gate must not skip past.
    fn note_constructor_args(&mut self, args: &[String], registry: &CommandRegistry) {
        if self.saw_constructor_arg {
            return;
        }
        self.saw_constructor_arg = args.iter().any(|a| may_be_constructor_word(a, registry));
    }
}

/// Callee proc qualified name → the class of the object it returns (the
/// factory-proc signal behind `set c [makeThing]`).
fn retain_normal_procedure_result_types(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    facts: &mut ObjectHandleFacts,
) {
    let Some(known_classes) = cu.retained_source_class_names(registry) else {
        facts.normal_procedure_results.clear();
        return;
    };
    let units = procedure_implementation_units(cu);
    let mut results = crate::type_infer::NormalProcedureResultTypes::new();
    for (allocation, unit) in &units {
        join_procedure_result(&mut results, allocation, unit.return_type.clone());
    }
    // Every round propagates an already inferred normal callee result through
    // one more body. Cycles without an independent result remain overdefined.
    for _ in 0..=units.len() {
        let mut next = crate::type_infer::NormalProcedureResultTypes::new();
        for (allocation, unit) in &units {
            let Some(metadata) = crate::type_infer::TypePropagationMetadata::for_function(
                unit,
                cu.ir_module.resolved_registry(),
            ) else {
                join_procedure_result(&mut next, allocation, crate::types::TypeLattice::unknown());
                continue;
            };
            let result = crate::type_infer::infer_function_return_type_with_results(
                &unit.cfg,
                &unit.sccp,
                &unit.types,
                metadata,
                known_classes,
                &unit.ssa,
                Some(&results),
            );
            join_procedure_result(&mut next, allocation, result);
        }
        if next == results {
            break;
        }
        results = next;
    }
    facts.normal_procedure_results = results;
}

fn join_procedure_result(
    results: &mut crate::type_infer::NormalProcedureResultTypes,
    allocation: &crate::command_binding::CommandAllocation,
    result: crate::types::TypeLattice,
) {
    results
        .entry(allocation.clone())
        .and_modify(|previous| *previous = crate::types::type_join(previous, &result))
        .or_insert(result);
}

/// Match generic analysis units to the source owner's original body inventory.
/// An allocation site or procedure name alone cannot select a replacement body.
fn procedure_implementation_units(
    cu: &CompilationUnit,
) -> Vec<(&crate::command_binding::CommandAllocation, &FunctionUnit)> {
    let mut units = Vec::new();
    for implementation in cu.ir_module.procedure_implementation_bodies.iter() {
        let parameters = implementation
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>();
        for (name, unit) in cu.procedures.iter().chain(&cu.body_units) {
            let procedure = cu
                .ir_module
                .procedures
                .get(name)
                .or_else(|| cu.ir_module.body_units.get(name));
            let Some(procedure) = procedure else {
                continue;
            };
            if procedure.body.executed_source.as_deref() != Some(&implementation.source)
                || procedure.body.namespace_context.as_deref()
                    != Some(&implementation.namespace_key)
                || procedure
                    .params
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    != parameters
            {
                continue;
            }
            if let Some(allocation) = cu
                .ir_module
                .installed_procedure_body_units
                .get(name)
                .or_else(|| cu.ir_module.original_declaration_body_units.get(name))
                && allocation != &implementation.allocation
            {
                continue;
            }
            units.push((&implementation.allocation, unit));
        }
    }
    units
}

fn returning_procs(cu: &CompilationUnit) -> HashMap<ProcedureId, String> {
    cu.procedures
        .values()
        .filter_map(|unit| {
            (unit.return_type.tcl_type() == Some(tcl_registry::TclType::Object))
                .then_some(unit.return_type.class_name())
                .flatten()
                .and_then(|class| unit.body_id.map(|id| (id, class.to_owned())))
        })
        .collect()
}

/// Advisory factory classes, retaining an unknown return-path residual.
/// These flow candidates never populate the strict returned-object type map.
fn returning_proc_candidates<'a>(
    cu: &'a CompilationUnit,
    registry: &CommandRegistry,
    source_analysis: Option<&crate::analyser::AnalysisResult>,
) -> HashMap<ProcedureId, String> {
    let mut returns = returning_procs(cu);
    let Some(context) =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &cu.ir_module)
    else {
        return returns;
    };
    let metadata = Some(context);
    // A retained generic body can have original constructor evidence without
    // an executable FunctionUnit. This is a class candidate only.
    for (id, procedure) in cu.ir_module.original_source_procedure_bodies() {
        if returns.contains_key(id) {
            continue;
        }
        let Some(statement) = procedure.body.statements.last() else {
            continue;
        };
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(statement);
        let candidates = tokens
            .map(|tokens| {
                crate::registry_invocation::advisory_return_values_with_metadata_context(
                    registry, metadata, tokens,
                )
            })
            .unwrap_or_default();
        for value in candidates {
            if let Some(class) = constructor_class(
                &value.legacy_text(),
                registry,
                tokens,
                metadata,
                source_analysis,
            ) {
                returns.insert(*id, class);
            }
        }
    }
    returns
}

/// Method [`FunctionUnit`] key (`::Class::method`) → the class of the object
/// it returns — the method-return counterpart of [`returning_procs`], behind
/// the `set b [$a make]` edge.
///
/// Direct declarations only: a receiver class that merely *inherits* the
/// method resolves nothing here (the lattice carries no MRO), so the edge
/// abstains rather than guess — the module-wide "no evidence ≠ no object"
/// contract.
fn returning_methods(cu: &CompilationUnit) -> HashMap<ProcedureId, &str> {
    cu.methods
        .iter()
        .filter_map(|(key, fu)| {
            (fu.return_type.tcl_type() == Some(tcl_registry::TclType::Object))
                .then_some(fu.return_type.class_name())
                .flatten()
                .map(|c| (*key, c))
        })
        .collect()
}

/// VTA-lite object-flow fixpoint.  Propagates object classes from the seeded
/// handles along four kinds of type-propagation edge — assignment (aliasing),
/// proc return, proc parameter, and constructor parameter — until no new class
/// reaches any node.  Nodes are name-keyed (a variable / parameter is one node
/// regardless of scope, VTA's field-based object-insensitive economy) and the
/// join is set union.  Highlight-only and union-imprecise, matching the rest of
/// this module.
fn propagate_object_flow<'a>(
    cu: &'a CompilationUnit,
    registry: &CommandRegistry,
    returns: &HashMap<ProcedureId, String>,
    sink: &mut FactSink,
) {
    let proc_params: HashMap<ProcedureId, &[String]> = cu
        .ir_module
        .procedures
        .iter()
        .map(|(id, procedure)| (*id, procedure.params.as_slice()))
        .collect();
    let ctor_params: HashMap<&str, Vec<(ProcedureId, &crate::ir::MethodDef)>> = {
        let mut constructors: HashMap<&str, Vec<(ProcedureId, &crate::ir::MethodDef)>> =
            HashMap::new();
        for (id, method) in &cu.ir_module.methods {
            if method.kind == MethodKind::Constructor {
                constructors
                    .entry(method.class_name.as_str())
                    .or_default()
                    .push((*id, method));
            }
        }
        constructors
    };
    let mut method_coordinates: HashMap<MethodCoordinate, Vec<ProcedureId>> = HashMap::new();
    for (id, method) in &cu.ir_module.methods {
        method_coordinates
            .entry(method.coordinate())
            .or_default()
            .push(*id);
    }
    let method_returns = returning_methods(cu);
    if returns.is_empty()
        && proc_params.is_empty()
        && ctor_params.is_empty()
        && method_returns.is_empty()
        && sink.facts.any_scope.is_empty()
    {
        return;
    }
    let index = FlowIndex {
        returns: returns.clone(),
        method_returns,
        method_coordinates,
        module: &cu.ir_module,
        proc_params,
        ctor_params,
    };

    // This is a finite, monotone domain: existing binding sites receive only
    // class identities already present in the seeds or return summaries.
    // Cycles converge when no set grows; a private round limit would lose
    // evidence along longer chains. Both scoped and union facts participate
    // in convergence, while reads retain their own scope attribution.
    let scoped = (sink.mode == FactMode::Full).then_some(sink.owners);
    loop {
        let bindings = scan_flow_edges(
            cu,
            registry,
            &sink.facts,
            scoped,
            &index,
            sink.source_analysis,
        );
        sink.stats.rounds += 1;
        let mut changed = false;
        for binding in bindings {
            match binding.kind {
                ObjectFlowEdge::Alias => sink.stats.alias_bindings += 1,
                ObjectFlowEdge::ProcReturn => sink.stats.return_bindings += 1,
                ObjectFlowEdge::MethodReturn => sink.stats.method_return_bindings += 1,
                ObjectFlowEdge::ProcParam => sink.stats.param_bindings += 1,
                ObjectFlowEdge::CtorParam => sink.stats.ctor_param_bindings += 1,
            }
            let entry = sink
                .facts
                .any_scope
                .entry(binding.name.clone())
                .or_default();
            for c in &binding.union_classes {
                changed |= entry.insert(c.clone());
            }
            if sink.mode == FactMode::Full && !binding.scoped_classes.is_empty() {
                let owner = sink.owners.owner_for(binding.owner, &binding.name);
                let entry = sink
                    .facts
                    .by_scope
                    .entry((owner, binding.name))
                    .or_default();
                // The scope-keyed fact can still be growing after the union has
                // settled (it lags by the round its source needed), so it drives
                // the loop too.  Extra rounds cannot change `any_scope`: unioning
                // a converged fixpoint with itself is a no-op.
                for c in binding.scoped_classes {
                    changed |= entry.insert(c);
                }
            }
        }
        if !changed {
            break;
        }
    }
}

/// The callee-side maps one fixpoint round resolves call edges against.
struct FlowIndex<'a> {
    /// Proc qualified name → returned object class.
    returns: HashMap<ProcedureId, String>,
    /// Method [`FunctionUnit`] key (`::Class::method`) → returned object
    /// class, for the `set b [$a make]` method-return edge.
    method_returns: HashMap<ProcedureId, &'a str>,
    method_coordinates: HashMap<MethodCoordinate, Vec<ProcedureId>>,
    module: &'a crate::ir::Module,
    /// Proc qualified name → parameter names.
    proc_params: HashMap<ProcedureId, &'a [String]>,
    /// Class qualified name → (constructor unit key, constructor parameters).
    ctor_params: HashMap<&'a str, Vec<(ProcedureId, &'a crate::ir::MethodDef)>>,
}

/// One type-propagation edge's product: the classes `name` gains, and the unit
/// the edge binds that name **in** (the assigning unit for an alias / return,
/// the callee for a parameter, the constructor for a constructor parameter).
///
/// The two class sets are the two facts the module maintains, and they are
/// **not** the same set:
///
/// * `union_classes` resolves the edge's source against the scope-blind
///   [`ObjectHandleFacts::any_scope`] map, and feeds it back — the deliberate,
///   documented highlight-grade imprecision this module has always had.
/// * `scoped_classes` resolves the edge's source *in the scope that owns it*
///   and feeds [`ObjectHandleFacts::by_scope`].  Empty means "no evidence in
///   the owning scope", which is the whole point: a `set y $x` in one proc
///   must not read a same-named `x` bound in another: a false singleton in
///   the narrow map is a wrong rename, not a missed one.
struct Binding {
    owner: BodyFrameId,
    name: String,
    union_classes: HashSet<String>,
    scoped_classes: HashSet<String>,
    kind: ObjectFlowEdge,
}

/// The two class sets one edge yields — see [`Binding`].
struct EdgeClasses {
    union: HashSet<String>,
    scoped: HashSet<String>,
}

impl EdgeClasses {
    /// An edge whose source is **unit-independent** (a callee's return type, a
    /// literal `[Class new]` argument): both facts agree, because there is no
    /// variable read to scope.
    fn unscoped(class: &str) -> Self {
        let one: HashSet<String> = std::iter::once(class.to_owned()).collect();
        Self {
            union: one.clone(),
            scoped: one,
        }
    }
}

/// One round of the VTA-lite fixpoint: scan every statement, reading current
/// node classes from `out`, and return the `(node, classes)` bindings the
/// type-propagation edges imply this round.  The caller unions them into `out`
/// and iterates until nothing changes.
fn scan_flow_edges(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    facts: &ObjectHandleFacts,
    scoped: Option<&OwnerIndex>,
    index: &FlowIndex,
    source_analysis: Option<&crate::analyser::AnalysisResult>,
) -> Vec<Binding> {
    let metadata_context =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &cu.ir_module);
    let ctor_params = &index.ctor_params;
    let resolve_ctor_class =
        |head: &str| -> Option<String> { ctor_params.contains_key(head).then(|| head.to_owned()) };

    let mut bindings: Vec<Binding> = Vec::new();
    let units = std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values());
    // A `FactMode::AnyScopeOnly` build has no scope-keyed fact to maintain, so
    // it never pays for the scoped lookups; the empty index only satisfies the
    // shared context type.
    let owners: &OwnerIndex = scoped.unwrap_or_else(|| OwnerIndex::shared_empty());
    let by_scope = scoped.map(|_| &facts.by_scope);
    for fu in units {
        let Some(frame) = frame_for_unit(cu, fu) else {
            continue;
        };
        let ctx = ScanContext {
            ctor_params,
            out: &facts.any_scope,
            by_scope,
            owners,
            registry,
            metadata_context,
            source_analysis,
            unit: frame,
            tokens: None,
        };
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let ctx = ScanContext {
                    tokens: stmt.tokens(),
                    ..ctx
                };
                scan_flow_statement(stmt, ctx, index, &resolve_ctor_class, &mut bindings);
            }
        }
    }
    for (unit, script) in cu
        .ir_module
        .original_source_procedure_bodies()
        .filter(|(name, _)| {
            metadata_context.is_some_and(|context| context.permits_logical_source_names())
                || !cu.procedures.contains_key(*name)
        })
        .map(|(id, procedure)| (BodyFrameId::Body(*id), &procedure.body))
        .chain(
            cu.ir_module
                .methods
                .iter()
                .filter(|(name, _)| !cu.methods.contains_key(*name))
                .map(|(id, method)| (BodyFrameId::Body(*id), &method.body)),
        )
    {
        let mut pending = vec![script];
        while let Some(script) = pending.pop() {
            for statement in &script.statements {
                let ctx = ScanContext {
                    ctor_params,
                    out: &facts.any_scope,
                    by_scope,
                    owners,
                    registry,
                    metadata_context,
                    source_analysis,
                    unit,
                    tokens: script.retained_source_tokens_for_statement(statement),
                };
                scan_flow_statement(statement, ctx, index, &resolve_ctor_class, &mut bindings);
                pending.extend(statement.child_scripts());
            }
        }
    }

    bindings
}

/// One advisory flow step, shared by executable CFG and retained generic IR.
fn scan_flow_statement(
    stmt: &Statement,
    ctx: ScanContext<'_>,
    index: &FlowIndex<'_>,
    resolve_ctor_class: &impl Fn(&str) -> Option<String>,
    bindings: &mut Vec<Binding>,
) {
    // Argument substitutions precede the parent handler. Their retained
    // constructor entries remain possible even when that handler's normal
    // assignment protocol is unknown after the child returns.
    scan_substitution_constructor_edges(resolve_ctor_class, ctx, bindings);
    let FlowIndex {
        returns,
        method_returns,
        proc_params,
        ..
    } = index;
    match stmt {
        Statement::AssignValue { name, value, .. } => {
            let Some(value) = ctx
                .tokens
                .and_then(|tokens| unique_original_value_word(tokens, value))
            else {
                return;
            };
            scan_assign_edges(
                AssignSite {
                    unit: ctx.unit,
                    name,
                    value,
                },
                ReturnEdges {
                    procs: returns,
                    methods: method_returns,
                    coordinates: &index.method_coordinates,
                    module: index.module,
                },
                ctx,
                bindings,
            );
        }
        Statement::Call { .. } => {
            for assignment in ctx.tokens.map_or_else(Vec::new, |tokens| {
                source_assignment_candidates(ctx.registry, ctx.metadata_context, tokens)
            }) {
                scan_assign_edges(
                    AssignSite {
                        unit: ctx.unit,
                        name: &assignment.name,
                        value: &assignment.value,
                    },
                    ReturnEdges {
                        procs: returns,
                        methods: method_returns,
                        coordinates: &index.method_coordinates,
                        module: index.module,
                    },
                    ctx,
                    bindings,
                );
            }
            // Proc-parameter edge: `f $obj` binds f's params — in
            // the *callee*, which is where the name lives.
            if let Some(target) = ctx
                .tokens
                .and_then(|tokens| tokens.source_binding.as_ref())
                .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target)
                .filter(|target| target.kind == crate::command_binding::BindingKind::Proc)
                && let Some((id, _)) = index.module.source_procedure_for_target(target)
                && let Some(params) = proc_params.get(&id)
                && let Some(effective) = ctx
                    .tokens
                    .and_then(crate::registry_invocation::effective_command_words)
            {
                emit_proc_param_bindings(id, params, &effective.words[1..], ctx, bindings);
            }
            // Constructor-parameter edge: `Class create NAME …`.
            emit_ctor_param_bindings(resolve_ctor_class, ctx, bindings);
        }
        _ => {}
    }
}

fn scan_substitution_constructor_edges(
    resolve_ctor_class: &impl Fn(&str) -> Option<String>,
    ctx: ScanContext<'_>,
    bindings: &mut Vec<Binding>,
) {
    let Some(parent) = ctx.tokens else {
        return;
    };
    let mut pending = vec![parent.clone()];
    while let Some(parent) = pending.pop() {
        let config =
            parent.native_lexer_config(tcl_lexer::LexerConfig::for_profile(ctx.registry.profile()));
        for word in parent.words() {
            let Some(children) =
                crate::value_shapes::command_substitution_tokens(word, Some(&parent), config)
            else {
                continue;
            };
            for child in children {
                if child.source_binding.as_ref().is_none_or(|binding| {
                    binding.runtime_reachability()
                        == crate::command_binding::SourceRuntimeReachability::NotEntered
                }) {
                    continue;
                }
                emit_ctor_param_bindings(
                    resolve_ctor_class,
                    ScanContext {
                        tokens: Some(&child),
                        ..ctx
                    },
                    bindings,
                );
                pending.push(child);
            }
        }
    }
}

/// One `set NAME VALUE` statement, with the unit it is written in.
#[derive(Clone, Copy)]
struct AssignSite<'a> {
    unit: BodyFrameId,
    name: &'a str,
    value: &'a WordExpr,
}

/// The callee-side return-type maps [`scan_assign_edges`] resolves a
/// bracketed value's head against: [`FlowIndex::returns`] and
/// [`FlowIndex::method_returns`].
#[derive(Clone, Copy)]
struct ReturnEdges<'a, 'b> {
    procs: &'b HashMap<ProcedureId, String>,
    methods: &'b HashMap<ProcedureId, &'a str>,
    coordinates: &'b HashMap<MethodCoordinate, Vec<ProcedureId>>,
    module: &'a crate::ir::Module,
}

/// The edges an assignment can carry — aliasing (`set A $B`), proc return
/// (`set A [make …]`), and method return (`set A [$obj make …]`).
/// Assignment edges bind in the assigning unit; child constructor edges
/// are scanned independently before this normal assignment projection.
fn scan_assign_edges(
    site: AssignSite,
    returns: ReturnEdges<'_, '_>,
    ctx: ScanContext,
    bindings: &mut Vec<Binding>,
) {
    if let Some(src) = deref_arg_var(site.value, ctx)
        && let Some(classes) = ctx.out.get(src).filter(|s| !s.is_empty())
    {
        bindings.push(Binding {
            owner: site.unit,
            name: site.name.to_owned(),
            union_classes: classes.clone(),
            scoped_classes: ctx.scoped_classes(src),
            kind: ObjectFlowEdge::Alias,
        });
    }
    let Some(nested) = ctx
        .tokens
        .and_then(|parent| sole_substitution_tokens(site.value, parent, ctx.registry))
    else {
        return;
    };
    let target = nested
        .source_binding
        .as_ref()
        .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target);
    // Return flow follows the retained implementation token, independently
    // of its written spelling or present command slot.
    if let Some(class) = target
        .filter(|target| target.kind == crate::command_binding::BindingKind::Proc)
        .and_then(|target| returns.module.source_procedure_for_target(target))
        .and_then(|(id, _)| returns.procs.get(&id))
    {
        // A legitimately cross-scope flow: the source is the *callee's* return
        // type, not a variable read, so there is no scope to confuse and both
        // facts take it.
        let classes = EdgeClasses::unscoped(class);
        bindings.push(Binding {
            owner: site.unit,
            name: site.name.to_owned(),
            union_classes: classes.union,
            scoped_classes: classes.scoped,
            kind: ObjectFlowEdge::ProcReturn,
        });
    }
    // Method-return edge: `set B [$a make …]` — the receiver's
    // classes are already tracked, and a directly-declared `::Class::make`
    // whose own return type names an object class types the captured handle,
    // exactly like a proc return.  The *receiver* is a variable read in this
    // unit, so the scope-keyed fact resolves it here (the union fact stays
    // blind, as for the aliasing edge). The selected grammar must decode a
    // literal member; computed member words supply no coordinate.
    let words = nested.words();
    if let Some(recv) = words.first().and_then(|word| {
        deref_arg_var(
            word,
            ScanContext {
                tokens: Some(&nested),
                ..ctx
            },
        )
    }) && let Some(method) = words
        .get(1)
        .and_then(|word| literal_source_word(word, &nested, ctx.registry))
    {
        let resolve = |classes: &HashSet<String>| -> HashSet<String> {
            classes
                .iter()
                .flat_map(|class| {
                    returns
                        .coordinates
                        .get(&MethodCoordinate {
                            class_label: class.clone(),
                            member_label: method.to_owned(),
                            kind: MethodKind::Method,
                        })
                        .into_iter()
                        .flatten()
                })
                .filter_map(|id| returns.methods.get(id))
                .map(|ret| (*ret).to_owned())
                .collect()
        };
        if let Some(recv_classes) = ctx.out.get(recv).filter(|s| !s.is_empty()) {
            let union_classes = resolve(recv_classes);
            if !union_classes.is_empty() {
                bindings.push(Binding {
                    owner: site.unit,
                    name: site.name.to_owned(),
                    union_classes,
                    scoped_classes: resolve(&ctx.scoped_classes(recv)),
                    kind: ObjectFlowEdge::MethodReturn,
                });
            }
        }
    }
}

/// Bind `callee`'s parameters to the object classes of a call's arguments.
/// Stops at `args` (a variadic tail is not positionally bindable).
fn emit_proc_param_bindings(
    callee: ProcedureId,
    params: &[String],
    args: &[WordExpr],
    ctx: ScanContext,
    bindings: &mut Vec<Binding>,
) {
    for (i, arg) in args.iter().enumerate() {
        let Some(pname) = params.get(i) else { break };
        if pname == "args" {
            break;
        }
        // Also legitimately cross-scope: the argument is resolved in the
        // *caller* (`ctx.unit`) and the parameter is bound in the callee.
        if let Some(classes) = arg_classes(arg, ctx) {
            bindings.push(Binding {
                owner: BodyFrameId::Body(callee),
                name: pname.clone(),
                union_classes: classes.union,
                scoped_classes: classes.scoped,
                kind: ObjectFlowEdge::ProcParam,
            });
        }
    }
}

/// The read-only context one scan round resolves an argument's classes
/// against.
///
/// `out` is the scope-blind union; `by_scope` (present only in
/// [`FactMode::Full`]) is the scope-keyed map the *narrow* fact is resolved
/// through.  `unit` is the [`FunctionUnit`] the statement being scanned lives
/// in — the scope a `$var` read in it resolves against.
#[derive(Clone, Copy)]
struct ScanContext<'a> {
    ctor_params: &'a HashMap<&'a str, Vec<(ProcedureId, &'a crate::ir::MethodDef)>>,
    out: &'a HashMap<String, HashSet<String>>,
    by_scope: Option<&'a HashMap<(ObjectFactOwner, String), HashSet<String>>>,
    owners: &'a OwnerIndex,
    registry: &'a CommandRegistry,
    metadata_context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    source_analysis: Option<&'a crate::analyser::AnalysisResult>,
    unit: BodyFrameId,
    tokens: Option<&'a crate::ir::CommandTokens>,
}

impl ScanContext<'_> {
    /// The classes `var` holds **in the unit being scanned** — the owning
    /// unit's binding, or its class's when `var` is one of that class's
    /// instance variables (the cross-method bridge).
    ///
    /// Empty when there is no binding in that scope, which is exactly what
    /// stops one unit's `x` from flowing into another's.
    fn scoped_classes(&self, var: &str) -> HashSet<String> {
        let Some(by_scope) = self.by_scope else {
            return HashSet::new();
        };
        let owner = self.owners.owner_for(self.unit, var);
        by_scope
            .get(&(owner, var.to_owned()))
            .cloned()
            .unwrap_or_default()
    }
}

/// Bind a constructor's parameters to the object classes of its arguments.
/// The registry's manufacturer descriptor decides how many leading structural
/// words precede the constructor payload; the flow pass never names a
/// manufacturer keyword.
fn emit_ctor_param_bindings(
    resolve_ctor_class: &impl Fn(&str) -> Option<String>,
    ctx: ScanContext,
    bindings: &mut Vec<Binding>,
) {
    let Some(tokens) = ctx.tokens else {
        return;
    };
    let Some((target, entry, payload_from)) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.constructor_entry(ctx.registry))
    else {
        return;
    };
    let Some(effective) = crate::registry_invocation::effective_command_words(tokens) else {
        return;
    };
    let Some(ctor_args) = effective.words.get(1 + payload_from..) else {
        return;
    };
    if ctor_args.is_empty() {
        return;
    }
    let Some(class) = resolve_ctor_class(&target.command) else {
        return;
    };
    let Some(constructors) = ctx.ctor_params.get(class.as_str()) else {
        return;
    };
    let mut matching = constructors.iter().filter(|(_, method)| {
        method
            .span
            .is_some_and(|span| span.start() == entry.declaration().offset)
            && method.body.executed_source.as_deref() == Some(entry.body())
            && method
                .params
                .iter()
                .map(String::as_str)
                .eq(entry.formals().iter().map(|(name, _)| name.as_str()))
    });
    let Some((ctor_unit, method)) = matching.next() else {
        return;
    };
    if matching.next().is_some() {
        return;
    }
    // The retained body entry chooses the unit. A document-final class name
    // cannot substitute a different constructor with equal parameter names.
    if method
        .span
        .is_none_or(|span| span.start() != entry.declaration().offset)
        || method.body.executed_source.as_deref() != Some(entry.body())
        || method
            .params
            .iter()
            .map(String::as_str)
            .ne(entry.formals().iter().map(|(name, _)| name.as_str()))
    {
        return;
    }
    for (i, arg) in ctor_args.iter().enumerate() {
        let Some((pname, _)) = entry.formals().get(i) else {
            break;
        };
        if pname == "args" {
            break;
        }
        if let Some(classes) = arg_classes(arg, ctx) {
            bindings.push(Binding {
                owner: BodyFrameId::Body(*ctor_unit),
                name: pname.clone(),
                union_classes: classes.union,
                scoped_classes: classes.scoped,
                kind: ObjectFlowEdge::CtorParam,
            });
        }
    }
}

/// The object classes an argument denotes: a tracked `$var` handle, or a direct
/// `[Class new]` registry constructor.  `None` when the argument is not a known
/// object.
///
/// A `$var` argument is resolved twice — blind for the union fact, and in
/// `ctx`'s own unit for the scope-keyed fact.  A literal constructor argument
/// is unit-independent, so both agree.  `by_scope` is a subset of `any_scope`
/// per name by construction, so an empty union implies an empty scoped set and
/// the blind lookup can gate both.
fn arg_classes(arg: &WordExpr, ctx: ScanContext) -> Option<EdgeClasses> {
    if let Some(var) = deref_arg_var(arg, ctx)
        && let Some(classes) = ctx.out.get(var).filter(|s| !s.is_empty())
    {
        return Some(EdgeClasses {
            union: classes.clone(),
            scoped: ctx.scoped_classes(var),
        });
    }
    let parent = ctx.tokens?;
    let tokens = sole_substitution_tokens(arg, parent, ctx.registry)?;
    constructor_class_candidate(
        &tokens,
        ctx.registry,
        ctx.metadata_context,
        ctx.source_analysis,
    )
    .map(|class| EdgeClasses::unscoped(&class))
}

fn unique_original_value_word<'a>(
    tokens: &'a crate::ir::CommandTokens,
    compatibility: &str,
) -> Option<&'a WordExpr> {
    let mut matching = tokens
        .words()
        .iter()
        .skip(1)
        .filter(|word| word.legacy_text() == compatibility);
    let word = matching.next()?;
    matching.next().is_none().then_some(word)
}

fn deref_arg_var<'a>(word: &'a WordExpr, ctx: ScanContext<'_>) -> Option<&'a str> {
    let (spelling, _) = word.sole_variable_substitution()?;
    let parent = ctx.tokens?;
    let config =
        parent.native_lexer_config(tcl_lexer::LexerConfig::for_profile(ctx.registry.profile()));
    let reference = tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config).ok()??;
    if reference.index.is_some()
        || tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
    {
        return None;
    }
    std::str::from_utf8(reference.name).ok()
}

fn literal_source_word(
    word: &WordExpr,
    parent: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
) -> Option<String> {
    let config =
        parent.native_lexer_config(tcl_lexer::LexerConfig::for_profile(registry.profile()));
    let value = crate::registry_invocation::effective_invocation_word(
        word,
        config.escapes,
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
    );
    std::str::from_utf8(value.literal_bytes()?)
        .ok()
        .map(str::to_owned)
}

/// Map every variable that holds an object *collection* — a `List`/`Dict`
/// whose elements are all `OBJECT(class)` — to the set of element class names,
/// read out of the SSA type lattice across the top level, procedures, and
/// method bodies of `cu`.
///
/// Keys are SSA variable names.  The map unions across scopes, which is the
/// interprocedural bridge the intraprocedural lattice cannot make on its own:
/// a `Pins` instance variable filled with `[Pin new]` handles in one method is
/// thereby known to be a `Dict` of `Pin` at a `[dict get $Pins $k] method …`
/// dispatch in a *different* method.  Highlight-only, matching the
/// imprecision tolerance of
/// [`object_handle_classes`].
#[must_use]
pub fn object_collection_classes(cu: &CompilationUnit) -> HashMap<String, HashSet<String>> {
    let mut out: HashMap<String, HashSet<String>> = HashMap::new();
    let units = std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values());
    for fu in units {
        for ((sym, _ver), t) in fu.types.iter() {
            if let Some(class) = t.element_class() {
                out.entry(fu.ssa.var_name(*sym).to_owned())
                    .or_default()
                    .insert(class.to_owned());
            }
        }
    }
    out
}

fn harvest_unit(
    fu: &FunctionUnit,
    frame: Option<BodyFrameId>,
    registry: &CommandRegistry,
    sink: &mut FactSink,
) {
    // Syntactic constructor assignments (`set VAR [Class new|create …]`) and
    // naming object-factories (`struct::graph myG` — the created object command
    // is `myG`, not a `set` target the SSA lattice can carry).
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignValue { name, value, .. } => {
                    if let Some((_head, args)) = parse_command_substitution_with_config(
                        value.trim(),
                        tcl_lexer::LexerConfig::for_profile(registry.profile()),
                    ) {
                        if let Some(class) = constructor_class(
                            value,
                            registry,
                            stmt.tokens(),
                            sink.metadata_context,
                            sink.source_analysis,
                        ) {
                            sink.bind(frame, name, &class);
                        }
                        // The nested-constructor half of the fast-path gate:
                        // `set w [Wrap new [listbox .l]]` reaches `arg_classes`
                        // through `emit_ctor_param_bindings`.
                        sink.note_constructor_args(&args, registry);
                    }
                }
                // Source advice uses the selected descriptor's naming layout;
                // normal handler candidates retain their independent proof.
                Statement::Call { args, .. } => {
                    if let Some(tokens) = stmt.tokens() {
                        harvest_source_command_candidates(tokens, frame, registry, sink);
                    }
                    // Both parameter edges read a call's arguments through
                    // `arg_classes`, whose `[Factory new]` branch needs no
                    // seeded handle — so the fast-path gate must see them.
                    sink.note_constructor_args(args, registry);
                }
                _ => {}
            }
        }
    }
    // SSA values typed `OBJECT(class)` — includes collection retrievals
    // (`set p [dict get $pins $k]`) the syntactic scan above cannot see.
    for ((sym, _ver), t) in fu.types.iter() {
        if t.tcl_type() == Some(tcl_registry::TclType::Object)
            && let Some(class) = t.class_name()
        {
            let name = fu.ssa.var_name(*sym).to_owned();
            sink.bind(frame, &name, class);
        }
    }
}

fn harvest_script_factories(
    script: &crate::ir::Script,
    owner: Option<BodyFrameId>,
    registry: &CommandRegistry,
    sink: &mut FactSink,
) {
    let mut pending = vec![script];
    while let Some(script) = pending.pop() {
        for statement in &script.statements {
            if let Some(tokens) = script.retained_source_tokens_for_statement(statement) {
                harvest_source_command_candidates(tokens, owner, registry, sink);
            }
            pending.extend(statement.child_scripts());
        }
    }
}

fn harvest_source_command_candidates(
    tokens: &crate::ir::CommandTokens,
    owner: Option<BodyFrameId>,
    registry: &CommandRegistry,
    sink: &mut FactSink,
) {
    for assignment in source_assignment_candidates(registry, sink.metadata_context, tokens) {
        if let Some(class) =
            sole_substitution_tokens(&assignment.value, tokens, registry).and_then(|child| {
                constructor_class_candidate(
                    &child,
                    registry,
                    sink.metadata_context,
                    sink.source_analysis,
                )
            })
        {
            sink.bind(owner, &assignment.name, &class);
        }
    }
    for factory in source_object_factory_candidates(registry, sink.metadata_context, tokens) {
        if let Some((name, class)) = factory.publication {
            sink.bind(owner, &name, class);
        }
    }
}

fn source_assignment_candidates(
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    tokens: &crate::ir::CommandTokens,
) -> Vec<crate::registry_invocation::AdvisoryValueAssignment> {
    let Some(context) = context else {
        return Vec::new();
    };
    if context.permits_logical_source_names() {
        return crate::registry_invocation::original_advisory_value_assignments_with_metadata_context(
            registry,
            Some(context),
            tokens,
        );
    }
    source_normal_representation(registry, Some(context), tokens)
        .and_then(|normal| normal.value_assignment())
        .map(
            |assignment| crate::registry_invocation::AdvisoryValueAssignment {
                name: assignment.name,
                value: assignment.value,
            },
        )
        .into_iter()
        .collect()
}

fn source_object_factory_candidates(
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    tokens: &crate::ir::CommandTokens,
) -> Vec<crate::registry_invocation::AdvisoryObjectFactory> {
    let Some(context) = context else {
        return Vec::new();
    };
    if context.permits_logical_source_names() {
        return crate::registry_invocation::advisory_object_factories_with_metadata_context(
            registry, context, tokens,
        );
    }
    let Some(normal) = source_normal_representation(registry, Some(context), tokens) else {
        return Vec::new();
    };
    vec![crate::registry_invocation::AdvisoryObjectFactory {
        publication: normal
            .naming_factory_candidate()
            .filter(|(name, _)| is_plain_object_name(name)),
        result_class: normal.callable_result_class_candidate(),
    }]
}

/// Query original normal metadata only under its supplied source generation.
/// Independent construction-result receipts remain separate from this query.
fn source_normal_representation(
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    tokens: &crate::ir::CommandTokens,
) -> Option<crate::registry_invocation::NormalRepresentationInvocation> {
    crate::registry_invocation::normal_representation_invocation_with_metadata_context(
        registry,
        Some(context?),
        tokens,
    )
}

/// The registry class named by a class-command manufacturer value, or `None`
/// when the value is not such a call. A `TclOO` class command may be
/// written with or without the leading `::` global qualifier; the registry's
/// [`CommandRegistry::object_class`] strips it as [`CommandRegistry::get`] does.
fn constructor_class(
    value: &str,
    registry: &CommandRegistry,
    parent: Option<&crate::ir::CommandTokens>,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    source_analysis: Option<&crate::analyser::AnalysisResult>,
) -> Option<String> {
    let parent = parent?;
    let mut result = None;
    for word in parent
        .words()
        .iter()
        .filter(|word| word.legacy_text() == value)
    {
        let tokens = sole_substitution_tokens(word, parent, registry)?;
        let class = constructor_class_candidate(&tokens, registry, context, source_analysis)?;
        if result.as_ref().is_some_and(|previous| previous != &class) {
            return None;
        }
        result = Some(class);
    }
    result
}

fn sole_substitution_tokens(
    word: &crate::ir::WordExpr,
    parent: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
) -> Option<crate::ir::CommandTokens> {
    word.sole_command_substitution()?;
    let mut commands = crate::value_shapes::command_substitution_tokens(
        word,
        Some(parent),
        parent.native_lexer_config(tcl_lexer::LexerConfig::for_profile(registry.profile())),
    )?;
    (commands.len() == 1).then(|| commands.remove(0))
}

fn constructor_class_candidate(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    source_analysis: Option<&crate::analyser::AnalysisResult>,
) -> Option<String> {
    if let Some(class) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.proved_construction_result(registry))
    {
        return Some(class);
    }
    let context = context?;
    if context.permits_logical_source_names()
        && let Some(analysis) = source_analysis
        && let Some(class) = original_logical_constructor_class(analysis, context, tokens)
    {
        return Some(class);
    }
    if let Some(binding) = tokens.source_binding.as_ref() {
        let candidates = binding.class_factory_candidates(registry);
        if let Some(first) = candidates.first()
            && candidates
                .iter()
                .all(|candidate| candidate.command == first.command)
        {
            return Some(first.command.clone());
        }
    }
    let factories = source_object_factory_candidates(registry, Some(context), tokens);
    let mut classes = factories.iter().filter_map(|factory| factory.result_class);
    if let Some(class) = classes.next() {
        return classes
            .all(|other| other == class)
            .then(|| class.to_owned());
    }
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry,
        Some(context),
        tokens,
    )?;
    let head = &invocation.facts.canonical_command;
    if let Some(method) = invocation.argument_literal(0)
        && registry
            .exported_manufacturer_method(head, &method)
            .is_some()
    {
        return registry
            .object_class(head)
            .map(|class| class.class_name.to_owned());
    }
    None
}

fn original_logical_constructor_class(
    analysis: &crate::analyser::AnalysisResult,
    context: crate::registry_invocation::InvocationMetadataContext<'_>,
    tokens: &crate::ir::CommandTokens,
) -> Option<String> {
    let input = context.source_analysis_input()?;
    if !context.permits_logical_source_names() || analysis.resolved_input.as_ref() != Some(input) {
        return None;
    }
    let image = analysis.retained_command_realm()?.original_source_image()?;
    let offset = tokens.words().first()?.source().span.start();
    let original = crate::registry_invocation::original_native_compiler_words(
        image,
        tokens.words(),
        offset,
        input.lexer_config(),
    )?;
    let call = crate::registry_invocation::source_structure::source_constructor_call_at(
        image.try_text().ok()?,
        analysis,
        offset,
    )?;
    (call.original_words() == original).then_some(())?;
    call.logical_constructor_shape(analysis)?;
    let class = call.class_declaration().logical_source_class(analysis)?;
    Some(class.qualified_name.clone())
}

/// Could `word` be the bracketed registry-constructor call that makes
/// [`arg_classes`] yield a class with **no** seeded handle in play
/// (`take [listbox .l]`)?
///
/// Deliberately over-approximate and deliberately cheap: it reads the head word
/// with a string split instead of lexing, because it runs on every argument of
/// every call during the harvest.  A false "yes" only costs the propagation
/// walk that would have run anyway; a false "no" would drop a binding, so a
/// head this cannot read as a bareword (a braced, quoted, substituted, or
/// expanded head) answers "yes".
fn may_be_constructor_word(word: &str, registry: &CommandRegistry) -> bool {
    let trimmed = word.trim();
    if !trimmed.starts_with('[') {
        return false;
    }
    let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|w| w.strip_suffix(']'))
        .map(str::trim_start)
    else {
        return false;
    };
    let head = inner
        .find(char::is_whitespace)
        .map_or(inner, |end| &inner[..end]);
    if head.is_empty() || head.starts_with(['{', '"', '$', '[', '\\']) {
        return true;
    }
    registry.object_class(head).is_some()
        || registry
            .get(head)
            .is_some_and(|s| s.creates_instance_at.is_some())
}

/// Whether `name` is a plain object-command name a naming factory binds — a
/// bareword, not a `$var` / `[subst]` and not one of the `struct` deserialise
/// operator words (`= | := | as | deserialize`) that occupy the name slot when
/// the object is built from a source instead of freshly named.
fn is_plain_object_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(['$', '[', '{'])
        && !matches!(name, "=" | ":=" | "as" | "deserialize")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn provided_unit(
        source: &str,
        providers: &[crate::provider_fixtures::Provider],
    ) -> (CommandRegistry, CompilationUnit) {
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let entry = crate::provider_fixtures::entry(&registry, providers);
        let context = std::sync::Arc::new(
            crate::environment_ingress::context_for_profile(registry.profile().unwrap())
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let unit = CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: &registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
                dialect: registry.profile(),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            context,
        );
        (registry, unit)
    }

    #[test]
    fn retained_source_script_assignments_share_the_original_constructor_seed() {
        let source = "oo::class create C {method m {} {}}\nproc build {} {set object [C new]; set inert {[C new]}}\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let registry = analysis.resolved_registry().unwrap();
        let unit = CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        );
        let (id, procedure) = unit
            .ir_module
            .original_source_procedure_bodies()
            .find(|(_, procedure)| procedure.name == "build")
            .unwrap();
        let owners = OwnerIndex::build(&unit, FactMode::Full);
        let mut sink = FactSink {
            facts: ObjectHandleFacts::default(),
            owners: &owners,
            metadata_context: crate::registry_invocation::InvocationMetadataContext::for_module(
                registry,
                &unit.ir_module,
            ),
            source_analysis: Some(&analysis),
            mode: FactMode::Full,
            saw_constructor_arg: false,
            stats: LatticeStats::default(),
        };
        harvest_script_factories(
            &procedure.body,
            Some(BodyFrameId::Body(*id)),
            registry,
            &mut sink,
        );
        assert!(
            sink.facts
                .any_scope
                .get("object")
                .is_some_and(|classes| classes.contains("::C"))
        );
        assert!(!sink.facts.any_scope.contains_key("inert"));
        assert!(sink.facts.proven_reads.is_empty());
        assert!(sink.facts.physical_type_reads.is_empty());
        assert!(sink.facts.normal_procedure_results.is_empty());
    }

    #[test]
    fn logical_object_class_unions_require_current_analysis_and_module_inputs() {
        let source = "oo::class create C {method m {} {}}\nset object [C new]\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let registry = analysis.resolved_registry().unwrap();
        let unit = CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        );
        let classes = logical_source_object_handle_classes(&unit, registry, &analysis).unwrap();
        assert!(
            classes
                .get("object")
                .is_some_and(|classes| classes.contains("::C"))
        );
        let mut missing = unit.clone();
        missing.ir_module.source_metadata_input = None;
        assert!(logical_source_object_handle_classes(&missing, registry, &analysis).is_none());
        let mut changed = unit.clone();
        changed.source.push(' ');
        assert!(logical_source_object_handle_classes(&changed, registry, &analysis).is_none());
        let foreign = crate::analyser::Analyser::new().analyse("oo::class create D {}", "tcl");
        assert!(logical_source_object_handle_classes(&unit, registry, &foreign).is_none());
        let other = tcl_registry::model::ingress::static_context_for("jim");
        assert!(logical_source_object_handle_classes(&unit, other.commands(), &analysis).is_none());
    }

    #[test]
    fn logical_constructor_candidates_require_the_actual_source_input_and_vector() {
        let source = "oo::class create C {method m {} {}}\nset object [C new]\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let registry = analysis.resolved_registry().unwrap();
        let context = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, input,
        )
        .unwrap();
        let offset = source.find("C new").unwrap();
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            &source[offset..offset + "C new".len()],
            u32::try_from(offset).unwrap(),
            input.lexer_config(),
        );
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            input.lexer_config(),
            &commands[0],
        );
        assert_eq!(
            original_logical_constructor_class(&analysis, context, &tokens).as_deref(),
            Some("::C")
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(original_logical_constructor_class(&missing, context, &tokens).is_none());
        let mut changed = tokens.clone();
        let WordExpr::Literal { text, .. } = &mut changed.word_exprs[1] else {
            panic!("actual literal selector")
        };
        *text = "unrepresentedSelector".to_owned();
        assert!(original_logical_constructor_class(&analysis, context, &changed).is_none());
        let foreign = crate::analyser::Analyser::new()
            .analyse("oo::class create D {}\nset object [D new]", "tcl");
        assert!(original_logical_constructor_class(&foreign, context, &tokens).is_none());
    }

    #[test]
    fn advisory_argument_edges_keep_original_substitution_shapes() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = r#"list $object "${two words}" {$object} {[C new]} prefix$object $object(key) ${object(key)}"#;
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &commands[0],
        );
        let ctor_params = HashMap::new();
        let out = HashMap::from([
            ("object".to_owned(), HashSet::from(["::C".to_owned()])),
            ("two words".to_owned(), HashSet::from(["::C".to_owned()])),
        ]);
        let module = crate::compilation_unit::CompilationUnit::build_for("", registry, false);
        let ctx = ScanContext {
            ctor_params: &ctor_params,
            out: &out,
            by_scope: None,
            owners: OwnerIndex::shared_empty(),
            registry,
            metadata_context: None,
            source_analysis: None,
            unit: BodyFrameId::Top(module.ir_module.procedure_ids.module_id()),
            tokens: Some(&tokens),
        };
        assert_eq!(deref_arg_var(&tokens.words()[1], ctx), Some("object"));
        assert_eq!(deref_arg_var(&tokens.words()[2], ctx), Some("two words"));
        assert!(arg_classes(&tokens.words()[1], ctx).is_some());
        assert!(arg_classes(&tokens.words()[2], ctx).is_some());
        for word in &tokens.words()[3..] {
            assert!(deref_arg_var(word, ctx).is_none(), "{word:?}");
            assert!(arg_classes(word, ctx).is_none(), "{word:?}");
        }
        assert!(sole_substitution_tokens(&tokens.words()[4], &tokens, registry).is_none());
    }

    #[test]
    fn source_positioned_receiver_proofs_do_not_reuse_an_older_object_value() {
        let registry = CommandRegistry::build_default();
        let source =
            "oo::class create Pin {}\nset x [Pin new]\n$x retained\nset x 0\nset y $x\n$y stale\n";
        let cu = CompilationUnit::build_for(source, &registry, false);
        let facts = object_handle_facts(&cu, &registry);
        let retained = u32::try_from(source.find("$x retained").unwrap()).unwrap();
        let stale = u32::try_from(source.find("$y stale").unwrap()).unwrap();
        assert_eq!(
            facts.classes_in_scope(retained, "x"),
            Some(&HashSet::from(["::Pin".to_owned()]))
        );
        assert!(facts.classes_in_scope(stale, "y").is_none(), "{facts:?}");
        let proof = facts
            .physical_type_reads
            .iter()
            .find(|proof| proof.source().span.start() == retained && proof.variable() == "x")
            .expect("the retained read has a closed physical address receipt");
        assert_eq!(proof.candidate_class(), "::Pin");
        assert!(proof.place().cell.is_some());
        assert_eq!(
            facts.physical_type_read_at(proof.frame(), proof.source(), proof.evaluation()),
            Some(proof)
        );
        let mut advisory = facts.clone();
        advisory
            .any_scope
            .insert("y".to_owned(), HashSet::from(["::Pin".to_owned()]));
        advisory.by_scope.insert(
            (
                ObjectFactOwner::Frame(BodyFrameId::Top(cu.ir_module.procedure_ids.module_id())),
                "y".to_owned(),
            ),
            HashSet::from(["::Pin".to_owned()]),
        );
        assert_eq!(advisory.physical_type_reads, facts.physical_type_reads);
        assert!(advisory.classes_in_scope(stale, "y").is_none());
    }

    #[test]
    fn opaque_constructor_cannot_prove_the_original_concrete_class() {
        let registry = CommandRegistry::build_default();
        let source = "oo::class create Other {}\noo::class create C {constructor {} {oo::objdefine [self] class Other}}\nset object [C new]\n$object method\n";
        let cu = CompilationUnit::build_for(source, &registry, false);
        let facts = object_handle_facts(&cu, &registry);
        let offset = u32::try_from(source.find("$object method").unwrap()).unwrap();
        assert!(
            facts
                .classes_in_scope(offset, "object")
                .is_none_or(|classes| !classes.contains("::C")),
            "{facts:?}"
        );
        assert!(
            facts.physical_type_reads.iter().all(|proof| {
                proof.source().span.start() != offset || proof.candidate_class() != "::C"
            }),
            "constructor reclassification cannot donate a ::C type-read receipt"
        );
    }

    #[test]
    fn same_cell_reclassification_does_not_prove_the_old_object_class() {
        let registry = CommandRegistry::build_default();
        let source = "oo::class create C {method valid {} {return C}}\noo::class create B {method valid {} {return B}}\nset object [C new]\noo::objdefine $object class B\n$object valid\n";
        let cu = CompilationUnit::build_for(source, &registry, false);
        let facts = object_handle_facts(&cu, &registry);
        let offset = u32::try_from(source.rfind("$object valid").unwrap()).unwrap();
        assert!(
            facts
                .classes_in_scope(offset, "object")
                .is_none_or(|classes| !classes.contains("::C")),
            "unchanged variable contents do not preserve the object's class: {facts:?}"
        );
    }

    #[test]
    fn materialised_receiver_reads_do_not_enter_document_offset_queries() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "oo::class create C {method valid {} {return ok}}\nset object [C new]\nset body {$object valid}\neval $body\n";
        let unit = CompilationUnit::build_for(source, registry, false);
        let facts = object_handle_facts(&unit, registry);
        assert!(
            facts
                .proven_reads
                .iter()
                .all(|read| read.variable != "object"),
            "{facts:?}"
        );
        let original = source.replacen("set body", "$object valid\nset body", 1);
        let unit = CompilationUnit::build_for(&original, registry, false);
        let facts = object_handle_facts(&unit, registry);
        let offset = u32::try_from(original.find("$object valid").unwrap()).unwrap();
        assert!(
            facts.instance_in_scope(offset, "object").is_some(),
            "{facts:?}"
        );
    }

    #[test]
    fn receiver_read_does_not_preserve_class_across_argument_mutation() {
        let registry = CommandRegistry::build_default();
        let source = "oo::class create C {method valid {arg} {return C}}\noo::class create B {method valid {arg} {return B}}\nproc mutate {object} {oo::objdefine $object class B; return ARG}\nset object [C new]\n$object valid [mutate $object]\n";
        let cu = CompilationUnit::build_for(source, &registry, false);
        let facts = object_handle_facts(&cu, &registry);
        let offset = u32::try_from(source.rfind("$object valid").unwrap()).unwrap();
        assert!(
            facts.classes_in_scope(offset, "object").is_none(),
            "a receiver read before argv is not current dispatch proof: {facts:?}"
        );
    }

    #[test]
    fn physical_type_receipts_do_not_close_object_method_dispatch() {
        let registry = CommandRegistry::build_default();
        let source = "oo::class create C {method valid {} {return ORIGINAL}}\nset object [C new]\noo::objdefine $object method valid {} {return REPLACED}\n$object valid\n";
        let cu = CompilationUnit::build_for(source, &registry, false);
        let facts = object_handle_facts(&cu, &registry);
        assert!(
            facts
                .physical_type_reads
                .iter()
                .all(PhysicalObjectTypeReadEvidence::object_dispatch_unknown)
        );
    }

    #[test]
    fn replaced_class_command_does_not_seed_factory_provenance() {
        let registry = CommandRegistry::build_default();
        let source = "oo::class create Pin {}\nrename Pin {}\nproc Pin {args} {return plain}\nset receiver [Pin new]\n";
        let cu = CompilationUnit::build_for(source, &registry, false);
        let facts = object_handle_facts(&cu, &registry);
        assert!(!facts.any_scope.contains_key("receiver"), "{facts:?}");
    }

    #[test]
    fn bareword_widget_path_is_a_handle() {
        // `ttk::treeview .t` is syntactically identical to the tcllib
        // naming-factory shape (`struct::graph g`) `harvest_unit` already
        // reads generically via `creates_instance_at`/`object_class` — so a
        // Tk widget's bareword path becomes a tracked handle with zero new
        // code in this pass, once the registry declares those two fields.
        let src = "package require Tk\nttk::treeview .t\n.t instate {selected} {}\n";
        let (registry, cu) = provided_unit(src, &[crate::provider_fixtures::Provider::Tk]);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get(".t").map(|s| s.contains("ttk::treeview")),
            Some(true),
            "`.t` should be a tracked ttk::treeview handle; got {map:?}"
        );
    }

    #[test]
    fn var_captured_widget_path_is_a_handle() {
        let src = "package require Tk\nset lb [listbox .l]\n$lb curselection\n";
        let (registry, cu) = provided_unit(src, &[crate::provider_fixtures::Provider::Tk]);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("lb").map(|s| s.contains("listbox")),
            Some(true),
            "`lb` should be a tracked listbox handle; got {map:?}"
        );
    }

    #[test]
    fn unprovided_widget_and_container_catalogues_do_not_donate_handles() {
        let registry = CommandRegistry::build_default();
        for source in [
            "package require Tk\nset receiver [listbox .l]",
            "package require struct::tree\nset receiver [struct::tree]",
        ] {
            let unit = CompilationUnit::build_for(source, &registry, false);
            assert!(!object_handle_classes(&unit, &registry).contains_key("receiver"));
        }
    }

    #[test]
    fn scalar_handle_from_constructor() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let src = "oo::class create Chart {method Xaxis {args} {}}\nset chart [Chart new]\n$chart Xaxis -name x\n";
        let cu = CompilationUnit::build_for(src, registry, false);
        let map = object_handle_classes(&cu, registry);
        assert_eq!(
            map.get("chart").map(|s| s.contains("::Chart")),
            Some(true),
            "chart should retain its actual declared constructor candidate; got {map:?}"
        );
        let unprovided = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for(
            "set chart [ticklecharts::chart new]\n$chart Xaxis -name x",
            &unprovided,
            false,
        );
        assert!(!object_handle_classes(&unit, &unprovided).contains_key("chart"));
    }

    #[test]
    fn registry_naming_factory_handles() {
        // A registry object-factory names its instance either positionally
        // (`struct::graph g`) or as a substitution result (`set g [struct::graph]`)
        // — both must be tracked as `struct::graph` handles for `$g walk …`
        // method-callback resolution.
        for (src, key) in [
            ("struct::graph myG\nmyG walk root -command cb\n", "myG"),
            ("set g [struct::graph]\n$g walk root -command cb\n", "g"),
            ("struct::tree myT\nmyT walkproc root cb\n", "myT"),
        ] {
            let source =
                format!("package require struct::graph\npackage require struct::tree\n{src}");
            let (registry, cu) = provided_unit(
                &source,
                &[
                    crate::provider_fixtures::Provider::Graph,
                    crate::provider_fixtures::Provider::Tree,
                ],
            );
            let map = object_handle_classes(&cu, &registry);
            let class = if src.contains("tree") {
                "struct::tree"
            } else {
                "struct::graph"
            };
            assert_eq!(
                map.get(key).map(|s| s.contains(class)),
                Some(true),
                "`{src}` should track {key} as a {class} handle; got {map:?}"
            );
        }
    }

    #[test]
    fn normal_callable_return_types_do_not_require_a_tcloo_class() {
        use crate::provider_fixtures::Provider;
        for (prelude, providers, expected) in [
            (
                "",
                vec![Provider::Tree],
                Some(tcl_registry::TclType::Object),
            ),
            ("", vec![], None),
            (
                "rename ::struct::tree ::savedTree; proc ::struct::tree args {return ordinary}\n",
                vec![Provider::Tree],
                Some(tcl_registry::TclType::String),
            ),
        ] {
            let source = format!(
                "package require struct::tree\n{prelude}proc make {{}} {{return [struct::tree]}}\nmake\n"
            );
            let (registry, unit) = provided_unit(&source, &providers);
            let facts = object_handle_facts(&unit, &registry);
            let result = unit
                .ir_module
                .top_level
                .statements
                .last()
                .and_then(|statement| {
                    unit.ir_module
                        .top_level
                        .retained_source_tokens_for_statement(statement)
                })
                .and_then(|tokens| tokens.source_binding.as_ref())
                .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target)
                .and_then(|target| facts.normal_procedure_result(target))
                .and_then(crate::types::TypeLattice::tcl_type);
            assert_eq!(
                result,
                expected,
                "{source}; units={:?}",
                unit.procedures
                    .iter()
                    .map(|(name, unit)| (name, &unit.return_type))
                    .collect::<Vec<_>>()
            );
            assert!(
                !facts.returns_object.contains_key(
                    unit.ir_module
                        .procedures
                        .unique_display("::make")
                        .unwrap()
                        .0
                )
            );
            assert!(facts.proven_reads.is_empty());
        }
    }

    #[test]
    fn normal_return_type_receipts_do_not_follow_a_reused_proc_name() {
        let source = "proc make {} {return 1}\nmake\nproc make {} {return ordinary}\nmake\n";
        let (registry, unit) = provided_unit(source, &[]);
        let facts = object_handle_facts(&unit, &registry);
        let calls = unit
            .ir_module
            .top_level
            .statements
            .iter()
            .filter_map(|statement| {
                let tokens = unit
                    .ir_module
                    .top_level
                    .retained_source_tokens_for_statement(statement)?;
                let target = tokens.source_binding.as_ref()?.proved_execution_target()?;
                (target.kind == crate::command_binding::BindingKind::Proc).then(|| {
                    facts
                        .normal_procedure_result(target)
                        .and_then(crate::types::TypeLattice::tcl_type)
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            calls,
            [
                Some(tcl_registry::TclType::Int),
                Some(tcl_registry::TclType::String)
            ]
        );
    }

    #[test]
    fn normal_naming_factory_candidates_require_the_actual_loaded_handler() {
        use crate::provider_fixtures::Provider;
        let create = "package require struct::graph\nstruct::graph g\n";
        let (registry, loaded) = provided_unit(create, &[Provider::Graph]);
        let candidates = object_handle_classes(&loaded, &registry);
        assert!(
            candidates
                .get("g")
                .is_some_and(|classes| classes.contains("struct::graph"))
        );
        // Nominal candidates do not fabricate physical variable/object reads.
        assert!(
            object_handle_facts(&loaded, &registry)
                .proven_reads
                .is_empty()
        );

        let (registry, unprovided) = provided_unit(create, &[]);
        assert!(!object_handle_classes(&unprovided, &registry).contains_key("g"));

        let source = "package require struct::graph\nrename ::struct::graph ::savedGraph\nproc ::struct::graph args {return DECOY}\nstruct::graph g\n";
        let (registry, replaced) = provided_unit(source, &[Provider::Graph]);
        assert!(!object_handle_classes(&replaced, &registry).contains_key("g"));
    }

    #[test]
    fn retained_procedure_factory_candidates_do_not_require_an_executable_unit() {
        use crate::provider_fixtures::Provider;
        for (prelude, providers, expected) in [
            ("", vec![Provider::Graph], true),
            ("", vec![], false),
            (
                "rename ::struct::graph ::savedGraph; proc ::struct::graph args {return DECOY}\n",
                vec![Provider::Graph],
                false,
            ),
        ] {
            let source = format!(
                "package require struct::graph\n{prelude}proc build {{}} {{struct::graph g; g node insert root}}\nbuild\n"
            );
            let (registry, unit) = provided_unit(&source, &providers);
            assert_eq!(
                object_handle_classes(&unit, &registry).contains_key("g"),
                expected,
                "{source}"
            );
            assert!(
                object_handle_facts(&unit, &registry)
                    .proven_reads
                    .is_empty()
            );
        }
    }

    #[test]
    fn registry_factory_operator_form_binds_nothing() {
        // `struct::graph = $serial` puts a deserialise *operator* (`=`) in the
        // `?name?` slot — it names no object command, so neither
        // `object_handle_classes` nor the analyser's `instance_classes` may bind
        // it (a bogus `=` handle would suppress real W123/W307 and mis-resolve a
        // command literally named `=`).
        let registry = CommandRegistry::build_default();
        for op in ["=", ":=", "as", "deserialize"] {
            let src = format!("struct::graph {op} $serial\n");
            let cu = CompilationUnit::build_for(&src, &registry, false);
            assert!(
                !object_handle_classes(&cu, &registry).contains_key(op),
                "`struct::graph {op}` must not track `{op}` as an object handle"
            );
            let r = crate::analyser::Analyser::new().analyse(&src, "tcl9.0");
            assert!(
                !r.instance_classes.contains_key(op),
                "`struct::graph {op}` must not bind `{op}` in instance_classes"
            );
        }
    }

    #[test]
    fn non_constructor_assignment_is_not_a_handle() {
        let registry = CommandRegistry::build_default();
        let src = "set x [expr {1 + 2}]\nset y hello\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert!(
            map.is_empty(),
            "no object handles expected for non-constructor assignments; got {map:?}"
        );
    }

    #[test]
    fn interproc_param_from_object_arg_is_a_handle() {
        // `set p [Pin new]; connect $p` — the object flows into `connect`'s
        // parameter `dev`, so `$dev method …` in the body resolves (the
        // param-receiver case the mro_eval experiment measured as 60% of ⊤).
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin { method cfg {args} {} }\n\
                   proc connect {dev} { $dev cfg }\n\
                   set p [Pin new]\n\
                   connect $p\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("dev").map(|s| s.contains("::Pin")),
            Some(true),
            "connect's param `dev` should be a ::Pin handle; got {map:?}"
        );
    }

    #[test]
    fn interproc_param_flows_through_call_chain() {
        // `a $p` → `b $x` → `$y cfg`: the class flows two hops through the
        // fixpoint, so the innermost param `y` is a ::Pin handle.
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin { method cfg {args} {} }\n\
                   proc a {x} { b $x }\n\
                   proc b {y} { $y cfg }\n\
                   set p [Pin new]\n\
                   a $p\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("y").map(|s| s.contains("::Pin")),
            Some(true),
            "the class should flow a→b so `y` is a ::Pin handle; got {map:?}"
        );
    }

    #[test]
    fn aliasing_copies_handle_class() {
        // `set a [Pin new]; set b $a` — the plain var-ref assignment aliases
        // `a`'s class onto `b`, so `$b method …` resolves (VTA assignment edge).
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin { method cfg {args} {} }\n\
                   set a [Pin new]\n\
                   set b $a\n\
                   $b cfg\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("b").map(|s| s.contains("::Pin")),
            Some(true),
            "`b` should alias `a`'s ::Pin class; got {map:?}"
        );
    }

    #[test]
    fn child_constructor_arguments_do_not_require_the_parent_assignment_handler() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for (parent, possible) in [
            ("outer [Wrap new $p]", true),
            ("if {0} {outer [Wrap new $p]}", false),
            ("outer [error STOP; Wrap new $p]", false),
        ] {
            let source = format!(
                "oo::class create Pin {{method cfg {{}} {{}}}}; oo::class create Wrap {{constructor {{inner}} {{$inner cfg}}}}; proc outer args {{return CUSTOM}}; set p [Pin new]; {parent}"
            );
            let cu = CompilationUnit::build_for_profile(&source, &registry, false, profile);
            let map = object_handle_classes(&cu, &registry);
            assert_eq!(
                map.get("inner")
                    .is_some_and(|classes| classes.contains("::Pin")),
                possible,
                "{source}: {map:?}",
            );
        }
    }

    #[test]
    fn constructor_param_typed_from_object_arg() {
        // Case B — an object passed *into* a constructor: `Wrap new $p` binds
        // the constructor's parameter `inner` to ::Pin, so `$inner method …`
        // inside the constructor body resolves (VTA constructor-param edge).
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin { method cfg {args} {} }\n\
                   oo::class create Wrap {\n\
                     variable held\n\
                     constructor {inner} { $inner cfg; set held $inner }\n\
                   }\n\
                   set p [Pin new]\n\
                   set w [Wrap new $p]\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("inner").map(|s| s.contains("::Pin")),
            Some(true),
            "constructor param `inner` should be a ::Pin handle; got {map:?}"
        );
        // …and the aliasing edge carries it onto the instance var it is stored
        // in, so a *different* method dispatching `$held m` resolves too.
        assert_eq!(
            map.get("held").map(|s| s.contains("::Pin")),
            Some(true),
            "instance var `held` should alias the ::Pin ctor param; got {map:?}"
        );
    }

    #[test]
    fn instance_var_from_constructor_param_bridges_methods() {
        // The full Case-B shape: an object flows in through the constructor,
        // is stored in an instance variable, and is dispatched on from an
        // unrelated method.  ctor-param + aliasing edges together resolve it.
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Motor { method spin {args} {} }\n\
                   oo::class create Car {\n\
                     variable engine\n\
                     constructor {e} { set engine $e }\n\
                     method go {} { $engine spin }\n\
                   }\n\
                   set m [Motor new]\n\
                   set c [Car new $m]\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("engine").map(|s| s.contains("::Motor")),
            Some(true),
            "instance var `engine` should be a ::Motor handle via ctor param; got {map:?}"
        );
    }

    #[test]
    fn snit_named_constructor_has_nominal_name_without_a_physical_class() {
        let source = "package require snit\nsnit::type foo { method smeth {} {} }\nset o [foo create x]\n$o smeth\n";
        let (registry, cu) = provided_unit(source, &[crate::provider_fixtures::Provider::Snit]);
        let facts = object_handle_facts(&cu, &registry);
        let read = u32::try_from(source.find("$o smeth").unwrap()).unwrap();
        assert!(facts.classes_in_scope(read, "o").is_none());
        assert!(facts.proven_reads.iter().all(|proof| proof.variable != "o"));
        let create = u32::try_from(source.find("foo create").unwrap()).unwrap();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let entry =
            crate::provider_fixtures::entry(&registry, &[crate::provider_fixtures::Provider::Snit]);
        let selected = crate::command_binding::SourceCommandBindings::analyse_with_options(
            source,
            config,
            &registry,
            entry.options(),
        );
        assert!(
            selected
                .invocation_at_source("::foo", create)
                .nominal_definition_name_result(&registry)
                .is_some()
        );
        let unprovided =
            crate::command_binding::SourceCommandBindings::analyse(source, config, &registry);
        assert!(
            unprovided
                .invocation_at_source("::foo", create)
                .nominal_definition_name_result(&registry)
                .is_none()
        );
    }

    #[test]
    fn collection_of_objects_is_tracked() {
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin {}\n\
                   dict set pins a [Pin new]\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_collection_classes(&cu);
        assert_eq!(
            map.get("pins").map(|s| s.contains("::Pin")),
            Some(true),
            "pins should be a collection of ::Pin; got {map:?}"
        );
    }

    #[test]
    fn collection_class_bridges_across_methods() {
        // The interprocedural case: one method fills the `pins`
        // collection, a *different* method dispatches on an element.  The
        // cross-scope union makes `pins` a collection-of-Pin at both sites.
        let registry = CommandRegistry::build_default();
        let src = "oo::class create Pin { method cfg {args} {} }\n\
                   oo::class create Dev {\n\
                     variable pins\n\
                     method add {k} { ::dict set pins $k [::Pin new] }\n\
                     method use {k} { [::dict get $pins $k] cfg -node 1 }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_collection_classes(&cu);
        assert_eq!(
            map.get("pins").map(|s| s.contains("::Pin")),
            Some(true),
            "pins should be a collection of ::Pin, harvested from method add; got {map:?}"
        );
    }

    #[test]
    fn spicegentcl_configurable_device_shape_resolves() {
        // The exact SpiceGenTcl shape: namespaced `oo::configurable` classes,
        // the collection built and dispatched in the *same* big method's switch
        // arms with fully-qualified constructors.  Locks in that an
        // `oo::configurable` class body is lowered, so its `[::ns::Pin new]`
        // writes type the `Pins` dict.
        let registry = CommandRegistry::build_default();
        let src = "namespace eval ::SpiceGenTcl {\n\
                     oo::configurable create Pin { property node }\n\
                     oo::configurable create Device {\n\
                       variable Pins\n\
                       method actOnPin {action pin node} {\n\
                         ::switch -- $action {\n\
                           add { ::dict append Pins $pin [::SpiceGenTcl::Pin new $pin $node] }\n\
                           node { [::dict get $Pins $pin] configure -node $node }\n\
                         }\n\
                       }\n\
                     }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let map = object_collection_classes(&cu);
        assert_eq!(
            map.get("Pins").map(|s| s.contains("::SpiceGenTcl::Pin")),
            Some(true),
            "Pins should be a collection of ::SpiceGenTcl::Pin; got {map:?}"
        );
    }

    /// Build a unit and its full fact set in one step (the tests below want
    /// both).
    fn facts_for(src: &str) -> (CommandRegistry, ObjectHandleFacts) {
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &registry, false);
        let facts = object_handle_facts(&cu, &registry);
        (registry, facts)
    }

    /// The classes `by_scope` binds for `(owner, var)`, sorted for a stable
    /// assertion message.
    fn scoped(facts: &ObjectHandleFacts, owner: &str, var: &str) -> Vec<String> {
        let owners = facts
            .owner_spans
            .iter()
            .filter_map(|span| {
                if span.unit == owner {
                    Some(ObjectFactOwner::Frame(span.frame))
                } else if span.class.as_deref() == Some(owner) {
                    span.class_owner.clone()
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();
        let mut values = owners
            .into_iter()
            .flat_map(|owner| {
                facts
                    .by_scope
                    .get(&(owner, var.to_owned()))
                    .into_iter()
                    .flatten()
                    .cloned()
            })
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();
        values
    }

    #[test]
    fn colliding_method_reports_have_distinct_scoped_object_candidates() {
        let source = "namespace eval A {}\noo::class create A::B {method c {} {set handle [Pin new]}}\noo::class create A {method B::c {} {set handle 0}}\n";
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for(source, &registry, false);
        let facts = object_handle_facts(&unit, &registry);
        let scopes = facts
            .owner_spans
            .iter()
            .filter(|scope| scope.unit == "::A::B::c")
            .collect::<Vec<_>>();
        assert_eq!(scopes.len(), 2);
        assert_ne!(scopes[0].frame, scopes[1].frame);
        assert_ne!(scopes[0].class_owner, scopes[1].class_owner);
    }

    #[test]
    fn any_scope_is_verbatim_object_handle_classes() {
        // The widened carrier must not perturb the map every existing consumer
        // reads: `any_scope` is the old return value, byte-for-byte.
        for src in [
            "set chart [ticklecharts::chart new]\n$chart Xaxis -name x\n",
            "oo::class create Pin { method cfg {args} {} }\n\
             proc connect {dev} { $dev cfg }\n\
             set p [Pin new]\nconnect $p\n",
            "set x [expr {1 + 2}]\n",
            "",
        ] {
            let registry = CommandRegistry::build_default();
            let cu = CompilationUnit::build_for(src, &registry, false);
            assert_eq!(
                object_handle_facts(&cu, &registry).any_scope,
                object_handle_classes(&cu, &registry),
                "any_scope must equal object_handle_classes for `{src}`"
            );
        }
    }

    #[test]
    fn empty_seed_fast_path_is_behaviour_preserving() {
        // The fast path skips the propagation walk when no edge can fire.  Pin
        // it against the unconditional walk on every shape that binds
        // from an *empty seed set* — a bare `out.is_empty()` gate (the obvious
        // one) silently drops all four of the middle cases here.
        for src in [
            // Nothing at all: the fast path fires.
            "proc noop {} { return 1 }\nset a 1\nputs $a\n",
            "proc a {x} { b $x }\nproc b {y} { puts $y }\na 1\n",
            // Proc-return edge from a factory whose object never lands in a
            // `set` target the harvest can see.
            "oo::class create Pin { method cfg {args} {} }\n\
             proc make {} { return [Pin new] }\n\
             set c [make]\n$c cfg\n",
            // `arg_classes`' direct-constructor branch: a registry factory
            // written straight into a proc-parameter, a constructor
            // parameter, and a bare naming factory.
            "proc take {dev} { $dev curselection }\ntake [listbox .l]\n",
            "oo::class create Wrap { constructor {inner} { $inner curselection } }\n\
             Wrap new [listbox .l]\n",
            "proc take {dev} { $dev walk root }\ntake [struct::graph]\n",
            // Nested inside an assigned value.
            "oo::class create Wrap { constructor {inner} {} }\n\
             set w [Wrap new [listbox .l]]\n",
        ] {
            let registry = CommandRegistry::build_default();
            let cu = CompilationUnit::build_for(src, &registry, false);
            assert_eq!(
                object_handle_classes(&cu, &registry),
                object_handle_classes_full_walk(&cu, &registry),
                "the empty-seed fast path changed the map for `{src}`"
            );
        }
    }

    #[test]
    fn owner_attribution_alias_edge_binds_in_the_assigning_unit() {
        // `set b $a` inside `mk` binds `b` in `::mk`, not globally — the
        // same-name collision across procs `any_scope` cannot avoid.
        let (_r, facts) = facts_for(
            "oo::class create Pin { method cfg {args} {} }\n\
             proc mk {} { set a [Pin new]\n set b $a\n $b cfg }\n\
             proc other {} { set b 7\n puts $b }\n",
        );
        assert_eq!(scoped(&facts, "::mk", "b"), vec!["::Pin".to_owned()]);
        assert!(
            scoped(&facts, "::other", "b").is_empty(),
            "`b` in ::other is an unrelated integer; by_scope must not widen \
             onto it (any_scope does: {:?})",
            facts.any_scope.get("b")
        );
    }

    #[test]
    fn generic_factory_return_candidates_flow_without_a_return_type_receipt() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (prefix, expected) in [
            ("oo::class create Pin {}", true),
            (
                "oo::class create Pin {}; rename Pin {}; proc Pin args {return plain}",
                false,
            ),
        ] {
            let source = format!(
                "{prefix}; proc make {{}} {{return [Pin new]}}; proc take {{dev}} {{return ok}}; set receiver [make]; take $receiver"
            );
            let mut unit = CompilationUnit::build_for_profile(
                &source,
                registry,
                false,
                registry.profile().unwrap(),
            );
            unit.procedures.remove(
                unit.ir_module
                    .procedures
                    .unique_display("::make")
                    .unwrap()
                    .0,
            );
            unit.procedures.remove(
                unit.ir_module
                    .procedures
                    .unique_display("::take")
                    .unwrap()
                    .0,
            );
            let facts = object_handle_facts(&unit, registry);
            assert_eq!(
                facts
                    .any_scope
                    .get("dev")
                    .is_some_and(|classes| classes.contains("::Pin")),
                expected,
                "the retained factory implementation selects the advisory flow: {facts:?}"
            );
            assert!(
                !facts.returns_object.contains_key(
                    unit.ir_module
                        .procedures
                        .unique_display("::make")
                        .unwrap()
                        .0
                )
            );
        }
    }

    #[test]
    fn owner_attribution_return_edge_binds_in_the_assigning_unit() {
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for(
            "oo::class create Pin { method cfg {args} {} }\n\
             proc make {} { set p [Pin new]\n return $p }\n\
             proc use {} { set q [make]\n $q cfg }\n",
            &registry,
            false,
        );
        let facts = object_handle_facts(&unit, &registry);
        assert_eq!(scoped(&facts, "::use", "q"), vec!["::Pin".to_owned()]);
        assert_eq!(
            facts
                .returns_object
                .get(
                    unit.ir_module
                        .procedures
                        .unique_display("::make")
                        .unwrap()
                        .0
                )
                .map(String::as_str),
            Some("::Pin"),
            "the factory-proc return fact must be exported; got {:?}",
            facts.returns_object
        );
    }

    #[test]
    fn owner_attribution_param_edge_binds_in_the_callee() {
        // `connect $p` binds `dev` in `::connect` — the callee, where the name
        // lives — not in the caller that supplied the argument.
        let (_r, facts) = facts_for(
            "oo::class create Pin { method cfg {args} {} }\n\
             proc connect {dev} { $dev cfg }\n\
             set p [Pin new]\nconnect $p\n",
        );
        assert_eq!(scoped(&facts, "::connect", "dev"), vec!["::Pin".to_owned()]);
        assert!(
            scoped(&facts, "::top", "dev").is_empty(),
            "the param edge must not bind `dev` at the call site's scope"
        );
        assert_eq!(scoped(&facts, "::top", "p"), vec!["::Pin".to_owned()]);
    }

    #[test]
    fn owner_attribution_ctor_param_edge_binds_in_the_constructor() {
        let (_r, facts) = facts_for(
            "oo::class create Pin { method cfg {args} {} }\n\
             oo::class create Wrap {\n\
               variable held\n\
               constructor {inner} { $inner cfg; set held $inner }\n\
             }\n\
             set p [Pin new]\nset w [Wrap new $p]\n",
        );
        assert_eq!(
            scoped(&facts, "::Wrap::<constructor>", "inner"),
            vec!["::Pin".to_owned()],
            "the ctor param is owned by the constructor unit; by_scope={:?}",
            facts.by_scope
        );
    }

    #[test]
    fn owner_attribution_instance_var_is_owned_by_the_class() {
        // The cross-method bridge: `engine` is written in `Car`'s constructor
        // and read in `Car`'s `go`.  Keying it by the *class* (not by either method)
        // is what makes the cross-method dispatch resolvable at all.
        let (_r, facts) = facts_for(
            "oo::class create Motor { method spin {args} {} }\n\
             oo::class create Car {\n\
               variable engine\n\
               constructor {e} { set engine $e }\n\
               method go {} { $engine spin }\n\
             }\n\
             set m [Motor new]\nset c [Car new $m]\n",
        );
        assert_eq!(
            scoped(&facts, "::Car", "engine"),
            vec!["::Motor".to_owned()],
            "instance var `engine` is owned by ::Car; by_scope={:?}",
            facts.by_scope
        );
        assert!(
            scoped(&facts, "::Car::<constructor>", "engine").is_empty(),
            "an instance variable must not also be keyed by the writing method"
        );
        // …and `classes_in_scope` finds it from a byte inside the *other*
        // method, via the class fallback.
        let offset = facts
            .owner_spans
            .iter()
            .find(|o| o.unit == "::Car::go")
            .expect("::Car::go has an owner span")
            .span
            .start();
        assert_eq!(
            facts
                .classes_in_scope(offset, "engine")
                .map(|s| s.contains("::Motor")),
            Some(true),
            "classes_in_scope inside ::Car::go must reach the class-owned name"
        );
    }

    #[test]
    fn collection_and_global_cell_facts_are_exported() {
        let (_r, facts) = facts_for(
            "oo::class create Pin { method cfg {args} {} }\n\
             dict set pins a [Pin new]\n\
             set ::board [listbox .l]\n",
        );
        assert_eq!(
            facts.collections.get("pins").map(|s| s.contains("::Pin")),
            Some(true),
            "collections must carry object_collection_classes verbatim; got {:?}",
            facts.collections
        );
        assert_eq!(
            facts
                .global_object_cells
                .get("::board")
                .map(|s| s.contains("listbox")),
            Some(true),
            "a `::`-qualified handle must be exported as a global cell; got {:?}",
            facts.global_object_cells
        );
        assert!(
            !facts.global_object_cells.contains_key("pins"),
            "an unqualified local is not a global cell"
        );
    }

    #[test]
    fn registry_factory_operator_form_binds_nothing_by_scope() {
        // The `by_scope` twin of `registry_factory_operator_form_binds_nothing`:
        // the deserialise operator words name no object command, so the
        // abstention must survive in the scope-keyed map too — a bogus `=`
        // binding there would be read by the rename/refusal consumers.
        let registry = CommandRegistry::build_default();
        for op in ["=", ":=", "as", "deserialize"] {
            let src = format!("struct::graph {op} $serial\n");
            let cu = CompilationUnit::build_for(&src, &registry, false);
            let facts = object_handle_facts(&cu, &registry);
            assert!(
                !facts.any_scope.contains_key(op),
                "`struct::graph {op}` must not track `{op}` in any_scope"
            );
            assert!(
                facts.by_scope.keys().all(|(_owner, name)| name != op),
                "`struct::graph {op}` must not track `{op}` in by_scope; got {:?}",
                facts.by_scope
            );
        }
    }

    #[test]
    fn by_scope_does_not_import_another_units_binding_through_an_alias() {
        // FP guard.  `x` is a `::Pin` in `::a` and a plain integer in `::b`.
        // `any_scope` unions the two — that is its documented, deliberate
        // imprecision — but the alias `set y $x` inside `::b` must resolve `x`
        // **in `::b`**, where there is no object.  A `(::b, y) → ::Pin` entry
        // would be a false *singleton* in the map the rename edits and the
        // "provably a different class" refusal gate read as authoritative: it
        // would rewrite an unrelated integer.
        let (_r, facts) = facts_for(
            "oo::class create Pin { method cfg {args} {} }\n\
             proc a {} { set x [Pin new] }\n\
             proc b {} { set x 0\n set y $x }\n",
        );
        assert_eq!(
            scoped(&facts, "::a", "x"),
            vec!["::Pin".to_owned()],
            "the seed itself must still be scope-keyed to ::a"
        );
        assert!(
            scoped(&facts, "::b", "y").is_empty(),
            "`y` in ::b aliases ::b's own integer `x`, not ::a's Pin; \
             by_scope must not import the other unit's binding (any_scope \
             legitimately unions it: {:?})",
            facts.any_scope.get("y")
        );
        assert!(
            scoped(&facts, "::b", "x").is_empty(),
            "::b's `x` is an integer; by_scope must not bind it"
        );
    }

    #[test]
    fn by_scope_alias_within_one_unit_still_propagates() {
        // TP twin of the guard above: the same alias edge, both ends in the
        // same unit, must still bind.
        let (_r, facts) = facts_for(
            "oo::class create Pin { method cfg {args} {} }\n\
             proc c {} { set p [Pin new]\n set q $p\n $q cfg }\n",
        );
        assert_eq!(
            scoped(&facts, "::c", "q"),
            vec!["::Pin".to_owned()],
            "an alias whose source is bound in the *same* unit must propagate; \
             by_scope={:?}",
            facts.by_scope
        );
    }

    #[test]
    fn by_scope_instance_var_alias_reads_the_class_owner() {
        // The cross-method bridge under scoped propagation: `engine` is
        // written in the constructor and read in another method.  The source lookup for
        // `set m $engine` inside `::Car::go` must fall back to the *class*
        // owner, or the bridge the design is built on would break.
        let (_r, facts) = facts_for(
            "oo::class create Motor { method spin {args} {} }\n\
             oo::class create Car {\n\
               variable engine\n\
               constructor {e} { set engine $e }\n\
               method go {} { set m $engine\n $m spin }\n\
             }\n\
             set mo [Motor new]\n\
             set c [Car new $mo]\n",
        );
        assert_eq!(
            scoped(&facts, "::Car", "engine"),
            vec!["::Motor".to_owned()],
            "the class-owned instance variable must still bind"
        );
        assert_eq!(
            scoped(&facts, "::Car::go", "m"),
            vec!["::Motor".to_owned()],
            "reading a class-owned instance variable from a method must resolve \
             through the class owner; by_scope={:?}",
            facts.by_scope
        );
    }

    #[test]
    fn by_scope_cross_unit_call_edges_still_bind() {
        // The two legitimately cross-scope edges must survive the scoped
        // source resolution: a proc-return edge (the source is the callee's
        // return type, which is unit-independent) and a proc-parameter edge
        // (the argument is resolved in the *caller*, bound in the callee).
        let (_r, facts) = facts_for(
            "oo::class create Pin { method cfg {args} {} }\n\
             proc make {} { set p [Pin new]\n return $p }\n\
             proc connect {dev} { $dev cfg }\n\
             proc drive {} { set q [make]\n connect $q }\n",
        );
        assert_eq!(
            scoped(&facts, "::drive", "q"),
            vec!["::Pin".to_owned()],
            "the proc-return edge binds in the assigning unit; by_scope={:?}",
            facts.by_scope
        );
        assert_eq!(
            scoped(&facts, "::connect", "dev"),
            vec!["::Pin".to_owned()],
            "the proc-parameter edge reads the argument in ::drive and binds \
             the parameter in ::connect; by_scope={:?}",
            facts.by_scope
        );
    }

    #[test]
    fn method_return_capture_types_the_handle() {
        // `set b [$a make]` — the receiver is already typed and
        // `::A::make`'s own return type names an object class, so the captured
        // handle is a ::B in both facts.
        let (_r, facts) = facts_for(
            "oo::class create A { method make {} { ::return [::B new] } }\n\
             oo::class create B { method greet {} { ::return \"hi\" } }\n\
             set a [A new]\n\
             set b [$a make]\n\
             $b greet\n",
        );
        assert_eq!(
            facts.any_scope.get("b").map(|s| s.contains("::B")),
            Some(true),
            "`b` should be a ::B handle via the method-return edge; got {:?}",
            facts.any_scope
        );
        assert_eq!(
            scoped(&facts, "::top", "b"),
            vec!["::B".to_owned()],
            "the method-return edge binds in the assigning unit; by_scope={:?}",
            facts.by_scope
        );
    }

    #[test]
    fn method_return_edge_abstains_on_computed_member_and_literal_return() {
        // TN twins of the edge: a computed member word proves nothing, and a
        // method returning a plain literal types no handle.
        let (_r, facts) = facts_for(
            "oo::class create A {\n\
               method make {} { return [B new] }\n\
               method name {} { return plain }\n\
             }\n\
             oo::class create B {}\n\
             set a [A new]\n\
             set m make\n\
             set b [$a $m]\n\
             set c [$a name]\n",
        );
        assert!(
            !facts.any_scope.contains_key("b"),
            "a computed member word must not bind; got {:?}",
            facts.any_scope
        );
        assert!(
            !facts.any_scope.contains_key("c"),
            "a literal-returning method must not bind; got {:?}",
            facts.any_scope
        );
    }

    #[test]
    fn method_return_edge_does_not_import_another_units_receiver() {
        // The by_scope guard for the new edge: `a` is a ::A only inside
        // `::mk`; a same-named `a` in `::other` is untyped there, so the
        // capture in `::other` must not bind in the scope-keyed map (the
        // union may — its documented imprecision).
        let (_r, facts) = facts_for(
            "oo::class create A { method make {} { ::return [::B new] } }\n\
             oo::class create B {}\n\
             proc mk {} { set a [A new]\n set b [$a make] }\n\
             proc other {} { set a 1\n set b [$a make] }\n",
        );
        assert_eq!(
            scoped(&facts, "::mk", "b"),
            vec!["::B".to_owned()],
            "the in-unit capture must bind; by_scope={:?}",
            facts.by_scope
        );
        assert!(
            scoped(&facts, "::other", "b").is_empty(),
            "::other's `a` is an integer; the scoped fact must not import \
             ::mk's receiver; by_scope={:?}",
            facts.by_scope
        );
    }

    #[test]
    fn owner_spans_are_sorted_and_resolve_the_innermost_scope() {
        let (_r, facts) = facts_for(
            "set g 1\n\
             proc alpha {} { set a 1 }\n\
             oo::class create K { method m {} { set b 2 } }\n\
             proc beta {} { set c 3 }\n",
        );
        assert!(
            facts
                .owner_spans
                .windows(2)
                .all(|w| w[0].span.start() <= w[1].span.start()),
            "owner_spans must be start-sorted for binary search; got {:?}",
            facts.owner_spans
        );
        // Top-level bytes resolve to `::top`, so a top-level handle has an
        // owner to key against rather than falling off the index.
        assert_eq!(
            facts.owner_at(0).map(|o| o.unit.as_str()),
            Some("::top"),
            "byte 0 is top-level code; got {:?}",
            facts.owner_at(0)
        );
        for unit in ["::alpha", "::beta", "::K::m"] {
            let span = facts
                .owner_spans
                .iter()
                .find(|o| o.unit == unit)
                .unwrap_or_else(|| panic!("{unit} must have an owner span"))
                .span;
            assert_eq!(
                facts.owner_at(span.start()).map(|o| o.unit.as_str()),
                Some(unit),
                "owner_at must resolve a byte inside {unit} to it, not to the \
                 enclosing top level; owner_spans={:?}",
                facts.owner_spans
            );
        }
        assert_eq!(
            facts
                .owner_spans
                .iter()
                .find(|o| o.unit == "::K::m")
                .and_then(|o| o.class.as_deref()),
            Some("::K"),
            "a method's owner span must name its class for the fallback key"
        );
    }
    #[test]
    fn factory_return_metadata_keeps_supplied_availability_and_refuses_missing_or_foreign_input() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Conditional factory class candidates; no successful return or allocated receiver.
        let baseline = crate::environment_ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        );
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut descriptor = registry.get("return").unwrap().clone();
        descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(descriptor);
        let context =
            std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(registry)));
        let registry = context.commands();
        let mut unit = CompilationUnit::build_with_context_registry(
            "oo::class create Pin {}; proc make {} {return [Pin new]}",
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
                dialect: registry.profile(),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            std::sync::Arc::clone(&context),
        );
        unit.procedures.remove(
            unit.ir_module
                .procedures
                .unique_display("::make")
                .unwrap()
                .0,
        );
        assert!(returning_procs(&unit).is_empty());
        let current = returning_proc_candidates(&unit, registry, None);
        assert_eq!(
            current
                .get(
                    unit.ir_module
                        .procedures
                        .unique_display("::make")
                        .unwrap()
                        .0
                )
                .map(String::as_str),
            Some("::Pin")
        );
        let original = unit
            .ir_module
            .procedures
            .get(
                unit.ir_module
                    .procedures
                    .unique_display("::make")
                    .unwrap()
                    .0,
            )
            .unwrap()
            .body
            .clone();
        let input = unit.ir_module.source_metadata_input.clone().unwrap();
        let older = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(std::sync::Arc::clone(registry)),
        );
        assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
        let foreign = crate::environment_ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
        );
        for withheld in [
            Some(crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                older,
                input.lexer_config(),
            )),
            Some(crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                foreign,
                input.lexer_config(),
            )),
            None,
        ] {
            unit.ir_module.source_metadata_input = withheld;
            assert!(returning_proc_candidates(&unit, registry, None).is_empty());
            assert_eq!(
                unit.ir_module
                    .procedures
                    .get(
                        unit.ir_module
                            .procedures
                            .unique_display("::make")
                            .unwrap()
                            .0
                    )
                    .unwrap()
                    .body,
                original
            );
        }
    }

    #[test]
    fn normal_result_query_uses_the_shared_roster_and_withdraws_missing_source_ownership() {
        // naming.source.original-collection-body-owner
        // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
        // Conditional result types under existing allocation guards, not runtime execution.
        let source = "proc make {} {return 1}\nmake\n";
        let (registry, unit) = provided_unit(source, &[]);
        assert!(
            unit.retained_source_class_names(&registry)
                .unwrap()
                .is_empty()
        );
        let target = unit
            .ir_module
            .top_level
            .statements
            .last()
            .and_then(|statement| {
                unit.ir_module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
            })
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target)
            .expect("original selected procedure allocation");
        let mut baseline = ObjectHandleFacts::default();
        retain_normal_procedure_result_types(&unit, &registry, &mut baseline);
        assert_eq!(
            baseline
                .normal_procedure_result(target)
                .and_then(crate::types::TypeLattice::tcl_type),
            Some(tcl_registry::TclType::Int)
        );
        for axis in 0..5 {
            let mut changed = unit.clone();
            match axis {
                0 => changed.source_class_facts = None,
                1 => changed.ir_module.retained_source_bindings = None,
                2 => changed.ir_module.source_metadata_input = None,
                3 => changed.ir_module.lexer_config.strict_quoting ^= true,
                _ => changed.source = "proc other {} {return 1}\nother\n".to_owned(),
            }
            let mut stale = baseline.clone();
            retain_normal_procedure_result_types(&changed, &registry, &mut stale);
            assert!(stale.normal_procedure_results.is_empty(), "axis {axis}");
            assert!(stale.normal_procedure_result(target).is_none());
        }
        let mut changed_store = registry;
        changed_store.insert(tcl_registry::CommandSpec {
            name: "unrelated_selected_command",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let mut stale = baseline;
        retain_normal_procedure_result_types(&unit, &changed_store, &mut stale);
        assert!(stale.normal_procedure_results.is_empty());
        assert!(stale.normal_procedure_result(target).is_none());
    }
}
