// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-only child ingress from one authentic current-interpreter load point.

use super::{
    CommandAllocationSite, OriginalSourceAnalysisOwner, SourceCommandBindings,
    SourceLookupSnapshot, SourceNamespaceKey,
};
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use tcl_lexer::NativeWord;

/// Opaque source-point owner. Its host must independently retain the current
/// parent URI/path/publication. This receipt grants no loading or Native entry.
#[derive(Debug, Clone)]
pub struct OriginalSourcePointEntry {
    payload: Arc<SourcePointPayload>,
    key: u64,
}

/// Unresolved applicability of a source-only file association.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalSourcePointObligation {
    /// The original selected source handler may not be the entered handler.
    HandlerSelection,
    /// Workspace image/decoding is not an attested native file read.
    FileReadAndDecode,
    /// The containing source frame/body remains conditionally applicable.
    ContainingBody,
}

/// Immutable origin identity issued only after complete parent-point comparison.
/// The matching full entry owns state; this token alone grants no state access.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OriginalSourcePointOrigin(Arc<SourcePointOriginToken>);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SourcePointOriginToken {
    issued_key: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SourcePointPayload {
    settings: Arc<crate::analyser::source_settings::OriginalAnalyserSourceSettings>,
    owner: Arc<OriginalSourceAnalysisOwner>,
    input: ResolvedAnalysisInput,
    site: CommandAllocationSite,
    words: Arc<[NativeWord]>,
    path: usize,
    snapshot: Arc<SourceLookupSnapshot>,
    namespace: SourceNamespaceKey,
    compilation_scope: tcl_runtime_api::SourceCompilationScope,
    region: Option<super::conditional_source::SourceConditionalRegionEntry>,
}

