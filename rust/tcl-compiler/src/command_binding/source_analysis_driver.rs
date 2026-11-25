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

//! The entry points that interpret one source instance and the root state they start from.

use super::namespace_context::SourceNamespaceKey;
use super::native_compiler_formals::OriginalNativeCompilerFormals;
use super::source_analysis_entry::SourceAnalysisOptions;
use super::source_origin::{SourceOriginId, SourceOriginKind};
use super::source_point_entry::OriginalSourcePointEntry;
use super::source_walk::{SourceBodyEntryPurpose, source_variables_in_frame};
use super::{
    AllocationMemoTable, ModuleCommandBindings, OriginalSourceAnalysisMemo,
    OriginalSourceAnalysisOwner, SourceCommandBindings, SourceExecutionContext,
    literal_object_pool, source_analysis_cache,
};
#[cfg(any(test, debug_assertions))]
use super::{original_command_table, root_owner_work};
#[cfg(any(test, debug_assertions))]
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::{Arc, Mutex, OnceLock};
use tcl_registry::CommandRegistry;

impl PartialEq for OriginalSourceAnalysisOwner {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for OriginalSourceAnalysisOwner {}

impl Hash for OriginalSourceAnalysisOwner {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::from_ref(self).hash(state);
    }
}

impl OriginalSourceAnalysisOwner {
    pub(super) fn derived_memo(self: &Arc<Self>) -> Arc<OriginalSourceAnalysisMemo> {
        static MEMOS: OnceLock<
            Mutex<AllocationMemoTable<OriginalSourceAnalysisOwner, OriginalSourceAnalysisMemo>>,
        > = OnceLock::new();
        MEMOS
            .get_or_init(|| Mutex::new(AllocationMemoTable::default()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_or_init(self)
    }
    pub(crate) fn matches(
        &self,
        bindings: &SourceCommandBindings,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        bindings
            .source_origin()
            .is_some_and(|source| Arc::ptr_eq(source, &self.source))
            && bindings.lexer_config == Some(self.config)
            && input.lexer_config() == self.config
            && self.metadata.source_analysis_input() == Some(input)
            && bindings.source_metadata_owner().source_analysis_input() == Some(input)
    }
}

/// Segmentation purpose and the survey-only conditional source control of
/// one whole-source interpretation.
#[derive(Clone, Copy)]
struct SourceInterpretationEntry<'a> {
    pub(super) purpose: tcl_lexer::SourceSegmentationPurpose,
    pub(super) logical_conditional_source_control: bool,
    native_formals: Option<&'a Arc<OriginalNativeCompilerFormals>>,
}

fn initial_source_interpretation_state(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    registry: &CommandRegistry,
    options: SourceAnalysisOptions<'_>,
) -> ModuleCommandBindings {
    let mut state = ModuleCommandBindings::initial_with_options(registry, options, Some(config));
    Arc::make_mut(&mut state.source_variables)
        .captured_cells
        .start_lineage();
    state.current_source_origin = Some(Arc::new(match options.source_point_entry {
        Some(parent) => SourceOriginId::from_kind(SourceOriginKind::InheritedPoint {
            parent: parent.source_origin(),
            source: image.clone(),
        }),
        None => SourceOriginId::authored_image(image.clone()),
    }));
    state.ordinary_literal_pool =
        literal_object_pool::SourceOrdinaryLiteralPool::at_entry(&state, options);
    state
}

fn source_entry_namespace_key(
    options: &SourceAnalysisOptions<'_>,
    frame: &crate::var_resolve::VariableExecutionFrame,
    namespace: &str,
) -> SourceNamespaceKey {
    options
        .source_point_entry
        .map(|entry| entry.namespace().clone())
        .or_else(|| frame.namespace_identity().cloned())
        .or_else(|| {
            options
                .native_entry
                .and_then(|entry| SourceNamespaceKey::from_native_entry(entry).ok())
        })
        .unwrap_or_else(|| SourceNamespaceKey::authored(namespace))
}

fn source_frame_namespace(frame: &crate::var_resolve::VariableExecutionFrame) -> &str {
    use crate::var_resolve::VariableExecutionFrame;
    match frame.layout() {
        VariableExecutionFrame::Namespace(namespace)
        | VariableExecutionFrame::NamespaceActivation { namespace, .. }
        | VariableExecutionFrame::Procedure { namespace, .. }
        | VariableExecutionFrame::Selected {
            namespace: Some(namespace),
            ..
        } => namespace,
        VariableExecutionFrame::Selected {
            namespace: None, ..
        }
        | VariableExecutionFrame::ReceiverMethod { .. }
        | VariableExecutionFrame::Global
        | VariableExecutionFrame::Unknown => "::",
        VariableExecutionFrame::NamespaceIdentity { .. } => {
            unreachable!("layout excludes identity wrappers")
        }
    }
}

pub(super) fn root_source_execution_context<'a>(
    frame: &'a crate::var_resolve::VariableExecutionFrame,
    namespace: &'a str,
    namespace_key: &'a SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
    registry: &'a CommandRegistry,
    options: SourceAnalysisOptions<'a>,
    source_purpose: tcl_lexer::SourceSegmentationPurpose,
) -> SourceExecutionContext<'a> {
    SourceExecutionContext {
        original_logical_body: None,
        control: crate::command_binding::source_execution_control::SourceExecutionControl::new(
            options.native_compilation,
            source_purpose,
            if options.source_point_entry.is_some() {
                SourceBodyEntryPurpose::Conditional
            } else {
                SourceBodyEntryPurpose::Selected
            },
            options.invocation_realm,
        ),
        compilation_snapshot: None,
        selected_compilation: None,
        original_variable_compilation: None,
        namespace,
        config,
        registry,
        depth: 0,
        frame,
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
        namespace_key: Some(namespace_key),
        expression_source: None,
    }
}

