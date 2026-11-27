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

//! Body-keyed MAY writes to global and namespace variables.
//!
//! The flow-insensitive scan collects every possible outer-place declaration
//! before projecting writes, so disjoint branches conservatively share alias
//! facts. Exact original procedure calls then propagate these summaries to a
//! monotone fixpoint. Unknown targets widen the outer frame instead of selecting
//! a body by its display name.
//!
//! Registry variable roles and binding-transition descriptors own recognition;
//! dynamic target and caller-frame obligations remain opaque.

mod source_may;
use super::frame_effects::FrameContext;
pub(crate) use source_may::OriginalSourceCallMayWrites;

use std::collections::{BTreeSet, HashMap};

use crate::command_binding::ModuleCommandBindings;
use crate::ir::{Module, Script, Statement};
use crate::ir_helpers::{ExecutionNamespace, nested_execution_bodies};
use crate::registry_invocation::{InvocationMetadataContext, InvocationMetadataInput};
use crate::var_observability::{State, stmt_gen_with_metadata_context};

/// Exact borrowed input for conditional global/caller-frame source summaries.
#[derive(Clone)]
struct GlobalWriteSemantics<'a> {
    module: Option<&'a Module>,
    registry: &'a tcl_registry::CommandRegistry,
    aliases: &'a ModuleCommandBindings,
    selection: InvocationMetadataInput<'a>,
    metadata: Option<InvocationMetadataContext<'a>>,
    config: tcl_lexer::LexerConfig,
    input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    logical_realm: Option<crate::realm::CommandBindingRealm>,
    /// Each body's caller-frame summary and the owners selecting a call's
    /// callees: a call writes in this frame what its callee writes in its
    /// caller's (`upvar 1 $name v; set v …`).
    caller_frames: Option<CallerFrameWrites<'a>>,
}

#[derive(Clone, Copy)]
struct CallerFrameWrites<'a> {
    owners: &'a super::OriginalProcedureSummaryOwners,
    frames: &'a HashMap<crate::ir::ProcedureId, super::upvar_info::UpvarInfo>,
}

impl<'a> GlobalWriteSemantics<'a> {
    fn logical_source(&self) -> bool {
        self.input
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context)
    }

    fn logical_footprint(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<(
        tcl_registry::VariableWriteProjection,
        Vec<tcl_registry::VariableCellAliasTransition>,
    )> {
        if let Some(module) = self.module {
            let input = module.source_metadata_input.as_ref()?;
            let words = crate::registry_invocation::source_structure::original_logical_registry_words_for_module(
                module, self.registry, tokens,
            )?;
            return words.with_source_schema(input.borrowed_context_registry(), |schema| {
                Some((
                    tcl_registry::CommandRegistry::variable_write_projection_for_selected_source(
                        schema,
                    ),
                    schema.authored_logical_source_variable_aliases()?,
                ))
            })?;
        }
        crate::registry_invocation::original_logical_variable_footprint_with_metadata_context(
            self.registry,
            self.metadata?,
            tokens,
        )
    }

    fn standalone(&self) -> bool {
        matches!(self.selection, InvocationMetadataInput::Standalone)
    }

    fn resolve_statement(
        &self,
        statement: &Statement,
        namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
    ) -> Vec<crate::command_binding::ResolvedBindingInvocation> {
        if self.standalone() {
            self.aliases
                .resolve_statement(statement, self.registry, namespace)
        } else {
            self.aliases.resolve_statement_with_metadata_context(
                statement,
                self.registry,
                self.metadata,
                namespace,
            )
        }
    }

    fn resolve_words(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
    ) -> Vec<tcl_registry::InvocationFacts> {
        if self.standalone() {
            self.aliases
                .resolve_command_words(words, self.registry, namespace)
        } else {
            self.aliases.resolve_command_words_with_metadata_context(
                words,
                self.registry,
                self.metadata,
                namespace,
            )
        }
    }

    fn statement_writes(
        &self,
        statement: &Statement,
        tokens: Option<&crate::ir::CommandTokens>,
        namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
    ) -> tcl_registry::VariableWriteProjection {
        if self.logical_source() {
            return tokens
                .and_then(|tokens| self.logical_footprint(tokens))
                .map_or(
                    tcl_registry::VariableWriteProjection {
                        opaque_variable_frame: true,
                        ..Default::default()
                    },
                    |(writes, _)| writes,
                );
        }
        if !self.standalone() {
            return self
                .aliases
                .variable_write_projection_with_metadata_context(
                    statement,
                    self.registry,
                    namespace,
                    self.metadata,
                );
        }
        let mut projection = tcl_registry::VariableWriteProjection::default();
        self.aliases
            .for_each_resolved_invocation(statement, namespace, |target, words| {
                if target.registry_backed {
                    merge_write_projection(
                        &mut projection,
                        self.registry.variable_write_projection(words),
                    );
                }
            });
        projection
    }

    fn command_writes(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
    ) -> tcl_registry::VariableWriteProjection {
        if !self.standalone() {
            return self
                .aliases
                .variable_write_projection_for_command_words_with_metadata_context(
                    words,
                    self.registry,
                    self.metadata,
                    namespace,
                );
        }
        let Some(head) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            return tcl_registry::VariableWriteProjection {
                opaque_variable_frame: true,
                ..Default::default()
            };
        };
        let mut projection = tcl_registry::VariableWriteProjection {
            opaque_variable_frame: self
                .aliases
                .target_resolution_may_be_unknown(head, namespace),
            ..Default::default()
        };
        self.aliases
            .for_each_resolved_command_words(words, namespace, |target, invocation| {
                if target.registry_backed {
                    merge_write_projection(
                        &mut projection,
                        self.registry.variable_write_projection(invocation),
                    );
                }
            });
        projection
    }

    fn from_module(
        module: &'a Module,
        registry: &'a tcl_registry::CommandRegistry,
        aliases: &'a ModuleCommandBindings,
    ) -> Option<Self> {
        let metadata = InvocationMetadataContext::for_module(registry, module)?;
        let input = metadata.source_analysis_input()?;
        Some(Self {
            module: Some(module),
            registry,
            aliases,
            selection: InvocationMetadataInput::SuppliedSource(Some(input)),
            metadata: Some(metadata),
            config: module.lexer_config,
            input: Some(input),
            logical_realm: original_logical_effect_realm(module, registry),
            caller_frames: None,
        })
    }

    /// The names the callees of the call `tokens` writes in this frame, by
    /// their caller-frame summaries projected onto its own words; `true` when
    /// one may write a name the summary cannot state.
    fn callee_caller_frame_writes(&self, tokens: &crate::ir::CommandTokens) -> (Vec<String>, bool) {
        let (Some(frames), Some(module)) = (self.caller_frames, self.module) else {
            return (Vec::new(), false);
        };
        let Some(calls) = frames.owners.calls(tokens, self.registry) else {
            return (Vec::new(), false);
        };
        let mut names = Vec::new();
        let mut opaque = false;
        for (callee, words) in calls {
            let (Some(info), Some(procedure)) =
                (frames.frames.get(&callee), module.procedures.get(&callee))
            else {
                continue;
            };
            let facts = words
                .words
                .iter()
                .skip(1)
                .map(|word| {
                    crate::registry_invocation::effective_invocation_word(
                        word,
                        self.config.escapes,
                        tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
                    )
                })
                .collect::<Vec<_>>();
            let arguments = facts
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
                .collect::<Vec<_>>();
            let effects = info.caller_side_effects(
                tcl_registry::InvocationArguments::structured(&arguments),
                &procedure.params,
            );
            opaque |= effects.opaque;
            names.extend(effects.defs);
        }
        (names, opaque)
    }
}

