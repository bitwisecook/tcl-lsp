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

//! Cross-event variable scope analysis for iRules connection
//! lifecycles.
//!
//! iRules ``when`` event handlers share a connection-scoped Tcl
//! stack — variables set in one event persist until the
//! connection closes or the variable is explicitly ``unset``.
//! This module computes which local variables flow across
//! ``when`` event boundaries so that per-procedure diagnostics
//! (dead stores, read-before-set, unused variables) can be
//! suppressed when the variable is actually live across events.
//!
//! Used by:
//!
//! - `analyser/diagnostics.rs::emit_racy_static_diagnostics`
//!   (IRULE4005 — ``static::`` cross-event flow from a
//!   non-RULE_INIT event).
//! - The CFG/SSA RBS / unused-var emitters (cross-event
//!   suppression).  The analyser threads
//!   source labels selected from per-handler typed cross-event keys through
//!   `emit_cfg_ssa_diagnostics_for_function` so a
//!   ``set::ip [IP::client_addr]`` in `CLIENT_ACCEPTED` that's
//!   read in `HTTP_REQUEST` is not falsely flagged as unused.

use std::collections::{HashMap, HashSet};

use tcl_registry::events::{EventLifecycleRelation, EventRegistry, EventVariableFrame};
use tcl_registry::{CommandRegistry, Traits};

use crate::compilation_unit::FunctionUnit;
use crate::ir::Statement;
use crate::registry_invocation::InvocationMetadataContext;
use crate::var_resolve::{VariableCellKey, VariableCellSet, VariableCellTable};

/// A resolved cross-event cell; namespace cells and flow locals never alias.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventCell {
    /// A Tcl namespace cell in the executing interpreter/worker.
    Namespace(VariableCellKey),
    /// A variable in the current connection frame.
    Connection(tcl_core_types::NameBytes),
    /// A supplied execution context retained separately from naming geometry.
    Executed {
        /// Exact underlying namespace or connection root.
        cell: Box<EventCell>,
        /// Independently supplied interpreter identity.
        interpreter: Option<String>,
        /// Worker, reload epoch and connection selected by execution.
        execution: Option<tcl_registry::f5::WorkerExecution>,
        /// Host storage policy of this resolved cell.
        storage_domain: Option<tcl_registry::f5::VariableStorageDomain>,
    },
}

impl EventCell {
    /// Whether the independently retained root is a connection-frame cell.
    /// This classification establishes no event ordering or current value.
    #[must_use]
    pub fn is_connection(&self) -> bool {
        match self {
            Self::Connection(_) => true,
            Self::Namespace(_) => false,
            Self::Executed { cell, .. } => cell.is_connection(),
        }
    }
}

/// Variable summary for a single ``when`` event handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventVarSummary {
    /// Event name (e.g. ``"CLIENT_ACCEPTED"``).
    pub event: String,
    /// Variable names defined (set) on at least one path
    /// through the event.
    pub defs: HashSet<String>,
    /// Variable names used at SSA version 0 (read before any
    /// local def).  These are candidate cross-event imports.
    pub uses_before_def: HashSet<String>,
    /// Variable names explicitly ``unset`` in this event.
    pub unsets: HashSet<String>,
    /// Resolved cells possibly defined, with their scalar SSA keys.
    pub cell_defs: HashMap<EventCell, VariableCellSet>,
    /// Resolved observable reads, including entry imports and namespace reads.
    pub cell_reads: HashMap<EventCell, VariableCellSet>,
    /// Cells defined on every normal exit path, after destructive writes.
    pub must_defs: HashSet<EventCell>,
    /// Worker-static storage selected by the actual binding policy.
    worker_static_cells: HashSet<EventCell>,
    /// Written labels captured at the operation selecting each exact key.
    source_labels: VariableCellTable<HashSet<String>>,
}

/// Cross-event variable scope analysis result.
///
/// Built once from
/// the ``::when::*`` subset of `CompilationUnit::procedures`
/// and cached on `CompilationUnit::connection_scope`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectionScope {
    /// Per-event summaries keyed by event name.
    pub summaries: HashMap<String, EventVarSummary>,
    /// Individual handlers retained separately; priority order is not a union.
    pub handlers: HashMap<String, EventVarSummary>,
    /// Reporting labels for variables defined here and read in another event.
    /// Consumers obtain protection labels through `handler_source_names`;
    /// this presentation set does not identify storage.
    pub cross_event_defs: HashSet<String>,
    /// Reporting labels for variables read here and defined in another event.
    /// This presentation set does not identify storage.
    pub cross_event_imports: HashSet<String>,
    /// ``static::`` vars defined in a non-RULE_INIT event and
    /// used cross-event — feeds the **IRULE4005** racy-static
    /// emitter.
    pub racy_static_defs: HashSet<String>,
    /// Exact worker namespace roots read across event boundaries.
    pub racy_static_cells: HashSet<EventCell>,
    cross_event_def_keys: HashMap<String, VariableCellSet>,
    cross_event_import_keys: HashMap<String, VariableCellSet>,
    cross_event_cells: HashMap<String, HashSet<EventCell>>,
    /// Per-handler connection cells whose readers can precede their writers.
    pub scope_concerns: HashMap<String, HashMap<EventCell, HashSet<String>>>,
}

impl ConnectionScope {
    /// Project written labels only after matching this handler's exact keys.
    /// These labels protect source-walking passes; they never select cells.
    #[must_use]
    pub fn handler_source_names(&self, handler: &str, imports_only: bool) -> HashSet<String> {
        let Some(summary) = self.handlers.get(handler) else {
            return HashSet::new();
        };
        let mut names = HashSet::new();
        let keys = self
            .cross_event_import_keys
            .get(handler)
            .into_iter()
            .flat_map(|keys| keys.iter());
        for key in keys {
            if let Some(labels) = summary.source_labels.get(key) {
                names.extend(labels.iter().cloned());
            }
        }
        if !imports_only {
            for key in self
                .cross_event_def_keys
                .get(handler)
                .into_iter()
                .flat_map(|keys| keys.iter())
            {
                if let Some(labels) = summary.source_labels.get(key) {
                    names.extend(labels.iter().cloned());
                }
            }
        }
        names
    }

    /// Whether this resolved operation touches a proved cross-event root.
    #[must_use]
    pub fn observes_cross_event_place(&self, handler: &str, place: &crate::place::Place) -> bool {
        cell_from_place(place).is_some_and(|cell| {
            self.cross_event_cells
                .get(handler)
                .is_some_and(|cells| cells.contains(&cell))
        })
    }

    /// Conservative source-only protection union, after per-handler key matching.
    #[must_use]
    pub fn source_names(&self) -> HashSet<String> {
        self.handlers
            .keys()
            .flat_map(|handler| self.handler_source_names(handler, false))
            .collect()
    }
}

/// Canonical entry frame for an iRules handler. Host namespaces provide
/// namespace availability only; they do not imply values or prior writers.
#[must_use]
pub fn event_resolve_context(event: &str) -> crate::var_resolve::ResolveContext {
    let mut entry = crate::var_resolve::ResolveContext::for_namespace("::");
    entry.hosted_execution_context = Some(tcl_registry::f5::BigIpExecutionContext::TmmIRule);
    let frame = EventRegistry::build().variable_frame(event);
    entry.frame_kind = if frame == EventVariableFrame::InitialisationNamespace {
        crate::var_resolve::VariableFrameKind::Global
    } else {
        crate::var_resolve::VariableFrameKind::Local
    };
    entry.dynamic_bindings = frame == EventVariableFrame::Unknown;
    // A connection event can receive values from earlier handlers or the host.
    // Their unknown contents do not make the local physical bindings dynamic.
    // Explicit stores and unsets still publish precise presence afterwards.
    if frame != EventVariableFrame::InitialisationNamespace {
        entry.contents_world = crate::var_resolve::ContentsWorld::Unknown;
    }
    entry.known_namespaces.extend(
        tcl_registry::f5::runtime_namespaces(tcl_registry::f5::BigIpExecutionContext::TmmIRule)
            .iter()
            .map(|namespace| (*namespace).to_owned()),
    );
    entry
}

