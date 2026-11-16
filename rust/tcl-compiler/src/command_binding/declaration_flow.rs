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
use super::source_read_scope::{trace_rbs_refusal, trace_rbs_required};
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
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
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

    #[cfg(test)]
    pub(crate) fn unread_gate_summary(&self, site: &CommandAllocationSite, name: &str) -> String {
        format!(
            "unknown_paths={} unknown_reads={} entry={:?} store={} read={} alias={}",
            self.unavailable,
            self.unknown_reads,
            self.invocation_may_be_reached(site),
            self.stores.contains(&(site.offset, name.to_owned())),
            self.all_reads.contains(name),
            self.aliases.contains(name)
        )
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
impl State {
    /// Whether `name` is defined, or is an array an element store made
    /// exist: an authored read of `$c(x)` names the array `c`, and
    /// `set c(x) 1` defines `c(x)`.
    fn defines_array_or_name(&self, name: &str) -> bool {
        self.defined.contains(name)
            || self.defined.iter().any(|defined| {
                tcl_syntax::naming::split_element_ref(defined)
                    .is_some_and(|(array, _)| array == name)
            })
    }

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

impl DeclarationReadPointScope<'_> {
    fn matches_frame(&self, frame: &crate::var_resolve::VariableExecutionFrame) -> bool {
        match self {
            Self::Observed(scopes) => scopes
                .iter()
                .any(|scope| scope.snapshot.state.variable_frame == *frame),
            Self::ConditionalDeclared(body) => body.frame == *frame,
            Self::ConditionalRoot(entry) => entry.frame() == frame,
            Self::ConditionalRootRelated { root, .. } => root.frame() == frame,
        }
    }

    fn accepts_source_scope(
        &self,
        scope: &super::source_read_scope::SourceReadScope,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        match self {
            Self::ConditionalDeclared(body) => body.matches_logical_read_scope(scope, config),
            Self::Observed(_) | Self::ConditionalRoot(_) | Self::ConditionalRootRelated { .. } => {
                true
            }
        }
    }
}

// Source graph branch only: the common Tcl Boolean constants need no selected
// Native parser. The complete source receipt supplies no Native preparation,
// error, result representation, frame, or executable rewriting permission.
fn logical_boolean_literal_condition(
    text: &str,
    dialect: tcl_registry::InvocationDialect,
    config: tcl_lexer::LexerConfig,
) -> Option<tcl_syntax::expr::ast::ExprNode> {
    use tcl_syntax::expr::ast::ExprNode;
    if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
        || dialect.tcl_version.is_some()
        || dialect.core_point.is_some()
        || dialect.authored_name_policy()?.authority()
            != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    {
        return None;
    }
    let literal = text.trim_matches(|character: char| character.is_ascii_whitespace());
    if !matches!(literal, "0" | "1") {
        return None;
    }
    let context = dialect.expression_parse_context(None);
    let original =
        tcl_syntax::expr::substitution::OriginalSourceExpression::capture(text, &context, config)?;
    let start = text.len().checked_sub(
        text.trim_start_matches(|c: char| c.is_ascii_whitespace())
            .len(),
    )?;
    let start = u32::try_from(start).ok()?;
    let expression = ExprNode::Literal {
        text: literal.to_owned(),
        start,
        end: start,
    };
    original.matches_tree(&expression).then_some(expression)
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
impl Builder<'_> {
    /// Naming geometry at this exact original root command. Later commands
    /// retain their own source and namespace applicability.
    fn representation_root_naming_scope(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        conditional_flow: Option<&crate::registry_invocation::DeclarationInvocationFlow>,
    ) -> Option<(
        crate::signature_scan::scope::SignatureNamespaceScope,
        tcl_syntax::naming::NamePolicyProtocol,
    )> {
        use super::declaration_layout::OriginalDiagnosticFrameEntry;
        let DeclarationFlowFrame::Existing(
            entry @ OriginalDiagnosticFrameEntry::RootScript {
                source,
                frame,
                namespace,
            },
        ) = self.entry
        else {
            return None;
        };
        let seed = self.inventory.source_seed.as_ref()?;
        let owner = seed.analysis_owner.as_ref()?;
        let input = seed.baseline.metadata_context.source_analysis_input()?;
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
            self.registry,
            input,
            self.config,
            Some(input.unit_profile()),
        )?;
        if *frame != crate::var_resolve::VariableExecutionFrame::Global
            || source.mapping != (ExecutedScriptMapping::Contiguous { base: 0 })
            || source.text != *source.origin.source_image()
            || !Arc::ptr_eq(&source.origin, &seed.origin)
            || !Arc::ptr_eq(&source.origin, &owner.source)
            || !Arc::ptr_eq(&site.source, &source.origin)
            || seed.config != self.config
            || owner.config != self.config
            || input.lexer_config() != self.config
            || owner.metadata != seed.baseline.metadata_context
            || owner.metadata.source_analysis_input() != Some(input)
            || seed.logical_input.is_some()
            || !seed.matches_registry(self.registry)
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        let related = self.representation_naming_is_related(tokens, site, conditional_flow)?;
        let rows = super::declaration_layout::original_declaration_layouts(
            self.inventory.layouts.get(site)?,
        )?;
        let mut selected = None;
        for row in rows {
            let current_entry = if related {
                matches!(
                    row.entry.as_ref(),
                    OriginalDiagnosticFrameEntry::ScriptActivation { .. }
                )
            } else {
                row.entry.as_ref() == entry
            };
            if !current_entry
                || !Arc::ptr_eq(&row.entry.source().origin, &source.origin)
                || row.config != self.config
                || row.namespace != *namespace
                || row.snapshot.state.variable_frame != *frame
                || row.words.as_ref() != tokens.words()
                || !row
                    .snapshot
                    .state
                    .current_source_origin
                    .as_ref()
                    .is_some_and(|origin| Arc::ptr_eq(origin, &source.origin))
                || row
                    .snapshot
                    .state
                    .retained_original_source_purpose(metadata, self.registry)
                    != Some(super::OriginalInvocationSourcePurpose::AuthoredSimulation)
                || crate::registry_invocation::original_native_compiler_words(
                    source.origin.source_image(),
                    &row.words,
                    site.offset,
                    self.config,
                )
                .is_none()
            {
                return None;
            }
            let policy = row
                .snapshot
                .state
                .baseline
                .execution_name_policy?
                .native_recipe()?;
            if policy.authority() != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation {
                return None;
            }
            let scope = row
                .snapshot
                .state
                .original_namespace_geometry(namespace, policy)?;
            let current = (scope, policy);
            if selected.as_ref().is_some_and(|prior| prior != &current) {
                return None;
            }
            selected = Some(current);
        }
        selected
    }

    fn representation_naming_is_related(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        conditional_flow: Option<&crate::registry_invocation::DeclarationInvocationFlow>,
    ) -> Option<bool> {
        Some(match self.source_read_point_scope(tokens, site)? {
            DeclarationReadPointScope::Observed(_) => false,
            DeclarationReadPointScope::ConditionalRootRelated { .. } => {
                let flow = conditional_flow?;
                if self.source_read_selection(tokens, site).as_ref() != Some(flow) {
                    return None;
                }
                true
            }
            _ => return None,
        })
    }

    /// Revalidate a conditional body against its actual immediate source
    /// parent. The returned context is a premise, not the child's snapshot.
    fn conditional_representation_body_scope(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
    ) -> Option<Arc<ConditionalRepresentationBodyContext>> {
        let held = self.current_representation_body.as_ref()?;
        let seed = self.inventory.source_seed.as_ref()?;
        let owner = seed.analysis_owner.as_ref()?;
        let input = seed.baseline.metadata_context.source_analysis_input()?;
        if !selected.representation_no_known_counterevidence
            || !Arc::ptr_eq(owner, &held.owner)
            || !Arc::ptr_eq(&site.source, &held.site.source)
            || !Arc::ptr_eq(&site.source, &seed.origin)
            || input != &held.input
            || input.lexer_config() != self.config
            || !seed.matches_registry(self.registry)
            || !held.binder.direct_phase_preserves_lookup()
            || held.count.iterations() < 2
            || !Self::conditional_substitution_matches(held, tokens, site, self.config)
        {
            return None;
        }
        let DeclarationReadPointScope::ConditionalRootRelated { root, .. } =
            self.source_read_point_scope(tokens, site)?
        else {
            return None;
        };
        if !Arc::ptr_eq(&root.source().origin, &held.site.source)
            || self.source_read_selection(tokens, site).as_ref() != Some(selected)
        {
            return None;
        }
        let rows = super::declaration_layout::original_declaration_layouts(
            self.inventory.layouts.get(site)?,
        )?;
        if rows.into_iter().any(|row| {
            row.snapshot
                .state
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                != Some(held.policy)
        }) {
            return None;
        }
        let catalogue = self
            .inventory
            .source_read_catalogue(self.registry, self.purpose)?;
        catalogue.scope_with_ancestry(site, tokens, self.config, |advice, context| {
            self.conditional_representation_immediate_parent(held, advice, context)
        })?;
        Some(Arc::clone(held))
    }

    fn conditional_substitution_matches(
        held: &ConditionalRepresentationBodyContext,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        let Some(substitution) = &held.substitution else {
            return true;
        };
        let original = &substitution.original;
        let mut expected_parent = held.clone();
        expected_parent.substitution = None;
        substitution.parent.as_ref() == &expected_parent
            && Arc::ptr_eq(&substitution.parent.site.source, &held.site.source)
            && Arc::ptr_eq(&substitution.parent_site.source, &site.source)
            && substitution.parent.substitution.is_none()
            && Self::conditional_substitution_parent_matches(
                &substitution.parent,
                &substitution.parent_site,
                &substitution.parent_words,
                config,
            )
            && tokens.synthetic.is_none()
            && tokens.original_words_match_argv_text()
            && substitution
                .parent_words
                .head_source()
                .and_then(|head| head.word())
                .is_some_and(|head| {
                    head.span().start() == substitution.parent_site.offset
                        && head.image() == site.source.source_image()
                        && head.config() == config
                })
            && substitution
                .parent_words
                .original_argument_word(substitution.operand)
                == Some(original.original_operand())
            && original.command().span.start() == site.offset
            && crate::registry_invocation::original_native_compiler_words(
                site.source.source_image(),
                tokens.words(),
                site.offset,
                config,
            )
            .as_deref()
                == Some(original.command().words.as_slice())
    }

    fn conditional_substitution_parent_matches(
        held: &ConditionalRepresentationBodyContext,
        site: &CommandAllocationSite,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        use crate::registry_invocation::source_structure::OriginalRegistrySource;
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            return false;
        };
        let original = advice.original_words();
        Arc::ptr_eq(&held.owner.source, &site.source)
            && Arc::ptr_eq(&advice.site().source, &site.source)
            && advice.site().offset == site.offset
            && advice.logical_source_input().is_none()
            && advice.matches_source(site.source.source_image(), config)
            && advice.matches_context(held.input.borrowed_context_registry())
            && words.matches_source(site.source.source_image(), config)
            && crate::registry_invocation::source_cut_declaration_origins(words)
            && original.len() == words.arguments().len() + 1
            && original.first() == words.head_source().and_then(|head| head.word())
            && original
                .iter()
                .skip(1)
                .enumerate()
                .all(|(index, word)| words.original_argument_word(index) == Some(word))
    }

    fn conditional_representation_immediate_parent(
        &self,
        held: &ConditionalRepresentationBodyContext,
        advice: &super::OriginalSourceCommandTransitionAdvice,
        context: &tcl_registry::model::ContextRegistry,
    ) -> bool {
        use super::source_transition_advice::OriginalSourceCatalogueBody;
        use crate::registry_invocation::source_structure::OriginalRegistrySource;
        let check = || -> Option<()> {
            if !std::ptr::eq(held.input.borrowed_context_registry(), context) {
                return None;
            }
            let OriginalSourceCatalogueBody::Script(body) =
                advice.original_catalogue_source_body()?
            else {
                return None;
            };
            if body.content_span() != held.body.content_span()
                || body.original_container() != held.body.original_container()
                || !body.matches_source(held.site.source.source_image(), self.config)
                || !body.inherits_immediate_caller_source_context(context)
                || !body
                    .source_words()
                    .source_script_bodies(context)
                    .contains(body.as_ref())
            {
                return None;
            }
            let parent_words = body.source_words();
            let OriginalRegistrySource::SourceTransitions(parent) = parent_words.source() else {
                return None;
            };
            if !Arc::ptr_eq(&parent.site().source, &held.site.source)
                || parent.site().offset != held.site.offset
                || !parent.matches_context(context)
                || !parent.matches_source(held.site.source.source_image(), self.config)
                || parent_words
                    .original_source_head_correspondence(&held.words, context)
                    .is_none()
            {
                return None;
            }
            let original = parent.original_words();
            if original.len() != held.words.arguments().len() + 1
                || original.first() != held.words.head_source().and_then(|head| head.word())
                || original
                    .iter()
                    .skip(1)
                    .enumerate()
                    .any(|(index, word)| held.words.original_argument_word(index) != Some(word))
                || parent_words.origins().get(1..) != held.words.origins().get(1..)
            {
                return None;
            }
            Some(())
        };
        check().is_some()
    }

    fn representation_plan(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
    ) -> Option<ConditionalRepresentationPlan> {
        use tcl_registry::value_transfer::{AnalysisContext, InvocationLayout};
        if !Self::representation_is_applicable(tokens, selected) {
            return None;
        }
        let naming = self.representation_plan_naming(tokens, site, selected)?;
        let words = selected.representation_words.as_ref()?;
        if !Self::representation_words_match_tokens(tokens, words) {
            return None;
        }
        let ConditionalRepresentationNaming {
            namespace,
            policy,
            body_context,
        } = naming;
        if !namespace.is_root() {
            return None;
        }
        let seed = self.inventory.source_seed.as_ref()?;
        let input = seed.baseline.metadata_context.source_analysis_input()?;
        let context = seed.source_context(self.registry)?;
        let source = Arc::new(words.clone());
        let captures = Self::representation_captures(tokens, words, &namespace, policy)?;
        let argument_words = Self::representation_schema_words(words);
        let texts = Self::representation_argument_texts(tokens);
        let (relation, prior_target, target, coercion, list_append, list_index) = words
            .with_source_schema(&context, |schema| {
                let view = crate::value_transfer::view_of(
                    schema,
                    &texts,
                    &argument_words,
                    InvocationLayout::Source,
                );
                let analysis = AnalysisContext {
                    script_encoding: input.script_encoding(),
                    ..AnalysisContext::detached(Some(input.unit_profile()))
                };
                let structure = crate::value_transfer::StructureInputs::new(view, &analysis);
                let relation =
                    schema.semantics.value.semantics().and_then(|semantics| {
                        semantics.conditional_source_representation(&structure)
                    });
                let list_append = schema.conditional_source_nonempty_list_append(&structure);
                if relation.is_some() && list_append.is_some() {
                    return None;
                }
                if (selected.effects.unknown_reads || !selected.effects.lookup_stable)
                    && !Self::representation_append_direct_phase(
                        schema,
                        selected,
                        words,
                        list_append,
                    )
                {
                    return None;
                }
                let prior_target = Self::conditional_prior_target(
                    schema.semantics.value.semantics(),
                    &structure,
                    relation,
                    words,
                );
                let target = Self::representation_schema_target(
                    schema,
                    words,
                    &namespace,
                    policy,
                    relation,
                    list_append,
                )
                .ok()?;
                let (coercion, list_index) = Self::representation_source_index(schema, &structure);
                Some((
                    relation,
                    prior_target,
                    target,
                    coercion,
                    list_append,
                    list_index,
                ))
            })??;
        (relation.is_some() || coercion.is_some() || list_append.is_some()).then_some(
            ConditionalRepresentationPlan {
                source,
                body_context,
                captures,
                relation,
                prior_target,
                target,
                coercion,
                list_append,
                list_index,
            },
        )
    }

    fn representation_schema_words(
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> Vec<tcl_registry::InvocationWord<'_>> {
        words
            .arguments()
            .iter()
            .map(|word| {
                crate::registry_invocation::source_structure::source_schema_word(
                    word.as_registry_word(),
                )
            })
            .collect()
    }

    fn representation_argument_texts(tokens: &CommandTokens) -> Vec<&str> {
        tokens
            .argv_texts
            .iter()
            .skip(1)
            .map(String::as_str)
            .collect()
    }

    fn representation_schema_target(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        relation: Option<tcl_registry::value_transfer::ConditionalSourceRepresentation>,
        append: Option<tcl_registry::value_transfer::ConditionalSourceListAppend>,
    ) -> Result<Option<ConditionalRepresentationTarget>, ()> {
        let target = Self::conditional_representation_target(relation).or_else(|| {
            append.map(tcl_registry::value_transfer::ConditionalSourceListAppend::target)
        });
        match target {
            Some(target) => {
                Self::representation_source_target(schema, words, namespace, policy, target)
                    .map(Some)
                    .ok_or(())
            }
            None => Ok(None),
        }
    }

    fn representation_source_target(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        target: tcl_registry::value_transfer::TargetId,
    ) -> Option<ConditionalRepresentationTarget> {
        use crate::signature_scan::variable_symbol::{
            OriginalVariableSymbolPurpose, OriginalVariableSymbolReceiver,
            SignatureSourceVariableSymbol,
        };
        let name = words.original_argument_value_input(target.0.0)?;
        if name.policy() != policy
            || tcl_syntax::naming::split_element_ref_bytes(name.bytes()).is_some()
        {
            return None;
        }
        let form = schema.authored_source_variable_receiver_operand_form(target.0.0)?;
        let symbol = SignatureSourceVariableSymbol::from_original_input(
            name,
            namespace,
            OriginalVariableSymbolPurpose::NamespaceStorage,
            OriginalVariableSymbolReceiver::Operand(form),
        )?;
        Some((target, symbol))
    }

    fn representation_source_index(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        inputs: &dyn tcl_registry::value_transfer::AnalysisInputs,
    ) -> (
        Option<tcl_registry::representation::ConditionalSourceListIndexCoercion>,
        Option<tcl_registry::value_transfer::ConditionalSourceLiteralListIndex>,
    ) {
        let index = schema.conditional_source_single_literal_list_index(inputs);
        let coercion = schema
            .semantics
            .representation_effect
            .conditional_source_list_index_coercion(
                schema.words.arguments(),
                schema.semantics.argument_offset,
            )
            .filter(|coercion| {
                index
                    .as_ref()
                    .is_some_and(|index| index.container_operand().0 == coercion.argument())
            });
        let index = index.filter(|index| {
            coercion.is_some_and(|coercion| index.container_operand().0 == coercion.argument())
        });
        (coercion, index)
    }

    fn representation_words_match_tokens(
        tokens: &CommandTokens,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> bool {
        words.arguments().len() + 1 == tokens.words().len()
            && tokens.argv_texts.len() == tokens.words().len()
            && crate::registry_invocation::source_cut_declaration_origins(words)
    }

    fn representation_plan_naming(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
    ) -> Option<ConditionalRepresentationNaming> {
        if selected.representation_quiet
            && let Some((namespace, policy)) =
                self.representation_root_naming_scope(tokens, site, Some(selected))
        {
            return Some(ConditionalRepresentationNaming {
                namespace,
                policy,
                body_context: None,
            });
        }
        let context = self.conditional_representation_body_scope(tokens, site, selected)?;
        Some(ConditionalRepresentationNaming {
            namespace: context.namespace.clone(),
            policy: context.policy,
            body_context: Some(context),
        })
    }

    fn conditional_representation_target(
        relation: Option<tcl_registry::value_transfer::ConditionalSourceRepresentation>,
    ) -> Option<tcl_registry::value_transfer::TargetId> {
        use tcl_registry::value_transfer::ConditionalSourceRepresentation;
        match relation {
            Some(ConditionalSourceRepresentation::IntegerResult { target }) => target,
            Some(ConditionalSourceRepresentation::ForwardValue { target, .. }) => Some(target),
            None => None,
        }
    }

    fn conditional_prior_target(
        semantics: Option<&dyn tcl_registry::value_transfer::CommandSemantics>,
        inputs: &dyn tcl_registry::value_transfer::AnalysisInputs,
        relation: Option<tcl_registry::value_transfer::ConditionalSourceRepresentation>,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> Option<representation::PriorTargetUpdate> {
        use tcl_registry::value_transfer::{ConditionalSourceRepresentation, PlanAnswer};
        let semantics = semantics?;
        let update = semantics.conditional_source_target_update(inputs)?;
        let PlanAnswer::CellReadModifyWrite { target, .. } = semantics.structure(inputs) else {
            return None;
        };
        if update.target() != target
            || relation
                != Some(ConditionalSourceRepresentation::IntegerResult {
                    target: Some(target),
                })
        {
            return None;
        }
        Some(representation::PriorTargetUpdate {
            relation: update,
            read_span: words.original_argument_word(target.0.0)?.span(),
        })
    }

    fn representation_loop_plan(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
    ) -> Option<ConditionalRepresentationLoopPlan> {
        use crate::signature_scan::variable_symbol::{
            OriginalVariableSymbolPurpose, OriginalVariableSymbolReceiver,
            SignatureSourceVariableSymbol,
        };
        let ScriptBodyFlow::Loop {
            initial,
            repeated,
            continued,
            conditions,
        } = &selected.flow
        else {
            return None;
        };
        let words = selected.representation_words.as_ref()?;
        let body = words.arguments().len().checked_sub(1)?;
        if !initial.is_empty()
            || !continued.is_empty()
            || !conditions.is_empty()
            || repeated.as_slice() != [body]
            || words.arguments().len() != 3
            || !matches!(tokens.words().last(), Some(WordExpr::BracedLiteral { .. }))
            || tokens.words().len() != words.arguments().len() + 1
            || tokens.argv_texts.len() != tokens.words().len()
            || !tokens.original_words_match_argv_text()
            || tokens.synthetic.is_some()
            || !crate::registry_invocation::source_cut_declaration_origins(words)
            || !selected.representation_quiet
            || selected.effects.unknown_writes
            || selected.effects.frame_reachable()
            || !selected.writes.is_empty()
            || !selected.reads.is_empty()
            || !selected.name_queries.is_empty()
            || !selected.aliases.is_empty()
            || !selected.caller_frame_aliases.is_empty()
            || !selected.removes.is_empty()
            || !selected.possible_output_writes.is_empty()
        {
            return None;
        }
        let (namespace, policy) =
            self.representation_root_naming_scope(tokens, site, Some(selected))?;
        if !namespace.is_root() {
            return None;
        }
        let context = self
            .inventory
            .source_seed
            .as_ref()?
            .source_context(self.registry)?;
        let (count, binder, binder_receipt) = words.with_source_schema(&context, |schema| {
            let args = schema.words.arguments();
            let facts = schema.facts();
            let count = facts.conditional_source_finite_iteration(args)?;
            if count.iterations() < 2 {
                return None;
            }
            let binder = schema.conditional_source_single_loop_binding()?;
            if !binder.direct_phase_preserves_lookup()
                || !crate::registry_invocation::source_advice_has_no_declared_lookup_mutation(
                    &facts,
                )
            {
                return None;
            }
            let input = words.original_argument_value_input(binder.operand())?;
            if input.policy() != policy
                || input.bytes() != binder.name()
                || tcl_syntax::naming::split_element_ref_bytes(input.bytes()).is_some()
                || selected.loop_bindings.len() != 1
                || selected.loop_bindings[0].as_bytes() != binder.name()
            {
                return None;
            }
            let symbol = SignatureSourceVariableSymbol::from_original_input(
                input,
                &namespace,
                OriginalVariableSymbolPurpose::NamespaceStorage,
                OriginalVariableSymbolReceiver::Operand(binder.receiver_form()),
            )?;
            Some((count, symbol, binder))
        })??;
        let source = Arc::new(words.clone());
        let body_context =
            self.representation_loop_body_context(ConditionalRepresentationBodyInputs {
                site,
                words: &source,
                body_argument: body,
                context: &context,
                namespace,
                policy,
                binder: binder_receipt,
                count,
            });
        Some(ConditionalRepresentationLoopPlan {
            source,
            body_context,
            count,
            binder,
            span: tokens.words().first()?.source().span,
        })
    }

    fn representation_loop_body_context(
        &self,
        request: ConditionalRepresentationBodyInputs<'_>,
    ) -> Option<Arc<ConditionalRepresentationBodyContext>> {
        let ConditionalRepresentationBodyInputs {
            site,
            words,
            body_argument,
            context,
            namespace,
            policy,
            binder,
            count,
        } = request;
        let seed = self.inventory.source_seed.as_ref()?;
        let owner = seed.analysis_owner.as_ref()?;
        let input = seed.baseline.metadata_context.source_analysis_input()?;
        if !Arc::ptr_eq(&site.source, &owner.source)
            || !Arc::ptr_eq(&site.source, &seed.origin)
            || input.lexer_config() != self.config
            || !std::ptr::eq(input.borrowed_context_registry(), context)
            || !seed.matches_registry(self.registry)
        {
            return None;
        }
        let container = words.original_argument_word(body_argument)?;
        let mut bodies = words
            .source_script_bodies_for(
                context,
                crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
            )
            .into_iter()
            .filter(|body| {
                body.original_container() == container
                    && body.matches_source(site.source.source_image(), self.config)
                    && body.inherits_immediate_caller_source_context(context)
            });
        let body = bodies.next()?;
        if bodies.next().is_some() {
            return None;
        }
        Some(Arc::new(ConditionalRepresentationBodyContext {
            owner: Arc::clone(owner),
            input: input.clone(),
            site: site.clone(),
            words: Arc::clone(words),
            body,
            namespace,
            policy,
            binder,
            count,
            substitution: None,
        }))
    }

    fn representation_is_applicable(
        tokens: &CommandTokens,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
    ) -> bool {
        !selected.effects.unknown_writes
            && selected.aliases.is_empty()
            && selected.caller_frame_aliases.is_empty()
            && selected.removes.is_empty()
            && selected.possible_output_writes.is_empty()
            && matches!(selected.flow, ScriptBodyFlow::None)
            && tokens.synthetic.is_none()
            && tokens.original_words_match_argv_text()
    }

    fn representation_append_direct_phase(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        append: Option<tcl_registry::value_transfer::ConditionalSourceListAppend>,
    ) -> bool {
        let Some(append) = append else { return false };
        let phase = append.conditional_direct_phase();
        let values = phase.evaluated_value_operands();
        let target = phase.current_target();
        let Some(name) = words.original_argument_value_input(target.0.0) else {
            return false;
        };
        let Ok(name) = std::str::from_utf8(name.bytes()) else {
            return false;
        };
        let facts = schema.facts();
        phase.conditionally_preserves_lookup()
            && target == append.target()
            && target.0.0 < values.start
            && values.end == words.arguments().len()
            && values.len() == append.appended_operand_count()
            && !selected.effects.frame_reachable()
            && selected.reads.len() == 1
            && selected.reads[0] == name
            && selected.writes.len() == 1
            && selected.writes[0] == name
            && selected.name_queries.is_empty()
            && crate::registry_invocation::source_advice_has_no_declared_lookup_mutation(&facts)
    }

    fn representation_captures(
        tokens: &CommandTokens,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<VecDeque<representation::CaptureSource>> {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        use crate::signature_scan::variable_symbol::{
            OriginalVariableSymbolPurpose, OriginalVariableSymbolReceiver,
            SignatureSourceVariableSymbol,
        };
        let mut captures = VecDeque::with_capacity(words.arguments().len());
        for (index, expression) in tokens.words().iter().skip(1).enumerate() {
            let word = words.original_argument_word(index)?;
            let capture = if expression.sole_variable_substitution().is_some() {
                let reference =
                    tcl_lexer::word_parts::whole_var_ref(word.bytes(), word.config()).ok()??;
                if reference.index.is_some()
                    || reference.name.contains(&0)
                    || tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
                {
                    return None;
                }
                let root = crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_word(
                    word, word.span(),
                    tcl_syntax::word_rules::WordValueRules::from_config(&word.config()), policy,
                )?;
                if root.is_separate_array_root() {
                    return None;
                }
                let name = SignatureSourceNameInput::OriginalVariableRoot(root);
                let symbol = SignatureSourceVariableSymbol::from_original_input(
                    &name,
                    namespace,
                    OriginalVariableSymbolPurpose::NamespaceStorage,
                    OriginalVariableSymbolReceiver::LexicalRoot,
                )?;
                representation::CaptureSource::Variable {
                    symbol,
                    span: word.span(),
                }
            } else if expression.sole_command_substitution().is_some() {
                representation::CaptureSource::LastResult
            } else if matches!(expression, WordExpr::Template { rejected: None, .. }) {
                let bytes = tcl_syntax::word_rules::original_static_word_source_bytes(word)?;
                representation::CaptureSource::Literal { bytes }
            } else if matches!(
                expression,
                WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
            ) {
                tcl_syntax::word_rules::original_static_word_source_bytes(word)
                    .map_or(representation::CaptureSource::Unknown, |bytes| {
                        representation::CaptureSource::Literal { bytes }
                    })
            } else {
                return None;
            };
            captures.push_back(capture);
        }
        Some(captures)
    }

    fn conditional_substitution_context(
        &self,
        plan: &ConditionalRepresentationPlan,
        site: &CommandAllocationSite,
        tokens: &CommandTokens,
        operand: usize,
        source: &ExecutedScriptSource,
    ) -> Option<Arc<ConditionalRepresentationBodyContext>> {
        use crate::registry_invocation::source_structure::original_single_source_command_substitution;
        use tcl_lexer::word_parts::ExecutablePart;
        let parent = plan.body_context.as_ref()?;
        if parent.substitution.is_some()
            || !self
                .current_representation_body
                .as_ref()
                .is_some_and(|held| Arc::ptr_eq(held, parent))
            || !Arc::ptr_eq(&parent.site.source, &site.source)
            || !Arc::ptr_eq(&source.origin, &site.source)
            || !Self::conditional_substitution_parent_matches(
                parent,
                site,
                &plan.source,
                self.config,
            )
            || tokens
                .words()
                .get(operand + 1)?
                .sole_command_substitution()
                .is_none()
        {
            return None;
        }
        let word = plan.source.original_argument_word(operand)?;
        if word.group().expand
            || word.config() != self.config
            || word.image() != site.source.source_image()
            || !plan
                .source
                .matches_source(site.source.source_image(), self.config)
        {
            return None;
        }
        let parent_words = crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            self.config,
        )?;
        if parent_words.first() != plan.source.head_source().and_then(|head| head.word())
            || parent_words.len() != plan.source.arguments().len() + 1
            || parent_words
                .iter()
                .skip(1)
                .enumerate()
                .any(|(index, candidate)| {
                    plan.source.original_argument_word(index) != Some(candidate)
                })
            || parent_words.get(operand + 1) != Some(word)
        {
            return None;
        }
        let original = original_single_source_command_substitution(word)?;
        let arena = word.executable_parts();
        let [part] = arena.list(arena.root()) else {
            return None;
        };
        let ExecutablePart::Command { body } = &part.part else {
            return None;
        };
        if source.mapping != (ExecutedScriptMapping::Contiguous { base: body.start() })
            || u32::try_from(source.text.len()).ok()? != body.end().checked_sub(body.start())?
            || site.source.source_image().bytes().get(body.as_range()) != Some(source.text.bytes())
        {
            return None;
        }
        let mut context = parent.as_ref().clone();
        context.substitution = Some(ConditionalRepresentationSubstitution {
            parent: Arc::clone(parent),
            parent_site: site.clone(),
            parent_words: Arc::clone(&plan.source),
            operand,
            original,
        });
        Some(Arc::new(context))
    }

    fn completion_frame(&self) -> tcl_registry::VariableAliasFrame {
        if self.purpose == DeclarationFlowPurpose::Representation
            && matches!(
                self.entry,
                DeclarationFlowFrame::Existing(
                    super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { .. }
                )
            )
        {
            tcl_registry::VariableAliasFrame::Global
        } else {
            tcl_registry::VariableAliasFrame::Procedure
        }
    }

    fn placeholder(&mut self) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node::default());
        id
    }
    fn unknown(&mut self, id: usize, next: usize) {
        if self.purpose == DeclarationFlowPurpose::Representation {
            self.nodes[id]
                .representation_events
                .push(representation::Event::Barrier);
        }
        self.nodes[id].unknown = true;
        self.nodes[id].availability.unavailable_declared_prefix = true;
        self.nodes[id].successors = vec![next];
    }
    fn script(&mut self, job: &Job) {
        let previous = self.current_representation_body.take();
        self.current_representation_body
            .clone_from(&job.representation_body);
        if let Some(context) = &job.representation_body {
            if let Some(substitution) = &context.substitution {
                self.nodes[job.entry].representation_events.push(
                    representation::Event::RequireBodyContext {
                        context: Arc::clone(&substitution.parent),
                    },
                );
            }
            self.nodes[job.entry]
                .representation_events
                .push(representation::Event::BodyContext {
                    context: Arc::clone(context),
                });
            if context.substitution.is_some() {
                self.nodes[job.entry]
                    .representation_events
                    .push(representation::Event::ClearResult);
            }
        }
        self.original_script(
            &job.source.text,
            job.source.base(),
            &job.source.origin,
            job.purpose,
            job.entry,
            job.exits,
        );
        self.current_representation_body = previous;
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
                let observations = trace_rbs_required(
                    super::declaration_layout::original_declaration_layouts(trace_rbs_required(
                        self.inventory.layouts.get(site),
                        site.offset,
                        "scopes.layouts",
                    )?),
                    site.offset,
                    "scopes.observations",
                )?;
                let mut scopes = Vec::new();
                for observation in observations {
                    if observation.entry.as_ref() != entry
                        || observation.config != self.config
                        || observation.words.as_ref() != tokens.words()
                    {
                        trace_rbs_refusal(site.offset, "scopes.row-current");
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
                    trace_rbs_refusal(site.offset, "scopes.read-current");
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

    fn conditional_root_scope(
        &self,
        catalogue: &super::source_read_scope::SourceReadCatalogue,
        entry: &super::declaration_layout::OriginalDiagnosticFrameEntry,
        site: &CommandAllocationSite,
        tokens: &CommandTokens,
        required_bodies: &[tcl_lexer::Span],
    ) -> Option<super::source_read_scope::SourceReadScope> {
        use super::declaration_layout::OriginalDiagnosticFrameEntry;
        let OriginalDiagnosticFrameEntry::RootScript { source, frame, .. } = entry else {
            return None;
        };
        let seed = self.inventory.source_seed.as_ref()?;
        let owner = seed.analysis_owner.as_ref()?;
        let input = seed.baseline.metadata_context.source_analysis_input()?;
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
            self.registry,
            input,
            self.config,
            Some(input.unit_profile()),
        )?;
        if *frame != crate::var_resolve::VariableExecutionFrame::Global
            || source.mapping != (ExecutedScriptMapping::Contiguous { base: 0 })
            || source.text != *source.origin.source_image()
            || !Arc::ptr_eq(&source.origin, &seed.origin)
            || !Arc::ptr_eq(&source.origin, &owner.source)
            || !Arc::ptr_eq(&source.origin, &site.source)
            || owner.config != self.config
            || seed.config != self.config
            || input.lexer_config() != self.config
            || seed.logical_input.is_some()
            || owner.metadata != seed.baseline.metadata_context
            || owner.metadata.source_analysis_input() != Some(input)
            || !seed.matches_registry(self.registry)
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        catalogue.scope_with_ancestry(site, tokens, self.config, |advice, context| {
            if !std::ptr::eq(Arc::as_ptr(&input.context_registry()), context) {
                return false;
            }
            let seen_bodies = std::cell::RefCell::new(Vec::new());
            let valid = |candidate: &super::OriginalSourceCommandTransitionAdvice| {
                let current = Arc::ptr_eq(&candidate.site().source, &source.origin)
                    && catalogue.admits_original_site(candidate.site())
                    && candidate.matches_context(context)
                    && candidate.matches_source(source.origin.source_image(), self.config)
                    && candidate.logical_source_input().is_none()
                    && candidate
                        .original_words()
                        .iter()
                        .all(|word| !word.group().expand)
                    && candidate
                        .arguments()
                        .iter()
                        .enumerate()
                        .all(|(index, word)| {
                            word.origin
                                == crate::registry_invocation::InvocationWordOrigin::Written(
                                    index + 1,
                                )
                        });
                if current && let Some(body) = candidate.original_catalogue_source_body() {
                    seen_bodies.borrow_mut().push(body.content_span());
                }
                current
            };
            let endpoint = |candidate: &super::OriginalSourceCommandTransitionAdvice| {
                self.conditional_root_endpoint(
                    candidate,
                    entry,
                    metadata,
                    required_bodies,
                    &seen_bodies.borrow(),
                )
            };
            super::source_read_scope::matches_read_body_ancestry_until(
                advice,
                context,
                self.config,
                true,
                valid,
                endpoint,
            )
        })
    }

    fn conditional_root_endpoint(
        &self,
        candidate: &super::OriginalSourceCommandTransitionAdvice,
        entry: &super::declaration_layout::OriginalDiagnosticFrameEntry,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        required_bodies: &[tcl_lexer::Span],
        seen_bodies: &[tcl_lexer::Span],
    ) -> bool {
        use super::declaration_layout::OriginalDiagnosticFrameEntry;
        let OriginalDiagnosticFrameEntry::RootScript {
            source,
            frame,
            namespace,
        } = entry
        else {
            return false;
        };
        if required_bodies
            .iter()
            .any(|body| !seen_bodies.contains(body))
        {
            return false;
        }
        let Some(rows) = self.inventory.layouts.get(candidate.site()) else {
            return false;
        };
        let Some(rows) = super::declaration_layout::original_declaration_layouts(rows) else {
            return false;
        };
        rows.into_iter().all(|row| {
            row.entry.as_ref() == entry
                && Arc::ptr_eq(&row.entry.source().origin, &source.origin)
                && row.namespace == *namespace
                && row.config == self.config
                && row.snapshot.state.variable_frame == *frame
                && row
                    .snapshot
                    .state
                    .retained_original_source_purpose(metadata, self.registry)
                    == Some(super::OriginalInvocationSourcePurpose::AuthoredSimulation)
                && row
                    .snapshot
                    .state
                    .current_source_origin
                    .as_ref()
                    .is_some_and(|origin| Arc::ptr_eq(origin, &source.origin))
                && crate::registry_invocation::original_native_compiler_words(
                    source.origin.source_image(),
                    &row.words,
                    candidate.site().offset,
                    self.config,
                )
                .as_deref()
                    == Some(candidate.original_words())
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
        if self.purpose == DeclarationFlowPurpose::Representation
            && let DeclarationFlowFrame::Existing(
                root @ super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
                    source,
                    frame,
                    namespace,
                },
            ) = self.entry
            && let Some(rows) = self.inventory.layouts.get(site)
        {
            let input = self
                .inventory
                .source_seed
                .as_ref()?
                .baseline
                .metadata_context
                .source_analysis_input()?;
            let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
                self.registry,
                input,
                self.config,
                Some(input.unit_profile()),
            )?;
            let rows = super::declaration_layout::original_declaration_layouts(rows)?;
            let mut bodies = Vec::new();
            for row in rows {
                let super::declaration_layout::OriginalDiagnosticFrameEntry::ScriptActivation {
                    source: child,
                    frame: child_frame,
                    namespace: child_namespace,
                    config,
                } = row.entry.as_ref()
                else {
                    return None;
                };
                let ExecutedScriptMapping::Contiguous { base } = child.mapping else {
                    return None;
                };
                let end = base.checked_add(u32::try_from(child.text.len()).ok()?)?;
                if !Arc::ptr_eq(&child.origin, &source.origin)
                    || !Arc::ptr_eq(&child.origin, &site.source)
                    || child_frame != frame
                    || child_namespace != namespace
                    || *config != self.config
                    || row.config != self.config
                    || row.words.as_ref() != tokens.words()
                    || row.namespace != *namespace
                    || row.snapshot.state.variable_frame != *frame
                    || row
                        .snapshot
                        .state
                        .retained_original_source_purpose(metadata, self.registry)
                        != Some(super::OriginalInvocationSourcePurpose::AuthoredSimulation)
                    || !row
                        .snapshot
                        .state
                        .current_source_origin
                        .as_ref()
                        .is_some_and(|origin| Arc::ptr_eq(origin, &source.origin))
                    || !row.entry.owns_source(&site.source, site.offset)
                    || source
                        .origin
                        .source_image()
                        .bytes()
                        .get(base as usize..end as usize)
                        != Some(child.text.bytes())
                {
                    return None;
                }
                bodies.push(tcl_lexer::Span::new(base, end));
            }
            return Some(DeclarationReadPointScope::ConditionalRootRelated { root, bodies });
        }
        // An original row that is present but invalid cannot borrow source
        // absence. Only an independently retained declaration may proceed.
        if self.inventory.layouts.contains_key(site) {
            return None;
        }
        if self.purpose == DeclarationFlowPurpose::Representation
            && let DeclarationFlowFrame::Existing(
                entry @ super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
                    ..
                },
            ) = self.entry
        {
            return Some(DeclarationReadPointScope::ConditionalRoot(entry));
        }
        let DeclarationFlowFrame::Existing(
            super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredProcedure(body),
        ) = self.entry
        else {
            return None;
        };
        Some(DeclarationReadPointScope::ConditionalDeclared(body))
    }

    fn source_read_body(&self, site: &CommandAllocationSite) -> Option<tcl_lexer::Span> {
        Some(match self.entry {
            DeclarationFlowFrame::Existing(entry) => {
                if !(matches!(
                    entry.frame(),
                    crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                        | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                ) || self.purpose == DeclarationFlowPurpose::Representation
                    && matches!(
                        entry,
                        super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { .. }
                    ))
                {
                    trace_rbs_refusal(site.offset, "selection.frame");
                    return None;
                }
                let source = entry.source();
                if source.origin != site.source || !entry.owns_source(&site.source, site.offset) {
                    trace_rbs_refusal(site.offset, "selection.source-owner");
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
                trace_rbs_required(
                    entry.owns_source(&site.source, site.offset).then_some(()),
                    site.offset,
                    "selection.read-owner",
                )?;
                trace_rbs_required(
                    entry.read.original_body_span(),
                    site.offset,
                    "selection.read-body",
                )?
            }
        })
    }

    fn source_read_selection(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<crate::registry_invocation::DeclarationInvocationFlow> {
        let body = self.source_read_body(site)?;
        // Original layout corruption is terminal even for a source-only query.
        let scopes = trace_rbs_required(
            self.source_read_point_scope(tokens, site),
            site.offset,
            "selection.scopes",
        )?;
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
                    && (self.purpose != DeclarationFlowPurpose::Representation
                        || point
                            .state
                            .current_source_origin
                            .as_ref()
                            .is_some_and(|origin| Arc::ptr_eq(origin, &site.source)))
                    && scopes.matches_frame(&point.state.variable_frame)
            });
        let mut points = points.peekable();
        if points.peek().is_some() {
            retained.source_binding =
                Some(super::SourceCommandBindings::query_dispatch_points(points));
        }
        let catalogue = trace_rbs_required(
            self.inventory
                .source_read_catalogue(self.registry, self.purpose),
            site.offset,
            "selection.catalogue",
        )?;
        let scope = trace_rbs_required(
            match self.entry {
                DeclarationFlowFrame::Existing(
                    entry @ super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
                        ..
                    },
                ) if self.purpose == DeclarationFlowPurpose::Representation => {
                    let bodies = match &scopes {
                        DeclarationReadPointScope::ConditionalRootRelated { bodies, .. } => {
                            bodies.as_slice()
                        }
                        _ => &[],
                    };
                    self.conditional_root_scope(catalogue, entry, site, &retained, bodies)
                }
                _ => catalogue.scope(site, &retained, body, self.config),
            },
            site.offset,
            "selection.scope",
        )?;
        if !scopes.accepts_source_scope(&scope, self.config) {
            return None;
        }
        if scope.advice().dialect() != self.dialect {
            trace_rbs_refusal(site.offset, "selection.dialect");
            return None;
        }
        trace_rbs_required(
            crate::registry_invocation::declaration_source_read_flow(
                self.registry,
                &retained,
                &scope,
                self.completion_frame(),
                self.purpose == DeclarationFlowPurpose::Representation,
            ),
            site.offset,
            "selection.flow",
        )
    }

    fn selection(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<(
        crate::registry_invocation::DeclarationInvocationFlow,
        Option<super::lifecycle_advice::SourceScopedLifecycleAdvice>,
    )> {
        if self.purpose == DeclarationFlowPurpose::Representation {
            let Some(source) = self.source_read_selection(tokens, site) else {
                let fallback = self.representation_observed_selection(tokens, site);
                return fallback.map(|selected| (selected, None));
            };
            if matches!(
                self.source_read_point_scope(tokens, site)?,
                DeclarationReadPointScope::Observed(_)
            ) {
                let (_, lifecycle) =
                    self.original_layout_selection_checked(tokens, site, Some(&source))?;
                if lifecycle.is_some() {
                    return None;
                }
            }
            return Some((source, None));
        }
        self.original_layout_selection(tokens, site).or_else(|| {
            self.source_read_selection(tokens, site)
                .map(|selected| (selected, None))
        })
    }

    /// Reuse the retained module realm for an already observed root command.
    /// The original word-source variant and all of its obligations remain held.
    fn representation_observed_selection(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<crate::registry_invocation::DeclarationInvocationFlow> {
        if !matches!(
            self.entry,
            DeclarationFlowFrame::Existing(
                super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { .. }
            )
        ) || !matches!(
            self.source_read_point_scope(tokens, site)?,
            DeclarationReadPointScope::Observed(_)
        ) {
            return None;
        }
        let (observed, lifecycle) = self.original_layout_selection(tokens, site)?;
        if lifecycle.is_some()
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        let realm = self.representation_realm?;
        let input = self
            .inventory
            .source_seed
            .as_ref()?
            .baseline
            .metadata_context
            .source_analysis_input()?;
        let mut retained = tokens.clone();
        realm.stamp_original_tokens(&mut retained);
        let words = crate::registry_invocation::source_structure::original_registry_words_for_supplied_tokens(
            realm, input, &retained,
        );
        let words = words?;
        let source = crate::registry_invocation::declaration_original_words_flow(
            self.registry,
            &retained,
            words,
            input.borrowed_context_registry(),
            self.completion_frame(),
            true,
        )?;
        let (verified, _) = self.original_layout_selection_checked(tokens, site, Some(&source))?;
        (verified == observed).then_some(source)
    }

    fn original_layout_selection(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<(
        crate::registry_invocation::DeclarationInvocationFlow,
        Option<super::lifecycle_advice::SourceScopedLifecycleAdvice>,
    )> {
        self.original_layout_selection_checked(tokens, site, None)
    }

    fn original_layout_selection_checked(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        source: Option<&crate::registry_invocation::DeclarationInvocationFlow>,
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
            let selected = crate::registry_invocation::declaration_invocation_flow_in_frame(
                self.registry,
                &retained,
                &advice,
                self.completion_frame(),
            )?;
            if let Some(source) = source {
                let input = self
                    .inventory
                    .source_seed
                    .as_ref()?
                    .baseline
                    .metadata_context
                    .source_analysis_input()?;
                crate::registry_invocation::declaration_source_flow_corresponds(
                    self.registry,
                    &retained,
                    &advice,
                    source,
                    &selected,
                    input.borrowed_context_registry(),
                )?;
            }
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
        let loop_plan = if self.purpose == DeclarationFlowPurpose::Representation {
            selected
                .as_ref()
                .and_then(|(selected, _)| self.representation_loop_plan(tokens, site, selected))
        } else {
            None
        };
        let loop_control = loop_plan.as_ref().map(|plan| Arc::clone(&plan.source));
        let mut representation_plan = if self.purpose == DeclarationFlowPurpose::Representation {
            let plan = selected
                .as_ref()
                .and_then(|(selected, _)| self.representation_plan(tokens, site, selected));
            if let Some(context) = plan.as_ref().and_then(|plan| plan.body_context.as_ref()) {
                self.nodes[id].representation_events.push(
                    representation::Event::RequireBodyContext {
                        context: Arc::clone(context),
                    },
                );
            }
            self.nodes[id]
                .representation_events
                .push(representation::Event::ClearResult);
            plan
        } else {
            None
        };
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some() {
            if let Some((flow, _)) = &selected {
                eprintln!(
                    "RBS_GRAPH stage=selection offset={} selected=true unknown_writes={} unknown_reads={} source_catalogue={} loop_flow={} case_flow={} writes={} loop_bindings={}",
                    site.offset,
                    flow.effects.unknown_writes,
                    flow.effects.unknown_reads,
                    matches!(flow.effects.schema_origin, crate::registry_invocation::DeclarationSchemaOrigin::OriginalSourceCatalogue),
                    matches!(&flow.flow, ScriptBodyFlow::Loop { .. }),
                    matches!(&flow.flow, ScriptBodyFlow::CaseBodies(_)),
                    flow.writes.len(),
                    flow.loop_bindings.len(),
                );
            } else {
                eprintln!(
                    "RBS_GRAPH stage=selection offset={} selected=false",
                    site.offset
                );
            }
        }
        let dispatch = self.placeholder();
        let operands =
            self.representation_operand_events(tokens, site, representation_plan.as_mut());
        let next = self.events_before(operands, dispatch, exits);
        self.nodes[id].successors = vec![next];
        let Some((selected, lifecycle)) = selected else {
            self.unknown(dispatch, exits.normal);
            return;
        };
        let after = self.selected_effect_nodes(id, dispatch, site, &selected, exits.normal);
        self.representation_completion(
            representation_plan,
            loop_control,
            tokens,
            site,
            after,
            dispatch,
        );
        let successors =
            self.command_flow(site, &selected, lifecycle.as_ref(), after, exits, loop_plan);
        self.nodes[dispatch].successors = successors;
    }

    fn representation_operand_events(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        mut representation_plan: Option<&mut ConditionalRepresentationPlan>,
    ) -> Vec<Event> {
        let mut operands = Vec::new();
        for (index, word) in tokens.words().iter().enumerate() {
            let first_event = operands.len();
            self.word_events(word, site, &mut operands);
            if let Some(plan) = representation_plan.as_deref_mut()
                && index > 0
            {
                let Some(from) = plan.captures.pop_front() else {
                    operands.push(Event::Representation(Box::new(
                        representation::Event::Barrier,
                    )));
                    continue;
                };
                if matches!(&from, representation::CaptureSource::LastResult) {
                    let context = match &operands[first_event..] {
                        [Event::Script(source)] => self.conditional_substitution_context(
                            plan,
                            site,
                            tokens,
                            index - 1,
                            source,
                        ),
                        _ => None,
                    };
                    if let Some(context) = context {
                        let Event::Script(source) =
                            std::mem::replace(&mut operands[first_event], Event::Unknown)
                        else {
                            unreachable!("checked single script event")
                        };
                        operands[first_event] = Event::ConditionalScript(source, context);
                    }
                }
                if matches!(&from, representation::CaptureSource::Variable { .. }) {
                    for event in &mut operands[first_event..] {
                        if matches!(event, Event::ReadUnavailable) {
                            *event = Event::ConditionalReadUnavailable;
                        }
                    }
                }
                operands.push(Event::Representation(Box::new(
                    representation::Event::Capture {
                        into: representation::OperandSnapshot {
                            site: site.offset,
                            operand: tcl_registry::value_transfer::OperandId(index - 1),
                        },
                        from,
                        source: Arc::clone(&plan.source),
                    },
                )));
            }
        }
        operands
    }

    fn selected_effect_nodes(
        &mut self,
        id: usize,
        dispatch: usize,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        next: usize,
    ) -> usize {
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
        self.nodes[after].successors = vec![next];
        after
    }

    fn representation_completion(
        &mut self,
        representation_plan: Option<ConditionalRepresentationPlan>,
        loop_control: Option<
            Arc<crate::registry_invocation::source_structure::OriginalRegistryWords>,
        >,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        after: usize,
        dispatch: usize,
    ) {
        if self.purpose == DeclarationFlowPurpose::Representation {
            if let (Some(plan), Some(first)) = (representation_plan, tokens.words().first()) {
                let span = first.source().span;
                if let Some(relation) = plan.relation {
                    self.nodes[after]
                        .representation_events
                        .push(representation::Event::Apply {
                            site: site.offset,
                            span,
                            relation,
                            prior_target: plan.prior_target,
                            target: plan.target,
                            source: Arc::clone(&plan.source),
                        });
                } else if let Some(relation) = plan.list_append {
                    if let Some(target) = plan.target {
                        self.nodes[after].representation_events.push(
                            representation::Event::ListAppend {
                                site: site.offset,
                                span,
                                relation,
                                target,
                                source: Arc::clone(&plan.source),
                            },
                        );
                    } else {
                        self.nodes[dispatch]
                            .representation_events
                            .push(representation::Event::Barrier);
                    }
                }
                if let Some(coercion) = plan.coercion {
                    self.nodes[after].representation_events.push(
                        representation::Event::ListIndex {
                            site: site.offset,
                            span,
                            coercion,
                            literal: plan.list_index,
                            source: plan.source,
                        },
                    );
                }
            } else if let Some(source) = loop_control {
                self.nodes[dispatch]
                    .representation_events
                    .push(representation::Event::Control { source });
            } else {
                self.nodes[dispatch]
                    .representation_events
                    .push(representation::Event::Barrier);
            }
        }
    }

    fn command_flow(
        &mut self,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        lifecycle: Option<&super::lifecycle_advice::SourceScopedLifecycleAdvice>,
        after: usize,
        exits: Exits,
        loop_plan: Option<ConditionalRepresentationLoopPlan>,
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
                self.loop_flow(site, selected, flow, after, exits, loop_plan)
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
        loop_plan: Option<ConditionalRepresentationLoopPlan>,
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
        let jobs_before = self.jobs.len();
        let repeated = self.sequence(site, &selected.effective, repeated, body_exits);
        self.attach_representation_body_job(jobs_before, loop_plan.as_ref());
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
        if let Some(plan) = loop_plan {
            let nodes = ConditionalRepresentationLoopNodes {
                first_gate: first_test,
                bindings,
                repeated_entry: repeated,
                header,
            };
            self.retain_representation_loop(plan, site, nodes, after, exits);
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
    fn attach_representation_body_job(
        &mut self,
        jobs_before: usize,
        plan: Option<&ConditionalRepresentationLoopPlan>,
    ) {
        let Some(context) = plan.and_then(|plan| plan.body_context.as_ref()) else {
            return;
        };
        if self.jobs.len() != jobs_before + 1 {
            return;
        }
        let Some(job) = self.jobs.last_mut() else {
            return;
        };
        let span = context.body.content_span();
        let length = u32::try_from(job.source.text.len()).ok();
        if Arc::ptr_eq(&job.source.origin, &context.site.source)
            && job.source.mapping == (ExecutedScriptMapping::Contiguous { base: span.start() })
            && length.and_then(|length| span.start().checked_add(length)) == Some(span.end())
            && context
                .site
                .source
                .source_image()
                .bytes()
                .get(span.as_range())
                == Some(job.source.text.bytes())
        {
            job.representation_body = Some(Arc::clone(context));
        }
    }

    fn retain_representation_loop(
        &mut self,
        plan: ConditionalRepresentationLoopPlan,
        site: &CommandAllocationSite,
        nodes: ConditionalRepresentationLoopNodes,
        after: usize,
        exits: Exits,
    ) {
        self.nodes[nodes.bindings]
            .representation_events
            .push(representation::Event::Reset {
                targets: vec![plan.binder],
                source: Arc::clone(&plan.source),
            });
        self.representation_loops.push(representation::LoopRegion {
            id: representation::LoopId(self.representation_loops.len()),
            site: site.offset,
            span: plan.span,
            source: plan.source,
            count: plan.count,
            first_gate: nodes.first_gate,
            bindings: nodes.bindings,
            repeated_entry: nodes.repeated_entry,
            header: nodes.header,
            normal_exit: after,
            error_exit: exits.error,
            returned_exit: exits.returned,
            other_exit: exits.other,
        });
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
                Event::Script(source) => {
                    if self.purpose == DeclarationFlowPurpose::Representation {
                        self.nodes[id]
                            .representation_events
                            .push(representation::Event::ClearResult);
                    }
                    self.jobs.push(Job {
                        source,
                        representation_body: None,
                        purpose: tcl_lexer::SourceSegmentationPurpose::NestedScript,
                        entry: id,
                        exits: Exits {
                            normal: next,
                            ..exits
                        },
                    });
                }
                Event::ConditionalScript(source, context) => {
                    self.jobs.push(Job {
                        source,
                        representation_body: Some(context),
                        purpose: tcl_lexer::SourceSegmentationPurpose::NestedScript,
                        entry: id,
                        exits: Exits {
                            normal: next,
                            ..exits
                        },
                    });
                }
                Event::Representation(event) => {
                    self.nodes[id].representation_events.push(*event);
                    self.nodes[id].successors = vec![next];
                }
                Event::ReadUnavailable => {
                    self.unknown(id, next);
                    self.nodes[id].declared_read_unavailable = true;
                    self.nodes[id].availability.unavailable_declared_prefix = false;
                }
                Event::ConditionalReadUnavailable => {
                    self.nodes[id].unknown = true;
                    self.nodes[id].successors = vec![next];
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
        let Some(source) = trace_rbs_required(source, site.offset, "condition.source") else {
            return (vec![Event::Unknown], None, None);
        };
        let dialect = self.entry_dialect();
        let Ok(text) = source.try_text() else {
            trace_rbs_refusal(site.offset, "condition.text");
            return (vec![Event::Unknown], None, None);
        };
        let parse_context = dialect.expression_parse_context(None);
        let expression = match parse_expr_checked_with_context(text, &parse_context) {
            CheckedExprParse::Parsed(expression) => Some(expression),
            CheckedExprParse::Unsupported(
                tcl_syntax::expr::parser::ExprParseUnsupported::UnknownNativeSyntax,
            ) => logical_boolean_literal_condition(text, dialect, self.config),
            _ => None,
        };
        let Some(expression) = expression else {
            trace_rbs_refusal(site.offset, "condition.parse");
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
                representation_body: None,
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
            || site.source.source_image().bytes().get(span.as_range()) != Some(bytes)
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
    #[cfg(test)]
    fn conditional_list_sites(&self) -> (BTreeSet<u32>, BTreeSet<u32>) {
        let mut appends = BTreeSet::new();
        let mut indices = BTreeSet::new();
        for event in self
            .nodes
            .iter()
            .flat_map(|node| &node.representation_events)
        {
            match event {
                representation::Event::ListAppend { site, .. } => {
                    appends.insert(*site);
                }
                representation::Event::ListIndex {
                    site,
                    literal: Some(_),
                    ..
                } => {
                    indices.insert(*site);
                }
                _ => {}
            }
        }
        (appends, indices)
    }

    #[cfg(test)]
    fn conditional_substitution_entries(&self) -> BTreeSet<u32> {
        self.nodes
            .iter()
            .flat_map(|node| node.representation_events.windows(3))
            .filter_map(|events| match events {
                [
                    representation::Event::RequireBodyContext { context: required },
                    representation::Event::BodyContext { context: entered },
                    representation::Event::ClearResult,
                ] => {
                    let substitution = entered.substitution.as_ref()?;
                    Arc::ptr_eq(required, &substitution.parent)
                        .then_some(substitution.original.command().span.start())
                }
                _ => None,
            })
            .collect()
    }

    #[cfg(test)]
    fn conditional_body_contexts(&self) -> Vec<Arc<ConditionalRepresentationBodyContext>> {
        self.nodes
            .iter()
            .flat_map(|node| &node.representation_events)
            .filter_map(|event| match event {
                representation::Event::BodyContext { context } => Some(Arc::clone(context)),
                _ => None,
            })
            .collect()
    }

    #[cfg(test)]
    fn conditional_loop_exits(&self) -> Vec<(u32, usize, usize, usize, usize)> {
        self.representation_loops
            .iter()
            .map(|region| {
                (
                    region.site,
                    region.normal_exit,
                    region.error_exit,
                    region.returned_exit,
                    region.other_exit,
                )
            })
            .collect()
    }

    fn evaluate(self, entry: usize) -> DeclarationFlowReport {
        let source = Arc::clone(self.entry.origin());
        #[cfg(test)]
        let conditional_list_events = if self.purpose == DeclarationFlowPurpose::Representation {
            representation::inspect_list_events(&self, entry)
        } else {
            Vec::new()
        };
        #[cfg(test)]
        let (conditional_list_append_sites, conditional_literal_index_sites) =
            self.conditional_list_sites();
        let representation_occurrences = if self.purpose == DeclarationFlowPurpose::Representation {
            representation::project(&self, entry)
        } else {
            Vec::new()
        };
        let mut report = DeclarationFlowReport {
            source,
            representation_occurrences,
            reads: BTreeMap::new(),
            reached: BTreeSet::new(),
            represented: self.nodes.iter().filter_map(|node| node.site).collect(),
            #[cfg(test)]
            conditional_handler_sites: self
                .nodes
                .iter()
                .filter_map(|node| node.handler_site)
                .collect(),
            #[cfg(test)]
            conditional_loop_sites: self
                .representation_loops
                .iter()
                .map(|region| region.site)
                .collect(),
            #[cfg(test)]
            conditional_list_append_sites,
            #[cfg(test)]
            conditional_literal_index_sites,
            #[cfg(test)]
            conditional_list_events,
            #[cfg(test)]
            conditional_substitution_entries: self.conditional_substitution_entries(),
            #[cfg(test)]
            conditional_body_contexts: self.conditional_body_contexts(),
            #[cfg(test)]
            conditional_loop_exits: self.conditional_loop_exits(),
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
        #[cfg(debug_assertions)]
        self.trace_rbs_report(&report, &warning_paths);
        report
    }

    #[cfg(debug_assertions)]
    fn trace_rbs_report(&self, report: &DeclarationFlowReport, warning_paths: &[Option<State>]) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some() {
            eprintln!(
                "RBS_GRAPH stage=report nodes={} warnings={} strict_reads={} unavailable={} unknown_reads={}",
                self.nodes.len(),
                report.warning_reads.len(),
                report.reads.len(),
                report.unavailable,
                report.unknown_reads,
            );
            for (id, node) in self.nodes.iter().enumerate() {
                let reached = warning_paths[id].is_some();
                let (unknown, defined, uncertain, aliases) =
                    warning_paths[id]
                        .as_ref()
                        .map_or((false, 0, 0, 0), |state| {
                            (
                                state.unknown_writers,
                                state.defined.len(),
                                state.uncertain_writes.len(),
                                state.aliases.len(),
                            )
                        });
                eprintln!(
                    "RBS_GRAPH stage=node id={id} site={} site_set={} handler={} handler_set={} unknown={} prefix_unavailable={} missing_read={} reads={} writes={} successors={} reached={reached} state_unknown={unknown} defined={defined} uncertain={uncertain} aliases={aliases}",
                    node.site.unwrap_or(0),
                    node.site.is_some(),
                    node.handler_site.unwrap_or(0),
                    node.handler_site.is_some(),
                    node.unknown,
                    node.availability.unavailable_declared_prefix,
                    node.declared_read_unavailable,
                    node.authored_reads.len(),
                    node.writes.len(),
                    node.successors.len(),
                );
                for successor in &node.successors {
                    eprintln!("RBS_GRAPH stage=edge id={id} next={successor}");
                }
            }
        }
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
            // docs/design/analysis/name-resolution-proofs/original-formal-argument-entry.md
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
                if !state.defines_array_or_name(name)
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
                if !state.defines_array_or_name(name)
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
        // A possible output write (`dict unset d a`, `lappend l x`) leaves a
        // value no earlier literal states: `set d {a 1}; dict unset d a`
        // holds no `a` (#2246).
        for name in node
            .removes
            .iter()
            .chain(&node.writes)
            .chain(&node.possible_output_writes)
            .chain(&node.aliases)
        {
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
            if !tcl_syntax::naming::is_qualified(name.as_bytes())
                && !name.contains('(')
                && !state.aliases.contains(name)
            {
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
    ConditionalScript(
        ExecutedScriptSource,
        Arc<ConditionalRepresentationBodyContext>,
    ),
    Representation(Box<representation::Event>),
    ConditionalReadUnavailable,
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
mod tests;

#[cfg(test)]
mod declared_procedure_source_read_occurrence_tests;

#[cfg(test)]
mod conditional_declared_read_point_tests;

#[cfg(test)]
mod logical_boolean_literal_condition_tests;
