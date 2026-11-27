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

//! The whole-module may-state: initial table, resolution, transitions, joins and the IR walk.

use super::source_analysis_entry::SourceAnalysisOptions;
use super::source_walk::SourceBodyEntryPurpose;
use super::{
    BindingWalkOutcome, DeferredImplementationId, DeferredSourceBody, DiscardedProcedureHistory,
    ModuleCommandBindings, RetainedEvaluatedBody, SourceCommandBindings, SourceExecutionContext,
    retained_source,
};
use crate::ir::{Module, Script, Statement};
use crate::var_escape::helpers::invocation_facts;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;
use tcl_names::command::SourceCommandKey;
use tcl_names::namespace::SourceNamespaceKey;
use tcl_registry::{CommandBindingDefinitionKind, CommandBindingTransition, CommandRegistry};

mod binding_transitions;
mod binding_walk;
mod bounded_classes;
mod evaluated_bodies;
mod initial_state;
mod invocation_transitions;
mod mutation_record;
mod namespace_transitions;
mod resolved_invocation;
mod source_order;
mod state_equality;
mod state_join;
mod target_resolution;

pub(super) use binding_transitions::apply_may_binding_transition;
use binding_walk::collect_normal_binding_states;
pub(super) use binding_walk::{BindingInvocationMetadata, apply_embedded_transitions_in};
pub(super) use bounded_classes::{BoundedClassConstruction, definition_body_command_tokens};
pub(crate) use evaluated_bodies::constructed_script_words_with_metadata_context;
#[cfg(test)]
pub(super) use evaluated_bodies::constructed_script_words_with_optional_metadata;
pub(super) use evaluated_bodies::qualify_execution_name;
#[cfg(test)]
pub(super) use invocation_transitions::{ProcedureDefinitionReplay, recover_procedure_definition};
pub(super) use invocation_transitions::{
    apply_declared_binding_transitions, apply_may_invocation_transitions,
};
#[cfg(test)]
pub(super) use namespace_transitions::collect_namespace_resolution;
pub(super) use namespace_transitions::{apply_namespace_transition, imported_binding_links};
pub(crate) use resolved_invocation::{ResolvedFrameBody, ResolvedFrameBodySelection};
pub(crate) use source_order::SourceBindingTimeline;

/// A registry invocation selected through the module-wide may-binding state.
/// Alias-prefix arguments have already been prepended.
pub(crate) struct ResolvedBindingInvocation {
    pub(super) command: String,
    pub(super) source_span: tcl_lexer::Span,
    pub(crate) facts: Box<tcl_registry::InvocationFacts>,
    pub(crate) arguments: Vec<String>,
    /// Post-alias argv values proven literal at their effective positions.
    /// Dynamic, expanded, and opaque positions are `None` even though
    /// [`Self::arguments`] retains their source spelling for diagnostics and
    /// token metadata alignment.
    pub(super) literal_arguments: Vec<Option<String>>,
    /// Genuine written scalar substitutions; captured alias values have none.
    pub(super) source_scalar_arguments: Vec<Option<String>>,
    /// Exact effective argv length after alias-prefix insertion. Expansion or
    /// another indeterminate source word leaves this unknown.
    pub(super) exact_argument_count: Option<usize>,
}