// Full issuer payload comparison precedes key issuance. Keys are never reused;
// strong receipts keep their complete payload alive. No public key constructor.
#[derive(Default)]
struct SourcePointInterner {
    buckets: HashMap<u64, Vec<(Weak<SourcePointPayload>, u64)>>,
    next_key: u64,
}
impl SourcePointInterner {
    fn issue(&mut self, payload: SourcePointPayload) -> Option<OriginalSourcePointEntry> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        payload.hash(&mut hasher);
        self.issue_in_bucket(payload, hasher.finish())
    }

    fn issue_in_bucket(
        &mut self,
        payload: SourcePointPayload,
        fingerprint: u64,
    ) -> Option<OriginalSourcePointEntry> {
        if self.next_key.is_multiple_of(64) {
            self.buckets.retain(|_, bucket| {
                bucket.retain(|(payload, _)| payload.strong_count() != 0);
                !bucket.is_empty()
            });
        }
        let bucket = self.buckets.entry(fingerprint).or_default();
        bucket.retain(|(retained, _)| retained.strong_count() != 0);
        if let Some((payload, key)) = bucket.iter().find_map(|(candidate, key)| {
            let candidate = candidate.upgrade()?;
            (candidate.as_ref() == &payload).then_some((candidate, *key))
        }) {
            return Some(OriginalSourcePointEntry { payload, key });
        }
        self.next_key = self.next_key.checked_add(1)?;
        let key = self.next_key;
        let payload = Arc::new(payload);
        bucket.push((Arc::downgrade(&payload), key));
        Some(OriginalSourcePointEntry { payload, key })
    }
}
fn issue_source_point(payload: SourcePointPayload) -> Option<OriginalSourcePointEntry> {
    static INTERNER: OnceLock<Mutex<SourcePointInterner>> = OnceLock::new();
    INTERNER
        .get_or_init(|| Mutex::new(SourcePointInterner::default()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .issue(payload)
}

impl PartialEq for OriginalSourcePointEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for OriginalSourcePointEntry {}
impl Hash for OriginalSourcePointEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}
impl PartialOrd for OriginalSourcePointEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OriginalSourcePointEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

#[derive(Default)]
struct SourcePointGraphMemo {
    graph: OnceLock<Option<Arc<super::source_transition_advice::OriginalSourcePointAdviceGraph>>>,
}

#[cfg(test)]
pub(super) struct SourcePointCacheRetirementProbe(Weak<SourcePointPayload>);
#[cfg(test)]
impl SourcePointCacheRetirementProbe {
    pub(super) fn is_live(&self) -> bool {
        self.0.strong_count() != 0
    }
}

impl OriginalSourcePointEntry {
    #[cfg(test)]
    pub(super) fn cache_retirement_probe(&self) -> SourcePointCacheRetirementProbe {
        SourcePointCacheRetirementProbe(Arc::downgrade(&self.payload))
    }

    // The history token has exactly this entry's existing private issued-key Eq.
    // Matching it supplies neither a live payload nor permission to serve facts.
    pub(super) fn matches_origin_token(&self, origin: &OriginalSourcePointOrigin) -> bool {
        self.key == origin.0.issued_key
    }

    /// Source association never establishes native loading or decoding.
    #[must_use]
    pub const fn obligations(&self) -> &'static [OriginalSourcePointObligation] {
        &[
            OriginalSourcePointObligation::HandlerSelection,
            OriginalSourcePointObligation::FileReadAndDecode,
            OriginalSourcePointObligation::ContainingBody,
        ]
    }

    pub(crate) fn source_origin(&self) -> OriginalSourcePointOrigin {
        OriginalSourcePointOrigin(Arc::new(SourcePointOriginToken {
            issued_key: self.key,
        }))
    }

    /// Rejoin the actual retained analysis, not another byte-equal analysis.
    /// Host publication retirement remains an independent currentness check.
    #[must_use]
    pub fn matches_parent_analysis(&self, parent: &AnalysisResult) -> bool {
        let Some(input) = parent.resolved_input.as_ref() else {
            return false;
        };
        let Some(realm) = parent.retained_command_realm() else {
            return false;
        };
        let bindings = realm.source_bindings_ref();
        parent.matches_original_source_image(
            self.payload.site.source.source_image(),
            input.lexer_config(),
        ) && input == &self.payload.input
            && self.payload.words.first().is_some_and(|head| {
                head.image() == self.payload.site.source.source_image()
                    && head.config() == input.lexer_config()
                    && head.span().start() == self.payload.site.offset
            })
            && self
                .payload
                .path
                .checked_add(1)
                .and_then(|path| self.payload.words.get(path))
                .is_some_and(|path| {
                    path.image() == self.payload.site.source.source_image()
                        && path.config() == input.lexer_config()
                })
            && realm.matches_resolved_analysis_input(input)
            && bindings
                .original_analysis_owner()
                .is_some_and(|owner| Arc::ptr_eq(owner, &self.payload.owner))
            && self.payload.owner.matches(bindings, input)
            && bindings.analyser_source_settings.as_ref() == Some(&self.payload.settings)
            && realm.matches_original_source_image(
                self.payload.site.source.source_image(),
                input.lexer_config(),
            )
    }

    /// Rejoin the same child source-only entry and complete current analysis.
    /// Physical child/parent publication currency remains the host's obligation.
    #[must_use]
    pub fn matches_child_analysis(&self, child: &AnalysisResult) -> bool {
        let Some(input) = child.resolved_input.as_ref() else {
            return false;
        };
        let Some(realm) = child.retained_command_realm() else {
            return false;
        };
        let Some(image) = realm.original_source_image() else {
            return false;
        };
        self.payload.region.is_none()
            && input == self.input()
            && child.matches_original_source_image(image, input.lexer_config())
            && realm.matches_resolved_analysis_input(input)
            && realm
                .source_bindings_ref()
                .analyser_source_settings
                .as_ref()
                == Some(&self.payload.settings)
            && realm
                .source_bindings_ref()
                .original_inherited_source_entry()
                == Some(self)
    }

    pub(super) fn source_advice_graph(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<Arc<super::source_transition_advice::OriginalSourcePointAdviceGraph>> {
        static MEMOS: OnceLock<
            Mutex<super::AllocationMemoTable<SourcePointPayload, SourcePointGraphMemo>>,
        > = OnceLock::new();
        let retained = self.input().borrowed_context_registry();
        if context.context() != retained.context()
            || context.commands().snapshot().semantic_key()
                != retained.commands().snapshot().semantic_key()
        {
            return None;
        }
        if let Some(region) = &self.payload.region {
            return region.incoming_graph(context);
        }
        // This cache is attached to the actual live payload, outside its full
        // Eq/Hash. The mutex is released before recursive source preparation.
        let memo = {
            MEMOS
                .get_or_init(|| Mutex::new(super::AllocationMemoTable::default()))
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get_or_init(&self.payload)
        };
        memo.graph
            .get_or_init(|| {
                let state = &self.payload.snapshot.state;
                let seed = super::source_transition_advice::OriginalSourceTransitionSeed {
                    analysis_owner: Some(Arc::clone(&self.payload.owner)),
                    origin: Arc::clone(&self.payload.site.source),
                    config: self.input().lexer_config(),
                    baseline: Arc::clone(&state.baseline),
                    logical_input: state.logical_source_name_advice_input().cloned(),
                    stock_source_effects: Vec::new(),
                };
                seed.source_point_graph(
                    context,
                    &self.payload.site,
                    &self.payload.words,
                    self.payload.path,
                    &self.payload.namespace,
                )
            })
            .clone()
    }

    pub(crate) fn source_analysis_entry(
        &self,
        registry: &tcl_registry::CommandRegistry,
        input: &ResolvedAnalysisInput,
    ) -> Option<super::SourceAnalysisEntry> {
        if self.payload.region.is_some()
            || input != self.input()
            || registry.snapshot().semantic_key()
                != input
                    .borrowed_context_registry()
                    .commands()
                    .snapshot()
                    .semantic_key()
        {
            return None;
        }
        #[cfg(test)]
        if self
            .source_advice_graph(input.borrowed_context_registry())
            .is_none()
        {
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_N13_POINT").is_some() {
                eprintln!(
                    "N13_CHILD_ENTRY stage=source-advice-graph offset={}",
                    self.payload.site.offset
                );
            }
            return None;
        }
        #[cfg(not(test))]
        self.source_advice_graph(input.borrowed_context_registry())?;
        Some(self.minted_entry(registry, input))
    }

    fn minted_entry(
        &self,
        registry: &tcl_registry::CommandRegistry,
        input: &ResolvedAnalysisInput,
    ) -> super::SourceAnalysisEntry {
        let mut entry = super::SourceAnalysisEntry::for_supplied_source(
            registry,
            input,
            input.lexer_config(),
            Some(input.unit_profile()),
        );
        entry.declared_commands = self
            .payload
            .snapshot
            .state
            .baseline
            .declared_surface
            .as_ref()
            .map(|contracts| contracts.owned_surface());
        entry.trusted_package_loaders = self
            .payload
            .snapshot
            .state
            .baseline
            .trusted_loaders
            .values()
            .cloned()
            .collect();
        entry
            .trusted_source_modules
            .clone_from(&self.payload.snapshot.state.baseline.trusted_source_modules);
        let baseline = &self.payload.snapshot.state.baseline;
        entry.invocation_realm = self.payload.snapshot.realm;
        entry.execution_name_policy = baseline.execution_name_policy;
        entry
            .logical_source_input
            .clone_from(&baseline.logical_source_input);
        entry
            .vendor_source_input
            .clone_from(&baseline.vendor_source_input);
        entry.invocation_dialect = baseline.dialect;
        entry.compilation_scope = self.payload.compilation_scope;
        entry.source_point_entry = Some(self.clone());
        entry.native_entry = None;
        entry.compiled_variable_provider = None;
        entry.native_compilation =
            tcl_registry::native_compilation::NativeCompilationContext::default();
        entry
    }

    pub(crate) fn accepts_child_contracts(
        &self,
        options: super::SourceAnalysisOptions<'_>,
    ) -> bool {
        options.retained_declared_command_contracts()
            == self.payload.snapshot.state.baseline.declared_surface
            && !options.unknown_entry
            && options.incoming_formals.is_empty()
            && options.trusted_source_modules
                == self
                    .payload
                    .snapshot
                    .state
                    .baseline
                    .trusted_source_modules
                    .as_slice()
            && options
                .trusted_package_loaders
                .iter()
                .map(|loader| (&loader.package, loader))
                .collect::<std::collections::BTreeMap<_, _>>()
                == self
                    .payload
                    .snapshot
                    .state
                    .baseline
                    .trusted_loaders
                    .iter()
                    .collect()
    }

    pub(super) fn is_source_only_interpretation(
        &self,
        state: &super::ModuleCommandBindings,
        context: super::SourceExecutionContext<'_>,
    ) -> bool {
        state.baseline.source_point_entry.as_ref() == Some(self)
            && state.current_source_origin.as_ref().is_some_and(|origin| {
                matches!(origin.kind(), super::SourceOriginKind::InheritedPoint { parent, .. }
                    if parent == &self.source_origin())
            })
            && state.baseline.metadata_context.source_analysis_input() == Some(self.input())
            && context.config == self.input().lexer_config()
            && context.registry.snapshot().semantic_key()
                == self
                    .input()
                    .borrowed_context_registry()
                    .commands()
                    .snapshot()
                    .semantic_key()
            && state.baseline.invocation_realm == self.payload.snapshot.realm
            && context.realm() == self.payload.snapshot.realm
            && context.body_entry() == super::SourceBodyEntryPurpose::Conditional
            && context.compilation()
                == tcl_registry::native_compilation::NativeCompilationContext::default()
            && context.compilation_snapshot.is_none()
            && context.selected_compilation.is_none()
            && context.original_variable_compilation.is_none()
            && state.baseline.native_compilation
                == tcl_registry::native_compilation::NativeCompilationContext::default()
            && state.baseline.native_entry.is_none()
            && state.baseline.hosted_execution_context.is_none()
            && state.baseline.compiled_variable_provider.is_none()
            && !state.baseline.unknown_entry
    }

    pub(crate) fn child_state(
        &self,
        registry: &tcl_registry::CommandRegistry,
        options: super::SourceAnalysisOptions<'_>,
        config: Option<tcl_lexer::LexerConfig>,
    ) -> Option<super::ModuleCommandBindings> {
        let expected = self.minted_entry(registry, self.input());
        if options.retained_metadata_context() != expected.metadata_context
            || options.source_point_entry != Some(self)
            || options.invocation_realm != expected.invocation_realm
            || options.execution_name_policy != expected.execution_name_policy
            || options.logical_source_input != expected.logical_source_input.as_ref()
            || options.vendor_source_input != expected.vendor_source_input.as_ref()
            || options.invocation_dialect != expected.invocation_dialect
            || options.compilation_scope != expected.compilation_scope
            || options.retained_metadata_context().source_analysis_input() != Some(self.input())
            || config != Some(self.input().lexer_config())
            || registry.snapshot().semantic_key()
                != self
                    .input()
                    .borrowed_context_registry()
                    .commands()
                    .snapshot()
                    .semantic_key()
            || options.native_entry.is_some()
            || options.hosted_execution_context.is_some()
            || options.native_compilation
                != tcl_registry::native_compilation::NativeCompilationContext::default()
            || options.compiled_variable_provider.is_some()
            || !self.accepts_child_contracts(options)
        {
            return None;
        }
        let mut state = self.payload.snapshot.state.clone();
        let mut baseline = (*state.baseline).clone();
        baseline.source_point_entry = Some(self.clone());
        baseline.invocation_realm = self.payload.snapshot.realm;
        baseline.semantics = match baseline.logical_source_input.as_ref() {
            Some(input) => registry.authored_source_semantics_in_context(
                input.context_registry().context(),
                baseline.invocation_realm,
            ),
            None => registry.effective_semantics_for_dialect_in_realm(
                baseline.dialect?,
                baseline.invocation_realm,
            ),
        };
        baseline.native_entry = None;
        baseline.native_compilation =
            tcl_registry::native_compilation::NativeCompilationContext::default();
        baseline.compiled_variable_provider = None;
        baseline.refresh_fingerprint();
        state.baseline = Arc::new(baseline);
        Some(state)
    }

    pub(crate) fn namespace_scope(
        &self,
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        use crate::signature_scan::scope::SignatureNamespaceScope;
        match self.namespace() {
            SourceNamespaceKey::Authored(key) if self.input().has_logical_source_name_context() => {
                Some(SignatureNamespaceScope::Symbolic(key.clone()))
            }
            namespace => {
                let policy = self
                    .payload
                    .snapshot
                    .state
                    .baseline
                    .execution_name_policy?
                    .native_recipe()?;
                if !matches!(
                    policy.recipe(),
                    tcl_syntax::naming::NativeNameProtocol::C(_)
                ) {
                    return None;
                }
                Some(SignatureNamespaceScope::C(
                    namespace.source_artifact_path()?,
                ))
            }
        }
    }

    pub(super) fn owns_ancestor_origin(&self, origin: &super::SourceOriginId) -> bool {
        let mut entry = Some(self);
        while let Some(point) = entry {
            if point.payload.site.source.as_ref() == origin {
                return true;
            }
            entry = point
                .payload
                .snapshot
                .state
                .baseline
                .source_point_entry
                .as_ref();
        }
        false
    }

    pub(super) fn matches_ancestor_analysis(&self, parent: &AnalysisResult) -> bool {
        let mut entry = Some(self);
        while let Some(point) = entry {
            if point.matches_parent_analysis(parent) {
                return true;
            }
            entry = point
                .payload
                .snapshot
                .state
                .baseline
                .source_point_entry
                .as_ref();
        }
        false
    }

    pub(super) fn for_conditional_region(
        &self,
        region: super::conditional_source::SourceConditionalRegionEntry,
        state: super::ModuleCommandBindings,
        namespace: SourceNamespaceKey,
    ) -> Option<Self> {
        if !region.matches_predecessor(self, &state, &namespace) {
            return None;
        }
        let mut payload = (*self.payload).clone();
        payload.snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            state,
            self.payload.snapshot.realm,
        ));
        payload.namespace = namespace;
        payload.region = Some(region);
        issue_source_point(payload)
    }

    pub(super) fn original_region_plan(
        &self,
        origin: &Arc<super::SourceOriginId>,
        config: tcl_lexer::LexerConfig,
    ) -> Option<tcl_lexer::NativeScriptWordsPlan> {
        if !matches!(origin.kind(), super::SourceOriginKind::InheritedPoint { parent, source }
            if parent == &self.source_origin()
            && source == self.payload.site.source.source_image())
        {
            return None;
        }
        self.payload.region.as_ref()?.original_plan(origin, config)
    }

    pub(super) fn conditional_region(
        &self,
    ) -> Option<&super::conditional_source::SourceConditionalRegionEntry> {
        self.payload.region.as_ref()
    }

    /// The issuing source's own state when `origin` is that source continuing
    /// past one of its conditional regions. A child view of another file, or
    /// any other origin, inherits naming state only and receives no issuer.
    pub(super) fn issuing_continuation_state(
        &self,
        origin: &super::SourceOriginId,
    ) -> Option<&super::ModuleCommandBindings> {
        // TEMP-ISSUER-TRACE
        if std::env::var_os("TCL_LSP_TEMP_SOURCE_ADVICE_TRACE").is_some() {
            let (kind, parent_eq, source_eq) = match origin.kind() {
                super::SourceOriginKind::InheritedPoint { parent, source } => ("inherited", parent == &self.source_origin(), source == self.payload.site.source.source_image()),
                super::SourceOriginKind::Authored(_) => ("authored", false, false),
                _ => ("other", false, false),
            };
            eprintln!("TEMP_ISSUER region={} kind={kind} parent_eq={parent_eq} source_eq={source_eq} site_offset={}", self.payload.region.is_some(), self.payload.site.offset);
        }
        let issuer = self.payload.region.as_ref()?.issuing_point();
        (issuer.payload.site == self.payload.site
            && matches!(origin.kind(), super::SourceOriginKind::InheritedPoint { parent, source }
                if parent == &self.source_origin()
                && source == self.payload.site.source.source_image()))
        .then_some(&issuer.payload.snapshot.state)
    }

    pub(super) fn internal_region_entry(&self) -> Option<super::SourceAnalysisEntry> {
        self.payload.region.as_ref()?;
        Some(self.minted_entry(
            self.input().borrowed_context_registry().commands(),
            self.input(),
        ))
    }

    pub(super) fn allocation_site(&self) -> &CommandAllocationSite {
        &self.payload.site
    }

    pub(super) fn source_file_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.payload.snapshot.state.baseline.dialect
    }

    /// Restore the actual parent's naming settings onto a fresh child driver.
    /// Physical child path/publication and diagnostic policy remain independent.
    pub fn configure_analyser(
        &self,
        parent: &AnalysisResult,
        analyser: &mut crate::analyser::Analyser,
    ) -> bool {
        if !self.matches_parent_analysis(parent) {
            return false;
        }
        self.payload.settings.apply(analyser);
        true
    }

    pub(super) fn analyser_source_settings(
        &self,
    ) -> Arc<crate::analyser::source_settings::OriginalAnalyserSourceSettings> {
        Arc::clone(&self.payload.settings)
    }

    pub(crate) fn accepts_analyser_source_settings(
        &self,
        analyser: &crate::analyser::Analyser,
    ) -> bool {
        self.payload.settings.as_ref()
            == &crate::analyser::source_settings::OriginalAnalyserSourceSettings::capture(analyser)
    }

    pub(crate) fn input(&self) -> &ResolvedAnalysisInput {
        &self.payload.input
    }

    pub(crate) fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.payload.snapshot.state.variable_frame
    }

    pub(crate) fn namespace(&self) -> &SourceNamespaceKey {
        &self.payload.namespace
    }
}

