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

//! Original declaration completion/read graph for conditional diagnostics.
//!
//! The graph owns lexical command layouts, not executed statements or SSA.
//! Missing observations preserve unknown writers. Body sources and command
//! sites retain the original image, logical grammar and declaration frame.

pub(super) mod representation;

use super::declaration_layout::DeclarationLayouts;
use super::{CommandAllocationSite, ExecutedScriptSource, SourceInvocationBinding};
use crate::ir::CommandTokens;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};
use tcl_registry::completion::CompletionCode;
use tcl_registry::completion_route::InvocationCompletionRoute as Route;
#[cfg(test)]
use tcl_source::words::WordExpr;
#[cfg(test)]
use tcl_source::words::WordPart;

mod command_flow;
mod evaluation;
mod events;
mod report_queries;
mod representation_plan;
mod selection;

#[cfg(test)]
use events::Event;
#[cfg(test)]
use events::logical_boolean_literal_condition;
use report_queries::original_procedure_entry_matches;

#[derive(Debug)]
pub(super) struct DeclarationFlowInventory {
    reports: DeclarationFlowReports,
    source_seed: Option<super::source_transition_advice::OriginalSourceTransitionSeed>,
    source_catalogue: OnceLock<Option<Arc<super::source_read_scope::SourceReadCatalogue>>>,
    representation_catalogue: OnceLock<Option<Arc<super::source_read_scope::SourceReadCatalogue>>>,
    source_catalogue_memo: Arc<super::source_read_scope::SourceReadCatalogueMemo>,
    layouts: Arc<DeclarationLayouts>,
    accesses: Arc<super::SourceVariableAccesses>,
    origin_accesses: Arc<super::OriginVariableAccesses>,
    points: Arc<Vec<super::SourceBindingPoint>>,
    origin_points: Arc<BTreeMap<Arc<super::SourceOriginId>, Vec<super::SourceBindingPoint>>>,
}

impl DeclarationFlowInventory {
    fn source_read_catalogue(
        &self,
        registry: &tcl_registry::CommandRegistry,
        purpose: DeclarationFlowPurpose,
    ) -> Option<&super::source_read_scope::SourceReadCatalogue> {
        let seed = self.source_seed.as_ref()?;
        // Foreign requests cannot poison the independently retained cache.
        seed.matches_registry(registry).then_some(())?;
        match purpose {
            DeclarationFlowPurpose::Reads => self
                .source_catalogue
                .get_or_init(|| self.source_catalogue_memo.get_or_prepare(seed, registry))
                .as_deref(),
            DeclarationFlowPurpose::Representation => self
                .representation_catalogue
                .get_or_init(|| {
                    super::source_read_scope::SourceReadCatalogue::prepare(seed, registry)
                        .map(Arc::new)
                })
                .as_deref(),
        }
    }

    pub(super) fn new(
        layouts: Arc<DeclarationLayouts>,
        accesses: Arc<super::SourceVariableAccesses>,
        origin_accesses: Arc<super::OriginVariableAccesses>,
        points: Arc<Vec<super::SourceBindingPoint>>,
        origin_points: Arc<BTreeMap<Arc<super::SourceOriginId>, Vec<super::SourceBindingPoint>>>,
    ) -> Self {
        Self {
            reports: DeclarationFlowReports::default(),
            source_seed: None,
            source_catalogue: OnceLock::new(),
            representation_catalogue: OnceLock::new(),
            source_catalogue_memo: Arc::default(),
            layouts,
            accesses,
            origin_accesses,
            points,
            origin_points,
        }
    }
}
// Derived reports never participate in source-inventory identity.
impl PartialEq for DeclarationFlowInventory {
    fn eq(&self, other: &Self) -> bool {
        self.source_seed == other.source_seed
            && match (&self.source_seed, &other.source_seed) {
                (Some(left), Some(right)) => {
                    Arc::ptr_eq(&left.origin, &right.origin)
                        && left.analysis_owner.is_some() == right.analysis_owner.is_some()
                }
                (None, None) => true,
                _ => false,
            }
            && self.layouts == other.layouts
            && self.accesses == other.accesses
            && self.origin_accesses == other.origin_accesses
            && self.points == other.points
            && self.origin_points == other.origin_points
    }
}
impl Eq for DeclarationFlowInventory {}