#[cfg(any(test, debug_assertions))]
fn trace_source_analysis(
    start: Option<std::time::Instant>,
    stage: &str,
    source_len: usize,
    bindings: &SourceCommandBindings,
) {
    if let Some(start) = start {
        root_owner_work::report(stage);
        eprintln!(
            "SOURCE_PHASE bytes={} stage={} ms={} points={} layouts={}",
            source_len,
            stage,
            start.elapsed().as_millis(),
            bindings.points.len(),
            bindings.declaration_layouts.len()
        );
        let mut owners = HashSet::new();
        let (mut requests, mut captures, mut rejected) = (0_usize, 0_usize, 0_usize);
        for origin in bindings
            .root_origin
            .iter()
            .chain(bindings.origin_points.keys())
        {
            let memo = origin.original_lexical_projection();
            if owners.insert(Arc::as_ptr(&memo)) {
                let (owner_requests, owner_captures, owner_rejected) = memo.trace_counts();
                requests += owner_requests;
                captures += owner_captures;
                rejected += owner_rejected;
            }
        }
        eprintln!(
            "SOURCE_LEXICAL_PROJECTION_PHASE bytes={source_len} stage={stage} retained_owners={} requests={requests} captures={captures} reuses={} rejected_captures={rejected}",
            owners.len(),
            requests.saturating_sub(captures),
        );
        let (requests, builds, rows, alternatives) =
            bindings.deferred_namespace_index.trace_counts();
        eprintln!(
            "SOURCE_DEFERRED_INDEX_PHASE bytes={source_len} stage={stage} requests={requests} builds={builds} reuses={} scanned_rows={rows} scanned_alternatives={alternatives}",
            requests.saturating_sub(builds),
        );
    }
}

#[cfg(any(test, debug_assertions))]
struct SourceAnalysisTrace {
    pub(super) phase: Option<std::time::Instant>,
    pub(super) lookup: Option<(u64, u128)>,
    queries: Option<original_command_table::OriginalSourceQueryTotals>,
}

#[cfg(any(test, debug_assertions))]
impl SourceAnalysisTrace {
    pub(super) fn start() -> Self {
        let phase_start = tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
            .is_some()
            .then(std::time::Instant::now);
        let lookup_start = phase_start.map(|_| original_command_table::baseline_lookup_totals());
        let query_start =
            phase_start.map(|_| original_command_table::original_source_query_totals());
        Self {
            phase: phase_start,
            lookup: lookup_start,
            queries: query_start,
        }
    }

    pub(super) fn phase(&self, stage: &str, source_len: usize, bindings: &SourceCommandBindings) {
        trace_source_analysis(self.phase, stage, source_len, bindings);
        trace_source_original_queries(self.queries.as_ref(), stage, source_len);
    }

    fn phase_with_lookup(&self, stage: &str, source_len: usize, bindings: &SourceCommandBindings) {
        self.phase(stage, source_len, bindings);
        trace_source_lookup(self.lookup, stage, source_len);
    }
}