/// Build a [`ConnectionScope`] from compiled ``when``
/// procedures.
///
/// `when_procedures`
/// should be the subset of `CompilationUnit::procedures` whose
/// qualified names start with ``::when::``.
///
/// Actual command and condition reads come from the shared place bridge.
/// Array elements project to their root storage cell for possible liveness;
/// the element-sensitive cell SSA owns value and existence proofs.
#[must_use]
pub fn build_connection_scope<S: std::hash::BuildHasher>(
    when_procedures: &HashMap<String, FunctionUnit, S>,
) -> ConnectionScope {
    if let Some(input) = when_procedures
        .values()
        .find_map(FunctionUnit::source_metadata_input)
    {
        return build_connection_scope_with_registry(
            when_procedures,
            input.borrowed_context_registry().commands(),
        );
    }
    // Explicit standalone compatibility only. A supplied-missing source owner
    // still refuses at the shared FU/point guards below.
    let registry =
        CommandRegistry::build_default().project_for_profile(tcl_dialect::DialectProfile::irules());
    build_connection_scope_with_registry(when_procedures, &registry)
}

/// Resolve cross-event cells through the document's shared registry and bindings.
#[must_use]
pub fn build_connection_scope_with_registry<S: std::hash::BuildHasher>(
    when_procedures: &HashMap<String, FunctionUnit, S>,
    commands: &CommandRegistry,
) -> ConnectionScope {
    build_connection_scope_in(when_procedures, commands, None)
}

/// Resolve the source-owned handlers under the same actual Module and FU input.
/// This does not issue event execution, physical frame or worker-cell receipts.
#[must_use]
pub(crate) fn build_connection_scope_for_module<S: std::hash::BuildHasher>(
    when_procedures: &HashMap<String, FunctionUnit, S>,
    commands: &CommandRegistry,
    module: &crate::ir::Module,
) -> ConnectionScope {
    build_connection_scope_in(when_procedures, commands, Some(module))
}

fn build_connection_scope_in<S: std::hash::BuildHasher>(
    when_procedures: &HashMap<String, FunctionUnit, S>,
    commands: &CommandRegistry,
    module: Option<&crate::ir::Module>,
) -> ConnectionScope {
    let events = EventRegistry::build();
    let mut summaries: HashMap<String, EventVarSummary> = HashMap::new();
    let mut handlers = HashMap::new();
    for (qname, fu) in when_procedures {
        let Some(body) = &fu.irules_event_body else {
            continue;
        };
        if function_metadata(fu, commands, module).is_none() {
            continue;
        }
        let event = body.event();
        let summary = extract_event_summary(event, fu, commands, module);
        handlers.insert(qname.clone(), summary.clone());
        if let Some(previous) = summaries.get_mut(event) {
            previous.defs.extend(summary.defs);
            previous.uses_before_def.extend(summary.uses_before_def);
            previous.unsets.extend(summary.unsets);
            previous
                .worker_static_cells
                .extend(summary.worker_static_cells);
            for (key, labels) in summary.source_labels {
                previous
                    .source_labels
                    .entry(key)
                    .or_default()
                    .extend(labels);
            }
            merge_cell_names(&mut previous.cell_defs, summary.cell_defs);
            merge_cell_names(&mut previous.cell_reads, summary.cell_reads);
            // An event-name union does not prove which handlers execute or
            // their order. Definite facts remain on the individual handlers.
            previous.must_defs.clear();
        } else {
            summaries.insert(event.to_owned(), summary);
        }
    }
    let cross_defs = HashSet::new();
    let cross_imports = HashSet::new();
    let racy_statics = HashSet::new();
    let racy_static_cells = HashSet::new();
    let cross_event_def_keys: HashMap<String, VariableCellSet> = HashMap::new();
    let cross_event_import_keys: HashMap<String, VariableCellSet> = HashMap::new();
    let cross_event_cells: HashMap<String, HashSet<EventCell>> = HashMap::new();
    let scope_concerns: HashMap<String, HashMap<EventCell, HashSet<String>>> = HashMap::new();
    let mut scope = ConnectionScope {
        summaries,
        handlers,
        cross_event_defs: cross_defs,
        cross_event_imports: cross_imports,
        racy_static_defs: racy_statics,
        racy_static_cells,
        cross_event_def_keys,
        cross_event_import_keys,
        cross_event_cells,
        scope_concerns,
    };
    collect_handler_relations(&events, &mut scope);
    scope
}

fn collect_handler_relations(events: &EventRegistry, scope: &mut ConnectionScope) {
    let ConnectionScope {
        handlers,
        cross_event_defs: cross_defs,
        cross_event_imports: cross_imports,
        racy_static_defs: racy_statics,
        racy_static_cells,
        cross_event_def_keys,
        cross_event_import_keys,
        cross_event_cells,
        scope_concerns,
        ..
    } = scope;
    for (writer_name, writer) in handlers.iter() {
        for (reader_name, reader) in handlers.iter() {
            if writer_name == reader_name {
                continue;
            }
            if writer.event != reader.event {
                for cell in writer.cell_defs.keys().filter(|cell| cell.is_connection()) {
                    if reader.cell_reads.contains_key(cell)
                        && let Some(note) = events.variable_scope_note(&writer.event, &reader.event)
                    {
                        scope_concerns
                            .entry(writer_name.clone())
                            .or_default()
                            .entry(cell.clone())
                            .or_default()
                            .insert(note);
                    }
                }
            }
            let relation = events.lifecycle_relation(&writer.event, &reader.event);
            if !matches!(
                relation,
                EventLifecycleRelation::InitialisationBeforeTraffic
                    | EventLifecycleRelation::MayPrecede
            ) {
                continue;
            }
            for (cell, names) in &writer.cell_defs {
                let Some(read_names) = reader.cell_reads.get(cell) else {
                    continue;
                };
                cross_event_def_keys
                    .entry(writer_name.clone())
                    .or_default()
                    .extend(names.iter().cloned());
                cross_event_import_keys
                    .entry(reader_name.clone())
                    .or_default()
                    .extend(read_names.iter().cloned());
                cross_event_cells
                    .entry(writer_name.clone())
                    .or_default()
                    .insert(cell.clone());
                cross_event_cells
                    .entry(reader_name.clone())
                    .or_default()
                    .insert(cell.clone());
                for key in names {
                    if let Some(labels) = writer.source_labels.get(key) {
                        cross_defs.extend(labels.iter().cloned());
                    }
                }
                for key in read_names {
                    if let Some(labels) = reader.source_labels.get(key) {
                        cross_imports.extend(labels.iter().cloned());
                    }
                }
                if events.variable_frame(&writer.event)
                    != EventVariableFrame::InitialisationNamespace
                    && writer.worker_static_cells.contains(cell)
                {
                    for key in names {
                        if let Some(labels) = writer.source_labels.get(key) {
                            racy_statics.extend(labels.iter().cloned());
                        }
                    }
                    racy_static_cells.insert(cell.clone());
                }
            }
        }
    }
}

