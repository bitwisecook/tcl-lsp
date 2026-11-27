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
use crate::ir::{MethodKind, ProcedureId};
use tcl_core_types::BodyFrameId;
#[cfg(test)]
use tcl_source::words::WordExpr;

mod constructor_harvest;
mod object_flow;
mod procedure_results;

#[cfg(test)]
use constructor_harvest::original_logical_constructor_class;
#[cfg(test)]
use constructor_harvest::sole_substitution_tokens;
use constructor_harvest::{harvest_script_factories, harvest_unit, may_be_constructor_word};
#[cfg(test)]
use object_flow::arg_classes;
#[cfg(test)]
use object_flow::deref_arg_var;
use object_flow::propagate_object_flow;
use procedure_results::{
    retain_normal_procedure_result_types, returning_proc_candidates, returning_procs,
};

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
        allocation: tcl_names::command::CommandAllocation,
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
    source: tcl_source::words::SourceSite,
    evaluation: crate::command_binding::SourceVariableEvaluationOwner,
    unit: String,
    frame: BodyFrameId,
    variable: String,
    place: tcl_names::place::Place,
    value: crate::ssa::ValueKey,
    class: String,
}

impl PhysicalObjectTypeReadEvidence {
    /// Exact lexical read and its retained provenance.
    #[must_use]
    pub fn source(&self) -> &tcl_source::words::SourceSite {
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
    pub fn place(&self) -> &tcl_names::place::Place {
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
        (target.kind == tcl_names::command::BindingKind::Proc).then_some(())?;
        self.normal_procedure_results
            .get(target.implementation_allocation.as_ref()?)
    }

    /// Select a physical type-read receipt at an exact source/evaluation boundary.
    /// This retains object-dispatch uncertainty and cannot prove a runtime class.
    #[must_use]
    pub fn physical_type_read_at(
        &self,
        frame: BodyFrameId,
        source: &tcl_source::words::SourceSite,
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
        || place.cell.as_ref()?.generation == tcl_names::place::CellGeneration::Unknown
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

#[cfg(test)]
mod tests;