fn original_logical_effect_realm(
    module: &Module,
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::realm::CommandBindingRealm> {
    module
        .source_metadata_input
        .as_ref()?
        .has_logical_source_name_context()
        .then_some(())?;
    crate::realm::original_logical_realm_from_source_bindings(
        module
            .retained_source_bindings
            .as_deref()?
            .positioned_for_module(module, registry)?,
    )
}

fn merge_write_projection(
    into: &mut tcl_registry::VariableWriteProjection,
    from: tcl_registry::VariableWriteProjection,
) {
    into.opaque_variable_frame |= from.opaque_variable_frame;
    for name in from.literal_names {
        if !into.literal_names.contains(&name) {
            into.literal_names.push(name);
        }
    }
    for name in from.read_before_write_names {
        if !into.read_before_write_names.contains(&name) {
            into.read_before_write_names.push(name);
        }
    }
}

fn literal_root(name: &str) -> &str {
    tcl_syntax::naming::split_element_ref(name).map_or(name, |(root, _)| root)
}

/// Per-proc summary: the outer-scope (global/namespace) variable names a
/// proc's body writes while aliased via `global` / `variable` / `upvar #0`,
/// or through a script it runs **at the global frame** (`uplevel #0 …`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GlobalWriteInfo {
    /// Original Logical MAY-write geometry; opaque frame effects remain independent.
    source_writes:
        Option<Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence>>,
    /// Literal outer-scope names this proc's body writes.
    pub names: BTreeSet<String>,
    /// The proc runs a script at the **global frame** that this analysis
    /// cannot read — `uplevel #0 $body`, `uplevel #0 [list set $v 1]` with
    /// an unresolvable target, or a global-frame script whose write target
    /// is not a plain literal.  Any global/namespace name may be written
    /// *or read* there, so a call site must widen to an opaque barrier
    /// instead of trusting [`Self::names`] — otherwise O102 forwards a stale
    /// global constant straight across the call.
    pub opaque_global_frame: bool,
}

impl std::hash::Hash for GlobalWriteInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.names, state);
        std::hash::Hash::hash(&self.opaque_global_frame, state);
    }
}

impl GlobalWriteInfo {
    /// Complete retained source MAY footprint; opacity never proves absence.
    pub(crate) fn original_source_writes(
        &self,
    ) -> Option<&[crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence]> {
        (!self.opaque_global_frame).then_some(())?;
        self.source_writes.as_deref()
    }

    /// True when the proc's body writes no outer-scope name.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && !self.opaque_global_frame
    }

    fn union_from(&mut self, other: &Self) -> bool {
        let old = self.clone();
        self.names.extend(other.names.iter().cloned());
        self.opaque_global_frame |= other.opaque_global_frame;
        match (&mut self.source_writes, &other.source_writes) {
            (Some(held), Some(incoming)) => {
                for place in incoming {
                    insert_source_write(held, place.clone());
                }
            }
            _ => self.source_writes = None,
        }
        *self != old
    }
}