impl DeclarationFlowInventory {
    /// Equality, rejecting inventories of different sizes before any deep walk.
    fn shares_inputs(&self, other: &Self) -> bool {
        self.layouts.len() == other.layouts.len()
            && self.accesses.len() == other.accesses.len()
            && self.origin_accesses.len() == other.origin_accesses.len()
            && self.points.len() == other.points.len()
            && self.origin_points.len() == other.origin_points.len()
            && self == other
    }
}

/// Derived inventory retained by this builder, plus the last one any clone of
/// it built. A fresh clone adopts the shared inventory (and the reports it
/// holds) only when a fresh inventory would be equal to it.
#[derive(Default)]
pub(super) struct SourceDeclarationFlowCache(
    Mutex<Option<Arc<DeclarationFlowInventory>>>,
    Arc<super::source_read_scope::SourceReadCatalogueMemo>,
    Arc<Mutex<Option<Arc<DeclarationFlowInventory>>>>,
);
impl Clone for SourceDeclarationFlowCache {
    fn clone(&self) -> Self {
        Self(Mutex::new(None), Arc::clone(&self.1), Arc::clone(&self.2))
    }
}
impl std::fmt::Debug for SourceDeclarationFlowCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceDeclarationFlowCache")
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DeclarationFlowFrame {
    Existing(super::declaration_layout::OriginalDiagnosticFrameEntry),
    Read(Arc<super::declaration_preview::OriginalProcedureReadFlowEntry>),
}
impl DeclarationFlowFrame {
    fn origin(&self) -> &Arc<super::SourceOriginId> {
        match self {
            Self::Existing(entry) => &entry.source().origin,
            Self::Read(entry) => &entry.header.declaration.source,
        }
    }
    fn owns_read(&self, origin: &Arc<super::SourceOriginId>, span: tcl_lexer::Span) -> bool {
        match self {
            Self::Existing(entry) => entry.owns_source(origin, span.start()),
            Self::Read(entry) => {
                &entry.header.declaration.source == origin && entry.read.owns_original_read(span)
            }
        }
    }
    fn owns_original_context(&self, context: &crate::var_resolve::ResolveContext) -> bool {
        match self {
            Self::Existing(entry) => entry.owns_original_context(context),
            Self::Read(entry) => super::SourceCommandBindings::context_owns_frame(
                context,
                &entry.header.frame,
                Some(&entry.header.namespace),
            ),
        }
    }
    fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        match self {
            Self::Existing(entry) => entry.parameters(),
            Self::Read(entry) => &entry.header.parameters,
        }
    }
    fn original_formal_topology(&self) -> Option<&super::formal_topology::OriginalFormalTopology> {
        match self {
            Self::Existing(entry) => entry.original_formal_topology(),
            Self::Read(entry) => entry.header.original_parameters.as_ref(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct DeclarationFlowKey {
    entry: Arc<DeclarationFlowFrame>,
    config: tcl_lexer::LexerConfig,
    dialect: tcl_registry::InvocationDialect,
    registry: tcl_registry::RegistrySemanticKey,
    purpose: DeclarationFlowPurpose,
    retained_module_realm: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DeclarationFlowPurpose {
    Reads,
    Representation,
}
/// One memoised declaration-flow report per selected key.
type DeclarationFlowEntries = Vec<(DeclarationFlowKey, Arc<DeclarationFlowReport>)>;

/// Reports of one inventory; equal inventories may share them.
#[derive(Default)]
struct DeclarationFlowReports(Arc<Mutex<DeclarationFlowEntries>>);
impl std::fmt::Debug for DeclarationFlowReports {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeclarationFlowReports")
            .finish_non_exhaustive()
    }
}
const MAX_DECLARATION_FLOW_REPORTS: usize = 64;
impl DeclarationFlowReports {
    fn get(&self, key: &DeclarationFlowKey) -> Option<Arc<DeclarationFlowReport>> {
        self.0
            .lock()
            .expect("declaration report cache poisoned")
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, report)| Arc::clone(report))
    }
    fn retain(
        &self,
        key: DeclarationFlowKey,
        report: DeclarationFlowReport,
    ) -> Arc<DeclarationFlowReport> {
        let mut reports = self.0.lock().expect("declaration report cache poisoned");
        if let Some((_, report)) = reports.iter().find(|(candidate, _)| candidate == &key) {
            return Arc::clone(report);
        }
        if reports.len() == MAX_DECLARATION_FLOW_REPORTS {
            reports.remove(0);
        }
        let report = Arc::new(report);
        reports.push((key, Arc::clone(&report)));
        report
    }
}
impl super::SourceCommandBindings {
    /// Conditional source-only representation observations from this exact
    /// retained module. No executable or physical-value fact is returned.
    pub(crate) fn conditional_representation_observations(
        &self,
        module: &crate::ir::Module,
        registry: &tcl_registry::CommandRegistry,
    ) -> Vec<super::SourceConversionObservation> {
        let Some(input) = module.source_metadata_input.as_ref() else {
            return Vec::new();
        };
        let source_current =
            self.matches_original_source_image(&module.source, input.lexer_config());
        let owner_current = self
            .original_analysis_owner()
            .is_some_and(|owner| owner.matches(self, input));
        if !source_current || !owner_current {
            return Vec::new();
        }
        let Some(realm) = crate::realm::document_realm_bindings_for_module(module, registry) else {
            return Vec::new();
        };
        for (site, observations) in &self.declaration_layouts {
            let Some(rows) = super::declaration_layout::original_declaration_layouts(observations)
            else {
                continue;
            };
            if !rows.clone().all(|row| {
                matches!(
                    row.entry.as_ref(),
                    super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { .. }
                )
            }) {
                continue;
            }
            let binding = self.invocation_at_source("", site.offset);
            let Some((_, mut tokens)) = binding.original_recorded_command() else {
                continue;
            };
            self.stamp_original_tokens(&mut tokens);
            let Some(report) = tokens.source_binding.as_ref().and_then(|binding| {
                binding.declaration_representation_flow_report(
                    &tokens,
                    input,
                    registry,
                    Some(&realm),
                )
            }) else {
                continue;
            };
            return report.representation_occurrences.clone();
        }
        Vec::new()
    }

    pub(super) fn current_declaration_flow_inventory(&self) -> Arc<DeclarationFlowInventory> {
        let mut next = DeclarationFlowInventory::new(
            self.declaration_layouts.shared(),
            self.variable_accesses.shared(),
            self.origin_variable_accesses.shared(),
            self.points.shared(),
            self.origin_points.shared(),
        );
        next.source_seed = self.original_source_transition_seed();
        next.source_catalogue_memo = Arc::clone(&self.declaration_flow_cache.1);
        let mut current = self
            .declaration_flow_cache
            .0
            .lock()
            .expect("declaration inventory cache poisoned");
        if let Some(current) = current.as_ref().filter(|current| current.as_ref() == &next) {
            return Arc::clone(current);
        }
        let mut shared = self
            .declaration_flow_cache
            .2
            .lock()
            .expect("shared declaration inventory cache poisoned");
        // Only a fresh clone consults its siblings' inventory; a builder that
        // has moved on would mostly compare against unequal inventories.
        if current.is_none()
            && let Some(adopted) = shared.as_ref().filter(|shared| shared.shares_inputs(&next))
        {
            // Keep this builder's own handles, so its later checks stay
            // shallow, and share the equal sibling's reports.
            next.reports = DeclarationFlowReports(Arc::clone(&adopted.reports.0));
            let next = Arc::new(next);
            *current = Some(Arc::clone(&next));
            return next;
        }
        let next = Arc::new(next);
        *current = Some(Arc::clone(&next));
        *shared = Some(Arc::clone(&next));
        next
    }
}

impl Hash for DeclarationFlowInventory {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // The other invocation axes discriminate source observations. Full
        // structural equality still validates this shared diagnostic inventory.
        self.layouts.len().hash(state);
    }
}

