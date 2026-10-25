// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original declaration completion/read graph for conditional diagnostics.
//!
//! The graph owns lexical command layouts, not executed statements or SSA.
//! Missing observations preserve unknown writers. Body sources and command
//! sites retain the original image, logical grammar and declaration frame.

use super::declaration_layout::DeclarationLayouts;
use super::{
    CommandAllocationSite, ExecutedScriptMapping, ExecutedScriptSource, SourceInvocationBinding,
};
use crate::ir::{CommandTokens, WordExpr, WordPart};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};
use tcl_registry::body_execution::BodyOperand;
use tcl_registry::completion::CompletionCode;
use tcl_registry::completion_route::InvocationCompletionRoute as Route;
use tcl_registry::script_body_flow::ScriptBodyFlow;

#[derive(Debug)]
pub(super) struct DeclarationFlowInventory {
    reports: DeclarationFlowReports,
    source_seed: Option<super::source_transition_advice::OriginalSourceTransitionSeed>,
    source_catalogue: OnceLock<Option<Arc<super::source_read_scope::SourceReadCatalogue>>>,
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
    ) -> Option<&super::source_read_scope::SourceReadCatalogue> {
        let seed = self.source_seed.as_ref()?;
        // Foreign requests cannot poison the independently retained cache.
        seed.matches_registry(registry).then_some(())?;
        self.source_catalogue
            .get_or_init(|| self.source_catalogue_memo.get_or_prepare(seed, registry))
            .as_deref()
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
            && self.layouts == other.layouts
            && self.accesses == other.accesses
            && self.origin_accesses == other.origin_accesses
            && self.points == other.points
            && self.origin_points == other.origin_points
    }
}
impl Eq for DeclarationFlowInventory {}

/// Derived inventory retained only by this mutable builder. Clones start empty.
#[derive(Default)]
pub(super) struct SourceDeclarationFlowCache(
    Mutex<Option<Arc<DeclarationFlowInventory>>>,
    Arc<super::source_read_scope::SourceReadCatalogueMemo>,
);
impl Clone for SourceDeclarationFlowCache {
    fn clone(&self) -> Self {
        Self(Mutex::new(None), Arc::clone(&self.1))
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
}
#[derive(Default)]
struct DeclarationFlowReports(Mutex<Vec<(DeclarationFlowKey, Arc<DeclarationFlowReport>)>>);
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
        let next = Arc::new(next);
        *current = Some(Arc::clone(&next));
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
    reads: BTreeMap<String, tcl_lexer::Span>,
    reached: BTreeSet<u32>,
    represented: BTreeSet<u32>,
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

fn original_procedure_entry_matches(
    frame: &super::declaration_layout::OriginalDiagnosticFrameEntry,
    procedure: &crate::ir::Procedure,
    image: &tcl_lexer::SourceImage,
) -> bool {
    let super::declaration_layout::OriginalDiagnosticFrameEntry::Body(entry) = frame else {
        return false;
    };
    entry.allocation().site.offset == procedure.span.start()
        && matches!(entry.source().origin.kind(), super::SourceOriginKind::Authored(source) if source == image)
        && entry.source().base() == procedure.body_offset
        && entry.source().try_text().ok() == procedure.body_source.as_deref()
        && entry.matches_parameters(
            &procedure
                .params
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )
}

impl DeclarationFlowReport {
    /// Validate an authored declaration independently from an entered script.
    /// This retains diagnostic frame ownership and grants no body execution.
    pub(crate) fn owns_original_procedure(
        &self,
        procedure: &crate::ir::Procedure,
        image: &tcl_lexer::SourceImage,
    ) -> bool {
        match &self.frame {
            DeclarationFlowFrame::Existing(frame) => {
                original_procedure_entry_matches(frame, procedure, image)
            }
            DeclarationFlowFrame::Read(entry) => entry.owns_original_procedure(procedure, image),
        }
    }

    pub(crate) fn owns_invocation(&self, site: &CommandAllocationSite) -> bool {
        self.source == site.source && self.represented.contains(&site.offset)
    }

    #[cfg(test)]
    pub(crate) fn potential_missing_reads(&self) -> &BTreeMap<String, tcl_lexer::Span> {
        &self.reads
    }

    /// Original local references under the declared frame interpretation.
    /// Unlike physical read advice, these occurrences carry no current cell,
    /// successful read, SSA version or closed-world absence. Unknown writers
    /// remain possible; a diagnostic must describe that condition explicitly.
    #[cfg(test)]
    pub(crate) fn potential_declared_reads(&self) -> &BTreeMap<String, tcl_lexer::Span> {
        &self.authored_reads
    }

    /// An original missing-read occurrence whose authored dispatch prefix is
    /// described. Unknown handler dispatch can initialise arbitrary locals, so
    /// its successors remain in the occurrence inventory but cannot supply an
    /// absence warning. Runtime callback alternatives remain independent.
    pub(crate) fn allows_declared_absence_warning(
        &self,
        name: &str,
        span: tcl_lexer::Span,
    ) -> bool {
        self.warning_reads.get(name) == Some(&span)
    }

    /// Missing-read candidates on an independently available original prefix.
    /// Unknown later iterations and runtime alternatives remain in the other
    /// graphs; this grants only the explicitly conditional local-frame warning.
    pub(crate) fn declared_absence_warning_occurrences(
        &self,
    ) -> &BTreeMap<String, tcl_lexer::Span> {
        &self.warning_reads
    }

    /// Adjacent literal stores under an available declared local-frame prefix.
    /// These occurrences grant no actual reachability, cell or editable store.
    pub(crate) fn conditional_overwrite_occurrences(
        &self,
    ) -> impl Iterator<
        Item = (
            &CommandAllocationSite,
            &CommandAllocationSite,
            &str,
            tcl_lexer::Span,
        ),
    > {
        self.conditional_overwrites
            .iter()
            .map(|store| (&store.first, &store.next, store.name.as_str(), store.target))
    }

    /// A reference defined on the declared diagnostic paths reaching that
    /// exact original occurrence. First loop entry is distinct from backedges;
    /// this condition never removes a runtime zero-trip or callback alternative.
    pub(crate) fn declared_read_is_defined(&self, name: &str, span: tcl_lexer::Span) -> bool {
        self.declared_defined_reads
            .get(name)
            .is_some_and(|spans| spans.contains(&span))
    }

    /// Original uses in reachable declared syntax. This can suppress a warning,
    /// but cannot keep a physical version live or grant a runtime read.
    pub(crate) fn has_authored_use(&self, name: &str) -> bool {
        self.authored_uses.contains_key(name)
    }