/// Close body-keyed outer-place MAY facts over exact original call edges.
/// Missing source ownership or a missing callee summary widens the outer frame.
pub(crate) fn detect_global_write_bodies_with_registry(
    module: &Module,
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<crate::ir::ProcedureId, GlobalWriteInfo> {
    super::frame_effects::detect_frame_effect_bodies(module, registry).global
}

/// The global-frame projection of [`super::frame_effects`]: each retained
/// body's own outer-place writes, closed over the owner's call edges, with
/// the owner's caller-frame summaries for the frames its callees reach.
pub(super) fn global_frame_summaries(
    module: &Module,
    registry: &tcl_registry::CommandRegistry,
    bindings: &ModuleCommandBindings,
    owners: &super::OriginalProcedureSummaryOwners,
    frames: &HashMap<crate::ir::ProcedureId, super::upvar_info::UpvarInfo>,
    edges: &super::frame_effects::CallEdges,
) -> HashMap<crate::ir::ProcedureId, GlobalWriteInfo> {
    let semantics =
        GlobalWriteSemantics::from_module(module, registry, bindings).map(|semantics| {
            GlobalWriteSemantics {
                caller_frames: Some(CallerFrameWrites { owners, frames }),
                ..semantics
            }
        });
    let mut rows = HashMap::new();
    for (id, procedure) in module.summary_procedure_bodies() {
        let namespace = procedure
            .body
            .namespace_context
            .as_deref()
            .and_then(crate::command_binding::SourceNamespaceKey::advisory_key);
        let entered = procedure
            .body
            .namespace_context
            .as_deref()
            .map(|key| bindings.entered_body_namespace(key));
        let body_semantics = semantics.as_ref().map(|semantics| GlobalWriteSemantics {
            aliases: entered.as_deref().unwrap_or(bindings),
            ..semantics.clone()
        });
        let mut info = match (body_semantics.as_ref(), namespace) {
            (Some(semantics), Some(namespace))
                if module.procedure_ids.owns(*id)
                    && module
                        .retained_source_bindings
                        .as_ref()
                        .is_some_and(|owner| owner.matches_summary_body(*id, procedure)) =>
            {
                own_body_global_writes(&procedure.body, semantics, &namespace)
            }
            _ => GlobalWriteInfo {
                opaque_global_frame: true,
                ..GlobalWriteInfo::default()
            },
        };
        info.opaque_global_frame |= edges.get(id).is_none_or(|(_, opaque)| *opaque);
        rows.insert(*id, info);
    }
    loop {
        let prior = rows.clone();
        for (id, (callees, _)) in edges {
            let Some(row) = rows.get_mut(id) else {
                continue;
            };
            for callee in callees {
                match prior.get(callee) {
                    Some(facts) => {
                        row.union_from(facts);
                    }
                    None => row.opaque_global_frame = true,
                }
            }
        }
        if rows == prior {
            break;
        }
    }
    rows
}

/// Outer-place MAY facts for each retained body, with original source-selected edges.
#[must_use]
pub fn detect_global_write_procs(
    module: &Module,
) -> HashMap<crate::ir::ProcedureId, GlobalWriteInfo> {
    detect_global_write_procs_with_registry(module, module.resolved_registry())
}

/// Outer-place MAY writes under the Module's exact retained registry.
#[must_use]
pub fn detect_global_write_procs_with_registry(
    module: &Module,
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<crate::ir::ProcedureId, GlobalWriteInfo> {
    detect_global_write_bodies_with_registry(module, registry)
}

/// What `body`, run in a frame of its own whose commands resolve in
/// `namespace` — a lambda's body, as `apply` runs it — writes in the global
/// frame: what a procedure body's summary states of its own statements
/// ([`own_body_global_writes`]), unioned with the summary of each procedure it
/// calls (`procedures`, the map [`detect_global_write_procs_with_registry`]
/// builds), a called procedure the map does not hold making it opaque, as a
/// procedure's transitive closure does.
pub(crate) fn own_frame_global_writes_with_metadata_context<'a>(
    body: &Script,
    registry: &'a tcl_registry::CommandRegistry,
    aliases: &'a ModuleCommandBindings,
    namespace: &str,
    procedures: &HashMap<crate::ir::ProcedureId, GlobalWriteInfo>,
    owners: &'a super::OriginalProcedureSummaryOwners,
    (metadata, config): (
        Option<InvocationMetadataContext<'a>>,
        tcl_lexer::LexerConfig,
    ),
) -> GlobalWriteInfo {
    let Some(metadata) = metadata.filter(|metadata| {
        metadata.matches_registry(registry)
            && metadata.source_analysis_input().map_or_else(
                || metadata.is_standalone(),
                |input| input.lexer_config().nested().normalized() == config.nested().normalized(),
            )
    }) else {
        return GlobalWriteInfo {
            opaque_global_frame: true,
            ..GlobalWriteInfo::default()
        };
    };
    let module = owners.module(registry);
    let semantics = GlobalWriteSemantics {
        module,
        logical_realm: module.and_then(|module| original_logical_effect_realm(module, registry)),
        caller_frames: None,
        registry,
        aliases,
        selection: if metadata.is_standalone() {
            InvocationMetadataInput::Standalone
        } else {
            InvocationMetadataInput::SuppliedSource(metadata.source_analysis_input())
        },
        metadata: Some(metadata),
        config,
        input: metadata.source_analysis_input(),
    };
    frame_global_writes(body, &semantics, namespace, procedures, owners)
}

/// Explicit source-only catalogue compatibility. An unprofiled catalogue has
/// no `SemanticContext`; that deliberate absence never admits supplied refusal.
pub(crate) fn own_frame_global_writes_standalone(
    body: &Script,
    registry: &tcl_registry::CommandRegistry,
    aliases: &ModuleCommandBindings,
    namespace: &str,
    procedures: &HashMap<crate::ir::ProcedureId, GlobalWriteInfo>,
    owners: &super::OriginalProcedureSummaryOwners,
    config: tcl_lexer::LexerConfig,
) -> GlobalWriteInfo {
    let semantics = GlobalWriteSemantics {
        module: None,
        logical_realm: None,
        caller_frames: None,
        registry,
        aliases,
        selection: InvocationMetadataInput::Standalone,
        metadata: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
        config,
        input: None,
    };
    frame_global_writes(body, &semantics, namespace, procedures, owners)
}

fn frame_global_writes(
    body: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
    procedures: &HashMap<crate::ir::ProcedureId, GlobalWriteInfo>,
    owners: &super::OriginalProcedureSummaryOwners,
) -> GlobalWriteInfo {
    let mut info = own_body_global_writes(body, semantics, namespace);
    let (calls, calls_opaque) = direct_call_targets(body, owners, semantics.registry);
    info.opaque_global_frame |= calls_opaque;
    for callee in calls {
        match procedures.get(&callee) {
            Some(summary) => {
                info.union_from(summary);
            }
            None => info.opaque_global_frame = true,
        }
    }
    info
}

pub(crate) fn insert_source_write(
    writes: &mut Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence>,
    place: crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
) {
    if !writes.iter().any(|held| held.symbol() == place.symbol()) {
        writes.push(place);
    }
}

pub(crate) fn logical_alias_targets_global(
    alias: &tcl_registry::VariableCellAliasTransition,
    dialect: Option<tcl_native_model::InvocationDialect>,
) -> Option<bool> {
    if let tcl_registry::VariableAliasTarget::CallerSelectedFrame { frame, .. } = &alias.target {
        let level = match frame {
            tcl_registry::CallerFrameSelection::DefaultCaller => {
                Some(tcl_registry::FrameLevel::Relative(1))
            }
            tcl_registry::CallerFrameSelection::Explicit(level) => {
                tcl_registry::FrameLevel::parse_for_dialect(level.literal()?, dialect?)
            }
        }?;
        return Some(level.is_global_frame());
    }
    Some(true)
}

// Hashes select buckets; each retained authenticated slot still decides equality.
type SourceAliasBucket = Vec<(
    crate::signature_scan::variable_symbol::SignatureSourceVariableSlot,
    Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence>,
)>;

fn logical_body_source_writes(
    body: &Script,
    semantics: &GlobalWriteSemantics<'_>,
) -> Option<Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence>> {
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    let module = semantics.module?;
    if module.has_dynamic_trace
        || !module.traced_commands.is_empty()
        || module.has_dynamic_variable_trace
        || !module.traced_variables.is_empty()
    {
        return None;
    }
    let realm = semantics.logical_realm.as_ref()?;
    let operands =
        crate::registry_invocation::source_structure::OriginalLogicalVariableOperands::borrowing(
            module,
            semantics.registry,
            realm,
        );
    let commands = operands.body_commands(body)?;
    let mut aliases: HashMap<u64, SourceAliasBucket> = HashMap::new();
    let mut writes = Vec::new();
    for tokens in &commands {
        if crate::registry_invocation::original_logical_procedure_calls_for_module(
            tokens,
            module,
            semantics.registry,
        )
        .is_some()
        {
            continue;
        }
        let operation = operands.operation(tokens)?;
        let (_, declared) = semantics.logical_footprint(tokens)?;
        for alias in declared {
            let local = operands.alias_local_slot(tokens, &alias)?;
            if !logical_alias_targets_global(&alias, operation.dialect)? {
                continue;
            }
            let target = operands.alias_source_place(tokens, &alias)?;
            let hash = std::hash::BuildHasher::hash_one(aliases.hasher(), &local);
            let bucket = aliases.entry(hash).or_default();
            let position = bucket.iter().position(|(slot, _)| slot == &local);
            let targets = if let Some(position) = position {
                &mut bucket[position].1
            } else {
                bucket.push((local, Vec::new()));
                &mut bucket.last_mut().expect("inserted alias slot").1
            };
            insert_source_write(targets, target.clone());
            if alias.writes_value {
                insert_source_write(&mut writes, target);
            }
        }
    }
    for tokens in &commands {
        if crate::registry_invocation::original_logical_procedure_calls_for_module(
            tokens,
            module,
            semantics.registry,
        )
        .is_some()
        {
            continue;
        }
        let (projection, _) = semantics.logical_footprint(tokens)?;
        if projection.opaque_variable_frame {
            return None;
        }
        for (written, form) in operands.write_arguments(tokens)? {
            let place = operands.argument_source_place(tokens, written, form)?;
            match place.symbol().slot() {
                local @ SignatureSourceVariableSlot::Local { .. } => {
                    if let Some(bucket) =
                        aliases.get(&std::hash::BuildHasher::hash_one(aliases.hasher(), local))
                        && let Some((_, targets)) = bucket.iter().find(|(slot, _)| slot == local)
                    {
                        for target in targets {
                            insert_source_write(&mut writes, target.clone());
                        }
                    }
                }
                SignatureSourceVariableSlot::C { .. } | SignatureSourceVariableSlot::Jim(_) => {
                    insert_source_write(&mut writes, place);
                }
            }
        }
    }
    Some(writes)
}

fn own_logical_body_global_writes(
    body: &Script,
    semantics: &GlobalWriteSemantics<'_>,
) -> GlobalWriteInfo {
    let inventory = logical_body_source_writes(body, semantics);
    let Some(writes) = inventory else {
        return GlobalWriteInfo {
            opaque_global_frame: true,
            ..GlobalWriteInfo::default()
        };
    };
    let mut info = GlobalWriteInfo {
        source_writes: Some(writes),
        ..GlobalWriteInfo::default()
    };
    for place in info
        .source_writes
        .as_ref()
        .expect("original source inventory")
    {
        let Some(report) = place.symbol().checked_c_namespace_key() else {
            info.opaque_global_frame = true;
            continue;
        };
        insert_outer_name(&mut info.names, &report);
    }
    info
}

/// Pass 1: union every `global` / `variable` declaration in `body` into one
/// flag state (order-independent), *and* separately collect the
/// (local-alias-name → real-outer-name) map for `upvar #0` / `upvar 0` /
/// `namespace upvar` declarations — those alias a *different* local name to
/// the outer one, so a write to the local name must be attributed to the
/// outer name, not recorded under the local spelling.  `global`/`variable`
/// are identity aliases (the declared name *is* the outer name), so they
/// stay in the flag-state path.  Pass 2: walk `body` again collecting every
/// write-target name, resolving each through the alias map first and
/// falling back to the flag-state identity check.
fn own_body_global_writes(
    body: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
) -> GlobalWriteInfo {
    if semantics.logical_source() {
        let mut info = own_logical_body_global_writes(body, semantics);
        info.opaque_global_frame |=
            semantics
                .aliases
                .script_has_opaque_binding_effect(body, semantics.registry, namespace);
        return info;
    }
    let registry = semantics.registry;
    let aliases = semantics.aliases;
    let mut state = State::default();
    let mut renamed_aliases = OuterAliasProjection::default();
    let execution_namespace = ExecutionNamespace::exact(namespace);
    accumulate_state(
        body,
        &mut state,
        &mut renamed_aliases,
        semantics,
        &execution_namespace,
    );

    let mut info = GlobalWriteInfo::default();
    // A literal binding with an unknown implementation, or an unbounded
    // transition executed by this body itself, can reach an arbitrary
    // procedure that writes the global frame. Unrelated module-wide opacity
    // is runtime-provenance state, not a per-call variable effect.
    if aliases.script_has_opaque_binding_effect(body, registry, namespace)
        || (!semantics.logical_source() && script_invokes_unknown_binding(body, aliases, namespace))
    {
        info.opaque_global_frame = true;
    }
    collect_write_targets(
        body,
        &state,
        &renamed_aliases,
        semantics,
        &execution_namespace,
        &mut info,
    );
    collect_global_frame_effects(body, semantics, namespace, &mut info);
    info
}

fn script_invokes_unknown_binding(
    script: &Script,
    aliases: &ModuleCommandBindings,
    namespace: &str,
) -> bool {
    fn walk(
        script: &Script,
        aliases: &ModuleCommandBindings,
        namespace: &ExecutionNamespace,
        depth: u32,
    ) -> bool {
        if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            return true;
        }
        script.statements.iter().any(|stmt| {
            let own = match stmt {
                Statement::Call { command, .. } | Statement::Barrier { command, .. } => {
                    let Some(command_namespace) = namespace.for_head_context(command) else {
                        return true;
                    };
                    aliases.target_resolution_may_be_unknown(command, command_namespace.as_ref())
                }
                _ => false,
            };
            own || nested_execution_bodies(stmt, namespace)
                .into_iter()
                .any(|(body, body_namespace)| walk(body, aliases, &body_namespace, depth + 1))
        })
    }

    walk(script, aliases, &ExecutionNamespace::exact(namespace), 0)
}

