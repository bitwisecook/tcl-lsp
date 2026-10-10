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

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::Statement;
use crate::value_shapes::parse_command_substitution_with_config;

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
/// | [`Self::returns_object`] | proc qname | none (one return type per proc) | factory-call typing |
/// | [`Self::global_object_cells`] | `::`-qualified name | all versions and documents may disagree | labelled cross-document assistance |
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObjectHandleFacts {
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
    pub by_scope: HashMap<(String, String), HashSet<String>>,
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
    /// Procedure qualified name → the class of the object it returns.  The
    /// factory-proc fact the VTA fixpoint computes internally, exported so a
    /// cross-document index can seed on it.
    pub returns_object: HashMap<String, String>,
    /// Normal result types at their actual implementation source, independently
    /// of class names, object allocations and method dispatch authority.
    normal_procedure_results: crate::type_infer::NormalProcedureResultTypes,
    /// The `::`-qualified subset of [`Self::any_scope`] — object handles that
    /// have a global spelling. Cross-document assistance may join these
    /// candidates; the spelling does not prove the cell's current contents.
    pub global_object_cells: HashMap<String, HashSet<String>>,
}

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
        unit: &str,
        source: &crate::ir::SourceSite,
        evaluation: &crate::command_binding::SourceVariableEvaluationOwner,
    ) -> Option<&PhysicalObjectTypeReadEvidence> {
        let mut proofs = self.physical_type_reads.iter().filter(|proof| {
            proof.unit == unit && &proof.source == source && &proof.evaluation == evaluation
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
    build_facts_gated(cu, registry, FactMode::AnyScopeOnly, false)
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
    build_facts_gated(cu, registry, FactMode::Full, true)
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

/// How much of [`ObjectHandleFacts`] a build populates.
///
/// The union-only mode exists so [`object_handle_classes`]'s existing
/// consumers (the optimiser, the compilation-unit interprocedural seed,
/// `type_infer`, semantic tokens) keep paying exactly what they paid before
/// the scope-keyed carrier was added.
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
    build_facts_gated(cu, registry, mode, true)
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
) -> (ObjectHandleFacts, LatticeStats) {
    let owners = OwnerIndex::build(cu, mode);
    let context =
        crate::registry_invocation::retained_module_metadata_context(registry, &cu.ir_module);
    let mut sink = FactSink {
        facts: ObjectHandleFacts::default(),
        owners: &owners,
        metadata_context: context.as_deref().map(Into::into),
        mode,
        saw_constructor_arg: false,
        stats: LatticeStats::default(),
    };
    let units = std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values());
    for fu in units {
        harvest_unit(fu, registry, &mut sink);
    }
    // A generic compiler refusal can leave a genuine normal handler in the
    // retained IR without a FunctionUnit. Its nominal factory candidate is
    // still useful for possible callback edges, independently of SSA proof.
    for (name, procedure) in &cu.ir_module.procedures {
        if !cu.procedures.contains_key(name) {
            harvest_script_factories(&procedure.body, name, registry, &mut sink);
        }
    }
    for (name, method) in &cu.ir_module.methods {
        if !cu.methods.contains_key(name) {
            harvest_script_factories(&method.body, name, registry, &mut sink);
        }
    }
    sink.stats.seeds = sink.facts.any_scope.len();
    let returns = returning_proc_candidates(cu, registry);
    // Fast path.  The early-out inside `propagate_object_flow`
    // checks the *callee-side* maps (returns / proc params / ctor params),
    // which are non-empty for any file that merely defines a proc — so every
    // ordinary non-OO file paid a full statement walk to discover that it had
    // nothing to propagate.  Skip the walk when no edge can fire, which is
    // exactly when all three of these hold:
    //
    //  - the harvest seeded no handle, so every `out`-driven edge (aliasing,
    //    and the `$var` half of both parameter edges) is dead;
    //  - no procedure returns an object, so the proc-return edge is dead; and
    //  - no argument is a bracketed registry constructor, so the *other* half
    //    of the parameter edges — `arg_classes`' `[Factory new]` branch, which
    //    reads no seed at all — is dead too.
    //
    // The third condition is why `out.is_empty()` alone is **not**
    // behaviour-preserving: `proc take {dev} {…}; take [listbox .l]` binds
    // `dev` from an empty seed set, as do `Wrap new [listbox .l]` and
    // `take [struct::graph]`.  `empty_seed_fast_path_is_behaviour_preserving`
    // pins all three shapes.
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
        facts.returns_object = returning_procs(cu)
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.clone()))
            .collect();
        retain_normal_procedure_result_types(cu, &mut facts);
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
                            access, read, &fu.name, &name, class, registry,
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
    /// Method [`FunctionUnit`] key → its class's qualified name.
    method_class: HashMap<String, String>,
    /// Class qualified name → the instance-variable names in scope for any of
    /// its methods.  A name in this set is owned by the *class*, so a handle
    /// stored in one method is visible to a dispatch in another.
    class_instance_vars: HashMap<String, HashSet<String>>,
    /// [`ObjectHandleFacts::owner_spans`], already sorted.
    spans: Vec<OwnerSpan>,
}