    pub(crate) fn authored_use_spans(&self, name: &str) -> &[tcl_lexer::Span] {
        self.authored_uses.get(name).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn has_named_use(&self, name: &str) -> bool {
        self.named_uses.contains(name)
    }

    /// Literal data mentions only suppress unused-store diagnostics. They are
    /// not reads, variable versions, liveness facts or executable source.
    pub(crate) fn has_quoted_use(&self, name: &str) -> bool {
        self.quoted_use_unavailable || self.quoted_uses.contains(name)
    }

    pub(crate) fn has_declared_alias(&self, name: &str) -> bool {
        self.aliases.contains(name)
    }

    /// Original declaration-local value immediately before the selected
    /// handler, after its original operands. Unknown writers and aliases
    /// withdraw this advice; no physical contents or SSA value is supplied.
    pub(crate) fn conditional_handler_literal(
        &self,
        site: &CommandAllocationSite,
        name: &str,
    ) -> Option<&str> {
        (site.source == self.source)
            .then(|| {
                self.conditional_handler_literals
                    .get(&site.offset)?
                    .get(name)
                    .map(String::as_str)
            })
            .flatten()
    }

    /// An original formal still carries its incoming value at this declared
    /// argument entry, before this command's operands and dispatch. The caller
    /// must independently close intervening operand effects. Unknown earlier
    /// writers and joins withdraw this source-only provenance; this command's
    /// unavailable handler cannot erase its own available argument prefix.
    /// This grants neither an actual read nor a caller value.
    pub(crate) fn conditional_argument_keeps_incoming(
        &self,
        site: &CommandAllocationSite,
        name: &str,
    ) -> bool {
        site.source == self.source
            && self
                .conditional_argument_incoming
                .get(&site.offset)
                .is_some_and(|names| names.contains(name))
    }

    /// Every original alias in this available declaration prefix selects the
    /// immediate caller's frame. This conditional name-layout fact does not
    /// establish a successful alias, actual caller address or receiver entry.
    pub(crate) fn conditional_alias_prefix_targets_caller(
        &self,
        site: &CommandAllocationSite,
    ) -> bool {
        // naming.tcloo.original-declared-receiver-caller-traits

        self.source == site.source
            && self
                .conditional_caller_alias_prefixes
                .contains(&site.offset)
    }

    /// Original argument entry under the accepted declared-frame layouts.
    /// Unknown runtime dispatch and writers remain possible. This purpose
    /// supplies candidate diagnostics only, never successful entry or SSA facts.
    pub(crate) fn declared_argument_entry(&self, site: &CommandAllocationSite) -> bool {
        self.source == site.source && self.declared_entries.contains(&site.offset)
    }

    /// Conditional argument-entry reachability at this original invocation.
    /// An unavailable predecessor returns `None`. This command's subsequent
    /// dispatch does not withdraw its own entry, but does withdraw successors.
    /// Absence never means proved dead runtime code.
    pub(crate) fn invocation_may_be_reached(&self, site: &CommandAllocationSite) -> Option<bool> {
        if self.source != site.source || !self.represented.contains(&site.offset) {
            return None;
        }
        if self.reached.contains(&site.offset) {
            return (!self.unavailable_before.contains(&site.offset)).then_some(true);
        }
        (!self.unavailable).then_some(false)
    }

    /// Conditional absence of an original local read in the complete native
    /// declaration layout. Runtime callbacks and observation remain open.
    pub(crate) fn local_store_may_be_unread(
        &self,
        site: &CommandAllocationSite,
        name: &str,
    ) -> bool {
        !self.unavailable
            && !self.unknown_reads
            && self.invocation_may_be_reached(site) == Some(true)
            && self.stores.contains(&(site.offset, name.to_owned()))
            && !self.all_reads.contains(name)
            && !self.has_authored_use(name)
            && !self.aliases.contains(name)
    }
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
            nodes: vec![Node::default()],
            jobs: Vec::new(),
            conditional_overwrites: Vec::new(),
            read_sites: BTreeSet::new(),
        };
        let entry = builder.placeholder();
        builder.jobs.push(Job {
            source: first.entry.source().clone(),
            purpose: match first.entry.as_ref() {
                super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { .. } => {
                    tcl_lexer::SourceSegmentationPurpose::WholeInput
                }
                _ => tcl_lexer::SourceSegmentationPurpose::NestedScript,
            },
            entry,
            exits: Exits::all(0),
        });
        while let Some(job) = builder.jobs.pop() {
            builder.script(&job);
        }
        Some(inventory.reports.retain(key, builder.evaluate(entry)))
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
impl State {
    fn join(&mut self, other: &Self) -> bool {
        let old = self.clone();
        self.defined.retain(|name| other.defined.contains(name));
        self.uncertain_writes
            .extend(other.uncertain_writes.iter().cloned());
        self.incoming.retain(|name| other.incoming.contains(name));
        self.aliases.extend(other.aliases.iter().cloned());
        self.caller_frame_aliases
            .retain(|name| other.caller_frame_aliases.contains(name));
        self.unknown_writers |= other.unknown_writers;
        self.literals
            .retain(|name, value| other.literals.get(name) == Some(value));
        *self != old
    }
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
struct Job {
    source: ExecutedScriptSource,
    purpose: tcl_lexer::SourceSegmentationPurpose,
    entry: usize,
    exits: Exits,
}
struct DeclarationSelectionScope<'a> {
    snapshot: &'a Arc<super::SourceLookupSnapshot>,
    namespace: &'a super::SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
}

/// Only a conditional source read may use a declared-body frame without a
/// per-command ordinary layout. This never creates a lookup snapshot or frame.
enum DeclarationReadPointScope<'a> {
    Observed(Vec<DeclarationSelectionScope<'a>>),
    ConditionalDeclared(&'a super::declaration_preview::DeclaredProcedureBody),
}

impl DeclarationReadPointScope<'_> {
    fn matches_frame(&self, frame: &crate::var_resolve::VariableExecutionFrame) -> bool {
        match self {
            Self::Observed(scopes) => scopes
                .iter()
                .any(|scope| scope.snapshot.state.variable_frame == *frame),
            Self::ConditionalDeclared(body) => body.frame == *frame,
        }
    }

    fn accepts_source_scope(
        &self,
        scope: &super::source_read_scope::SourceReadScope,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        match self {
            Self::Observed(_) => true,
            Self::ConditionalDeclared(body) => body.matches_logical_read_scope(scope, config),
        }
    }
}

struct Builder<'a> {
    inventory: &'a DeclarationFlowInventory,
    registry: &'a tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: tcl_registry::InvocationDialect,
    entry: &'a DeclarationFlowFrame,
    nodes: Vec<Node>,
    jobs: Vec<Job>,
    conditional_overwrites: Vec<(usize, DeclaredOverwrite)>,
    read_sites: BTreeSet<CommandAllocationSite>,
}
impl Builder<'_> {
    fn placeholder(&mut self) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node::default());
        id
    }
    fn unknown(&mut self, id: usize, next: usize) {
        self.nodes[id].unknown = true;
        self.nodes[id].availability.unavailable_declared_prefix = true;
        self.nodes[id].successors = vec![next];
    }
    fn script(&mut self, job: &Job) {
        self.original_script(
            &job.source.text,
            job.source.base(),
            &job.source.origin,
            job.purpose,
            job.entry,
            job.exits,
        );
    }
    fn original_script(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        origin: &Arc<super::SourceOriginId>,
        purpose: tcl_lexer::SourceSegmentationPurpose,
        entry: usize,
        exits: Exits,
    ) {
        let Some(segments) = crate::segmenter::segment_commands_image_with_offset_and_config(
            image,
            base,
            purpose.config(self.config),
        ) else {
            self.unknown(entry, exits.normal);
            return;
        };
        self.retain_read_sites(image, base, origin, &segments);
        let mut next = exits.normal;
        let mut next_original = None;
        for segment in segments.iter().rev() {
            if segment.is_partial {
                let unavailable = self.placeholder();
                self.unknown(unavailable, exits.other);
                next = unavailable;
                next_original = None;
                continue;
            }
            let tokens = CommandTokens::from_segmented(
                &image.source_map().with_base(base, 0, 0),
                self.config,
                segment,
            );
            let id = self.placeholder();
            let site = CommandAllocationSite {
                source: Arc::clone(origin),
                offset: segment.span.start(),
            };
            let exits = Exits {
                normal: next,
                ..exits
            };
            self.command(id, &tokens, &site, exits);
            if let Some((next_tokens, next_site)) = &next_original {
                self.retain_conditional_overwrite(id, &tokens, &site, next_tokens, next_site);
            }
            next_original = Some((tokens, site));
            next = id;
        }
        self.nodes[entry].successors = vec![next];
    }

    fn retain_read_sites(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        origin: &Arc<super::SourceOriginId>,
        segments: &[crate::segmenter::SegmentedCommand],
    ) {
        if let DeclarationFlowFrame::Read(read) = self.entry {
            for segment in segments {
                if segment.is_partial {
                    break;
                }
                let tokens = super::source_command_tokens_boxed(image, base, self.config, segment);
                let site = CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset: segment.span.start(),
                };
                if !read.owns_source(origin, site.offset) {
                    break;
                }
                self.read_sites.insert(site.clone());
                if self.source_read_selection(&tokens, &site).is_none()
                    && !super::declaration_preview::retains_successor_layout(
                        &site,
                        &tokens,
                        &read.snapshot,
                        &read.header.namespace,
                        self.config,
                        self.registry,
                    )
                {
                    break;
                }
            }
        }
    }

    fn retain_conditional_overwrite(
        &mut self,
        first_id: usize,
        first: &CommandTokens,
        first_site: &CommandAllocationSite,
        next: &CommandTokens,
        next_site: &CommandAllocationSite,
    ) {
        let Some((first_name, target)) = self.literal_local_setter(first, first_site) else {
            return;
        };
        let Some((next_name, _)) = self.literal_local_setter(next, next_site) else {
            return;
        };
        if first_name == next_name && first_site.source == next_site.source {
            self.conditional_overwrites.push((
                first_id,
                DeclaredOverwrite {
                    first: first_site.clone(),
                    next: next_site.clone(),
                    name: first_name,
                    target,
                },
            ));
        }
    }

    fn literal_local_setter(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<(String, tcl_lexer::Span)> {
        let (selected, _) = self.selection(tokens, site)?;
        let [(name, _)] = selected.literal_stores.as_slice() else {
            return None;
        };
        if selected.effects.unknown_writes
            || selected.effects.unknown_reads
            || selected.writes.as_slice() != std::slice::from_ref(name)
            || !selected.reads.is_empty()
            || !selected.aliases.is_empty()
            || !matches!(selected.flow, ScriptBodyFlow::None)
            || tcl_syntax::naming::is_qualified(name.as_bytes())
            || tcl_syntax::naming::split_element_ref(name).is_some()
            || selected.effective.words.iter().skip(1).any(|word| {
                crate::registry_invocation::effective_invocation_word(
                    word,
                    self.dialect.lexer_grammar.escapes,
                    self.dialect.word_values,
                )
                .literal_bytes()
                .is_none()
            })
        {
            return None;
        }
        let target_at = selected.literal_store_operand?;
        let crate::registry_invocation::InvocationWordOrigin::Written(written) =
            selected.effective.origins.get(target_at)?
        else {
            return None;
        };
        let target = tokens.words().get(*written)?.source().span;
        Some((name.clone(), target))
    }
    fn selection_scopes(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<Vec<DeclarationSelectionScope<'_>>> {
        Some(match self.entry {
            DeclarationFlowFrame::Existing(entry) => {
                let observations = super::declaration_layout::original_declaration_layouts(
                    self.inventory.layouts.get(site)?,
                )?;
                let mut scopes = Vec::new();
                for observation in observations {
                    if observation.entry.as_ref() != entry
                        || observation.config != self.config
                        || observation.words.as_ref() != tokens.words()
                    {
                        return None;
                    }
                    scopes.push(DeclarationSelectionScope {
                        snapshot: &observation.snapshot,
                        namespace: &observation.namespace,
                        config: observation.config,
                    });
                }
                scopes
            }
            DeclarationFlowFrame::Read(entry) => {
                if !self.read_sites.contains(site) || !entry.owns_source(&site.source, site.offset)
                {
                    return None;
                }
                let source = entry.header.declaration.source.source_image();
                let text = source.try_text().ok()?;
                let first = tokens.words().first()?.original_written_span(text)?;
                let last = tokens.words().last()?.original_written_span(text)?;
                if first.start() != site.offset {
                    return None;
                }
                let image = tcl_lexer::SourceImage::from_bytes(
                    source
                        .bytes()
                        .get(first.start() as usize..last.end() as usize)?,
                    source.channel(),
                );
                let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
                    &image,
                    first.start(),
                    self.config.nested(),
                )?;
                let [segment] = segments.as_slice() else {
                    return None;
                };
                let original =
                    super::source_command_tokens_boxed(&image, first.start(), self.config, segment);
                if original.words() != tokens.words() {
                    return None;
                }
                vec![DeclarationSelectionScope {
                    snapshot: &entry.snapshot,
                    namespace: &entry.header.namespace,
                    config: self.config,
                }]
            }
        })
    }

    fn source_read_point_scope(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<DeclarationReadPointScope<'_>> {
        if let Some(scopes) = self.selection_scopes(tokens, site) {
            return Some(DeclarationReadPointScope::Observed(scopes));
        }
        // An original row that is present but invalid cannot borrow source
        // absence. Only an independently retained declaration may proceed.
        if self.inventory.layouts.contains_key(site) {
            return None;
        }
        let DeclarationFlowFrame::Existing(
            super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredProcedure(body),
        ) = self.entry
        else {
            return None;
        };
        Some(DeclarationReadPointScope::ConditionalDeclared(body))
    }

    fn source_read_selection(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<crate::registry_invocation::DeclarationInvocationFlow> {
        let body = match self.entry {
            DeclarationFlowFrame::Existing(entry) => {
                if !matches!(
                    entry.frame(),
                    crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                        | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                ) {
                    return None;
                }
                let source = entry.source();
                if source.origin != site.source || !entry.owns_source(&site.source, site.offset) {
                    return None;
                }
                tcl_lexer::Span::new(
                    source.base(),
                    source
                        .base()
                        .checked_add(u32::try_from(source.text.len()).ok()?)?,
                )
            }
            DeclarationFlowFrame::Read(entry) => {
                entry.owns_source(&site.source, site.offset).then_some(())?;
                entry.read.original_body_span()?
            }
        };
        // Original layout corruption is terminal even for a source-only query.
        let scopes = self.source_read_point_scope(tokens, site)?;
        let mut retained = tokens.clone();
        let points = self
            .inventory
            .points
            .iter()
            .chain(
                self.inventory
                    .origin_points
                    .get(&site.source)
                    .into_iter()
                    .flatten(),
            )
            .filter(|point| {
                point.dispatch
                    && point.offset == site.offset
                    && point.state.current_source_origin.as_ref() == Some(&site.source)
                    && scopes.matches_frame(&point.state.variable_frame)
            });
        let mut points = points.peekable();
        if points.peek().is_some() {
            retained.source_binding =
                Some(super::SourceCommandBindings::query_dispatch_points(points));
        }
        let scope = self.inventory.source_read_catalogue(self.registry)?.scope(
            site,
            &retained,
            body,
            self.config,
        )?;
        if !scopes.accepts_source_scope(&scope, self.config) {
            return None;
        }
        if scope.advice().dialect() != self.dialect {
            return None;
        }
        crate::registry_invocation::declaration_source_read_flow(self.registry, &retained, &scope)
    }

    fn selection(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<(
        crate::registry_invocation::DeclarationInvocationFlow,
        Option<super::lifecycle_advice::SourceScopedLifecycleAdvice>,
    )> {
        self.original_layout_selection(tokens, site).or_else(|| {
            self.source_read_selection(tokens, site)
                .map(|selected| (selected, None))
        })
    }

    fn original_layout_selection(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<(
        crate::registry_invocation::DeclarationInvocationFlow,
        Option<super::lifecycle_advice::SourceScopedLifecycleAdvice>,
    )> {
        let mut agreed = None;
        for DeclarationSelectionScope {
            snapshot,
            namespace,
            config,
        } in self.selection_scopes(tokens, site)?
        {
            let advice = super::original_site_operand_layout_advice(
                site, tokens, snapshot, namespace, config,
            )?;
            if advice.dialect() != self.dialect {
                return None;
            }
            let mut retained = tokens.clone();
            let points = self
                .inventory
                .points
                .iter()
                .chain(
                    self.inventory
                        .origin_points
                        .get(&site.source)
                        .into_iter()
                        .flatten(),
                )
                .filter(|point| {
                    point.dispatch
                        && point.offset == site.offset
                        && point.state.current_source_origin.as_ref() == Some(&site.source)
                        && point.state.variable_frame == snapshot.state.variable_frame
                });
            let mut points = points.peekable();
            if points.peek().is_some() {
                retained.source_binding =
                    Some(super::SourceCommandBindings::query_dispatch_points(points));
            }
            let selected = crate::registry_invocation::declaration_invocation_flow(
                self.registry,
                &retained,
                &advice,
            )?;
            let lifecycle = if selected.flow == ScriptBodyFlow::CapturedLifecycle {
                Some(match self.entry {
                    DeclarationFlowFrame::Existing(entry) => advice
                        .scoped_declaration_lifecycle_advice(&retained, self.registry, entry)?,
                    DeclarationFlowFrame::Read(entry) => advice
                        .scoped_procedure_read_lifecycle_advice(&retained, self.registry, entry)?,
                })
            } else {
                None
            };
            let selected = (selected, lifecycle);
            if agreed.as_ref().is_some_and(|old| old != &selected) {
                return None;
            }
            agreed = Some(selected);
        }
        agreed
    }
    fn command(
        &mut self,
        id: usize,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        exits: Exits,
    ) {
        self.nodes[id].site = Some(site.offset);
        let selected = self.selection(tokens, site);
        let dispatch = self.placeholder();
        let mut operands = Vec::new();
        for word in tokens.words() {
            self.word_events(word, site, &mut operands);
        }
        let next = self.events_before(operands, dispatch, exits);
        self.nodes[id].successors = vec![next];
        let Some((selected, lifecycle)) = selected else {
            self.unknown(dispatch, exits.normal);
            return;
        };
        self.nodes[id].availability.schema_origin = selected.effects.schema_origin;
        for &operand in &selected.quoted_operands {
            match self.quoted_mentions(site, &selected.effective, operand) {
                Ok(mentions) => self.nodes[id].quoted_mentions.extend(mentions),
                Err(()) => self.nodes[id].availability.quoted_use_unavailable = true,
            }
        }
        self.nodes[dispatch].handler_site = Some(site.offset);
        self.nodes[dispatch].aliases.clone_from(&selected.aliases);
        self.nodes[dispatch]
            .caller_frame_aliases
            .clone_from(&selected.caller_frame_aliases);
        self.nodes[dispatch].unknown = selected.effects.unknown_writes;
        self.nodes[dispatch].named_reads.clone_from(&selected.reads);
        self.nodes[dispatch]
            .name_queries
            .clone_from(&selected.name_queries);
        self.nodes[dispatch].availability.unknown_reads = selected.effects.unknown_reads;
        let after = self.placeholder();
        self.nodes[after].store_site = Some(site.offset);
        self.nodes[after].writes.clone_from(&selected.writes);
        self.nodes[after]
            .possible_output_writes
            .clone_from(&selected.possible_output_writes);
        self.nodes[after]
            .literal_stores
            .clone_from(&selected.literal_stores);
        self.nodes[after]
            .increments
            .clone_from(&selected.increments);
        self.nodes[after]
            .accumulator_writes
            .clone_from(&selected.accumulator_writes);
        self.nodes[after].removes.clone_from(&selected.removes);
        self.nodes[after].successors = vec![exits.normal];
        let successors = self.command_flow(site, &selected, lifecycle.as_ref(), after, exits);
        self.nodes[dispatch].successors = successors;
    }

    fn command_flow(
        &mut self,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        lifecycle: Option<&super::lifecycle_advice::SourceScopedLifecycleAdvice>,
        after: usize,
        exits: Exits,
    ) -> Vec<usize> {
        let body_exits = Exits {
            normal: after,
            ..exits
        };
        let flow_entry = match &selected.flow {
            ScriptBodyFlow::None | ScriptBodyFlow::Deferred => {
                return self.handler_completion_flow(selected, body_exits);
            }
            ScriptBodyFlow::Sequence(arguments) => {
                self.sequence(site, &selected.effective, arguments, body_exits)
            }
            ScriptBodyFlow::CaseBodies(case) => {
                let mut edges = case
                    .bodies
                    .iter()
                    .map(|operand| self.body(site, &selected.effective, *operand, body_exits))
                    .collect::<Vec<_>>();
                if case.no_match_possible {
                    edges.push(after);
                }
                // An uncertain option/subject layout also has an abrupt path.
                if case.selection_unknown {
                    edges.push(exits.error);
                }
                let branch = self.placeholder();
                self.nodes[branch].successors = edges;
                branch
            }
            ScriptBodyFlow::Alternatives(arguments) => {
                let branch = self.placeholder();
                self.nodes[branch].successors = arguments
                    .iter()
                    .map(|argument| {
                        self.body(
                            site,
                            &selected.effective,
                            BodyOperand {
                                argument: *argument,
                                list_element: None,
                            },
                            body_exits,
                        )
                    })
                    .chain(std::iter::once(after))
                    .collect();
                branch
            }
            ScriptBodyFlow::Conditional(branches) => {
                self.conditional_flow(site, selected, branches, after, body_exits, exits)
            }
            flow @ ScriptBodyFlow::Loop { .. } => {
                self.loop_flow(site, selected, flow, after, exits)
            }
            ScriptBodyFlow::Capture(
                tcl_registry::catch_invocation::CatchInvocationSelection::Valid(capture),
            ) if capture.ignored_codes == 0 => {
                let caught = Exits {
                    normal: after,
                    error: after,
                    returned: after,
                    broken: after,
                    continued: after,
                    other: exits.other,
                    cleanup_entry: exits.cleanup_entry,
                };
                self.body(
                    site,
                    &selected.effective,
                    BodyOperand {
                        argument: capture.script_at,
                        list_element: None,
                    },
                    caught,
                )
            }
            ScriptBodyFlow::Expressions(arguments) => {
                self.expressions(site, &selected.effective, arguments, after, exits)
            }
            ScriptBodyFlow::CapturedLifecycle => {
                let Some(lifecycle) = lifecycle else {
                    let unknown = self.placeholder();
                    self.unknown(unknown, after);
                    return vec![unknown];
                };
                self.lifecycle_flow(site, selected, lifecycle, after, exits)
            }
            ScriptBodyFlow::ConcatenatedExpression { argument_offset }
                if selected.effective.words.len() == argument_offset + 2 =>
            {
                self.expressions(site, &selected.effective, &[*argument_offset], after, exits)
            }
            // Expressions may contain lazily selected script substitutions;
            // unsupported value/lifecycle protocols retain an unknown writer.
            _ => {
                let unknown = self.placeholder();
                self.unknown(unknown, after);
                unknown
            }
        };
        vec![flow_entry]
    }

    fn handler_completion_flow(
        &mut self,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        exits: Exits,
    ) -> Vec<usize> {
        let mut successors = exits.route(selected.completion);
        if let Some(cleanup) = exits.cleanup_entry
            && selected.completion.abrupt_possible()
            && (!selected.writes.is_empty() || !selected.possible_output_writes.is_empty())
        {
            // Successful output roles do not describe partial stores
            // on failure, including a completion caught by cleanup.
            let uncertain = self.placeholder();
            self.nodes[uncertain].uncertain_writes = selected
                .writes
                .iter()
                .chain(&selected.possible_output_writes)
                .cloned()
                .collect();
            self.nodes[uncertain].successors = vec![cleanup];
            for successor in &mut successors {
                if *successor == cleanup {
                    *successor = uncertain;
                }
            }
            successors.sort_unstable();
            successors.dedup();
        }
        successors
    }

    fn lifecycle_flow(
        &mut self,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        lifecycle: &super::lifecycle_advice::SourceScopedLifecycleAdvice,
        after: usize,
        exits: Exits,
    ) -> usize {
        let complete = self.placeholder();
        self.nodes[complete].successors = Exits {
            normal: after,
            ..exits
        }
        .route(selected.completion);
        let repeat = self.placeholder();
        self.nodes[repeat].successors = vec![complete];
        let captured = |normal, cleanup_entry| Exits {
            normal,
            error: normal,
            returned: normal,
            broken: normal,
            continued: normal,
            other: exits.other,
            cleanup_entry,
        };
        let phases = &lifecycle.invocation.selection;
        let phase_body = |builder: &mut Self, operand: BodyOperand, exits: Exits| {
            let Some(argument) = operand
                .argument
                .checked_add(lifecycle.invocation.argument_offset)
            else {
                let unknown = builder.placeholder();
                builder.unknown(unknown, exits.normal);
                return unknown;
            };
            builder.body(
                site,
                &selected.effective,
                BodyOperand {
                    argument,
                    ..operand
                },
                exits,
            )
        };
        let cleanup = phases.cleanup.map_or(repeat, |operand| {
            phase_body(self, operand, captured(repeat, exits.cleanup_entry))
        });
        let body = phases.body.map_or(cleanup, |operand| {
            phase_body(self, operand, captured(cleanup, Some(cleanup)))
        });
        let setup = phases.setup.map_or(body, |operand| {
            phase_body(
                self,
                operand,
                Exits {
                    normal: body,
                    ..captured(cleanup, Some(cleanup))
                },
            )
        });
        if lifecycle
            .invocation
            .contract
            .repetition(lifecycle.loader.version.as_deref())
            == tcl_registry::body_execution::BodyRepetition::MayRepeat
        {
            self.nodes[repeat].successors.push(setup);
        }
        let choice = self.placeholder();
        self.nodes[choice].successors = vec![setup, complete];
        choice
    }

    fn conditional_flow(
        &mut self,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        branches: &[(Option<usize>, usize)],
        after: usize,
        body_exits: Exits,
        exits: Exits,
    ) -> usize {
        let mut otherwise = after;
        for &(condition, argument) in branches.iter().rev() {
            let body = self.body(
                site,
                &selected.effective,
                BodyOperand {
                    argument,
                    list_element: None,
                },
                body_exits,
            );
            otherwise = if let Some(condition) = condition {
                self.condition(
                    site,
                    &selected.effective,
                    condition,
                    ConditionTargets {
                        yes: body,
                        no: otherwise,
                        first_iteration: false,
                    },
                    exits,
                )
            } else {
                body
            };
        }
        otherwise
    }

    fn loop_flow(
        &mut self,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        flow: &ScriptBodyFlow,
        after: usize,
        exits: Exits,
    ) -> usize {
        let ScriptBodyFlow::Loop {
            initial,
            repeated,
            continued,
            conditions,
        } = flow
        else {
            unreachable!("selected loop flow");
        };
        let header = self.placeholder();
        let bindings = self.placeholder();
        self.nodes[bindings]
            .writes
            .clone_from(&selected.loop_bindings);
        let update = self.sequence(
            site,
            &selected.effective,
            continued,
            Exits {
                normal: header,
                ..exits
            },
        );
        let body_exits = Exits {
            normal: header,
            broken: after,
            continued: update,
            ..exits
        };
        let repeated = self.sequence(site, &selected.effective, repeated, body_exits);
        self.nodes[bindings].successors = vec![repeated];
        let mut test = bindings;
        if conditions.is_empty() {
            let branch = self.placeholder();
            self.nodes[branch].successors = vec![after, bindings];
            test = branch;
        } else {
            for &condition in conditions.iter().rev() {
                test = self.condition(
                    site,
                    &selected.effective,
                    condition,
                    ConditionTargets {
                        yes: test,
                        no: after,
                        first_iteration: false,
                    },
                    exits,
                );
            }
        }
        self.nodes[header].successors = vec![test];
        let mut first_test = bindings;
        if conditions.is_empty() {
            let branch = self.placeholder();
            self.nodes[branch].successors = vec![after, bindings];
            self.nodes[branch].unknown_list_first =
                (selected.list_loop && selected.first_list_iteration.is_none()).then_some(bindings);
            self.nodes[branch].declared_successors = selected
                .first_list_iteration
                .map(|nonempty| vec![if nonempty { bindings } else { after }]);
            first_test = branch;
        } else {
            for &condition in conditions.iter().rev() {
                first_test = self.condition(
                    site,
                    &selected.effective,
                    condition,
                    ConditionTargets {
                        yes: first_test,
                        no: after,
                        first_iteration: true,
                    },
                    exits,
                );
            }
        }
        self.sequence(
            site,
            &selected.effective,
            initial,
            Exits {
                normal: first_test,
                ..exits
            },
        )
    }
    fn events_before(&mut self, events: Vec<Event>, next: usize, exits: Exits) -> usize {
        let mut next = next;
        for event in events.into_iter().rev() {
            let id = self.placeholder();
            match event {
                Event::AuthoredRead(name, span) => {
                    self.nodes[id].authored_reads.push((name, span));
                    self.nodes[id].successors = vec![next];
                }
                Event::Read(name, span) => {
                    self.nodes[id].reads.push((name, span));
                    self.nodes[id].successors = vec![next];
                }
                Event::Script(source) => self.jobs.push(Job {
                    source,
                    purpose: tcl_lexer::SourceSegmentationPurpose::NestedScript,
                    entry: id,
                    exits: Exits {
                        normal: next,
                        ..exits
                    },
                }),
                Event::ReadUnavailable => {
                    self.unknown(id, next);
                    self.nodes[id].declared_read_unavailable = true;
                    self.nodes[id].availability.unavailable_declared_prefix = false;
                }
                Event::Unknown => self.unknown(id, next),
            }
            next = id;
        }
        next
    }

    fn condition(
        &mut self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        argument: usize,
        targets: ConditionTargets,
        exits: Exits,
    ) -> usize {
        let ConditionTargets {
            yes,
            no,
            first_iteration,
        } = targets;
        let branch = self.placeholder();
        let (events, value, expression) = self.expression_events(site, effective, argument);
        self.nodes[branch].condition = expression.map(|expression| DeclaredCondition {
            expression,
            yes,
            no,
            first_iteration,
        });
        self.nodes[branch].successors = match value {
            Some(true) => vec![yes],
            Some(false) => vec![no],
            None => vec![yes, no, exits.error],
        };
        self.events_before(events, branch, exits)
    }

    fn expressions(
        &mut self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        arguments: &[usize],
        next: usize,
        exits: Exits,
    ) -> usize {
        let mut next = next;
        for &argument in arguments.iter().rev() {
            let (events, _, _) = self.expression_events(site, effective, argument);
            next = self.events_before(events, next, exits);
        }
        next
    }

    fn expression_events(
        &self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        argument: usize,
    ) -> (
        Vec<Event>,
        Option<bool>,
        Option<tcl_syntax::expr::ast::ExprNode>,
    ) {
        use tcl_syntax::expr::ast::{BinOp, ExprNode};
        use tcl_syntax::expr::parser::{CheckedExprParse, parse_expr_checked_with_context};
        let source = self.body_source(
            site,
            effective,
            BodyOperand {
                argument,
                list_element: None,
            },
        );
        let Some(source) = source else {
            return (vec![Event::Unknown], None, None);
        };
        let dialect = self.entry_dialect();
        let Ok(text) = source.try_text() else {
            return (vec![Event::Unknown], None, None);
        };
        let CheckedExprParse::Parsed(expression) =
            parse_expr_checked_with_context(text, &dialect.expression_parse_context(None))
        else {
            return (vec![Event::Unknown], None, None);
        };
        let value = if let ExprNode::Literal { text, .. } = &expression {
            tcl_syntax::boolean::truthiness_with(text, dialect.numbers)
        } else {
            None
        };
        let mut pending = vec![&expression];
        let mut events = Vec::new();
        let mut scalar_condition = true;
        while let Some(node) = pending.pop() {
            match node {
                ExprNode::Literal { .. } => {}
                ExprNode::Var { text, start, .. } => {
                    let Some(scalar) =
                        self.expression_variable_events(site, &source, text, *start, &mut events)
                    else {
                        return (vec![Event::Unknown], None, None);
                    };
                    scalar_condition &= scalar;
                }
                ExprNode::Unary { operand, .. } => pending.push(operand),
                ExprNode::Binary { op, left, right }
                    if !matches!(op, BinOp::And | BinOp::Or | BinOp::WordAnd | BinOp::WordOr) =>
                {
                    pending.extend([right.as_ref(), left.as_ref()]);
                }
                // Lazy operands and dispatched math/script/string substitutions
                // need their independently retained evaluation topology.
                _ => return (vec![Event::Unknown], None, None),
            }
        }
        (events, value, scalar_condition.then_some(expression))
    }

    fn expression_variable_events(
        &self,
        site: &CommandAllocationSite,
        source: &ExecutedScriptSource,
        text: &str,
        start: u32,
        events: &mut Vec<Event>,
    ) -> Option<bool> {
        let reference =
            tcl_lexer::word_parts::whole_var_ref(text.as_bytes(), self.config).ok()??;
        let start = source.base().checked_add(start)?;
        let span = reference.source_span(text.as_bytes(), 0, start)?;
        let name = std::str::from_utf8(reference.name).ok()?;
        if reference.index.is_some() {
            let end = start.checked_add(u32::try_from(text.len()).ok()?)?;
            let first_event = events.len();
            self.variable(text, tcl_lexer::Span::new(start, end), site, events);
            // The index keeps its original read/script evaluation order, but
            // this expression projection cannot certify the final array read.
            for event in &mut events[first_event..] {
                if matches!(event, Event::Read(_, read_span) if *read_span == span) {
                    *event = Event::ReadUnavailable;
                }
            }
            return Some(false);
        }
        if !tcl_syntax::naming::is_qualified(name.as_bytes())
            && self.entry.owns_read(&site.source, span)
        {
            events.push(Event::AuthoredRead(name.to_owned(), span));
        }
        if !self.retained_local_read(site, span, text, name, true) {
            events.push(Event::ReadUnavailable);
        } else if !tcl_syntax::naming::is_qualified(name.as_bytes()) {
            events.push(Event::Read(name.to_owned(), span));
        }
        Some(true)
    }

    fn sequence(
        &mut self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        arguments: &[usize],
        exits: Exits,
    ) -> usize {
        let mut next = exits.normal;
        for argument in arguments.iter().rev() {
            next = self.body(
                site,
                effective,
                BodyOperand {
                    argument: *argument,
                    list_element: None,
                },
                Exits {
                    normal: next,
                    ..exits
                },
            );
        }
        next
    }
    fn body(
        &mut self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        operand: BodyOperand,
        exits: Exits,
    ) -> usize {
        let id = self.placeholder();
        let source = self.body_source(site, effective, operand);
        if let Some(source) = source {
            self.jobs.push(Job {
                source,
                purpose: tcl_lexer::SourceSegmentationPurpose::NestedScript,
                entry: id,
                exits,
            });
        } else {
            self.unknown(id, exits.normal);
        }
        id
    }
    fn body_source(
        &self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        operand: BodyOperand,
    ) -> Option<ExecutedScriptSource> {
        let written = effective.written_argument(operand.argument)?;
        let word = effective.words.get(operand.argument + 1)?;
        let dialect = self.entry_dialect();
        let crate::registry_invocation::EffectiveInvocationWord::Literal(value) =
            crate::registry_invocation::effective_invocation_word(
                word,
                self.config.escapes,
                dialect.word_values,
            )
        else {
            return None;
        };
        let mut source =
            ExecutedScriptSource::from_word(site.clone(), written, word, &value, self.config);
        if let Some(element) = operand.list_element {
            source = source.list_element(site.clone(), written, element, dialect.word_values)?;
        }
        matches!(source.mapping, ExecutedScriptMapping::Contiguous { .. }).then_some(source)
    }
    fn quoted_mentions(
        &self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        operand: BodyOperand,
    ) -> Result<BTreeSet<String>, ()> {
        use tcl_lexer::word_parts::{ExecutablePart, ExecutablePartArena, SubstFlags};
        if !matches!(
            effective.words.get(operand.argument + 1),
            Some(WordExpr::BracedLiteral { .. })
        ) {
            return Ok(BTreeSet::new());
        }
        let source = self.body_source(site, effective, operand).ok_or(())?;
        let end = source
            .base()
            .checked_add(u32::try_from(source.text.len()).map_err(|_| ())?)
            .ok_or(())?;
        let mut regions = vec![tcl_lexer::Span::new(source.base(), end)];
        let mut mentions = BTreeSet::new();
        while let Some(region) = regions.pop() {
            let arena = ExecutablePartArena::decompose(
                site.source.source_image().clone(),
                region,
                SubstFlags::default(),
                self.config,
            )
            .map_err(|_| ())?;
            let mut lists = vec![arena.root()];
            while let Some(list) = lists.pop() {
                for part in arena.list(list) {
                    match &part.part {
                        ExecutablePart::Variable { name, index } => {
                            let name = std::str::from_utf8(arena.bytes(*name).ok_or(())?)
                                .map_err(|_| ())?;
                            if !tcl_syntax::naming::is_qualified(name.as_bytes()) {
                                mentions.insert(name.to_owned());
                            }
                            lists.extend(*index);
                        }
                        ExecutablePart::Command { body } => regions.push(*body),
                        ExecutablePart::Expression { expression } => regions.push(*expression),
                        ExecutablePart::Text(_) => {}
                        ExecutablePart::ParseError(_) => return Err(()),
                    }
                }
            }
        }
        Ok(mentions)
    }

    fn entry_dialect(&self) -> tcl_registry::InvocationDialect {
        self.dialect
    }
    fn word_events(&self, word: &WordExpr, site: &CommandAllocationSite, output: &mut Vec<Event>) {
        match word {
            WordExpr::Variable { spelling, source } => {
                self.variable(spelling, source.span, site, output);
            }
            WordExpr::CommandSubstitution { spelling, source } => {
                Self::substitution(spelling, source.span, site, output);
            }
            WordExpr::Template { parts, .. } => {
                for part in parts {
                    match part {
                        WordPart::Variable { spelling, source } => {
                            self.variable(spelling, source.span, site, output);
                        }
                        WordPart::CommandSubstitution { spelling, source } => {
                            Self::substitution(spelling, source.span, site, output);
                        }
                        WordPart::Opaque { .. } => output.push(Event::Unknown),
                        WordPart::Text { .. } => {}
                    }
                }
            }
            WordExpr::Expand { .. } | WordExpr::Opaque { .. } => output.push(Event::Unknown),
            _ => {}
        }
    }
    fn variable(
        &self,
        spelling: &str,
        span: tcl_lexer::Span,
        site: &CommandAllocationSite,
        output: &mut Vec<Event>,
    ) {
        #[derive(Clone, Copy)]
        enum Task<'a> {
            Part(&'a tcl_lexer::word_parts::SpannedExecutablePart),
            Read(&'a tcl_lexer::word_parts::SpannedExecutablePart),
        }
        use tcl_lexer::word_parts::{ExecutablePart, ExecutablePartArena, SubstFlags};
        let Some(end) = u32::try_from(spelling.len())
            .ok()
            .and_then(|len| span.start().checked_add(len))
        else {
            output.push(Event::Unknown);
            return;
        };
        if site
            .source
            .source_image()
            .bytes()
            .get(span.start() as usize..end as usize)
            != Some(spelling.as_bytes())
        {
            output.push(Event::Unknown);
            return;
        }
        let Ok(arena) = ExecutablePartArena::decompose(
            site.source.source_image().clone(),
            tcl_lexer::Span::new(span.start(), end),
            SubstFlags::default(),
            self.config,
        ) else {
            output.push(Event::Unknown);
            return;
        };
        let mut tasks = arena
            .list(arena.root())
            .iter()
            .rev()
            .map(Task::Part)
            .collect::<Vec<_>>();
        while let Some(task) = tasks.pop() {
            let part = match task {
                Task::Part(part) | Task::Read(part) => part,
            };
            match task {
                Task::Part(_) => match &part.part {
                    ExecutablePart::Variable { index, .. } => {
                        tasks.push(Task::Read(part));
                        if let Some(index) = index {
                            tasks.extend(arena.list(*index).iter().rev().map(Task::Part));
                        }
                    }
                    ExecutablePart::Command { body } => {
                        if let Some(source) = Self::original_region(site, *body) {
                            output.push(Event::Script(source));
                        } else {
                            output.push(Event::Unknown);
                        }
                    }
                    ExecutablePart::Text(_) => {}
                    _ => output.push(Event::Unknown),
                },
                Task::Read(_) => {
                    let ExecutablePart::Variable { name, .. } = part.part else {
                        unreachable!()
                    };
                    let Some(name) = arena
                        .bytes(name)
                        .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    else {
                        output.push(Event::Unknown);
                        continue;
                    };
                    let Some(read_span) = arena.source_span(part) else {
                        output.push(Event::Unknown);
                        continue;
                    };
                    let Some(original) = arena
                        .bytes(part.span)
                        .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    else {
                        output.push(Event::Unknown);
                        continue;
                    };
                    if !tcl_syntax::naming::is_qualified(name.as_bytes())
                        && self.entry.owns_read(&site.source, read_span)
                    {
                        output.push(Event::AuthoredRead(name.to_owned(), read_span));
                    }
                    if self.retained_local_read(site, read_span, original, name, false) {
                        if !tcl_syntax::naming::is_qualified(name.as_bytes()) {
                            output.push(Event::Read(name.to_owned(), read_span));
                        }
                    } else {
                        output.push(Event::ReadUnavailable);
                    }
                }
            }
        }
    }

    fn retained_local_read(
        &self,
        site: &CommandAllocationSite,
        span: tcl_lexer::Span,
        spelling: &str,
        name: &str,
        expression: bool,
    ) -> bool {
        use super::declaration_layout::{
            direct_declaration_operand_owner, local_read_scope_is_excluded,
        };
        let mut found = false;
        for access in self
            .inventory
            .accesses
            .get(&span.start())
            .into_iter()
            .flatten()
            .chain(
                self.inventory
                    .origin_accesses
                    .get(&site.source)
                    .and_then(|accesses| accesses.get(&span.start()))
                    .into_iter()
                    .flatten(),
            )
        {
            if access.source.span != span || access.original_spelling != spelling {
                continue;
            }
            let owned = if expression {
                strict_expression_owner(&access.owner, site)
            } else {
                direct_declaration_operand_owner(&access.owner, site)
            };
            if !owned {
                return false;
            }
            let Some(observations) = self.inventory.layouts.get(site).and_then(|observations| {
                super::declaration_layout::original_declaration_layouts(observations)
            }) else {
                return false;
            };
            let contexts = access
                .context_alternatives()
                .iter()
                .filter(|context| self.entry.owns_original_context(context))
                .collect::<Vec<_>>();
            if contexts.is_empty() {
                continue;
            }
            if observations.clone().any(|observation| {
                !observation.entry.owns_source(&site.source, span.start())
                    || local_read_scope_is_excluded(
                        &observation.snapshot.state.source_variables,
                        name,
                    )
                    || contexts.iter().any(|context| {
                        local_read_scope_is_excluded(context, name)
                            || context.activation
                                != observation.snapshot.state.source_variables.activation
                    })
            }) {
                return false;
            }
            found = true;
        }
        found
    }

    fn original_region(
        site: &CommandAllocationSite,
        span: tcl_lexer::Span,
    ) -> Option<ExecutedScriptSource> {
        let image = tcl_lexer::SourceImage::from_bytes(
            site.source.source_image().bytes().get(span.as_range())?,
            site.source.source_image().channel(),
        );
        ExecutedScriptSource::contiguous_image(Arc::clone(&site.source), image, span.start())
    }
    fn substitution(
        spelling: &str,
        span: tcl_lexer::Span,
        site: &CommandAllocationSite,
        output: &mut Vec<Event>,
    ) {
        let bytes = spelling.as_bytes();
        if bytes.first() != Some(&b'[')
            || bytes.last() != Some(&b']')
            || site
                .source
                .source_image()
                .bytes()
                .get(span.start() as usize..=span.end() as usize)
                != Some(bytes)
        {
            output.push(Event::Unknown);
            return;
        }
        let image = tcl_lexer::SourceImage::from_bytes(
            &bytes[1..bytes.len() - 1],
            site.source.source_image().channel(),
        );
        if let Some(source) = ExecutedScriptSource::contiguous_image(
            Arc::clone(&site.source),
            image,
            span.start() + 1,
        ) {
            output.push(Event::Script(source));
        } else {
            output.push(Event::Unknown);
        }
    }
    fn evaluate(self, entry: usize) -> DeclarationFlowReport {
        let source = Arc::clone(self.entry.origin());
        let mut report = DeclarationFlowReport {
            source,
            reads: BTreeMap::new(),
            reached: BTreeSet::new(),
            represented: self.nodes.iter().filter_map(|node| node.site).collect(),
            unavailable_before: BTreeSet::new(),
            unavailable: false,
            unknown_reads: false,
            all_reads: BTreeSet::new(),
            authored_reads: BTreeMap::new(),
            warning_reads: BTreeMap::new(),
            conditional_overwrites: Vec::new(),
            declared_defined_reads: BTreeMap::new(),
            authored_uses: BTreeMap::new(),
            named_uses: BTreeSet::new(),
            quoted_uses: BTreeSet::new(),
            quoted_use_unavailable: false,
            declared_entries: BTreeSet::new(),
            conditional_handler_literals: BTreeMap::new(),
            conditional_argument_incoming: BTreeMap::new(),
            conditional_caller_alias_prefixes: BTreeSet::new(),
            aliases: BTreeSet::new(),
            stores: BTreeSet::new(),
            frame: self.entry.clone(),
        };
        let states = self.strict_states(entry);
        self.record_strict_observations(&mut report, &states);
        let declared = self.declared_states(entry, false, false);
        let accumulator_paths = self.declared_states(entry, true, false);
        let warning_paths = self.declared_states(entry, false, true);
        let accumulators: BTreeSet<String> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(id, _)| accumulator_paths[*id].is_some())
            .flat_map(|(_, node)| node.accumulator_writes.iter().cloned())
            .collect();
        self.record_declared_observations(
            &mut report,
            &declared,
            &accumulator_paths,
            &accumulators,
        );
        self.record_warning_observations(&mut report, &warning_paths);
        report
    }

    fn record_warning_observations(
        &self,
        report: &mut DeclarationFlowReport,
        warning_paths: &[Option<State>],
    ) {
        for (id, store) in &self.conditional_overwrites {
            if !report.unknown_reads
                && !report.aliases.contains(&store.name)
                && warning_paths[*id].as_ref().is_some_and(|state| {
                    !state.unknown_writers
                        && !state.uncertain_writes.contains(&store.name)
                        && !state.aliases.contains(&store.name)
                })
            {
                report.conditional_overwrites.push(DeclaredOverwrite {
                    first: store.first.clone(),
                    next: store.next.clone(),
                    name: store.name.clone(),
                    target: store.target,
                });
            }
        }
        for (id, node) in self.nodes.iter().enumerate() {
            let Some(state) = &warning_paths[id] else {
                continue;
            };
            // naming.compiler.original-formal-argument-entry

            if let Some(site) = node.site
                && !state.unknown_writers
            {
                report
                    .conditional_argument_incoming
                    .insert(site, state.incoming.clone());
                if state.aliases.is_subset(&state.caller_frame_aliases) {
                    report.conditional_caller_alias_prefixes.insert(site);
                }
            }
            if let Some(site) = node.handler_site
                && !state.unknown_writers
            {
                let literals = state
                    .literals
                    .iter()
                    .filter(|(name, _)| !state.aliases.contains(*name))
                    .map(|(name, value)| (name.clone(), value.clone()))
                    .collect();
                report.conditional_handler_literals.insert(site, literals);
            }
            for (name, span) in &node.authored_reads {
                if !state.defined.contains(name)
                    && !state.uncertain_writes.contains(name)
                    && !state.aliases.contains(name)
                {
                    report
                        .warning_reads
                        .entry(name.clone())
                        .and_modify(|earlier| {
                            if span.start() < earlier.start() {
                                *earlier = *span;
                            }
                        })
                        .or_insert(*span);
                }
            }
        }
    }

    fn initial_state(&self) -> State {
        let names = self.entry.original_formal_topology().map_or_else(
            || {
                self.entry
                    .parameters()
                    .iter()
                    .map(|formal| formal.name.clone())
                    .collect::<BTreeSet<_>>()
            },
            |topology| {
                topology
                    .parameters()
                    .iter()
                    .filter_map(|formal| std::str::from_utf8(&formal.name).ok().map(str::to_owned))
                    .collect::<BTreeSet<_>>()
            },
        );
        State {
            incoming: names.clone(),
            defined: names,
            ..State::default()
        }
    }

    fn strict_states(&self, entry: usize) -> Vec<Option<State>> {
        let mut states = vec![None; self.nodes.len()];
        states[entry] = Some(self.initial_state());
        let mut queue = VecDeque::from([entry]);
        while let Some(id) = queue.pop_front() {
            let node = &self.nodes[id];
            let mut state = states[id]
                .as_ref()
                .expect("queued diagnostic state")
                .clone();
            state.unknown_writers |= node.unknown;
            if node.unknown {
                state.incoming.clear();
            }
            for name in node
                .removes
                .iter()
                .chain(&node.writes)
                .chain(&node.possible_output_writes)
                .chain(&node.aliases)
            {
                state.incoming.remove(name);
            }
            state.aliases.extend(node.aliases.iter().cloned());
            for name in &node.removes {
                state.defined.remove(name);
            }
            state.defined.extend(node.writes.iter().cloned());
            Self::transfer_uncertain_writes(node, &mut state);
            for &successor in &node.successors {
                let changed = if let Some(existing) = &mut states[successor] {
                    existing.join(&state)
                } else {
                    states[successor] = Some(state.clone());
                    true
                };
                if changed {
                    queue.push_back(successor);
                }
            }
        }
        states
    }

    fn record_strict_observations(
        &self,
        report: &mut DeclarationFlowReport,
        states: &[Option<State>],
    ) {
        for (id, node) in self.nodes.iter().enumerate() {
            let Some(state) = &states[id] else {
                continue;
            };
            let source_catalogue_only = node.availability.schema_origin
                == crate::registry_invocation::DeclarationSchemaOrigin::OriginalSourceCatalogue;
            report.unavailable |= node.unknown || source_catalogue_only;
            if let Some(site) = node.site {
                report.reached.insert(site);
                if state.unknown_writers || source_catalogue_only {
                    report.unavailable_before.insert(site);
                }
            }
            for name in &node.named_reads {
                report.authored_uses.entry(name.clone()).or_default();
                report.named_uses.insert(name.clone());
            }
            report.unknown_reads |= node.availability.unknown_reads;
            report.all_reads.extend(node.named_reads.iter().cloned());
            report.all_reads.extend(node.name_queries.iter().cloned());
            report.named_uses.extend(node.name_queries.iter().cloned());
            report
                .all_reads
                .extend(node.reads.iter().map(|(name, _)| name.clone()));
            report.aliases.extend(node.aliases.iter().cloned());
            if let Some(site) = node.store_site {
                report
                    .stores
                    .extend(node.writes.iter().map(|name| (site, name.clone())));
            }
            for (name, span) in &node.reads {
                if !state.unknown_writers
                    && !state.defined.contains(name)
                    && !state.uncertain_writes.contains(name)
                    && !state.aliases.contains(name)
                {
                    report
                        .reads
                        .entry(name.clone())
                        .and_modify(|old| {
                            if span.start() < old.start() {
                                *old = *span;
                            }
                        })
                        .or_insert(*span);
                }
            }
        }
    }

    fn record_declared_observations(
        &self,
        report: &mut DeclarationFlowReport,
        declared: &[Option<State>],
        accumulator_paths: &[Option<State>],
        accumulators: &BTreeSet<String>,
    ) {
        for (id, node) in self.nodes.iter().enumerate() {
            let Some(state) = &declared[id] else {
                continue;
            };
            report
                .quoted_uses
                .extend(node.quoted_mentions.iter().cloned());
            report.quoted_use_unavailable |= node.availability.quoted_use_unavailable;
            // Original argument entry precedes this command's dispatch.
            // An unavailable handler layout cannot erase that candidate entry
            // or grant successful dispatch to any consumer.
            if let Some(site) = node.site {
                report.declared_entries.insert(site);
            }
            for (name, span) in &node.authored_reads {
                report
                    .authored_uses
                    .entry(name.clone())
                    .or_default()
                    .push(*span);
                let accumulator_defined = accumulators.contains(name)
                    && accumulator_paths[id]
                        .as_ref()
                        .is_some_and(|state| state.defined.contains(name));
                if !state.unknown_writers
                    && !state.uncertain_writes.contains(name)
                    && (state.defined.contains(name) || accumulator_defined)
                {
                    report
                        .declared_defined_reads
                        .entry(name.clone())
                        .or_default()
                        .push(*span);
                }
                if !state.defined.contains(name)
                    && !accumulator_defined
                    && !state.aliases.contains(name)
                {
                    report
                        .authored_reads
                        .entry(name.clone())
                        .and_modify(|old| {
                            if span.start() < old.start() {
                                *old = *span;
                            }
                        })
                        .or_insert(*span);
                }
            }
        }
    }

    fn declared_states(
        &self,
        entry: usize,
        assumes_accumulator_iterations: bool,
        requires_available_prefix: bool,
    ) -> Vec<Option<State>> {
        let mut states = vec![None; self.nodes.len()];
        states[entry] = Some(self.initial_state());
        let policy = crate::tcl_expr_eval::FoldPolicy::from_octal(None)
            .with_invocation_dialect(self.dialect);
        let mut queue = VecDeque::from([entry]);
        while let Some(id) = queue.pop_front() {
            let node = &self.nodes[id];
            let mut state = states[id].as_ref().expect("queued declared state").clone();
            Self::transfer_declared_state(node, &mut state, policy);
            if requires_available_prefix
                && (state.unknown_writers || (node.unknown && !node.declared_read_unavailable))
            {
                continue;
            }
            let successors =
                Self::declared_successors(node, &state, policy, assumes_accumulator_iterations);
            for successor in successors {
                let changed = if let Some(existing) = &mut states[successor] {
                    existing.join(&state)
                } else {
                    states[successor] = Some(state.clone());
                    true
                };
                if changed {
                    queue.push_back(successor);
                }
            }
        }
        states
    }
    fn transfer_declared_state(
        node: &Node,
        state: &mut State,
        policy: crate::tcl_expr_eval::FoldPolicy,
    ) {
        let previous = state.literals.clone();
        state.unknown_writers |= node.availability.unavailable_declared_prefix;
        if node.unknown && !node.declared_read_unavailable {
            state.literals.clear();
            state.incoming.clear();
        }
        if state.unknown_writers {
            state.incoming.clear();
        }
        for name in node
            .removes
            .iter()
            .chain(&node.writes)
            .chain(&node.possible_output_writes)
            .chain(&node.aliases)
        {
            state.incoming.remove(name);
        }
        for name in node.removes.iter().chain(&node.aliases) {
            state.caller_frame_aliases.remove(name);
        }
        state
            .caller_frame_aliases
            .extend(node.caller_frame_aliases.iter().cloned());
        state.aliases.extend(node.aliases.iter().cloned());
        for name in node.removes.iter().chain(&node.writes).chain(&node.aliases) {
            state.literals.remove(name);
        }
        for name in &node.removes {
            state.defined.remove(name);
        }
        state.defined.extend(node.writes.iter().cloned());
        state
            .defined
            .extend(node.possible_output_writes.iter().cloned());
        for (name, value) in &node.literal_stores {
            if !name.contains("::") && !name.contains('(') && !state.aliases.contains(name) {
                state.literals.insert(name.clone(), value.clone());
            }
        }
        for (name, amount) in &node.increments {
            let value = previous.get(name).and_then(|value| {
                let crate::tcl_expr_eval::TclValue::Int(old) =
                    crate::tcl_expr_eval::parse_integer_operand_with_policy(value, policy)?
                else {
                    return None;
                };
                let crate::tcl_expr_eval::TclValue::Int(amount) =
                    crate::tcl_expr_eval::parse_integer_operand_with_policy(amount, policy)?
                else {
                    return None;
                };
                old.checked_add(amount).map(|value| value.to_string())
            });
            if let Some(value) = value {
                state.literals.insert(name.clone(), value);
            }
        }
        Self::transfer_uncertain_writes(node, state);
    }

    fn transfer_uncertain_writes(node: &Node, state: &mut State) {
        for name in node.writes.iter().chain(&node.removes).chain(&node.aliases) {
            state.uncertain_writes.remove(name);
        }
        for name in &node.uncertain_writes {
            state.uncertain_writes.insert(name.clone());
            state.defined.remove(name);
            state.incoming.remove(name);
            state.literals.remove(name);
        }
    }

    fn declared_successors(
        node: &Node,
        state: &State,
        policy: crate::tcl_expr_eval::FoldPolicy,
        assumes_accumulator_iterations: bool,
    ) -> Vec<usize> {
        if let Some(condition) = &node.condition {
            let env = state
                .literals
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        crate::tcl_expr_eval::EnvValue::Str(value.clone()),
                    )
                })
                .collect();
            match crate::tcl_expr_eval::eval_tcl_expr_with_policy(
                &condition.expression,
                &env,
                policy,
            )
            .map(|value| value.is_truthy())
            {
                Some(true) => vec![condition.yes],
                Some(false) => vec![condition.no],
                None if condition.first_iteration => vec![condition.yes],
                None => node.successors.clone(),
            }
        } else if assumes_accumulator_iterations && node.unknown_list_first.is_some() {
            // The accumulator diagnostic states the condition that each
            // enclosing unknown list loop iterates; runtime zero-trip
            // paths remain in the strict graph and ordinary read advice.
            node.unknown_list_first.into_iter().collect()
        } else {
            node.declared_successors
                .as_ref()
                .unwrap_or(&node.successors)
                .clone()
        }
    }
}
enum Event {
    AuthoredRead(String, tcl_lexer::Span),
    Read(String, tcl_lexer::Span),
    Script(ExecutedScriptSource),
    ReadUnavailable,
    Unknown,
}