/// Conditional paths under the original native layouts. Runtime handler,
/// observer and object-method alternatives remain outside this diagnostic
/// graph and cannot be withdrawn by any query on this report.
pub(crate) struct DeclarationFlowReport {
    source: Arc<super::SourceOriginId>,
    representation_occurrences: Vec<representation::SourceConversionObservation>,
    reads: BTreeMap<String, tcl_lexer::Span>,
    reached: BTreeSet<u32>,
    represented: BTreeSet<u32>,
    #[cfg(test)]
    conditional_handler_sites: BTreeSet<u32>,
    #[cfg(test)]
    conditional_loop_sites: BTreeSet<u32>,
    #[cfg(test)]
    conditional_list_append_sites: BTreeSet<u32>,
    #[cfg(test)]
    conditional_literal_index_sites: BTreeSet<u32>,
    #[cfg(test)]
    conditional_list_events: Vec<representation::ListEventState>,
    #[cfg(test)]
    conditional_substitution_entries: BTreeSet<u32>,
    #[cfg(test)]
    conditional_body_contexts: Vec<Arc<ConditionalRepresentationBodyContext>>,
    #[cfg(test)]
    conditional_loop_exits: Vec<(u32, usize, usize, usize, usize)>,
    unavailable_before: BTreeSet<u32>,
    unavailable: bool,
    unknown_reads: bool,
    all_reads: BTreeSet<String>,
    authored_reads: BTreeMap<String, tcl_lexer::Span>,
    warning_reads: BTreeMap<String, tcl_lexer::Span>,
    conditional_overwrites: Vec<DeclaredOverwrite>,
    declared_defined_reads: BTreeMap<String, Vec<tcl_lexer::Span>>,
    authored_uses: BTreeMap<String, Vec<tcl_lexer::Span>>,
    named_uses: BTreeSet<String>,
    quoted_uses: BTreeSet<String>,
    quoted_use_unavailable: bool,
    declared_entries: BTreeSet<u32>,
    conditional_handler_literals: BTreeMap<u32, BTreeMap<String, String>>,
    conditional_argument_incoming: BTreeMap<u32, BTreeSet<String>>,
    conditional_caller_alias_prefixes: BTreeSet<u32>,
    aliases: BTreeSet<String>,
    stores: BTreeSet<(u32, String)>,
    frame: DeclarationFlowFrame,
}