/// Pass 3: the writes a proc performs by running a script **at
/// the global frame** — `uplevel #0 {…}` (lowered to [`Statement::UpFrame`]
/// with `absolute` set and shift `0`) and `uplevel #0 [list CMD …]` /
/// `uplevel #0 $body` (still a plain call/barrier statement).  These need no
/// `global`/`variable` declaration at all, so the flag-state passes above
/// cannot see them — before this pass, `proc setter {} {uplevel #0 {set x
/// 99}}` summarised as writing nothing and O102 forwarded the caller's stale
/// `x` straight across `setter` (tclsh 9.0.3/9.0.4: prints `99`, the
/// optimised program printed `5`).
///
/// A literal write target goes into [`GlobalWriteInfo::names`]; anything
/// unreadable (a dynamic script, an unresolvable constructed target, a
/// non-literal name) widens [`GlobalWriteInfo::opaque_global_frame`], which
/// the call site turns into an opaque barrier.  Frame targeting mirrors
/// [`super::upvar_info`]'s table exactly: only the **global** frame lands
/// here — caller-relative levels are the caller-frame summary's business,
/// and `uplevel 0` stays in the callee's own frame.
fn collect_global_frame_effects(
    script: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
    info: &mut GlobalWriteInfo,
) {
    collect_global_frame_effects_in_frame(script, semantics, namespace, FrameContext::Own, info);
}

/// Continue the global-frame pass under a known selected frame.  A nested
/// `uplevel 0` inherits its enclosing selected frame: inside `uplevel #0`, it
/// still writes globals, whereas inside `uplevel 1` it still writes the
/// caller.  Keeping that transform here prevents a nested [`Statement::UpFrame`]
/// from being silently treated as a body in the defining procedure's frame.
fn collect_global_frame_effects_in_frame(
    script: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
    frame: FrameContext,
    info: &mut GlobalWriteInfo,
) {
    use tcl_registry::frame_effect::FrameLevel;
    for stmt in &script.statements {
        if !stmt.is_executable_invocation() {
            continue;
        }
        match stmt {
            // A literal body runs in the frame its level selects from the
            // enclosing one: `uplevel 0` inside `uplevel #0` still writes
            // globals, inside `uplevel 1` still the caller. A body that may
            // run at the global frame (`#0`, or a level nothing static
            // places) writes its names there; any other frame is not
            // globally enumerable, but a `#0` nested in it still is.
            Statement::UpFrame {
                absolute,
                frame_shift,
                body,
                ..
            } => {
                let selected = frame.level(match u32::try_from(*frame_shift) {
                    Ok(shift) if *absolute => FrameLevel::Absolute(shift),
                    Ok(shift) => FrameLevel::Relative(shift),
                    Err(_) => FrameLevel::Dynamic,
                });
                if selected.may_be_global() {
                    record_global_upframe_body(body, semantics, info);
                    collect_global_frame_effects_in_frame(body, semantics, "::", selected, info);
                } else {
                    collect_global_frame_effects_in_frame(
                        body, semantics, namespace, selected, info,
                    );
                }
                continue;
            }
            Statement::Call { .. } | Statement::Barrier { .. } => record_selected_global_body(
                stmt,
                script.retained_source_tokens_for_statement(stmt),
                semantics,
                namespace,
                frame,
                info,
            ),
            _ => {}
        }
        let execution_namespace = crate::ir_helpers::ExecutionNamespace::exact(namespace);
        for (body, body_namespace) in
            crate::ir_helpers::nested_execution_bodies(stmt, &execution_namespace)
        {
            let crate::ir_helpers::ExecutionNamespace::Exact(body_namespace) = body_namespace
            else {
                info.opaque_global_frame = true;
                continue;
            };
            collect_global_frame_effects_in_frame(body, semantics, &body_namespace, frame, info);
        }
    }
}