impl OwnerIndex {
    /// A shared index that owns nothing — the scan context a
    /// [`FactMode::AnyScopeOnly`] build carries, which never consults it.
    /// Mirrors [`crate::compilation_unit::ModuleTraceFacts::none`]'s
    /// "immutable empty, handed out for any lifetime" shape.
    fn shared_empty() -> &'static Self {
        static EMPTY: std::sync::OnceLock<OwnerIndex> = std::sync::OnceLock::new();
        EMPTY.get_or_init(|| Self {
            method_class: HashMap::new(),
            class_instance_vars: HashMap::new(),
            spans: Vec::new(),
        })
    }

    fn build(cu: &CompilationUnit, mode: FactMode) -> Self {
        let mut method_class: HashMap<String, String> = HashMap::new();
        let mut class_instance_vars: HashMap<String, HashSet<String>> = HashMap::new();
        let mut spans: Vec<OwnerSpan> = Vec::new();
        if mode == FactMode::AnyScopeOnly {
            return Self {
                method_class,
                class_instance_vars,
                spans,
            };
        }
        for (mqname, m) in &cu.ir_module.methods {
            method_class.insert(mqname.clone(), m.class_name.clone());
            class_instance_vars
                .entry(m.class_name.clone())
                .or_default()
                .extend(m.instance_vars.iter().cloned());
            if let Some(span) = m.span {
                spans.push(OwnerSpan {
                    span,
                    unit: mqname.clone(),
                    class: Some(m.class_name.clone()),
                });
            }
        }
        for p in cu.ir_module.procedures.values() {
            spans.push(OwnerSpan {
                span: p.span,
                unit: p.qualified_name.clone(),
                class: None,
            });
        }
        // The top level owns everything no proc or method body covers.  Its
        // span starts at 0, so it sorts first and `owner_at`'s
        // greatest-start-that-contains walk still finds the enclosing proc for
        // a byte inside one — but a top-level `set chart [Chart new]` now has
        // an owner to resolve against instead of falling off the index.
        //
        // Synthetic *body units* (`apply` lambdas, `namespace eval` blocks)
        // deliberately get **no** span: the harvest does not visit them
        // (`build_facts_gated` iterates top level / procedures / methods), so a
        // byte inside one resolves to its enclosing owner, which is where any
        // knowledge about the name actually lives.
        spans.push(OwnerSpan {
            span: Span::new(0, u32::try_from(cu.source.len()).unwrap_or(u32::MAX)),
            unit: cu.top_level.name.clone(),
            class: None,
        });
        // Sorted by start ascending for the binary search, then by end
        // *descending* so that when two scopes begin at the same byte — a proc
        // written at offset 0 and the whole-file `::top` entry — the wider one
        // sorts first and `owner_at`'s backward walk still reaches the narrower
        // (innermost) one first.  The unit key is the final tie-break, so the
        // vector is a deterministic function of the source rather than of
        // `HashMap` iteration order (the per-item differential gate compares
        // whole `AnalysisResult`s for equality).
        spans.sort_by(|a, b| {
            (a.span.start(), std::cmp::Reverse(a.span.end()), &a.unit).cmp(&(
                b.span.start(),
                std::cmp::Reverse(b.span.end()),
                &b.unit,
            ))
        });
        Self {
            method_class,
            class_instance_vars,
            spans,
        }
    }

    /// The owner key for `name` as written inside `unit`: the enclosing class
    /// when `name` is one of its instance variables, else the unit itself.
    fn owner_for<'a>(&'a self, unit: &'a str, name: &str) -> &'a str {
        if let Some(class) = self.method_class.get(unit)
            && self
                .class_instance_vars
                .get(class)
                .is_some_and(|vars| vars.contains(name))
        {
            return class;
        }
        unit
    }
}

