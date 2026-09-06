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

//! Flow-sensitive command-binding lattice.
//!
//! Owns both command-binding views used by compiler consumers. The
//! flow-sensitive CFG lattice tracks what each command *name* resolves to at
//! every program point — its original builtin, a user proc, an `interp alias`,
//! or an opaque target. [`ModuleCommandBindings`] is the richer whole-module
//! may-state: it retains every live alias target and its prepended arguments so
//! effect summaries can resolve stored bodies independent of definition order.
//! Both consume registry-owned transitions; no effect consumer interprets a
//! command-table mutation itself.
//!
//! Consumers: the W128 diagnostic ("call to a command renamed/deleted
//! earlier in this file") in `analyser`, and — via the flow-insensitive
//! whole-module summary [`scan_module_command_mutations`] — the
//! optimiser's builtin-fold trust gate.
//!
//! Predecessors come from [`CfgFunction::block_successors`], the canonical
//! successor view shared by CFG analyses.  This includes analysis-only `try`
//! exception edges, so a handler conservatively joins command mutations that
//! may have occurred before control transfers to it.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

use crate::alias::is_current_interpreter;
use crate::cfg::BlockId;
use crate::cfg::Function as CfgFunction;
use crate::ir::{Module, Script, Statement};
use crate::ir_helpers::{evaluated_command_substitutions, nested_bodies};
use crate::naming::is_dynamic_word;
use crate::naming::normalise_qualified_name as nqn;
use crate::var_escape::helpers::invocation_facts;
use tcl_registry::{
    CommandBindingDefinitionKind, CommandBindingTransition, CommandRegistry,
    EffectiveRegistrySemantics, NamespaceTransition, StateTransition, StateTransitionDomain,
    TransitionSubject,
};

/// The lattice element a command name resolves to.
///
/// Height-3 join lattice: [`BindingKind::Bottom`] (⊥) is the identity,
/// a concrete binding joined with itself is unchanged, and two
/// *different* bindings rise to [`BindingKind::Unknown`] (⊤).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// ⊥ — identity for join (no contribution yet).
    Bottom,
    /// The original core/registry command, unperturbed.
    Builtin,
    /// A user procedure (`target` = its canonical qname).
    Proc,
    /// A concrete registry-described command whose narrower identity is not
    /// relevant to this lattice.
    Command,
    /// A `TclOO`/snit/itcl class or instance command created by a
    /// registry-described definer (`target` = its canonical qname).
    /// Distinct from [`Self::Proc`] so `NAME destroy` — the universal
    /// object method — is only modelled as a deletion for names that
    /// actually denote objects.
    Class,
    /// An `interp alias` (`target` = the alias target name).
    Alias,
    /// Renamed/deleted-away or never-defined → dispatches to `unknown`.
    Opaque,
    /// ⊤ — conflicting bindings at a merge, or dynamic mutation.
    Unknown,
}

/// A command-name binding: its [`BindingKind`] plus an optional target
/// (the proc qname for [`BindingKind::Proc`], the alias target for
/// [`BindingKind::Alias`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The kind of binding.
    pub kind: BindingKind,
    /// Target name for `Proc` / `Alias` bindings; `None` otherwise.
    pub target: Option<String>,
}

/// One terminal command target selected by the module-wide may-binding
/// resolver. `prepended` contains every literal argument contributed by an
/// `interp alias` chain, in invocation order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ResolvedCommandTarget {
    pub(crate) command: String,
    pub(crate) prepended: Vec<String>,
    /// Whether `command` denotes the registry descriptor of that spelling.
    /// A retained user procedure is an exact call-graph target but must not
    /// inherit a same-named builtin's registry semantics.
    pub(crate) registry_backed: bool,
    terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum MayBinding {
    Target(ResolvedCommandTarget),
    /// The binding is absent on this path, so Tcl falls back from a local name
    /// to the global spelling (or reports `unknown` at the root).
    Missing,
    /// A command exists, but its implementation cannot be named statically.
    Unknown,
}

/// Immutable registry facts shared by every may-state in one analysis.
///
/// A command-binding walk forks and joins at every structured statement. The
/// fresh-interpreter command set and unresolved-command handlers never change
/// during that walk, so carrying them in each [`ModuleCommandBindings`] clone
/// made every branch and join copy the whole registry universe. Keep that
/// baseline behind one [`Arc`] and let the lattice state stay sparse.
#[derive(Debug, Default)]
struct BindingBaseline {
    /// Registry-owned, generation-specific facts shared with CFG construction.
    semantics: Arc<EffectiveRegistrySemantics>,
}

impl PartialEq for BindingBaseline {
    fn eq(&self, other: &Self) -> bool {
        self.semantics.binding_names() == other.semantics.binding_names()
            && self.semantics.unresolved_command_handlers()
                == other.semantics.unresolved_command_handlers()
    }
}

impl Eq for BindingBaseline {}

impl BindingBaseline {
    fn for_registry(registry: &CommandRegistry) -> Self {
        Self {
            semantics: registry.effective_semantics(),
        }
    }
}

impl Hash for BindingBaseline {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.semantics.binding_fingerprint().hash(state);
    }
}

/// Historical namespace-lookup effects discovered through the closed command
/// lattice. Keeping this beside the binding state means alias-prefixed and
/// recovered invocations publish the same resolution facts as direct calls.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct NamespaceResolutionProjection {
    changed: bool,
    dynamic: bool,
    rebound_names: BTreeSet<String>,
    opaque_namespaces: BTreeSet<String>,
}

/// Compact identity for every mutable observation axis other than the live
/// binding map. All set-valued axes below are monotone during the binding
/// walk, so their cardinality changes exactly when their semantic value grows.
/// The immutable baseline and the once-published root-boundary map are omitted:
/// neither can change during one invocation transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Mirrors independent monotone observation axes.
struct NonBindingObservationStamp {
    opaque_domain: bool,
    opaque_binding_mutation: bool,
    dynamic_proc_binding: bool,
    namespace_changed: bool,
    namespace_dynamic: bool,
    namespace_rebound_count: usize,
    namespace_opaque_count: usize,
    procedure_body_count: usize,
    rebound_count: usize,
    proc_rebound_count: usize,
    has_redefined_procedures: bool,
}

impl NamespaceResolutionProjection {
    fn record(
        &mut self,
        transition: &NamespaceTransition,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) {
        let site = match namespace {
            crate::ir_helpers::ExecutionNamespace::Exact(site) => Some(site.as_str()),
            crate::ir_helpers::ExecutionNamespace::RuntimeSelected => None,
        };
        let mut rebound = std::collections::HashSet::new();
        let mut opaque = std::collections::HashSet::new();
        let mut changed = false;
        if !collect_namespace_resolution(transition, site, &mut rebound, &mut changed, &mut opaque)
        {
            self.dynamic = true;
        }
        self.changed |= changed;
        self.rebound_names.extend(rebound);
        self.opaque_namespaces.extend(opaque);
    }

    fn join(&mut self, other: &Self) -> bool {
        let mut joined = false;
        if other.changed && !self.changed {
            self.changed = true;
            joined = true;
        }
        if other.dynamic && !self.dynamic {
            self.dynamic = true;
            joined = true;
        }
        let rebound_count = self.rebound_names.len();
        self.rebound_names
            .extend(other.rebound_names.iter().cloned());
        joined |= self.rebound_names.len() != rebound_count;
        let opaque_count = self.opaque_namespaces.len();
        self.opaque_namespaces
            .extend(other.opaque_namespaces.iter().cloned());
        joined |= self.opaque_namespaces.len() != opaque_count;
        joined
    }
}

/// Module-wide may-binding resolver for command-effect consumers.
///
/// This is deliberately the sole owner of the richer, alias-prefix-preserving
/// command lattice. Consumers can resolve a source statement, enumerate the
/// source spellings observed by the lattice, or ask whether a source spelling
/// may reach an unknown implementation; they cannot mutate or reinterpret the
/// command table themselves.
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)] // Independent opacity/trust axes; none implies another.
pub struct ModuleCommandBindings {
    /// Sparse flow-sensitive binding state. Analysis forks this state for
    /// every executable root, while most roots leave the command table
    /// unchanged. Copy-on-write keeps those forks allocation-cheap and
    /// detaches only at the centralized replacement/join seams below.
    bindings: Arc<HashMap<String, BTreeSet<MayBinding>>>,
    /// Binding state reachable between executable roots. Unlike `bindings`,
    /// this excludes transient pre/post states within a top-level or body
    /// execution and is therefore safe to replay as another root's entry. It
    /// is published once after the fixpoint and then cloned into each
    /// per-script analysis, so allocation sharing is the natural ownership
    /// model.
    root_boundary_bindings: Arc<HashMap<String, BTreeSet<MayBinding>>>,
    /// Immutable registry facts. Kept separate from `bindings` so the sparse
    /// may-state only publishes names that a module transition actually
    /// affects.
    baseline: Arc<BindingBaseline>,
    /// A registry-declared mutation whose affected name cannot be bounded.
    opaque_domain: bool,
    /// The opaque domain arose from a command-binding/procedure mutation,
    /// rather than from a namespace/lookup transition. Only this dimension
    /// poisons the optimiser's whole-module command-trust projection.
    opaque_binding_mutation: bool,
    /// A dynamic command-binding transition subject (or a runtime-selected
    /// executable root) invalidates a procedure's declared identity. This is
    /// intentionally narrower than [`Self::opaque_binding_mutation`]: an
    /// unavailable/autoloaded body can affect builtin folding without proving
    /// that every retained procedure name was rebound.
    dynamic_proc_binding: bool,
    /// Namespace resolution transitions selected after alias-prefix and
    /// command-binding resolution. This is historical rather than final-state
    /// data: restoring a path/import later cannot make an earlier fold sound.
    namespace_resolution: NamespaceResolutionProjection,
    /// Qualified procedure names for which this analysis owns a retained or
    /// source-recovered body. A `Define(Procedure)` transition may publish a
    /// precise user-command target only when the invocation that produced it
    /// has just placed that body in this inventory.
    /// Copy-on-write because this inventory is module-wide and normally
    /// immutable after initial discovery. The flow-sensitive binding walk
    /// forks state for every executable root; sharing the common inventory
    /// keeps a module with N procedures from cloning N names for each of its
    /// N roots. Exact readable `proc` recovery is the only path that detaches
    /// a branch.
    procedure_bodies: Arc<BTreeSet<String>>,
    /// Exact current-interpreter names vacated, replaced, or introduced by a
    /// move/delete/alias transition. Unlike `bindings`, this is historical:
    /// restoring the final binding does not restore trust in the intervening
    /// procedure identity.
    rebound_names: BTreeSet<String>,
    /// Flow-insensitive, namespace-candidate rebound names for procedure
    /// call-site trust. This retains the legacy scanner's conservative
    /// local-or-global interpretation without weakening the exact binding
    /// resolver above.
    proc_rebound_names: BTreeSet<String>,
    /// `Module` records a duplicate procedure declaration even when each
    /// source body was readable and could be replayed. The optimiser's
    /// historical trust contract deliberately treats that metadata as a
    /// whole-domain mutation.
    has_redefined_procedures: bool,
}

impl PartialEq for ModuleCommandBindings {
    fn eq(&self, other: &Self) -> bool {
        // This public state participates in semantic interning. Two
        // independent analyses of the same registry must therefore compare
        // equal even though each owns a distinct baseline allocation.
        self.baseline == other.baseline
            && self.bindings == other.bindings
            && self.root_boundary_bindings == other.root_boundary_bindings
            && self.opaque_domain == other.opaque_domain
            && self.opaque_binding_mutation == other.opaque_binding_mutation
            && self.dynamic_proc_binding == other.dynamic_proc_binding
            && self.namespace_resolution == other.namespace_resolution
            && self.procedure_bodies == other.procedure_bodies
            && self.rebound_names == other.rebound_names
            && self.proc_rebound_names == other.proc_rebound_names
            && self.has_redefined_procedures == other.has_redefined_procedures
    }
}

impl Eq for ModuleCommandBindings {}

impl ModuleCommandBindings {
    #[cfg(test)]
    pub(crate) fn effective_semantics(&self) -> &Arc<EffectiveRegistrySemantics> {
        &self.baseline.semantics
    }

    /// Fast equality for clones and branches inside *one* analysis. Public
    /// equality remains semantic for Salsa/context interning; the fixpoint
    /// owns one baseline [`Arc`], so its allocation identity is sufficient.
    fn same_state(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.baseline, &other.baseline)
            && (Arc::ptr_eq(&self.bindings, &other.bindings) || self.bindings == other.bindings)
            && (Arc::ptr_eq(&self.root_boundary_bindings, &other.root_boundary_bindings)
                || self.root_boundary_bindings == other.root_boundary_bindings)
            && self.opaque_domain == other.opaque_domain
            && self.opaque_binding_mutation == other.opaque_binding_mutation
            && self.dynamic_proc_binding == other.dynamic_proc_binding
            && self.namespace_resolution == other.namespace_resolution
            && (Arc::ptr_eq(&self.procedure_bodies, &other.procedure_bodies)
                || self.procedure_bodies == other.procedure_bodies)
            && self.rebound_names == other.rebound_names
            && self.proc_rebound_names == other.proc_rebound_names
            && self.has_redefined_procedures == other.has_redefined_procedures
    }

    fn non_binding_observation_stamp(&self) -> NonBindingObservationStamp {
        NonBindingObservationStamp {
            opaque_domain: self.opaque_domain,
            opaque_binding_mutation: self.opaque_binding_mutation,
            dynamic_proc_binding: self.dynamic_proc_binding,
            namespace_changed: self.namespace_resolution.changed,
            namespace_dynamic: self.namespace_resolution.dynamic,
            namespace_rebound_count: self.namespace_resolution.rebound_names.len(),
            namespace_opaque_count: self.namespace_resolution.opaque_namespaces.len(),
            procedure_body_count: self.procedure_bodies.len(),
            rebound_count: self.rebound_names.len(),
            proc_rebound_count: self.proc_rebound_names.len(),
            has_redefined_procedures: self.has_redefined_procedures,
        }
    }
}

impl std::hash::Hash for ModuleCommandBindings {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut entries: Vec<_> = self.bindings.iter().collect();
        entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
        entries.hash(state);
        let mut boundary_entries: Vec<_> = self.root_boundary_bindings.iter().collect();
        boundary_entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
        boundary_entries.hash(state);
        self.baseline.hash(state);
        self.opaque_domain.hash(state);
        self.opaque_binding_mutation.hash(state);
        self.dynamic_proc_binding.hash(state);
        self.namespace_resolution.hash(state);
        self.procedure_bodies.hash(state);
        self.rebound_names.hash(state);
        self.proc_rebound_names.hash(state);
        self.has_redefined_procedures.hash(state);
    }
}

/// A registry invocation selected through the module-wide may-binding state.
/// Alias-prefix arguments have already been prepended.
pub(crate) struct ResolvedBindingInvocation {
    command: String,
    source_span: tcl_lexer::Span,
    pub(crate) facts: Box<tcl_registry::InvocationFacts>,
    pub(crate) arguments: Vec<String>,
    /// Post-alias argv values proven literal at their effective positions.
    /// Dynamic, expanded, and opaque positions are `None` even though
    /// [`Self::arguments`] retains their source spelling for diagnostics and
    /// token metadata alignment.
    literal_arguments: Vec<Option<String>>,
    /// Exact effective argv length after alias-prefix insertion. Expansion or
    /// another indeterminate source word leaves this unknown.
    exact_argument_count: Option<usize>,
}

/// Frame selected for a registry-resolved evaluated script body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedFrameBodySelection {
    /// The frame in which the invocation itself executes.
    Current,
    /// A frame selected by the invocation's registry-owned level grammar.
    Selected(tcl_registry::frame_effect::FrameLevel),
}

/// Source-safe projection of one registry-resolved frame-evaluated body.
pub(crate) enum ResolvedFrameBody {
    /// The invocation has no script-in-frame descriptor.
    NotApplicable,
    /// Tcl rejects the invocation before a body can run.
    KnownError,
    /// One exact source script and its selected frame.
    Readable {
        source: String,
        selection: ResolvedFrameBodySelection,
    },
    /// A body may run in the selected frame, but its source is not exact.
    Opaque {
        selection: ResolvedFrameBodySelection,
    },
}

impl ResolvedBindingInvocation {
    /// Resolve a frame-evaluated body after alias-prefix composition. This is
    /// the sole owner of body-word positioning for late aliases to `eval`,
    /// `uplevel`, and any dialect command with the same registry descriptor.
    pub(crate) fn resolved_frame_body(
        &self,
        registry: &CommandRegistry,
        bindings: &ModuleCommandBindings,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) -> ResolvedFrameBody {
        use tcl_registry::frame_effect::FrameArgLayout;

        let Some(spec) = self.facts.frame_effect else {
            return ResolvedFrameBody::NotApplicable;
        };
        let refs: Vec<&str> = self.arguments.iter().map(String::as_str).collect();
        let (body_index, body_len, selection) = match spec.layout {
            FrameArgLayout::ScriptInCurrentFrame => {
                (0, refs.len(), ResolvedFrameBodySelection::Current)
            }
            FrameArgLayout::ScriptInSelectedFrame => {
                let (level, body) = spec.resolve_for_version(&refs, registry.runtime_version());
                (
                    refs.len().saturating_sub(body.len()),
                    body.len(),
                    ResolvedFrameBodySelection::Selected(level),
                )
            }
            FrameArgLayout::AliasPairs | FrameArgLayout::OpaqueCallerVars => {
                return ResolvedFrameBody::NotApplicable;
            }
        };
        if body_len == 0 {
            return ResolvedFrameBody::KnownError;
        }
        if self.exact_argument_count.is_none() || body_len != 1 {
            return ResolvedFrameBody::Opaque { selection };
        }
        readable_script_argument(self, body_index, registry, bindings, namespace)
            .map_or(ResolvedFrameBody::Opaque { selection }, |source| {
                ResolvedFrameBody::Readable { source, selection }
            })
    }
}

impl ModuleCommandBindings {
    /// Extend the retained-body inventory at the one copy-on-write mutation
    /// seam. Ordinary state forks keep sharing the module's original set;
    /// only a branch that recovers a previously-unseen procedure allocates.
    fn extend_procedure_bodies(&mut self, names: impl IntoIterator<Item = String>) -> bool {
        let additions: Vec<String> = names
            .into_iter()
            .filter(|name| !self.procedure_bodies.contains(name))
            .collect();
        if additions.is_empty() {
            return false;
        }
        let bodies = Arc::make_mut(&mut self.procedure_bodies);
        bodies.extend(additions);
        true
    }

    /// Widen command resolution without claiming an unknown command-table
    /// mutation. Namespace lookup changes are relevant to source-safe
    /// resolution but do not by themselves invalidate optimiser trust.
    fn mark_opaque_resolution(&mut self) {
        self.opaque_domain = true;
    }

    /// Widen both command resolution and the optimiser's command-trust
    /// projection because an unbounded command-binding effect may occur.
    fn mark_opaque_binding_mutation(&mut self) {
        self.opaque_domain = true;
        self.opaque_binding_mutation = true;
    }

    /// Record the narrow event that invalidates an otherwise-retained
    /// procedure's declared binding at arbitrary call sites.
    fn mark_dynamic_proc_binding(&mut self) {
        self.mark_opaque_binding_mutation();
        self.dynamic_proc_binding = true;
    }

    fn record_proc_rebound_candidates(
        &mut self,
        name: &str,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) {
        let crate::ir_helpers::ExecutionNamespace::Exact(namespace) = namespace else {
            self.mark_dynamic_proc_binding();
            return;
        };
        insert_rebound_candidates(name, namespace, &mut self.proc_rebound_names);
    }

    /// Build the closed may-binding state for every retained executable root.
    #[must_use]
    pub(crate) fn analyse(module: &Module, registry: &CommandRegistry) -> Self {
        let discarded = discarded_procedure_history(module, registry);
        let opaque_binding_mutation = discarded.opaque
            || module.oo_evidence.unretained_executable_roots
            || discarded
                .modules
                .iter()
                .any(|module| module.oo_evidence.unretained_executable_roots);
        let dynamic_proc_binding = module
            .independent_executable_script_roots()
            .into_iter()
            .any(|(script, namespace)| {
                matches!(namespace, crate::ir::ExecutionNamespace::RuntimeSelected)
                    && crate::ir_helpers::requires_runtime_command_namespace(script, registry)
            });
        let mut procedure_bodies: BTreeSet<String> = module.procedures.keys().cloned().collect();
        for discarded_module in &discarded.modules {
            procedure_bodies.extend(discarded_module.procedures.keys().cloned());
        }
        let state = Self {
            baseline: Arc::new(BindingBaseline::for_registry(registry)),
            opaque_domain: opaque_binding_mutation,
            opaque_binding_mutation,
            dynamic_proc_binding,
            procedure_bodies: Arc::new(procedure_bodies),
            rebound_names: discarded.rebound_names,
            has_redefined_procedures: !module.redefined_procedures.is_empty(),
            ..Self::default()
        };
        // Execute the top-level root once in source order. Its intermediate
        // states are observable, but they are not valid entry states for a
        // later procedure invocation: replaying a rename/restore sequence
        // from their union creates command-table states Tcl can never reach.
        let mut roots = module.independent_executable_script_roots().into_iter();
        let (top_level, top_namespace) = roots
            .next()
            .expect("a module always publishes its top-level script root");
        let mut retained_roots = RetainedBindingRoots::default();
        retained_roots.extend_module_roots(module, true);
        for discarded_module in &discarded.modules {
            retained_roots.extend_module_roots(discarded_module, false);
        }
        let top = collect_binding_states(
            top_level,
            registry,
            &state,
            &top_namespace,
            &mut retained_roots,
        );
        let mut live = top.post;
        let mut observed = top.observed;

        // Procedure, method, body-unit, and recovered roots may run in any
        // order and more than once. Iterate their *post* states to a fixpoint;
        // retain intermediate states for consumer queries without feeding
        // those historical states back as executable entry states.
        loop {
            let before = live.clone();
            let root_count_before = retained_roots.len();
            let body_roots = retained_roots.snapshot();
            let mut next = live.clone();
            for root in &body_roots {
                let outcome = collect_binding_states(
                    &root.script,
                    registry,
                    &live,
                    &root.namespace,
                    &mut retained_roots,
                );
                if !next.same_state(&outcome.post) {
                    next.join(&outcome.post);
                }
                // An effect-free root observes only its unchanged entry state.
                // That state is already represented by the top-level history
                // (and by every prior fixpoint round), so comparing/joining it
                // against the larger historical union merely rescans the full
                // binding map once per procedure. A root with any transient or
                // lasting transition produces a distinct observed state and
                // still takes the ordinary lattice join below.
                if !live.same_state(&outcome.observed) && !observed.same_state(&outcome.observed) {
                    observed.join(&outcome.observed);
                }
            }
            if next.same_state(&before) && retained_roots.len() == root_count_before {
                // Publish every state that was genuinely observable during a
                // root execution, including temporary aliases. Only boundary
                // post-states feed the fixpoint above, so this historical
                // union is never replayed into an impossible command cycle.
                observed.root_boundary_bindings = next.bindings;
                return observed;
            }
            live = next;
        }
    }