fn strict_expression_owner(
    owner: &super::SourceVariableEvaluationOwner,
    site: &CommandAllocationSite,
) -> bool {
    let mut pending = vec![owner];
    while let Some(owner) = pending.pop() {
        match owner {
            super::SourceVariableEvaluationOwner::NativeExpression { invocation, .. }
                if invocation == site => {}
            super::SourceVariableEvaluationOwner::Alternatives(owners) if !owners.is_empty() => {
                pending.extend(owners);
            }
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::interprocedural::test_body_identity::fixture_id_if_present;

    use super::*;

    #[test]
    fn lifecycle_subcommand_schedules_its_original_phase_scripts() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut registry =
            tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let mut descriptor = registry.get("tcltest::test").unwrap().clone();
        descriptor.name = "offset-lifecycle";
        descriptor.required_package = None;
        descriptor.subcommands = Box::leak(Box::new([tcl_registry::SubCommand {
            name: "run",
            arity: descriptor.arity,
            arg_role_resolver: descriptor.arg_role_resolver,
            arg_role_resolver_roles: descriptor.arg_role_resolver_roles,
            body_kind: descriptor.body_kind,
            body_execution: descriptor.body_execution,
            completion: descriptor.completion,
            ..tcl_registry::SubCommand::DEFAULT
        }]));
        registry.insert(descriptor);
        let loader = crate::lowering::stock_body_provider_loader(
            tcl_registry::body_execution::TCLTEST_STOCK_PROVIDER,
            None,
        )
        .unwrap();
        let wrapper = "offset-lifecycle run n d -cleanup {puts CLEANUP} -body {puts BODY} -setup {puts SETUP}";
        let source = format!("package require tcltest\nproc p {{}} {{{wrapper}}}");
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let bindings = super::super::SourceCommandBindings::analyse_with_options(
            &source,
            config,
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                trusted_package_loaders: std::slice::from_ref(&loader),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find(wrapper).unwrap()).unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(wrapper, offset, config)
                .remove(0);
        let mut tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceImage::document(&source).source_map(),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref().unwrap();
        let site = binding.invocation_site().unwrap();
        let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
        let observation = super::super::declaration_layout::original_declaration_layouts(
            inventory.layouts.get(site).unwrap(),
        )
        .unwrap()
        .next()
        .unwrap();
        let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
        let mut builder = Builder {
            inventory,
            registry: &registry,
            config,
            dialect: tcl_registry::InvocationDialect::of_profile(profile),
            entry: &frame,
            nodes: Vec::new(),
            jobs: Vec::new(),
            conditional_overwrites: Vec::new(),
            read_sites: BTreeSet::new(),
        };
        let (selected, lifecycle) = builder.selection(&tokens, site).unwrap();
        let lifecycle = lifecycle.unwrap();
        assert_eq!(lifecycle.invocation.argument_offset, 1);
        let after = builder.placeholder();
        builder.lifecycle_flow(site, &selected, &lifecycle, after, Exits::all(after));
        let scripts = builder
            .jobs
            .iter()
            .map(|job| job.source.text.try_text().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(scripts, ["puts CLEANUP", "puts BODY", "puts SETUP"]);
    }

    #[test]
    fn declaration_selection_keeps_absent_dispatch_separate_from_invalid_reached_owner() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let source = "set anchor 1; puts $absent";
        let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let mut tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceImage::document(source).source_map(),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref().unwrap();
        let site = binding.invocation_site().unwrap();
        let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
        let observation = super::super::declaration_layout::original_declaration_layouts(
            inventory.layouts.get(site).unwrap(),
        )
        .unwrap()
        .next()
        .unwrap();
        let dialect = observation
            .snapshot
            .state
            .source_variables
            .invocation_dialect
            .unwrap();
        let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
        let selection = |inventory| {
            let builder = Builder {
                inventory,
                registry,
                config,
                dialect,
                entry: &frame,
                nodes: Vec::new(),
                jobs: Vec::new(),
                conditional_overwrites: Vec::new(),
                read_sites: BTreeSet::new(),
            };
            let mut raw = tokens.clone();
            raw.source_binding = None;
            builder.selection(&raw, site)
        };
        assert!(
            selection(inventory).is_some(),
            "genuine original reached owner"
        );
        let absent = DeclarationFlowInventory::new(
            Arc::clone(&inventory.layouts),
            Arc::clone(&inventory.accesses),
            Arc::clone(&inventory.origin_accesses),
            Arc::new(Vec::new()),
            Arc::new(BTreeMap::new()),
        );
        assert!(
            selection(&absent).is_some(),
            "sealed raw declaration without dispatch"
        );
        let mut invalid = inventory.points.as_ref().clone();
        assert!(
            invalid
                .iter()
                .any(|point| point.dispatch && point.offset == site.offset)
        );
        for point in &mut invalid {
            point.original_written_words = None;
            point.compiled_execution.original_words = None;
        }
        let invalid = DeclarationFlowInventory::new(
            Arc::clone(&inventory.layouts),
            Arc::clone(&inventory.accesses),
            Arc::clone(&inventory.origin_accesses),
            Arc::new(invalid),
            Arc::new(BTreeMap::new()),
        );
        assert!(
            selection(&invalid).is_none(),
            "invalid reached owner remains terminal"
        );
    }

    #[test]
    fn source_read_scope_preserves_reached_owner_refusal_after_loop() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let source = "proc p {n} {set anchor 1; while {$n > 0} {set x 1; incr n -1}; puts $x}";
        let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
        let offset = u32::try_from(source.find("puts $x").unwrap()).unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config("puts $x", offset, config)
                .remove(0);
        let mut tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceImage::document(source).source_map(),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref().unwrap();
        let site = binding.invocation_site().unwrap();
        let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
        let observation = super::super::declaration_layout::original_declaration_layouts(
            inventory.layouts.get(site).unwrap(),
        )
        .unwrap()
        .next()
        .unwrap();
        let dialect = observation
            .snapshot
            .state
            .source_variables
            .invocation_dialect
            .unwrap();
        let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
        let selected = |inventory| {
            let builder = Builder {
                inventory,
                registry,
                config,
                dialect,
                entry: &frame,
                nodes: Vec::new(),
                jobs: Vec::new(),
                conditional_overwrites: Vec::new(),
                read_sites: BTreeSet::new(),
            };
            let mut raw = tokens.clone();
            raw.source_binding = None;
            builder.source_read_selection(&raw, site)
        };
        assert!(selected(inventory).is_some());
        let mut points = inventory.points.as_ref().clone();
        assert!(
            points
                .iter()
                .any(|point| point.dispatch && point.offset == site.offset)
        );
        for point in &mut points {
            point.original_written_words = None;
            point.compiled_execution.original_words = None;
        }
        let mut invalid = DeclarationFlowInventory::new(
            Arc::clone(&inventory.layouts),
            Arc::clone(&inventory.accesses),
            Arc::clone(&inventory.origin_accesses),
            Arc::new(points),
            Arc::new(BTreeMap::new()),
        );
        invalid.source_seed = inventory.source_seed.clone();
        assert!(
            selected(&invalid).is_none(),
            "malformed reached metadata stays terminal"
        );
    }

    fn report(source: &str) -> Arc<DeclarationFlowReport> {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
        let original = "set anchor 1";
        let offset = u32::try_from(source.find(original).unwrap()).unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(original, offset, config)
                .remove(0);
        let mut tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceImage::document(source).source_map(),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_flow_report(registry)
            .expect("accepted original declaration owns a diagnostic graph")
    }

    #[test]
    fn original_declaration_flow_cache_retains_full_inventory_and_query_identity() {
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = owner.commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let source = "set anchor 1; list $anchor";
        let mut bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
        let call = bindings.invocation_at_source("set", 0);
        let first = call.declaration_flow_report(registry).unwrap();
        let again = bindings
            .invocation_at_source("set", 0)
            .declaration_flow_report(registry)
            .unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        let inventory = call.declaration_flow_inventory.as_ref().unwrap();
        let key = inventory.reports.0.lock().unwrap()[0].0.clone();
        let mut changed = key.clone();
        changed.config.strict_quoting = !changed.config.strict_quoting;
        assert!(inventory.reports.get(&changed).is_none());
        changed = key.clone();
        changed.registry = tcl_registry::model::ingress::static_context_for("tcl8.5")
            .commands()
            .snapshot()
            .semantic_key();
        assert!(inventory.reports.get(&changed).is_none());
        changed = key.clone();
        let DeclarationFlowFrame::Existing(original_entry) = key.entry.as_ref() else {
            panic!("retained original diagnostic frame");
        };
        let super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
            source,
            frame,
            namespace,
        } = original_entry
        else {
            panic!("original root entry");
        };
        let mut foreign_source = source.clone();
        foreign_source.text = tcl_lexer::SourceImage::native(source.text.bytes());
        foreign_source.origin = Arc::new(super::super::SourceOriginId::authored_image(
            foreign_source.text.clone(),
        ));
        changed.entry = Arc::new(DeclarationFlowFrame::Existing(
            super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
                source: foreign_source,
                frame: frame.clone(),
                namespace: namespace.clone(),
            },
        ));
        assert!(inventory.reports.get(&changed).is_none());
        let clone = bindings.clone();
        assert!(clone.declaration_flow_cache.0.lock().unwrap().is_none());
        let detached = clone.current_declaration_flow_inventory();
        assert!(!Arc::ptr_eq(inventory, &detached));
        assert!(detached.reports.0.lock().unwrap().is_empty());
        let point = bindings.points[0].clone();
        bindings.points.push(point);
        let next = bindings.current_declaration_flow_inventory();
        assert!(!Arc::ptr_eq(inventory, &next));
        assert!(next.reports.0.lock().unwrap().is_empty());
        assert!(Arc::ptr_eq(
            &first,
            &call.declaration_flow_report(registry).unwrap()
        ));
    }

    #[test]
    fn original_diagnostic_frame_keeps_root_values_and_expanded_loop_bindings() {
        let source = "set spec {{1}}; set names {x}; foreach $names {*}$spec {puts $x}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let mut tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceImage::document(source).source_map(),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        let result = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_flow_report(registry)
            .expect("original global script owns a diagnostic frame");
        assert!(!result.potential_declared_reads().contains_key("x"));
        assert!(result.has_authored_use("x"));
    }

    #[test]
    fn original_declaration_graph_preserves_nested_missing_local_reads() {
        for (source, name) in [
            (
                "proc f {n} {set anchor 1; switch -- $n a {for {set j 0} {$j < 3} {incr j} {puts $missing}}}",
                "missing",
            ),
            (
                "proc f {dict} {set anchor 1; ::foreach {k v} $dict {puts \"$k=$missing\"}}",
                "missing",
            ),
            (
                "proc f {items} {set anchor 1; foreach x $items {puts $y; set y $x}}",
                "y",
            ),
        ] {
            let result = report(source);
            let span = result
                .potential_declared_reads()
                .get(name)
                .copied()
                .unwrap_or_else(|| panic!("original occurrence missing: {source}"));
            assert!(
                result.allows_declared_absence_warning(name, span),
                "original prefix unavailable: {source}"
            );
        }
    }

    #[test]
    fn conditional_missing_read_warning_keeps_first_entry_and_rejects_unknown_prefixes() {
        let first =
            report("proc p {flag} {set anchor 1; while {$flag} {puts $missing; operation}}");
        let span = first.declared_absence_warning_occurrences()["missing"];
        assert!(first.allows_declared_absence_warning("missing", span));
        assert!(!first.local_store_may_be_unread(
            &CommandAllocationSite {
                source: Arc::clone(&first.source),
                offset: span.start(),
            },
            "missing"
        ));
        for source in [
            "proc p {} {set anchor 1; operation; puts $missing}",
            "proc p {name} {set anchor 1; set $name 1; puts $missing}",
        ] {
            let blocked = report(source);
            assert!(blocked.potential_declared_reads().contains_key("missing"));
            assert!(
                !blocked
                    .declared_absence_warning_occurrences()
                    .contains_key("missing"),
                "{source}"
            );
        }
    }

    #[test]
    fn original_declaration_overwrite_advice_is_conditional_and_ordered() {
        let source = "proc p {target} {set anchor 1; set b [set {$target}]; set a 1; set a 2; return \"$a$b\"}";
        let declared = report(source);
        let occurrences = declared
            .conditional_overwrite_occurrences()
            .collect::<Vec<_>>();
        let (first, next, name, target) = occurrences
            .iter()
            .find(|(_, _, name, _)| *name == "a")
            .expect("the declared adjacent setters retain only conditional ordering");
        assert!(first.offset < next.offset);
        assert_eq!(
            &source[target.start() as usize..target.end() as usize],
            *name
        );
        let advice = crate::registry_invocation::conditional_declared_overwrite_advice(&declared)
            .find(|advice| advice.name() == "a")
            .unwrap();
        assert!(advice.owns_source(&tcl_lexer::SourceImage::document(source)));
        assert!(
            !advice.owns_source(&tcl_lexer::SourceImage::document(&format!(
                "# relocated\n{source}"
            ),))
        );
        for source in [
            "proc p {target} {set anchor 1; set b [set $target]; set a 1; set a 2; return \"$a$b\"}",
            "proc p {} {set anchor 1; operation; set a 1; set a 2; return $a}",
            "proc p {} {set anchor 1; set a 1; puts $a; set a 2; return $a}",
            "proc p {} {set anchor 1; upvar 1 remote a; set a 1; set a 2; return $a}",
            "proc p {} {set anchor 1; set ::a 1; set ::a 2; return $::a}",
        ] {
            assert!(
                report(source)
                    .conditional_overwrite_occurrences()
                    .all(|(_, _, name, _)| name != "a" && name != "::a"),
                "{source}"
            );
        }
    }

    #[test]
    fn original_procedure_store_graph_survives_abrupt_reads_without_unknown_writer_grants() {
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = owner.commands();
        let profile = registry.profile().unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        for (source, expected) in [
            (
                "proc f {p} {set b [set {$p}]; set a 1; set a 2; return \"$a$b\"}",
                true,
            ),
            (
                "proc f {p} {set b [set $p]; set a 1; set a 2; return \"$a$b\"}",
                false,
            ),
            (
                "proc f {p} {unknown_host; set b [set {$p}]; set a 1; set a 2; return \"$a$b\"}",
                false,
            ),
            (
                "proc f {p} {set b [set {$p}]; rename set saved; set a 1; set a 2; return $a}",
                false,
            ),
            (
                "proc f {p} {set b [set {$p}]; operation; set a 1; set a 2; return $a}",
                false,
            ),
            (
                "proc f {p} {set b [set {$p}]; set other [operation]; set a 1; set a 2; return $a}",
                false,
            ),
        ] {
            let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
            let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
                source, registry, false, profile,
            );
            let procedure = fixture_id_if_present(&unit.ir_module.procedures, "::f")
                .and_then(|id| unit.ir_module.procedures.get(&id))
                .unwrap();
            let image = tcl_lexer::SourceImage::document(source);
            let report = bindings
                .original_procedure_declaration_flow(procedure, &image, registry)
                .expect("the exact original declaration owns conditional diagnostics");
            assert_eq!(
                report
                    .conditional_overwrite_occurrences()
                    .any(|(_, _, name, _)| name == "a"),
                expected,
                "{source}"
            );
            assert!(
                bindings
                    .original_procedure_declaration_flow(
                        procedure,
                        &tcl_lexer::SourceImage::document(&format!("{source}\n")),
                        registry,
                    )
                    .is_none()
            );
            let mut changed = procedure.clone();
            changed.params.push("different".to_owned());
            assert!(
                bindings
                    .original_procedure_declaration_flow(&changed, &image, registry)
                    .is_none()
            );
        }
    }

    #[test]
    fn unentered_declaration_suffix_does_not_grant_original_dispatch() {
        let source = "proc f {p} {set b [set {$p}]; set a 1; set a 2; return $a}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl8.6")).unwrap(),
        );
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let bindings = super::super::SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..super::super::SourceAnalysisOptions::default()
            },
        );
        let original = "set a 1";
        let offset = u32::try_from(source.find(original).unwrap()).unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(original, offset, config)
                .remove(0);
        let mut tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceImage::document(source).source_map(),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding.declaration_operand_layout_advice(&tokens).is_some(),
            "site={:?}, layout={:?}",
            binding.invocation_site(),
            binding
                .declaration_layout_observations
                .as_ref()
                .map(|observations| observations
                    .iter()
                    .map(|observation| (
                        &observation.namespace,
                        &observation.snapshot.state.variable_frame,
                        observation
                            .snapshot
                            .state
                            .targets("set", &observation.namespace)
                            .iter()
                            .map(|target| (target.command.clone(), target.registry_backed))
                            .collect::<Vec<_>>()
                    ))
                    .collect::<Vec<_>>())
        );
        assert!(!binding.unobserved_native_dispatch());
        assert!(!binding.original_arguments_complete_normally(&tokens));
    }

    #[test]
    fn original_declaration_graph_keeps_zero_trip_and_no_match_successors() {
        for (source, missing) in [
            (
                "proc p {mode} {set anchor 1; switch -- $mode {a {set x 1}}; puts $x}",
                true,
            ),
            (
                "proc p {mode} {set anchor 1; switch -- $mode {a {set x 1} default {set x 2}}; puts $x}",
                false,
            ),
            (
                "proc p {mode} {set anchor 1; switch -- $mode {a {return} default {error STOP}}; puts $x}",
                false,
            ),
            (
                "proc p {rows} {set anchor 1; foreach i $rows {set x 1}; puts $x}",
                true,
            ),
            (
                "proc p {} {set anchor 1; while {0} {set x 1}; puts $x}",
                true,
            ),
            (
                "proc p {} {set anchor 1; if {0} {puts $x}; set x 1; puts $x}",
                false,
            ),
        ] {
            let result = report(source);
            assert_eq!(
                result.potential_missing_reads().contains_key("x"),
                missing,
                "{source}"
            );
        }
    }

    #[test]
    fn conditional_reachability_preserves_terminal_and_unavailable_paths() {
        for (source, expected) in [
            ("proc p {} {set anchor 1; return; puts AFTER}", Some(false)),
            (
                "proc p {} {set anchor 1; set x [operation]; puts AFTER}",
                None,
            ),
            ("proc p {} {set anchor 1; set x 1; puts AFTER}", Some(true)),
        ] {
            let result = report(source);
            let site = CommandAllocationSite {
                source: Arc::clone(&result.source),
                offset: u32::try_from(source.find("puts AFTER").unwrap()).unwrap(),
            };
            assert_eq!(
                result.invocation_may_be_reached(&site),
                expected,
                "{source}"
            );
            let other = CommandAllocationSite {
                source: Arc::new(super::super::SourceOriginId::authored(&"puts AFTER".into())),
                offset: 0,
            };
            assert_eq!(result.invocation_may_be_reached(&other), None);
            let unrepresented = CommandAllocationSite {
                source: Arc::clone(&result.source),
                offset: 1,
            };
            assert_eq!(result.invocation_may_be_reached(&unrepresented), None);
        }
    }

    #[test]
    fn array_index_effects_do_not_become_scalar_read_permission() {
        let source = "proc p {} {set anchor 1; puts $a([operation]); puts $x}";
        let result = report(source);
        assert!(!result.potential_missing_reads().contains_key("x"));
        assert_eq!(
            result.invocation_may_be_reached(&CommandAllocationSite {
                source: Arc::clone(&result.source),
                offset: u32::try_from(source.rfind("puts").unwrap()).unwrap(),
            }),
            None
        );
    }

    #[test]
    fn declared_loop_first_entry_does_not_supply_runtime_zero_trip_proof() {
        for (source, missing) in [
            (
                "proc p {n} {set anchor 1; while {$n > 0} {set x 1; incr n -1}; puts $x}",
                false,
            ),
            (
                "proc p {} {set anchor 1; foreach i {a b} {set x $i}; puts $x}",
                false,
            ),
            (
                "proc p {} {set anchor 1; foreach i {} {set x $i}; puts $x}",
                true,
            ),
            (
                "proc p {rows} {set anchor 1; foreach i $rows {set x $i}; puts $x}",
                true,
            ),
            (
                "proc p {} {set anchor 1; for {set i 0} {$i < 3} {incr i} {set x $i}; puts $x}",
                false,
            ),
            (
                "proc p {} {set anchor 1; for {set i 5} {$i < 3} {incr i} {set x $i}; puts $x}",
                true,
            ),
            (
                "proc p {} {set anchor 1; for {set i 0; incr i 5} {$i < 3} {incr i} {set x $i}; puts $x}",
                true,
            ),
            (
                "proc p {n} {set anchor 1; for {set i 0; set i $n} {$i < 3} {incr i} {set x $i}; puts $x}",
                false,
            ),
            (
                "proc p {} {set anchor 1; foreach i {1} {continue; set x 1}; puts $x}",
                true,
            ),
            (
                "proc p {rows} {set anchor 1; foreach i $rows {puts $x; set x $i}}",
                true,
            ),
            (
                "proc p {rows cols} {set anchor 1; foreach r $rows {foreach c $cols {lappend x [list $r $c]}}; puts $x}",
                false,
            ),
            (
                "proc p {rows cols} {set anchor 1; foreach r $rows {foreach c $cols {if {$c} {lappend x $c}}}; puts $x}",
                true,
            ),
            (
                "proc p {rows} {set anchor 1; foreach r $rows {puts $x; lappend x $r}}",
                true,
            ),
        ] {
            let result = report(source);
            assert_eq!(
                result.potential_declared_reads().contains_key("x"),
                missing,
                "{source}"
            );
        }
        let source = "proc p {n} {set anchor 1; while {$n > 0} {set x 1; incr n -1}; puts $x}";
        let result = report(source);
        let site = CommandAllocationSite {
            source: Arc::clone(&result.source),
            offset: u32::try_from(source.rfind("puts").unwrap()).unwrap(),
        };
        assert!(result.declared_argument_entry(&site));
        assert_eq!(result.invocation_may_be_reached(&site), None);
        assert!(!result.local_store_may_be_unread(&site, "x"));
    }

    #[test]
    fn indexed_conditions_preserve_declared_zero_trip_reads() {
        let source = "proc p {key} {set anchor 1; while {$a($key)} {set x 1}; puts $x}";
        let result = report(source);
        assert!(result.has_authored_use("a"));
        assert!(result.has_authored_use("key"));
        assert!(result.potential_declared_reads().contains_key("a"));
        assert!(result.potential_declared_reads().contains_key("x"));
        let after_loop = CommandAllocationSite {
            source: Arc::clone(&result.source),
            offset: u32::try_from(source.rfind("puts").unwrap()).unwrap(),
        };
        assert_eq!(result.invocation_may_be_reached(&after_loop), None);
        assert!(!result.local_store_may_be_unread(&after_loop, "x"));
    }

    #[test]
    fn c86_indexed_conditions_preserve_original_terminal_scripts() {
        // C8.6 executes the index before reading the array and propagates its
        // return. Jim's index completion contract differs from this fixture.
        let source = "proc p {} {set anchor 1; while {$a([return DONE])} {set x 1}; puts $x}";
        let result = report(source);
        assert!(!result.has_authored_use("x"));
        assert!(!result.potential_declared_reads().contains_key("x"));
        let after_loop = CommandAllocationSite {
            source: Arc::clone(&result.source),
            offset: u32::try_from(source.rfind("puts").unwrap()).unwrap(),
        };
        assert_eq!(result.invocation_may_be_reached(&after_loop), Some(false));
        assert!(!result.declared_argument_entry(&after_loop));
    }

    #[test]
    fn authored_read_occurrences_preserve_unknown_runtime_and_array_effects() {
        let source = "proc p {} {set anchor 1; puts $a([operation]); puts $x}";
        let result = report(source);
        assert!(result.potential_declared_reads().contains_key("a"));
        assert!(result.potential_declared_reads().contains_key("x"));
        assert!(
            !result.allows_declared_absence_warning("x", result.potential_declared_reads()["x"])
        );
        assert!(!result.potential_missing_reads().contains_key("x"));
        let site = CommandAllocationSite {
            source: Arc::clone(&result.source),
            offset: u32::try_from(source.rfind("puts").unwrap()).unwrap(),
        };
        assert!(result.owns_invocation(&site));
        assert_eq!(result.invocation_may_be_reached(&site), None);
        assert!(!result.local_store_may_be_unread(&site, "x"));
        assert!(!result.owns_invocation(&CommandAllocationSite {
            source: Arc::new(super::super::SourceOriginId::authored(&source.into())),
            offset: 1,
        }));
    }

    #[test]
    fn conditional_output_candidates_do_not_become_physical_stores() {
        for pattern in ["[a-z]", "^"] {
            let source = format!("proc p {{}} {{set anchor 1; regexp {{{pattern}}} x v; puts $v}}");
            let result = report(&source);
            assert!(!result.potential_declared_reads().contains_key("v"));
            assert!(!result.local_store_may_be_unread(
                &CommandAllocationSite {
                    source: Arc::clone(&result.source),
                    offset: u32::try_from(source.find("regexp").unwrap()).unwrap(),
                },
                "v"
            ));
        }
        let result = report("proc p {} {set anchor 1; regexp x y v; puts $v}");
        assert!(result.potential_declared_reads().contains_key("v"));
        let result = report("proc p {} {set anchor 1; operation; puts $other}");
        let span = result.potential_declared_reads()["other"];
        assert!(result.has_authored_use("other"));
        assert!(!result.allows_declared_absence_warning("other", span));
        let result = report("proc p {} {set anchor 1; error stop; puts $other}");
        assert!(!result.has_authored_use("other"));
    }

    #[test]
    fn quoted_data_mentions_do_not_supply_executed_reads() {
        for body in [
            "puts {$literal}",
            "foreach item {$literal} {puts $item}",
            "switch -- mode {$literal {puts ARM} default {puts DEFAULT}}",
        ] {
            let source = format!("proc p {{}} {{set anchor 1; {body}}}");
            let result = report(&source);
            assert!(result.has_quoted_use("literal"), "{body}");
            assert!(!result.has_authored_use("literal"), "{body}");
            assert!(
                !result.potential_missing_reads().contains_key("literal"),
                "{body}"
            );
            assert!(
                !result.potential_declared_reads().contains_key("literal"),
                "{body}"
            );
        }
        let result = report("proc p {} {set anchor 1; puts {plain}; return; puts {$dead}}");
        assert!(!result.has_quoted_use("plain"));
        assert!(!result.has_quoted_use("dead"));
        let result = report("proc p {} {set anchor 1; puts $live}");
        assert!(!result.has_quoted_use("live"));
        assert!(result.has_authored_use("live"));
    }

    #[test]
    fn declared_aliases_and_terminal_paths_keep_distinct_read_occurrences() {
        let result = report(
            "proc p {} {set anchor 1; variable ::ns::children; puts $children; puts $other; return; puts $dead}",
        );
        assert!(result.has_declared_alias("children"));
        assert!(!result.potential_declared_reads().contains_key("children"));
        assert!(result.potential_declared_reads().contains_key("other"));
        assert!(!result.has_authored_use("dead"));
        let result = report("proc p {} {set anchor 1; set {$n} 1; set {$n} 2; return [set {$n}]}");
        assert!(result.has_named_use("$n"));
        assert!(!result.has_authored_use("n"));
    }

    #[test]
    fn original_call_entry_precedes_its_unknown_dispatch() {
        let source = "proc p {} {set anchor 1; run notacommand; puts AFTER}";
        let result = report(source);
        for (command, expected) in [("run notacommand", Some(true)), ("puts AFTER", None)] {
            assert_eq!(
                result.invocation_may_be_reached(&CommandAllocationSite {
                    source: Arc::clone(&result.source),
                    offset: u32::try_from(source.find(command).unwrap()).unwrap(),
                }),
                expected,
                "{command}"
            );
        }
        let source = "proc p {} {set anchor 1; operation; run notacommand}";
        let result = report(source);
        assert_eq!(
            result.invocation_may_be_reached(&CommandAllocationSite {
                source: Arc::clone(&result.source),
                offset: u32::try_from(source.find("run notacommand").unwrap()).unwrap(),
            }),
            None
        );
    }
}