fn record_selected_global_body(
    stmt: &Statement,
    tokens: Option<&crate::ir::CommandTokens>,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
    frame: FrameContext,
    info: &mut GlobalWriteInfo,
) {
    if semantics.logical_source() {
        record_original_logical_global_body(tokens, semantics, frame, info);
        return;
    }
    let registry = semantics.registry;
    let aliases = semantics.aliases;
    let execution_namespace = ExecutionNamespace::exact(namespace);
    for invocation in semantics.resolve_statement(stmt, namespace) {
        use crate::command_binding::{ResolvedFrameBody, ResolvedFrameBodySelection};
        let body = if semantics.standalone() {
            invocation.resolved_frame_body_standalone(registry, aliases, &execution_namespace)
        } else {
            invocation.resolved_frame_body_with_metadata_context(
                registry,
                aliases,
                &execution_namespace,
                semantics.metadata,
            )
        };
        let selected_global =
            |selection: ResolvedFrameBodySelection| frame.select(selection).may_be_global();
        match body {
            ResolvedFrameBody::Readable { source, selection } if selected_global(selection) => {
                if !record_literal_global_body(&source, semantics, info) {
                    info.opaque_global_frame = true;
                }
            }
            ResolvedFrameBody::Opaque { selection } if selected_global(selection) => {
                info.opaque_global_frame = true;
            }
            ResolvedFrameBody::NotApplicable
            | ResolvedFrameBody::KnownError
            | ResolvedFrameBody::Readable { .. }
            | ResolvedFrameBody::Opaque { .. } => {}
        }
    }
}

fn record_original_logical_global_body(
    tokens: Option<&crate::ir::CommandTokens>,
    semantics: &GlobalWriteSemantics<'_>,
    frame: FrameContext,
    info: &mut GlobalWriteInfo,
) {
    use tcl_registry::frame_effect::{FrameArgLayout, FrameArgumentResolution};
    let Some((tokens, module, metadata)) = tokens
        .zip(semantics.module)
        .zip(semantics.metadata)
        .map(|((tokens, module), metadata)| (tokens, module, metadata))
    else {
        info.opaque_global_frame = true;
        return;
    };
    let Some(invocation) =
        crate::registry_invocation::original_logical_frame_invocation_with_metadata_context(
            semantics.registry,
            metadata,
            tokens,
        )
    else {
        // Direct-call closure and variable footprints keep their independent
        // unknown-target obligations; this pass adds selected script effects.
        return;
    };
    let Some(layout) = invocation.layout().filter(|layout| {
        matches!(
            layout,
            FrameArgLayout::ScriptInCurrentFrame | FrameArgLayout::ScriptInSelectedFrame
        )
    }) else {
        return;
    };
    let (level, start) = match invocation.arguments() {
        Some(FrameArgumentResolution::Valid {
            level,
            level_word_len,
        }) => (level, invocation.offset() + level_word_len),
        Some(FrameArgumentResolution::Invalid) => return,
        Some(FrameArgumentResolution::Unknown) | None => {
            info.opaque_global_frame |=
                layout == FrameArgLayout::ScriptInSelectedFrame || frame.may_be_global();
            return;
        }
    };
    let selected = if layout == FrameArgLayout::ScriptInCurrentFrame {
        frame
    } else {
        frame.level(level)
    };
    let selected_global = selected.may_be_global();
    if !selected_global {
        return;
    }
    if invocation.count() != start + 1 {
        info.opaque_global_frame = true;
        return;
    }
    let Some(body) = invocation.source_body(module, semantics.registry, tokens, start) else {
        info.opaque_global_frame = true;
        return;
    };
    record_global_upframe_body(&body, semantics, info);
    collect_global_frame_effects_in_frame(&body, semantics, "::", selected, info);
}

fn record_global_upframe_body(
    body: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    info: &mut GlobalWriteInfo,
) {
    // The literal body was lowered before all later module command-binding
    // transitions were known. Re-project its writes through the closed binding
    // lattice so an alias prefix such as `interp alias {} put {} set x`
    // contributes `x` even when the source call's lower-time `defs` is empty.
    // The body runs at #0, so bare command heads resolve in the global namespace.
    let namespace = ExecutionNamespace::exact("::");
    let projection = project_script_writes(body, semantics, &namespace);
    // A name the body links to another global (`upvar 0 g local`, `global`,
    // `upvar #0`) writes that global too: tclsh 8.4 to 9.1 print 1 for
    // `proc p {} {uplevel #0 {upvar 0 g local; set local 1}}; set g 0; p; puts $g`.
    let mut aliases = OuterAliasProjection {
        frame_is_global: true,
        ..OuterAliasProjection::default()
    };
    accumulate_state(
        body,
        &mut State::default(),
        &mut aliases,
        semantics,
        &namespace,
    );
    let writes = !projection.literal_names.is_empty();
    for name in projection.literal_names {
        record_global_frame_write(&name, info);
        let root = literal_root(&name);
        for outer in aliases.targets.get(root).into_iter().flatten() {
            record_global_frame_write(outer, info);
        }
        info.opaque_global_frame |= aliases.opaque_locals.contains(root);
    }
    if writes {
        for outer in &aliases.dynamic_local_targets {
            record_global_frame_write(outer, info);
        }
        info.opaque_global_frame |= aliases.dynamic_local_opaque;
    }
    info.opaque_global_frame |= projection.opaque_variable_frame;
}

fn record_literal_global_body(
    source: &str,
    semantics: &GlobalWriteSemantics<'_>,
    info: &mut GlobalWriteInfo,
) -> bool {
    let registry = semantics.registry;
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, semantics.config);
    if let Some(input) = semantics.input {
        lowerer = lowerer
            .with_dialect(Some(input.unit_profile()))
            .with_resolved_analysis_input(input.clone());
        if let Some(entry) = crate::command_binding::SourceAnalysisEntry::for_logical_source(input)
        {
            lowerer.set_source_analysis_options(entry.options());
        }
    } else if semantics.standalone() {
        lowerer = lowerer.with_dialect(registry.profile());
    } else {
        return false;
    }
    let module = crate::lowering::lower_to_ir_with(lowerer, source);
    // An alias-baked `uplevel #0` reaches this path as an ordinary call, so
    // its body did not participate in the original module lowering. Project
    // the freshly lowered body through the original module's closed bindings:
    // a later `interp alias {} put {} set x` must be just as visible here as
    // it is for a directly lowered `Statement::UpFrame`.
    record_global_upframe_body(&module.top_level, semantics, info);
    true
}