    /// Resolve every live source-safe invocation selected by the exact active
    /// registry. Alias chains are expanded to their terminal target.
    #[must_use]
    pub(crate) fn resolve_statement(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        namespace: &str,
    ) -> Vec<ResolvedBindingInvocation> {
        let source_span = stmt.span();
        let (Statement::Call { args, tokens, .. } | Statement::Barrier { args, tokens, .. }) = stmt
        else {
            return Vec::new();
        };
        if tokens.is_none() {
            return Vec::new();
        }
        let context = registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile);
        let mut resolved = Vec::new();
        self.for_each_resolved_invocation(stmt, namespace, |target, words| {
            if !target.registry_backed {
                return;
            }
            let Some(facts) =
                tcl_registry::model::semantic::resolve_structured_invocation_in_context(
                    registry, context, words,
                )
                .resolved()
                .map(|resolved| Box::new(resolved.facts()))
            else {
                return;
            };
            let mut arguments = target.prepended.clone();
            arguments.extend(args.iter().cloned());
            let invocation_arguments = words.arguments();
            let exact_argument_count = invocation_arguments.exact_argv_len();
            let literal_arguments = (0..arguments.len())
                .map(|index| match invocation_arguments.argv_at(index) {
                    tcl_registry::InvocationArgument::Word(
                        tcl_registry::InvocationWord::Literal(literal),
                    ) => Some(literal.to_owned()),
                    tcl_registry::InvocationArgument::Word(_)
                    | tcl_registry::InvocationArgument::Indeterminate
                    | tcl_registry::InvocationArgument::Missing => None,
                })
                .collect();
            resolved.push(ResolvedBindingInvocation {
                command: target.command.clone(),
                source_span,
                facts,
                arguments,
                literal_arguments,
                exact_argument_count,
            });
        });
        resolved
    }

    /// Visit every effective terminal invocation selected for `stmt`.
    ///
    /// This is the single source of truth for joining source words with the
    /// literal arguments prepended by an `interp alias` chain. Both retained
    /// user procedures and registry-backed targets are reported; consumers
    /// select the semantic domain they own from [`ResolvedCommandTarget`].
    /// A source-aware statement keeps substitution and expansion opaque, while
    /// a hand-built statement without tokens retains the legacy all-literal
    /// argument view.
    pub(crate) fn for_each_resolved_invocation<F>(
        &self,
        stmt: &Statement,
        namespace: &str,
        mut visit: F,
    ) where
        F: for<'w> FnMut(&'w ResolvedCommandTarget, tcl_registry::InvocationWords<'w>),
    {
        let (Statement::Call {
            command,
            args,
            tokens,
            ..
        }
        | Statement::Barrier {
            command,
            args,
            tokens,
            ..
        }) = stmt
        else {
            return;
        };
        let targets = self.targets(command, namespace);
        if let Some(tokens) = tokens {
            let words = tokens.words();
            if !matches!(
                words
                    .first()
                    .map(crate::registry_invocation::invocation_word),
                Some(tcl_registry::InvocationWord::Literal(_))
            ) {
                return;
            }
            for target in targets {
                let mut arguments: Vec<_> = target
                    .prepended
                    .iter()
                    .map(|word| tcl_registry::InvocationWord::Literal(word.as_str()))
                    .collect();
                arguments.extend(
                    words
                        .get(1..)
                        .unwrap_or_default()
                        .iter()
                        .map(crate::registry_invocation::invocation_word),
                );
                visit(
                    &target,
                    tcl_registry::InvocationWords::structured(
                        tcl_registry::InvocationWord::Literal(&target.command),
                        &arguments,
                    ),
                );
            }
            return;
        }

        for target in targets {
            let arguments: Vec<_> = target
                .prepended
                .iter()
                .chain(args.iter())
                .map(|word| tcl_registry::InvocationWord::Literal(word.as_str()))
                .collect();
            visit(
                &target,
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&target.command),
                    &arguments,
                ),
            );
        }
    }

    /// Visit every effective terminal invocation selected for parsed embedded
    /// command words.
    ///
    /// This is the sole binding owner for joining a tokenised embedded command
    /// with the literal argv prefix supplied by an `interp alias` chain. The
    /// caller supplies the exact execution namespace and words tokenised with
    /// the active dialect; consumers never repeat binding or prefix logic.
    pub(crate) fn for_each_resolved_command_words<F>(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        namespace: &str,
        mut visit: F,
    ) where
        F: for<'w> FnMut(&'w ResolvedCommandTarget, tcl_registry::InvocationWords<'w>),
    {
        let Some(command) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            return;
        };
        for target in self.targets(command, namespace) {
            let mut arguments: Vec<_> = target
                .prepended
                .iter()
                .map(|word| tcl_registry::InvocationWord::Literal(word.as_str()))
                .collect();
            arguments.extend(
                words
                    .get(1..)
                    .unwrap_or_default()
                    .iter()
                    .map(crate::ir_helpers::CommandWord::invocation_word),
            );
            visit(
                &target,
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&target.command),
                    &arguments,
                ),
            );
        }
    }

    /// Resolve parsed embedded command words through the same closed binding,
    /// alias-prefix, registry, and dialect context as direct statements.
    #[must_use]
    pub(crate) fn resolve_command_words(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        namespace: &str,
    ) -> Vec<tcl_registry::InvocationFacts> {
        let context = registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile);
        let mut resolved = Vec::new();
        self.for_each_resolved_command_words(words, namespace, |_, invocation_words| {
            if let Some(facts) =
                tcl_registry::model::semantic::resolve_structured_invocation_in_context(
                    registry,
                    context,
                    invocation_words,
                )
                .resolved()
                .map(|invocation| invocation.facts())
            {
                resolved.push(facts);
            }
        });
        resolved
    }

    /// Project every registry-backed variable value write a direct statement
    /// may perform after command binding and alias-prefix resolution.
    ///
    /// This is the compiler's single bridge from the module command lattice
    /// to the registry-owned write projection.  In particular, an alias such
    /// as `interp alias {} put {} set x` contributes the literal `x` before
    /// the source call's words, while an alias to a retained user procedure
    /// never borrows a same-named registry command's semantics.
    #[must_use]
    pub(crate) fn variable_write_projection(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        namespace: &str,
    ) -> tcl_registry::VariableWriteProjection {
        let literal_head = match stmt {
            Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. } => {
                tokens.as_ref().is_none_or(|tokens| {
                    tokens.synthetic.is_none()
                        && tokens.words().first().is_none_or(|head| {
                            crate::registry_invocation::invocation_word(head)
                                .literal()
                                .is_some()
                        })
                })
            }
            _ => return tcl_registry::VariableWriteProjection::default(),
        };
        if !literal_head {
            // The ordinary computed-command dispatch path owns this barrier;
            // do not manufacture a second variable-frame effect here.
            return tcl_registry::VariableWriteProjection::default();
        }

        let mut projection = tcl_registry::VariableWriteProjection::default();
        self.for_each_resolved_invocation(stmt, namespace, |target, words| {
            if !target.registry_backed {
                return;
            }
            let candidate = registry.variable_write_projection(words);
            projection.opaque_variable_frame |= candidate.opaque_variable_frame;
            for name in candidate.literal_names {
                if !projection.literal_names.contains(&name) {
                    projection.literal_names.push(name);
                }
            }
        });
        projection
    }

    /// Every source spelling explicitly represented by the closed may-state.
    #[must_use]
    pub(crate) fn source_spellings(&self) -> Vec<String> {
        let mut names: Vec<_> = self.bindings.keys().cloned().collect();
        names.sort_unstable();
        names
    }

    /// Whether any command spelling may have been affected by an unbounded
    /// command-table mutation.
    #[must_use]
    pub(crate) const fn has_opaque_domain(&self) -> bool {
        self.opaque_domain
    }

    /// Project this already-computed binding summary into the optimiser's
    /// flow-insensitive command-binding trust view.
    ///
    /// This is the allocation boundary for consumers that already retain a
    /// [`ModuleCommandBindings`]: they must not repeat the module walk via
    /// [`scan_module_command_mutations`]. The published state includes every
    /// transient executable binding, so a rename followed by a restore still
    /// distrusts the affected builtin. Registry-declared namespace effects are
    /// recorded by the same resolved invocation walk, including alias prefixes
    /// and recovered executable roots.
    #[must_use]
    pub(crate) fn mutation_projection(&self, registry: &CommandRegistry) -> ModuleCommandMutations {
        let mut names = std::collections::HashSet::new();
        for (name, observed) in self.bindings.iter() {
            if default_binding(name, registry).kind != BindingKind::Builtin {
                continue;
            }
            let original = Self::unmodified_bindings(name, self.baseline.semantics.binding_names());
            if *observed != original {
                names.insert(name.clone());
            }
        }
        ModuleCommandMutations {
            names,
            rebound: self
                .rebound_names
                .iter()
                .chain(self.namespace_resolution.rebound_names.iter())
                .cloned()
                .collect(),
            dynamic: self.opaque_binding_mutation
                || self.namespace_resolution.dynamic
                || self.has_redefined_procedures,
            resolution_changed: self.namespace_resolution.changed,
            opaque_namespaces: self
                .namespace_resolution
                .opaque_namespaces
                .iter()
                .cloned()
                .collect(),
        }
    }

    /// Project the prepared lattice into the narrower trust fact used when a
    /// call site wants to seed a retained procedure's parameters. This keeps
    /// the legacy contract: only an explicit rebinding or a dynamic
    /// command-binding transition disqualifies the declared procedure
    /// identity; source/lookup/body opacity alone does not.
    #[must_use]
    pub(crate) fn proc_binding_trust_projection(&self) -> ProcBindingTrustProjection {
        ProcBindingTrustProjection {
            rebound: self.proc_rebound_names.iter().cloned().collect(),
            dynamic: self.dynamic_proc_binding || self.has_redefined_procedures,
        }
    }

    /// Whether executing `script` itself introduces an unbounded command
    /// binding effect, excluding opacity inherited from unrelated module
    /// roots. Effect-summary consumers use this delta query instead of
    /// projecting one root's uncertainty onto every procedure.
    #[must_use]
    pub(crate) fn script_has_opaque_binding_effect(
        &self,
        script: &Script,
        registry: &CommandRegistry,
        namespace: &str,
    ) -> bool {
        let mut baseline = self.clone();
        baseline.opaque_domain = false;
        baseline.opaque_binding_mutation = false;
        baseline.dynamic_proc_binding = false;
        baseline.bindings.clone_from(&self.root_boundary_bindings);
        let mut retained_roots = RetainedBindingRoots::default();
        let execution_namespace = crate::ir::ExecutionNamespace::exact(namespace);
        let outcome = collect_binding_states(
            script,
            registry,
            &baseline,
            &execution_namespace,
            &mut retained_roots,
        );
        outcome.post.opaque_binding_mutation || outcome.observed.opaque_binding_mutation
    }

    #[cfg(test)]
    fn rebound_names(&self) -> impl Iterator<Item = &String> {
        self.rebound_names.iter()
    }

    /// Resolve every terminal target a source spelling may invoke in
    /// `namespace`, including arguments prepended by alias chains.
    #[must_use]
    pub(crate) fn targets(&self, name: &str, namespace: &str) -> BTreeSet<ResolvedCommandTarget> {
        self.resolve_targets(name, namespace, &mut BTreeSet::new())
    }

    /// Whether invoking `name` in `namespace` may reach an implementation that
    /// cannot be named statically.
    #[must_use]
    pub(crate) fn target_may_be_unknown(&self, name: &str, namespace: &str) -> bool {
        self.has_opaque_domain() || self.target_resolution_may_be_unknown(name, namespace)
    }

    /// Whether this exact source spelling's resolved binding may be unknown,
    /// excluding unrelated module-wide opacity. Semantic effect consumers use
    /// this narrower query; runtime provenance still consumes
    /// [`Self::target_may_be_unknown`] and [`Self::has_opaque_domain`].
    #[must_use]
    pub(crate) fn target_resolution_may_be_unknown(&self, name: &str, namespace: &str) -> bool {
        self.resolve_target_may_be_unknown(name, namespace, &mut BTreeSet::new())
    }

    fn lookup_key(&self, name: &str, namespace: &str) -> Option<String> {
        if name.starts_with("::") {
            let key = nqn(name);
            return self.bindings.contains_key(&key).then_some(key);
        }
        if namespace != "::" {
            let key = tcl_syntax::naming::qualify(namespace, name);
            if self.bindings.contains_key(&key) {
                return Some(key);
            }
        }
        let key = nqn(name);
        self.bindings.contains_key(&key).then_some(key)
    }

    /// Candidate source bindings for Tcl's local-then-global lookup. A local
    /// may-set containing `Missing` does not suppress global fallback.
    fn source_keys(&self, name: &str, namespace: &str) -> Vec<String> {
        if name.starts_with("::") || namespace == "::" {
            return vec![nqn(name)];
        }
        let local = tcl_syntax::naming::qualify(namespace, name);
        let global = nqn(name);
        let Some(bindings) = self.bindings.get(&local) else {
            return vec![global];
        };
        let mut keys = Vec::new();
        if bindings
            .iter()
            .any(|binding| !matches!(binding, MayBinding::Missing))
        {
            keys.push(local);
        }
        if bindings.contains(&MayBinding::Missing) {
            keys.push(global);
        }
        keys
    }

    fn resolve_target_may_be_unknown(
        &self,
        name: &str,
        namespace: &str,
        visiting: &mut BTreeSet<String>,
    ) -> bool {
        let Some(key) = self.lookup_key(name, namespace) else {
            // Sparse state omits untouched registry commands. Any other
            // absent literal is dispatched through this dialect's active
            // registry-declared unknown-handler carrier, when one exists.
            if self.baseline.semantics.binding_names().contains(&nqn(name)) {
                return false;
            }
            return self.unresolved_target_may_be_unknown(name, visiting);
        };
        if !visiting.insert(key.clone()) {
            return true;
        }
        let unknown = self.bindings[&key].iter().any(|binding| match binding {
            MayBinding::Unknown => true,
            MayBinding::Missing
                if !name.starts_with("::")
                    && namespace != "::"
                    && key == tcl_syntax::naming::qualify(namespace, name) =>
            {
                self.resolve_target_may_be_unknown(name, "::", visiting)
            }
            MayBinding::Missing => self.unresolved_target_may_be_unknown(name, visiting),
            MayBinding::Target(target) if target.terminal => false,
            MayBinding::Target(target) => {
                self.resolve_target_may_be_unknown(&target.command, "::", visiting)
            }
        });
        visiting.remove(&key);
        unknown
    }

    /// Whether Tcl's registry-declared unresolved-command fallback can reach
    /// an implementation whose effects are not statically bounded. A missing
    /// handler never dispatches recursively through itself.
    fn unresolved_target_may_be_unknown(
        &self,
        missing_name: &str,
        visiting: &mut BTreeSet<String>,
    ) -> bool {
        let missing_key = nqn(missing_name);
        self.baseline
            .semantics
            .unresolved_command_handlers()
            .iter()
            .filter(|handler| **handler != missing_key)
            .any(|handler| self.resolve_target_may_be_unknown(handler, "::", visiting))
    }

    /// Resolve Tcl's registry-declared unresolved-command fallback, retaining
    /// the missing command name as the argument Tcl appends to the handler's
    /// command prefix. A missing handler is terminal failure, not recursion.
    fn unresolved_targets(
        &self,
        missing_name: &str,
        visiting: &mut BTreeSet<String>,
    ) -> BTreeSet<ResolvedCommandTarget> {
        let missing_key = nqn(missing_name);
        let mut resolved = BTreeSet::new();
        for handler in self
            .baseline
            .semantics
            .unresolved_command_handlers()
            .iter()
            .filter(|handler| **handler != missing_key)
        {
            for mut terminal in self.resolve_targets(handler, "::", visiting) {
                terminal.prepended.push(missing_name.to_owned());
                resolved.insert(terminal);
            }
        }
        resolved
    }

    fn resolve_targets(
        &self,
        name: &str,
        namespace: &str,
        visiting: &mut BTreeSet<String>,
    ) -> BTreeSet<ResolvedCommandTarget> {
        let Some(key) = self.lookup_key(name, namespace) else {
            if self.baseline.semantics.binding_names().contains(&nqn(name)) {
                return BTreeSet::from([ResolvedCommandTarget {
                    command: name.to_owned(),
                    prepended: Vec::new(),
                    registry_backed: true,
                    terminal: true,
                }]);
            }
            return self.unresolved_targets(name, visiting);
        };
        if !visiting.insert(key.clone()) {
            return BTreeSet::new();
        }
        let mut resolved = BTreeSet::new();
        for binding in &self.bindings[&key] {
            match binding {
                MayBinding::Missing
                    if !name.starts_with("::")
                        && namespace != "::"
                        && key == tcl_syntax::naming::qualify(namespace, name) =>
                {
                    resolved.extend(self.resolve_targets(name, "::", visiting));
                }
                MayBinding::Target(target) if target.terminal => {
                    resolved.insert(target.clone());
                }
                MayBinding::Target(target) => {
                    // Alias targets execute in the target interpreter's global
                    // command namespace, independent of the source namespace.
                    for mut terminal in self.resolve_targets(&target.command, "::", visiting) {
                        terminal.prepended.extend(target.prepended.iter().cloned());
                        resolved.insert(terminal);
                    }
                }
                MayBinding::Missing => {
                    resolved.extend(self.unresolved_targets(name, visiting));
                }
                MayBinding::Unknown => {}
            }
        }
        visiting.remove(&key);
        resolved
    }

    /// Replace sparse binding entries without detaching a shared state when
    /// every requested value is already present.
    fn replace_bindings(
        &mut self,
        bindings: impl IntoIterator<Item = (String, BTreeSet<MayBinding>)>,
    ) -> bool {
        let replacements: Vec<_> = bindings
            .into_iter()
            .filter(|(key, value)| self.bindings.get(key) != Some(value))
            .collect();
        if replacements.is_empty() {
            return false;
        }
        Arc::make_mut(&mut self.bindings).extend(replacements);
        true
    }

    fn replace(&mut self, key: String, bindings: BTreeSet<MayBinding>) {
        self.replace_bindings([(key, bindings)]);
    }

    /// Join one known-changed binding into the historical may-state. The
    /// ordinary lattice join handles arbitrary branches; a deterministic
    /// registry transition already identifies its exact key and should not
    /// rescan every unchanged binding merely to publish that one delta.
    fn join_binding_from(&mut self, other: &Self, key: &str) -> bool {
        let mut joined = self.bindings.get(key).cloned().unwrap_or_else(|| {
            Self::unmodified_bindings(key, self.baseline.semantics.binding_names())
        });
        joined.extend(other.bindings.get(key).cloned().unwrap_or_else(|| {
            Self::unmodified_bindings(key, self.baseline.semantics.binding_names())
        }));
        self.replace_bindings([(key.to_owned(), joined)])
    }

    fn remove(&mut self, key: String) {
        self.replace(key, BTreeSet::from([MayBinding::Missing]));
    }

    fn unmodified_bindings(
        key: &str,
        initial_registry_bindings: &BTreeSet<String>,
    ) -> BTreeSet<MayBinding> {
        if initial_registry_bindings.contains(key) {
            BTreeSet::from([MayBinding::Target(ResolvedCommandTarget {
                command: key.to_owned(),
                prepended: Vec::new(),
                registry_backed: true,
                terminal: true,
            })])
        } else {
            // A name first introduced by this module did not exist on the
            // pre-transition path. Treating it as an executable self-target
            // invents a user command and makes an exact alias spuriously
            // opaque when the before/after states are joined.
            BTreeSet::from([MayBinding::Missing])
        }
    }

    /// Join two sparse states sharing one immutable registry baseline.
    ///
    /// The boolean lets callers avoid publishing an observational history
    /// entry when a transition's alternatives made no lattice change.
    fn join(&mut self, other: &Self) -> bool {
        debug_assert!(
            Arc::ptr_eq(&self.baseline, &other.baseline),
            "a binding analysis may only join states from one registry baseline"
        );
        let mut changed = false;
        if other.opaque_domain && !self.opaque_domain {
            self.opaque_domain = true;
            changed = true;
        }
        if other.opaque_binding_mutation && !self.opaque_binding_mutation {
            self.opaque_binding_mutation = true;
            changed = true;
        }
        if other.dynamic_proc_binding && !self.dynamic_proc_binding {
            self.dynamic_proc_binding = true;
            changed = true;
        }
        changed |= self.namespace_resolution.join(&other.namespace_resolution);
        if !Arc::ptr_eq(&self.procedure_bodies, &other.procedure_bodies)
            && self.procedure_bodies != other.procedure_bodies
        {
            changed |= self.extend_procedure_bodies(other.procedure_bodies.iter().cloned());
        }
        let rebound_count = self.rebound_names.len();
        self.rebound_names
            .extend(other.rebound_names.iter().cloned());
        changed |= self.rebound_names.len() != rebound_count;
        let proc_rebound_count = self.proc_rebound_names.len();
        self.proc_rebound_names
            .extend(other.proc_rebound_names.iter().cloned());
        changed |= self.proc_rebound_names.len() != proc_rebound_count;
        if other.has_redefined_procedures && !self.has_redefined_procedures {
            self.has_redefined_procedures = true;
            changed = true;
        }
        if !Arc::ptr_eq(&self.bindings, &other.bindings) && self.bindings != other.bindings {
            let keys: BTreeSet<String> = self
                .bindings
                .keys()
                .chain(other.bindings.keys())
                .cloned()
                .collect();
            let joined = keys.into_iter().map(|key| {
                let mut bindings = self.bindings.get(&key).cloned().unwrap_or_else(|| {
                    Self::unmodified_bindings(&key, self.baseline.semantics.binding_names())
                });
                bindings.extend(other.bindings.get(&key).cloned().unwrap_or_else(|| {
                    Self::unmodified_bindings(&key, self.baseline.semantics.binding_names())
                }));
                (key, bindings)
            });
            changed |= self.replace_bindings(joined.collect::<Vec<_>>());
        }
        changed
    }
}