#[cfg(any(test, debug_assertions))]
fn trace_source_lookup(before: Option<(u64, u128)>, stage: &str, source_len: usize) {
    if let Some((before_calls, before_nanos)) = before {
        let (calls, nanos) = original_command_table::baseline_lookup_totals();
        eprintln!(
            "SOURCE_LOOKUP_PHASE bytes={} stage={} calls={} ms={}",
            source_len,
            stage,
            calls.saturating_sub(before_calls),
            nanos.saturating_sub(before_nanos) / 1_000_000
        );
    }
}

#[cfg(any(test, debug_assertions))]
fn trace_source_original_queries(
    before: Option<&original_command_table::OriginalSourceQueryTotals>,
    stage: &str,
    source_len: usize,
) {
    let Some(before) = before else {
        return;
    };
    let after = original_command_table::original_source_query_totals();
    let provider = after.provider;
    eprintln!(
        "SOURCE_PROVIDER_VALIDATION_PHASE bytes={source_len} stage={stage} scope=cumulative-inclusive calls={} ms={} required_rows={} dependency_rows={} optional_rows={}",
        provider.calls.saturating_sub(before.provider.calls),
        provider.nanos.saturating_sub(before.provider.nanos) / 1_000_000,
        provider
            .required_rows
            .saturating_sub(before.provider.required_rows),
        provider
            .dependency_rows
            .saturating_sub(before.provider.dependency_rows),
        provider
            .optional_rows
            .saturating_sub(before.provider.optional_rows),
    );
    for (query, (before, after)) in original_command_table::ORIGINAL_GEOMETRY_QUERIES
        .iter()
        .zip(before.geometry.iter().zip(after.geometry.iter()))
    {
        eprintln!(
            "SOURCE_ORIGINAL_GEOMETRY_PHASE bytes={source_len} stage={stage} query={query} scope=cumulative-inclusive calls={} ms={} world_clones={} clone_us={}",
            after.calls.saturating_sub(before.calls),
            after.nanos.saturating_sub(before.nanos) / 1_000_000,
            after.world_clones.saturating_sub(before.world_clones),
            after.clone_nanos.saturating_sub(before.clone_nanos) / 1_000,
        );
    }
}

impl SourceCommandBindings {
    pub(crate) fn retain_analyser_source_settings(
        &mut self,
        settings: Arc<crate::analyser::source_settings::OriginalAnalyserSourceSettings>,
    ) {
        self.analyser_source_settings = Some(settings);
    }

    pub(crate) fn retained_analyser_source_settings(
        &self,
    ) -> Option<&crate::analyser::source_settings::OriginalAnalyserSourceSettings> {
        self.analyser_source_settings.as_deref()
    }

    pub(crate) fn original_analysis_owner(&self) -> Option<&Arc<OriginalSourceAnalysisOwner>> {
        self.original_analysis_owner.as_ref()
    }

    /// Borrow the original analysis availability ingress, including terminal
    /// supplied-missing ownership. This creates no source or execution entry.
    pub(crate) fn source_metadata_owner(
        &self,
    ) -> &crate::registry_invocation::OwnedInvocationMetadataContext {
        &self.final_state.baseline.metadata_context
    }

    /// Interpret a fresh authoring entry using the selected registry profile,
    /// or the compiler convenience driver's Tcl 9.0 native target when none
    /// is selected. Use [`Self::analyse_with_options`] for another runtime or
    /// interpreter history; assistance metadata alone is not an entry proof.
    #[must_use]
    pub fn analyse(
        source: &str,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) -> Self {
        Self::analyse_in_namespace(source, "::", config, registry)
    }