fn merge_cell_names(
    destination: &mut HashMap<EventCell, VariableCellSet>,
    source: HashMap<EventCell, VariableCellSet>,
) {
    for (cell, names) in source {
        destination.entry(cell).or_default().extend(names);
    }
}

fn function_metadata<'a>(
    function: &'a FunctionUnit,
    commands: &CommandRegistry,
    module: Option<&crate::ir::Module>,
) -> Option<Option<InvocationMetadataContext<'a>>> {
    if let Some(module) = module {
        module
            .retained_source_bindings
            .as_deref()?
            .matches_module(module, commands)
            .then_some(())?;
        return function
            .invocation_metadata_context_for_module(commands, module)
            .map(Some);
    }
    if function.source_metadata_input().is_some() {
        return function.invocation_metadata_context(commands).map(Some);
    }
    function
        .cfg
        .metadata_context
        .is_standalone()
        .then_some(())?;
    function.cfg.metadata_context.metadata_context(commands)
}

fn point_metadata<'a>(
    function: &FunctionUnit,
    tokens: &'a crate::ir::CommandTokens,
    commands: &CommandRegistry,
    module: Option<&crate::ir::Module>,
) -> Option<Option<InvocationMetadataContext<'a>>> {
    if let Some(module) = module {
        function.invocation_metadata_context_for_module(commands, module)?;
        return tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_module(tokens, module, commands)
            .map(Some);
    }
    tokens
        .source_binding
        .as_ref()?
        .original_invocation_metadata_for_function(tokens, function, commands)
}

/// Selected destruction at an authentic original operation. Unknown source,
/// availability or member selection establishes neither destruction nor a
/// preservation fact; independent physical cell proofs remain required.
pub(crate) fn statement_destroys(
    function: &FunctionUnit,
    block: crate::cfg::BlockId,
    index: usize,
    commands: &CommandRegistry,
    module: Option<&crate::ir::Module>,
) -> Option<bool> {
    function_metadata(function, commands, module)?;
    let statement = function.cfg.blocks.get(&block)?.statements.get(index)?;
    if !matches!(
        statement,
        Statement::Call { .. } | Statement::Barrier { .. }
    ) {
        return Some(false);
    }
    let tokens = function.cfg.source_tokens_at(block, index)?;
    let context = point_metadata(function, tokens, commands, module)?;
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        commands, context, tokens,
    )
    .or_else(|| {
        let context = context?;
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            commands, context, tokens,
        )
        .or_else(|| {
            crate::registry_invocation::original_structured_invocation_with_metadata_context(
                commands, context, tokens,
            )
        })
    })?;
    if !matches!(
        invocation.facts.subcommand,
        tcl_registry::OwnedSubcommandResolution::NotApplicable
            | tcl_registry::OwnedSubcommandResolution::Exact { .. }
            | tcl_registry::OwnedSubcommandResolution::UniquePrefix { .. }
    ) {
        return None;
    }
    let destroys = invocation.facts.traits.contains(Traits::DESTROYS_VARIABLE);
    let Some(context) = context else {
        return Some(destroys);
    };
    let realm = tokens.source_binding.as_ref()?.invocation_realm()?;
    invocation.with_metadata_schema(commands, context, realm, |schema| {
        Some(
            destroys
                || schema
                    .authored_source_descriptors()
                    .subcommand
                    .is_some_and(|subcommand| subcommand.destructive),
        )
    })
}

/// A selected original call may clear coverage even when its destructive
/// variable operand is unknown. Missing point metadata remains a clobber;
/// structural statements retain their independent typed boundary effects.
fn statement_clobbers(
    function: &FunctionUnit,
    block: crate::cfg::BlockId,
    index: usize,
    commands: &CommandRegistry,
    module: Option<&crate::ir::Module>,
) -> bool {
    let Some(mut context) = function_metadata(function, commands, module) else {
        return true;
    };
    let Some(statement) = function
        .cfg
        .blocks
        .get(&block)
        .and_then(|block| block.statements.get(index))
    else {
        return true;
    };
    if statement.is_executable_invocation() {
        if let Some(tokens) = function.cfg.source_tokens_at(block, index) {
            let Some(original) = point_metadata(function, tokens, commands, module) else {
                return true;
            };
            context = original;
        } else if matches!(
            statement,
            Statement::Call { .. }
                | Statement::Barrier { .. }
                | Statement::NativeCall { .. }
                | Statement::UpFrame { .. }
        ) {
            return true;
        }
    }
    crate::memory_ssa::is_clobber_with_metadata_context(statement, commands, context)
}

pub(crate) fn cell_from_place(place: &crate::place::Place) -> Option<EventCell> {
    if place.dynamic {
        return None;
    }
    let root = if place.is_global() {
        EventCell::Namespace(crate::var_resolve::cell_key(place))
    } else if place.ns == crate::place::LOCAL_NS {
        // A resolved native root never re-enters through its reporting label.
        // Places without cell identity belong to the explicit authored model.
        EventCell::Connection(place.cell.as_ref().map_or_else(
            || tcl_core_types::NameBytes::from(place.name.as_bytes()),
            |cell| cell.name.clone(),
        ))
    } else {
        return None;
    };
    match &place.cell {
        Some(cell) if cell.execution.is_some() || cell.interpreter.is_some() => {
            Some(EventCell::Executed {
                cell: Box::new(root),
                interpreter: cell.interpreter.clone(),
                execution: cell.execution,
                storage_domain: cell.storage_domain,
            })
        }
        _ => Some(root),
    }
}

/// Source labels are selected through exact operation symbols. Authored
/// qualified labels remain useful reporting aliases; native debug encodings
/// never enter either presentation or source-walking protection sets.
fn labels_at(
    fu: &FunctionUnit,
    block: crate::cfg::BlockId,
    index: usize,
    key: &VariableCellKey,
    place: &crate::place::Place,
) -> HashSet<String> {
    let mut labels: HashSet<String> = fu
        .ssa
        .source_symbols_at(block, index)
        .into_iter()
        .flat_map(|symbols| symbols.iter())
        .filter(|(_, symbol)| fu.ssa.cell_key(**symbol) == key)
        .map(|(name, _)| name.clone())
        .collect();
    if place.ns == crate::place::LOCAL_NS {
        labels.insert(place.name.clone());
    }
    match key.root() {
        VariableCellKey::Authored(name) => {
            labels.insert(name.clone());
        }
        VariableCellKey::Namespace {
            identity: crate::command_binding::SourceNamespaceKey::Authored(_),
            ..
        } => {
            labels.insert(crate::naming::qualify(&place.ns, &place.name));
        }
        _ => {}
    }
    labels
}