#[derive(Default)]
struct DiscardedProcedureHistory {
    modules: Vec<Module>,
    rebound_names: BTreeSet<String>,
    opaque: bool,
}

/// Recover the bodies which [`Module::procedures`] cannot retain when one
/// statically named procedure is defined more than once. The declaration
/// statements remain in executable IR, so typed procedure-definition
/// provenance can distinguish a readable discarded body from a genuinely
/// unavailable one. Readable bodies are lowered and walked like every retained
/// procedure body; only an unreadable history widens the whole command domain.
fn discarded_procedure_history(
    module: &Module,
    registry: &CommandRegistry,
) -> DiscardedProcedureHistory {
    fn walk(
        script: &Script,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        retained_procedures: &HashSet<(String, tcl_lexer::Span)>,
        registry: &CommandRegistry,
        occurrences: &mut HashMap<String, usize>,
        history: &mut DiscardedProcedureHistory,
        depth: u32,
    ) {
        if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            history.opaque = true;
            return;
        }
        for stmt in &script.statements {
            if let Some((name, body)) = procedure_declaration(stmt, registry) {
                let crate::ir_helpers::ExecutionNamespace::Exact(declaration_namespace) = namespace
                else {
                    // A caller-selected frame makes an otherwise literal
                    // procedure name runtime-dependent.
                    history.opaque = true;
                    continue;
                };
                let qname = tcl_syntax::naming::qualify(declaration_namespace, &name);
                let count = occurrences.entry(qname.clone()).or_default();
                *count += 1;
                if *count > 1 {
                    history.rebound_names.insert(qname.clone());
                }
                let represented_exactly =
                    retained_procedures.contains(&(qname.clone(), stmt.span()));
                if represented_exactly {
                    // The retained body is already one of `module`'s
                    // executable roots. An unreadable source body, however,
                    // may have lowered to an empty placeholder and cannot be
                    // treated as an exact no-op.
                    if body.is_none() {
                        history.opaque = true;
                    }
                } else if let Some(body) = body {
                    let recovered = lower_recovered_procedure_body(&body, &qname, registry);
                    // A second generation of discarded bodies is not
                    // available from the parent module's declaration
                    // history. Preserve soundness rather than recursively
                    // guessing which nested definition survived.
                    history.opaque |= !recovered.redefined_procedures.is_empty();
                    history.modules.push(recovered);
                } else {
                    history.opaque = true;
                }
            }

            for (body, body_namespace) in
                crate::ir_helpers::nested_execution_bodies(stmt, namespace)
            {
                walk(
                    body,
                    &body_namespace,
                    retained_procedures,
                    registry,
                    occurrences,
                    history,
                    depth + 1,
                );
            }
        }
    }

    let mut history = DiscardedProcedureHistory::default();
    let retained_procedures = module
        .procedures
        .iter()
        .map(|(qname, procedure)| (qname.clone(), procedure.span))
        .collect();
    let mut occurrences = HashMap::new();
    // Keep body units here: namespace/apply barriers do not retain their body
    // as nested statement IR, so each separately lowered body unit is the one
    // place this declaration-history scan can see definitions inside it.
    for (script, namespace) in module.executable_script_roots() {
        walk(
            script,
            &namespace,
            &retained_procedures,
            registry,
            &mut occurrences,
            &mut history,
            0,
        );
    }
    history
}

fn procedure_declaration(
    stmt: &Statement,
    registry: &CommandRegistry,
) -> Option<(String, Option<String>)> {
    use tcl_registry::SemanticOperationId;
    use tcl_registry::hooks::LoweringHookId;

    let (Statement::Call {
        command,
        args,
        tokens,
        ..
    }
    | Statement::Barrier {
        command,
        args,
        tokens,
        ..
    }) = stmt
    else {
        return None;
    };
    let facts = invocation_facts(stmt, registry)?;
    if facts.operation != SemanticOperationId::StructuredLowering(LoweringHookId::Proc) {
        return None;
    }
    let name = facts
        .state_transitions
        .declared()?
        .command_bindings()
        .find_map(|transition| match transition {
            CommandBindingTransition::Define { name, kind }
                if *kind == CommandBindingDefinitionKind::Procedure =>
            {
                name.literal()
            }
            _ => None,
        })?;
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let body_index = registry
        .arg_indices_for_role(command, &arg_refs, tcl_registry::ArgRole::Body)
        .into_iter()
        .next();
    let body = body_index.and_then(|index| {
        tokens
            .as_ref()?
            .words()
            .get(index + 1)
            .and_then(|word| crate::registry_invocation::invocation_word(word).literal())
            .map(str::to_owned)
    });
    Some((name.to_owned(), body))
}

fn procedure_namespace(qname: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
    if holder.is_empty() {
        "::".to_owned()
    } else {
        holder.to_owned()
    }
}

fn lower_recovered_procedure_body(body: &str, qname: &str, registry: &CommandRegistry) -> Module {
    let body_namespace = procedure_namespace(qname);
    let config = registry
        .profile()
        .map_or_else(tcl_lexer::LexerConfig::default, |profile| {
            tcl_lexer::LexerConfig::from_grammar(profile.grammar)
        });
    let mut lowerer =
        crate::lowering::Lowerer::with_config(registry, config).with_dialect(registry.profile());
    let unrooted = body_namespace.strip_prefix("::").unwrap_or(&body_namespace);
    lowerer.lower_procedure_target(body, unrooted);
    lowerer.finish_module(body)
}

struct BindingWalkOutcome {
    /// State reachable after the script completes.
    post: ModuleCommandBindings,
    /// Union of states observed after executable invocations within the
    /// script. This is queryable history, never a replay entry state.
    observed: ModuleCommandBindings,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RetainedBindingRoot {
    /// A recovered body must outlive the temporary `Module` used to lower
    /// it, but fixpoint snapshots need only duplicate a handle, not its full
    /// IR tree.
    script: Arc<Script>,
    namespace: crate::ir::ExecutionNamespace,
}

/// Monotone inventory of callable roots discovered while replaying exact,
/// readable Tcl. It is intentionally separate from [`ModuleCommandBindings`]:
/// roots control the fixpoint worklist, while the published lattice and memo
/// identity contain only observable command-table state.
#[derive(Default)]
struct RetainedBindingRoots {
    roots: Vec<RetainedBindingRoot>,
    seen: HashSet<RetainedBindingRoot>,
    evaluated_bodies: HashMap<crate::ir::ExecutionNamespace, Vec<RetainedEvaluatedBody>>,
}

struct RetainedEvaluatedBody {
    span: tcl_lexer::Span,
    source: String,
    script: Arc<crate::ir::Script>,
}

impl RetainedBindingRoots {
    fn insert(&mut self, script: &Script, namespace: crate::ir::ExecutionNamespace) {
        let root = RetainedBindingRoot {
            script: Arc::new(script.clone()),
            namespace,
        };
        if self.seen.insert(root.clone()) {
            self.roots.push(root);
        }
    }

    /// Retain every non-top root, or every root when `skip_top` is false.
    fn extend_module_roots(&mut self, module: &Module, skip_top: bool) {
        // Evaluated bodies keep their owning source location so identical Tcl
        // text lowered under different command states cannot collide. The
        // binding walk selects one only from inside the invocation currently
        // being replayed.
        let mut body_units: Vec<_> = module.body_units.iter().collect();
        body_units.sort_by(|a, b| {
            a.1.span
                .start()
                .cmp(&b.1.span.start())
                .then_with(|| a.0.cmp(b.0))
        });
        for (qname, unit) in body_units {
            let Ok(start) = usize::try_from(unit.span.start()) else {
                continue;
            };
            let Ok(end) = usize::try_from(unit.span.end()) else {
                continue;
            };
            let Some(source) = module.source.get(start..end) else {
                continue;
            };
            let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
            let namespace =
                crate::ir::ExecutionNamespace::exact(if holder.is_empty() { "::" } else { holder });
            self.evaluated_bodies
                .entry(namespace)
                .or_default()
                .push(RetainedEvaluatedBody {
                    span: unit.span,
                    source: source.to_owned(),
                    script: Arc::new(unit.body.clone()),
                });
        }
        for (script, namespace) in module
            .independent_executable_script_roots()
            .into_iter()
            .skip(usize::from(skip_top))
        {
            self.insert(script, namespace);
        }
    }

    fn evaluated_body(
        &self,
        source: &str,
        namespace: &crate::ir::ExecutionNamespace,
        invocation_span: tcl_lexer::Span,
    ) -> Option<Arc<crate::ir::Script>> {
        self.evaluated_bodies
            .get(namespace)?
            .iter()
            .find_map(|body| {
                (body.source == source
                    && body.span.start() >= invocation_span.start()
                    && body.span.end() <= invocation_span.end())
                .then(|| Arc::clone(&body.script))
            })
    }

    fn len(&self) -> usize {
        self.roots.len()
    }

    fn snapshot(&self) -> Vec<RetainedBindingRoot> {
        self.roots.clone()
    }
}

fn observe_binding_state(
    observed: &mut Option<ModuleCommandBindings>,
    current: &ModuleCommandBindings,
) {
    if let Some(state) = observed {
        // A call with no command-table transition is still an executable
        // observation (the caller's final fallback covers that state), but
        // it must not repeatedly merge an identical sparse lattice point.
        if !state.same_state(current) {
            state.join(current);
        }
    } else {
        *observed = Some(current.clone());
    }
}

// Flow-sensitive recursive join over every structured IR form.
#[allow(clippy::too_many_lines)]
fn collect_binding_states(
    script: &Script,
    registry: &CommandRegistry,
    initial: &ModuleCommandBindings,
    namespace: &crate::ir::ExecutionNamespace,
    retained_roots: &mut RetainedBindingRoots,
) -> BindingWalkOutcome {
    // Recursive walker keeps branch joins and side effects together.
    #[allow(clippy::too_many_lines)]
    fn walk(
        script: &Script,
        registry: &CommandRegistry,
        current: &mut ModuleCommandBindings,
        observed: &mut Option<ModuleCommandBindings>,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        retained_roots: &mut RetainedBindingRoots,
        depth: u32,
    ) {
        if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            current.mark_opaque_binding_mutation();
            observe_binding_state(observed, current);
            return;
        }
        // Typed-lowering sites are sparse sidecars, but a large generated
        // script can have many statements and many sites. Index them once for
        // this script instead of scanning every site for every statement.
        // Keep the index borrowed: a repeated synthetic statement span still
        // observes every site attached to that source command, exactly as the
        // former filter loop did.
        let mut binding_sites_by_span: HashMap<_, Vec<_>> = HashMap::new();
        for site in script.command_binding_sites.iter() {
            binding_sites_by_span
                .entry(site.span)
                .or_default()
                .push(site);
        }
        let statement_spans: HashSet<_> = script.statements.iter().map(Statement::span).collect();
        for stmt in &script.statements {
            if apply_embedded_transitions(stmt, registry, current, namespace) {
                observe_binding_state(observed, current);
            }
            let mut binding_site_widened = false;
            for site in binding_sites_by_span
                .get(&stmt.span())
                .into_iter()
                .flatten()
            {
                if !command_binding_site_is_exact(site, current, namespace) {
                    current.mark_opaque_resolution();
                    binding_site_widened = true;
                }
            }
            if binding_site_widened {
                observe_binding_state(observed, current);
            }
            if let Statement::Call { command, .. } | Statement::Barrier { command, .. } = stmt {
                if let Some(statement_namespace) = namespace.for_head(command) {
                    let observation_was_updated = apply_may_invocation_transitions(
                        stmt,
                        registry,
                        current,
                        observed,
                        statement_namespace,
                        namespace,
                        retained_roots,
                    );
                    if !observation_was_updated {
                        observe_binding_state(observed, current);
                    }
                } else {
                    current.mark_opaque_resolution();
                    observe_binding_state(observed, current);
                }
            }
            match stmt {
                Statement::Block { .. } | Statement::UpFrame { .. } => {
                    for (body, body_namespace) in
                        crate::ir_helpers::nested_execution_bodies(stmt, namespace)
                    {
                        walk(
                            body,
                            registry,
                            current,
                            observed,
                            &body_namespace,
                            retained_roots,
                            depth + 1,
                        );
                    }
                }
                Statement::If {
                    clauses, else_body, ..
                } => {
                    let incoming = current.clone();
                    let mut joined: Option<ModuleCommandBindings> = None;
                    for body in clauses.iter().map(|clause| &clause.body).chain(else_body) {
                        let mut branch = incoming.clone();
                        walk(
                            body,
                            registry,
                            &mut branch,
                            observed,
                            namespace,
                            retained_roots,
                            depth + 1,
                        );
                        if let Some(state) = &mut joined {
                            if !state.same_state(&branch) {
                                state.join(&branch);
                            }
                        } else {
                            joined = Some(branch);
                        }
                    }
                    if else_body.is_none() {
                        if let Some(state) = &mut joined {
                            if !state.same_state(&incoming) {
                                state.join(&incoming);
                            }
                        } else {
                            joined = Some(incoming.clone());
                        }
                    }
                    *current = joined.unwrap_or(incoming);
                    observe_binding_state(observed, current);
                }
                _ => {
                    for body in nested_bodies(stmt) {
                        let incoming = current.clone();
                        let mut branch = incoming.clone();
                        walk(
                            body,
                            registry,
                            &mut branch,
                            observed,
                            namespace,
                            retained_roots,
                            depth + 1,
                        );
                        if !current.same_state(&branch) {
                            current.join(&branch);
                        }
                        observe_binding_state(observed, current);
                    }
                }
            }
        }
        // A typed dependency without a surviving owner statement cannot be
        // ordered against command-table transitions. Fail closed rather than
        // silently trusting an orphaned sidecar after an IR transform.
        if binding_sites_by_span
            .keys()
            .any(|span| !statement_spans.contains(span))
        {
            current.mark_opaque_resolution();
            observe_binding_state(observed, current);
        }
    }

    let mut current = initial.clone();
    let mut observed = None;
    walk(
        script,
        registry,
        &mut current,
        &mut observed,
        namespace,
        retained_roots,
        0,
    );
    BindingWalkOutcome {
        observed: observed.unwrap_or_else(|| current.clone()),
        post: current,
    }
}

/// Whether one typed-lowering dependency still denotes exactly the registry
/// implementation consumed by the compiler at this execution point.
fn command_binding_site_is_exact(
    site: &crate::ir::CommandBindingSite,
    bindings: &ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
) -> bool {
    let source_namespace = if site.binding.name.starts_with("::") {
        "::"
    } else {
        let crate::ir_helpers::ExecutionNamespace::Exact(namespace) = namespace else {
            return false;
        };
        let recorded = tcl_syntax::naming::root_unrooted_key(&site.binding.resolution_namespace);
        if recorded != *namespace {
            return false;
        }
        namespace
    };
    if bindings.target_may_be_unknown(&site.binding.name, source_namespace) {
        return false;
    }
    let targets = bindings.targets(&site.binding.name, source_namespace);
    let mut targets = targets.iter();
    let Some(target) = targets.next() else {
        return false;
    };
    if targets.next().is_some() {
        return false;
    }
    target.registry_backed
        && target.prepended.is_empty()
        && nqn(&target.command) == nqn(&site.binding.identity)
}

/// Apply command-table effects from evaluated `[...]` words before the outer
/// statement. The shared inventory is intentionally value-conservative: an
/// incomplete parse or computed command head widens the command domain.
fn apply_embedded_transitions(
    stmt: &Statement,
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
) -> bool {
    let embedded = evaluated_command_substitutions(stmt, registry);
    let observed = embedded.opaque || !embedded.commands.is_empty();
    if embedded.opaque {
        bindings.mark_opaque_binding_mutation();
    }
    for words in embedded.commands {
        let Some(head) = words.first() else {
            continue;
        };
        let Some(head_name) = head.literal() else {
            bindings.mark_opaque_binding_mutation();
            continue;
        };
        let Some(command_namespace) = namespace.for_head(head_name) else {
            bindings.mark_opaque_binding_mutation();
            continue;
        };
        let source_may_be_unknown = bindings.target_may_be_unknown(head_name, command_namespace);
        let facts = bindings.resolve_command_words(&words, registry, command_namespace);
        apply_resolved_may_transitions(
            facts,
            source_may_be_unknown,
            // A recovered invocation has no structured IR body. If it can
            // evaluate Tcl text, that text may change any command binding.
            true,
            bindings,
            namespace,
        );
    }
    observed
}