impl SourceCommandBindings {
    pub(crate) fn original_inherited_source_entry(&self) -> Option<&OriginalSourcePointEntry> {
        let point = self.final_state.baseline.source_point_entry.as_ref()?;
        let origin = self.root_origin.as_ref()?;
        if self.analyser_source_settings.as_ref() != Some(&point.payload.settings)
            || self.source_metadata_owner().source_analysis_input() != Some(point.input())
            || !matches!(origin.kind(), super::SourceOriginKind::InheritedPoint { parent, .. } if parent == &point.source_origin())
        {
            return None;
        }
        Some(point)
    }

    pub(crate) fn matches_inherited_source_origin(&self, origin: &super::SourceOriginId) -> bool {
        self.original_inherited_source_entry().is_some()
            && self.root_origin.as_deref() == Some(origin)
    }

    #[cfg(test)]
    fn trace_original_source_point_state(
        first: &super::SourceBindingPoint,
        state: &super::ModuleCommandBindings,
        offset: u32,
    ) {
        if std::env::var_os("TCL_LSP_TRACE_N13_POINT").is_some() {
            eprintln!(
                "N13_POINT_STATE offset={offset} argv={} native={} hosted={} opaque={} unclosed={} step={} quiet={} unknown_frame={} selected_frame={} receiver_frame={} registry_target={} inherited={}",
                first.original_arguments_complete_normally,
                state.baseline.native_entry.is_some(),
                state.baseline.hosted_execution_context.is_some(),
                state.has_opaque_domain(),
                state.unclosed_command_world,
                state.source_step_observed(),
                state.command_observers.is_quiet(),
                matches!(
                    state.variable_frame.layout(),
                    crate::var_resolve::VariableExecutionFrame::Unknown
                ),
                matches!(
                    state.variable_frame.layout(),
                    crate::var_resolve::VariableExecutionFrame::Selected { .. }
                ),
                matches!(
                    state.variable_frame.layout(),
                    crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                ),
                first
                    .original_dispatch_binding()
                    .proved_target()
                    .is_some_and(|target| target.registry_backed),
                state.baseline.source_point_entry.is_some(),
            );
        }
    }