impl ModuleCommandBindings {
    /// Build the executable may-state under the same entry contract as source
    /// interpretation. Catalogue-only callers retain unknown loader effects.
    #[must_use]
    pub(crate) fn analyse_with_options(
        module: &Module,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Self {
        if let Some(observed) = Self::analyse_retained_source(module, registry, options) {
            return observed;
        }
        Self::analyse_retained_roots(module, registry, options)
    }

    /// Prefer the source kernel's actual execution history over IR replay.
    fn analyse_retained_source(
        module: &Module,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Option<Self> {
        if let Some(retained) = &module.retained_source_bindings
            && let Some(observed) = retained.projection(module, registry, options)
        {
            return Some(observed);
        }
        if !module.source.is_empty() {
            let config = module.native_lexer_config();
            let namespace = if module.top_level_namespace.is_empty() {
                "::"
            } else {
                &module.top_level_namespace
            };
            let frame = options.source_point_entry.map_or_else(
                || {
                    retained_source::source_analysis_frame(
                        options.native_entry,
                        namespace,
                        module.top_level_kind,
                    )
                },
                |point| point.frame().clone(),
            );
            let interpreted = SourceCommandBindings::analyse_image_in_frame_with_purpose(
                &module.source,
                &frame,
                config,
                registry,
                options,
                match module.top_level_kind {
                    crate::ir::TopLevelKind::Script => {
                        tcl_lexer::SourceSegmentationPurpose::WholeInput
                    }
                    crate::ir::TopLevelKind::ProcedureBody => {
                        tcl_lexer::SourceSegmentationPurpose::NestedScript
                    }
                },
            )?;
            return Some(interpreted.module_projection(module));
        }
        None
    }

    fn discarded_opaque_binding_mutation(
        module: &Module,
        discarded: &DiscardedProcedureHistory,
    ) -> bool {
        discarded.opaque
            || module.oo_evidence.unretained_executable_roots
            || discarded
                .modules
                .iter()
                .any(|module| module.oo_evidence.unretained_executable_roots)
    }

    /// An IR-only caller supplies executable roots without an original source instance.
    fn analyse_retained_roots(
        module: &Module,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Self {
        use crate::analysis_control::{AnalysisCheckpoint, checkpoint};
        let discarded = discarded_procedure_history(module, registry);
        let opaque_binding_mutation = Self::discarded_opaque_binding_mutation(module, &discarded);
        let dynamic_proc_binding = module
            .independent_executable_script_roots()
            .into_iter()
            .any(|(script, namespace)| {
                matches!(namespace, crate::ir::ExecutionNamespace::RuntimeSelected)
                    && crate::ir_helpers::requires_runtime_command_namespace(script, registry)
            });
        let procedure_bodies = retained_procedure_names(module, &discarded);
        let mut state = Self {
            opaque_domain: opaque_binding_mutation || options.unknown_entry,
            opaque_binding_mutation: opaque_binding_mutation || options.unknown_entry,
            dynamic_proc_binding,
            procedure_bodies: Arc::new(procedure_bodies),
            rebound_names: discarded
                .rebound_names
                .into_iter()
                .map(SourceCommandKey::authored)
                .collect::<BTreeSet<_>>()
                .into(),
            has_redefined_procedures: !module.redefined_procedures.is_empty(),
            ..Self::initial_with_options(registry, options, Some(module.native_lexer_config()))
        };
        let baseline = Arc::make_mut(&mut state.baseline);
        baseline.declared = Arc::new(module.declared_frame_effects.clone());
        baseline.refresh_fingerprint();
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
        let top = collect_normal_binding_states(
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
            if checkpoint(AnalysisCheckpoint::Fixpoint) {
                return observed;
            }
            let before = live.clone();
            let root_count_before = retained_roots.len();
            let body_roots = retained_roots.snapshot();
            let mut next = live.clone();
            for root in &body_roots {
                let outcome = collect_normal_binding_states(
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
            for root in retained_roots
                .source_roots
                .values()
                .cloned()
                .collect::<Vec<_>>()
            {
                let frame = tcl_names::variable::VariableExecutionFrame::Procedure {
                    namespace: root.namespace.clone(),
                    identity: root.identity.clone(),
                }
                .with_namespace_identity(root.namespace_key.clone());
                let outcome = interpret_binding_source(
                    &root.source,
                    root.offset,
                    &root.namespace,
                    &frame,
                    registry,
                    &live,
                    &mut retained_roots,
                );
                next.join(&outcome.post);
                observed.join(&outcome.observed);
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
}

/// Recover the bodies which [`Module::procedures`] cannot retain when one
/// statically named procedure is defined more than once. The declaration
/// statements remain in executable IR, so typed procedure-definition
/// provenance can distinguish a readable discarded body from a genuinely
/// unavailable one. Readable bodies are lowered and walked like every retained
/// procedure body; only an unreadable history widens the whole command domain.
fn retained_procedure_names(
    module: &Module,
    discarded: &DiscardedProcedureHistory,
) -> BTreeSet<String> {
    let mut procedure_bodies: BTreeSet<String> = module
        .procedures
        .values()
        .map(|procedure| procedure.qualified_name.clone())
        .collect();
    for discarded_module in &discarded.modules {
        procedure_bodies.extend(
            discarded_module
                .procedures
                .values()
                .map(|procedure| procedure.qualified_name.clone()),
        );
    }
    procedure_bodies
}

pub(super) fn discarded_procedure_history(
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
                    let recovered = recovered_procedure_source(&body, &qname);
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
        .map(|(_, procedure)| (procedure.qualified_name.clone(), procedure.span))
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

pub(super) fn procedure_declaration(
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

pub(super) fn procedure_namespace(qname: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
    if holder.is_empty() {
        "::".to_owned()
    } else {
        holder.to_owned()
    }
}

fn recovered_procedure_source(body: &str, qname: &str) -> Module {
    Module {
        source: tcl_lexer::SourceImage::document(body),
        top_level_namespace: procedure_namespace(qname),
        ..Module::default()
    }
}

/// Shared ownership for a binding walk. Normal historical analysis leaves the
/// timeline absent; source-order projection records statement snapshots.
struct BindingWalkContext<'a> {
    pub(super) registry: &'a CommandRegistry,
    retained_roots: &'a mut RetainedBindingRoots,
    timeline: Option<&'a mut SourceBindingTimeline>,
    source_order_mode: bool,
}

fn interpret_binding_source(
    source: &str,
    offset: u32,
    namespace: &str,
    frame: &tcl_names::variable::VariableExecutionFrame,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    retained: &mut RetainedBindingRoots,
) -> BindingWalkOutcome {
    let config = registry
        .profile()
        .map_or_else(tcl_lexer::LexerConfig::default, |profile| {
            tcl_lexer::LexerConfig::from_grammar(profile.grammar)
        });
    let mut interpretation = SourceCommandBindings {
        deferred: retained.source_roots.clone().into(),
        ..SourceCommandBindings::default()
    };
    let mut post = bindings.clone();
    interpretation.walk_source(
        source,
        offset,
        &mut post,
        &SourceExecutionContext {
            original_logical_body: None,
            control: crate::command_binding::source_execution_control::SourceExecutionControl::new(
                if matches!(
                    frame.layout(),
                    tcl_names::variable::VariableExecutionFrame::Procedure { .. }
                        | tcl_names::variable::VariableExecutionFrame::ReceiverMethod { .. }
                ) {
                    bindings.baseline.body_compilation()
                } else {
                    bindings.baseline.native_compilation
                },
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
                SourceBodyEntryPurpose::Conditional,
                bindings.baseline.invocation_realm,
            ),
            compilation_snapshot: None,
            selected_compilation: None,
            original_variable_compilation: None,
            namespace,
            namespace_key: frame.namespace_identity(),
            config,
            registry,
            frame,
            depth: 0,
            invocation_offset: 0,
            variable_read_owner: None,
            original_written_projection: None,
            written_arguments: None,
            written_values: None,
            written_name_values: None,
            written_representations: None,
            written_objects: None,
            written_method_prefixes: None,
            written_variable_reads: None,
            expression_source: None,
        },
    );
    let mut observed = post.clone();
    for point in &interpretation.points {
        observed.join(&point.state);
    }
    retained.source_roots.extend(interpretation.deferred);
    BindingWalkOutcome { post, observed }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct RetainedBindingRoot {
    /// A recovered body must outlive the temporary `Module` used to lower
    /// it, but fixpoint snapshots need only duplicate a handle, not its full
    /// IR tree.
    pub(super) script: Arc<Script>,
    pub(super) namespace: crate::ir::ExecutionNamespace,
}

/// Monotone inventory of callable roots discovered while replaying exact,
/// readable Tcl. It is intentionally separate from [`ModuleCommandBindings`]:
/// roots control the fixpoint worklist, while the published lattice and memo
/// identity contain only observable command-table state.
#[derive(Default)]
pub(super) struct RetainedBindingRoots {
    source_roots: BTreeMap<DeferredImplementationId, DeferredSourceBody>,
    pub(super) roots: Vec<RetainedBindingRoot>,
    pub(super) seen: HashSet<RetainedBindingRoot>,
    evaluated_bodies: HashMap<crate::ir::ExecutionNamespace, Vec<RetainedEvaluatedBody>>,
}

impl RetainedBindingRoots {
    pub(super) fn insert(&mut self, script: &Script, namespace: crate::ir::ExecutionNamespace) {
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
        if !skip_top
            && module.top_level.statements.is_empty()
            && !module.source.is_empty()
            && let Ok(source) = module.source.try_text()
        {
            let namespace = if module.top_level_namespace.is_empty() {
                "::"
            } else {
                &module.top_level_namespace
            };
            let identity = format!("recovered:{namespace}:{source}");
            self.source_roots.insert(
                DeferredImplementationId {
                    command: identity.clone(),
                    generation: 0,
                    allocation: None,
                },
                DeferredSourceBody {
                    realm: module.source_entry.invocation_realm,
                    identity,
                    implementation_generation: 0,
                    source: source.to_owned(),
                    offset: 0,
                    namespace: namespace.to_owned(),
                    namespace_key: module
                        .top_level_namespace_context
                        .clone()
                        .unwrap_or_else(|| SourceNamespaceKey::authored(namespace)),
                    event: None,
                    receiver_method: false,
                    receiver_namespace: None,
                    future_frame: None,
                    parameters: Vec::new(),
                    original_parameters: None,
                    statics: None,
                    implementation_allocation: None,
                    source_origin: None,
                    executed_script: None,
                    executed_word: None,
                },
            );
        }
        // Evaluated bodies keep their owning source location so identical Tcl
        // text lowered under different command states cannot collide. The
        // binding walk selects one only from inside the invocation currently
        // being replayed.
        let mut body_units: Vec<_> = module.body_lowerings().collect();
        body_units.sort_by(|a, b| {
            a.1.span
                .start()
                .cmp(&b.1.span.start())
                .then_with(|| a.0.cmp(b.0))
        });
        for (_, unit) in body_units {
            let Ok(start) = usize::try_from(unit.span.start()) else {
                continue;
            };
            let Ok(end) = usize::try_from(unit.span.end()) else {
                continue;
            };
            let Some(source) = module
                .source
                .try_text()
                .ok()
                .and_then(|source| source.get(start..end))
            else {
                continue;
            };
            let namespace = unit
                .body
                .execution_namespace(crate::ir::ExecutionNamespace::RuntimeSelected);
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

    pub(super) fn evaluated_body(
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

    pub(super) fn len(&self) -> usize {
        self.roots.len() + self.source_roots.len()
    }

    pub(super) fn snapshot(&self) -> Vec<RetainedBindingRoot> {
        self.roots.clone()
    }
}