fn apply_may_invocation_transitions(
    stmt: &Statement,
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    observed: &mut Option<ModuleCommandBindings>,
    command_namespace: &str,
    execution_namespace: &crate::ir_helpers::ExecutionNamespace,
    retained_roots: &mut RetainedBindingRoots,
) -> bool {
    let source_may_be_unknown = match stmt {
        Statement::Call { command, .. } | Statement::Barrier { command, .. } => {
            bindings.target_may_be_unknown(command, command_namespace)
        }
        _ => false,
    };
    let invocations = bindings.resolve_statement(stmt, registry, command_namespace);
    let single_exact_invocation = !source_may_be_unknown && invocations.len() == 1;
    let mut joined: Option<ModuleCommandBindings> = source_may_be_unknown.then(|| bindings.clone());
    let mut exact_definition_key = None;
    for invocation in invocations {
        // A single resolved invocation has no alternative to join. Move the
        // state through that transfer directly so its copy-on-write maps stay
        // uniquely owned; cloning here made a linear run of N exact `proc`
        // declarations copy the growing map N times.
        let mut alternative = if single_exact_invocation {
            std::mem::take(bindings)
        } else {
            bindings.clone()
        };
        let non_binding_before =
            single_exact_invocation.then(|| alternative.non_binding_observation_stamp());
        let procedure_body =
            recover_procedure_definition(&invocation, registry, execution_namespace);
        let precise_procedure = match &procedure_body {
            ProcedureDefinitionReplay::Recovered { qname, module } => {
                let inventory_was_complete = !module.oo_evidence.unretained_executable_roots
                    && std::iter::once(qname)
                        .chain(module.procedures.keys())
                        .all(|name| alternative.procedure_bodies.contains(name));
                if module.oo_evidence.unretained_executable_roots {
                    alternative.mark_opaque_binding_mutation();
                }
                alternative.extend_procedure_bodies(
                    std::iter::once(qname.clone()).chain(module.procedures.keys().cloned()),
                );
                retained_roots.extend_module_roots(module, false);
                if single_exact_invocation && inventory_was_complete {
                    exact_definition_key = exact_procedure_definition_key(
                        &invocation.facts,
                        execution_namespace,
                        qname,
                    );
                }
                Some(qname.as_str())
            }
            ProcedureDefinitionReplay::NotProcedure
            | ProcedureDefinitionReplay::KnownError
            | ProcedureDefinitionReplay::Unavailable => None,
        };
        if !matches!(&procedure_body, ProcedureDefinitionReplay::KnownError) {
            apply_declared_binding_transitions(
                &invocation.facts,
                &mut alternative,
                execution_namespace,
                precise_procedure,
            );
        }
        let body_shape_is_selected = invocation_selects_evaluated_body(&invocation.facts);
        if body_shape_is_selected {
            // An authored Proc-lowering spec may both install a procedure and
            // evaluate a body immediately. That body owns an arbitrary nested
            // binding transfer, so the declared one-key transition no longer
            // proves that the invocation changed only the procedure name.
            // Stock Tcl `proc` carries DEFERS_BODY and stays on the hot path.
            exact_definition_key = None;
        }
        let body_selection_is_indeterminate =
            invocation_may_select_evaluated_body(&invocation.facts);
        let readable_body_was_applied = body_shape_is_selected
            && apply_readable_evaluated_body(
                &invocation,
                registry,
                &mut alternative,
                observed,
                execution_namespace,
                retained_roots,
            );
        if invocation
            .facts
            .traits
            .contains(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
            || (invocation
                .facts
                .traits
                .contains(tcl_registry::Traits::DYNAMIC_EVAL_BODY)
                && ((!body_shape_is_selected && body_selection_is_indeterminate)
                    || (body_shape_is_selected && !readable_body_was_applied)))
        {
            alternative.mark_opaque_binding_mutation();
        }
        if non_binding_before
            .is_some_and(|before| before != alternative.non_binding_observation_stamp())
        {
            // The typed procedure definition also changed another observable
            // axis (for example LOADS_EXTERNAL_UNIT made command bindings
            // opaque). The one-key historical delta is no longer a complete
            // observation, so retain the ordinary full-state join.
            exact_definition_key = None;
        }
        if let Some(state) = &mut joined {
            if !state.same_state(&alternative) {
                state.join(&alternative);
            }
        } else {
            joined = Some(alternative);
        }
    }
    if let Some(state) = joined {
        *bindings = state;
    }
    observe_exact_procedure_definition(observed, bindings, exact_definition_key)
}

/// Publish one exact definition into the historical view without a full-map
/// lattice join. Returns whether the caller's ordinary observation is already
/// complete.
fn observe_exact_procedure_definition(
    observed: &mut Option<ModuleCommandBindings>,
    bindings: &ModuleCommandBindings,
    key: Option<String>,
) -> bool {
    let Some(key) = key else {
        return false;
    };
    if let Some(history) = observed {
        history.join_binding_from(bindings, &key);
        return true;
    }

    // No earlier invocation supplied the pre-transition state. For this first
    // exact definition that state is the registry baseline; seed it explicitly
    // before joining the new procedure target.
    let mut history = bindings.clone();
    let mut historical = ModuleCommandBindings::unmodified_bindings(
        &key,
        history.baseline.semantics.binding_names(),
    );
    historical.extend(bindings.bindings.get(&key).cloned().unwrap_or_else(|| {
        ModuleCommandBindings::unmodified_bindings(&key, history.baseline.semantics.binding_names())
    }));
    history.replace(key, historical);
    *observed = Some(history);
    true
}

/// The exact, registry-declared procedure binding established by one
/// invocation, when that is its sole state transition. This deliberately keys
/// the fast path on typed transition data rather than the spelling `proc`.
fn exact_procedure_definition_key(
    facts: &tcl_registry::InvocationFacts,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    precise_procedure: &str,
) -> Option<String> {
    let transitions = facts.state_transitions.declared()?;
    let [fact] = transitions.facts() else {
        return None;
    };
    let StateTransition::CommandBinding(CommandBindingTransition::Define { name, kind }) =
        &fact.transition
    else {
        return None;
    };
    if *kind != CommandBindingDefinitionKind::Procedure {
        return None;
    }
    let key = qualify_execution_name(namespace, name.literal()?)?;
    (key == precise_procedure).then_some(key)
}

fn apply_resolved_may_transitions(
    invocations: Vec<tcl_registry::InvocationFacts>,
    source_may_be_unknown: bool,
    dynamic_body_is_opaque: bool,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
) {
    let mut joined: Option<ModuleCommandBindings> = source_may_be_unknown.then(|| bindings.clone());
    for facts in invocations {
        let mut alternative = bindings.clone();
        if facts
            .traits
            .contains(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
            || (dynamic_body_is_opaque
                && facts
                    .traits
                    .contains(tcl_registry::Traits::DYNAMIC_EVAL_BODY)
                && (invocation_selects_evaluated_body(&facts)
                    || invocation_may_select_evaluated_body(&facts)))
        {
            alternative.mark_opaque_binding_mutation();
        }
        apply_declared_binding_transitions(&facts, &mut alternative, namespace, None);
        if let Some(state) = &mut joined {
            if !state.same_state(&alternative) {
                state.join(&alternative);
            }
        } else {
            joined = Some(alternative);
        }
    }
    if let Some(state) = joined {
        *bindings = state;
    }
}

/// Whether the resolved invocation selects an argument that Tcl evaluates as
/// script. Some ensemble roots carry `DYNAMIC_EVAL_BODY` for only selected
/// subcommands, so the root trait alone cannot make a precise leaf opaque.
fn invocation_selects_evaluated_body(facts: &tcl_registry::InvocationFacts) -> bool {
    use tcl_registry::SemanticOperationId;
    use tcl_registry::frame_effect::FrameArgLayout;
    use tcl_registry::hooks::LoweringHookId;

    if facts.traits.contains(tcl_registry::Traits::DEFERS_BODY) {
        return false;
    }

    matches!(
        facts.operation,
        SemanticOperationId::StructuredLowering(
            LoweringHookId::Apply | LoweringHookId::NamespaceEval
        )
    ) || facts
        .arg_roles
        .iter()
        .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
        || facts.frame_effect.is_some_and(|effect| {
            matches!(
                effect.layout,
                FrameArgLayout::ScriptInCurrentFrame | FrameArgLayout::ScriptInSelectedFrame
            )
        })
}

/// Whether a resolved registry head still has an invocation-time choice of a
/// body-bearing leaf. The root's broad dynamic-evaluation trait is meaningful
/// for an indeterminate ensemble subcommand, but must not taint an exact
/// non-body or deferred leaf.
fn invocation_may_select_evaluated_body(facts: &tcl_registry::InvocationFacts) -> bool {
    matches!(
        &facts.subcommand,
        tcl_registry::OwnedSubcommandResolution::Indeterminate { .. }
    ) || (!facts.arg_roles_complete && !facts.traits.contains(tcl_registry::Traits::DEFERS_BODY))
}

fn apply_declared_binding_transitions(
    facts: &tcl_registry::InvocationFacts,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    precise_procedure: Option<&str>,
) {
    let Some(transitions) = facts.state_transitions.declared() else {
        return;
    };
    if matches!(
        transitions.command_resolution_impact(),
        tcl_registry::CommandResolutionImpact::Unbounded
    ) {
        bindings.mark_opaque_resolution();
    }
    for fact in transitions.facts() {
        if let StateTransition::Namespace(transition) = &fact.transition {
            bindings.namespace_resolution.record(transition, namespace);
        }
    }
    if transitions.facts().iter().any(|fact| {
        matches!(
            &fact.transition,
            StateTransition::Widen(widening)
                if widening.domains.contains(&StateTransitionDomain::CommandBindings)
        )
    }) {
        bindings.mark_dynamic_proc_binding();
    }
    for transition in transitions.command_bindings() {
        apply_may_binding_transition(bindings, transition, namespace, precise_procedure);
    }
}

enum ProcedureDefinitionReplay {
    NotProcedure,
    /// Tcl rejects the invocation before installing a command binding.
    KnownError,
    /// A procedure may be installed, but its runtime body is not source-safe.
    Unavailable,
    Recovered {
        qname: String,
        module: Box<Module>,
    },
}

/// Recover the body installed by the registry's typed procedure-definition
/// operation, including arguments prepended by an alias chain. This is the
/// only path that authorises a precise non-registry command target: a bare
/// `Define(Procedure)` fact is intentionally insufficient because consumers
/// could otherwise dispatch to a procedure whose effects were never walked.
fn recover_procedure_definition(
    invocation: &ResolvedBindingInvocation,
    registry: &CommandRegistry,
    namespace: &crate::ir_helpers::ExecutionNamespace,
) -> ProcedureDefinitionReplay {
    use tcl_registry::SemanticOperationId;
    use tcl_registry::hooks::LoweringHookId;

    if invocation.facts.operation != SemanticOperationId::StructuredLowering(LoweringHookId::Proc) {
        return ProcedureDefinitionReplay::NotProcedure;
    }
    let Some(argument_count) = invocation.exact_argument_count else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    if argument_count != 3 {
        return ProcedureDefinitionReplay::KnownError;
    }
    if !invocation.facts.arg_roles_complete {
        return ProcedureDefinitionReplay::Unavailable;
    }
    let role_index = |role| {
        invocation
            .facts
            .arg_roles
            .iter()
            .find_map(|(index, found)| (*found == role).then_some(usize::from(*index)))
    };
    let (Some(params_index), Some(body_index)) = (
        role_index(tcl_registry::ArgRole::ParamList),
        role_index(tcl_registry::ArgRole::Body),
    ) else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    let Some(name) = invocation
        .facts
        .state_transitions
        .declared()
        .and_then(|transitions| {
            transitions
                .command_bindings()
                .find_map(|transition| match transition {
                    CommandBindingTransition::Define { name, kind }
                        if *kind == CommandBindingDefinitionKind::Procedure =>
                    {
                        name.literal()
                    }
                    _ => None,
                })
        })
    else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    let (Some(params), Some(body)) = (
        invocation
            .literal_arguments
            .get(params_index)
            .and_then(Option::as_deref),
        invocation
            .literal_arguments
            .get(body_index)
            .and_then(Option::as_deref),
    ) else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    if tcl_syntax::formal_params::parse_formal_parameters(params).is_err() {
        return ProcedureDefinitionReplay::KnownError;
    }

    let Some(qname) = qualify_execution_name(namespace, name) else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    let module = lower_recovered_procedure_body(body, &qname, registry);
    ProcedureDefinitionReplay::Recovered {
        qname,
        module: Box::new(module),
    }
}

/// Apply command-binding transitions from a statically readable script body
/// evaluated by a runtime barrier. Structural commands whose body location or
/// namespace cannot be expressed by the generic frame-effect grammar are
/// selected by their registry-owned semantic operation, never by spelling.
// Registry frame/body layouts require one ordered state transition.
#[allow(clippy::too_many_lines)]
fn apply_readable_evaluated_body(
    invocation: &ResolvedBindingInvocation,
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    observed: &mut Option<ModuleCommandBindings>,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    retained_roots: &mut RetainedBindingRoots,
) -> bool {
    use tcl_registry::SemanticOperationId;
    use tcl_registry::frame_effect::{FrameArgLayout, FrameLevel};
    use tcl_registry::hooks::LoweringHookId;

    let arg_refs: Vec<&str> = invocation.arguments.iter().map(String::as_str).collect();
    let body_indices =
        registry.arg_indices_for_role(&invocation.command, &arg_refs, tcl_registry::ArgRole::Body);
    let body_interpreter = invocation.facts.body_interpreter.resolve_with(|index| {
        invocation
            .literal_arguments
            .get(index)
            .and_then(Option::as_deref)
    });
    let (sources, body_namespace, procedure_target) = match invocation.facts.operation {
        SemanticOperationId::StructuredLowering(LoweringHookId::Apply) => {
            let Some(argument_count) = invocation.exact_argument_count else {
                return false;
            };
            let Some(lambda) = invocation
                .literal_arguments
                .first()
                .and_then(Option::as_deref)
            else {
                return false;
            };
            let Ok(elements) = tcl_syntax::list::split_list(lambda) else {
                // Tcl rejects a malformed lambda before entering its body.
                return true;
            };
            if !(2..=3).contains(&elements.len()) {
                return true;
            }
            let Ok(parameters) =
                tcl_syntax::formal_params::parse_formal_parameters(elements[0].as_ref())
            else {
                return true;
            };
            let variadic = tcl_syntax::formal_params::has_trailing_args(&parameters);
            let fixed_parameters = if variadic {
                &parameters[..parameters.len().saturating_sub(1)]
            } else {
                parameters.as_slice()
            };
            let minimum_arguments = fixed_parameters
                .iter()
                .rposition(|parameter| parameter.default.is_none())
                .map_or(0, |index| index + 1);
            let supplied_arguments = argument_count.saturating_sub(1);
            if supplied_arguments < minimum_arguments
                || (!variadic && supplied_arguments > fixed_parameters.len())
            {
                // Known arity errors occur before any body command executes.
                return true;
            }
            let body_namespace = match elements.get(2) {
                Some(name) if name.is_empty() => Some("::".to_owned()),
                Some(name) => qualified_namespace("::", name),
                None => Some("::".to_owned()),
            };
            let Some(body_namespace) = body_namespace else {
                return true;
            };
            (
                vec![elements[1].to_string()],
                Some(crate::ir::ExecutionNamespace::exact(body_namespace)),
                true,
            )
        }
        SemanticOperationId::StructuredLowering(LoweringHookId::NamespaceEval) => {
            let Some(argument_count) = invocation.exact_argument_count else {
                return false;
            };
            let Some(&first_body) = body_indices.first() else {
                return false;
            };
            let Some(target_index) = first_body.checked_sub(1) else {
                return false;
            };
            let Some(target) = invocation
                .literal_arguments
                .get(target_index)
                .and_then(Option::as_deref)
            else {
                return false;
            };
            let Some(source) =
                readable_script_argument(invocation, first_body, registry, bindings, namespace)
            else {
                return false;
            };
            if first_body + 1 != argument_count {
                return false;
            }
            let Some(body_namespace) = qualify_execution_name(namespace, target) else {
                // Tcl rejects an empty namespace before evaluating its body.
                return target.is_empty();
            };
            (
                vec![source],
                Some(crate::ir::ExecutionNamespace::exact(body_namespace)),
                false,
            )
        }
        _ => {
            let Some(argument_count) = invocation.exact_argument_count else {
                return false;
            };
            let (first_body, body_namespace) =
                match (invocation.facts.frame_effect, &body_interpreter) {
                    (Some(effect), _) => match effect.layout {
                        FrameArgLayout::ScriptInCurrentFrame => (0, Some(namespace.clone())),
                        FrameArgLayout::ScriptInSelectedFrame => {
                            let (level, body) =
                                effect.resolve_for_version(&arg_refs, registry.runtime_version());
                            let body_namespace = match level {
                                FrameLevel::Absolute(0) => {
                                    Some(crate::ir::ExecutionNamespace::exact("::"))
                                }
                                level if level.is_current_frame() => Some(namespace.clone()),
                                // The caller's defining namespace is unavailable in a
                                // per-procedure root. We can still prove a constructed
                                // script harmless to the command table below; any
                                // actual binding transition remains opaque.
                                _ => None,
                            };
                            (arg_refs.len().saturating_sub(body.len()), body_namespace)
                        }
                        FrameArgLayout::AliasPairs | FrameArgLayout::OpaqueCallerVars => {
                            return false;
                        }
                    },
                    // A body executed by a named child interpreter cannot directly
                    // update this module's command table. Still prove the readable
                    // script free of command-binding mutations before preserving
                    // the parent lattice: that remains safe if a child alias later
                    // re-enters a command in the parent interpreter.
                    (None, tcl_registry::InterpreterScope::Named(_)) => {
                        let Some(&first_body) = body_indices.first() else {
                            return false;
                        };
                        (first_body, None)
                    }
                    (
                        None,
                        tcl_registry::InterpreterScope::Current
                        | tcl_registry::InterpreterScope::Any,
                    ) => {
                        return false;
                    }
                };
            if first_body >= argument_count {
                return false;
            }
            let indices: Vec<usize> = if invocation
                .facts
                .traits
                .contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
            {
                // Multi-word concat is exact at runtime but reconstructing its
                // Tcl quoting here would create a second concat interpreter.
                if first_body + 1 != argument_count {
                    return false;
                }
                vec![first_body]
            } else if body_indices.is_empty() {
                (first_body..argument_count).collect()
            } else {
                body_indices
            };
            let Some(sources) = indices
                .into_iter()
                .map(|index| {
                    readable_script_argument(invocation, index, registry, bindings, namespace)
                })
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };
            (sources, body_namespace, false)
        }
    };

    let Some(body_namespace) = body_namespace else {
        return sources
            .iter()
            .all(|source| command_bindings_unchanged_by_script(source, registry, bindings));
    };

    let crate::ir::ExecutionNamespace::Exact(body_namespace) = body_namespace else {
        return sources
            .iter()
            .all(|source| command_bindings_unchanged_by_script(source, registry, bindings));
    };

    for source in sources {
        let execution_namespace = crate::ir::ExecutionNamespace::exact(&body_namespace);
        if let Some(script) =
            retained_roots.evaluated_body(&source, &execution_namespace, invocation.source_span)
        {
            let outcome = collect_binding_states(
                &script,
                registry,
                bindings,
                &execution_namespace,
                retained_roots,
            );
            observe_binding_state(observed, &outcome.observed);
            *bindings = outcome.post;
            continue;
        }
        let config = registry
            .profile()
            .map_or_else(tcl_lexer::LexerConfig::default, |profile| {
                tcl_lexer::LexerConfig::from_grammar(profile.grammar)
            });
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_dialect(registry.profile());
        let unrooted_namespace = body_namespace.strip_prefix("::").unwrap_or(&body_namespace);
        if procedure_target {
            lowerer.lower_procedure_target(&source, unrooted_namespace);
        } else {
            lowerer.lower_script_target(&source, unrooted_namespace);
        }
        let module = lowerer.finish_module(&source);
        // Evaluating a script executes only its top-level root. Procedure and
        // method bodies defined by that script become possible future roots;
        // executing them here would invent command-table mutations at
        // definition time and poison later exact binding resolution.
        bindings.extend_procedure_bodies(module.procedures.keys().cloned());
        if module.oo_evidence.unretained_executable_roots {
            bindings.mark_opaque_binding_mutation();
        }
        retained_roots.extend_module_roots(&module, true);
        let script_namespace = if module.top_level_namespace.is_empty() {
            "::"
        } else {
            &module.top_level_namespace
        };
        let execution_namespace = crate::ir::ExecutionNamespace::exact(script_namespace);
        let outcome = collect_binding_states(
            &module.top_level,
            registry,
            bindings,
            &execution_namespace,
            retained_roots,
        );
        observe_binding_state(observed, &outcome.observed);
        *bindings = outcome.post;
    }
    true
}

/// Prove that a source-safe script cannot alter the closed command-binding
/// state, even when a frame shift leaves its execution namespace unknown.
///
/// Every namespace already represented by a procedure or command binding is
/// checked, plus the global namespace. A fresh sentinel namespace covers
/// transitions whose result depends only on qualification (for example, a
/// definition in a previously unseen caller namespace). The ordinary
/// transition walker remains the semantic owner: this helper does not name or
/// reinterpret any command.
fn command_bindings_unchanged_by_script(
    source: &str,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
) -> bool {
    let config = registry
        .profile()
        .map_or_else(tcl_lexer::LexerConfig::default, |profile| {
            tcl_lexer::LexerConfig::from_grammar(profile.grammar)
        });
    let mut namespaces = BTreeSet::from([
        "::".to_owned(),
        "::__tcl_compiler_unknown_caller".to_owned(),
    ]);
    for qname in bindings
        .bindings
        .keys()
        .chain(bindings.procedure_bodies.iter())
    {
        let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
        namespaces.insert(if holder.is_empty() {
            "::".to_owned()
        } else {
            holder.to_owned()
        });
    }
    namespaces.into_iter().all(|namespace| {
        // Typed-lowering dependencies are namespace-bearing provenance. Re-
        // lower for each candidate instead of stamping the sentinel namespace
        // onto a script subsequently interpreted elsewhere.
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_dialect(registry.profile());
        let unrooted = namespace.strip_prefix("::").unwrap_or(&namespace);
        lowerer.lower_script_target(source, unrooted);
        let module = lowerer.finish_module(source);
        let mut retained_roots = RetainedBindingRoots::default();
        let execution_namespace = crate::ir::ExecutionNamespace::exact(&namespace);
        let outcome = collect_binding_states(
            &module.top_level,
            registry,
            bindings,
            &execution_namespace,
            &mut retained_roots,
        );
        outcome.post.same_state(bindings) && outcome.observed.same_state(bindings)
    })
}

fn readable_script_argument(
    invocation: &ResolvedBindingInvocation,
    index: usize,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
) -> Option<String> {
    invocation
        .literal_arguments
        .get(index)
        .and_then(Option::as_deref)
        .map(str::to_owned)
        .or_else(|| {
            let source = invocation.arguments.get(index)?;
            let crate::ir_helpers::ExecutionNamespace::Exact(namespace) = namespace else {
                return None;
            };
            constructed_script_words(source, registry, bindings, namespace)
                .map(tcl_syntax::list::join_list)
        })
}

fn qualify_execution_name(
    namespace: &crate::ir_helpers::ExecutionNamespace,
    name: &str,
) -> Option<String> {
    if name.is_empty() {
        return None;
    }
    let canonical = nqn(name);
    if name.starts_with("::") {
        return Some(canonical);
    }
    let crate::ir_helpers::ExecutionNamespace::Exact(namespace) = namespace else {
        return None;
    };
    Some(tcl_syntax::naming::qualify(
        namespace,
        canonical.strip_prefix("::").unwrap_or(&canonical),
    ))
}

fn qualified_namespace(parent: &str, child: &str) -> Option<String> {
    if child.is_empty() {
        return None;
    }
    // A namespace name does not have the empty-command-tail meaning of a
    // trailing separator. Normalise the written namespace through the shared
    // syntax owner before joining its relative suffix to the constructed key.
    let canonical = nqn(child);
    if child.starts_with("::") {
        Some(canonical)
    } else {
        Some(tcl_syntax::naming::qualify(
            parent,
            canonical.strip_prefix("::").unwrap_or(&canonical),
        ))
    }
}

/// Project the exact command words returned by one source-proven list
/// constructor invocation.
///
/// Recognition is registry-owned through [`tcl_registry::ReturnElements`],
/// while [`ModuleCommandBindings`] proves that the effective constructor head
/// still reaches only registry-backed implementations. Every source operand
/// must have one literal value under the active lexer grammar: substitution,
/// expansion, recovery, or a rebound constructor fails closed so the caller
/// can widen the affected frame.
pub(crate) fn constructed_script_words(
    word: &str,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &str,
) -> Option<Vec<String>> {
    let inner = word
        .strip_prefix('[')
        .and_then(|word| word.strip_suffix(']'))?;
    let config = registry
        .profile()
        .map_or_else(tcl_lexer::LexerConfig::default, |profile| {
            tcl_lexer::LexerConfig::from_grammar(profile.grammar)
        });
    let commands = crate::segmenter::segment_commands_with_offset_and_config(inner, 0, config);
    let [command] = commands.as_slice() else {
        return None;
    };
    if command.is_partial {
        return None;
    }
    let source_map = tcl_lexer::SourceMap::new(inner);
    let tokens = crate::ir::CommandTokens::from_segmented(&source_map, config, command);
    let words: Vec<String> = tokens
        .words()
        .iter()
        .map(|word| {
            match crate::registry_invocation::effective_invocation_word(word, config.escapes) {
                crate::registry_invocation::EffectiveInvocationWord::Literal(value) => Some(value),
                crate::registry_invocation::EffectiveInvocationWord::Dynamic
                | crate::registry_invocation::EffectiveInvocationWord::Expanded
                | crate::registry_invocation::EffectiveInvocationWord::Opaque => None,
            }
        })
        .collect::<Option<_>>()?;
    let (builder, operands) = words.split_first()?;
    // A dynamic mutation may have replaced the constructor even when its
    // literal spelling has no individually enumerable transition. Requiring
    // whole-module provenance keeps runtime replay and static effect summaries
    // on the same conservative rule.
    if bindings.target_may_be_unknown(builder, namespace) {
        return None;
    }

    let mut projections = BTreeSet::new();
    let targets = bindings.targets(builder, namespace);
    if targets.is_empty() {
        return None;
    }
    for target in targets {
        if !target.registry_backed {
            return None;
        }
        let mut arguments = target.prepended;
        arguments.extend(operands.iter().cloned());
        let argument_refs: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let invocation = registry.resolve_invocation(
            &target.command,
            &argument_refs,
            registry.own_surface_query(),
        )?;
        let Some(tcl_registry::ReturnElements::ListOfArgs { from }) =
            invocation.semantics.return_elements
        else {
            return None;
        };
        let projected = arguments.get(usize::from(from)..)?.to_vec();
        if projected.first().is_none_or(String::is_empty) {
            return None;
        }
        projections.insert(projected);
    }
    let mut projections = projections.into_iter();
    let unique = projections.next()?;
    projections.next().is_none().then_some(unique)
}

// Exhaustive interpreter of registry command-binding transitions.
#[allow(clippy::too_many_lines)]
fn apply_may_binding_transition(
    bindings: &mut ModuleCommandBindings,
    transition: &CommandBindingTransition,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    precise_procedure: Option<&str>,
) {
    match transition {
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
        } => {
            let Some(source_interpreter) = source_interpreter.literal() else {
                bindings.mark_dynamic_proc_binding();
                return;
            };
            if !matches!(source_interpreter, "" | "{}") {
                return;
            }
            let Some(alias) = alias.literal() else {
                bindings.mark_dynamic_proc_binding();
                return;
            };
            bindings.record_proc_rebound_candidates(alias, namespace);
            let key = nqn(alias);
            bindings.rebound_names.insert(key.clone());
            let binding = if is_current_interpreter(target_interpreter) {
                target.literal().and_then(|target| {
                    arguments
                        .iter()
                        .map(TransitionSubject::literal)
                        .map(|argument| argument.map(str::to_owned))
                        .collect::<Option<Vec<_>>>()
                        .map(|prepended| {
                            MayBinding::Target(ResolvedCommandTarget {
                                command: target.to_owned(),
                                prepended,
                                registry_backed: true,
                                terminal: false,
                            })
                        })
                })
            } else {
                None
            };
            bindings.replace(
                key,
                BTreeSet::from([binding.unwrap_or(MayBinding::Unknown)]),
            );
        }
        CommandBindingTransition::Move { from, to } => {
            let (Some(from), Some(to)) = (from.literal(), to.literal()) else {
                bindings.mark_dynamic_proc_binding();
                return;
            };
            bindings.record_proc_rebound_candidates(from, namespace);
            if !to.is_empty() {
                bindings.record_proc_rebound_candidates(to, namespace);
            }
            let Some(from_namespace) = namespace.for_head(from) else {
                bindings.mark_dynamic_proc_binding();
                return;
            };
            let Some(to_key) = qualify_execution_name(namespace, to) else {
                bindings.mark_dynamic_proc_binding();
                return;
            };
            let from_keys = bindings.source_keys(from, from_namespace);
            if from_keys.len() != 1 {
                bindings.mark_dynamic_proc_binding();
            }
            let mut moved = BTreeSet::new();
            for from_key in from_keys {
                bindings.rebound_names.insert(from_key.clone());
                moved.extend(
                    bindings
                        .bindings
                        .get(&from_key)
                        .cloned()
                        .unwrap_or_else(|| {
                            ModuleCommandBindings::unmodified_bindings(
                                &from_key,
                                bindings.baseline.semantics.binding_names(),
                            )
                        }),
                );
                bindings.remove(from_key);
            }
            bindings.rebound_names.insert(to_key.clone());
            bindings.replace(to_key, moved);
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            let affects_current = match interpreter.as_ref().and_then(TransitionSubject::literal) {
                None if interpreter.is_some() => {
                    bindings.mark_dynamic_proc_binding();
                    return;
                }
                None | Some("" | "{}") => true,
                Some(_) => false,
            };
            if !affects_current {
                return;
            }
            let Some(name) = name.literal() else {
                bindings.mark_dynamic_proc_binding();
                return;
            };
            bindings.record_proc_rebound_candidates(name, namespace);
            // `interp alias {} NAME {}` names NAME in the source
            // interpreter's global namespace; bare `rename NAME {}` uses the
            // current command namespace.
            let deletion_namespace = if interpreter.is_some() {
                Some("::")
            } else {
                namespace.for_head(name)
            };
            let Some(deletion_namespace) = deletion_namespace else {
                bindings.mark_dynamic_proc_binding();
                return;
            };
            let keys = bindings.source_keys(name, deletion_namespace);
            if keys.len() != 1 {
                bindings.mark_dynamic_proc_binding();
            }
            for key in keys {
                bindings.rebound_names.insert(key.clone());
                bindings.remove(key);
            }
        }
        CommandBindingTransition::Define { name, kind } => {
            if let Some(name) = name.literal() {
                let Some(key) = qualify_execution_name(namespace, name) else {
                    bindings.mark_opaque_binding_mutation();
                    return;
                };
                let binding = if *kind == CommandBindingDefinitionKind::Procedure {
                    if precise_procedure == Some(key.as_str())
                        && bindings.procedure_bodies.contains(&key)
                    {
                        MayBinding::Target(ResolvedCommandTarget {
                            command: key.clone(),
                            prepended: Vec::new(),
                            registry_backed: false,
                            terminal: true,
                        })
                    } else {
                        bindings.mark_opaque_binding_mutation();
                        MayBinding::Unknown
                    }
                } else {
                    MayBinding::Unknown
                };
                bindings.replace(key, BTreeSet::from([binding]));
            } else {
                bindings.mark_opaque_binding_mutation();
            }
        }
        CommandBindingTransition::Unknown { .. } => bindings.mark_dynamic_proc_binding(),
    }
}