/// Whether any statement in `script` (recursively) assigns through a
/// **dynamic** write target (`set $n 1`, `set ${tok}(k) 1`) — a write
/// [`crate::ssa::defs_of`] cannot name and therefore silently drops, which
/// a frame-effect summary must widen on instead.  Shared with
/// [`super::upvar_info`]'s inlined-`uplevel`-body scan, which has the
/// identical blind spot one frame nearer.
pub(super) fn script_has_dynamic_write_target(script: &Script) -> bool {
    script.statements.iter().any(|stmt| {
        let own = match stmt {
            Statement::AssignConst {
                name, name_braced, ..
            }
            | Statement::AssignExpr {
                name, name_braced, ..
            }
            | Statement::AssignValue {
                name, name_braced, ..
            }
            | Statement::Incr {
                name, name_braced, ..
            } => crate::ssa::is_dynamic_write_target(name, *name_braced),
            _ => false,
        };
        own || crate::ir_helpers::nested_bodies(stmt)
            .into_iter()
            .any(script_has_dynamic_write_target)
    })
}

/// Record a known global-frame destination under the shared observation purpose.
fn record_global_frame_write(name: &str, info: &mut GlobalWriteInfo) {
    insert_outer_name(&mut info.names, literal_root(name));
}

fn accumulate_state(
    script: &Script,
    state: &mut State,
    renamed_aliases: &mut OuterAliasProjection,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &ExecutionNamespace,
) {
    let registry = semantics.registry;
    for stmt in &script.statements {
        if let Some(command_namespace) = statement_command_namespace(stmt, namespace) {
            stmt_gen_with_metadata_context(stmt, state, registry, semantics.metadata);
            collect_renamed_outer_alias(
                stmt,
                script.retained_source_tokens_for_statement(stmt),
                renamed_aliases,
                semantics,
                command_namespace.as_ref(),
            );
        }
        if semantics.logical_source()
            && let Some(tokens) = script.retained_source_tokens_for_statement(stmt)
        {
            let (_, aliases) = original_nested_footprint(tokens, semantics);
            for alias in &aliases {
                collect_renamed_outer_alias_transition(alias, renamed_aliases, semantics);
            }
        }
        let embedded = if semantics.logical_source() && semantics.module.is_some() {
            crate::ir_helpers::EvaluatedCommandSubstitutions::default()
        } else {
            source_substitutions(stmt, semantics, namespace)
        };
        for words in embedded.all_commands() {
            let Some(head) = words
                .first()
                .and_then(crate::ir_helpers::CommandWord::literal)
            else {
                continue;
            };
            let Some(command_namespace) = namespace.for_head_context(head) else {
                continue;
            };
            for facts in semantics.resolve_words(words, command_namespace.as_ref()) {
                collect_renamed_outer_alias_facts(&facts, renamed_aliases, semantics);
            }
        }
        for (body, body_namespace) in nested_execution_bodies(stmt, namespace) {
            accumulate_state(body, state, renamed_aliases, semantics, &body_namespace);
        }
    }
}

/// Child words retain their own original lookup and complete source input;
/// the parent's text cannot select a descriptor for an extracted fragment.
fn original_nested_footprint(
    tokens: &crate::ir::CommandTokens,
    semantics: &GlobalWriteSemantics<'_>,
) -> (
    tcl_registry::VariableWriteProjection,
    Vec<tcl_registry::VariableCellAliasTransition>,
) {
    let mut writes = tcl_registry::VariableWriteProjection::default();
    let mut aliases = Vec::new();
    let Some(metadata) = semantics.metadata else {
        writes.opaque_variable_frame = true;
        return (writes, aliases);
    };
    let mut pending = vec![(tokens.clone(), 0_u32)];
    let mut seen = BTreeSet::new();
    while let Some((parent, depth)) = pending.pop() {
        if depth > 8 {
            writes.opaque_variable_frame = true;
            continue;
        }
        if depth != 0 {
            let Some(site) = parent
                .source_binding
                .as_ref()
                .and_then(|binding| binding.invocation_site())
            else {
                writes.opaque_variable_frame = true;
                continue;
            };
            if !seen.insert(site.offset) {
                continue;
            }
            if let Some((child_writes, child_aliases)) = semantics.logical_footprint(&parent) {
                merge_write_projection(&mut writes, child_writes);
                aliases.extend(child_aliases);
            } else {
                writes.opaque_variable_frame = true;
            }
            if let Some(module) = semantics.module {
                if let Some(bodies) = crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
                    module, semantics.registry, &parent,
                ) {
                    pending.extend(bodies.into_iter().map(|body| (body, depth + 1)));
                } else {
                    writes.opaque_variable_frame = true;
                }
            }
        }
        let Some(children) = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            &parent,
            semantics.config,
            semantics.registry,
            metadata,
        ) else {
            writes.opaque_variable_frame = true;
            continue;
        };
        for child in children {
            if let Some(child) = child.tokens {
                pending.push((child, depth + 1));
            } else {
                writes.opaque_variable_frame = true;
            }
        }
    }
    (writes, aliases)
}

fn source_substitutions(
    stmt: &Statement,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &ExecutionNamespace,
) -> crate::ir_helpers::EvaluatedCommandSubstitutions {
    let resolve = |head: &str| semantics.aliases.resolved_embedded_head(head, namespace);
    if semantics.standalone() {
        crate::ir_helpers::evaluated_command_substitutions_with_heads(
            stmt,
            semantics.registry,
            Some(&resolve),
        )
    } else {
        crate::ir_helpers::evaluated_command_substitutions_with_heads_and_metadata_context(
            stmt,
            semantics.registry,
            Some(&resolve),
            semantics.metadata,
            semantics.config,
        )
    }
}

/// Resolve the command namespace relevant to one statement in a selected
/// execution frame. Structural statements use the selected frame directly;
/// absolute call heads remain resolvable even when a caller-selected frame is
/// otherwise unknown.
fn statement_command_namespace<'a>(
    stmt: &Statement,
    namespace: &'a ExecutionNamespace,
) -> Option<std::borrow::Cow<'a, crate::command_binding::SourceNamespaceKey>> {
    match stmt {
        Statement::Call { command, .. } | Statement::Barrier { command, .. } => {
            namespace.for_head_context(command)
        }
        _ => namespace.for_head_context(""),
    }
}

/// Record the literal local-to-outer mapping declared by registry-owned
/// variable-cell alias transitions.  This includes renamed command spellings:
/// lowering retains their canonical target, and the descriptor — not the
/// source head — decides whether the invocation establishes an alias.
#[derive(Default)]
struct OuterAliasProjection {
    /// Every exact outer cell a literal local may denote on some path.
    targets: HashMap<String, BTreeSet<String>>,
    /// Literal locals whose outer cell cannot be enumerated.
    opaque_locals: BTreeSet<String>,
    /// Exact outer cells installed under a runtime-selected local name. Any
    /// concrete local write may therefore reach each of these cells.
    dynamic_local_targets: BTreeSet<String>,
    /// A runtime-selected local name aliases a non-enumerable outer cell.
    dynamic_local_opaque: bool,
    /// The script runs at the global frame, so an alias into its own frame
    /// (`upvar 0 g local`) is an alias of a global.
    frame_is_global: bool,
}

