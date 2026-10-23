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
pub(crate) use source_may::OriginalSourceCallMayWrites;

use std::collections::{BTreeSet, HashMap};

use crate::command_binding::ModuleCommandBindings;
use crate::ir::{Module, Script, Statement};
use crate::ir_helpers::{ExecutionNamespace, nested_execution_bodies};
use crate::registry_invocation::{InvocationMetadataContext, InvocationMetadataInput};
use crate::var_observability::{State, stmt_gen_with_metadata_context};

/// Exact borrowed input for conditional global/caller-frame source summaries.
struct GlobalWriteSemantics<'a> {
    module: Option<&'a Module>,
    registry: &'a tcl_registry::CommandRegistry,
    aliases: &'a ModuleCommandBindings,
    selection: InvocationMetadataInput<'a>,
    metadata: Option<InvocationMetadataContext<'a>>,
    config: tcl_lexer::LexerConfig,
    input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    logical_realm: Option<crate::realm::CommandBindingRealm>,
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
                .map(|(writes, _)| writes)
                .unwrap_or(tcl_registry::VariableWriteProjection {
                    opaque_variable_frame: true,
                    ..Default::default()
                });
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
        })
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
    let bindings = ModuleCommandBindings::analyse(module, registry);
    let semantics = GlobalWriteSemantics::from_module(module, registry, &bindings);
    let owners = super::OriginalProcedureSummaryOwners::for_module(module, registry);
    let mut rows = HashMap::new();
    let mut edges = HashMap::new();
    for (id, procedure) in module.original_source_procedure_bodies() {
        let namespace = procedure
            .body
            .namespace_context
            .as_deref()
            .and_then(crate::command_binding::SourceNamespaceKey::advisory_key);
        let mut info = match (semantics.as_ref(), namespace) {
            (Some(semantics), Some(namespace))
                if module.procedure_ids.owns(*id)
                    && module
                        .retained_source_bindings
                        .as_ref()
                        .is_some_and(|owner| {
                            owner.matches_original_procedure_summary(*id, procedure)
                        }) =>
            {
                own_body_global_writes(&procedure.body, semantics, &namespace)
            }
            _ => GlobalWriteInfo {
                opaque_global_frame: true,
                ..GlobalWriteInfo::default()
            },
        };
        let (calls, opaque) = direct_call_targets(&procedure.body, &owners, registry);
        info.opaque_global_frame |= opaque;
        edges.insert(*id, calls);
        rows.insert(*id, info);
    }
    loop {
        let prior = rows.clone();
        for (id, callees) in &edges {
            let row = rows.get_mut(id).expect("retained body row");
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
    metadata: Option<InvocationMetadataContext<'a>>,
    config: tcl_lexer::LexerConfig,
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

fn insert_source_write(
    writes: &mut Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence>,
    place: crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
) {
    if !writes.iter().any(|held| held.symbol() == place.symbol()) {
        writes.push(place);
    }
}

fn logical_alias_targets_global(
    alias: &tcl_registry::VariableCellAliasTransition,
    dialect: Option<tcl_registry::InvocationDialect>,
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
    collect_global_frame_effects_in_frame(script, semantics, namespace, false, info);
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
    in_global_frame: bool,
    info: &mut GlobalWriteInfo,
) {
    for stmt in &script.statements {
        if !stmt.is_executable_invocation() {
            continue;
        }
        match stmt {
            Statement::UpFrame {
                absolute: true,
                frame_shift: 0,
                body,
                ..
            } => {
                record_global_upframe_body(body, semantics, info);
                collect_global_frame_effects_in_frame(body, semantics, "::", true, info);
                continue;
            }
            // Relative level zero preserves the selected frame.  It is a
            // global-frame script only when its enclosing literal body had
            // already selected `#0`.
            Statement::UpFrame {
                absolute: false,
                frame_shift: 0,
                body,
                ..
            } if in_global_frame => {
                record_global_upframe_body(body, semantics, info);
                collect_global_frame_effects_in_frame(body, semantics, "::", true, info);
                continue;
            }
            Statement::UpFrame { body, .. } => {
                // A different selected frame is not globally enumerable, but
                // an absolute `#0` nested inside it still is; keep walking to
                // find that explicit reset without attributing ordinary body
                // writes to the global frame.
                collect_global_frame_effects_in_frame(body, semantics, namespace, false, info);
                continue;
            }
            Statement::Call { .. } | Statement::Barrier { .. } => record_selected_global_body(
                stmt,
                script.retained_source_tokens_for_statement(stmt),
                semantics,
                namespace,
                in_global_frame,
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
            collect_global_frame_effects_in_frame(
                body,
                semantics,
                &body_namespace,
                in_global_frame,
                info,
            );
        }
    }
}

fn record_selected_global_body(
    stmt: &Statement,
    tokens: Option<&crate::ir::CommandTokens>,
    semantics: &GlobalWriteSemantics<'_>,
    namespace: &str,
    in_global_frame: bool,
    info: &mut GlobalWriteInfo,
) {
    use tcl_registry::frame_effect::FrameLevel;
    if semantics.logical_source() {
        record_original_logical_global_body(tokens, semantics, in_global_frame, info);
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
        let selected_global = |selection| match selection {
            ResolvedFrameBodySelection::Current
            | ResolvedFrameBodySelection::Selected(FrameLevel::Relative(0)) => in_global_frame,
            ResolvedFrameBodySelection::Selected(FrameLevel::Absolute(0)) => true,
            ResolvedFrameBodySelection::Selected(
                FrameLevel::Relative(_) | FrameLevel::Absolute(_) | FrameLevel::Dynamic,
            ) => false,
        };
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
    in_global_frame: bool,
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
                layout == FrameArgLayout::ScriptInSelectedFrame || in_global_frame;
            return;
        }
    };
    let selected_global = if layout == FrameArgLayout::ScriptInCurrentFrame {
        in_global_frame
    } else {
        level.is_global_frame() || (level.is_current_frame() && in_global_frame)
    };
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
    collect_global_frame_effects_in_frame(&body, semantics, "::", true, info);
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
    let projection = project_script_writes(body, semantics, &ExecutionNamespace::exact("::"));
    for name in projection.literal_names {
        record_global_frame_write(&name, info);
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
    let projection = project_script_writes(
        &module.top_level,
        semantics,
        &ExecutionNamespace::exact("::"),
    );
    for name in projection.literal_names {
        record_global_frame_write(&name, info);
    }
    info.opaque_global_frame |= projection.opaque_variable_frame;
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
    crate::variable_observation_names::record_variable_write_name(
        &mut info.names,
        literal_root(name),
        crate::variable_observation_names::VariableWriteObservationPurpose::OuterScope,
    );
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
                .map(|input| tcl_registry::InvocationDialect::of_profile(input.unit_profile()))
                .or_else(|| {
                    semantics
                        .metadata
                        .and_then(|metadata| metadata.context().environment.point())
                        .map(tcl_registry::InvocationDialect::of_point)
                })
            else {
                return Some(OuterAliasTarget::Opaque);
            };
            match tcl_registry::frame_effect::FrameLevel::parse_native_bytes(
                level.as_bytes(),
                dialect,
            ) {
                Some(Some(level)) if level.is_global_frame() => Some(literal(variable)),
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
    let Some(outer) = selected_outer_alias_target(&alias.target, semantics) else {
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
                    && (target.starts_with("::")
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
fn insert_outer_name(names: &mut BTreeSet<String>, name: &str) {
    crate::variable_observation_names::record_variable_write_name(
        names,
        name,
        crate::variable_observation_names::VariableWriteObservationPurpose::OuterScope,
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
mod tests {
    use super::*;
    use tcl_registry::{ArgRole, CommandRegistry, CommandSpec, StateTransitionDescriptor};
    fn summary_id(module: &Module, label: &str) -> crate::ir::ProcedureId {
        let mut matching: Vec<_> = module
            .original_source_procedure_bodies()
            .filter(|(_, procedure)| {
                procedure.qualified_name == label
                    || procedure.name == label
                    || tcl_syntax::naming::key_tail(&procedure.name) == label
            })
            .map(|(id, _)| *id)
            .collect();
        if matching.is_empty() {
            let registry = module.resolved_registry();
            let owners = super::super::OriginalProcedureSummaryOwners::for_module(module, registry);
            for script in std::iter::once(&module.top_level).chain(
                module
                    .original_source_procedure_bodies()
                    .map(|(_, procedure)| &procedure.body),
            ) {
                for statement in &script.statements {
                    let Some(tokens) = script.retained_source_tokens_for_statement(statement)
                    else {
                        continue;
                    };
                    if super::super::summary_candidates::original_head(tokens).as_deref()
                        == Some(label)
                    {
                        for (id, _) in owners
                            .calls(tokens, registry)
                            .expect("original fixture call is closed")
                        {
                            if !matching.contains(&id) {
                                matching.push(id);
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(
            matching.len(),
            1,
            "authored fixture summary {label:?} must identify one body"
        );
        matching[0]
    }

    fn module(src: &str) -> Module {
        module_with_registry(src, &CommandRegistry::build_default())
    }

    fn module_with_registry(src: &str, registry: &CommandRegistry) -> Module {
        let context = tcl_registry::model::ingress::static_context_for("tcl")
            .with_command_store(registry.snapshot().shared_registry());
        let profile = tcl_registry::model::ingress::static_context_for("tcl")
            .commands()
            .profile()
            .unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(context),
            config,
        );
        let lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_dialect(Some(profile))
            .with_resolved_analysis_input(input);
        crate::lowering::lower_to_ir_with(lowerer, src)
    }

    #[test]
    fn original_logical_outer_writes_retain_typed_alias_targets_and_incomplete_coverage() {
        let original =
            module("proc left {} {global ::g; set g 1}\nproc right {} {global ::::g; set g 2}");
        let rows = detect_global_write_procs(&original);
        let left = &rows[&summary_id(&original, "::left")];
        let right = &rows[&summary_id(&original, "::right")];
        let [left] = left
            .original_source_writes()
            .expect("complete first source footprint")
        else {
            panic!("one original alias target");
        };
        let [right] = right
            .original_source_writes()
            .expect("complete second source footprint")
        else {
            panic!("one original alias target");
        };
        assert_eq!(left.symbol(), right.symbol());
        assert_ne!(left.span(), right.span());
        let mut changed = original.clone();
        changed.source = tcl_lexer::SourceImage::document(
            "proc left {} {global ::other; set other 1}\nproc right {} {global ::::g; set g 2}",
        );
        assert!(
            detect_global_write_procs(&changed)
                .values()
                .all(|row| row.original_source_writes().is_none())
        );
        let mut observed = original.clone();
        observed.traced_commands.insert("::set".to_owned());
        assert!(
            detect_global_write_procs(&observed)
                .values()
                .all(|row| row.original_source_writes().is_none())
        );
        let unknown = module("proc p {} {global g; set g 1; unknown_call}");
        let rows = detect_global_write_procs(&unknown);
        let row = &rows[&summary_id(&unknown, "::p")];
        assert!(row.opaque_global_frame);
        assert!(
            row.original_source_writes().is_none(),
            "a known write cannot close an unknown call remainder"
        );
        let dynamic = module("proc p {name} {global g; set $name 1}");
        let rows = detect_global_write_procs(&dynamic);
        assert!(
            rows[&summary_id(&dynamic, "::p")]
                .original_source_writes()
                .is_none()
        );
    }

    #[test]
    fn original_global_source_write_keeps_empty_scalar_and_array_root() {
        // experiments/name-handling/results/source-empty-write-projection-20261014/oracles/observations.json
        for body in ["set {} SCALAR", "set {(key)} ARRAY"] {
            let module = module(&format!("proc p {{}} {{uplevel #0 {{{body}}}}}"));
            assert!(module.source_entry.native_entry.is_none());
            let summaries = detect_global_write_procs(&module);
            assert!(summaries[&summary_id(&module, "::p")].names.contains(""));
        }
        let dynamic = module("proc p {name} {uplevel #0 {set $name VALUE}}");
        assert!(
            detect_global_write_procs(&dynamic)[&summary_id(&dynamic, "::p")].opaque_global_frame
        );
    }

    #[test]
    fn original_immediate_body_inventory_keeps_protected_children_and_refusals() {
        let module = module("catch {set ::hits VALUE}");
        let registry = module.resolved_registry();
        let parent = module
            .top_level
            .retained_source_tokens_for_statement(module.top_level.statements.first().unwrap())
            .unwrap();
        let children = crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
            &module, registry, parent,
        ).expect("complete original protected body");
        assert_eq!(children.len(), 1);
        assert!(module.source_entry.native_entry.is_none());
        assert_eq!(
            crate::registry_invocation::original_logical_variable_footprint_with_metadata_context(
                registry,
                crate::registry_invocation::InvocationMetadataContext::for_module(
                    registry, &module
                )
                .unwrap(),
                &children[0]
            )
            .unwrap()
            .0
            .literal_names,
            vec!["::hits"]
        );
        let mut stale = parent.clone();
        stale.argv_texts[1] = "set ::other VALUE".to_owned();
        assert!(crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
            &module, registry, &stale,
        ).is_none());
        let mut changed = module.clone();
        changed.source_metadata_input = None;
        assert!(crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
            &changed, registry, parent,
        ).is_none());
        let dynamic = self::module("catch $script");
        let parent = dynamic
            .top_level
            .retained_source_tokens_for_statement(dynamic.top_level.statements.first().unwrap())
            .unwrap();
        assert!(crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
            &dynamic, dynamic.resolved_registry(), parent,
        ).is_none());
    }

    #[test]
    fn logical_source_child_effects_keep_readonly_ownership_and_refuse_stale_modules() {
        let module = module("catch {set ::hits VALUE}");
        let registry = module.resolved_registry();
        let input = module.source_metadata_input.as_ref().unwrap();
        let parent = module
            .top_level
            .retained_source_tokens_for_statement(&module.top_level.statements[0])
            .unwrap();
        let children = crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(&module, registry, parent).unwrap();
        let child = &children[0];
        let binding = child.source_binding.as_ref().unwrap();
        assert!(binding.lookup_state.is_none());
        assert!(
            binding
                .original_invocation_metadata_for_module(child, &module, registry)
                .is_none()
        );
        let words = binding
            .original_logical_source_words(child, input, registry)
            .unwrap();
        assert!(matches!(
            words.source(),
            crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
                _
            )
        ));
        assert!(!words.operands_preserve_source_lookup());
        assert_eq!(
            words
                .with_source_schema(
                    input.borrowed_context_registry(),
                    tcl_registry::CommandRegistry::variable_write_projection_for_selected_source
                )
                .unwrap()
                .literal_names,
            vec!["::hits"]
        );
        let mut stale = child.clone();
        stale.argv_texts[1] = "::other".to_owned();
        assert!(crate::registry_invocation::source_structure::original_logical_registry_words_for_module(&module, registry, &stale).is_none());
        let mut missing_carrier = child.clone();
        missing_carrier.source_binding = Some(Default::default());
        assert!(crate::registry_invocation::source_structure::original_logical_registry_words_for_module(&module, registry, &missing_carrier).is_none());
        let mut missing_words = child.clone();
        missing_words.source_binding = None;
        assert!(crate::registry_invocation::source_structure::original_logical_registry_words_for_module(&module, registry, &missing_words).is_none());
        for facet in 0..5 {
            let mut stale = module.clone();
            match facet {
                0 => stale.source_metadata_input = None,
                1 => stale.source = tcl_lexer::SourceImage::document("catch {set ::other VALUE}"),
                2 => stale.retained_source_bindings = None,
                3 => stale.lexer_config.strict_quoting = !stale.lexer_config.strict_quoting,
                4 => stale.source_entry.unknown_entry = true,
                _ => unreachable!(),
            }
            assert!(crate::registry_invocation::source_structure::original_logical_registry_words_for_module(&stale, registry, child).is_none(), "changed facet {facet}");
        }
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let mut stronger = module.clone();
        stronger.source_entry.native_entry = Some(std::sync::Arc::new(entry));
        assert!(crate::registry_invocation::source_structure::original_logical_registry_words_for_module(&stronger, registry, child).is_none());
    }

    #[test]
    fn protected_child_substitutions_retain_their_own_source_effect_carriers() {
        let module = module("catch {set ::hits [incr ::other]}");
        let registry = module.resolved_registry();
        let metadata =
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, &module)
                .unwrap();
        let parent = module
            .top_level
            .retained_source_tokens_for_statement(&module.top_level.statements[0])
            .unwrap();
        let body = crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(&module, registry, parent).unwrap();
        let children = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            &body[0],
            module.native_lexer_config(),
            registry,
            metadata,
        )
        .unwrap();
        assert_eq!(children.len(), 1);
        let child = children[0].tokens.as_ref().unwrap();
        let (writes, _) =
            crate::registry_invocation::original_logical_variable_footprint_with_metadata_context(
                registry, metadata, child,
            )
            .unwrap();
        assert_eq!(writes.literal_names, vec!["::other"]);
        assert!(!writes.opaque_variable_frame);
        assert!(crate::registry_invocation::source_structure::original_logical_registry_words_for_module(&module, registry, child).is_some());
        let mut stale = child.clone();
        stale.argv_texts[1] = "::unowned".to_owned();
        assert!(crate::registry_invocation::source_structure::original_logical_registry_words_for_module(&module, registry, &stale).is_none());
    }

    #[test]
    fn original_substitution_protected_bodies_share_global_write_coverage() {
        // experiments/name-handling/results/immediate-protected-body-writes-20261014/oracles/observations.json
        for body in [
            "catch {incr ::hits}",
            "catch {catch {incr ::hits}}",
            "try {incr ::hits} finally {incr ::done}",
        ] {
            let module = module(&format!("proc p {{}} {{set result [{body}]}}"));
            let info = detect_global_write_procs(&module);
            assert!(
                info[&summary_id(&module, "::p")].names.contains("hits"),
                "{body}"
            );
        }
    }

    #[test]
    fn original_global_alias_writes_keep_empty_local_and_remote_names() {
        // experiments/name-handling/results/global-empty-alias-cell-20261014/oracles/observations.json
        for (body, target) in [
            ("upvar #0 {} local; set local VALUE", ""),
            ("upvar #0 target {}; set {} VALUE", "target"),
            ("upvar #0 {} {}; set {} VALUE", ""),
        ] {
            let module = module(&format!("proc p {{}} {{{body}}}"));
            let summaries = detect_global_write_procs(&module);
            assert!(
                summaries[&summary_id(&module, "::p")]
                    .names
                    .contains(target),
                "{body}"
            );
        }
        let module = module("proc p {name} {upvar #0 {} $name; set ordinary VALUE}");
        let summaries = detect_global_write_procs(&module);
        assert!(summaries[&summary_id(&module, "::p")].names.contains(""));
    }

    #[test]
    fn standalone_frame_write_advice_keeps_unprofiled_catalogue_separate_from_missing_input() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Explicit catalogue advice proves no actual source input, frame or write.
        let registry = CommandRegistry::build_default();
        assert!(registry.profile().is_none());
        let module = crate::lowering::lower_to_ir("set ::standalone VALUE", &registry);
        let bindings = ModuleCommandBindings::analyse(&module, &registry);
        let procedures = HashMap::new();
        let compatible = own_frame_global_writes_standalone(
            &module.top_level,
            &registry,
            &bindings,
            "::",
            &procedures,
            &super::super::OriginalProcedureSummaryOwners::for_module(&module, &registry),
            module.lexer_config,
        );
        assert!(compatible.names.contains("::standalone"));
        assert!(compatible.names.contains("standalone"));
        let missing = own_frame_global_writes_with_metadata_context(
            &module.top_level,
            &registry,
            &bindings,
            "::",
            &procedures,
            &super::super::OriginalProcedureSummaryOwners::for_module(&module, &registry),
            None,
            module.lexer_config,
        );
        assert!(missing.opaque_global_frame);
        assert!(missing.names.is_empty());
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let refused = own_frame_global_writes_with_metadata_context(
            &module.top_level,
            &registry,
            &bindings,
            "::",
            &procedures,
            &super::super::OriginalProcedureSummaryOwners::for_module(&module, &registry),
            Some(foreign.as_ref().into()),
            module.lexer_config,
        );
        assert!(refused.opaque_global_frame);
        assert!(refused.names.is_empty());
    }

    #[test]
    fn supplied_global_write_metadata_keeps_alias_arguments_and_literal_names() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Conditional source summaries establish no Native frame/link/current cell.
        let registry = CommandRegistry::build_default();
        for (source, expected) in [
            (
                "interp alias {} put {} set {$scalar(open}; proc p {} {global {$scalar(open}; put VALUE}",
                "$scalar(open",
            ),
            (
                "rename set moved; interp alias {} put {} moved {café}; proc p {} {global café; set result [put VALUE]}",
                "café",
            ),
            (
                "interp alias {} bind {} upvar #0 {$outer}; proc p {} {bind {$local}; set {$local} VALUE}",
                "$outer",
            ),
            (
                "proc p {} {uplevel #0 {set {scalar(open} VALUE}}",
                "scalar(open",
            ),
            (
                "interp alias {} at-root {} uplevel #0; proc p {} {at-root {set {café(open} VALUE}}",
                "café(open",
            ),
        ] {
            let module = module_with_registry(source, &registry);
            let information = detect_global_write_procs_with_registry(&module, &registry);
            assert!(
                information[&summary_id(&module, "::p")]
                    .names
                    .contains(expected),
                "{source}: {information:?}"
            );
        }
        for source in [
            "proc set args {}; interp alias {} put {} set hidden; proc p {} {put VALUE}",
            "interp alias {} put {} set hidden; rename set moved; proc set args {}; proc p {} {put VALUE}",
        ] {
            let module = module_with_registry(source, &registry);
            assert!(
                !detect_global_write_procs_with_registry(&module, &registry)
                    [&summary_id(&module, "::p")]
                    .names
                    .contains("hidden")
            );
        }
    }

    #[test]
    fn supplied_global_write_summary_refuses_missing_foreign_and_stale_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let registry = CommandRegistry::build_default();
        let module = module_with_registry("proc p {} {uplevel #0 {set g VALUE}}", &registry);
        assert!(
            detect_global_write_procs_with_registry(&module, &registry)
                [&summary_id(&module, "::p")]
                .names
                .contains("g")
        );
        let mut missing = module.clone();
        missing.source_metadata_input = None;
        assert!(
            detect_global_write_procs_with_registry(&missing, &registry)
                [&summary_id(&missing, "::p")]
                .opaque_global_frame
        );
        let mut foreign = module.clone();
        let input = module.source_metadata_input.as_ref().unwrap();
        foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
            input.lexer_config(),
        ));
        assert!(
            detect_global_write_procs_with_registry(&foreign, &registry)
                [&summary_id(&foreign, "::p")]
                .opaque_global_frame
        );
        let mut stale = module;
        stale.lexer_config.expand_syntax = !stale.lexer_config.expand_syntax;
        assert!(
            detect_global_write_procs_with_registry(&stale, &registry)[&summary_id(&stale, "::p")]
                .opaque_global_frame
        );
    }

    #[test]
    fn supplied_script_write_projection_uses_current_roles_instead_of_call_defs() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "gated-write",
            surface: registry.get("dict").unwrap().surface,
            arg_roles: &[(0, ArgRole::VarWrite)],
            ..CommandSpec::DEFAULT
        });
        let module = module_with_registry("gated-write {$scalar(open} VALUE", &registry);
        let aliases = ModuleCommandBindings::analyse(&module, &registry);
        for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
            let context = tcl_registry::model::ingress::static_context_for(environment)
                .with_command_store(registry.snapshot().shared_registry());
            let projection = script_value_write_projection_with_metadata_context(
                &module.top_level,
                &registry,
                &aliases,
                "::",
                Some((&context).into()),
                module.lexer_config,
            );
            assert_eq!(
                projection
                    .literal_names
                    .contains(&"$scalar(open".to_owned()),
                available,
                "{environment}: {projection:?}"
            );
            if !available {
                assert!(projection.opaque_variable_frame);
            }
        }
        assert!(
            script_value_write_projection_with_metadata_context(
                &module.top_level,
                &registry,
                &aliases,
                "::",
                None,
                module.lexer_config
            )
            .opaque_variable_frame
        );
    }

    #[test]
    fn supplied_global_write_call_closure_includes_source_expression_targets() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let module = module(
            "proc mutate {} {global g; set g VALUE; return 0}; proc p {} {return [expr {[mutate]}]}",
        );
        assert!(
            detect_global_write_procs(&module)[&summary_id(&module, "::p")]
                .names
                .contains("g")
        );
    }

    #[test]
    fn empty_module_has_no_writes() {
        let m = module("");
        assert!(detect_global_write_procs(&m).is_empty());
    }

    /// Equal leaf names retain independent body summaries.
    #[test]
    fn same_tailed_bodies_retain_independent_write_summaries() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let m = module(
            "namespace eval ::b {}; namespace eval ::a {}\n\
             proc ::b::run {} { global bbb; set bbb 1 }\n\
             proc ::a::run {} { global aaa; set aaa 1 }\n",
        );
        let info = detect_global_write_procs(&m);
        let a = summary_id(&m, "::a::run");
        let b = summary_id(&m, "::b::run");
        assert_ne!(a, b);
        assert_eq!(info[&a].names, BTreeSet::from(["aaa".to_owned()]));
        assert_eq!(info[&b].names, BTreeSet::from(["bbb".to_owned()]));
        let union: BTreeSet<_> = info[&a].names.union(&info[&b].names).cloned().collect();
        assert_eq!(union, BTreeSet::from(["aaa".to_owned(), "bbb".to_owned()]));
    }

    #[test]
    fn proc_with_no_global_write_is_empty() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let m = module("proc ::p {} { set x 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get(&summary_id(&m, "::p")).unwrap().is_empty());
    }

    #[test]
    fn direct_global_write_recorded() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // The `mutate` repro: global x; set x 99.
        let m = module("proc ::mutate {} { global x; set x 99 }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::mutate"))
                .unwrap()
                .names
                .contains("x")
        );
        // The fixture selector accepts either display for this single body.
        assert!(
            info.get(&summary_id(&m, "mutate"))
                .unwrap()
                .names
                .contains("x")
        );
    }

    #[test]
    fn variable_namespace_write_recorded() {
        let m = module("proc ::ns::p {} { variable v; set v 1 }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::ns::p"))
                .unwrap()
                .names
                .contains("v")
        );
    }

    #[test]
    fn upvar_hash_zero_counts_as_global() {
        let m = module("proc ::p {} { upvar #0 g local; set local 1 }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::p"))
                .unwrap()
                .names
                .contains("g")
        );
        // Not the local alias name — recording "local" would be a no-op at
        // the caller (no such caller-frame variable), missing the real "g".
        assert!(
            !info
                .get(&summary_id(&m, "::p"))
                .unwrap()
                .names
                .contains("local")
        );
    }

    #[test]
    fn namespace_upvar_rename_resolves_to_outer_name() {
        let m = module("proc ::ns::p {} { namespace upvar ::ns nsvar localvar\nset localvar 1 }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::ns::p"))
                .unwrap()
                .names
                .contains("nsvar")
        );
        assert!(
            !info
                .get(&summary_id(&m, "::ns::p"))
                .unwrap()
                .names
                .contains("localvar")
        );
    }

    #[test]
    fn branch_alternative_outer_aliases_are_unioned() {
        let m = module("proc ::p {c} { if {$c} { upvar #0 a x } else { upvar #0 b x }; set x 1 }");
        let info = detect_global_write_procs(&m);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["a".to_owned(), "b".to_owned()])
        );
    }

    #[test]
    fn dynamic_outer_alias_target_widens_the_global_frame() {
        let m = module("proc ::p {name} { upvar #0 $name x; set x 1 }");
        let info = detect_global_write_procs(&m);
        assert!(
            info[&summary_id(&m, "::p")].opaque_global_frame,
            "got {:?}",
            info[&summary_id(&m, "::p")]
        );
    }

    #[test]
    fn dynamic_namespace_alias_target_widens_the_global_frame() {
        let m = module("proc ::p {ns} { namespace upvar $ns x local; set local 1 }");
        let info = detect_global_write_procs(&m);
        assert!(
            info[&summary_id(&m, "::p")].opaque_global_frame,
            "got {:?}",
            info[&summary_id(&m, "::p")]
        );
    }

    #[test]
    fn caller_frame_upvar_is_not_a_global_write() {
        // `upvar 1` aliases the *caller* frame, not global/namespace —
        // must not be treated as an outer-scope write.
        let m = module("proc ::p {} { upvar 1 caller_x local; set local 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get(&summary_id(&m, "::p")).unwrap().is_empty());
    }

    #[test]
    fn read_only_global_is_not_a_write() {
        let m = module("proc ::p {} { global x; puts $x }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::p")).unwrap().is_empty(),
            "read-only declaration produced effects: {:?}",
            info.get(&summary_id(&m, "::p")).unwrap()
        );
    }

    #[test]
    fn renamed_global_descriptor_establishes_alias_without_counting_declaration_as_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "rename global declare_global\n\
             proc ::p {} { declare_global x; set x 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn renamed_trace_descriptor_is_not_mistaken_for_a_value_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "rename trace observe\n\
             proc ::p {} { variable x; observe add variable x write callback }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(info[&summary_id(&m, "::p")].is_empty());
    }

    #[test]
    fn custom_replacement_under_builtin_spelling_uses_its_own_descriptor() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "global",
            arg_roles: &[(0, ArgRole::VarWrite)],
            state_transitions: Some(StateTransitionDescriptor::EMPTY),
            world_effects: Some(tcl_registry::WorldEffectDescriptor::VARIABLE_WRITE),
            native_compilation: Some(tcl_registry::native_compilation::NativeCompilationSpec {
                grammar: tcl_registry::native_compilation::NativeCompilationGrammar::NoHook,
                operation: tcl_registry::SemanticOperationId::Invoke,
                body: tcl_registry::native_compilation::NativeBodyCompilation::Direct,
            }),
            successful_handler: Some(
                tcl_registry::native_compilation::SuccessfulHandlerSpec::VariableOperands,
            ),
            ..CommandSpec::DEFAULT
        });
        let m = module_with_registry("proc ::p {} { variable x; global x value }", &registry);
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn variable_declaration_without_value_is_not_a_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry("proc ::p {} { variable x }", &registry);
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(info[&summary_id(&m, "::p")].is_empty());
    }

    #[test]
    fn variable_value_form_records_the_namespace_value_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry("proc ::p {} { variable x 1 }", &registry);
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn variable_mixed_pairs_only_record_value_bearing_cells() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry("proc ::p {} { variable x 1 y }", &registry);
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn alias_baked_level_is_part_of_exact_transition_resolution() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             proc ::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn alias_defined_after_the_procedure_is_live_when_the_body_runs() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {} { g x local; set local 1 }\n\
             interp alias {} g {} upvar #0",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn alias_established_inside_the_procedure_applies_to_later_calls() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {} { interp alias {} g {} upvar #0; g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn alias_established_in_evaluated_substitution_applies_to_later_calls() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {} {\
                 set ignored [interp alias {} g {} upvar #0]; \
                 g x local; \
                 set local 1\
             }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn late_namespaced_alias_resolves_inside_namespace_eval_block() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::n::p {} { eval { g x local; set local 1 } }\n\
             interp alias {} ::n::g {} upvar #0",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::n::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn alias_established_by_another_callable_procedure_is_in_the_may_state() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::install {} { interp alias {} g {} upvar #0 }\n\
             proc ::p {} { install; g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn alias_created_in_namespaced_proc_is_global_in_source_interpreter() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::n::install {} { interp alias {} g {} upvar #0 }\n\
             proc ::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn dynamic_command_binding_transition_widens_the_global_frame_summary() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {from to} { rename $from $to; unknown_target }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(info[&summary_id(&m, "::p")].opaque_global_frame);
    }

    #[test]
    fn canonical_alias_target_participates_in_transitive_write_closure() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::mutate {} { global x; set x 99 }\n\
             interp alias {} call_mutate {} mutate\n\
             proc ::outer {} { call_mutate }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::outer")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn late_alias_summary_is_published_under_its_source_spelling() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::mutate {} { global x; set x 99 }\n\
             proc ::outer {} { call_mutate }\n\
             interp alias {} call_mutate {} mutate",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "call_mutate")].names,
            BTreeSet::from(["x".to_owned()])
        );
        assert_eq!(
            info[&summary_id(&m, "::outer")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn alias_global_target_survives_same_tail_namespace_collision() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::mutate {} { global x; set x 99 }\n\
             proc ::z::mutate {} {}\n\
             interp alias {} call_mutate {} mutate\n\
             proc ::outer {} { call_mutate }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "call_mutate")].names,
            BTreeSet::from(["x".to_owned()])
        );
        assert_eq!(
            info[&summary_id(&m, "::outer")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn rename_preserves_every_module_live_alias_state_with_its_baked_arguments() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             rename g h\n\
             proc ::p {} { h x local; set local 1 }\n\
             proc ::old_name {} { g y local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
        // Summaries deliberately use the module-wide may-state: `g` was a live
        // binding at one executable point, so a stored body calling it remains
        // conservative even though this particular definition follows the
        // rename. Lifetime-sensitive procedure reachability is not encoded in
        // the summary key.
        assert_eq!(
            info[&summary_id(&m, "::old_name")].names,
            BTreeSet::from(["y".to_owned()])
        );
    }

    #[test]
    fn alias_chain_composes_every_baked_argument_in_order() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} bind_global {} upvar #0\n\
             interp alias {} bind_x {} bind_global x\n\
             interp alias {} bind_local {} bind_x\n\
             proc ::p {} { bind_local local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn embedded_global_alias_transition_maps_later_local_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {} { set _ [upvar #0 g local]; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["g".to_owned()])
        );
        assert!(!info[&summary_id(&m, "::p")].opaque_global_frame);
    }

    #[test]
    fn embedded_alias_prefix_preserves_global_alias_arguments() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} bind {} upvar #0 g\n\
             proc ::p {} { set _ [bind local]; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["g".to_owned()])
        );
        assert!(!info[&summary_id(&m, "::p")].opaque_global_frame);
    }

    #[test]
    fn embedded_namespace_alias_transition_maps_later_local_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::p {} { set _ [namespace upvar ::target x local]; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
        assert!(!info[&summary_id(&m, "::p")].opaque_global_frame);
    }

    #[test]
    fn alias_target_prefix_resolves_in_the_target_interpreters_global_namespace() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             proc ::n::upvar args {}\n\
             proc ::n::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::n::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn conditional_alias_override_joins_both_live_bindings() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             if {$replace} { interp alias {} g {} puts }\n\
             proc ::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn conditional_namespace_alias_keeps_global_fallback_live() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             proc ::n::install {replace} {\n\
                 if {$replace} { interp alias {} ::n::g {} puts }\n\
             }\n\
             proc ::n::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::n::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn non_mutating_alias_alternative_preserves_the_incoming_binding() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} upvar #0\n\
             interp alias {} mutate {} list\n\
             if {$replace} { interp alias {} mutate {} rename }\n\
             mutate g {}\n\
             proc ::p {} { g x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn alias_baked_namespace_upvar_prefix_is_part_of_exact_transition_resolution() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} bind_ns {} namespace upvar ::target\n\
             proc ::p {} { bind_ns x local; set local 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn alias_baked_variable_name_still_records_value_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} initialise {} variable x\n\
             proc ::p {} { initialise 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn alias_baked_set_target_is_a_global_value_write() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} put {} set x\n\
             proc ::p {} { global x; put 1 }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn alias_baked_custom_varwrite_target_uses_its_registry_descriptor() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "write-cell",
            arg_roles: &[(0, ArgRole::VarWrite)],
            ..CommandSpec::DEFAULT
        });
        let m = module_with_registry(
            "interp alias {} write_x {} write-cell x\n\
             proc ::p {} { global x; write_x value }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn write_on_conditional_branch_still_counted() {
        // Sound over-approximation: flow-insensitive, so a write inside
        // an `if` still counts even though it may not always execute.
        let m = module("proc ::p {} { global x\nif {$c} { set x 1 } }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::p"))
                .unwrap()
                .names
                .contains("x")
        );
    }

    #[test]
    fn declaration_after_write_still_counted() {
        // Flow-insensitive union: order within the body doesn't matter.
        let m = module("proc ::p {} { set x 1\nglobal x }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::p"))
                .unwrap()
                .names
                .contains("x")
        );
    }

    #[test]
    fn logical_nested_write_projection_requires_original_children_not_literal_brackets() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let registry = CommandRegistry::build_default();
        for (source, expected) in [
            ("proc p {} {set y [incr x]}", true),
            ("proc p {} {set y {[incr x]}}", false),
        ] {
            let module = module_with_registry(source, &registry);
            let bindings = ModuleCommandBindings::analyse(&module, &registry);
            let semantics =
                GlobalWriteSemantics::from_module(&module, &registry, &bindings).unwrap();
            let procedure = &module.procedures[&summary_id(&module, "::p")];
            let statement = procedure.body.statements.first().unwrap();
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let (writes, _) = original_nested_footprint(tokens, &semantics);
            assert_eq!(
                writes.literal_names.contains(&"x".to_owned()),
                expected,
                "{source}"
            );
            assert!(!writes.opaque_variable_frame, "{source}");
            let mut changed = tokens.clone();
            changed.argv_texts[2] = "[incr foreign]".to_owned();
            let (rejected, _) = original_nested_footprint(&changed, &semantics);
            assert!(rejected.opaque_variable_frame, "{source}");
            assert!(rejected.literal_names.is_empty(), "{source}");
        }
    }

    #[test]
    fn embedded_incr_in_command_substitution_recorded() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let m = module("proc ::p {} { global x\nset y [incr x] }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::p"))
                .unwrap()
                .names
                .contains("x")
        );
    }

    #[test]
    fn expanded_global_write_target_widens_the_global_frame() {
        let m = module("proc ::p {names} { global {*}$names; set x 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info[&summary_id(&m, "::p")].opaque_global_frame);
    }

    #[test]
    fn embedded_proc_call_in_assign_value_propagates_global_write() {
        let m = module(
            "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::outer {} { set result \"value=[mutate]\" }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info[&summary_id(&m, "::outer")].names.contains("x"));
    }

    #[test]
    fn recovered_procedure_without_a_published_summary_is_opaque() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "namespace eval ::n [list proc writer {} {global g; set g 1}]\n\
             proc ::caller {} {::n::writer}",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(info[&summary_id(&m, "::caller")].opaque_global_frame);
    }

    #[test]
    fn redefined_procedure_generation_is_an_opaque_global_effect() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc p {} {}\n\
             rename p old\n\
             proc p {} {global g; set g 1}\n\
             proc caller {} {p}",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(info[&summary_id(&m, "::p")].opaque_global_frame);
        assert!(info[&summary_id(&m, "::caller")].opaque_global_frame);
    }

    #[test]
    fn embedded_proc_call_in_return_propagates_global_write() {
        let m = module(
            "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::outer {} { return [mutate] }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info[&summary_id(&m, "::outer")].names.contains("x"));
    }

    #[test]
    fn embedded_proc_calls_in_assign_expr_and_expr_eval_propagate_global_write() {
        let m = module(
            "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::assign_expr {} { set result [expr {[mutate] + 1}] }\n\
             proc ::expr_eval {} { expr {[mutate] + 1} }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info[&summary_id(&m, "::assign_expr")].names.contains("x"));
        assert!(info[&summary_id(&m, "::expr_eval")].names.contains("x"));
    }

    #[test]
    fn embedded_proc_calls_in_if_and_while_conditions_propagate_global_write() {
        let m = module(
            "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::conditional {} { if {[mutate]} { return } }\n\
             proc ::loop {} { while {[mutate]} { break } }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info[&summary_id(&m, "::conditional")].names.contains("x"));
        assert!(info[&summary_id(&m, "::loop")].names.contains("x"));
    }

    #[test]
    fn dynamic_eval_body_widens_later_command_binding_resolution() {
        // The body can install `late` before the following call executes:
        // `outer {interp alias {} late {} mutate}`. Its text is dynamic in
        // the stored procedure, so the registry's dynamic-body fact and the
        // lowered Barrier must conservatively widen the command table.
        let m = module(
            "proc ::mutate {} { global x; set x 99 }\n\
             proc ::outer {body} { eval $body; late }\n\
             outer {interp alias {} late {} mutate}",
        );
        let info = detect_global_write_procs(&m);
        assert!(info[&summary_id(&m, "::outer")].opaque_global_frame);
    }

    #[test]
    fn transitive_closure_through_direct_call() {
        // `outer` calls `mutate`, which writes global x — `outer`'s
        // summary must include `x` too (transitive).
        let m = module("proc ::mutate {} { global x; set x 99 }\nproc ::outer {} { mutate }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::outer"))
                .unwrap()
                .names
                .contains("x")
        );
    }

    #[test]
    fn recursive_proc_terminates_and_includes_write() {
        let m = module(
            "proc ::p {n} { if {$n == 0} { return }\n global x\n incr x\n p [expr {$n-1}] }",
        );
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::p"))
                .unwrap()
                .names
                .contains("x")
        );
    }

    #[test]
    fn mutually_recursive_procs_share_the_write() {
        let m = module("proc ::a {} { global x; set x 1; b }\nproc ::b {} { a }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::a"))
                .unwrap()
                .names
                .contains("x")
        );
        assert!(
            info.get(&summary_id(&m, "::b"))
                .unwrap()
                .names
                .contains("x")
        );
    }

    #[test]
    fn uplevel_hash_zero_literal_body_records_global_writes() {
        // `uplevel #0 {set x 99}` needs no `global`
        // declaration at all (tclsh 9.0.3/9.0.4: the caller-visible `x`
        // really is 99 afterwards).
        let m = module("proc ::setter {} { uplevel #0 { set x 99 } }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::setter"))
                .unwrap()
                .names
                .contains("x")
        );
        assert!(
            !info
                .get(&summary_id(&m, "::setter"))
                .unwrap()
                .opaque_global_frame
        );
    }

    #[test]
    fn uplevel_hash_zero_literal_body_resolves_late_alias_prefix_write() {
        // The proc body is lowered before the later alias exists, so `put`
        // carries no lower-time def.  Tcl 9.0.4 runs the alias prefix as
        // `set x 99` in frame #0; the closed module binding projection must
        // therefore attribute the global `x` write to `setter`.
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::setter {} { uplevel #0 { put 99 } }\n\
             interp alias {} put {} set x",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::setter")].names,
            BTreeSet::from(["x".to_owned()])
        );
        assert!(!info[&summary_id(&m, "::setter")].opaque_global_frame);
    }

    #[test]
    fn alias_baked_global_literal_body_resolves_late_inner_alias() {
        // `g` is itself installed after the procedure body was lowered, so
        // this takes the plain-call literal-body path rather than UpFrame.
        // Its inner `put` alias is later still and must remain visible.
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::setter {} { g { put 99 } }\n\
             interp alias {} g {} uplevel #0\n\
             interp alias {} put {} set x",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::setter")].names,
            BTreeSet::from(["x".to_owned()])
        );
        assert!(!info[&summary_id(&m, "::setter")].opaque_global_frame);
    }

    #[test]
    fn nested_relative_zero_inside_global_upframe_stays_global() {
        // The inner `uplevel 0` inherits the outer `#0` frame rather than
        // returning to the defining procedure's frame.
        let m = module("proc ::setter {} { uplevel #0 { uplevel 0 { ::set g 1 } } }");
        let info = detect_global_write_procs(&m);
        assert!(
            info[&summary_id(&m, "::setter")].names.contains("g"),
            "got {info:?}"
        );
        assert!(
            !info[&summary_id(&m, "::setter")].opaque_global_frame,
            "got {info:?}"
        );
    }

    #[test]
    fn late_alias_to_relative_zero_inside_global_frame_stays_global() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::setter {} {uplevel #0 {::inner {::set g 1}}}\n\
             interp alias {} ::inner {} uplevel 0",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::setter")].names,
            BTreeSet::from(["g".to_owned()])
        );
        assert!(!info[&summary_id(&m, "::setter")].opaque_global_frame);
    }

    #[test]
    fn late_alias_to_eval_inside_global_frame_stays_global() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::setter {} {uplevel #0 {::inner {::set g 1}}}\n\
             interp alias {} ::inner {} eval",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::setter")].names,
            BTreeSet::from(["g".to_owned()])
        );
        assert!(!info[&summary_id(&m, "::setter")].opaque_global_frame);
    }

    #[test]
    fn transparent_eval_inside_global_upframe_uses_global_command_resolution() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "proc ::helper {} { global global_hit; set global_hit 1 }\n\
             namespace eval ::n {\n\
                 proc helper {} { global namespaced_hit; set namespaced_hit 1 }\n\
                 proc setter {} { uplevel #0 { eval { helper } } }\n\
             }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert!(
            info[&summary_id(&m, "::n::setter")]
                .names
                .contains("global_hit")
        );
        assert!(
            !info[&summary_id(&m, "::n::setter")]
                .names
                .contains("namespaced_hit"),
            "a transparent block must not restore its lowering-time namespace: {info:?}"
        );
        assert!(!info[&summary_id(&m, "::n::setter")].opaque_global_frame);
    }

    #[test]
    fn transparent_eval_inside_global_upframe_resolves_a_late_global_alias() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "namespace eval ::n {\n\
                 proc put args {}\n\
                 proc setter {} { uplevel #0 { eval { put 2 } } }\n\
             }\n\
             interp alias {} put {} set x",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::n::setter")].names,
            BTreeSet::from(["x".to_owned()]),
            "a transparent Block inherits the selected global command namespace"
        );
        assert!(!info[&summary_id(&m, "::n::setter")].opaque_global_frame);
    }

    #[test]
    fn uplevel_hash_zero_constructed_list_records_global_writes() {
        // `uplevel #0 [list set k 77]` — tclsh 9.0.4: the global `k` is 77.
        let m = module("proc ::s3 {} { uplevel #0 [list set k 77] }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::s3"))
                .unwrap()
                .names
                .contains("k")
        );
        assert!(
            !info
                .get(&summary_id(&m, "::s3"))
                .unwrap()
                .opaque_global_frame
        );
    }

    #[test]
    fn constructed_global_body_with_dynamic_operand_is_opaque() {
        let m = module("proc ::p {name} { uplevel #0 [list set $name 1] }");
        let info = detect_global_write_procs(&m);
        assert!(info[&summary_id(&m, "::p")].opaque_global_frame);
        assert!(info[&summary_id(&m, "::p")].names.is_empty());
    }

    #[test]
    fn rebound_list_constructor_widens_global_frame_effects() {
        let m = module(
            "proc list args { return {set hidden 1} }\n\
             proc ::p {} { uplevel #0 [list set claimed 1] }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info[&summary_id(&m, "::p")].opaque_global_frame);
        assert!(info[&summary_id(&m, "::p")].names.is_empty());
    }

    #[test]
    fn alias_baked_global_uplevel_frame_is_resolved_from_invocation_facts() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} uplevel #0\n\
             proc ::p {} { g {set x 1} }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
        assert!(!info[&summary_id(&m, "::p")].opaque_global_frame);
    }

    #[test]
    fn constructed_global_uplevel_resolves_command_in_global_namespace() {
        let registry = CommandRegistry::build_default();
        let m = module_with_registry(
            "interp alias {} g {} set\n\
             interp alias {} ::n::g {} list\n\
             proc ::n::p {} { uplevel #0 [list g x 1] }",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        assert_eq!(
            info[&summary_id(&m, "::n::p")].names,
            BTreeSet::from(["x".to_owned()])
        );
    }

    #[test]
    fn uplevel_hash_zero_dynamic_script_is_opaque() {
        // `uplevel #0 $body` — any global may be written or read.
        let m = module("proc ::s2 {body} { uplevel #0 $body }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::s2"))
                .unwrap()
                .opaque_global_frame
        );
        assert!(!info.get(&summary_id(&m, "::s2")).unwrap().is_empty());
    }

    #[test]
    fn uplevel_hash_zero_opaqueness_is_transitive() {
        let m = module("proc ::s2 {body} { uplevel #0 $body }\nproc ::outer {body} { s2 $body }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::outer"))
                .unwrap()
                .opaque_global_frame
        );
    }

    #[test]
    fn uplevel_hash_zero_non_writing_script_stays_empty() {
        // TN — a global-frame script that writes nothing (`uplevel #0
        // [list puts hi]`) contributes neither names nor opaqueness.
        let m = module("proc ::shout {} { uplevel #0 [list puts hi] }");
        let info = detect_global_write_procs(&m);
        assert!(info.get(&summary_id(&m, "::shout")).unwrap().is_empty());
    }

    #[test]
    fn uplevel_caller_frame_is_not_a_global_write() {
        // An absolute value-write command proves the body only writes the
        // caller's frame, which is `upvar_info`'s business. A relative `set`
        // could be shadowed in the caller namespace by an arbitrary global-
        // writing procedure and must therefore widen this summary.
        let m = module("proc ::p {} { uplevel 1 { ::set n 1 } }");
        let info = detect_global_write_procs(&m);
        assert!(info.get(&summary_id(&m, "::p")).unwrap().is_empty());
    }

    #[test]
    fn uplevel_hash_zero_dynamic_write_target_in_literal_body_is_opaque() {
        // `uplevel #0 {set $n 1}` — the body is readable but the written
        // name is not.
        let m = module("proc ::p {n} { uplevel #0 \"set $n 1\" }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::p"))
                .unwrap()
                .opaque_global_frame
        );
    }

    #[test]
    fn unrelated_global_write_is_not_confused_with_other_procs() {
        let m =
            module("proc ::mutate_y {} { global y; set y 1 }\nproc ::p {} { global x; set x 1 }");
        let info = detect_global_write_procs(&m);
        assert!(
            info.get(&summary_id(&m, "::p"))
                .unwrap()
                .names
                .contains("x")
        );
        assert!(
            !info
                .get(&summary_id(&m, "::p"))
                .unwrap()
                .names
                .contains("y")
        );
    }
    #[test]
    fn global_body_summaries_use_issued_identity_and_preserve_current_header_truth() {
        let original = module("proc mutate {} {set ::x VALUE}");
        let id = summary_id(&original, "::mutate");
        let expected = detect_global_write_procs(&original);
        assert!(expected[&id].names.contains("x"));
        let mut labelled = original.clone();
        if let Some(procedure) = labelled.procedures.get_mut(&id) {
            procedure.qualified_name = "report-only-label".to_owned();
        } else {
            labelled.body_units.get_mut(&id).unwrap().qualified_name =
                "report-only-label".to_owned();
        }
        assert_eq!(detect_global_write_procs(&labelled), expected);
        let mut wrong_kind = original.clone();
        if let Some(procedure) = wrong_kind.procedures.get_mut(&id) {
            procedure.body_kind = crate::ir::ProcedureBodyKind::Event;
        } else {
            wrong_kind.body_units.get_mut(&id).unwrap().body_kind =
                crate::ir::ProcedureBodyKind::Event;
        }
        let changed = detect_global_write_procs(&wrong_kind);
        assert!(changed.get(&id).is_none_or(|info| info.opaque_global_frame));
    }
}