impl Binding {
    const fn of(kind: BindingKind) -> Self {
        Self { kind, target: None }
    }

    /// True when the name still denotes its original core builtin.
    #[must_use]
    pub fn is_original_builtin(&self) -> bool {
        self.kind == BindingKind::Builtin
    }

    /// True when the name denotes a concrete, foldable user proc.
    #[must_use]
    pub fn is_foldable_proc(&self) -> bool {
        self.kind == BindingKind::Proc && self.target.is_some()
    }
}

/// A sparse per-name binding map.  An absent name takes its *default*
/// binding (a pure function of the name — builtin if the registry knows
/// the bare global name, else opaque).  `wildcard` marks "every name is
/// ⊤ from here" after a dynamic mutation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct State {
    map: HashMap<String, Binding>,
    wildcard: bool,
}

/// The unperturbed binding of `qname` before any rename/proc/alias.
///
/// Only an unqualified global name the registry knows is a `Builtin`
/// (`::string` → `string`); a namespaced tail (`::ns::foo`) or an
/// unknown name is `Opaque`.
fn default_binding(qname: &str, registry: &CommandRegistry) -> Binding {
    let bare = qname.strip_prefix("::").unwrap_or(qname);
    if !bare.contains("::") && registry.get(bare).is_some() {
        Binding::of(BindingKind::Builtin)
    } else {
        Binding::of(BindingKind::Opaque)
    }
}

/// Resolve `qname`'s binding within `state`, honouring wildcard + default.
fn binding_in(state: &State, qname: &str, registry: &CommandRegistry) -> Binding {
    if state.wildcard {
        return Binding::of(BindingKind::Unknown);
    }
    state
        .map
        .get(qname)
        .cloned()
        .unwrap_or_else(|| default_binding(qname, registry))
}

/// Lattice join: ⊥ is identity, equal stays, anything else rises to ⊤.
fn join_binding(a: &Binding, b: &Binding) -> Binding {
    if a.kind == BindingKind::Bottom {
        return b.clone();
    }
    if b.kind == BindingKind::Bottom {
        return a.clone();
    }
    if a == b {
        return a.clone();
    }
    Binding::of(BindingKind::Unknown)
}

/// Record a namespace transition's effect on command *resolution*.
///
/// Returns `false` when the target namespace is not literal, which the caller
/// takes as the lattice top: the transition could have landed anywhere.
///
/// An import binds each pattern's *tail* in the target namespace, so a
/// literal, glob-free pattern is exact — `namespace import -force
/// ::evil::answer` shadowing a declared `::ns::answer`. A glob (`::lib::*`)
/// names commands this scan cannot enumerate, so only the *target* namespace
/// becomes opaque, and a name elsewhere — including in the namespace imported
/// *from* — stays trustworthy.
fn collect_namespace_resolution(
    transition: &NamespaceTransition,
    site: Option<&str>,
    rebound: &mut std::collections::HashSet<String>,
    resolution: &mut bool,
    opaque: &mut std::collections::HashSet<String>,
) -> bool {
    let (namespace, patterns) = match transition {
        NamespaceTransition::Import {
            namespace,
            patterns,
        }
        | NamespaceTransition::Forget {
            namespace,
            patterns,
        } => (namespace, Some(patterns)),
        NamespaceTransition::SetPath { namespace, .. }
        | NamespaceTransition::SetUnknown { namespace, .. }
        | NamespaceTransition::Ensemble { namespace }
        | NamespaceTransition::Delete { namespace } => (namespace, None),
        // Creating a namespace, or changing its export list, resolves nothing
        // differently for a caller in it.
        NamespaceTransition::Ensure { .. } | NamespaceTransition::Export { .. } => return true,
    };
    *resolution = true;
    let Some(target) = transition_namespace(namespace, site) else {
        return false;
    };
    let Some(patterns) = patterns else {
        opaque.insert(target);
        return true;
    };
    for pattern in patterns {
        match literal_subject(pattern) {
            Some(text) if !text.contains(['*', '?', '[']) => {
                let tail = text.rsplit("::").next().unwrap_or(text);
                rebound.insert(nqn(&join_ns(&target, tail)));
            }
            _ => {
                opaque.insert(target.clone());
            }
        }
    }
    true
}

/// The namespace a [`NamespaceTransitionTarget`] denotes: the invocation's own
/// namespace for `Current`, the literal text for a `Named` operand, and `None`
/// when the operand is not literal (the caller then takes the lattice top).
fn transition_namespace(
    target: &tcl_registry::NamespaceTransitionTarget,
    current: Option<&str>,
) -> Option<String> {
    match target {
        tcl_registry::NamespaceTransitionTarget::Current => current.map(nqn),
        tcl_registry::NamespaceTransitionTarget::Named(subject) => {
            literal_subject(subject).map(nqn)
        }
    }
}

/// `ns` + `tail` as one qualified name, without doubling the separator.
fn join_ns(ns: &str, tail: &str) -> String {
    if ns == "::" {
        format!("::{tail}")
    } else {
        format!("{ns}::{tail}")
    }
}

fn literal_subject(subject: &TransitionSubject) -> Option<&str> {
    match subject {
        TransitionSubject::Literal(value) => Some(value),
        TransitionSubject::Unknown { .. } => None,
    }
}

fn definition_binding(kind: CommandBindingDefinitionKind, qname: String) -> Binding {
    let kind = match kind {
        CommandBindingDefinitionKind::Command => BindingKind::Command,
        CommandBindingDefinitionKind::Procedure => BindingKind::Proc,
        CommandBindingDefinitionKind::Object => BindingKind::Class,
    };
    Binding {
        kind,
        target: Some(qname),
    }
}

fn apply_binding_transition(
    transition: &CommandBindingTransition,
    state: &mut State,
    registry: &CommandRegistry,
) {
    match transition {
        CommandBindingTransition::Define { name, kind } => {
            let Some(name) = literal_subject(name) else {
                state.wildcard = true;
                return;
            };
            if name.is_empty() {
                return;
            }
            let qname = nqn(name);
            state
                .map
                .insert(qname.clone(), definition_binding(*kind, qname));
        }
        CommandBindingTransition::Move { from, to } => {
            let (Some(from), Some(to)) = (literal_subject(from), literal_subject(to)) else {
                state.wildcard = true;
                return;
            };
            let from = nqn(from);
            let moved = binding_in(state, &from, registry);
            state.map.insert(from, Binding::of(BindingKind::Opaque));
            if !to.is_empty() {
                state.map.insert(nqn(to), moved);
            }
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            let affects_current = match interpreter {
                None => true,
                Some(subject) => match literal_subject(subject) {
                    Some("") => true,
                    Some(_) => false,
                    None => {
                        state.wildcard = true;
                        return;
                    }
                },
            };
            if !affects_current {
                return;
            }
            let Some(name) = literal_subject(name) else {
                state.wildcard = true;
                return;
            };
            state
                .map
                .insert(nqn(name), Binding::of(BindingKind::Opaque));
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter: _,
            target,
            arguments: _,
        } => {
            let Some(source_interpreter) = literal_subject(source_interpreter) else {
                state.wildcard = true;
                return;
            };
            if !source_interpreter.is_empty() {
                return;
            }
            let (Some(alias), Some(target)) = (literal_subject(alias), literal_subject(target))
            else {
                state.wildcard = true;
                return;
            };
            state.map.insert(
                nqn(alias),
                Binding {
                    kind: BindingKind::Alias,
                    target: Some(nqn(target)),
                },
            );
        }
        CommandBindingTransition::Unknown { .. } => state.wildcard = true,
    }
}

/// Apply the registry's closed transition description.
///
/// `None` means the invocation was unresolved or deliberately unstamped.
/// `Some(false)` means the selected descriptor was closed but did not change
/// command bindings.  That distinction is load-bearing for a registry spelling
/// shadowed by a live object command: an explicitly transition-free static
/// descriptor must not suppress the registry-described runtime receiver path.
fn apply_registry_transitions(
    stmt: &Statement,
    state: &mut State,
    registry: &CommandRegistry,
) -> Option<bool> {
    let facts = invocation_facts(stmt, registry)?;
    let transitions = facts.state_transitions.declared()?;
    let mut applied = false;
    for fact in transitions.facts() {
        match &fact.transition {
            StateTransition::CommandBinding(transition) => {
                apply_binding_transition(transition, state, registry);
                applied = true;
            }
            StateTransition::Widen(widening)
                if widening
                    .domains
                    .contains(&StateTransitionDomain::CommandBindings) =>
            {
                state.wildcard = true;
                return Some(true);
            }
            StateTransition::Interpreter(_)
            | StateTransition::VariableCellAlias(_)
            | StateTransition::Namespace(_)
            | StateTransition::Trace(_)
            | StateTransition::ObjectDispatch(_)
            | StateTransition::Widen(_) => {}
        }
    }
    Some(applied)
}

/// Apply `stmt`'s command-table mutation to `state` in place.
///
/// Registry-declared [`StateTransition`] facts are authoritative for ordinary
/// definitions, renames, deletions, and aliases.  Unstamped legacy mutators
/// widen conservatively instead of being re-decoded here.  Runtime object
/// receiver calls remain a small separate path because their source head is a
/// value, not a statically registered command.
fn stmt_gen(stmt: &Statement, state: &mut State, registry: &CommandRegistry) {
    let (Statement::Call { args, .. } | Statement::Barrier { args, .. }) = stmt else {
        return;
    };
    if state.wildcard {
        return; // already maximally conservative
    }
    if apply_registry_transitions(stmt, state, registry) == Some(true) {
        return;
    }

    // The canonical command falls back to the source spelling.
    let cmd = stmt.canonical_command_or_source();

    // Class lifecycle.  A registry-described definer creates a command;
    // registry-declared object-surface methods can delete it or manufacture
    // another named object command.
    if let Some(created) = definer_created_command(registry, cmd, args) {
        state.map.insert(
            nqn(&created),
            Binding {
                kind: BindingKind::Class,
                target: Some(nqn(&created)),
            },
        );
        return;
    }
    let head = nqn(cmd);
    let head_binding = binding_in(state, &head, registry);
    if head_binding.kind == BindingKind::Class {
        if args
            .first()
            .is_some_and(|w| registry.is_destructive_object_method(w))
        {
            state.map.insert(head, Binding::of(BindingKind::Opaque));
        } else if let Some(name_idx) = args.first().and_then(|method| {
            registry
                .is_manufacturer_method(method)
                .then(|| registry.uniform_manufacturer_names_instance_at(method))
                .flatten()
        }) && let Some(name) = args.get(name_idx)
            && !is_dynamic_word(name)
        {
            state.map.insert(
                nqn(name),
                Binding {
                    kind: BindingKind::Class,
                    target: head_binding.target,
                },
            );
        }
    }
}

/// The command a registry-described class definer creates, when this call
/// is a creation: `METACLASS create NAME …` / `METACLASS createWithNamespace
/// NAME …` for the `TclOo` family (gated on `IS_OO_METACLASS`, mirroring the
/// analyser's dual gate), `DEFINER NAME BODY` for snit/itcl.  `None` for
/// non-definers, `new` (auto-named), or a dynamic name.
fn definer_created_command(
    registry: &CommandRegistry,
    cmd: &str,
    args: &[String],
) -> Option<String> {
    let spec = registry.get(cmd)?;
    let grammar = spec.definition_body?;
    if !grammar.family.manufactures_runtime_commands() {
        return None;
    }
    let name = match grammar.family {
        tcl_registry::definer::DefinerFamily::TclOo => {
            if !spec.traits.contains(tcl_registry::Traits::IS_OO_METACLASS) {
                return None;
            }
            let method = registry.exported_manufacturer_method(cmd, args.first()?)?;
            args.get(usize::from(method.names_instance_at?))?
        }
        tcl_registry::definer::DefinerFamily::Snit | tcl_registry::definer::DefinerFamily::Itcl => {
            args.first()?
        }
        tcl_registry::definer::DefinerFamily::SpecTcl
        | tcl_registry::definer::DefinerFamily::SslicTcl => return None,
    };
    (!name.is_empty() && !is_dynamic_word(name)).then(|| name.clone())
}

/// Join predecessor exit states into a block-entry state.
///
/// A name absent from a finished predecessor exit takes its **default**
/// binding, whereas a name not yet contributed to the accumulator is
/// **⊥** (identity for join) — so the merge is per-name across all
/// predecessors at once, seeded at ⊥.  One wildcard predecessor forces
/// the whole merge to wildcard.
fn merge_preds(pred_exits: &[&State], registry: &CommandRegistry) -> State {
    if pred_exits.is_empty() {
        return State::default();
    }
    if pred_exits.iter().any(|pe| pe.wildcard) {
        return State {
            map: HashMap::new(),
            wildcard: true,
        };
    }
    let mut relevant: HashSet<&String> = HashSet::new();
    for pe in pred_exits {
        relevant.extend(pe.map.keys());
    }
    let mut entry = State::default();
    for name in relevant {
        let mut acc = Binding::of(BindingKind::Bottom);
        for pe in pred_exits {
            let b = pe
                .map
                .get(name)
                .cloned()
                .unwrap_or_else(|| default_binding(name, registry));
            acc = join_binding(&acc, &b);
        }
        if acc != default_binding(name, registry) {
            entry.map.insert(name.clone(), acc);
        }
    }
    entry
}

/// Result of the command-binding analysis for one function/script.
///
/// `block_entry` holds the lattice state at each block's entry;
/// point-wise queries replay the gen of the statements before the
/// queried index.  Borrows the `cfg` and `registry` for the
/// point-wise query API.
pub struct CommandBinding<'a> {
    block_entry: HashMap<BlockId, State>,
    ordered_blocks: Vec<BlockId>,
    cfg: &'a CfgFunction,
    registry: &'a CommandRegistry,
}