#[cfg(test)]
mod declared_procedure_source_read_occurrence_tests {
    #[test]
    fn logical_source_occurrences_do_not_lend_generic_flow_or_current_tree_facts() {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .unwrap()
            .default_context_registry();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let registry = input.borrowed_context_registry().commands();
        for (source, expected) in [
            ("proc f {p} {set anchor 1; puts $missing}", true),
            ("proc f {p} {unknown_host; puts $missing}", false),
        ] {
            let mut lowerer = crate::lowering::Lowerer::with_config(registry, input.lexer_config())
                .with_resolved_analysis_input(input.clone());
            lowerer.lower(source);
            let module = lowerer.finish_module(source);
            let procedure = module
                .procedures
                .values()
                .next()
                .expect("genuine Logical declaration");
            let tokens = procedure.body.statements[0].tokens().unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            let report = binding.declaration_flow_report(registry).unwrap();
            assert!(!report.owns_original_procedure(procedure, &module.source));
            let reads = binding
                .original_declared_procedure_read_occurrences(
                    tokens,
                    &input,
                    procedure,
                    &module.source,
                    registry,
                )
                .expect("genuine original-source occurrence purpose");
            assert_eq!(
                reads.occurrences().any(|(name, _)| name == "missing"),
                expected
            );
            let mut changed_tree = procedure.clone();
            changed_tree.body.statements.clear();
            assert!(!report.owns_original_procedure(&changed_tree, &module.source));
            assert!(
                binding
                    .original_declared_procedure_read_occurrences(
                        tokens,
                        &input,
                        &changed_tree,
                        &module.source,
                        registry,
                    )
                    .is_some(),
                "original source occurrences do not claim current tree custody"
            );
            let mut foreign = procedure.clone();
            foreign.declaration_site = None;
            assert!(
                binding
                    .original_declared_procedure_read_occurrences(
                        tokens,
                        &input,
                        &foreign,
                        &module.source,
                        registry,
                    )
                    .is_none()
            );
            assert!(
                binding
                    .original_declared_procedure_read_occurrences(
                        tokens,
                        &input,
                        procedure,
                        &tcl_lexer::SourceImage::document(&format!("# foreign\n{source}")),
                        registry,
                    )
                    .is_none()
            );
            let other = tcl_registry::model::ingress::resolve_known_environment("tcl9.0")
                .unwrap()
                .default_context_registry();
            assert!(
                binding
                    .original_declared_procedure_read_occurrences(
                        tokens,
                        &input,
                        procedure,
                        &module.source,
                        other.commands(),
                    )
                    .is_none()
            );
        }
    }
}