    /// Interpret an isolated source instance in its actual command namespace.
    #[must_use]
    pub fn analyse_in_namespace(
        source: &str,
        namespace: &str,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) -> Self {
        Self::analyse_in_namespace_with_options(
            source,
            namespace,
            config,
            registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(crate::environment_ingress::authoring_invocation_dialect(
                    registry, None, config,
                )),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..SourceAnalysisOptions::default()
            },
        )
    }

    /// Interpret source under explicitly supplied entry and loader contracts.
    #[must_use]
    pub fn analyse_with_options(
        source: &str,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Self {
        Self::analyse_in_namespace_with_options(source, "::", config, registry, options)
    }

    /// Interpret a source instance with its namespace and explicit entry facts.
    #[must_use]
    pub fn analyse_in_namespace_with_options(
        source: &str,
        namespace: &str,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Self {
        let frame = if namespace == "::" {
            crate::var_resolve::VariableExecutionFrame::Global
        } else {
            crate::var_resolve::VariableExecutionFrame::Namespace(namespace.to_owned())
        };
        Self::analyse_in_frame_with_options(source, &frame, config, registry, options)
    }

    /// Interpret source in a proved activation rather than inferring variable
    /// ownership from a namespace spelling.
    #[must_use]
    pub fn analyse_in_frame_with_options(
        source: &str,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Self {
        Self::analyse_image_in_frame_with_options(
            &tcl_lexer::SourceImage::document(source),
            frame,
            config,
            registry,
            options,
        )
        .expect("document source is valid UTF-8")
    }

    /// Interpret a checked text view while retaining the original source channel.
    /// Opaque native source has no Unicode analysis projection.
    #[must_use]
    pub fn analyse_image_in_frame_with_options(
        image: &tcl_lexer::SourceImage,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Option<Self> {
        Self::analyse_image_in_frame_with_purpose(
            image,
            frame,
            config,
            registry,
            options,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
    }

    /// Interpret a producer-selected whole input or nested script while keeping
    /// the complete parent config for original word and metadata correspondence.
    /// Parser purpose supplies no entered source/frame/Native implementation.
    #[must_use]
    pub fn analyse_image_in_frame_with_purpose(
        image: &tcl_lexer::SourceImage,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
        purpose: tcl_lexer::SourceSegmentationPurpose,
    ) -> Option<Self> {
        let source = image.try_text().ok()?;
        Some(Self::analyse_source_in_frame_with_options(
            source,
            image,
            frame,
            config,
            registry,
            options,
            SourceInterpretationEntry {
                purpose,
                logical_conditional_source_control: false,
                native_formals: None,
            },
        ))
    }

    pub(crate) fn analyse_image_with_original_native_formals(
        image: &tcl_lexer::SourceImage,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
        purpose: tcl_lexer::SourceSegmentationPurpose,
        native_formals: Option<&Arc<OriginalNativeCompilerFormals>>,
    ) -> Option<Self> {
        let source = image.try_text().ok()?;
        Some(Self::analyse_source_in_frame_with_options(
            source,
            image,
            frame,
            config,
            registry,
            options,
            SourceInterpretationEntry {
                purpose,
                logical_conditional_source_control: false,
                native_formals,
            },
        ))
    }

    /// Interpret the fixed package-index source survey's whole document under
    /// its positively retained Logical input, entering first-match conditional
    /// bodies as C Tcl selects them. This interpretation is never memoised, so
    /// no general Logical consumer can receive it; other inputs decline.
    pub(crate) fn analyse_package_index_survey(
        source: &str,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Option<Self> {
        options.retained_logical_source_input(registry, Some(config))?;
        Some(Self::analyse_source_in_frame_with_options(
            source,
            &tcl_lexer::SourceImage::document(source),
            &crate::var_resolve::VariableExecutionFrame::Global,
            config,
            registry,
            options,
            SourceInterpretationEntry {
                purpose: tcl_lexer::SourceSegmentationPurpose::WholeInput,
                logical_conditional_source_control: true,
                native_formals: None,
            },
        ))
    }

    fn initial_source_analysis_bindings(
        options: &SourceAnalysisOptions<'_>,
        logical_conditional_source_control: bool,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        Self {
            analyser_source_settings: options.analyser_source_settings.cloned().or_else(|| {
                options
                    .source_point_entry
                    .map(OriginalSourcePointEntry::analyser_source_settings)
            }),
            logical_conditional_source_control,
            compilation_scope: options.compilation_scope,
            unentered: options.unentered,
            lexer_config: Some(config),
            ..Self::default()
        }
    }

    fn initial_original_analysis_owner(
        state: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        options: SourceAnalysisOptions<'_>,
    ) -> Option<Arc<OriginalSourceAnalysisOwner>> {
        state.current_source_origin.as_ref().map(|source| {
            Arc::new(OriginalSourceAnalysisOwner {
                source_arguments_empty: options.incoming_formals.is_empty(),
                source: Arc::clone(source),
                metadata: state.baseline.metadata_context.clone(),
                config,
            })
        })
    }

    fn update_source_interpretation_frame(
        state: &mut ModuleCommandBindings,
        frame: &crate::var_resolve::VariableExecutionFrame,
    ) {
        if &state.variable_frame != frame {
            state.source_variables = source_variables_in_frame(&state.source_variables, frame);
            state.variable_frame = frame.clone();
        }
    }

    /// The supplier runs the whole document inside a procedure (a loader's
    /// `eval` or `source`): a fresh activation over the fresh interpreter,
    /// named after this source origin so no document procedure can share it,
    /// holding the supplier's locals and `global` links before the first line.
    /// Otherwise the supplied frame is the entry activation.
    fn entry_activation(
        state: &mut ModuleCommandBindings,
        options: &SourceAnalysisOptions<'_>,
        frame: &crate::var_resolve::VariableExecutionFrame,
        namespace_key: &SourceNamespaceKey,
        registry: &CommandRegistry,
    ) -> crate::var_resolve::VariableExecutionFrame {
        let Some(entry) = options
            .entry_procedure_frame()
            .filter(|_| matches!(frame, crate::var_resolve::VariableExecutionFrame::Global))
        else {
            return frame.clone().with_namespace_identity(namespace_key.clone());
        };
        let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: source_frame_namespace(frame).to_owned(),
            identity: super::source_origin::source_activation_name(
                state.current_source_origin.as_ref(),
                "entry-procedure",
                0,
            ),
        }
        .with_namespace_identity(namespace_key.clone());
        let mut variables = state.source_variables.enter_called_frame(&frame);
        for (name, value) in entry.locals() {
            match value {
                Some(value) => variables.bind_literal_incoming(name, value, registry),
                None => variables.bind_unknown_incoming(name, registry),
            }
        }
        for name in entry.globals() {
            crate::variable_bindings::bind_entry_global_link(&mut variables, name, registry);
        }
        state.source_variables = Arc::new(variables);
        state.variable_frame = frame.clone();
        frame
    }

    fn analyse_source_in_frame_with_options(
        source: &str,
        image: &tcl_lexer::SourceImage,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
        entry: SourceInterpretationEntry<'_>,
    ) -> Self {
        let SourceInterpretationEntry {
            purpose,
            logical_conditional_source_control,
            native_formals,
        } = entry;
        let native_formals = native_formals.filter(|_| options.source_point_entry.is_none());
        #[cfg(any(test, debug_assertions))]
        let tracing = SourceAnalysisTrace::start();
        let config = options.native_lexer_config(config);
        let frame = options
            .source_point_entry
            .map_or(frame, |entry| entry.frame());
        let options = SourceAnalysisOptions {
            stock_tclsh_stdout: options.stock_tclsh_stdout.filter(|_| {
                matches!(
                    frame.layout(),
                    crate::var_resolve::VariableExecutionFrame::Global
                ) && purpose == tcl_lexer::SourceSegmentationPurpose::WholeInput
                    && image.channel() == tcl_lexer::SourceChannel::Document
            }),
            ..options
        };
        // The survey-scoped interpretation is never shared through the memo.
        let memo_key = if logical_conditional_source_control || native_formals.is_some() {
            None
        } else {
            source_analysis_cache::SourceAnalysisCacheKey::at_entry(
                image, frame, config, registry, options, purpose,
            )
        };
        #[cfg(test)]
        let memo_key = memo_key.filter(|_| !super::binding_value_fixpoint_probe::active());
        if let Some(mut bindings) = memo_key.as_ref().and_then(source_analysis_cache::lookup) {
            // Reused source facts retain a distinct attachment to this analysis call.
            bindings.original_analysis_owner = bindings
                .original_analysis_owner
                .as_deref()
                .cloned()
                .map(Arc::new);
            #[cfg(any(test, debug_assertions))]
            trace_source_analysis(tracing.phase, "memo", source.len(), &bindings);
            return bindings;
        }
        let namespace = source_frame_namespace(frame);
        let mut bindings = Self::initial_source_analysis_bindings(
            &options,
            logical_conditional_source_control,
            config,
        );
        let mut state = initial_source_interpretation_state(image, config, registry, options);
        bindings
            .root_origin
            .clone_from(&state.current_source_origin);
        bindings.original_analysis_owner =
            Self::initial_original_analysis_owner(&state, config, options);
        let namespace_key = source_entry_namespace_key(&options, frame, namespace);
        let frame = Self::entry_activation(&mut state, &options, frame, &namespace_key, registry);
        // This API's supplied frame is the entry activation. It is not a
        // nested call whose final state should restore the initial model frame.
        Self::update_source_interpretation_frame(&mut state, &frame);
        for name in options.incoming_formals {
            Arc::make_mut(&mut state.source_variables).bind_unknown_incoming(name, registry);
        }
        #[cfg(any(test, debug_assertions))]
        tracing.phase("initial", source.len(), &bindings);
        let context = root_source_execution_context(
            &frame,
            namespace,
            &namespace_key,
            config,
            registry,
            options,
            purpose,
        );
        if let Some(owner) = native_formals {
            let scope = owner.bind_scope(&state, context, options.incoming_formals);
            Arc::make_mut(&mut state.source_variables).original_native_compiler_formals = scope;
        }
        if options.unentered {
            bindings.final_state = Arc::new(state);
            return bindings;
        }
        let root_outcomes = bindings.walk_source(source, 0, &mut state, &context);
        if crate::analysis_control::interrupted() {
            return Self::default();
        }
        #[cfg(any(test, debug_assertions))]
        tracing.phase_with_lookup("walk", source.len(), &bindings);
        bindings.conditional_source.root_outcomes = Some(Arc::new(root_outcomes.clone()));
        bindings.original_completed_root_state = options
            .source_point_entry
            .is_none()
            .then(|| root_outcomes.completed_normal_world())
            .flatten();
        if let Some(normal) = root_outcomes.normal {
            state = *normal;
        }
        if options.compilation_scope == tcl_runtime_api::SourceCompilationScope::WholeModule {
            bindings.analyse_deferred_entries(&state, config, registry);
        }
        if crate::analysis_control::interrupted() {
            return Self::default();
        }
        #[cfg(any(test, debug_assertions))]
        tracing.phase_with_lookup("deferred", source.len(), &bindings);
        bindings.invalidate_unpositioned_projections();
        bindings.final_state = Arc::new(state);
        if options.compilation_scope == tcl_runtime_api::SourceCompilationScope::WholeModule {
            bindings.retain_jim_wrapper_future_bodies(config, registry);
        }
        bindings.retain_completed_source_catalogues(config, registry, memo_key);
        #[cfg(any(test, debug_assertions))]
        tracing.phase("complete", source.len(), &bindings);
        bindings
    }

    /// The catalogues and memo entry a completed interpretation leaves.
    fn retain_completed_source_catalogues(
        &mut self,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        memo_key: Option<source_analysis_cache::SourceAnalysisCacheKey>,
    ) {
        self.retain_logical_receiver_source_catalogues(config, registry);
        self.retain_original_declaration_source_operand_layouts(registry);
        if let Some(key) = memo_key {
            source_analysis_cache::retain(key, self);
        }
    }

    /// Interpret a materialised script using the actual before-call state and
    /// selected activation. Missing dispatch points cannot create entry proof.
    #[must_use]
    pub fn analyse_script_at_site(
        &self,
        script: &str,
        source_base: u32,
        callsite: u32,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let mut points = self.dispatch_points_at(callsite);
        let first = points.next()?;
        let realm = first.realm;
        let mut state = first.state.clone();
        for point in points {
            if point.realm != realm {
                return None;
            }
            state.join(&point.state);
        }
        let selected = state.source_variables.in_frame(frame);
        let config = selected
            .invocation_dialect
            .map_or(config, |native| config.with_grammar(native.lexer_grammar));
        let namespace = selected.namespace.clone();
        let mut bindings = Self {
            lexer_config: Some(config),
            root_origin: state.current_source_origin.clone(),
            deferred: self.deferred.clone(),
            ..Self::default()
        };
        let compilation = state.baseline.native_compilation;
        let root_outcomes = bindings.walk_source(
            script,
            source_base,
            &mut state,
            &SourceExecutionContext {
                original_logical_body: None,
                control:
                    crate::command_binding::source_execution_control::SourceExecutionControl::new(
                        compilation,
                        tcl_lexer::SourceSegmentationPurpose::NestedScript,
                        SourceBodyEntryPurpose::Selected,
                        realm,
                    ),
                compilation_snapshot: None,
                selected_compilation: None,
                original_variable_compilation: None,
                namespace: &namespace,
                frame,
                config,
                registry,
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
                namespace_key: selected.namespace_identity.as_ref(),
                expression_source: None,
            },
        );
        bindings.original_completed_root_state = root_outcomes.completed_normal_world();
        if let Some(normal) = root_outcomes.normal {
            state = *normal;
        }
        bindings.invalidate_unpositioned_projections();
        bindings.final_state = Arc::new(state);
        bindings.retain_logical_receiver_source_catalogues(config, registry);
        bindings.retain_original_declaration_source_operand_layouts(registry);
        Some(bindings)
    }
}