enum OuterAliasTarget {
    Exact(String),
    Opaque,
}

fn collect_renamed_outer_alias(
    stmt: &Statement,
    tokens: Option<&crate::ir::CommandTokens>,
    renamed_aliases: &mut OuterAliasProjection,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
) {
    if semantics.logical_source() {
        if let Some((_, aliases)) = tokens.and_then(|tokens| semantics.logical_footprint(tokens)) {
            for alias in &aliases {
                collect_renamed_outer_alias_transition(alias, renamed_aliases, semantics);
            }
        }
        return;
    }
    for invocation in semantics.resolve_statement(stmt, namespace) {
        collect_renamed_outer_alias_facts(&invocation.facts, renamed_aliases, semantics);
    }
}

fn selected_outer_alias_target(
    target: &tcl_registry::VariableAliasTarget,
    semantics: &GlobalWriteSemantics<'_>,
    frame_is_global: bool,
) -> Option<OuterAliasTarget> {
    let literal = |variable: &tcl_registry::TransitionSubject| {
        variable.literal().map_or(OuterAliasTarget::Opaque, |name| {
            OuterAliasTarget::Exact(literal_root(name).to_owned())
        })
    };
    match target {
        tcl_registry::VariableAliasTarget::Global { variable }
        | tcl_registry::VariableAliasTarget::CurrentNamespace { variable } => {
            Some(literal(variable))
        }
        tcl_registry::VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => Some(if namespace.literal().is_some() {
            literal(variable)
        } else {
            OuterAliasTarget::Opaque
        }),
        tcl_registry::VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
            let tcl_registry::CallerFrameSelection::Explicit(level) = frame else {
                return None;
            };
            let Some(level) = level.literal() else {
                return Some(OuterAliasTarget::Opaque);
            };
            let Some(dialect) = semantics
                .input
                .map(|input| tcl_native_model::InvocationDialect::of_profile(input.unit_profile()))
                .or_else(|| {
                    semantics
                        .metadata
                        .and_then(|metadata| metadata.context().environment.point())
                        .map(tcl_native_model::InvocationDialect::of_point)
                })
            else {
                return Some(OuterAliasTarget::Opaque);
            };
            match tcl_registry::frame_effect::FrameLevel::parse_native_bytes(
                level.as_bytes(),
                dialect,
            ) {
                Some(Some(level))
                    if level.is_global_frame() || (frame_is_global && level.is_current_frame()) =>
                {
                    Some(literal(variable))
                }
                // Invalid literal levels install no alias; other known levels
                // belong to the caller-frame summary instead of this one.
                Some(_) => None,
                // Counted NUL and unavailable numeric recipes are unresolved,
                // not evidence that an alias cannot have been installed.
                None => Some(OuterAliasTarget::Opaque),
            }
        }
    }
}

fn collect_renamed_outer_alias_facts(
    facts: &tcl_registry::InvocationFacts,
    renamed_aliases: &mut OuterAliasProjection,
    semantics: &GlobalWriteSemantics<'_>,
) {
    let Some(transitions) = facts.state_transitions.declared() else {
        return;
    };
    for fact in transitions.facts() {
        let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
            continue;
        };
        collect_renamed_outer_alias_transition(alias, renamed_aliases, semantics);
    }
}

fn collect_renamed_outer_alias_transition(
    alias: &tcl_registry::VariableCellAliasTransition,
    renamed_aliases: &mut OuterAliasProjection,
    semantics: &GlobalWriteSemantics<'_>,
) {
    let Some(outer) =
        selected_outer_alias_target(&alias.target, semantics, renamed_aliases.frame_is_global)
    else {
        return;
    };
    match (alias.local.literal(), outer) {
        (Some(local), OuterAliasTarget::Exact(outer)) => {
            let local = literal_root(local);
            renamed_aliases
                .targets
                .entry(local.to_owned())
                .or_default()
                .insert(outer);
        }
        (Some(local), OuterAliasTarget::Opaque) => {
            let local = literal_root(local);
            renamed_aliases.opaque_locals.insert(local.to_owned());
        }
        (None, OuterAliasTarget::Exact(outer)) => {
            renamed_aliases.dynamic_local_targets.insert(outer);
        }
        (None, OuterAliasTarget::Opaque) => renamed_aliases.dynamic_local_opaque = true,
    }
}

fn collect_write_targets(
    script: &Script,
    state: &State,
    renamed_aliases: &OuterAliasProjection,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &ExecutionNamespace,
    info: &mut GlobalWriteInfo,
) {
    for stmt in &script.statements {
        if statement_command_namespace(stmt, namespace).is_some() {
            let (targets, opaque) = own_write_targets(script, stmt, semantics, namespace);
            info.opaque_global_frame |= opaque;
            for target in targets {
                let target = literal_root(&target);
                if let Some(outers) = renamed_aliases.targets.get(target) {
                    for outer in outers {
                        insert_outer_name(&mut info.names, outer);
                    }
                }
                if renamed_aliases.opaque_locals.contains(target) {
                    info.opaque_global_frame = true;
                }
                for outer in &renamed_aliases.dynamic_local_targets {
                    insert_outer_name(&mut info.names, outer);
                }
                info.opaque_global_frame |= renamed_aliases.dynamic_local_opaque;
                // A `::`-qualified target is outer-scope by its spelling
                // alone, with no `global`, `variable` or `upvar` in the body:
                // `set y [incr ::hits]` writes the global `hits` (#2214).
                if !renamed_aliases.targets.contains_key(target)
                    && !renamed_aliases.opaque_locals.contains(target)
                    && (tcl_syntax::naming::is_absolute(target)
                        || state.get(target).is_some_and(|f| f.writes_outer_scope()))
                {
                    insert_outer_name(&mut info.names, target);
                }
            }
        } else if matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. }) {
            // A relative command in a runtime-selected caller frame may reach
            // a procedure with arbitrary global effects.
            info.opaque_global_frame = true;
        }
        for (body, body_namespace) in nested_execution_bodies(stmt, namespace) {
            collect_write_targets(
                body,
                state,
                renamed_aliases,
                semantics,
                &body_namespace,
                info,
            );
        }
    }
}

/// Record `name` as an outer-scope name the procedure writes. A
/// `::`-qualified name also records the spelling a caller at the global
/// frame reads the same variable by — `::hits` is `hits` there — since a
/// call site applies the summary's names as its own definitions, and a
/// definition of `::hits` alone leaves the caller's `hits` forwarding a
/// stale constant across the call.
///
/// A rooted spelling is first the naming owner's qualified name, so a colon
/// run (`global ::::g`) records the same global as `global ::g`.
fn insert_outer_name(names: &mut BTreeSet<String>, name: &str) {
    let qualified = tcl_syntax::naming::is_absolute_bytes(name.as_bytes())
        .then(|| tcl_syntax::naming::normalise_qualified_name(name));
    crate::command_binding::trace_registrations::record_possible_write_name(
        names,
        qualified.as_deref().unwrap_or(name),
        crate::command_binding::trace_registrations::WriteNamePolicy::OuterScope,
    );
}