impl CommandBinding<'_> {
    fn state_at_block(&self, block: BlockId, stmt_idx: usize) -> State {
        let mut state = self.block_entry.get(&block).cloned().unwrap_or_default();
        if let Some(blk) = self.cfg.blocks.get(&block) {
            for stmt in blk.statements.iter().take(stmt_idx) {
                stmt_gen(stmt, &mut state, self.registry);
            }
        }
        state
    }

    /// The binding of `command_name` when `block::stmt_idx` executes.
    #[must_use]
    pub fn binding_at(&self, block: BlockId, stmt_idx: usize, command_name: &str) -> Binding {
        binding_in(
            &self.state_at_block(block, stmt_idx),
            &nqn(command_name),
            self.registry,
        )
    }

    /// True when `command_name` still denotes its core builtin here.
    #[must_use]
    pub fn is_original_builtin_at(
        &self,
        block: BlockId,
        stmt_idx: usize,
        command_name: &str,
    ) -> bool {
        self.binding_at(block, stmt_idx, command_name)
            .is_original_builtin()
    }

    /// Every command name perturbed from its default *anywhere* in the
    /// body — the flow-insensitive union over all points of names whose
    /// binding ever differs from its default.
    #[must_use]
    pub fn rebound_names(&self) -> HashSet<String> {
        let mut names = HashSet::new();
        for block in &self.ordered_blocks {
            let mut state = self.block_entry.get(block).cloned().unwrap_or_default();
            self.collect_rebound(&state, &mut names);
            if let Some(blk) = self.cfg.blocks.get(block) {
                for stmt in &blk.statements {
                    stmt_gen(stmt, &mut state, self.registry);
                    self.collect_rebound(&state, &mut names);
                }
            }
        }
        names
    }

    fn collect_rebound(&self, state: &State, names: &mut HashSet<String>) {
        for (name, binding) in &state.map {
            if *binding != default_binding(name, self.registry) {
                names.insert(name.clone());
            }
        }
    }

    /// True when some path performs a *dynamic* command-table mutation.
    #[must_use]
    pub fn has_wildcard(&self) -> bool {
        for block in &self.ordered_blocks {
            let mut state = self.block_entry.get(block).cloned().unwrap_or_default();
            if state.wildcard {
                return true;
            }
            if let Some(blk) = self.cfg.blocks.get(block) {
                for stmt in &blk.statements {
                    stmt_gen(stmt, &mut state, self.registry);
                    if state.wildcard {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Compute the flow-sensitive command-binding lattice for `cfg`.
///
/// `initial` seeds the entry block's state — the command bindings
/// already in force when this function begins.  The top-level analysis
/// seeds it with every module procedure (`{qname: Proc(qname)}`) so a
/// proc defined inside a `namespace eval` block is still known to be a
/// proc, while top-level `rename` / redefinition events perturb it
/// flow-sensitively.
#[must_use]
pub fn analyse_command_binding<'a>(
    cfg: &'a CfgFunction,
    registry: &'a CommandRegistry,
    initial: &[(String, Binding)],
) -> CommandBinding<'a> {
    let mut preds: HashMap<BlockId, Vec<BlockId>> =
        cfg.blocks.keys().map(|id| (*id, Vec::new())).collect();
    for &id in cfg.blocks.keys() {
        for succ in cfg.block_successors(id) {
            if let Some(v) = preds.get_mut(&succ) {
                v.push(id);
            }
        }
    }

    let order = cfg.reverse_postorder();
    let seed = State {
        map: initial.iter().cloned().collect(),
        wildcard: false,
    };

    let mut block_entry: HashMap<BlockId, State> = cfg
        .blocks
        .keys()
        .map(|id| (*id, State::default()))
        .collect();
    let mut block_exit = block_entry.clone();

    // Monotonic forward fixpoint: the per-name lattice has height 3 and
    // the join only rises, so RPO iteration terminates.
    let mut changed = true;
    while changed {
        changed = false;
        for id in &order {
            let entry = {
                let mut pred_states: Vec<&State> = preds
                    .get(id)
                    .map(|ps| ps.iter().map(|p| &block_exit[p]).collect())
                    .unwrap_or_default();
                if *id == cfg.entry {
                    pred_states.push(&seed);
                }
                merge_preds(&pred_states, registry)
            };
            block_entry.insert(*id, entry.clone());
            let mut exit_state = entry;
            if let Some(blk) = cfg.blocks.get(id) {
                for stmt in &blk.statements {
                    stmt_gen(stmt, &mut exit_state, registry);
                }
            }
            if exit_state != block_exit[id] {
                block_exit.insert(*id, exit_state);
                changed = true;
            }
        }
    }

    CommandBinding {
        block_entry,
        ordered_blocks: order,
        cfg,
        registry,
    }
}

/// Conservative, flow-insensitive summary of command rebindings across a
/// whole module — the input to the optimiser's builtin-fold trust gate.
///
/// A `rename` / proc redef / `interp alias` buried in a proc body only
/// takes effect when that proc is *called*, and the cross-proc call order
/// is not statically known.  Rather than a full interprocedural
/// call-effect fixpoint, this takes the sound over-approximation: any
/// core builtin some body may rebind is treated as untrusted
/// *everywhere*.  Top-level rebindings stay precise via the
/// flow-sensitive [`CommandBinding`] lattice; this whole-module union is
/// the conservative fold gate.
///
/// `Default` trusts everything (no names, not dynamic) — the
/// "no mutations observed" baseline.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleCommandMutations {
    /// Canonical names of core builtins some body may rebind.
    names: std::collections::HashSet<String>,
    /// Canonical names that are the *source* or *target* of a `rename`, or
    /// the alias name of an `interp alias`, anywhere in the module — i.e.
    /// a name that no longer reliably denotes the proc it was declared as
    /// (or never denoted one at all). Unlike `names`, this is NOT
    /// restricted to builtins: a plain `proc NAME { … }` declaration is
    /// deliberately excluded (declaring a name as itself is the expected,
    /// trustworthy binding) — only `rename` / `interp alias` touching the
    /// name is recorded. Feeds [`Self::trusts_proc_binding`].
    rebound: std::collections::HashSet<String>,
    /// A body performs a dynamic `rename`/alias/proc (target not
    /// statically known) → resolution of *any* name is opaque.
    dynamic: bool,
    /// A body changes how a namespace *resolves* command names, without
    /// renaming anything: `namespace import`/`forget`, `namespace path`,
    /// `namespace unknown`, `namespace ensemble`, or a namespace delete.
    /// These never touch [`Self::rebound`], yet
    /// `namespace import -force ::evil::abs` into `::tcl::mathfunc` replaces
    /// what `abs(…)` resolves to just as a `rename` would. Recorded whatever
    /// the target namespace, because the pattern list is a Tcl value this
    /// scan does not evaluate.
    resolution_changed: bool,
    /// Namespaces whose command resolution was changed in a way this scan
    /// cannot enumerate — a globbed or non-literal `namespace import`, a
    /// `namespace path`/`unknown`/`ensemble`, or a namespace delete. A name
    /// *in* one of these is no longer trustworthy; a name outside them is
    /// unaffected, which is what keeps `namespace import ::lib::*` into
    /// `::app` from distrusting `::lib::helper` itself.
    opaque_namespaces: std::collections::HashSet<String>,
}

/// Prepared, narrow procedure-binding trust projection for call-site
/// parameter seeding. Unlike [`ModuleCommandMutations`], it intentionally
/// excludes unresolved commands, external units, and unavailable bodies when
/// those facts do not name a dynamic command-binding transition.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProcBindingTrustProjection {
    rebound: std::collections::HashSet<String>,
    dynamic: bool,
}

impl ProcBindingTrustProjection {
    /// Whether `proc_name` still denotes its declared procedure at an
    /// arbitrary later call site.
    #[must_use]
    pub(crate) fn trusts_proc_binding(&self, proc_name: &str) -> bool {
        !self.dynamic && !self.rebound.contains(&nqn(proc_name))
    }

    /// Whether a dynamic command-binding transition made every procedure
    /// binding untrustworthy.
    #[must_use]
    pub(crate) const fn has_dynamic_binding_transition(&self) -> bool {
        self.dynamic
    }
}

impl ModuleCommandMutations {
    /// Whether any command-table mutation has a dynamic source or target.
    ///
    /// Consumers retaining typed proof declines need to distinguish this
    /// lattice top from a statically known rebinding of one command name.
    #[must_use]
    pub const fn has_dynamic_mutation(&self) -> bool {
        self.dynamic
    }

    /// Whether `name` sits in a namespace whose resolution this scan could
    /// not enumerate — see [`Self::opaque_namespaces`].
    fn import_shadowed(&self, name: &str) -> bool {
        if self.opaque_namespaces.is_empty() {
            return false;
        }
        let qualified = nqn(name);
        let namespace = crate::optimiser::helpers::naming::namespace_from_qualified(&qualified);
        self.opaque_namespaces.contains(&namespace)
    }

    /// Whether any body changes namespace command *resolution* — see
    /// [`Self::resolution_changed`]. A consumer folding a command it resolved
    /// itself must decline when this is true.
    #[must_use]
    pub const fn changes_command_resolution(&self) -> bool {
        self.resolution_changed
    }

    /// Whether any command whose canonical name lies under `prefix` is
    /// rebound — a `rename` source or target, or an `interp alias` name.
    ///
    /// The math-function gate asks this about `::tcl::mathfunc::`: `expr`
    /// resolves `abs(…)` through the command table, so
    /// `rename ::tcl::mathfunc::abs {}` must stop the compiler evaluating it
    /// natively (C Tcl then raises `invalid command name`).
    #[must_use]
    pub fn rebinds_under(&self, prefix: &str) -> bool {
        let bare = prefix.strip_prefix("::").unwrap_or(prefix);
        self.rebound.iter().any(|name| {
            let canonical = name.strip_prefix("::").unwrap_or(name);
            canonical.starts_with(bare)
        })
    }

    /// True when `command_name` is not clobbered by any body mutation —
    /// i.e. the optimiser may still fold it with its original builtin
    /// semantics.
    #[must_use]
    pub fn trusts(&self, command_name: &str) -> bool {
        if self.dynamic || self.import_shadowed(command_name) {
            return false;
        }
        !self.names.contains(&nqn(command_name))
    }

    /// The everything-is-untrusted lattice top: `trusts` /
    /// `trusts_proc_binding` answer `false` for every name. The sound
    /// stand-in when a consumer has **no whole-module view at all** (the
    /// analyser's isolated per-item body pass, issue #1132) — folding with
    /// builtin semantics is then never permitted.
    #[must_use]
    pub fn distrust_all() -> Self {
        Self {
            names: std::collections::HashSet::new(),
            rebound: std::collections::HashSet::new(),
            dynamic: true,
            resolution_changed: true,
            opaque_namespaces: std::collections::HashSet::new(),
        }
    }

    /// A canonical, hashable snapshot of this summary — see
    /// [`CommandTrustSnapshot`].
    #[must_use]
    pub fn snapshot(&self) -> CommandTrustSnapshot {
        let mut untrusted_builtins: Vec<String> = self.names.iter().cloned().collect();
        untrusted_builtins.sort_unstable();
        let mut rebound: Vec<String> = self.rebound.iter().cloned().collect();
        rebound.sort_unstable();
        CommandTrustSnapshot {
            untrusted_builtins,
            rebound,
            dynamic: self.dynamic,
            resolution_changed: self.resolution_changed,
            opaque_namespaces: {
                let mut v: Vec<String> = self.opaque_namespaces.iter().cloned().collect();
                v.sort_unstable();
                v
            },
        }
    }

    /// True when `proc_name` can still be trusted to denote the module
    /// procedure it was declared as at an arbitrary later call site — i.e.
    /// its bare name was never the subject of a later `rename` (as the old
    /// name being moved away *or* the new name a different command moved
    /// onto) or `interp alias` (as the alias name) anywhere in the module.
    ///
    /// Flow-insensitive and whole-module, like [`Self::trusts`]: a
    /// rebinding buried in a proc body only takes effect when that proc
    /// runs, and the cross-proc call order isn't statically known, so any
    /// observed rebinding of the name is treated as live everywhere. This
    /// is what makes it sound to gate the optimiser's proc-call constant
    /// fold (O103) on this query — folding a call to the *original* proc's
    /// constant return would miscompile a script that later does
    /// `rename otherProc thisName` or `interp alias {} thisName {} other`.
    #[must_use]
    pub fn trusts_proc_binding(&self, proc_name: &str) -> bool {
        if self.dynamic || self.import_shadowed(proc_name) {
            return false;
        }
        !self.rebound.contains(&nqn(proc_name))
    }
}

/// A canonical (sorted), hashable form of [`ModuleCommandMutations`], so
/// the whole-module trust fact can ride inside a memoisation key — the
/// analyser's per-item body pass carries it on each deferred body whose
/// text could fold a command substitution (issue #1132), keeping the
/// isolated fragment memo sound when a `rename` elsewhere in the file
/// appears or disappears.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandTrustSnapshot {
    untrusted_builtins: Vec<String>,
    rebound: Vec<String>,
    dynamic: bool,
    /// Part of the key: a memo taken with namespace command resolution
    /// unperturbed must not be reused once a `namespace import` appears.
    resolution_changed: bool,
    opaque_namespaces: Vec<String>,
}

impl CommandTrustSnapshot {
    /// Rebuild the queryable summary this snapshot was taken from.
    #[must_use]
    pub fn to_mutations(&self) -> ModuleCommandMutations {
        ModuleCommandMutations {
            names: self.untrusted_builtins.iter().cloned().collect(),
            rebound: self.rebound.iter().cloned().collect(),
            dynamic: self.dynamic,
            resolution_changed: self.resolution_changed,
            opaque_namespaces: self.opaque_namespaces.iter().cloned().collect(),
        }
    }
}

/// Collect tampered-with *core builtins* (default `Builtin` but observed
/// otherwise) plus the wildcard flag from `state` into the accumulators.
/// A freshly-defined user proc (default `Opaque` → `Proc`) is deliberately
/// excluded: it doesn't untrust any builtin.
fn collect_tampered_builtins(
    state: &State,
    registry: &CommandRegistry,
    names: &mut std::collections::HashSet<String>,
    dynamic: &mut bool,
) {
    if state.wildcard {
        *dynamic = true;
    }
    for (name, binding) in &state.map {
        let default = default_binding(name, registry);
        if *binding != default && default.kind == BindingKind::Builtin {
            names.insert(name.clone());
        }
    }
}

/// Record the proc-binding-relevant names touched by a `rename` or `interp
/// alias` statement: the *source* name (vacated — no longer denotes what it
/// used to) and, for a static target, the *destination* name (now denoting
/// whatever the source used to, not its own original declaration, if it
/// even had one). A `proc` (re)declaration is deliberately NOT recorded —
/// declaring `NAME` as itself is the expected, trustworthy binding; only
/// *moving* a binding onto a name, or *vacating* a name that used to denote
/// a proc, breaks the "this bare name still denotes the proc it was
/// declared as" invariant [`ModuleCommandMutations::trusts_proc_binding`]
/// needs. Independent of the [`State`] lattice — a direct syntactic scan,
/// since (unlike the builtin-only trust gate) there is no meaningful
/// "default" binding to diff a proc name against: a plain declaration
/// *also* changes the name's binding away from its textual default, so
/// diffing against `default_binding` cannot distinguish "declared itself"
/// from "rebound to something else".
fn collect_proc_rebindings(
    stmt: &Statement,
    namespace: &str,
    registry: &CommandRegistry,
    rebound: &mut std::collections::HashSet<String>,
    dynamic: &mut bool,
    resolution: &mut bool,
    opaque: &mut std::collections::HashSet<String>,
) {
    if !matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. }) {
        return;
    }
    let ns_of_site = namespace;
    if let Some(facts) = invocation_facts(stmt, registry)
        && let Some(transitions) = facts.state_transitions.declared()
    {
        for fact in transitions.facts() {
            match &fact.transition {
                StateTransition::CommandBinding(CommandBindingTransition::Move { from, to }) => {
                    let (Some(from), Some(to)) = (literal_subject(from), literal_subject(to))
                    else {
                        *dynamic = true;
                        return;
                    };
                    insert_rebound_candidates(from, namespace, rebound);
                    if !to.is_empty() {
                        insert_rebound_candidates(to, namespace, rebound);
                    }
                }
                StateTransition::CommandBinding(CommandBindingTransition::Delete {
                    interpreter,
                    name,
                }) => {
                    let affects_current = match interpreter {
                        None => true,
                        Some(subject) => match literal_subject(subject) {
                            Some("") => true,
                            Some(_) => false,
                            None => {
                                *dynamic = true;
                                return;
                            }
                        },
                    };
                    if affects_current {
                        let Some(name) = literal_subject(name) else {
                            *dynamic = true;
                            return;
                        };
                        insert_rebound_candidates(name, namespace, rebound);
                    }
                }
                StateTransition::CommandBinding(CommandBindingTransition::Alias {
                    source_interpreter,
                    alias,
                    ..
                }) => {
                    let Some(source_interpreter) = literal_subject(source_interpreter) else {
                        *dynamic = true;
                        return;
                    };
                    if source_interpreter.is_empty() {
                        let Some(alias) = literal_subject(alias) else {
                            *dynamic = true;
                            return;
                        };
                        insert_rebound_candidates(alias, namespace, rebound);
                    }
                }
                StateTransition::CommandBinding(CommandBindingTransition::Unknown { .. }) => {
                    *dynamic = true;
                    return;
                }
                StateTransition::Widen(widening)
                    if widening
                        .domains
                        .contains(&StateTransitionDomain::CommandBindings) =>
                {
                    *dynamic = true;
                    return;
                }
                // Resolution changes, not rebindings: no name moves, but what
                // a name resolves to in some namespace does.
                //
                // An import binds each pattern's *tail* in the target
                // namespace, so a literal, glob-free pattern is exact — that
                // is `namespace import -force ::evil::answer` shadowing a
                // declared `::ns::answer`. A glob (`::lib::*`) names commands
                // this scan cannot enumerate, so only the *target* namespace
                // becomes opaque; a name elsewhere stays trustworthy.
                StateTransition::Namespace(transition) => {
                    if !collect_namespace_resolution(
                        transition,
                        Some(ns_of_site),
                        rebound,
                        resolution,
                        opaque,
                    ) {
                        *dynamic = true;
                        return;
                    }
                }
                StateTransition::CommandBinding(CommandBindingTransition::Define { .. })
                | StateTransition::Interpreter(_)
                | StateTransition::VariableCellAlias(_)
                | StateTransition::Trace(_)
                | StateTransition::ObjectDispatch(_)
                | StateTransition::Widen(_) => {}
            }
        }
    }
}

/// Record every name a bare `rename` / `interp alias` argument could
/// resolve to when it runs inside `namespace` — Tcl resolves an
/// unqualified command name against the *current* namespace at the point
/// the `rename`/`interp alias` executes, not the global namespace (a
/// `proc ::ns::doit {} { rename triple double }` renames `::ns::triple`
/// to `::ns::double`, not `::triple`/`::double` — confirmed against
/// tclsh 9.0.4). This scan is flow-insensitive and doesn't know whether a
/// same-named command already exists in `namespace` at that point, so it
/// conservatively records BOTH the namespace-relative and the
/// global-rooted candidate for a bare name — the same sound
/// over-approximation [`collect_tampered_builtins`] already applies.
/// A name that already contains `::` resolves unambiguously (rooted at
/// `::`, matching the optimiser's own `resolve_proc_qname` simplified
/// qualification rule), so only one candidate is recorded for it.
fn insert_rebound_candidates(name: &str, namespace: &str, rebound: &mut impl Extend<String>) {
    if name.contains("::") || namespace == "::" {
        rebound.extend([nqn(name)]);
        return;
    }
    rebound.extend([nqn(&format!("{namespace}::{name}")), nqn(name)]);
}

/// The mutable rebinding-tracking state [`walk_body_calls`] threads
/// through its recursive descent, grouped into one struct (rather than
/// three separate `&mut` parameters) so adding the depth-cap parameter
/// below doesn't push the function over clippy's `too_many_arguments`
/// threshold.
struct RebindState<'a> {
    names: &'a mut std::collections::HashSet<String>,
    rebound: &'a mut std::collections::HashSet<String>,
    dynamic: &'a mut bool,
    resolution: &'a mut bool,
    opaque: &'a mut std::collections::HashSet<String>,
}