fn extract_event_summary(
    event: &str,
    fu: &FunctionUnit,
    commands: &CommandRegistry,
    module: Option<&crate::ir::Module>,
) -> EventVarSummary {
    let entry = event_resolve_context(event);
    let fallback_contexts;
    let contexts = if let Some(contexts) = &fu.ssa.point_contexts {
        contexts
    } else {
        fallback_contexts = crate::variable_bindings::build_point_resolve_contexts_with_entry(
            &fu.cfg, entry, commands,
        );
        &fallback_contexts
    };
    let mut cell_defs: HashMap<EventCell, VariableCellSet> = HashMap::new();
    let mut cell_reads: HashMap<EventCell, VariableCellSet> = HashMap::new();
    let mut worker_static_cells = HashSet::new();
    let mut source_labels: VariableCellTable<HashSet<String>> = VariableCellTable::default();
    let mut defs: HashSet<String> = HashSet::new();
    let mut uses_v0: HashSet<String> = HashSet::new();
    let mut unsets: HashSet<String> = HashSet::new();

    for (block_id, block) in &fu.ssa.blocks {
        for (statement_index, stmt) in block.statements.iter().enumerate() {
            let context = contexts.before_statement(*block_id, statement_index);
            let Some(is_unset) =
                statement_destroys(fu, *block_id, statement_index, commands, module)
            else {
                continue;
            };
            let places = if is_unset {
                crate::place_bridge::statement_mutation_places_with_continuation(
                    &stmt.statement,
                    context,
                    contexts.after_statement(*block_id, statement_index),
                    commands,
                )
            } else {
                crate::place_bridge::def_places_with_continuation(
                    &stmt.statement,
                    context,
                    contexts.after_statement(*block_id, statement_index),
                    commands,
                )
            };
            for place in places {
                let Some(cell) = cell_from_place(&place) else {
                    continue;
                };
                let Some(key) = crate::var_resolve::canonical_binding_value_key(&place) else {
                    continue;
                };
                if is_unset {
                    let labels = labels_at(fu, *block_id, statement_index, &key, &place);
                    unsets.extend(labels);
                } else {
                    if place.cell.as_ref().is_some_and(|cell| {
                        cell.storage_domain
                            == Some(tcl_registry::f5::VariableStorageDomain::WorkerNamespace)
                    }) {
                        worker_static_cells.insert(cell.clone());
                    }
                    let labels = labels_at(fu, *block_id, statement_index, &key, &place);
                    defs.extend(labels.iter().cloned());
                    source_labels.entry(key.clone()).or_default().extend(labels);
                    cell_defs.entry(cell).or_default().insert(key);
                }
            }
        }
    }

    collect_event_reads(
        fu,
        contexts,
        commands,
        &mut uses_v0,
        &mut source_labels,
        &mut cell_reads,
    );
    let must_defs = definite_exit_cells(fu, commands, contexts, module);
    EventVarSummary {
        event: event.to_string(),
        defs,
        uses_before_def: uses_v0,
        unsets,
        cell_defs,
        cell_reads,
        must_defs,
        worker_static_cells,
        source_labels,
    }
}

fn collect_event_reads(
    fu: &FunctionUnit,
    contexts: &crate::variable_bindings::PointResolveContexts,
    commands: &CommandRegistry,
    uses_v0: &mut HashSet<String>,
    source_labels: &mut VariableCellTable<HashSet<String>>,
    cell_reads: &mut HashMap<EventCell, VariableCellSet>,
) {
    // The common place reader includes registry VarRead roles, nested
    // substitutions and conditions. Literal text never manufactures a read.
    for (block_id, block) in &fu.cfg.blocks {
        for (index, statement) in block.statements.iter().enumerate() {
            for place in
                crate::place_bridge::read_places_at(statement, *block_id, index, contexts, commands)
            {
                let Some(cell) = cell_from_place(&place) else {
                    continue;
                };
                let Some(key) = crate::var_resolve::canonical_binding_value_key(&place) else {
                    continue;
                };
                let labels = labels_at(fu, *block_id, index, &key, &place);
                uses_v0.extend(labels.iter().cloned());
                source_labels.entry(key.clone()).or_default().extend(labels);
                cell_reads.entry(cell).or_default().insert(key);
            }
        }
        if let Some(terminator) = &block.terminator {
            for place in crate::place_bridge::terminator_read_places_at(
                terminator, *block_id, contexts, commands,
            ) {
                let Some(cell) = cell_from_place(&place) else {
                    continue;
                };
                let Some(key) = crate::var_resolve::canonical_binding_value_key(&place) else {
                    continue;
                };
                let labels = labels_at(fu, *block_id, usize::MAX, &key, &place);
                uses_v0.extend(labels.iter().cloned());
                source_labels.entry(key.clone()).or_default().extend(labels);
                cell_reads.entry(cell).or_default().insert(key);
            }
        }
    }
}

fn definite_exit_cells(
    fu: &FunctionUnit,
    commands: &CommandRegistry,
    contexts: &crate::variable_bindings::PointResolveContexts,
    module: Option<&crate::ir::Module>,
) -> HashSet<EventCell> {
    use std::collections::VecDeque;
    let mut incoming: HashMap<crate::cfg::BlockId, HashSet<EventCell>> =
        HashMap::from([(fu.cfg.entry, HashSet::new())]);
    let mut outgoing = HashMap::new();
    let mut queue = VecDeque::from([fu.cfg.entry]);
    while let Some(id) = queue.pop_front() {
        let Some(block) = fu.cfg.blocks.get(&id) else {
            continue;
        };
        let before = incoming[&id].clone();
        let mut state = before.clone();
        if let Some(ssa) = fu.ssa.blocks.get(&id) {
            for (index, statement) in ssa.statements.iter().enumerate() {
                let context = contexts.before_statement(id, index);
                if statement_clobbers(fu, id, index, commands, module) {
                    // A proved native command without callbacks keeps existing
                    // cells. An unresolved or evaluating call can destroy them.
                    state.clear();
                }
                let Some(destroys) = statement_destroys(fu, id, index, commands, module) else {
                    state.clear();
                    continue;
                };
                let places = if destroys {
                    crate::place_bridge::statement_mutation_places_with_continuation(
                        &statement.statement,
                        context,
                        contexts.after_statement(id, index),
                        commands,
                    )
                } else {
                    crate::place_bridge::def_places_with_continuation(
                        &statement.statement,
                        context,
                        contexts.after_statement(id, index),
                        commands,
                    )
                };
                for place in places {
                    let Some(cell) = cell_from_place(&place) else {
                        continue;
                    };
                    if destroys {
                        state.remove(&cell);
                    } else if !place.observed
                        && crate::var_resolve::canonical_binding_value_key(&place).is_some_and(
                            |key| {
                                statement.defs.keys().any(|symbol| {
                                    !statement.may_defs.contains(symbol)
                                        && fu.ssa.cell_key(*symbol) == &key
                                })
                            },
                        )
                    {
                        state.insert(cell);
                    }
                }
            }
        }
        outgoing.insert(id, state.clone());
        for successor in block.successors() {
            propagate_definite_cells(&mut incoming, &mut queue, successor, &state);
        }
        // An exception can arise before any statement in this block completes.
        for &(source, handler) in &fu.cfg.exception_edges {
            if source == id {
                propagate_definite_cells(&mut incoming, &mut queue, handler, &before);
            }
        }
    }
    let mut exits = fu.cfg.blocks.iter().filter_map(|(id, block)| {
        if block.successors().is_empty() {
            outgoing.get(id).cloned()
        } else {
            None
        }
    });
    let Some(mut definite) = exits.next() else {
        return HashSet::new();
    };
    for exit in exits {
        definite.retain(|cell| exit.contains(cell));
    }
    definite
}