/// The variable name(s) `stmt` itself directly writes — its own `name`
/// field for the direct-assignment statement kinds, `Call::defs` for a
/// Possible value destinations of the structured statement and its evaluated
/// substitutions. Generic calls use the current selected roles and original
/// alias prefix, rather than stale lower-time Call definitions. The Registry
/// distinguishes cell aliasing and trace installation from value writes.
fn own_write_targets(
    script: &Script,
    stmt: &Statement,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &ExecutionNamespace,
) -> (Vec<String>, bool) {
    let structural = crate::ir_helpers::structural_variable_write_projection(stmt);
    let mut out = structural.literal_names;
    let Some(command_namespace) = statement_command_namespace(stmt, namespace) else {
        return (out, true);
    };
    let embedded = if semantics.logical_source() && semantics.module.is_some() {
        crate::ir_helpers::EvaluatedCommandSubstitutions::default()
    } else {
        source_substitutions(stmt, semantics, namespace)
    };
    let direct = semantics.statement_writes(
        stmt,
        script.retained_source_tokens_for_statement(stmt),
        command_namespace.as_ref(),
    );
    // Variable writes count from an in-frame expression word too: `set y
    // [expr {[incr ::hits]}]` writes the global exactly as `[incr ::hits]` in
    // a bare word does.
    let mut opaque =
        structural.opaque_variable_frame || direct.opaque_variable_frame || embedded.opaque;
    out.extend(direct.literal_names);
    // What the callees write in this frame through their caller-frame
    // summaries is this statement's write, resolved below as a direct one:
    // `bump ::g`, `upvar #0 g x; bump x` and `global g; bump g` all write the
    // global `g` (#2410). A computed name a callee writes is no outer write
    // this summary can name; the caller-frame summary keeps it opaque there.
    if let Some(tokens) = script.retained_source_tokens_for_statement(stmt) {
        if stmt.is_executable_invocation() {
            out.extend(semantics.callee_caller_frame_writes(tokens).0);
        }
        if let Some(owners) = semantics.caller_frames.map(|frames| frames.owners) {
            for child in owners
                .children(tokens, semantics.registry)
                .unwrap_or_default()
            {
                out.extend(semantics.callee_caller_frame_writes(&child).0);
            }
        }
    }
    if semantics.logical_source()
        && let Some(tokens) = script.retained_source_tokens_for_statement(stmt)
    {
        let (nested, _) = original_nested_footprint(tokens, semantics);
        out.extend(nested.literal_names);
        opaque |= nested.opaque_variable_frame;
    }
    for words in embedded.all_commands() {
        let Some(holder) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
            .and_then(|head| namespace.for_head_context(head))
        else {
            opaque = true;
            continue;
        };
        let projection = semantics.command_writes(words, holder.as_ref());
        out.extend(projection.literal_names);
        opaque |= projection.opaque_variable_frame;
    }
    (out, opaque)
}

/// Conditional source write footprint under exact availability and grammar.
/// Missing or foreign metadata stays opaque; this grants no frame entry/write.
pub(super) fn script_value_write_projection_with_metadata_context(
    script: &Script,
    registry: &tcl_registry::CommandRegistry,
    aliases: &ModuleCommandBindings,
    namespace: &(impl crate::command_binding::NamespaceKeyQuery + ?Sized),
    metadata: Option<InvocationMetadataContext<'_>>,
    config: tcl_lexer::LexerConfig,
) -> tcl_registry::VariableWriteProjection {
    let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
        return tcl_registry::VariableWriteProjection {
            opaque_variable_frame: true,
            ..tcl_registry::VariableWriteProjection::default()
        };
    };
    project_script_writes(
        script,
        &GlobalWriteSemantics {
            module: None,
            logical_realm: None,
            caller_frames: None,
            registry,
            aliases,
            selection: if metadata.is_standalone() {
                InvocationMetadataInput::Standalone
            } else {
                InvocationMetadataInput::SuppliedSource(metadata.source_analysis_input())
            },
            metadata: Some(metadata),
            config,
            input: metadata.source_analysis_input(),
        },
        &ExecutionNamespace::SourceContext(namespace.namespace_key().into_owned()),
    )
}

fn project_script_writes(
    script: &Script,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &ExecutionNamespace,
) -> tcl_registry::VariableWriteProjection {
    fn walk(
        script: &Script,
        semantics: &GlobalWriteSemantics<'_>,
        namespace: &ExecutionNamespace,
        names: &mut BTreeSet<String>,
        opaque: &mut bool,
        depth: u32,
    ) {
        if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            *opaque = true;
            return;
        }
        for stmt in &script.statements {
            let (targets, stmt_opaque) = own_write_targets(script, stmt, semantics, namespace);
            names.extend(targets);
            *opaque |= stmt_opaque;
            // A nested UpFrame explicitly selects a different variable
            // frame, so its body is not a write of the script's own frame.
            // The enclosing global/caller-frame collectors classify that
            // shifted effect separately.
            if matches!(stmt, Statement::UpFrame { .. }) {
                continue;
            }
            let execution_namespace = namespace;
            for (body, body_namespace) in
                crate::ir_helpers::nested_execution_bodies(stmt, execution_namespace)
            {
                if matches!(body_namespace, ExecutionNamespace::RuntimeSelected) {
                    *opaque = true;
                    continue;
                }
                walk(body, semantics, &body_namespace, names, opaque, depth + 1);
            }
        }
    }

    let mut names = BTreeSet::new();
    let mut opaque = false;
    walk(script, semantics, namespace, &mut names, &mut opaque, 0);
    tcl_registry::VariableWriteProjection {
        literal_names: names.into_iter().collect(),
        read_before_write_names: Vec::new(),
        opaque_variable_frame: opaque,
    }
}

/// Original issued callees in direct statements, substitutions and nested bodies.
/// Incomplete source receipts leave an opaque edge in the summary graph.
pub(super) fn direct_call_targets(
    body: &Script,
    owners: &super::OriginalProcedureSummaryOwners,
    registry: &tcl_registry::CommandRegistry,
) -> (BTreeSet<crate::ir::ProcedureId>, bool) {
    let mut calls = BTreeSet::new();
    let mut opaque = false;
    let mut pending = vec![body];
    while let Some(script) = pending.pop() {
        for statement in &script.statements {
            pending.extend(statement.child_scripts());
            let Some(tokens) = script.retained_source_tokens_for_statement(statement) else {
                if statement.is_executable_invocation() {
                    opaque = true;
                }
                continue;
            };
            if statement.is_executable_invocation() {
                match owners.may_calls(tokens, registry) {
                    Some(selected) => {
                        calls.extend(selected.bodies);
                        opaque |= selected.unknown_alternatives;
                    }
                    None => opaque = true,
                }
            }
            match owners.children(tokens, registry) {
                Some(children) => {
                    for child in children {
                        match owners.may_calls(&child, registry) {
                            Some(selected) => {
                                calls.extend(selected.bodies);
                                opaque |= selected.unknown_alternatives;
                            }
                            None => opaque = true,
                        }
                    }
                }
                None if tokens
                    .words()
                    .iter()
                    .any(|word| word.legacy_text().contains('[')) =>
                {
                    opaque = true;
                }
                None => {}
            }
        }
    }
    (calls, opaque)
}

#[cfg(test)]
mod tests;