#[cfg(test)]
mod conditional_declared_read_point_tests {
    use super::{Builder, DeclarationFlowFrame, DeclarationFlowInventory};
    use crate::command_binding::{
        CommandAllocationSite, SourceAnalysisOptions, SourceCommandBindings,
    };
    use crate::ir::CommandTokens;
    use std::collections::BTreeSet;
    use std::sync::Arc;

    struct Fixture {
        input: crate::analyser::ResolvedAnalysisInput,
        inventory: Arc<DeclarationFlowInventory>,
        binding: crate::command_binding::SourceInvocationBinding,
        frame: DeclarationFlowFrame,
        config: tcl_lexer::LexerConfig,
        dialect: tcl_registry::InvocationDialect,
        tokens: CommandTokens,
        site: CommandAllocationSite,
    }

    impl Fixture {
        fn new(source: &str, anchor: &str, tail: &str) -> Self {
            let profile = tcl_dialect::DialectProfile::plain_tcl();
            let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
                    .unwrap()
                    .default_context_registry(),
                config,
            );
            let registry = input.borrowed_context_registry().commands();
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                config,
                registry,
                SourceAnalysisOptions::for_logical_source(&input).unwrap(),
            );
            let image = tcl_lexer::SourceImage::document(source);
            let offset = u32::try_from(source.find(anchor).unwrap()).unwrap();
            let segment = crate::segmenter::segment_commands_with_offset_and_config(
                &source[offset as usize..source.len() - 1],
                offset,
                config,
            )
            .remove(0);
            let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
            bindings.stamp_original_tokens(&mut tokens);
            let binding = tokens.source_binding.as_ref().unwrap();
            let site = binding.invocation_site().unwrap();
            let inventory = Arc::clone(binding.declaration_flow_inventory.as_ref().unwrap());
            let observation =
                crate::command_binding::declaration_layout::original_declaration_layouts(
                    inventory.layouts.get(site).unwrap(),
                )
                .unwrap()
                .next()
                .unwrap();
            let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
            assert!(matches!(&frame, DeclarationFlowFrame::Existing(
                crate::command_binding::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredProcedure(_))));
            let dialect = observation
                .snapshot
                .state
                .source_variables
                .invocation_dialect
                .unwrap();
            let owner_binding = binding.clone();
            let offset = u32::try_from(source.find(tail).unwrap()).unwrap();
            let site = CommandAllocationSite {
                source: Arc::clone(&site.source),
                offset,
            };
            let segment = crate::segmenter::segment_commands_with_offset_and_config(
                &source[offset as usize..source.len() - 1],
                offset,
                config,
            )
            .remove(0);
            let tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
            Self {
                input,
                inventory,
                binding: owner_binding,
                frame,
                config,
                dialect,
                tokens,
                site,
            }
        }

        fn select(
            &self,
            inventory: &DeclarationFlowInventory,
        ) -> Option<crate::registry_invocation::DeclarationInvocationFlow> {
            Builder {
                inventory,
                registry: self.input.borrowed_context_registry().commands(),
                config: self.config,
                dialect: self.dialect,
                entry: &self.frame,
                nodes: Vec::new(),
                jobs: Vec::new(),
                conditional_overwrites: Vec::new(),
                read_sites: BTreeSet::new(),
            }
            .source_read_selection(&self.tokens, &self.site)
        }

        fn with_layouts_and_points(
            &self,
            layouts: crate::command_binding::declaration_layout::DeclarationLayouts,
            points: Vec<crate::command_binding::SourceBindingPoint>,
            origin_points: std::collections::BTreeMap<
                Arc<crate::command_binding::SourceOriginId>,
                Vec<crate::command_binding::SourceBindingPoint>,
            >,
        ) -> DeclarationFlowInventory {
            let mut inventory = DeclarationFlowInventory::new(
                Arc::new(layouts),
                Arc::clone(&self.inventory.accesses),
                Arc::clone(&self.inventory.origin_accesses),
                Arc::new(points),
                Arc::new(origin_points),
            );
            inventory.source_seed = self.inventory.source_seed.clone();
            inventory
        }
    }

    #[test]
    fn logical_missing_layout_keeps_conditional_reads_without_faking_an_observation() {
        let fixture = Fixture::new(
            "proc p {x} {switch -glob $x {a* {return 1} default {set z 9}}; puts $missing}",
            "switch",
            "puts",
        );
        assert!(!fixture.inventory.layouts.contains_key(&fixture.site));
        let selected = fixture.select(&fixture.inventory).unwrap();
        assert_eq!(
            selected.effects.schema_origin,
            crate::registry_invocation::DeclarationSchemaOrigin::OriginalSourceCatalogue
        );
        let mut layouts = fixture.inventory.layouts.as_ref().clone();
        layouts.insert(fixture.site.clone(), Vec::new());
        let corrupt = fixture.with_layouts_and_points(
            layouts,
            fixture.inventory.points.as_ref().clone(),
            fixture.inventory.origin_points.as_ref().clone(),
        );
        assert!(corrupt.layouts.contains_key(&fixture.site));
        assert!(
            fixture.select(&corrupt).is_none(),
            "present invalid layout remains terminal"
        );
    }

    #[test]
    fn logical_missing_layout_does_not_override_corrupt_reached_point_metadata() {
        let fixture = Fixture::new(
            "proc p {} {set anchor 1; puts $missing}",
            "set anchor",
            "puts",
        );
        let mut layouts = fixture.inventory.layouts.as_ref().clone();
        assert!(layouts.remove(&fixture.site).is_some());
        let genuine = fixture.with_layouts_and_points(
            layouts.clone(),
            fixture.inventory.points.as_ref().clone(),
            fixture.inventory.origin_points.as_ref().clone(),
        );
        assert!(fixture.select(&genuine).is_some());
        let frame = match &fixture.frame {
            DeclarationFlowFrame::Existing(entry) => entry.frame(),
            _ => unreachable!(),
        };
        let mut points = fixture.inventory.points.as_ref().clone();
        let mut origin_points = fixture.inventory.origin_points.as_ref().clone();
        assert!(
            points
                .iter()
                .chain(origin_points.values().flatten())
                .any(|point| point.dispatch
                    && point.offset == fixture.site.offset
                    && point.state.variable_frame == *frame
                    && point.state.current_source_origin.as_ref() == Some(&fixture.site.source))
        );
        assert!(
            points
                .iter()
                .chain(origin_points.values().flatten())
                .any(|point| {
                    point.dispatch
                        && point.offset == fixture.site.offset
                        && point.state.variable_frame == *frame
                        && point.state.current_source_origin.as_ref() == Some(&fixture.site.source)
                        && (point.original_written_words.is_some()
                            || point.compiled_execution.original_words.is_some())
                }),
            "the negative actually withdraws a retained original vector"
        );
        for point in points
            .iter_mut()
            .chain(origin_points.values_mut().flatten())
        {
            point.original_written_words = None;
            point.compiled_execution.original_words = None;
        }
        let corrupt = fixture.with_layouts_and_points(layouts, points, origin_points);
        assert!(
            fixture.select(&corrupt).is_none(),
            "reached point refusal remains terminal"
        );
    }
    #[test]
    fn conditional_read_scope_keeps_unknown_prefix_and_wrong_point_refusals() {
        let source = "proc p {} {set anchor 1; unknown_command; puts $missing}";
        let mut fixture = Fixture::new(source, "set anchor", "puts");
        assert!(!fixture.inventory.layouts.contains_key(&fixture.site));
        let report = fixture
            .binding
            .declaration_flow_report(fixture.input.borrowed_context_registry().commands())
            .unwrap();
        assert!(report.unavailable);
        let span = report.potential_declared_reads()["missing"];
        assert!(
            !report
                .declared_absence_warning_occurrences()
                .contains_key("missing")
        );
        assert!(!report.allows_declared_absence_warning("missing", span));
        let original = fixture.site.clone();
        fixture.site.offset += 1;
        assert_ne!(fixture.site, original);
        assert!(fixture.select(&fixture.inventory).is_none());
    }
}