/// Missing-read occurrences from an original Logical declaration's source.
/// This purpose can add conditional warnings only. It exposes no report,
/// definedness, alias suppression, store, CFG, entered-frame or execution facts.
pub(crate) struct OriginalDeclaredProcedureReadOccurrences {
    report: Arc<DeclarationFlowReport>,
    uncertain_statics: BTreeSet<String>,
}

impl OriginalDeclaredProcedureReadOccurrences {
    pub(crate) fn occurrences(&self) -> impl Iterator<Item = (&String, &tcl_lexer::Span)> {
        self.report.warning_reads.iter().filter(|(name, _)| {
            !self.report.aliases.contains(*name) && !self.uncertain_statics.contains(*name)
        })
    }
}

struct DeclaredOverwrite {
    first: CommandAllocationSite,
    next: CommandAllocationSite,
    name: String,
    target: tcl_lexer::Span,
}

impl SourceInvocationBinding {
    /// Original-source warnings are independent of current lowered CFG custody.
    /// A retained Logical vector owns its complete input/configuration/registry;
    /// the declared frame owns the source image, allocation, formals and namespace.
    /// Neither this query nor its result can suppress an existing diagnostic.
    pub(crate) fn original_declared_procedure_read_occurrences(
        &self,
        tokens: &CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        procedure: &crate::ir::Procedure,
        image: &tcl_lexer::SourceImage,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalDeclaredProcedureReadOccurrences> {
        input.has_logical_source_name_context().then_some(())?;
        self.original_invocation_metadata_for_input(tokens, input, registry)?;
        let report = self.declaration_flow_report(registry)?;
        let DeclarationFlowFrame::Existing(
            super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredProcedure(entry),
        ) = &report.frame
        else {
            return None;
        };
        entry
            .matches_original_procedure_source(procedure, image)
            .then_some(())?;
        let inventory = self.declaration_flow_inventory.as_ref()?;
        let uncertain_statics =
            entry.static_source_uncertainty(&inventory.layouts, input, registry)?;
        Some(OriginalDeclaredProcedureReadOccurrences {
            report,
            uncertain_statics,
        })
    }

    pub(crate) fn declaration_flow_report(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Arc<DeclarationFlowReport>> {
        self.declaration_flow_report_for(registry, DeclarationFlowPurpose::Reads, None)
    }

    fn declaration_flow_report_for(
        &self,
        registry: &tcl_registry::CommandRegistry,
        purpose: DeclarationFlowPurpose,
        representation_realm: Option<&crate::realm::CommandBindingRealm>,
    ) -> Option<Arc<DeclarationFlowReport>> {
        let observations = super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let first = observations.clone().next()?;
        if observations.clone().any(|item| item.entry != first.entry) {
            return None;
        }
        let inventory = self.declaration_flow_inventory.as_ref()?;
        let dialect = first.snapshot.state.source_variables.invocation_dialect?;
        if observations
            .clone()
            .any(|item| item.snapshot.state.source_variables.invocation_dialect != Some(dialect))
        {
            return None;
        }
        let config = first.config;
        if observations.clone().any(|item| item.config != config) {
            return None;
        }
        let key = DeclarationFlowKey {
            entry: Arc::new(DeclarationFlowFrame::Existing(first.entry.as_ref().clone())),
            config,
            dialect,
            registry: registry.snapshot().semantic_key(),
            purpose,
            retained_module_realm: representation_realm.is_some(),
        };
        if let Some(report) = inventory.reports.get(&key) {
            return Some(report);
        }
        // Nested source queries must never run under the cache lock.
        let frame = DeclarationFlowFrame::Existing(first.entry.as_ref().clone());
        let mut builder = Builder {
            inventory,
            registry,
            config,
            dialect,
            entry: &frame,
            purpose,
            representation_realm,
            representation_loops: Vec::new(),
            current_representation_body: None,
            nodes: vec![Node::default()],
            jobs: Vec::new(),
            conditional_overwrites: Vec::new(),
            read_sites: BTreeSet::new(),
        };
        let entry = builder.placeholder();
        let exits = if purpose == DeclarationFlowPurpose::Representation {
            Exits {
                normal: 0,
                error: builder.placeholder(),
                returned: builder.placeholder(),
                broken: builder.placeholder(),
                continued: builder.placeholder(),
                other: builder.placeholder(),
                cleanup_entry: None,
            }
        } else {
            Exits::all(0)
        };
        builder.jobs.push(Job {
            source: first.entry.source().clone(),
            representation_body: None,
            purpose: match first.entry.as_ref() {
                super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { .. } => {
                    tcl_lexer::SourceSegmentationPurpose::WholeInput
                }
                _ => tcl_lexer::SourceSegmentationPurpose::NestedScript,
            },
            entry,
            exits,
        });
        while let Some(job) = builder.jobs.pop() {
            builder.script(&job);
        }
        Some(inventory.reports.retain(key, builder.evaluate(entry)))
    }

    /// Representation observations are conditional source evidence only.
    /// They do not publish runtime command or variable-cell authority.
    fn declaration_representation_flow_report(
        &self,
        tokens: &CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
        representation_realm: Option<&crate::realm::CommandBindingRealm>,
    ) -> Option<Arc<DeclarationFlowReport>> {
        if tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || tokens.source_binding.as_ref() != Some(self)
        {
            return None;
        }
        self.original_invocation_metadata_for_input(tokens, input, registry)?;
        let seed = self
            .declaration_flow_inventory
            .as_ref()?
            .source_seed
            .as_ref()?;
        if !Arc::ptr_eq(
            self.original_analysis_owner.as_ref()?,
            seed.analysis_owner.as_ref()?,
        ) || seed.baseline.metadata_context.source_analysis_input() != Some(input)
            || seed.config != input.lexer_config()
        {
            return None;
        }
        if let Some(realm) = representation_realm {
            let bindings = realm.source_bindings_ref();
            if !realm.matches_resolved_analysis_input(input)
                || !realm.matches_original_source_image(seed.origin.source_image(), seed.config)
                || !seed.matches_registry(registry)
                || !bindings
                    .source_origin()
                    .is_some_and(|origin| Arc::ptr_eq(origin, &seed.origin))
                || !bindings.original_analysis_owner().is_some_and(|owner| {
                    seed.analysis_owner
                        .as_ref()
                        .is_some_and(|original| Arc::ptr_eq(owner, original))
                })
                || bindings.current_declaration_flow_inventory().as_ref()
                    != self.declaration_flow_inventory.as_deref()?
            {
                return None;
            }
        }
        self.declaration_flow_report_for(
            registry,
            DeclarationFlowPurpose::Representation,
            representation_realm,
        )
    }
}

impl super::SourceCommandBindings {
    pub(crate) fn original_procedure_read_flow(
        &self,
        procedure: &crate::ir::Procedure,
        image: &tcl_lexer::SourceImage,
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Arc<DeclarationFlowReport>> {
        if !self.matches_original_source_image(image, input.lexer_config()) {
            return None;
        }
        let binding = self.invocation_at_source("", procedure.span.start());
        let (_, mut tokens) = binding.original_recorded_command()?;
        tokens.source_binding = Some(binding);
        let read = Arc::new(self.original_procedure_read_flow_entry(&tokens, registry, input)?);
        if !read.owns_original_procedure(procedure, image) {
            return None;
        }
        let config = input.lexer_config();
        let dialect = super::source_analysis_entry::source_input_dialect(input);
        let inventory = self.current_declaration_flow_inventory();
        let frame = DeclarationFlowFrame::Read(Arc::clone(&read));
        let key = DeclarationFlowKey {
            entry: Arc::new(frame.clone()),
            config,
            dialect,
            registry: registry.snapshot().semantic_key(),
            purpose: DeclarationFlowPurpose::Reads,
            retained_module_realm: false,
        };
        if let Some(report) = inventory.reports.get(&key) {
            return Some(report);
        }
        let body = read.read.original_body_span()?;
        let source = tcl_lexer::SourceImage::from_bytes(
            image.bytes().get(body.as_range())?,
            image.channel(),
        );
        let mut builder = Builder {
            inventory: &inventory,
            registry,
            config,
            dialect,
            entry: &frame,
            purpose: DeclarationFlowPurpose::Reads,
            representation_realm: None,
            representation_loops: Vec::new(),
            current_representation_body: None,
            nodes: vec![Node::default()],
            jobs: Vec::new(),
            conditional_overwrites: Vec::new(),
            read_sites: BTreeSet::new(),
        };
        let entry = builder.placeholder();
        builder.original_script(
            &source,
            body.start(),
            &read.header.declaration.source,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
            entry,
            Exits::all(0),
        );
        while let Some(job) = builder.jobs.pop() {
            builder.script(&job);
        }
        Some(inventory.reports.retain(key, builder.evaluate(entry)))
    }

    /// Original declaration graph, independent of executable CFG carriers.
    /// Every returned report validates the declaration allocation, original
    /// image, body and native formals; it supplies only conditional advice.
    pub(crate) fn original_procedure_declaration_flow(
        &self,
        procedure: &crate::ir::Procedure,
        image: &tcl_lexer::SourceImage,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Arc<DeclarationFlowReport>> {
        self.declaration_layouts.iter().find_map(|(site, observations)| {
            if !matches!(site.source.kind(), super::SourceOriginKind::Authored(source) if source == image)
                || !original_procedure_entry_matches(
                    &super::declaration_layout::original_declaration_layouts(observations)?.next()?.entry,
                    procedure,
                    image,
                )
            {
                return None;
            }
            let binding = super::SourceInvocationBinding {
                declaration_layout_observations: Some(Arc::from(observations.as_slice())),
                declaration_flow_inventory: Some(self.current_declaration_flow_inventory()),
                ..Default::default()
            };
            let report = binding.declaration_flow_report(registry)?;
            report.owns_original_procedure(procedure, image).then_some(report)
        })
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
struct State {
    defined: BTreeSet<String>,
    uncertain_writes: BTreeSet<String>,
    aliases: BTreeSet<String>,
    caller_frame_aliases: BTreeSet<String>,
    unknown_writers: bool,
    literals: BTreeMap<String, String>,
    incoming: BTreeSet<String>,
}

#[derive(Default)]
struct Node {
    site: Option<u32>,
    store_site: Option<u32>,
    handler_site: Option<u32>,
    reads: Vec<(String, tcl_lexer::Span)>,
    authored_reads: Vec<(String, tcl_lexer::Span)>,
    named_reads: Vec<String>,
    name_queries: Vec<String>,
    quoted_mentions: BTreeSet<String>,
    availability: NodeAvailability,
    writes: Vec<String>,
    uncertain_writes: Vec<String>,
    possible_output_writes: Vec<String>,
    removes: Vec<String>,
    aliases: Vec<String>,
    caller_frame_aliases: Vec<String>,
    unknown: bool,
    declared_read_unavailable: bool,
    successors: Vec<usize>,
    declared_successors: Option<Vec<usize>>,
    condition: Option<DeclaredCondition>,
    literal_stores: Vec<(String, String)>,
    increments: Vec<(String, String)>,
    accumulator_writes: Vec<String>,
    unknown_list_first: Option<usize>,
    representation_events: Vec<representation::Event>,
}
#[derive(Default)]
struct NodeAvailability {
    schema_origin: crate::registry_invocation::DeclarationSchemaOrigin,
    quoted_use_unavailable: bool,
    unavailable_declared_prefix: bool,
    unknown_reads: bool,
}
#[derive(Clone, Copy)]
struct ConditionTargets {
    yes: usize,
    no: usize,
    first_iteration: bool,
}
struct DeclaredCondition {
    expression: tcl_syntax::expr::ast::ExprNode,
    yes: usize,
    no: usize,
    first_iteration: bool,
}
#[derive(Clone, Copy)]
struct Exits {
    normal: usize,
    error: usize,
    returned: usize,
    broken: usize,
    continued: usize,
    other: usize,
    cleanup_entry: Option<usize>,
}
impl Exits {
    const fn all(id: usize) -> Self {
        Self {
            normal: id,
            error: id,
            returned: id,
            broken: id,
            continued: id,
            other: id,
            cleanup_entry: None,
        }
    }
    fn route(self, route: Route) -> Vec<usize> {
        match route {
            Route::Tcl(CompletionCode::Ok) => vec![self.normal],
            Route::Tcl(CompletionCode::Error) => vec![self.error],
            Route::Tcl(CompletionCode::Break) => vec![self.broken],
            Route::Tcl(CompletionCode::Continue) => vec![self.continued],
            Route::Return(_) | Route::Tcl(CompletionCode::Return) | Route::Tailcall { .. } => {
                vec![self.returned]
            }
            Route::ReturnOrError | Route::TailcallOrError { .. } => vec![self.returned, self.error],
            Route::ExitOrError | Route::CatchableExitOrError => vec![self.other, self.error],
            Route::UnknownAbrupt => vec![
                self.error,
                self.returned,
                self.broken,
                self.continued,
                self.other,
            ],
            Route::TclAlternatives(codes) => codes
                .iter()
                .flat_map(|code| self.route(Route::Tcl(*code)))
                .collect(),
            Route::Unknown => vec![
                self.normal,
                self.error,
                self.returned,
                self.broken,
                self.continued,
                self.other,
            ],
            route if route.normal_possible() => vec![self.normal, self.other],
            _ => vec![self.other],
        }
    }
}
// Conditional source-only body premises; this is never an actual child
// snapshot, execution admission, or a variable/command lookup proof.
#[derive(Clone)]
struct ConditionalRepresentationBodyContext {
    owner: Arc<super::OriginalSourceAnalysisOwner>,
    input: crate::analyser::ResolvedAnalysisInput,
    site: CommandAllocationSite,
    words: Arc<crate::registry_invocation::source_structure::OriginalRegistryWords>,
    body: crate::registry_invocation::OriginalSourceScriptBody,
    namespace: crate::signature_scan::scope::SignatureNamespaceScope,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    binder: tcl_registry::iteration_entry::ConditionalSourceSingleLoopBinding,
    count: tcl_registry::iteration_entry::ConditionalSourceFiniteIteration,
    substitution: Option<ConditionalRepresentationSubstitution>,
}

#[derive(Clone)]
struct ConditionalRepresentationSubstitution {
    parent: Arc<ConditionalRepresentationBodyContext>,
    parent_site: CommandAllocationSite,
    parent_words: Arc<crate::registry_invocation::source_structure::OriginalRegistryWords>,
    operand: usize,
    original: crate::registry_invocation::source_structure::OriginalSingleSourceCommandSubstitution,
}

impl PartialEq for ConditionalRepresentationSubstitution {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.parent, &other.parent)
            && Arc::ptr_eq(&self.parent_site.source, &other.parent_site.source)
            && self.parent_site.offset == other.parent_site.offset
            && self.parent_words == other.parent_words
            && self.operand == other.operand
            && self.original == other.original
    }
}
impl Eq for ConditionalRepresentationSubstitution {}

impl PartialEq for ConditionalRepresentationBodyContext {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && Arc::ptr_eq(&self.site.source, &other.site.source)
            && self.site.offset == other.site.offset
            && self.input == other.input
            && self.words == other.words
            && self.body == other.body
            && self.namespace == other.namespace
            && self.policy == other.policy
            && self.binder == other.binder
            && self.count == other.count
            && self.substitution == other.substitution
    }
}
impl Eq for ConditionalRepresentationBodyContext {}

struct Job {
    source: ExecutedScriptSource,
    representation_body: Option<Arc<ConditionalRepresentationBodyContext>>,
    purpose: tcl_lexer::SourceSegmentationPurpose,
    entry: usize,
    exits: Exits,
}
struct DeclarationSelectionScope<'a> {
    snapshot: &'a Arc<super::SourceLookupSnapshot>,
    namespace: &'a super::SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
}

struct ConditionalRepresentationNaming {
    namespace: crate::signature_scan::scope::SignatureNamespaceScope,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    body_context: Option<Arc<ConditionalRepresentationBodyContext>>,
}

type ConditionalRepresentationTarget = (
    tcl_registry::value_transfer::TargetId,
    crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
);

struct ConditionalRepresentationPlan {
    source: Arc<crate::registry_invocation::source_structure::OriginalRegistryWords>,
    body_context: Option<Arc<ConditionalRepresentationBodyContext>>,
    captures: VecDeque<representation::CaptureSource>,
    relation: Option<tcl_registry::value_transfer::ConditionalSourceRepresentation>,
    prior_target: Option<representation::PriorTargetUpdate>,
    target: Option<ConditionalRepresentationTarget>,
    coercion: Option<tcl_registry::representation::ConditionalSourceListIndexCoercion>,
    list_append: Option<tcl_registry::value_transfer::ConditionalSourceListAppend>,
    list_index: Option<tcl_registry::value_transfer::ConditionalSourceLiteralListIndex>,
}

struct ConditionalRepresentationLoopPlan {
    source: Arc<crate::registry_invocation::source_structure::OriginalRegistryWords>,
    body_context: Option<Arc<ConditionalRepresentationBodyContext>>,
    count: tcl_registry::iteration_entry::ConditionalSourceFiniteIteration,
    binder: crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    span: tcl_lexer::Span,
}

struct ConditionalRepresentationBodyInputs<'a> {
    site: &'a CommandAllocationSite,
    words: &'a Arc<crate::registry_invocation::source_structure::OriginalRegistryWords>,
    body_argument: usize,
    context: &'a tcl_registry::model::ContextRegistry,
    namespace: crate::signature_scan::scope::SignatureNamespaceScope,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    binder: tcl_registry::iteration_entry::ConditionalSourceSingleLoopBinding,
    count: tcl_registry::iteration_entry::ConditionalSourceFiniteIteration,
}

#[derive(Clone, Copy)]
struct ConditionalRepresentationLoopNodes {
    first_gate: usize,
    bindings: usize,
    repeated_entry: usize,
    header: usize,
}

/// Only a conditional source read may use a declared-body frame without a
/// per-command ordinary layout. This never creates a lookup snapshot or frame.
enum DeclarationReadPointScope<'a> {
    Observed(Vec<DeclarationSelectionScope<'a>>),
    ConditionalDeclared(&'a super::declaration_preview::DeclaredProcedureBody),
    ConditionalRoot(&'a super::declaration_layout::OriginalDiagnosticFrameEntry),
    ConditionalRootRelated {
        root: &'a super::declaration_layout::OriginalDiagnosticFrameEntry,
        bodies: Vec<tcl_lexer::Span>,
    },
}

struct Builder<'a> {
    inventory: &'a DeclarationFlowInventory,
    registry: &'a tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: tcl_registry::InvocationDialect,
    entry: &'a DeclarationFlowFrame,
    purpose: DeclarationFlowPurpose,
    representation_realm: Option<&'a crate::realm::CommandBindingRealm>,
    representation_loops: Vec<representation::LoopRegion>,
    current_representation_body: Option<Arc<ConditionalRepresentationBodyContext>>,
    nodes: Vec<Node>,
    jobs: Vec<Job>,
    conditional_overwrites: Vec<(usize, DeclaredOverwrite)>,
    read_sites: BTreeSet<CommandAllocationSite>,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod declared_procedure_source_read_occurrence_tests;

#[cfg(test)]
mod conditional_declared_read_point_tests;

#[cfg(test)]
mod logical_boolean_literal_condition_tests;