    /// Exact original words at a unanimous post-argv/pre-handler point.
    /// Compiler-only previews cannot substitute for that point inventory.
    pub(crate) fn original_source_point_entry(
        &self,
        input: &ResolvedAnalysisInput,
        original: &[NativeWord],
        path: usize,
    ) -> Option<OriginalSourcePointEntry> {
        macro_rules! refuse_point {
            ($stage:literal) => {{
                #[cfg(test)]
                if std::env::var_os("TCL_LSP_TRACE_N13_POINT").is_some() {
                    eprintln!(
                        "N13_POINT stage={} offset={:?}",
                        $stage,
                        original.first().map(|word| word.span().start())
                    );
                }
                return None;
            }};
        }
        let Some(owner) = self.original_analysis_owner() else {
            refuse_point!("owner-missing");
        };
        if !owner.matches(self, input) {
            refuse_point!("owner-input");
        }
        let head = original.first()?;
        let offset = head.span().start();
        let config = input.lexer_config();
        if self.lexer_config != Some(config)
            || self.source_metadata_owner().source_analysis_input() != Some(input)
            || original.iter().any(|word| {
                word.image() != head.image() || word.config() != config || word.group().expand
            })
            || original.get(path.checked_add(1)?).is_none()
        {
            refuse_point!("vector-metadata");
        }
        let mut points = self
            .dispatch_points_at(offset)
            .filter(|point| !point.declaration_preview);
        let Some(first) = points.next() else {
            refuse_point!("no-dispatch-point");
        };
        let state = &first.state;
        #[cfg(test)]
        Self::trace_original_source_point_state(first, state, offset);
        if !first.original_arguments_complete_normally
            || state.baseline.native_entry.is_some()
            || state.baseline.hosted_execution_context.is_some()
            || state.has_opaque_domain()
            || state.unclosed_command_world
            || state.source_step_observed()
            || !state.command_observers.is_quiet()
            || matches!(
                state.variable_frame.layout(),
                crate::var_resolve::VariableExecutionFrame::Unknown
                    | crate::var_resolve::VariableExecutionFrame::Selected { .. }
                    | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
            )
            || points.any(|point| point != first)
            || first
                .original_dispatch_binding()
                .proved_target()
                .is_none_or(|target| !target.registry_backed)
        {
            refuse_point!("state-or-alternatives");
        }
        let origin = state.current_source_origin.as_ref()?;
        let written = first.original_written_words.as_ref()?;
        if written.site().source != *origin || !written.matches_native_words(original) {
            refuse_point!("written-origin-vector");
        }
        let payload = SourcePointPayload {
            settings: Arc::clone(self.analyser_source_settings.as_ref()?),
            owner: Arc::clone(owner),
            input: input.clone(),
            site: CommandAllocationSite {
                source: Arc::clone(origin),
                offset,
            },
            words: Arc::from(original),
            path,
            snapshot: Arc::new(SourceLookupSnapshot::in_realm(state.clone(), first.realm)),
            namespace: first.namespace_key.clone(),
            compilation_scope: self.compilation_scope,
            region: None,
        };
        issue_source_point(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point() -> OriginalSourcePointEntry {
        let source = "source child.tcl";
        let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        crate::registry_invocation::source_structure::original_project_source_load_at(
            source, &parent, 0,
        )
        .unwrap()
        .source_point_entry(&parent)
        .unwrap()
    }

    #[test]
    fn source_point_identity_exhaustion_refuses_only_new_receipts() {
        let a = point();
        let b = point();
        let mut interner = SourcePointInterner::default();
        let retained = interner.issue((*a.payload).clone()).unwrap();
        interner.next_key = u64::MAX;
        assert_eq!(interner.issue((*a.payload).clone()), Some(retained));
        assert!(interner.issue((*b.payload).clone()).is_none());
    }
    #[test]
    fn source_point_hash_collision_keeps_complete_parent_payloads_distinct() {
        let a = point();
        let b = point();
        let mut interner = SourcePointInterner::default();
        let first = interner.issue_in_bucket((*a.payload).clone(), 0).unwrap();
        let second = interner.issue_in_bucket((*b.payload).clone(), 0).unwrap();
        assert_ne!(first, second);
        assert_eq!(
            interner.issue_in_bucket((*a.payload).clone(), 0),
            Some(first)
        );
        assert_eq!(
            interner.issue_in_bucket((*b.payload).clone(), 0),
            Some(second)
        );
    }
    #[test]
    fn inherited_source_options_refuse_each_changed_entry_axis() {
        let point = point();
        let input = point.input();
        let registry = input.borrowed_context_registry().commands();
        let entry = point.source_analysis_entry(registry, input).unwrap();
        let options = entry.options();
        assert!(
            point
                .child_state(registry, options, Some(input.lexer_config()))
                .is_some()
        );
        let other_realm = match options.invocation_realm {
            tcl_dialect::model::InvocationRealm::RuleLoader => {
                tcl_dialect::model::InvocationRealm::InterpreterRuntime
            }
            tcl_dialect::model::InvocationRealm::InterpreterRuntime => {
                tcl_dialect::model::InvocationRealm::RuleLoader
            }
        };
        let variants = [
            super::super::SourceAnalysisOptions {
                invocation_realm: other_realm,
                ..options
            },
            super::super::SourceAnalysisOptions {
                execution_name_policy: None,
                ..options
            },
            super::super::SourceAnalysisOptions {
                invocation_dialect: None,
                ..options
            },
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V9_0,
                )),
                ..options
            },
            super::super::SourceAnalysisOptions {
                logical_source_input: Some(input),
                ..options
            },
            super::super::SourceAnalysisOptions {
                vendor_source_input: Some(input),
                ..options
            },
            super::super::SourceAnalysisOptions {
                compilation_scope: tcl_runtime_api::SourceCompilationScope::EnteredSource,
                ..options
            },
            super::super::SourceAnalysisOptions {
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                    ..Default::default()
                },
                ..options
            },
        ];
        for changed in variants {
            assert!(
                point
                    .child_state(registry, changed, Some(input.lexer_config()))
                    .is_none(),
                "{changed:?}"
            );
            let bindings = SourceCommandBindings::analyse_with_options(
                "set local VALUE",
                input.lexer_config(),
                registry,
                changed,
            );
            assert!(bindings.original_inherited_source_entry().is_none());
            assert!(
                bindings
                    .source_metadata_owner()
                    .source_analysis_input()
                    .is_none()
            );
            assert!(bindings.original_completed_root_state.is_none());
        }
    }

    #[test]
    fn inherited_source_entry_never_becomes_a_runtime_lowering_target() {
        let point = point();
        let input = point.input();
        let registry = input.borrowed_context_registry().commands();
        let entry = point.source_analysis_entry(registry, input).unwrap();
        for procedure in [false, true] {
            let mut lowerer = crate::lowering::Lowerer::with_config(registry, input.lexer_config());
            lowerer.set_source_analysis_options(entry.options());
            let module = if procedure {
                lowerer.lower_procedure_target("set local VALUE", "foreign")
            } else {
                lowerer.lower_script_target("set local VALUE", "foreign")
            };
            assert!(matches!(
                module.top_level.statements.as_slice(),
                [crate::ir::Statement::Barrier { .. }]
            ));
            assert_eq!(
                module.source_entry.native_compilation,
                tcl_registry::native_compilation::NativeCompilationContext::default()
            );
            assert!(module.source_entry.native_entry.is_none());
            assert!(module.source_entry.compiled_variable_provider.is_none());
            assert_eq!(module.original_native_declaration_body_units.len(), 0);
        }
        let mut bytecode = crate::lowering::Lowerer::with_config(registry, input.lexer_config())
            .for_bytecode_backend();
        bytecode.set_source_analysis_options(entry.options());
        let module = bytecode.lower("set local VALUE");
        assert!(matches!(
            module.top_level.statements.as_slice(),
            [crate::ir::Statement::Barrier { .. }]
        ));
        assert_eq!(
            module.source_entry.native_compilation,
            tcl_registry::native_compilation::NativeCompilationContext::default()
        );
    }

    #[test]
    fn inherited_child_unit_retains_the_actual_point_after_external_handles_drop() {
        let source = "source child.tcl";
        let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let point = crate::registry_invocation::source_structure::original_project_source_load_at(
            source, &parent, 0,
        )
        .unwrap()
        .source_point_entry(&parent)
        .unwrap();
        let weak_payload = Arc::downgrade(&point.payload);
        let weak_owner = Arc::downgrade(&point.payload.owner);
        let input = point.input().clone();
        let child_source = "proc run {} {return VALUE}";
        let child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
            .unwrap();
        assert!(point.matches_child_analysis(&child));
        let mut unit = child
            .original_optimiser_source_unit(
                child_source,
                input.borrowed_context_registry().commands(),
                None,
            )
            .unwrap();
        assert!(Arc::ptr_eq(
            &unit
                .ir_module
                .source_entry
                .source_point_entry
                .as_ref()
                .unwrap()
                .payload,
            &point.payload,
        ));
        drop(child);
        drop(point);
        drop(parent);
        let payload = weak_payload
            .upgrade()
            .expect("retained unit owns full point");
        let owner = weak_owner
            .upgrade()
            .expect("retained point owns actual parent");
        assert!(Arc::ptr_eq(&payload.owner, &owner));
        let strong_before = Arc::strong_count(&payload);
        // Count this exact field's strong ownership independently of any cache
        // or other unit receipts that may also retain the same full payload.
        let retained = unit
            .ir_module
            .source_entry
            .source_point_entry
            .take()
            .unwrap();
        assert!(Arc::ptr_eq(&retained.payload, &payload));
        drop(retained);
        assert_eq!(Arc::strong_count(&payload), strong_before - 1);
    }

    #[test]
    fn source_entry_reborrow_keeps_the_same_inherited_request_without_a_unit_build() {
        let point = point();
        let input = point.input();
        let registry = input.borrowed_context_registry().commands();
        let source = "proc run {} {return VALUE}";
        let child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry(source, "tcl8.6", &point)
            .unwrap();
        let entry = child
            .original_source_analysis_entry(source, registry, None)
            .unwrap();
        assert_eq!(entry.source_point_entry.as_ref(), Some(&point));
        assert!(entry.native_entry.is_none());
        assert_eq!(
            entry.native_compilation,
            tcl_registry::native_compilation::NativeCompilationContext::default()
        );
        assert!(
            child
                .original_source_analysis_entry("WRONG", registry, None)
                .is_none()
        );
        let mut unavailable = child.clone();
        unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
            environment: "tcl8.6".to_owned(),
            overlay: 1,
        });
        assert!(
            unavailable
                .original_source_analysis_entry(source, registry, None)
                .is_none()
        );
        let mut wrong_config = child.clone();
        wrong_config.body_lexer_config = None;
        assert!(
            wrong_config
                .original_source_analysis_entry(source, registry, None)
                .is_none()
        );
    }

    fn assert_bypass_has_no_source_input(bypass: &crate::analyser::AnalysisResult) {
        assert!(
            bypass
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .source_metadata_owner()
                .source_analysis_input()
                .is_none()
        );
    }

    #[test]
    fn inherited_ingress_reapplies_and_revalidates_actual_parent_source_settings() {
        let source = "source child.tcl";
        let parent = crate::analyser::Analyser::new()
            .with_extra_commands(std::collections::HashSet::from(["parent_hint".to_owned()]))
            .analyse(source, "tcl8.6");
        let point = crate::registry_invocation::source_structure::original_project_source_load_at(
            source, &parent, 0,
        )
        .unwrap()
        .source_point_entry(&parent)
        .unwrap();
        let mut mismatched = crate::analyser::Analyser::new();
        assert!(
            mismatched
                .analyse_with_source_point_entry("", "tcl8.6", &point)
                .is_none()
        );
        let mut child =
            crate::analyser::Analyser::new().with_file_path(Some("child.tcl".to_owned()));
        assert!(point.configure_analyser(&parent, &mut child));
        let analysis = child
            .analyse_with_source_point_entry("", "tcl8.6", &point)
            .unwrap();
        assert!(point.matches_child_analysis(&analysis));
        let mut configured = crate::analyser::Analyser::new();
        assert!(analysis.configure_source_analyser(
            "",
            point.input().borrowed_context_registry().commands(),
            &mut configured,
        ));
        assert!(point.accepts_analyser_source_settings(&configured));
        assert!(analysis.accepts_source_analyser_settings(
            "",
            point.input().borrowed_context_registry().commands(),
            &configured,
        ));
        configured
            .workspace_classes
            .insert("foreign_hint".to_owned());
        assert!(!analysis.accepts_source_analyser_settings(
            "",
            point.input().borrowed_context_registry().commands(),
            &configured,
        ));
        let variants: &[fn(&mut crate::analyser::Analyser)] = &[
            |a| a.extra_commands = Arc::new(std::collections::HashSet::new()),
            |a| a.pack_overlay = 41,
            |a| {
                a.declared_targets
                    .push(("tcl".to_owned(), "8.6-".to_owned()));
            },
            |a| {
                a.package_provides
                    .push(("Pkg".to_owned(), vec!["Other".to_owned()]));
            },
            |a| a.deep_param_traits = true,
            |a| a.structure_only = true,
            |a| a.defer_proc_bodies = true,
            |a| {
                a.workspace_classes.insert("Other".to_owned());
            },
            |a| {
                a.workspace_bare_word_classes.insert("Other".to_owned());
            },
            |a| a.workspace_class_factories = Some(Arc::new(std::collections::BTreeMap::default())),
            |a| {
                a.workspace_subclass_methods =
                    Some(Arc::new(std::collections::BTreeMap::default()));
            },
            |a| {
                a.library_versions = crate::analyser::Analyser::new()
                    .with_bigip_version(Some("16.1".to_owned()))
                    .library_versions;
            },
        ];
        for mutate in variants {
            let mut child = crate::analyser::Analyser::new();
            assert!(point.configure_analyser(&parent, &mut child));
            mutate(&mut child);
            assert!(
                child
                    .analyse_with_source_point_entry("", "tcl8.6", &point)
                    .is_none()
            );
        }
        let entry = point
            .source_analysis_entry(
                point.input().borrowed_context_registry().commands(),
                point.input(),
            )
            .unwrap();
        let bypass = crate::analyser::Analyser::new()
            .with_source_analysis_entry(Arc::new(entry))
            .analyse("", "tcl8.6");
        assert!(!point.matches_child_analysis(&bypass));
        assert_bypass_has_no_source_input(&bypass);
        let foreign = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(!point.configure_analyser(&foreign, &mut configured));
    }

    #[test]
    fn source_point_token_lifetime_and_equality_keep_the_full_live_receipt() {
        fn hash(value: &impl Hash) -> u64 {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }
        let a = point();
        let clone = a.clone();
        let token = a.source_origin();
        assert_eq!(a, clone);
        assert_eq!(a.cmp(&clone), std::cmp::Ordering::Equal);
        assert_eq!(hash(&a), hash(&clone));
        assert_eq!(token, clone.source_origin());
        assert_eq!(token.cmp(&clone.source_origin()), std::cmp::Ordering::Equal);
        assert_eq!(hash(&token), hash(&clone.source_origin()));
        assert!(
            a.source_advice_graph(a.input().borrowed_context_registry())
                .is_some()
        );
        assert_eq!(a, clone);
        assert_eq!(token, clone.source_origin());
        assert_eq!(hash(&a), hash(&clone));
        let mut interner = SourcePointInterner::default();
        let issued = interner.issue((*a.payload).clone()).unwrap();
        let old_token = issued.source_origin();
        drop(issued);
        let replacement = interner.issue((*a.payload).clone()).unwrap();
        assert_ne!(old_token, replacement.source_origin());
        assert_ne!(
            old_token.cmp(&replacement.source_origin()),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn inherited_source_selection_retains_names_without_native_compiler_admission() {
        let point = point();
        let child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry("source leaf.tcl", "tcl8.6", &point)
            .unwrap();
        let bindings = child
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let dispatch = bindings.dispatch_points_at(0).next().unwrap();
        let registry = point.input().borrowed_context_registry().commands();
        let entry = point.minted_entry(registry, point.input());
        let context = super::super::root_source_execution_context(
            &dispatch.state.variable_frame,
            "::",
            &dispatch.namespace_key,
            point.input().lexer_config(),
            registry,
            entry.options(),
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        );
        assert!(point.is_source_only_interpretation(&dispatch.state, context));
        let mut different = context;
        different.config.strict_quoting = !different.config.strict_quoting;
        assert!(!point.is_source_only_interpretation(&dispatch.state, different));
        different = context;
        different.control = different
            .control
            .with_body_entry(super::super::SourceBodyEntryPurpose::Selected);
        assert!(!point.is_source_only_interpretation(&dispatch.state, different));
        let mut foreign = dispatch.state.clone();
        foreign.current_source_origin = Some(Arc::clone(&point.payload.site.source));
        assert!(!point.is_source_only_interpretation(&foreign, context));
        let mut baseline = (*dispatch.state.baseline).clone();
        baseline.source_point_entry = None;
        foreign = dispatch.state.clone();
        foreign.baseline = Arc::new(baseline);
        assert!(!point.is_source_only_interpretation(&foreign, context));
        let mut baseline = (*dispatch.state.baseline).clone();
        baseline.metadata_context =
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
        foreign = dispatch.state.clone();
        foreign.baseline = Arc::new(baseline);
        assert!(!point.is_source_only_interpretation(&foreign, context));
        assert!(!dispatch.state.has_opaque_domain());
        assert!(!dispatch.compiled_execution.unknown);
        assert!(dispatch.compiled_execution.admission.is_none());
        assert!(dispatch.compiled_execution.admitted.is_none());
        assert!(dispatch.compiled_execution.policy.is_none());
        assert!(dispatch.compiled_execution.proofs.is_empty());
        assert!(dispatch.compiled_execution.structured.is_none());
        assert!(dispatch.state.baseline.compilation_dialect().is_none());
        assert!(dispatch.state.baseline.native_entry.is_none());
        assert!(
            crate::registry_invocation::source_structure::original_project_source_load_at(
                "source leaf.tcl",
                &child,
                0,
            )
            .unwrap()
            .source_point_entry(&child)
            .is_some()
        );
        let opaque = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry("unknown_writer; source leaf.tcl", "tcl8.6", &point)
            .unwrap();
        assert!(
            crate::registry_invocation::source_structure::original_project_source_load_at(
                "unknown_writer; source leaf.tcl",
                &opaque,
                16,
            )
            .unwrap()
            .source_point_entry(&opaque)
            .is_none()
        );
    }

    #[test]
    fn source_point_allocator_compares_same_owner_actual_point_snapshots() {
        let source = "namespace eval lib {}; namespace eval other {}; namespace eval a {namespace path ::lib; list ONE}; namespace eval b {namespace path ::other; list TWO}; source child.tcl";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let final_offset = u32::try_from(source.find("source child.tcl").unwrap()).unwrap();
        let entry = crate::registry_invocation::source_structure::original_project_source_load_at(
            source,
            &analysis,
            final_offset,
        )
        .unwrap()
        .source_point_entry(&analysis)
        .unwrap();
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        // The allocator compares genuine point payloads independently of the
        // separate public SourceLoad-schema issuer; these are not public loads.
        let payload = |command: &str| {
            let offset = u32::try_from(source.find(command).unwrap()).unwrap();
            let point = bindings.dispatch_points_at(offset).next().unwrap();
            let image = tcl_lexer::SourceImage::document(source);
            let plan = tcl_lexer::native_script_words_in(
                image,
                tcl_lexer::Span::new(offset, offset + u32::try_from(command.len()).unwrap()),
                entry.input().lexer_config(),
            )
            .unwrap();
            SourcePointPayload {
                settings: Arc::clone(&entry.payload.settings),
                owner: Arc::clone(&entry.payload.owner),
                input: entry.input().clone(),
                site: CommandAllocationSite {
                    source: Arc::clone(point.state.current_source_origin.as_ref().unwrap()),
                    offset,
                },
                words: Arc::from(plan.commands[0].words.clone()),
                path: 0,
                snapshot: Arc::new(SourceLookupSnapshot::in_realm(
                    point.state.clone(),
                    point.realm,
                )),
                namespace: point.namespace_key.clone(),
                compilation_scope: bindings.compilation_scope,
                region: None,
            }
        };
        let a = payload("list ONE");
        let b = payload("list TWO");
        assert!(Arc::ptr_eq(&a.owner, &b.owner));
        assert_ne!(a.site, b.site);
        assert_ne!(a.namespace, b.namespace);
        assert_ne!(
            a.snapshot.state.namespace_paths,
            b.snapshot.state.namespace_paths
        );
        assert_ne!(a.snapshot, b.snapshot);
        let mut interner = SourcePointInterner::default();
        let first = interner.issue_in_bucket(a.clone(), 0).unwrap();
        let second = interner.issue_in_bucket(b.clone(), 0).unwrap();
        assert_ne!(first, second);
        assert_eq!(interner.issue_in_bucket(a, 0), Some(first));
        assert_eq!(interner.issue_in_bucket(b, 0), Some(second));
    }
}

#[cfg(test)]
mod unavailable_entry_reborrow_controls {
    use super::*;

    #[test]
    fn source_entry_reborrow_refuses_unavailable_ordinary_and_unknown_history() {
        let source = "proc sample {} {puts VALUE}\n";
        let ordinary = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let input = ordinary.resolved_input.as_ref().unwrap().clone();
        let registry = input.borrowed_context_registry().commands();
        let unknown_entry = Arc::new(super::super::SourceAnalysisEntry::unknown_history_for_test(
            &input,
        ));
        let unknown = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .with_source_analysis_entry(unknown_entry)
            .analyse(source, "tcl8.6");
        for (analysis, unknown_history) in [(ordinary, false), (unknown, true)] {
            let entry = analysis
                .original_source_analysis_entry(source, registry, None)
                .unwrap();
            assert_eq!(entry.options().unknown_entry, unknown_history);
            assert!(entry.options().source_point_entry.is_none());
            let mut unavailable = analysis.clone();
            unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
                environment: "tcl8.6".to_owned(),
                overlay: 1,
            });
            assert!(
                unavailable
                    .original_source_analysis_entry(source, registry, None)
                    .is_none()
            );
            let mut wrong_config = analysis.clone();
            wrong_config.body_lexer_config = None;
            assert!(
                wrong_config
                    .original_source_analysis_entry(source, registry, None)
                    .is_none()
            );
        }
    }
}
