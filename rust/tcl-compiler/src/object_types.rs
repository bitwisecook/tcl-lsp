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
    if std::ptr::eq(fu, std::ptr::from_ref(&cu.top_level)) {
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

mod authenticated_report;
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
        if context.is_some_and(
            crate::registry_invocation::InvocationMetadataContext::permits_logical_source_names,
        ) || !cu.procedures.contains_key(name)
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
            .filter(|(name, _)| tcl_syntax::naming::is_absolute(name))
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
        for (name, unit) in cu.procedures.iter().chain(cu.body_lowering_functions()) {
            let procedure = cu
                .ir_module
                .procedures
                .get(name)
                .or_else(|| cu.ir_module.body_lowering(name));
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
fn returning_proc_candidates(
    cu: &CompilationUnit,
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
        let candidates = tokens.map_or_default(|tokens| {
            crate::registry_invocation::advisory_return_values_with_metadata_context(
                registry, metadata, tokens,
            )
        });
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
fn propagate_object_flow(
    cu: &CompilationUnit,
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
            metadata_context.is_some_and(
                crate::registry_invocation::InvocationMetadataContext::permits_logical_source_names,
            ) || !cu.procedures.contains_key(*name)
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
                            member_label: method.clone(),
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
    // A factory that publishes an instance command returns that command's
    // name (`set t [ttk::treeview .tree]` leaves `.tree` in `t`), so the
    // word holding the result is a handle of the published class.
    let mut published = factories
        .iter()
        .filter_map(|factory| factory.publication.as_ref().map(|(_, class)| *class));
    if let Some(class) = published.next() {
        return published
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
mod tests;