/// Apply the gen of every `Call` / `Barrier` in `script` (recursing into
/// nested structured bodies, in source order) to `state`, collecting
/// after *each* mutation — so a builtin renamed away and later restored
/// (`rename string ms; …; rename ms string`) is still recorded as
/// tampered within that window. `depth` is the nesting level of `script`
/// — reuses [`crate::optimiser::MAX_OPTIMISER_WALK_DEPTH`] (this walker
/// isn't itself part of the optimiser module, but shares the same
/// `Script`/`Statement`-tree-depth semantics as every walker guarded by
/// that constant, so a second identically-valued constant would only add
/// drift risk).
fn walk_body_calls(
    script: &crate::ir::Script,
    state: &mut State,
    registry: &CommandRegistry,
    namespace: &str,
    rebind: &mut RebindState<'_>,
    depth: u32,
) {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        *rebind.dynamic = true;
        return;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::Call { .. } | Statement::Barrier { .. } => {
                collect_proc_rebindings(
                    stmt,
                    namespace,
                    registry,
                    rebind.rebound,
                    rebind.dynamic,
                    rebind.resolution,
                    rebind.opaque,
                );
                stmt_gen(stmt, state, registry);
                collect_tampered_builtins(state, registry, rebind.names, rebind.dynamic);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    walk_body_calls(&c.body, state, registry, namespace, rebind, depth + 1);
                }
                if let Some(b) = else_body {
                    walk_body_calls(b, state, registry, namespace, rebind, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                walk_body_calls(init, state, registry, namespace, rebind, depth + 1);
                walk_body_calls(next, state, registry, namespace, rebind, depth + 1);
                walk_body_calls(body, state, registry, namespace, rebind, depth + 1);
            }
            Statement::While { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Foreach { body, .. } => {
                walk_body_calls(body, state, registry, namespace, rebind, depth + 1);
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                walk_body_calls(body, state, registry, namespace, rebind, depth + 1);
                for h in handlers {
                    walk_body_calls(&h.body, state, registry, namespace, rebind, depth + 1);
                }
                if let Some(fb) = finally_body {
                    walk_body_calls(fb, state, registry, namespace, rebind, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        walk_body_calls(b, state, registry, namespace, rebind, depth + 1);
                    }
                }
                if let Some(b) = default_body {
                    walk_body_calls(b, state, registry, namespace, rebind, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// Summarise command-table mutations across the whole module — a
/// CFG-free recursive IR walk over the top-level script *and* every proc
/// / method body, so it can run before per-function CFGs are built.
///
/// Tampered-with core builtins and rebound names generally are reported
/// (see [`collect_tampered_builtins`]).  The result feeds both the
/// optimiser's builtin-fold trust gate ([`ModuleCommandMutations::trusts`])
/// and its proc-call fold trust gate
/// ([`ModuleCommandMutations::trusts_proc_binding`]).
#[must_use]
pub fn scan_module_command_mutations(
    ir_module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> ModuleCommandMutations {
    let command_bindings = ModuleCommandBindings::analyse(ir_module, registry);
    scan_module_command_mutations_with_bindings(ir_module, registry, &command_bindings)
}

/// Join registry-declared namespace-resolution effects with an already-built
/// closed command-binding lattice.
///
/// Compilation-unit construction uses this seam so CFG construction,
/// optimiser trust, and runtime provenance consume one binding analysis. The
/// public convenience wrapper above remains available to callers that do not
/// already retain the prepared lattice.
#[must_use]
pub(crate) fn scan_module_command_mutations_with_bindings(
    ir_module: &crate::ir::Module,
    registry: &CommandRegistry,
    command_bindings: &ModuleCommandBindings,
) -> ModuleCommandMutations {
    let mut names = std::collections::HashSet::new();
    let mut rebound = std::collections::HashSet::new();
    let mut dynamic = !ir_module.redefined_procedures.is_empty();
    let mut resolution_changed = false;
    let mut opaque_namespaces = std::collections::HashSet::new();

    let mut has_runtime_selected_root = false;
    {
        let mut visit = |script: &crate::ir::Script, namespace: &str| {
            let mut state = State::default();
            let mut rebind = RebindState {
                names: &mut names,
                rebound: &mut rebound,
                dynamic: &mut dynamic,
                resolution: &mut resolution_changed,
                opaque: &mut opaque_namespaces,
            };
            walk_body_calls(script, &mut state, registry, namespace, &mut rebind, 0);
        };

        // This flow-insensitive mutation projection must inspect body units:
        // unlike the closed binding walk above, it does not replay readable
        // invocation bodies from source.
        for (script, namespace) in ir_module.executable_script_roots() {
            match namespace {
                crate::ir::ExecutionNamespace::Exact(namespace) => visit(script, &namespace),
                crate::ir::ExecutionNamespace::RuntimeSelected => {
                    // A relative command in a receiver/caller-selected namespace
                    // can reach an implementation absent from the static module.
                    // Absolute-only bodies remain namespace-invariant and can use
                    // the ordinary scan. The closed binding analysis below still
                    // retains exact absolute transitions in opaque bodies.
                    if crate::ir_helpers::requires_runtime_command_namespace(script, registry) {
                        has_runtime_selected_root = true;
                    } else {
                        visit(script, "::");
                    }
                }
            }
        }
    }
    dynamic |= has_runtime_selected_root;

    // The source-recursive legacy walk above sees bodies retained in `Module`,
    // while this owner also recovers bodies installed through alias prefixes.
    // Project its closed may-state into the same optimiser trust summary so
    // every compiler consumer agrees about those runtime-created procedures.
    let projected = command_bindings.mutation_projection(registry);
    names.extend(projected.names);
    rebound.extend(projected.rebound);
    dynamic |= projected.dynamic;
    resolution_changed |= projected.resolution_changed;
    opaque_namespaces.extend(projected.opaque_namespaces);

    ModuleCommandMutations {
        names,
        rebound,
        dynamic,
        resolution_changed,
        opaque_namespaces,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn analyse(src: &str) -> (CompilationUnit, CommandRegistry) {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &reg, false);
        (cu, reg)
    }

    /// A `namespace import` that shadows a proc declared in the same
    /// namespace must distrust that binding — tclsh answers `EVIL` for the
    /// sheet below, so folding `::ns::answer` with the declared body is
    /// wrong. A *globbed* import elsewhere must not distrust the source
    /// namespace it imports *from*, or an ordinary `namespace import ::lib::*`
    /// would stop every fold in `::lib`.
    #[test]
    fn an_import_shadowing_a_declared_proc_distrusts_only_its_namespace() {
        let reg = CommandRegistry::build_default();
        let shadowed = "namespace eval ::evil { proc answer {} {return EVIL} ; namespace export answer }\n             namespace eval ::ns { proc answer {} { return 42 } ; namespace import -force ::evil::answer }\n";
        let cu = CompilationUnit::build_for(shadowed, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !m.trusts_proc_binding("::ns::answer"),
            "the import shadows the declared proc"
        );
        assert!(
            m.trusts_proc_binding("::evil::answer"),
            "the namespace imported *from* is untouched"
        );

        // A wildcard import makes only its target namespace opaque.
        let wildcard = "namespace eval ::lib { proc helper {} { return 1 } ; namespace export helper }\n             namespace eval ::app { namespace import ::lib::* }\n";
        let cu = CompilationUnit::build_for(wildcard, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts_proc_binding("::app::helper"));
        assert!(
            m.trusts_proc_binding("::lib::helper"),
            "the imported-from namespace still folds"
        );
    }

    #[test]
    fn independently_analysed_summaries_are_semantically_equal() {
        let (cu, reg) = analyse("interp alias {} e {} expr\ne {1 + 1}");
        let first = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        let second = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

        assert!(!Arc::ptr_eq(&first.bindings, &second.bindings));
        assert!(!Arc::ptr_eq(
            &first.root_boundary_bindings,
            &second.root_boundary_bindings
        ));
        assert_eq!(first, second);
        assert!(
            !first.same_state(&second),
            "the analysis-local fast path deliberately requires one shared baseline Arc"
        );
    }

    #[test]
    fn binding_state_forks_copy_procedure_inventory_only_for_a_new_body() {
        let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
        let original = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        let mut fork = original.clone();

        assert!(Arc::ptr_eq(
            &original.procedure_bodies,
            &fork.procedure_bodies
        ));
        assert!(original.same_state(&fork));

        assert!(!fork.extend_procedure_bodies(["::first".to_owned()]));
        assert!(
            Arc::ptr_eq(&original.procedure_bodies, &fork.procedure_bodies),
            "re-observing a retained body must not detach the shared inventory"
        );

        assert!(fork.extend_procedure_bodies(["::recovered".to_owned()]));
        assert!(!Arc::ptr_eq(
            &original.procedure_bodies,
            &fork.procedure_bodies
        ));
        assert!(!original.procedure_bodies.contains("::recovered"));
        assert!(fork.procedure_bodies.contains("::recovered"));
        assert!(!original.same_state(&fork));
    }

    #[test]
    fn binding_state_forks_copy_binding_map_only_for_a_changed_entry() {
        let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
        let original = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        let mut unchanged = original.clone();

        assert!(Arc::ptr_eq(&original.bindings, &unchanged.bindings));
        assert!(Arc::ptr_eq(
            &original.root_boundary_bindings,
            &unchanged.root_boundary_bindings
        ));
        assert!(!unchanged.join(&original));
        assert!(
            Arc::ptr_eq(&original.bindings, &unchanged.bindings),
            "joining an identical fork must not detach the shared map"
        );

        let (existing_name, existing_bindings) = original
            .bindings
            .iter()
            .next()
            .expect("procedure analysis publishes at least one binding");
        unchanged.replace(existing_name.clone(), existing_bindings.clone());
        assert!(
            Arc::ptr_eq(&original.bindings, &unchanged.bindings),
            "replacing an entry with its current value must not detach the shared map"
        );

        let mut changed = original.clone();
        changed.replace(
            "::recovered".to_owned(),
            BTreeSet::from([MayBinding::Unknown]),
        );
        assert!(!Arc::ptr_eq(&original.bindings, &changed.bindings));
        assert!(!original.bindings.contains_key("::recovered"));
        assert_eq!(
            changed.bindings.get("::recovered"),
            Some(&BTreeSet::from([MayBinding::Unknown]))
        );
        assert!(
            Arc::ptr_eq(
                &original.root_boundary_bindings,
                &changed.root_boundary_bindings
            ),
            "a flow-state mutation must not copy the immutable boundary map"
        );
        assert!(Arc::ptr_eq(
            &original.procedure_bodies,
            &changed.procedure_bodies
        ));
        assert!(!original.same_state(&changed));
    }

    #[test]
    fn exact_procedure_delta_preserves_history_and_root_boundary() {
        let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        let procedure_target = MayBinding::Target(ResolvedCommandTarget {
            command: "::first".to_owned(),
            prepended: Vec::new(),
            registry_backed: false,
            terminal: true,
        });

        assert_eq!(
            bindings.bindings.get("::first"),
            Some(&BTreeSet::from([
                MayBinding::Missing,
                procedure_target.clone()
            ])),
            "the historical view includes the state before and after definition"
        );
        assert_eq!(
            bindings.root_boundary_bindings.get("::first"),
            Some(&BTreeSet::from([procedure_target])),
            "only the post-definition binding is replayable at another root"
        );
    }

    #[test]
    fn exact_procedure_delta_retains_non_binding_opacity() {
        let mut reg = CommandRegistry::build_default();
        let mut external_proc = reg.get("proc").expect("core proc spec").clone();
        external_proc.traits |= tcl_registry::Traits::LOADS_EXTERNAL_UNIT;
        reg.insert(external_proc);

        // The leading command ensures the historical state already exists
        // when the exact definition runs. The former one-key fast path then
        // suppressed the ordinary observation and lost the external-unit
        // opacity which accompanied the otherwise-exact Define(Procedure).
        let cu =
            CompilationUnit::build_for("set marker 1\nproc created {} {return ok}", &reg, false);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

        assert!(bindings.opaque_binding_mutation);
        assert!(bindings.has_opaque_domain());
        assert!(
            bindings.mutation_projection(&reg).has_dynamic_mutation(),
            "the historical mutation projection must retain external-unit opacity"
        );
    }

    #[test]
    fn exact_procedure_delta_retains_immediate_body_binding_mutation() {
        let mut reg = CommandRegistry::build_default();
        let mut immediate_proc = reg.get("proc").expect("core proc spec").clone();
        immediate_proc
            .traits
            .remove(tcl_registry::Traits::DEFERS_BODY);
        immediate_proc.frame_effect = Some(tcl_registry::FrameEffectSpec {
            level_word: tcl_registry::FrameLevelWord::None,
            layout: tcl_registry::FrameArgLayout::ScriptInCurrentFrame,
        });
        reg.insert(immediate_proc);

        // This authored Proc-lowering operation both defines `created` and
        // immediately evaluates its body. The nested definition changes a
        // second binding without necessarily growing any non-binding axis:
        // both procedure bodies can already be present in the module inventory.
        let cu = CompilationUnit::build_for(
            "set marker 1\nproc created {} {proc set {} {return hijacked}}",
            &reg,
            false,
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

        assert!(
            !bindings.mutation_projection(&reg).trusts("set"),
            "the historical view must retain the immediately evaluated body's builtin redefinition"
        );
    }

    #[test]
    fn narrow_proc_binding_projection_excludes_unavailable_body_opacity() {
        let (cu, reg) = analyse("missing_command argument\nproc retained {} {return ok}");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

        assert!(
            bindings.mutation_projection(&reg).has_dynamic_mutation(),
            "an unresolved command may load code that changes builtin trust"
        );
        assert!(
            bindings
                .proc_binding_trust_projection()
                .trusts_proc_binding("::retained"),
            "unavailable code does not itself prove that a retained proc was rebound"
        );
    }

    #[test]
    fn narrow_proc_binding_projection_tracks_dynamic_rebinding_subjects() {
        let (cu, reg) =
            analyse("set name retained\nrename $name saved\nproc retained {} {return ok}");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        let trust = bindings.proc_binding_trust_projection();

        assert!(trust.has_dynamic_binding_transition());
        assert!(!trust.trusts_proc_binding("::retained"));
    }

    #[test]
    fn narrow_proc_binding_projection_retains_static_namespace_candidates() {
        let (cu, reg) = analyse("namespace eval ::n {rename retained saved}");
        let trust =
            ModuleCommandBindings::analyse(&cu.ir_module, &reg).proc_binding_trust_projection();

        assert!(!trust.has_dynamic_binding_transition());
        assert!(!trust.trusts_proc_binding("::n::retained"));
        assert!(
            !trust.trusts_proc_binding("::retained"),
            "the legacy source scan also records a global fallback candidate"
        );
    }

    #[test]
    fn unperturbed_builtin_is_builtin_no_rebound() {
        let (cu, reg) = analyse("string toupper a");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        assert!(cb.is_original_builtin_at(fu.cfg.entry, 0, "string"));
        assert!(cb.rebound_names().is_empty());
        assert!(!cb.has_wildcard());
    }

    #[test]
    fn class_destroy_makes_the_class_command_opaque() {
        // `Animal destroy` deletes the class command: the binding is Class
        // before the destroy and Opaque after, so a later `Animal new`
        // draws W128.  Definer creation and the destructive method are
        // both registry data (definition_body / oo::object's `destroy`).
        let (cu, reg) = analyse(
            "oo::class create Animal {}
Animal new
Animal destroy
Animal new",
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        let animal_calls: Vec<_> = fu.cfg.blocks[&entry]
            .statements
            .iter()
            .enumerate()
            .filter_map(|(index, stmt)| {
                matches!(stmt, Statement::Call { command, .. } | Statement::Barrier { command, .. }
                    if command == "Animal")
                .then_some(index)
            })
            .collect();
        assert_eq!(
            cb.binding_at(entry, animal_calls[0], "Animal").kind,
            BindingKind::Class
        );
        assert_eq!(
            cb.binding_at(entry, animal_calls[2], "Animal").kind,
            BindingKind::Opaque,
            "the class command is deleted after `Animal destroy`"
        );
        assert!(cb.rebound_names().contains("::Animal"));
    }

    #[test]
    fn instance_destroy_makes_the_instance_command_opaque() {
        // `Animal create fido` binds the instance command; `fido destroy`
        // deletes it; the class itself stays bound.
        let (cu, reg) = analyse(
            "oo::class create Animal {}
Animal create fido
fido destroy
fido bark
Animal new",
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        let invocation_index = |command: &str, first_arg: &str| {
            fu.cfg.blocks[&entry]
                .statements
                .iter()
                .position(|stmt| {
                    matches!(stmt,
                        Statement::Call { command: source, args, .. }
                        | Statement::Barrier { command: source, args, .. }
                        if source == command
                            && args.first().is_some_and(|arg| arg == first_arg))
                })
                .unwrap()
        };
        assert_eq!(
            cb.binding_at(entry, invocation_index("fido", "bark"), "fido")
                .kind,
            BindingKind::Opaque
        );
        assert_eq!(
            cb.binding_at(entry, invocation_index("Animal", "new"), "Animal")
                .kind,
            BindingKind::Class
        );
    }

    #[test]
    fn closed_static_descriptor_does_not_mask_a_live_object_receiver() {
        let mut reg = CommandRegistry::build_default();
        reg.insert(tcl_registry::CommandSpec {
            name: "Animal",
            state_transitions: Some(tcl_registry::StateTransitionDescriptor::EMPTY),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let cu = CompilationUnit::build_for(
            "oo::class create Animal {}\nAnimal destroy\nAnimal new",
            &reg,
            false,
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        assert_eq!(
            cb.binding_at(fu.cfg.entry, 2, "Animal").kind,
            BindingKind::Opaque,
            "the live object binding wins over the shadowed static descriptor"
        );
    }

    #[test]
    fn destroy_as_ordinary_argument_is_not_a_deletion() {
        // A proc named `destroy` taking a class name as an ARGUMENT must
        // not delete anything: the head is the proc, not the class.
        let (cu, reg) = analyse(
            "oo::class create Animal {}
proc destroy {x} { puts $x }
destroy Animal
Animal new",
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        assert_eq!(
            cb.binding_at(entry, 3, "Animal").kind,
            BindingKind::Class,
            "the class survives an unrelated `destroy` call"
        );
    }

    #[test]
    fn snit_type_creation_binds_a_class() {
        let (cu, reg) = analyse(
            "snit::type Dog {}
Dog destroy
Dog create d",
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        assert_eq!(cb.binding_at(entry, 1, "Dog").kind, BindingKind::Class);
        assert_eq!(cb.binding_at(entry, 2, "Dog").kind, BindingKind::Opaque);
    }

    #[test]
    fn rename_deletion_makes_old_name_opaque_flow_sensitively() {
        // `string` is its builtin before the rename, opaque after.
        let (cu, reg) = analyse("string toupper a\nrename string {}\nstring toupper b");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        assert!(cb.is_original_builtin_at(entry, 0, "string"));
        assert_eq!(
            cb.binding_at(entry, 2, "string").kind,
            BindingKind::Opaque,
            "string is renamed away before stmt 2"
        );
        assert!(cb.rebound_names().contains("::string"));
    }

    #[test]
    fn rename_redirect_moves_binding_to_new_name() {
        let (cu, reg) = analyse("rename string mystr\nmystr toupper b");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        // After the rename: old `string` is opaque, `mystr` inherits the
        // builtin binding `string` denoted.
        assert_eq!(cb.binding_at(entry, 1, "string").kind, BindingKind::Opaque);
        assert_eq!(cb.binding_at(entry, 1, "mystr").kind, BindingKind::Builtin);
    }

    #[test]
    fn proc_redefinition_binds_name_to_proc() {
        let (cu, reg) = analyse("proc string {x} { return $x }\nstring foo");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        let b = cb.binding_at(entry, 1, "string");
        assert_eq!(b.kind, BindingKind::Proc);
        assert_eq!(b.target.as_deref(), Some("::string"));
        assert!(!cb.is_original_builtin_at(entry, 1, "string"));
        assert!(cb.rebound_names().contains("::string"));
    }

    #[test]
    fn dynamic_rename_collapses_to_wildcard() {
        let (cu, reg) = analyse("set x foo\nrename $x bar\nstring toupper a");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        assert!(cb.has_wildcard(), "dynamic rename sets the wildcard");
        // Under the wildcard everything resolves to Unknown (⊤), never a
        // concrete binding — so no spurious W128 can fire.
        let entry = fu.cfg.entry;
        assert_eq!(cb.binding_at(entry, 2, "string").kind, BindingKind::Unknown);
    }

    #[test]
    fn expanded_embedded_rename_widens_module_bindings() {
        // Tcl 9.0.4 expands this one source word into rename's two arguments:
        // `saved_llength {a b c}` subsequently returns 3. The source position
        // before expansion is therefore not a literal one-argument rename.
        let (cu, reg) = analyse("set ignored [rename {*}{llength saved_llength}]");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "expanded rename operands must conservatively widen command bindings"
        );
    }

    #[test]
    fn expanded_embedded_command_head_widens_module_bindings() {
        // Tcl 9.0.4 executes this as `rename string saved_string`; the expanded
        // source word is not a static command name, even though its wrapped
        // value is literal.
        let (cu, reg) = analyse("set ignored [{*}{rename string saved_string}]");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "an expanded command head has unknown command identity and argv shape"
        );
    }

    #[test]
    fn embedded_non_body_ensemble_leaf_preserves_command_trust() {
        let (cu, reg) = analyse("proc p {} {set x [namespace qualifiers ::a::b]}");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a resolved non-body namespace leaf must not inherit the root's dynamic-body opacity"
        );
        let mutations = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(mutations.trusts("namespace"));
        assert!(mutations.trusts("self"));
    }

    #[test]
    fn namespace_code_captures_direct_and_embedded_bodies_without_replaying_them() {
        for source in [
            "namespace code {rename set saved_set}",
            "set body {rename set saved_set}\nset callback [namespace code $body]",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                !bindings.has_opaque_domain(),
                "namespace code stores rather than executes its body: {source}"
            );
            assert!(
                !bindings
                    .targets("::saved_set", "::")
                    .iter()
                    .any(|target| target.command == "::set"),
                "a captured namespace code body must not mutate bindings now: {source}"
            );
        }
    }

    #[test]
    fn evaluated_body_interpreter_realm_preserves_only_proven_parent_bindings() {
        for source in [
            "interp create slave\ninterp eval slave {set x 99}",
            "interp create slave\ninterp eval slave {puts child}",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                !bindings.has_opaque_domain(),
                "a harmless readable body in a named child leaves the parent command table unchanged: {source}"
            );
        }

        for source in [
            "interp eval {} {rename set saved_set}",
            "set child slave\ninterp eval $child {set x 99}",
            "interp create slave\ninterp eval slave {rename set saved_set}",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "a current, unknown, or command-mutating child body must fail closed: {source}"
            );
        }
    }

    #[test]
    fn namespace_resolution_transitions_widen_but_creation_and_export_do_not() {
        for source in [
            "namespace eval ::n {}",
            "namespace eval ::n {namespace export *}",
            "set name v\nglobal $name",
            "set name v\nvariable $name",
            "set ns ::n\nnamespace upvar $ns v local",
            "set pattern *\nnamespace export $pattern",
        ] {
            let (cu, reg) = analyse(source);
            assert!(
                !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
                "a namespace or variable-cell transition that preserves lookup must stay closed: {source}"
            );
        }

        for source in [
            "namespace delete ::n",
            "namespace import -force ::m::*",
            "namespace forget ::m::*",
            "namespace path {::m}",
            "namespace unknown handler",
            "namespace ensemble create",
            "set value ::m\nnamespace path $value",
            "set value handler\nnamespace unknown $value",
        ] {
            let (cu, reg) = analyse(source);
            assert!(
                ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
                "a resolution-changing namespace transition must fail closed: {source}"
            );
        }
    }

    #[test]
    fn namespace_resolution_impact_survives_embedding_and_alias_prefixes() {
        for source in [
            "set result [namespace delete ::n]",
            "set result [namespace import -force ::m::*]",
            "set result [namespace forget ::m::*]",
            "set result [namespace path {::m}]",
            "set result [namespace unknown handler]",
            "set result [namespace ensemble create]",
            "interp alias {} mutate {} namespace delete\nmutate ::n",
            "interp alias {} mutate {} namespace import -force\nmutate ::m::*",
            "interp alias {} mutate {} namespace forget\nmutate ::m::*",
            "interp alias {} mutate {} namespace path\nmutate {::m}",
            "interp alias {} mutate {} namespace unknown\nmutate handler",
            "interp alias {} mutate {} namespace ensemble\nmutate create",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "resolved nested dispatch must retain namespace lookup impact: {source}"
            );
            assert!(
                scan_module_command_mutations(&cu.ir_module, &reg).changes_command_resolution(),
                "resolved nested dispatch must reach the shared mutation projection: {source}"
            );
            assert!(
                !scan_module_command_mutations(&cu.ir_module, &reg).has_dynamic_mutation(),
                "namespace lookup provenance must not become an unrelated dynamic proc rebinding: {source}"
            );
        }

        for source in [
            "set result [namespace export *]",
            "interp alias {} ensure_n {} namespace eval ::n {}\nensure_n",
            "interp alias {} export_any {} namespace export\nexport_any *",
        ] {
            let (cu, reg) = analyse(source);
            assert!(
                !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
                "Ensure and Export preserve existing command resolution: {source}"
            );
        }
    }

    #[test]
    fn external_unit_execution_widens_direct_and_embedded_command_bindings() {
        for source in [
            "source external.tcl",
            "set result [source external.tcl]",
            "load extension.so",
            "set result [load extension.so]",
            "auto_load widget",
            "set result [auto_load widget]",
            "auto_import pkg::*",
            "set result [auto_import pkg::*]",
            "package require Example",
            "set result [package require Example]",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "unavailable external code may mutate any command binding: {source}"
            );
        }
    }

    #[test]
    fn direct_unknown_and_unresolved_heads_widen_for_autoload() {
        for source in [
            "unknown missing_command",
            "missing_command argument",
            "set result [missing_command argument]",
        ] {
            let (cu, reg) = analyse(source);
            assert!(
                ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
                "default unknown may autoload a unit that mutates command bindings: {source}"
            );
        }

        let (cu, reg) = analyse("proc local_command {} {return ok}\nlocal_command");
        assert!(
            !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
            "a retained local procedure is resolved without the unknown path"
        );

        for source in [
            "rename unknown {}\nmissing_command argument",
            "interp alias {} unknown {} puts\nmissing_command argument",
        ] {
            let (cu, reg) = analyse(source);
            assert!(
                !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
                "a removed or transition-free replacement handler cannot autoload: {source}"
            );
        }

        let (cu, reg) =
            analyse("proc unknown {command args} {rename $command saved_command}\nmissing_command");
        assert!(
            ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
            "a retained dynamic unknown handler body remains opaque"
        );
    }

    #[test]
    fn a_missing_root_binding_dispatches_through_the_live_unknown_handler() {
        let (cu, reg) = analyse(
            "rename unknown old_unknown\n\
             interp alias {} unknown {} rename expr\n\
             proc maybe {} {}\n\
             rename maybe {}\n\
             maybe",
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.rebound_names().any(|name| name == "::expr"),
            "calling the missing command must execute the replacement unknown prefix and move expr"
        );
        assert!(
            bindings
                .targets("maybe", "::")
                .iter()
                .any(|target| target.command == "::expr"),
            "the unknown prefix must move expr onto the missing spelling"
        );

        let (cu, reg) = analyse("rename unknown {}\nmissing_command");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(bindings.targets("missing_command", "::").is_empty());
        assert!(!bindings.target_may_be_unknown("missing_command", "::"));
    }

    #[test]
    fn indeterminate_ensemble_dispatch_widens_direct_and_embedded_bodies() {
        for source in [
            "set op eval\nnamespace $op ::n {rename set saved_set}",
            "set op eval\nset result [namespace $op ::n {rename set saved_set}]",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "a dynamic subcommand can select a body-bearing leaf: {source}"
            );
        }
    }

    #[test]
    fn static_apply_replays_binding_changes_in_default_and_explicit_namespaces() {
        for (source, moved_name) in [
            ("apply {{} {rename set saved_set}}", "::saved_set"),
            (
                "namespace eval ::n {}\napply {{} {rename set saved_set} ::n}",
                "::n::saved_set",
            ),
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                !bindings.has_opaque_domain(),
                "a literal apply lambda is exactly replayable: {source}"
            );
            assert!(
                bindings
                    .targets(moved_name, "::")
                    .iter()
                    .any(|target| target.command == "::set"),
                "apply body did not move set to {moved_name}: {source}"
            );
        }
    }

    #[test]
    fn dynamic_apply_lambda_keeps_the_command_domain_opaque() {
        let (cu, reg) = analyse(
            "set lambda {{} {rename set saved_set}}\n\
             apply $lambda",
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "a substituted lambda may execute unavailable command mutations"
        );
    }

    #[test]
    fn alias_chain_prefix_replays_apply_body_after_arity_validation() {
        let source = "interp alias {} apply_saved {} apply {{} {rename set saved_set}}\n\
                      interp alias {} run_saved {} apply_saved\n\
                      run_saved";
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(!bindings.has_opaque_domain());
        assert!(
            bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "the alias chain must supply apply's lambda"
        );

        for source in [
            "interp alias {} bad {} apply {{x} {rename set saved_set}}\nbad",
            "interp alias {} bad {} apply {{{x y z}} {rename set saved_set}}\nbad",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(!bindings.has_opaque_domain(), "known apply error: {source}");
            assert!(
                !bindings
                    .targets("::saved_set", "::")
                    .iter()
                    .any(|target| target.command == "::set"),
                "an arity or formal-list error must precede the body: {source}"
            );
        }
    }

    #[test]
    fn static_namespace_eval_replays_binding_changes_in_target_namespace() {
        let (cu, reg) = analyse("namespace eval n { rename set saved_set }");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a literal namespace and body are exactly replayable"
        );
        assert!(
            bindings
                .targets("::n::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "namespace eval body must run in ::n"
        );
    }

    #[test]
    fn readable_nested_body_retains_intermediate_binding_history() {
        let (cu, reg) =
            analyse("namespace eval ::n { rename ::set ::saved_set; rename ::saved_set ::set }");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "the transient binding is observable while the evaluated body runs"
        );
        assert!(
            !bindings
                .root_boundary_bindings
                .get("::saved_set")
                .is_some_and(|states| states.iter().any(|state| matches!(
                    state,
                    MayBinding::Target(target) if target.command == "::set"
                ))),
            "transient history must not become a replayable root boundary"
        );
    }

    #[test]
    fn dynamic_namespace_eval_target_or_body_keeps_the_domain_opaque() {
        for source in [
            "set ns ::n\nnamespace eval $ns { rename set saved_set }",
            "set body {rename set saved_set}\nnamespace eval ::n $body",
            "set old set\nnamespace eval ::n [list rename $old saved_set]",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "dynamic namespace eval input must not be replayed as source: {source}"
            );
        }
    }

    #[test]
    fn alias_chain_prefix_replays_namespace_eval_in_normalised_namespace() {
        let source = "interp alias {} in_n {} namespace eval n::\n\
                      interp alias {} run_in_n {} in_n\n\
                      run_in_n {rename set saved_set}";
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(!bindings.has_opaque_domain());
        assert!(
            bindings
                .targets("::n::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "the alias chain must supply namespace eval's normalised target"
        );
    }

    #[test]
    fn empty_namespace_eval_prefix_does_not_execute_the_body() {
        let (cu, reg) = analyse(
            "interp alias {} in_empty {} namespace eval {}\n\
             in_empty {rename set saved_set}",
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(!bindings.has_opaque_domain());
        assert!(
            !bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "namespace eval rejects an empty target before entering its body"
        );
    }

    #[test]
    fn static_delete_and_recreate_does_not_widen_unrelated_commands() {
        let (cu, reg) = analyse(
            "proc p {} {}\n\
             rename p {}\n\
             proc p {} { set local 1 }",
        );
        assert!(cu.ir_module.redefined_procedures.contains("::p"));
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a readable discarded procedure body must not poison the command domain"
        );
        assert!(
            !bindings.target_may_be_unknown("set", "::"),
            "the unrelated set command remains exactly known"
        );
    }

    #[test]
    fn absolute_heads_keep_relative_transition_operands_in_the_current_namespace() {
        let (cu, reg) = analyse("namespace eval ::n {::rename ::set saved_set}");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::n::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "absolute head lookup must not replace the command's current namespace"
        );
        assert!(
            !bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "the relative destination must not be moved into the global namespace"
        );
    }

    #[test]
    fn absolute_proc_head_keeps_current_namespace_for_discarded_body() {
        let (cu, reg) = analyse(
            "namespace eval ::n {\n\
                 ::proc p {} {::rename ::set saved_set}\n\
                 ::proc p {} {}\n\
             }",
        );
        assert!(cu.ir_module.redefined_procedures.contains("::n::p"));
        let history = discarded_procedure_history(&cu.ir_module, &reg);
        assert!(!history.opaque);
        assert!(
            history
                .modules
                .iter()
                .any(|module| module.top_level_namespace == "::n")
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::n::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set")
        );
        assert!(
            !bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set")
        );
    }

    #[test]
    fn consumed_typed_head_is_revalidated_in_future_roots() {
        let (cu, reg) = analyse(
            "proc ::uses_set {} { set x 1 }\n\
             rename ::set ::saved_set",
        );
        assert!(
            cu.ir_module.procedures["::uses_set"]
                .body
                .command_binding_sites
                .iter()
                .next()
                .is_some(),
            "structured lowering must retain the consumed set dependency"
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "the callable procedure may execute after set no longer denotes the compiled identity"
        );
    }

    #[test]
    fn uplevel_global_proc_body_is_recovered_in_its_runtime_namespace() {
        for wrapper in [
            "uplevel #0 {proc p {} {rename expr saved_expr}}",
            "uplevel #0 {eval {proc p {} {rename expr saved_expr}}}",
        ] {
            let source = format!(
                "proc p {{}} {{}}\n\
                 namespace eval ::n {{\n\
                     proc installer {{}} {{ {wrapper} }}\n\
                 }}"
            );
            let (cu, reg) = analyse(&source);
            assert!(
                cu.ir_module.redefined_procedures.contains("::p"),
                "the #0 body is lowered in the same global namespace Tcl selects"
            );
            let history = discarded_procedure_history(&cu.ir_module, &reg);
            assert!(!history.opaque, "literal runtime body is exact: {wrapper}");
            assert!(
                history
                    .modules
                    .iter()
                    .any(|module| module.top_level_namespace == "::"),
                "misqualified retained bodies must be re-lowered globally: {wrapper}"
            );
            assert!(history.rebound_names.contains("::p"));

            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                !bindings.has_opaque_domain(),
                "literal bodies should stay closed: {bindings:#?}"
            );
            assert!(
                bindings
                    .targets("::saved_expr", "::")
                    .iter()
                    .any(|target| target.command == "::expr"),
                "the recovered body must mutate global expr, not ::n::expr: {wrapper}"
            );
            assert!(
                !scan_module_command_mutations(&cu.ir_module, &reg).trusts_proc_binding("::p"),
                "both runtime definitions of ::p make its identity unstable"
            );
        }
    }

    #[test]
    fn unreadable_uplevel_global_proc_body_is_opaque() {
        let (cu, reg) = analyse(
            "namespace eval ::n {\n\
                 proc installer {body} { uplevel #0 {proc p {} $body} }\n\
             }",
        );
        let history = discarded_procedure_history(&cu.ir_module, &reg);
        assert!(
            history.opaque,
            "a misqualified retained procedure cannot borrow an unreadable lexical body"
        );
    }

    #[test]
    fn unavailable_discarded_redefinition_body_keeps_the_domain_opaque() {
        let (cu, reg) = analyse(
            "proc install {replacement} {\n\
                 set p_name {p}\n\
                 proc p {} {}\n\
                 rename p {}\n\
                 proc $p_name {} $replacement\n\
             }\n\
             install {rename set saved_set}\n\
             p",
        );
        assert!(cu.ir_module.redefined_procedures.contains("::p"));
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "a runtime-supplied discarded body may mutate any command binding"
        );
    }

    #[test]
    fn direct_and_alias_prefixed_proc_bodies_are_recovered_before_binding() {
        for source in [
            "proc p {} {rename expr saved_expr}\np",
            "interp alias {} makep {} proc p; makep {} {rename expr saved_expr}; p",
            "interp alias {} makep_target {} proc p\n\
             interp alias {} makep {} makep_target\n\
             makep {} {rename expr saved_expr}\n\
             p",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                !bindings.has_opaque_domain(),
                "a literal typed proc body is exactly recoverable: {source}"
            );
            assert!(
                bindings
                    .targets("::saved_expr", "::")
                    .iter()
                    .any(|target| target.command == "::expr"),
                "the recovered procedure body must contribute its expr rename: {source}"
            );
            let mutations = scan_module_command_mutations(&cu.ir_module, &reg);
            assert!(
                !mutations.trusts("expr"),
                "compiler consumers must not trust expr after the recovered body: {source}"
            );
        }
    }

    #[test]
    fn constructed_namespace_eval_defers_procedure_body_effects_to_the_root_fixpoint() {
        let (cu, reg) =
            analyse("namespace eval ::n [list proc mutate {} {rename ::set ::saved_set}]");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "the constructed script and procedure body are both exact"
        );
        assert!(
            bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "the constructed procedure must become a future executable root"
        );
    }

    #[test]
    fn constructed_procedure_roots_discovered_by_a_root_reach_the_next_round() {
        let (cu, reg) = analyse(
            "namespace eval ::n [list proc installer {} {\
                 proc late {} {rename ::set ::saved_set}\
             }]",
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "a nested definition discovered while replaying installer must schedule late"
        );
    }

    #[test]
    fn constructed_procedure_redefinitions_retain_every_possible_body() {
        let (cu, reg) = analyse(
            "namespace eval ::n [list proc mutate {} {rename ::set ::saved_set}]\n\
             namespace eval ::n [list proc mutate {} {rename ::expr ::saved_expr}]",
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        for (alias, target) in [("::saved_set", "::set"), ("::saved_expr", "::expr")] {
            assert!(
                bindings
                    .targets(alias, "::")
                    .iter()
                    .any(|resolved| resolved.command == target),
                "every exact replacement body remains a possible future root: {alias}"
            );
        }
    }

    #[test]
    fn constructed_namespaced_tcloo_method_uses_runtime_receiver_namespace() {
        let (cu, reg) = analyse(
            "namespace eval ::n { oo::class create C {\
                 method mutate {} {set x 1}\
             } }",
        );
        assert_eq!(
            cu.ir_module.methods["::n::C::mutate"].execution_namespace,
            crate::ir::ExecutionNamespace::RuntimeSelected
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "an object-local set may shadow the typed-lowered global builtin"
        );
    }

    #[test]
    fn unretained_tcloo_body_keeps_the_command_domain_opaque() {
        let (cu, reg) = analyse(
            "set ::body {::rename ::set ::saved_set}\n\
             oo::class create C {method mutate {} $::body}",
        );
        assert!(cu.ir_module.oo_evidence.unretained_executable_roots);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "a runtime-callable body absent from the retained root inventory must fail closed"
        );
        assert!(
            scan_module_command_mutations(&cu.ir_module, &reg).has_dynamic_mutation(),
            "the optimiser projection must retain closed-lattice opacity"
        );
    }

    #[test]
    fn constructed_tcloo_method_keeps_absolute_command_heads_exact() {
        let (cu, reg) = analyse(
            "namespace eval ::n [list oo::class create C {\
                 method mutate {} {::rename ::set ::saved_set}\
             }]",
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "an absolute command and target remain namespace-invariant"
        );
    }

    #[test]
    fn unavailable_direct_or_alias_prefixed_proc_body_is_opaque() {
        for source in [
            "set body {rename expr saved_expr}\nproc p {} $body\np",
            "set body {rename expr saved_expr}\n\
             interp alias {} makep {} proc p\n\
             makep {} $body\n\
             p",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "a procedure target without a retained or recovered body is opaque: {source}"
            );
            assert!(
                bindings.target_may_be_unknown("expr", "::"),
                "the unavailable procedure body must make expr untrusted: {source}"
            );
            assert!(
                !scan_module_command_mutations(&cu.ir_module, &reg).trusts("expr"),
                "the unavailable body makes the whole command-trust projection dynamic: {source}"
            );
        }
    }

    #[test]
    fn try_handler_joins_command_mutation_from_exception_edge() {
        // A fall-through-capable try body may fail before or after the rename.
        // The analysis-only exception edges therefore contribute both the
        // pre-try Builtin state and the body-exit Opaque state to the handler.
        let (cu, reg) = analyse(
            "proc ::p {} {\n try {\n  rename string {}\n } on error {} {\n  string length abc\n }\n}",
        );
        let fu = cu.function("::p").unwrap();
        let handler = fu
            .cfg
            .blocks
            .iter()
            .find_map(|(&id, block)| block.name.starts_with("try_handler").then_some(id))
            .expect("try handler block");
        assert!(
            fu.cfg
                .exception_edges
                .iter()
                .any(|&(_, target)| target == handler),
            "the test must exercise an analysis-only exception edge"
        );

        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        assert_eq!(
            cb.binding_at(handler, 0, "string").kind,
            BindingKind::Unknown,
            "the handler must not infer either the original or renamed binding"
        );
        assert!(
            !cb.is_original_builtin_at(handler, 0, "string"),
            "an optimisation must not trust the builtin at handler entry"
        );
    }

    #[test]
    fn seed_marks_module_procs_as_proc() {
        // The W128 seed: a name seeded as PROC resolves to Proc at entry.
        let (cu, reg) = analyse("nonbuiltin a b");
        let fu = cu.function("::top").unwrap();
        let seed = vec![(
            "::myproc".to_owned(),
            Binding {
                kind: BindingKind::Proc,
                target: Some("::myproc".to_owned()),
            },
        )];
        let cb = analyse_command_binding(&fu.cfg, &reg, &seed);
        assert_eq!(
            cb.binding_at(fu.cfg.entry, 0, "myproc").kind,
            BindingKind::Proc
        );
    }

    #[test]
    fn module_mutations_distrust_rebound_builtins_only() {
        let reg = CommandRegistry::build_default();
        // A builtin renamed inside a proc body is distrusted everywhere
        // (over-approximation); a fresh user proc untrusts nothing.
        let cu = CompilationUnit::build_for(
            "proc clobber {} { rename string {} }\nproc myproc {} { return 1 }",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts("string"), "string is rebound in a proc body");
        assert!(m.trusts("lappend"), "an untouched builtin stays trusted");
        assert!(m.trusts("myproc"), "a fresh user proc untrusts nothing");

        // A dynamic mutation distrusts every name.
        let cu2 = CompilationUnit::build_for("set x foo\nrename $x bar", &reg, false);
        let m2 = scan_module_command_mutations(&cu2.ir_module, &reg);
        assert!(!m2.trusts("string") && !m2.trusts("lappend"));
    }

    /// Regression coverage for issue #996: `walk_body_calls` recurses once
    /// per nested `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`
    /// body, with no depth cap of its own before this fix. Transitively
    /// bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass today,
    /// so this is defence-in-depth / consistency with every other
    /// full-tree walker in this crate, not a currently-reproducible
    /// crash. 1000 levels of source nesting is comfortably past this new
    /// cap; the assertion is that `scan_module_command_mutations` returns
    /// at all, not what it returns. Spawns its own big-stack thread since
    /// the lexer/CST/segmenter stages upstream of the lowering cap still
    /// walk the full un-truncated source nesting before that cap trims
    /// it — same rationale as
    /// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_walk_body_calls() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = "proc clobber {} {\n".to_owned();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("rename string {}\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        src.push_str("}\n");
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let reg = CommandRegistry::build_default();
                let cu = CompilationUnit::build_for(&src, &reg, false);
                let _ = scan_module_command_mutations(&cu.ir_module, &reg);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn trusts_proc_binding_true_for_untouched_proc() {
        // TP control: a proc never named by any `rename` / `interp alias`
        // is trusted.
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for("proc myproc {} { return 1 }", &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(m.trusts_proc_binding("myproc"));
        assert!(m.trusts_proc_binding("::myproc"));
    }

    #[test]
    fn trusts_proc_binding_false_for_rename_source_and_target() {
        // FP guard: `rename triple double` perturbs BOTH names — `triple`
        // (vacated, no longer denotes what it did) and `double` (now
        // denotes `triple`'s body, not `double`'s own declaration).
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple double\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts_proc_binding("double"), "rename target");
        assert!(!m.trusts_proc_binding("triple"), "rename source");
    }

    #[test]
    fn trusts_proc_binding_false_for_interp_alias_name() {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc answer {} { return 42 }\nproc other {} { return 99 }\ninterp alias {} answer {} other\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts_proc_binding("answer"));
        // The alias *target* itself is untouched — still trusted.
        assert!(m.trusts_proc_binding("other"));
    }

    #[test]
    fn trusts_proc_binding_follows_alias_resolved_mutations() {
        let reg = CommandRegistry::build_default();
        let renamed = CompilationUnit::build_for(
            "proc p {} {return P}\nproc q {} {return Q}\n\
             interp alias {} r {} rename\n\
             r p oldp\nr q p\n",
            &reg,
            false,
        );
        let mutations = scan_module_command_mutations(&renamed.ir_module, &reg);
        for name in ["p", "q", "oldp"] {
            assert!(
                !mutations.trusts_proc_binding(name),
                "alias-resolved rename touched {name}"
            );
        }

        let aliased = CompilationUnit::build_for(
            "proc p {} {return P}\nproc q {} {return Q}\n\
             interp alias {} replace {} interp alias {} p {} q\n\
             replace\n",
            &reg,
            false,
        );
        let mutations = scan_module_command_mutations(&aliased.ir_module, &reg);
        assert!(!mutations.trusts_proc_binding("p"));
        assert!(mutations.trusts_proc_binding("q"));
    }

    #[test]
    fn trusts_proc_binding_unaffected_by_unrelated_rename() {
        // TN control: renaming a DIFFERENT proc must not untrust this one —
        // `trusts_proc_binding` is per-name, unlike the whole-module
        // `dynamic` wildcard.
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple somethingElse\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(m.trusts_proc_binding("double"));
    }

    #[test]
    fn trusts_proc_binding_false_for_namespace_relative_rename() {
        // FP guard (reported in code review): a bare `rename` argument
        // inside a namespaced proc resolves relative to THAT proc's own
        // namespace, not the global namespace — `rename triple double`
        // inside `proc ::ns::doit` renames `::ns::triple` onto
        // `::ns::double`. An earlier version always rooted the bare names
        // globally (`::triple`/`::double`), so it never distrusted the
        // actually-affected namespaced names.
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "namespace eval ::ns {\n    proc double {n} { expr {$n * 2} }\n    proc triple {n} { expr {$n * 3} }\n}\nproc ::ns::doit {} { rename triple double }\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !m.trusts_proc_binding("::ns::double"),
            "namespace-relative rename target"
        );
        assert!(
            !m.trusts_proc_binding("::ns::triple"),
            "namespace-relative rename source"
        );
        // The scan is flow-insensitive (it can't know whether `double`/
        // `triple` already existed as GLOBAL commands at the point
        // `::ns::doit` runs), so it conservatively distrusts the
        // global-rooted candidate too — a deliberate, sound
        // over-approximation (a missed fold, never a wrong one),
        // mirroring `collect_tampered_builtins`'s existing philosophy.
        assert!(
            !m.trusts_proc_binding("::double"),
            "global-rooted candidate"
        );
        assert!(
            !m.trusts_proc_binding("::triple"),
            "global-rooted candidate"
        );
    }

    // Regression: `proc max {...}` must not be distrusted. `max`/`min` read
    // like `tcl::mathop` operator words, but real Tcl never registered them
    // there (verified against tclsh 8.6/9.0 — `info commands
    // ::tcl::mathop::*` never lists them); they exist only as unrelated
    // `expr` math functions. A now-fixed registry bug once carried bare
    // `max`/`min` `CommandSpec` entries as if they were `tcl::mathop`
    // members, which made `default_binding` treat them as pre-existing
    // builtins — so a completely ordinary `proc max {...}` looked like it
    // was "renaming a builtin", silently blocking O103 from folding calls to
    // it (caught by `tests/optimiser.rs::interprocedural_constant_folding`).
    #[test]
    fn module_mutations_do_not_distrust_proc_named_like_mathop_word() {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc max {a b} {\n    if {$a > $b} { return $a } else { return $b }\n}\nset v [max 3 7]\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            m.trusts("max"),
            "a plain proc sharing a name with an (incorrectly bare-registered) \
             tcl::mathop-lookalike must not be distrusted"
        );
    }
}