fn propagate_definite_cells(
    incoming: &mut HashMap<crate::cfg::BlockId, HashSet<EventCell>>,
    queue: &mut std::collections::VecDeque<crate::cfg::BlockId>,
    successor: crate::cfg::BlockId,
    state: &HashSet<EventCell>,
) {
    if let Some(previous) = incoming.get_mut(&successor) {
        let old = previous.len();
        previous.retain(|cell| state.contains(cell));
        if previous.len() != old {
            queue.push_back(successor);
        }
    } else {
        incoming.insert(successor, state.clone());
        queue.push_back(successor);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cross_event_namespace_cells_keep_native_incarnations_separate() {
        use crate::{
            command_binding::SourceNamespaceKey,
            place::{CellGeneration, CellIdentity, CellOwner},
        };
        use tcl_runtime_api::native_compilation::{
            NativeInterpreterIdentity, NativeNamespaceContext,
        };

        let interpreter = NativeInterpreterIdentity {
            owner: NativeInterpreterIdentity::fresh_owner(),
            interpreter: 0,
        };
        let native = |token| {
            let mut place = crate::place::scalar("x", "::static", false);
            place.cell = Some(CellIdentity {
                owner: CellOwner::NamespaceIdentity(Box::new(SourceNamespaceKey::Native(
                    NativeNamespaceContext {
                        interpreter,
                        token,
                        path: tcl_core_types::ByteNamespacePath::from_segments(["static"]),
                    },
                ))),
                name: "x".into(),
                generation: CellGeneration::Incoming,
                interpreter: None,
                storage_domain: Some(tcl_registry::f5::VariableStorageDomain::InterpreterNamespace),
                execution: None,
            });
            place
        };
        let original = cell_from_place(&native(1)).unwrap();
        let recreated = cell_from_place(&native(2)).unwrap();
        let authored = cell_from_place(&crate::place::scalar("x", "::static", false)).unwrap();
        assert_ne!(original, recreated);
        assert_ne!(original, authored);
        let definitions = HashMap::from([(original.clone(), VariableCellSet::default())]);
        assert!(definitions.contains_key(&cell_from_place(&native(1)).unwrap()));
        assert!(!definitions.contains_key(&recreated));
        assert!(!definitions.contains_key(&authored));
        let original_key = crate::var_resolve::cell_key(&native(1))
            .with_lifetime(4)
            .with_index("k");
        let recreated_key = crate::var_resolve::cell_key(&native(2))
            .with_lifetime(4)
            .with_index("k");
        let handler = "::when::HTTP_REQUEST".to_owned();
        let summary = EventVarSummary {
            event: "HTTP_REQUEST".to_owned(),
            defs: HashSet::new(),
            uses_before_def: HashSet::new(),
            unsets: HashSet::new(),
            cell_defs: HashMap::new(),
            cell_reads: HashMap::new(),
            must_defs: HashSet::new(),
            worker_static_cells: HashSet::new(),
            source_labels: [
                (
                    original_key.clone(),
                    HashSet::from(["static::x(k)".to_owned()]),
                ),
                (
                    recreated_key,
                    HashSet::from(["wrong_incarnation".to_owned()]),
                ),
                (
                    VariableCellKey::Authored(original_key.compatibility_name()),
                    HashSet::from(["forged_label".to_owned()]),
                ),
            ]
            .into_iter()
            .collect(),
        };
        let mut scope = ConnectionScope::default();
        scope.handlers.insert(handler.clone(), summary);
        scope
            .cross_event_import_keys
            .insert(handler.clone(), [original_key].into());
        scope
            .cross_event_cells
            .insert(handler.clone(), HashSet::from([original]));
        // Public labels cannot donate a cell or an imported source binding.
        scope.cross_event_imports.insert("forged_label".to_owned());
        assert_eq!(
            scope.handler_source_names(&handler, true),
            HashSet::from(["static::x(k)".to_owned()])
        );
        assert_eq!(
            scope.source_names(),
            HashSet::from(["static::x(k)".to_owned()])
        );
        assert!(scope.observes_cross_event_place(&handler, &native(1)));
        assert!(!scope.observes_cross_event_place(&handler, &native(2)));
        assert!(
            !scope.observes_cross_event_place(
                &handler,
                &crate::place::scalar("x", "::static", false)
            )
        );
        assert!(!scope.observes_cross_event_place("::when::HTTP_RESPONSE", &native(1)));
    }

    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn cu(source: &str) -> CompilationUnit {
        // `when` is registry-resolved.  This
        // module's tests all lower iRule code (`when
        // CLIENT_ACCEPTED { ... }`), so the test registry must
        // load iRules to make `when` resolve to `LoweringHookId
        // ::When`.  Without the load, `when` would fall through
        // to `lower_default` and no `::when::*` procedures would
        // be registered for the connection-scope builder to walk.
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::irules());
        CompilationUnit::build_for(source, &registry, false)
    }

    fn when_procs(cu: &CompilationUnit) -> HashMap<String, FunctionUnit> {
        cu.procedures
            .iter()
            .filter(|(_, unit)| unit.irules_event_body.is_some())
            .map(|(qn, fu)| (qn.clone(), fu.clone()))
            .collect()
    }

    fn supplied_unit(
        source: &str,
        profile: &'static tcl_dialect::DialectProfile,
        context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
        native: bool,
    ) -> (CompilationUnit, Option<tcl_vm::Vm>) {
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(context),
            config,
        );
        let (owner, entry) = if native {
            let (owner, entry) =
                crate::environment_ingress::captured_native_entry_with_owner(profile);
            (
                Some(owner),
                Some(crate::command_binding::SourceAnalysisEntry {
                    native_entry: Some(std::sync::Arc::new(entry)),
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                }),
            )
        } else {
            (None, None)
        };
        let unit = CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            entry.as_ref(),
            &input,
        );
        (unit, owner)
    }

    fn original_position(function: &FunctionUnit, head: &str) -> (crate::cfg::BlockId, usize) {
        function
            .cfg
            .blocks
            .iter()
            .find_map(|(&block, data)| {
                data.statements.iter().enumerate().find_map(|(index, _)| {
                    let tokens = function.cfg.source_tokens_at(block, index)?;
                    (tokens.argv_texts.first().is_some_and(|word| word == head))
                        .then_some((block, index))
                })
            })
            .expect("original complete statement carrier")
    }

    #[test]
    fn original_connection_destruction_keeps_current_availability_and_source_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // The actual entry retains native source lookup. This query describes
        // selected destruction; it does not execute an unset or supply a cell.
        use crate::analyser::ResolvedAnalysisInput;
        use std::sync::Arc;
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut registry = CommandRegistry::build_default().project_for_profile(profile);
        let mut unset = registry.get("unset").unwrap().clone();
        unset.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(unset);
        let current = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.6")
                .with_command_store(Arc::new(registry)),
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.commands())),
        );
        assert!(Arc::ptr_eq(current.commands(), older.commands()));
        let (unit, _native_owner) = supplied_unit(
            "set {$literal} VALUE; unset {$literal}",
            profile,
            &current,
            true,
        );
        let function = &unit.top_level;
        let (block, index) = original_position(function, "unset");
        assert_eq!(
            statement_destroys(
                function,
                block,
                index,
                current.commands(),
                Some(&unit.ir_module)
            ),
            Some(true)
        );
        assert!(!statement_clobbers(
            function,
            block,
            index,
            current.commands(),
            Some(&unit.ir_module)
        ));
        let tokens = function.cfg.source_tokens_at(block, index).unwrap();
        let config = function.source_lexer_config();
        for facet in 0..4 {
            let mut changed = function.clone();
            match facet {
                0 => changed.source_metadata_input = None,
                1 => {
                    changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
                        profile,
                        profile,
                        Arc::clone(&older),
                        config,
                    ))
                }
                2 => {
                    changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
                        profile,
                        profile,
                        tcl_registry::model::ingress::resolve_environment("tcl9.1")
                            .default_context_registry(),
                        config,
                    ))
                }
                3 => changed.source_config.strict_quoting = !changed.source_config.strict_quoting,
                _ => unreachable!(),
            }
            assert!(
                point_metadata(&changed, tokens, current.commands(), Some(&unit.ir_module))
                    .is_none(),
                "changed point facet {facet}"
            );
            assert_eq!(
                statement_destroys(
                    &changed,
                    block,
                    index,
                    current.commands(),
                    Some(&unit.ir_module)
                ),
                None,
                "changed destruction facet {facet}"
            );
            assert!(
                statement_clobbers(
                    &changed,
                    block,
                    index,
                    current.commands(),
                    Some(&unit.ir_module)
                ),
                "changed clobber facet {facet}"
            );
        }
        let mut stale = unit.ir_module.clone();
        stale.top_level_namespace = "::stale".into();
        assert_eq!(
            statement_destroys(function, block, index, current.commands(), Some(&stale)),
            None
        );
        let mut changed = tokens.clone();
        changed.word_exprs.pop();
        assert!(
            point_metadata(
                function,
                &changed,
                current.commands(),
                Some(&unit.ir_module)
            )
            .is_none()
        );
    }

    #[test]
    fn original_connection_destruction_keeps_captured_aliases_and_unknown_members_separate() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Genuine Logical selected source operations grant no Native handler,
        // successful mutation, event worker, storage identity or definite exit.
        use std::sync::Arc;
        let profile = tcl_dialect::DialectProfile::find("tcl").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let current = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.6")
                .with_command_store(Arc::new(registry)),
        );
        for (source, head) in [
            (
                "interp alias {} erase {} array unset; erase {$literal}",
                "erase",
            ),
            ("rename array moved; moved unset {$literal}", "moved"),
        ] {
            let (unit, _owner) = supplied_unit(source, profile, &current, false);
            let (block, index) = original_position(&unit.top_level, head);
            let tokens = unit.top_level.cfg.source_tokens_at(block, index).unwrap();
            assert!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .proved_execution_target()
                    .is_none()
            );
            assert_eq!(
                statement_destroys(
                    &unit.top_level,
                    block,
                    index,
                    current.commands(),
                    Some(&unit.ir_module)
                ),
                Some(true),
                "original source {source}"
            );
        }
        for source in [
            "array $member {$literal}",
            "proc array {args} {}; array unset {$literal}",
        ] {
            let (unit, _owner) = supplied_unit(source, profile, &current, false);
            let (block, index) = original_position(&unit.top_level, "array");
            assert_eq!(
                statement_destroys(
                    &unit.top_level,
                    block,
                    index,
                    current.commands(),
                    Some(&unit.ir_module)
                ),
                None,
                "unselected member or shadow {source}"
            );
            assert!(statement_clobbers(
                &unit.top_level,
                block,
                index,
                current.commands(),
                Some(&unit.ir_module)
            ));
        }
    }

    #[test]
    fn original_connection_summary_requires_the_hosted_module_and_function_owner() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Retained appliance authoring policy identifies a source handler; it
        // supplies no executing TMM worker or current connection-frame slots.
        let environment = tcl_registry::model::ingress::resolve_environment("f5-irules");
        let context = environment.default_context_registry();
        let (unit, _owner) = supplied_unit(
            "when HTTP_REQUEST {set local VALUE}",
            environment.unit_profile(),
            &context,
            false,
        );
        let procedures = when_procs(&unit);
        let function = procedures.values().next().expect("actual event body owner");
        assert!(function_metadata(function, context.commands(), Some(&unit.ir_module)).is_some());
        let scope =
            build_connection_scope_for_module(&procedures, context.commands(), &unit.ir_module);
        assert!(scope.summaries.contains_key("HTTP_REQUEST"));
        let mut missing = procedures.clone();
        for function in missing.values_mut() {
            function.source_metadata_input = None;
        }
        assert!(
            build_connection_scope_for_module(&missing, context.commands(), &unit.ir_module)
                .summaries
                .is_empty()
        );
        let mut stale = unit.ir_module.clone();
        stale.top_level_namespace = "::other".into();
        assert!(
            build_connection_scope_for_module(&procedures, context.commands(), &stale)
                .summaries
                .is_empty()
        );
        let mut absent = unit.ir_module.clone();
        absent.source_metadata_input = None;
        assert!(
            build_connection_scope_for_module(&procedures, context.commands(), &absent)
                .summaries
                .is_empty()
        );
    }

    #[test]
    // Implementation contract: naming.variable.registry-event-frame-identity
    // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
    fn original_event_frames_require_the_selected_body_and_not_function_labels() {
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::irules());
        let source = "when HTTP_REQUEST {set local 1}";
        let unit = CompilationUnit::build_for(source, &registry, false);
        let event = unit.procedures.get("::when::HTTP_REQUEST").unwrap();
        let body = event
            .irules_event_body
            .as_ref()
            .expect("selected original event body");
        assert_eq!(body.event(), "HTTP_REQUEST");
        assert!(body.matches_source(
            &tcl_lexer::SourceImage::document(source),
            unit.ir_module.lexer_config
        ));
        assert!(!body.matches_source(
            &tcl_lexer::SourceImage::document(&format!("{source}# changed")),
            unit.ir_module.lexer_config
        ));
        let mut changed_config = unit.ir_module.lexer_config;
        changed_config.strict_quoting = !changed_config.strict_quoting;
        assert!(!body.matches_source(&tcl_lexer::SourceImage::document(source), changed_config));
        let points = event
            .ssa
            .point_contexts
            .as_ref()
            .expect("actual event point contexts");
        let (&first, _) = event
            .cfg
            .blocks
            .iter()
            .find(|(_, block)| !block.statements.is_empty())
            .unwrap();
        let actual_entry = points.context_before(first, 0).unwrap();
        assert_eq!(
            actual_entry.frame_kind,
            crate::var_resolve::VariableFrameKind::Local
        );
        assert_eq!(
            actual_entry.hosted_execution_context,
            Some(tcl_registry::f5::BigIpExecutionContext::TmmIRule)
        );
        assert_eq!(actual_entry.execution, None);
        let entry = body.conditional_entry();
        assert_eq!(
            entry.frame_kind,
            crate::var_resolve::VariableFrameKind::Local
        );
        assert_eq!(
            entry.hosted_execution_context,
            Some(tcl_registry::f5::BigIpExecutionContext::TmmIRule)
        );
        assert_eq!(entry.execution, None);
        assert!(entry.constant_values.is_empty());
        let handlers =
            HashMap::from([("report label unrelated to event".to_owned(), event.clone())]);
        let scope = build_connection_scope_with_registry(&handlers, &registry);
        assert!(scope.summaries.contains_key("HTTP_REQUEST"));
        assert!(
            !scope
                .summaries
                .contains_key("report label unrelated to event")
        );
        let mut unowned = event.clone();
        unowned.irules_event_body = None;
        let scope = build_connection_scope_with_registry(
            &HashMap::from([("::when::RULE_INIT".to_owned(), unowned)]),
            &registry,
        );
        assert!(
            scope.summaries.is_empty(),
            "a display label cannot manufacture event policy"
        );
    }

    #[test]
    // Implementation contract: naming.variable.registry-event-frame-identity
    // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
    fn original_procedure_prefix_and_foreign_body_cannot_select_event_storage() {
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::irules());
        let source = "namespace eval ::when {}\nproc ::when::HTTP_REQUEST {} {set local 1}";
        let unit = CompilationUnit::build_for(source, &registry, false);
        assert!(unit.ir_module.irules_event_bodies.is_empty());
        assert!(
            unit.procedures
                .values()
                .all(|function| function.irules_event_body.is_none())
        );
        let source = "when HTTP_REQUEST {set local 1}";
        let module = crate::lowering::lower_to_ir(source, &registry);
        let body = module
            .irules_event_bodies
            .get("::when::HTTP_REQUEST")
            .unwrap();
        let procedure = module.procedures.get("::when::HTTP_REQUEST").unwrap();
        assert!(body.owns_procedure(procedure, &module.source, module.lexer_config, &registry));
        let mut changed = procedure.clone();
        changed.span = tcl_lexer::Span::new(procedure.span.start() + 1, procedure.span.end());
        assert!(!body.owns_procedure(&changed, &module.source, module.lexer_config, &registry));
        changed = procedure.clone();
        changed.body.statements.clear();
        assert!(!body.owns_procedure(&changed, &module.source, module.lexer_config, &registry));
        let foreign = CommandRegistry::build_default();
        assert!(!body.owns_procedure(procedure, &module.source, module.lexer_config, &foreign));
    }

    #[test]
    fn cross_event_labels_follow_exact_native_operation_symbols() {
        use crate::command_binding::SourceNamespaceKey;
        use tcl_runtime_api::native_compilation::{
            NativeInterpreterIdentity, NativeNamespaceContext,
        };

        let unit = cu("when HTTP_REQUEST { set static::x 1; log local0. $static::x }");
        let mut fu = when_procs(&unit).into_values().next().unwrap();
        let (block, index, symbol) = fu
            .cfg
            .blocks
            .iter()
            .find_map(|(&block, data)| {
                (0..data.statements.len()).find_map(|index| {
                    fu.ssa
                        .source_symbols_at(block, index)?
                        .get("static::x")
                        .map(|&symbol| (block, index, symbol))
                })
            })
            .expect("original written variable has a positioned SSA symbol");
        let interpreter = NativeInterpreterIdentity {
            owner: NativeInterpreterIdentity::fresh_owner(),
            interpreter: 0,
        };
        let native_key = |token| VariableCellKey::Namespace {
            identity: SourceNamespaceKey::Native(NativeNamespaceContext {
                interpreter,
                token,
                path: tcl_core_types::ByteNamespacePath::from_segments(["static"]),
            }),
            simple: "x".into(),
        };
        let selected = native_key(1);
        let relocation = crate::var_resolve::VariableProofRelocation {
            storage_keys: HashMap::from([(fu.ssa.cell_key(symbol).clone(), selected.clone())]),
            ..Default::default()
        };
        fu.ssa.relocate_variable_proofs(&relocation);
        let place = crate::place::scalar("x", "::static", false);
        assert_eq!(
            labels_at(&fu, block, index, &selected, &place),
            HashSet::from(["static::x".to_owned()])
        );
        assert!(labels_at(&fu, block, index, &native_key(2), &place).is_empty());
        assert!(
            labels_at(
                &fu,
                block,
                index,
                &VariableCellKey::Authored(selected.compatibility_name()),
                &place
            )
            .iter()
            .all(|label| label != "static::x")
        );
    }

    #[test]
    fn cross_event_cells_keep_native_bytes_and_supplied_execution_context() {
        // Implementation contract: naming.variable.event-cell-execution-isolation
        // docs/design/analysis/name-resolution-proofs/variable-event-cell-execution-isolation.md
        use crate::place::{CellGeneration, CellIdentity, CellOwner};
        use tcl_registry::f5::WorkerExecution;
        let local = |bytes: &[u8], execution| {
            let mut place = crate::place::scalar("SAME_REPORT", crate::place::LOCAL_NS, false);
            place.cell = Some(CellIdentity {
                owner: CellOwner::Activation("authored handler".into()),
                name: bytes.into(),
                generation: CellGeneration::Incoming,
                interpreter: None,
                storage_domain: None,
                execution,
            });
            place
        };
        let first = cell_from_place(&local(b"v\xed\xa0\x80", None)).unwrap();
        let other = cell_from_place(&local(b"v\xed\xa0\x81", None)).unwrap();
        assert_ne!(first, other, "display equality cannot merge opaque roots");
        assert!(first.is_connection());
        let execution = WorkerExecution {
            worker: Some(1),
            initialisation_epoch: 2,
            connection: Some(3),
        };
        let selected = cell_from_place(&local(b"x", Some(execution))).unwrap();
        assert!(selected.is_connection());
        assert_eq!(
            selected,
            cell_from_place(&local(b"x", Some(execution))).unwrap()
        );
        for other in [
            WorkerExecution {
                worker: Some(4),
                ..execution
            },
            WorkerExecution {
                initialisation_epoch: 5,
                ..execution
            },
            WorkerExecution {
                connection: Some(6),
                ..execution
            },
            WorkerExecution {
                worker: None,
                ..execution
            },
        ] {
            assert_ne!(
                selected,
                cell_from_place(&local(b"x", Some(other))).unwrap()
            );
        }
        assert_ne!(selected, cell_from_place(&local(b"x", None)).unwrap());
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let namespace = |execution| {
            let mut context = crate::var_resolve::ResolveContext::for_function("::event");
            context.known_namespaces.insert("::static".into());
            context.execution = Some(execution);
            crate::var_resolve::resolve_literal_place("::static::x", &context, false, registry)
        };
        let initial = cell_from_place(&namespace(execution)).unwrap();
        assert!(!initial.is_connection());
        assert_eq!(
            initial,
            cell_from_place(&namespace(WorkerExecution {
                connection: Some(99),
                ..execution
            }))
            .unwrap(),
            "worker namespace ownership excludes connection identity"
        );
        assert_ne!(
            initial,
            cell_from_place(&namespace(WorkerExecution {
                worker: Some(4),
                ..execution
            }))
            .unwrap()
        );
        assert_ne!(
            initial,
            cell_from_place(&namespace(WorkerExecution {
                initialisation_epoch: 5,
                ..execution
            }))
            .unwrap()
        );
    }

    #[test]
    fn build_connection_scope_empty_when_no_when_procs() {
        let cu = cu("proc foo {} {}");
        let cs = build_connection_scope(&when_procs(&cu));
        assert!(cs.summaries.is_empty());
        assert!(cs.cross_event_defs.is_empty());
        assert!(cs.cross_event_imports.is_empty());
        assert!(cs.racy_static_defs.is_empty());
    }

    #[test]
    fn build_connection_scope_records_defs_and_uses() {
        // Two when blocks: CLIENT_ACCEPTED writes ``ip``,
        // HTTP_REQUEST reads ``ip``.  Cross-event flow is
        // valid (CLIENT_ACCEPTED fires before HTTP_REQUEST).
        let source = "
            when CLIENT_ACCEPTED { set ip [IP::client_addr] }
            when HTTP_REQUEST { log local0. \"$ip\" }
        ";
        let cu = cu(source);
        let cs = build_connection_scope(&when_procs(&cu));
        // Each handler keeps its own captured SSA proof key, while the
        // lifecycle joins their common physical connection cell.
        let cell = EventCell::Connection("ip".into());
        assert!(
            cs.summaries["CLIENT_ACCEPTED"].cell_defs[&cell]
                .iter()
                .all(|key| cs
                    .cross_event_def_keys
                    .values()
                    .any(|keys| keys.contains(key))),
            "{cs:?}"
        );
        assert!(
            cs.summaries["HTTP_REQUEST"].cell_reads[&cell]
                .iter()
                .all(|key| cs
                    .cross_event_import_keys
                    .values()
                    .any(|keys| keys.contains(key))),
            "{cs:?}"
        );
        assert!(cs.cross_event_defs.contains("ip"));
        assert!(cs.cross_event_imports.contains("ip"));
        assert!(cs.source_names().contains("ip"));
        assert!(
            cs.cross_event_defs
                .iter()
                .all(|name| !name.starts_with("@frame:"))
        );
        // No static:: var ⇒ no racy_static_defs.
        assert!(cs.racy_static_defs.is_empty());
    }

    #[test]
    fn build_connection_scope_flags_racy_static() {
        // ``static::counter`` written in HTTP_REQUEST and read
        // in HTTP_RESPONSE — both per-request events; the
        // cross-event flow is racy.
        let source = "
            when HTTP_REQUEST { incr static::counter }
            when HTTP_RESPONSE { log local0. \"$static::counter\" }
        ";
        let cu = cu(source);
        let cs = build_connection_scope(&when_procs(&cu));
        assert!(cs.racy_static_defs.contains("::static::counter"));
    }

    #[test]
    fn build_connection_scope_skips_rule_init_static() {
        // ``static::`` written in RULE_INIT is **not** racy —
        // RULE_INIT runs once at iRule load.
        let source = "
            when RULE_INIT { set static::config 1 }
            when HTTP_REQUEST { log local0. \"$static::config\" }
        ";
        let cu = cu(source);
        let cs = build_connection_scope(&when_procs(&cu));
        assert!(!cs.racy_static_defs.contains("::static::config"));
    }

    #[test]
    fn build_connection_scope_reads_unset_through_its_canonical_name() {
        // Lowering stamps the resolved `::unset` on the statement, so a
        // surface-only `command == "unset"` test misses the globally
        // qualified spelling and mistakes the unset for a live def.
        let unsets_for = |source: &str| {
            let cu = cu(source);
            build_connection_scope(&when_procs(&cu))
                .summaries
                .get("HTTP_REQUEST")
                .expect("HTTP_REQUEST summary")
                .unsets
                .clone()
        };
        let bare = unsets_for("when HTTP_REQUEST { set static::c 1\nunset static::c }");
        let qualified = unsets_for("when HTTP_REQUEST { set static::c 1\n::unset static::c }");
        assert!(
            bare.contains("::static::c"),
            "bare spelling records the unset"
        );
        assert_eq!(bare, qualified, "both spellings are the same command");
    }

    #[test]
    fn build_connection_scope_merges_duplicate_events() {
        // Two ``when CLIENT_ACCEPTED`` blocks — their summaries
        // are merged.
        let source = "
            when CLIENT_ACCEPTED { set a 1 }
            when CLIENT_ACCEPTED { set b 2 }
            when HTTP_REQUEST { log local0. \"$a $b\" }
        ";
        let cu = cu(source);
        let cs = build_connection_scope(&when_procs(&cu));
        let s = cs
            .summaries
            .get("CLIENT_ACCEPTED")
            .expect("CLIENT_ACCEPTED summary");
        assert!(
            s.cell_defs.contains_key(&EventCell::Connection("a".into()))
                && s.cell_defs.contains_key(&EventCell::Connection("b".into())),
            "{s:?}"
        );
    }
    #[test]
    fn initialisation_globals_do_not_define_connection_locals() {
        let unit = cu("when RULE_INIT {set config 1}\nwhen HTTP_REQUEST {log local0. $config}");
        let scope = build_connection_scope(&when_procs(&unit));
        assert!(!scope.cross_event_imports.contains("config"));
        assert!(
            scope.summaries["RULE_INIT"]
                .cell_defs
                .contains_key(&EventCell::Namespace("::config".into()),)
        );
    }

    #[test]
    fn absolute_and_relative_static_spellings_share_the_worker_cell() {
        let unit = cu(
            "when RULE_INIT {set static::app_flag 1}\nwhen HTTP_REQUEST {log local0. $::static::app_flag}",
        );
        let scope = build_connection_scope(&when_procs(&unit));
        assert!(scope.cross_event_defs.contains("::static::app_flag"));
        assert!(scope.cross_event_imports.contains("::static::app_flag"));
    }

    #[test]
    fn conditional_assignment_is_a_may_definition() {
        let unit = cu(
            "when CLIENT_ACCEPTED {if {[IP::client_addr] eq {x}} {set flag 1}}\nwhen HTTP_REQUEST {log local0. $flag}",
        );
        let scope = build_connection_scope(&when_procs(&unit));
        let summary = &scope.summaries["CLIENT_ACCEPTED"];
        assert!(
            summary
                .cell_defs
                .contains_key(&EventCell::Connection("flag".into()))
        );
        assert!(
            !summary
                .must_defs
                .contains(&EventCell::Connection("flag".into()))
        );
    }

    #[test]
    fn unknown_connection_subject_preserves_possible_switch_body_definitions() {
        let sources = [
            "when CLIENT_ACCEPTED {switch $mode {loud {set debug 1} default {set debug 0}}}\nwhen HTTP_REQUEST {if {$debug} {log local0. hi}}",
            "when CLIENT_ACCEPTED { switch $mode { loud { set debug 1 } default { set debug 0 } } }\nwhen HTTP_REQUEST { if {$debug} { log local0. hi } }",
        ];
        for source in sources {
            let unit = cu(source);
            let scope = build_connection_scope(&when_procs(&unit));
            let debug = EventCell::Connection("debug".into());
            assert!(
                scope.summaries["CLIENT_ACCEPTED"]
                    .cell_defs
                    .contains_key(&debug),
                "scope: {scope:?}; dispatches: {:?}",
                possible_body_dispatch_evidence(&unit)
            );
            assert!(
                !scope.summaries["CLIENT_ACCEPTED"]
                    .must_defs
                    .contains(&debug)
            );
            assert!(
                scope.summaries["CLIENT_ACCEPTED"].cell_defs[&debug]
                    .iter()
                    .all(|key| scope
                        .cross_event_def_keys
                        .values()
                        .any(|keys| keys.contains(key))),
                "{scope:?}"
            );
            assert!(
                scope.summaries["HTTP_REQUEST"].cell_reads[&debug]
                    .iter()
                    .all(|key| scope
                        .cross_event_import_keys
                        .values()
                        .any(|keys| keys.contains(key))),
                "{scope:?}"
            );
        }
    }

    fn possible_body_dispatch_evidence(unit: &CompilationUnit) -> Vec<String> {
        let registry = unit.ir_module.resolved_registry();
        let mut evidence = Vec::new();
        for (name, function) in &unit.procedures {
            for block in function.cfg.blocks.values() {
                for statement in &block.statements {
                    let Some(tokens) = statement.tokens() else {
                        continue;
                    };
                    let Some(binding) = &tokens.source_binding else {
                        continue;
                    };
                    let topology = crate::registry_invocation::possible_body_invocation(
                        registry, None, tokens,
                    )
                    .map(|body| body.topology);
                    let handler_layout = crate::registry_invocation::resolved_handler_invocation(
                        registry, None, tokens,
                    )
                    .map(|invocation| {
                        format!(
                            "contract={:?}, roles={}, arity={:?}, frame={:?}, words={:?}",
                            invocation.facts.successful_handler,
                            invocation.facts.arg_roles_complete,
                            invocation.facts.arity_accepts_frozen_arguments(),
                            binding.variable_context.alias_frame(),
                            invocation.evaluated_words
                        )
                    });
                    evidence.push(format!(
                        "{name} {:?}: reached={:?}, unknown={}, frame={:?}, contents={:?}, handler={:?}, topology={topology:?}, layout={handler_layout:?}",
                        tokens.argv_texts,
                        binding.runtime_reachability(),
                        binding.unknown,
                        binding.variable_frame,
                        binding.variable_context.contents_world,
                        binding.proved_handler_target().map(|target| &target.command),
                    ));
                }
            }
        }
        evidence
    }

    #[test]
    fn literal_info_exists_text_is_not_a_cross_event_read() {
        let unit = cu(
            "when CLIENT_ACCEPTED {set value 1}\nwhen HTTP_REQUEST {log local0. {info exists value}}",
        );
        let scope = build_connection_scope(&when_procs(&unit));
        assert!(!scope.cross_event_defs.contains("value"));
    }
}