/// Accumulator threaded through the harvest and the fixpoint so the union and
/// the scope-keyed map are filled from the same binding events.
struct FactSink<'a> {
    facts: ObjectHandleFacts,
    owners: &'a OwnerIndex,
    metadata_context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
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
    fn bind(&mut self, unit: &str, name: &str, class: &str) {
        self.facts
            .any_scope
            .entry(name.to_owned())
            .or_default()
            .insert(class.to_owned());
        if self.mode == FactMode::Full {
            let owner = self.owners.owner_for(unit, name).to_owned();
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
fn retain_normal_procedure_result_types(cu: &CompilationUnit, facts: &mut ObjectHandleFacts) {
    let units = procedure_implementation_units(cu);
    let mut results = crate::type_infer::NormalProcedureResultTypes::new();
    for (allocation, unit) in &units {
        join_procedure_result(&mut results, allocation, unit.return_type.clone());
    }
    let known_classes = cu
        .ir_module
        .methods
        .values()
        .map(|method| method.class_name.clone())
        .collect::<HashSet<_>>();
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
                &known_classes,
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

fn returning_procs(cu: &CompilationUnit) -> HashMap<&str, String> {
    cu.procedures
        .values()
        .filter_map(|unit| {
            (unit.return_type.tcl_type() == Some(tcl_registry::TclType::Object))
                .then_some(unit.return_type.class_name())
                .flatten()
                .map(|class| (unit.name.as_str(), class.to_owned()))
        })
        .collect()
}

/// Advisory factory classes, retaining an unknown return-path residual.
/// These flow candidates never populate the strict returned-object type map.
fn returning_proc_candidates<'a>(
    cu: &'a CompilationUnit,
    registry: &CommandRegistry,
) -> HashMap<&'a str, String> {
    let mut returns = returning_procs(cu);
    let Some(context) =
        crate::registry_invocation::retained_module_metadata_context(registry, &cu.ir_module)
    else {
        return returns;
    };
    let metadata = Some(context.as_ref().into());
    // A retained generic body can have original constructor evidence without
    // an executable FunctionUnit. This is a class candidate only.
    for (name, procedure) in &cu.ir_module.procedures {
        if returns.contains_key(name.as_str()) {
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
                crate::registry_invocation::advisory_return_values_in_context(&context, tokens)
            })
            .unwrap_or_default();
        for value in candidates {
            if let Some(class) = constructor_class(&value.legacy_text(), registry, tokens, metadata)
            {
                returns.insert(name, class);
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
fn returning_methods(cu: &CompilationUnit) -> HashMap<&str, &str> {
    cu.methods
        .iter()
        .filter_map(|(key, fu)| {
            (fu.return_type.tcl_type() == Some(tcl_registry::TclType::Object))
                .then_some(fu.return_type.class_name())
                .flatten()
                .map(|c| (key.as_str(), c))
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
    returns: &HashMap<&'a str, String>,
    sink: &mut FactSink,
) {
    // Callee proc qualified name → parameter names.
    let proc_params: HashMap<&str, &[String]> = cu
        .ir_module
        .procedures
        .values()
        .map(|p| (p.qualified_name.as_str(), p.params.as_slice()))
        .collect();
    // Class qualified name → its constructor's `FunctionUnit` key + parameter
    // names (the IR module keys a constructor `::Class::<constructor>`).
    let ctor_params: HashMap<&str, (&str, &crate::ir::MethodDef)> = cu
        .ir_module
        .methods
        .iter()
        .filter_map(|(k, m)| {
            k.ends_with("::<constructor>")
                .then_some((m.class_name.as_str(), (k.as_str(), m)))
        })
        .collect();
    let method_returns = returning_methods(cu);
    if returns.is_empty()
        && proc_params.is_empty()
        && ctor_params.is_empty()
        && method_returns.is_empty()
    {
        return;
    }
    let index = FlowIndex {
        returns: returns.clone(),
        method_returns,
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
        let bindings = scan_flow_edges(cu, registry, &sink.facts, scoped, &index);
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
                let owner = sink
                    .owners
                    .owner_for(&binding.owner, &binding.name)
                    .to_owned();
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
    returns: HashMap<&'a str, String>,
    /// Method [`FunctionUnit`] key (`::Class::method`) → returned object
    /// class, for the `set b [$a make]` method-return edge.
    method_returns: HashMap<&'a str, &'a str>,
    /// Proc qualified name → parameter names.
    proc_params: HashMap<&'a str, &'a [String]>,
    /// Class qualified name → (constructor unit key, constructor parameters).
    ctor_params: HashMap<&'a str, (&'a str, &'a crate::ir::MethodDef)>,
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
    owner: String,
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
) -> Vec<Binding> {
    let context =
        crate::registry_invocation::retained_module_metadata_context(registry, &cu.ir_module);
    let metadata_context = context.as_deref().map(Into::into);
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
        let ctx = ScanContext {
            ctor_params,
            out: &facts.any_scope,
            by_scope,
            owners,
            registry,
            metadata_context,
            unit: &fu.name,
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
        .procedures
        .iter()
        .filter(|(name, _)| !cu.procedures.contains_key(*name))
        .map(|(name, procedure)| (name.as_str(), &procedure.body))
        .chain(
            cu.ir_module
                .methods
                .iter()
                .filter(|(name, _)| !cu.methods.contains_key(*name))
                .map(|(name, method)| (name.as_str(), &method.body)),
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
            scan_assign_edges(
                AssignSite {
                    unit: ctx.unit,
                    name,
                    value,
                },
                ReturnEdges {
                    procs: returns,
                    methods: method_returns,
                },
                ctx,
                bindings,
            );
        }
        Statement::Call { .. } => {
            if let Some(assignment) = ctx
                .tokens
                .and_then(|tokens| {
                    source_normal_representation(ctx.registry, ctx.metadata_context, tokens)
                })
                .and_then(|normal| normal.value_assignment())
            {
                scan_assign_edges(
                    AssignSite {
                        unit: ctx.unit,
                        name: &assignment.name,
                        value: &assignment.value.legacy_text(),
                    },
                    ReturnEdges {
                        procs: returns,
                        methods: method_returns,
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
                && let Some((callee, params)) = proc_params.get_key_value(target.command.as_str())
                && let Some(effective) = ctx
                    .tokens
                    .and_then(crate::registry_invocation::effective_command_words)
            {
                let args = effective
                    .words
                    .iter()
                    .skip(1)
                    .map(crate::ir::WordExpr::legacy_text)
                    .collect::<Vec<_>>();
                emit_proc_param_bindings(callee, params, &args, ctx, bindings);
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
    unit: &'a str,
    name: &'a str,
    value: &'a str,
}

/// The callee-side return-type maps [`scan_assign_edges`] resolves a
/// bracketed value's head against: [`FlowIndex::returns`] and
/// [`FlowIndex::method_returns`].
#[derive(Clone, Copy)]
struct ReturnEdges<'a, 'b> {
    procs: &'b HashMap<&'a str, String>,
    methods: &'b HashMap<&'a str, &'a str>,
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
    let v = site.value.trim();
    // Aliasing edge: `set A $B` copies B's classes to A.  `B` is read *in this
    // unit*, so the scope-keyed fact resolves it here and nowhere else.
    if let Some(src) = deref_arg_var(v)
        && let Some(classes) = ctx.out.get(src).filter(|s| !s.is_empty())
    {
        bindings.push(Binding {
            owner: site.unit.to_owned(),
            name: site.name.to_owned(),
            union_classes: classes.clone(),
            scoped_classes: ctx.scoped_classes(src),
            kind: ObjectFlowEdge::Alias,
        });
    }
    if !v.starts_with('[') {
        return;
    }
    let Some((cmd, args)) = parse_command_substitution_with_config(
        v,
        tcl_lexer::LexerConfig::for_profile(ctx.registry.profile()),
    ) else {
        return;
    };
    let nested = ctx.tokens.and_then(|parent| {
        let mut candidates = parent
            .words()
            .iter()
            .filter(|word| word.legacy_text() == site.value);
        let word = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        sole_substitution_tokens(word, parent, ctx.registry)
    });
    let target = nested
        .as_ref()
        .and_then(|tokens| tokens.source_binding.as_ref())
        .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target);
    // Return flow follows the retained implementation token, independently
    // of its written spelling or present command slot.
    if let Some(class) = target
        .filter(|target| target.kind == crate::command_binding::BindingKind::Proc)
        .and_then(|target| returns.procs.get(target.command.as_str()))
    {
        // A legitimately cross-scope flow: the source is the *callee's* return
        // type, not a variable read, so there is no scope to confuse and both
        // facts take it.
        let classes = EdgeClasses::unscoped(class);
        bindings.push(Binding {
            owner: site.unit.to_owned(),
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
    // blind, as for the aliasing edge); the method word must be a plain
    // bareword — a computed member (`[$a $m]`) proves nothing.
    if let Some(recv) = deref_arg_var(&cmd)
        && let Some(method) = args
            .first()
            .map(String::as_str)
            .filter(|m| !m.is_empty() && !m.starts_with(['$', '[', '{', '"']))
    {
        let resolve = |classes: &HashSet<String>| -> HashSet<String> {
            classes
                .iter()
                .filter_map(|c| returns.methods.get(format!("{c}::{method}").as_str()))
                .map(|ret| (*ret).to_owned())
                .collect()
        };
        if let Some(recv_classes) = ctx.out.get(recv).filter(|s| !s.is_empty()) {
            let union_classes = resolve(recv_classes);
            if !union_classes.is_empty() {
                bindings.push(Binding {
                    owner: site.unit.to_owned(),
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
    callee: &str,
    params: &[String],
    args: &[String],
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
                owner: callee.to_owned(),
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
    ctor_params: &'a HashMap<&'a str, (&'a str, &'a crate::ir::MethodDef)>,
    out: &'a HashMap<String, HashSet<String>>,
    by_scope: Option<&'a HashMap<(String, String), HashSet<String>>>,
    owners: &'a OwnerIndex,
    registry: &'a CommandRegistry,
    metadata_context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    unit: &'a str,
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
            .get(&(owner.to_owned(), var.to_owned()))
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
    let ctor_args = effective
        .words
        .iter()
        .skip(1 + payload_from)
        .map(crate::ir::WordExpr::legacy_text)
        .collect::<Vec<_>>();
    if ctor_args.is_empty() {
        return;
    }
    let Some(class) = resolve_ctor_class(&target.command) else {
        return;
    };
    let Some((ctor_unit, method)) = ctx.ctor_params.get(class.as_str()) else {
        return;
    };
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
                owner: (*ctor_unit).to_owned(),
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
fn arg_classes(arg: &str, ctx: ScanContext) -> Option<EdgeClasses> {
    if let Some(var) = deref_arg_var(arg)
        && let Some(classes) = ctx.out.get(var).filter(|s| !s.is_empty())
    {
        return Some(EdgeClasses {
            union: classes.clone(),
            scoped: ctx.scoped_classes(var),
        });
    }
    constructor_class(arg, ctx.registry, ctx.tokens, ctx.metadata_context)
        .map(|class| EdgeClasses::unscoped(&class))
}

/// The variable name a `$name` / `${name}` argument dereferences, or `None` for
/// a non-plain reference.
fn deref_arg_var(text: &str) -> Option<&str> {
    let rest = text.trim().strip_prefix('$')?;
    Some(
        rest.strip_prefix('{')
            .and_then(|r| r.strip_suffix('}'))
            .unwrap_or(rest),
    )
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

fn harvest_unit(fu: &FunctionUnit, registry: &CommandRegistry, sink: &mut FactSink) {
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
                        if let Some(class) =
                            constructor_class(value, registry, stmt.tokens(), sink.metadata_context)
                        {
                            sink.bind(&fu.name, name, &class);
                        }
                        // The nested-constructor half of the fast-path gate:
                        // `set w [Wrap new [listbox .l]]` reaches `arg_classes`
                        // through `emit_ctor_param_bindings`.
                        sink.note_constructor_args(&args, registry);
                    }
                }
                // A registry naming factory (`creates_instance_at` + a class)
                // names the new object command positionally, e.g. `struct::graph
                // myG` / `struct::tree myT`.  Only a plain bareword name binds
                // (the `= | := | as | deserialize` operator forms and dynamic
                // `$name` do not create a statically-known handle).
                Statement::Call { args, .. } => {
                    if let Some((name, class)) = stmt
                        .tokens()
                        .and_then(|tokens| {
                            source_normal_representation(registry, sink.metadata_context, tokens)
                        })
                        .and_then(|normal| normal.naming_factory_candidate())
                        && is_plain_object_name(&name)
                    {
                        sink.bind(&fu.name, &name, class);
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
            sink.bind(&fu.name, &name, class);
        }
    }
}

fn harvest_script_factories(
    script: &crate::ir::Script,
    owner: &str,
    registry: &CommandRegistry,
    sink: &mut FactSink,
) {
    let mut pending = vec![script];
    while let Some(script) = pending.pop() {
        for statement in &script.statements {
            if let Some((name, class)) = script
                .retained_source_tokens_for_statement(statement)
                .and_then(|tokens| {
                    source_normal_representation(registry, sink.metadata_context, tokens)
                })
                .and_then(|normal| normal.naming_factory_candidate())
                && is_plain_object_name(&name)
            {
                sink.bind(owner, &name, class);
            }
            pending.extend(statement.child_scripts());
        }
    }
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
) -> Option<String> {
    let parent = parent?;
    let mut result = None;
    for word in parent
        .words()
        .iter()
        .filter(|word| word.legacy_text() == value)
    {
        let tokens = sole_substitution_tokens(word, parent, registry)?;
        let class = constructor_class_candidate(&tokens, registry, context)?;
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
) -> Option<String> {
    if let Some(class) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.proved_construction_result(registry))
    {
        return Some(class);
    }
    let context = context?;
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
    if let Some(class) = source_normal_representation(registry, Some(context), tokens)
        .and_then(|normal| normal.callable_result_class_candidate())
    {
        return Some(class.to_owned());
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
            facts.physical_type_read_at(proof.unit(), proof.source(), proof.evaluation()),
            Some(proof)
        );
        let mut advisory = facts.clone();
        advisory
            .any_scope
            .insert("y".to_owned(), HashSet::from(["::Pin".to_owned()]));
        advisory.by_scope.insert(
            ("::top".to_owned(), "y".to_owned()),
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
            assert!(!facts.returns_object.contains_key("::make"));
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
        let mut v: Vec<String> = facts
            .by_scope
            .get(&(owner.to_owned(), var.to_owned()))
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        v.sort();
        v
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
            unit.procedures.remove("::make");
            unit.procedures.remove("::take");
            let facts = object_handle_facts(&unit, registry);
            assert_eq!(
                facts
                    .any_scope
                    .get("dev")
                    .is_some_and(|classes| classes.contains("::Pin")),
                expected,
                "the retained factory implementation selects the advisory flow: {facts:?}"
            );
            assert!(!facts.returns_object.contains_key("::make"));
        }
    }

    #[test]
    fn owner_attribution_return_edge_binds_in_the_assigning_unit() {
        let (_r, facts) = facts_for(
            "oo::class create Pin { method cfg {args} {} }\n\
             proc make {} { set p [Pin new]\n return $p }\n\
             proc use {} { set q [make]\n $q cfg }\n",
        );
        assert_eq!(scoped(&facts, "::use", "q"), vec!["::Pin".to_owned()]);
        assert_eq!(
            facts.returns_object.get("::make").map(String::as_str),
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
        unit.procedures.remove("::make");
        assert!(returning_procs(&unit).is_empty());
        let current = returning_proc_candidates(&unit, registry);
        assert_eq!(current.get("::make").map(String::as_str), Some("::Pin"));
        let original = unit.ir_module.procedures["::make"].body.clone();
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
            assert!(returning_proc_candidates(&unit, registry).is_empty());
            assert_eq!(unit.ir_module.procedures["::make"].body, original);
        }
    }
}
