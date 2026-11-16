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

//! Conditional operand layouts projected from the lookup world retained at a source point.

use super::binding_projection::{
    CommandTargetLookup, source_binding_from_original_input_in, source_binding_projection_in,
};
use super::command_identity::CommandAllocationSite;
use super::conditional_body::{SourceConditionalBodyEntry, SourceDeclaredReceiverBodyEntry};
use super::namespace_context::SourceNamespaceKey;
use super::{
    BindingKind, ModuleCommandBindings, OriginalCompilationLookupAdvice, RBS_OPERAND_TRACE_ACTIVE,
    RbsOperandTraceScope, SourceCommandTarget, SourceInvocationBinding, SourceLookupSnapshot,
    literal_object_pool, original_compiler_effects, root_owner_work, source_analysis_entry,
};
use std::hash::Hash;
use std::sync::{Arc, OnceLock};
use tcl_registry::CommandRegistry;

impl OriginalCompilationLookupAdvice {
    /// An opaque original command world can contain unrelated handlers. A
    /// retained candidate then supplies diagnostic layout, not its operation.
    pub(crate) fn has_opaque_handler_alternatives(&self) -> bool {
        self.snapshot.state.has_opaque_domain()
    }

    /// Unclosed candidate coverage remains an explicit source-only residual.
    pub(crate) fn has_unclosed_source_coverage(&self) -> bool {
        !self.closed_lookup()
            || self.has_opaque_handler_alternatives()
            || self.snapshot.state.source_step_observed()
            || !self.snapshot.state.command_observers.is_quiet()
    }

    /// Independently selected Authored/Standalone model, never a missing Native input.
    pub(crate) fn is_authored_standalone_source(&self) -> bool {
        let baseline = &self.snapshot.state.baseline;
        baseline.metadata_context.is_standalone()
            && baseline.native_entry.is_none()
            && baseline.logical_source_input.is_none()
            && baseline.vendor_source_input.is_none()
            && baseline.hosted_execution_context.is_none()
            && !baseline.unknown_entry
    }

    /// Original candidate lookup is complete in its independently known namespace.
    pub(crate) fn closed_lookup(&self) -> bool {
        crate::registry_invocation::host_structure_observer::flag("native.known", !self.unknown)
            && crate::registry_invocation::host_structure_observer::flag(
                "native.present",
                !self.may_be_absent,
            )
            && crate::registry_invocation::host_structure_observer::flag(
                "native.frame_namespace",
                crate::var_resolve::ResolveContext::default()
                    .in_frame(&self.snapshot.state.variable_frame)
                    .namespace_known,
            )
            && crate::registry_invocation::host_structure_observer::flag(
                "native.namespace_match",
                self.snapshot
                    .state
                    .variable_frame
                    .namespace_identity()
                    .is_none_or(|namespace| namespace == &self.namespace),
            )
    }

    /// Closed command layout in the positively retained Logical source world.
    /// Namespace and original target consensus are independent of a physical
    /// variable frame; this cannot certify native lookup or an entered frame.
    /// [`Self::closed_logical_source_lookup`] for a hosted F5 rule's advisory
    /// walk: the retained hosted input, not a Logical one, owns the lookup.
    pub(crate) fn closed_hosted_source_lookup(&self) -> bool {
        let state = &self.snapshot.state;
        state
            .baseline
            .vendor_source_input
            .as_ref()
            .is_some_and(|input| source_analysis_entry::source_input_dialect(input) == self.dialect)
            && !self.unknown
            && !self.may_be_absent
            && !self.targets.is_empty()
            && !state.has_opaque_domain()
            && matches!(&self.namespace, SourceNamespaceKey::Authored(_))
            && state.namespaces.contains(&self.namespace)
    }

    pub(crate) fn closed_logical_source_lookup(&self) -> bool {
        let state = &self.snapshot.state;
        state
            .logical_source_name_advice_input()
            .is_some_and(|input| source_analysis_entry::source_input_dialect(input) == self.dialect)
            && !self.unknown
            && !self.may_be_absent
            && !self.targets.is_empty()
            && !state.has_opaque_domain()
            && matches!(&self.namespace, SourceNamespaceKey::Authored(_))
            && state.namespaces.contains(&self.namespace)
    }

    /// A declaration's conditional receiver-body traits use its authentic
    /// source namespace. The future private runtime namespace stays unknown.
    pub(crate) fn closed_receiver_trait_lookup(
        &self,
        entry: &SourceDeclaredReceiverBodyEntry,
    ) -> bool {
        // naming.tcloo.original-declared-receiver-caller-traits
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
        let state = &self.snapshot.state;
        !self.unknown
            && !self.may_be_absent
            && !self.targets.is_empty()
            && self.frame() == entry.preview_frame()
            && self.namespace() == entry.declaration_namespace_context()
            && !self.has_opaque_handler_alternatives()
            && !state.source_step_observed()
            && state.command_observers.is_quiet()
            && self.targets.iter().all(|target| {
                target.registry_backed
                    && target.kind == BindingKind::Builtin
                    && !state.source_execution_observed(target.identity.as_ref())
                    && !state.runtime_execution_observed(target.identity.as_ref())
            })
    }

    /// Original procedure-body syntax for a conditional parameter report.
    /// Actual entry, Native lookup and successful alias operations stay open.
    pub(crate) fn closed_body_parameter_source_lookup(
        &self,
        entry: &SourceConditionalBodyEntry,
    ) -> bool {
        self.closed_body_parameter_source_parts(
            entry.owns_source(&self.site.source, self.site.offset),
            entry.frame(),
            entry.namespace_context(),
        )
    }

    pub(super) fn closed_body_parameter_source_parts(
        &self,
        owns_source: bool,
        frame: &crate::var_resolve::VariableExecutionFrame,
        namespace: Option<&SourceNamespaceKey>,
    ) -> bool {
        let state = &self.snapshot.state;
        self.closed_lookup()
            && state.logical_source_name_advice_input().is_none()
            && owns_source
            && self.frame() == frame
            && Some(self.namespace()) == namespace
            && !self.targets.is_empty()
            && !self.has_opaque_handler_alternatives()
            && !state.source_step_observed()
            && state.command_observers.is_quiet()
            && self.targets.iter().all(|target| {
                target.registry_backed
                    && target.kind == BindingKind::Builtin
                    && target.prepended.is_empty()
                    && !state.source_execution_observed(target.identity.as_ref())
                    && !state.runtime_execution_observed(target.identity.as_ref())
            })
    }

    /// Reborrow complete original words only after the exact declaration row,
    /// metadata owner and actual body/frame join. No executed read is issued.
    pub(crate) fn original_body_parameter_source_words(
        &self,
        tokens: &crate::ir::CommandTokens,
        entry: &SourceConditionalBodyEntry,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) -> Option<Vec<tcl_lexer::NativeWord>> {
        (self.config == config).then_some(())?;
        self.closed_body_parameter_source_lookup(entry)
            .then_some(())?;
        self.original_declaration_metadata_context(registry, tokens)
            .ok()?;
        let original = self.site.source.original_lexical_command(
            tokens.words(),
            self.site.offset,
            self.config,
        )?;
        Some(original.native_words().to_vec())
    }

    pub(crate) fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.snapshot.state.variable_frame
    }
    pub(crate) fn namespace(&self) -> &SourceNamespaceKey {
        &self.namespace
    }

    pub(crate) fn expression_pool_state(
        &self,
    ) -> tcl_registry::conditional_expression::ConditionalExpressionPoolState {
        use tcl_registry::conditional_expression::ConditionalExpressionPoolState as Pool;
        let state = &self.snapshot.state;
        if !state.opaque_domain
            && !state.source_variables.dynamic_traces
            && state.ordinary_literal_pool.as_ref().is_some_and(
                literal_object_pool::SourceOrdinaryLiteralPool::initial_numeric_representations,
            )
        {
            Pool::InitialAuthoredPool
        } else {
            Pool::Unknown
        }
    }
    pub(crate) fn targets(&self) -> &[SourceCommandTarget] {
        &self.targets
    }

    pub(crate) const fn dialect(&self) -> tcl_registry::InvocationDialect {
        self.dialect
    }

    pub(crate) fn realm(&self) -> tcl_dialect::model::InvocationRealm {
        self.snapshot.realm
    }

    /// Original variable context for declaration-only name projections.
    pub(crate) fn original_variable_context(&self) -> &Arc<crate::var_resolve::ResolveContext> {
        &self.snapshot.state.source_variables
    }

    pub(crate) fn alias_frame(&self) -> tcl_registry::VariableAliasFrame {
        self.snapshot.state.source_variables.alias_frame()
    }
}

impl SourceLookupSnapshot {
    pub(super) fn new(state: ModuleCommandBindings) -> Self {
        Self::in_realm(state, tcl_dialect::model::InvocationRealm::RuleLoader)
    }

    pub(super) fn in_realm(
        state: ModuleCommandBindings,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Self {
        Self {
            realm,
            state,
            fingerprint: OnceLock::new(),
            compiler_name_effects:
                original_compiler_effects::SourceCompilerNameEffectsCache::default(),
        }
    }

    pub(super) fn fingerprint(&self) -> u64 {
        *self.fingerprint.get_or_init(|| {
            #[cfg(any(test, debug_assertions))]
            let _timing = root_owner_work::Timing::at(root_owner_work::Query::Fingerprint);
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.state.hash(&mut hasher);
            self.realm.hash(&mut hasher);
            std::hash::Hasher::finish(&hasher)
        })
    }
}

impl PartialEq for SourceLookupSnapshot {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(any(test, debug_assertions))]
        let _timing = root_owner_work::Timing::at(root_owner_work::Query::Equality);
        self.realm == other.realm && self.state == other.state
    }
}

impl Eq for SourceLookupSnapshot {}

impl Hash for SourceLookupSnapshot {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.fingerprint().hash(state);
    }
}

impl SourceInvocationBinding {
    /// MAY layout from the genuine Authored/Standalone world at this source point.
    /// Present Native, hosted or supplied-source ownership cannot enter this route.
    pub(crate) fn original_authored_source_layout(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalCompilationLookupAdvice> {
        let retained = self.retained_registry_for_tokens(tokens)?;
        if retained.snapshot().semantic_key() != registry.snapshot().semantic_key() {
            return None;
        }
        let point = self.lookup_state.as_ref()?;
        let baseline = &point.state.baseline;
        if !baseline.metadata_context.is_standalone()
            || baseline.native_entry.is_some()
            || baseline.logical_source_input.is_some()
            || baseline.vendor_source_input.is_some()
            || baseline.hosted_execution_context.is_some()
            || baseline.unknown_entry
        {
            return None;
        }
        let advice = original_site_operand_layout_advice(
            self.invocation_site()?,
            tokens,
            point,
            &self.lookup_namespace_key,
            self.original_lexer_config_for_tokens(tokens)?,
        )?;
        if !advice.is_authored_standalone_source()
            || advice
                .targets()
                .iter()
                .any(|target| !target.prepended.is_empty())
        {
            return None;
        }
        Some(advice)
    }

    /// Candidate lexical layout of this original static head, captured before
    /// its arguments ran. Source geometry and logical grammar are retained;
    /// replaced/non-native implementations cannot borrow catalogue contracts.
    /// This query supplies no normal handler, entered body or absence proof.
    pub(crate) fn original_compilation_lookup_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        let snapshot = self.compiler_lookup_state.as_ref()?;
        let site = CommandAllocationSite {
            source: Arc::clone(snapshot.state.current_source_origin.as_ref()?),
            offset: tokens.words().first()?.source().span.start(),
        };
        original_site_operand_layout_advice(
            &site,
            tokens,
            snapshot,
            &self.lookup_namespace_key,
            self.original_lexer_config_for_tokens(tokens)?,
        )
    }

    /// Conditional operand layout at the original source point. Declaration
    /// and compiler observations keep their existing purposes; only their
    /// absence permits the separate positively Logical reached-source route.
    /// This supplies no compiler preparation, Native handler or Normal result.
    pub(crate) fn original_conditional_operand_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<OriginalCompilationLookupAdvice> {
        trace_rbs_operand_option(
            "root.declaration",
            self.declaration_operand_layout_advice(tokens),
        )
        .or_else(|| {
            trace_rbs_operand_option(
                "root.compiler",
                self.original_compilation_lookup_advice(tokens),
            )
        })
        .or_else(|| {
            trace_rbs_operand_option(
                "root.logical",
                self.original_logical_source_operand_layout_advice(tokens, registry),
            )
        })
        .or_else(|| {
            trace_rbs_operand_option(
                "root.hosted",
                self.original_hosted_source_operand_layout_advice(tokens, registry),
            )
        })
    }

    /// Source roles from the captured post-operand hosted lookup world: the
    /// advisory walk of a hosted F5 rule, never an entered handler.
    fn original_hosted_source_operand_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<OriginalCompilationLookupAdvice> {
        if self.compiler_lookup_state.is_some() || self.declaration_layout_observations.is_some() {
            return None;
        }
        let metadata = self
            .original_source_metadata_for_advice(tokens, registry)
            .ok()??;
        let input = metadata.source_analysis_input()?;
        input.has_hosted_source_name_context().then_some(())?;
        self.original_invocation_metadata_for_input(tokens, input, registry)?;
        let snapshot = self.lookup_state.as_ref()?;
        let state = &snapshot.state;
        if state.baseline.vendor_source_input.as_ref() != Some(input)
            || self.variable_context.as_ref() != state.source_variables.as_ref()
            || self.variable_frame != state.variable_frame
            || self.lookup_namespace != self.lookup_namespace_key.display().unwrap_or_default()
            || self.unknown
            || self.may_be_absent
        {
            return None;
        }
        let variables = &state.source_variables;
        variables
            .namespace_identity
            .as_ref()
            .is_some_and(|namespace| namespace == &self.lookup_namespace_key)
            .then_some(())?;
        let advice = original_site_operand_layout_advice(
            self.invocation_site()?,
            tokens,
            snapshot,
            &self.lookup_namespace_key,
            self.original_lexer_config_for_tokens(tokens)?,
        )?;
        (advice.closed_hosted_source_lookup() && advice.targets == self.targets).then_some(advice)
    }

    /// Source roles from the captured post-operand Logical lookup world.
    /// A refused present compiler/declaration owner cannot use this sibling.
    pub(super) fn original_logical_source_operand_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<OriginalCompilationLookupAdvice> {
        if self.compiler_lookup_state.is_some() || self.declaration_layout_observations.is_some() {
            return None;
        }
        let metadata = self
            .original_source_metadata_for_advice(tokens, registry)
            .ok()??;
        metadata.permits_logical_source_names().then_some(())?;
        let input = metadata.source_analysis_input()?;
        self.original_invocation_metadata_for_input(tokens, input, registry)?;
        let snapshot = self.lookup_state.as_ref()?;
        let state = &snapshot.state;
        if state.logical_source_name_advice_input() != Some(input)
            || self.variable_context.as_ref() != state.source_variables.as_ref()
            || self.variable_frame != state.variable_frame
            || self.lookup_namespace != self.lookup_namespace_key.display().unwrap_or_default()
            || self.unknown
            || self.may_be_absent
        {
            return None;
        }
        let variables = &state.source_variables;
        let namespace_matches = variables.namespace_identity.as_ref().map_or_else(
            || {
                matches!(&self.lookup_namespace_key,
                SourceNamespaceKey::Authored(namespace) if namespace == &variables.namespace)
            },
            |namespace| namespace == &self.lookup_namespace_key,
        );
        namespace_matches.then_some(())?;
        let advice = original_site_operand_layout_advice(
            self.invocation_site()?,
            tokens,
            snapshot,
            &self.lookup_namespace_key,
            self.original_lexer_config_for_tokens(tokens)?,
        )?;
        (advice.closed_logical_source_lookup() && advice.targets == self.targets).then_some(advice)
    }
}

/// Validate original geometry and logical word rules before projecting candidate layout.
pub(super) fn original_operand_layout_advice(
    binding: &SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalCompilationLookupAdvice> {
    original_site_operand_layout_advice(
        binding.invocation_site()?,
        tokens,
        snapshot,
        namespace,
        config,
    )
}

pub(super) fn original_declaration_call_layout_advice(
    binding: &SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalCompilationLookupAdvice> {
    original_site_layout_advice(
        binding.invocation_site()?,
        tokens,
        snapshot,
        namespace,
        OriginalOperandLayoutPurpose::DeclarationTargets,
        config,
    )
}

// Logical source declarations have no physical implementation allocation.
// This projection shares the exact original geometry and source world checks,
// while the allocation-bearing declaration/formal purpose stays separate.
pub(super) fn original_logical_declaration_call_layout_advice(
    binding: &SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalCompilationLookupAdvice> {
    original_site_layout_advice(
        binding.invocation_site()?,
        tokens,
        snapshot,
        namespace,
        OriginalOperandLayoutPurpose::LogicalDeclarationTargets,
        config,
    )
}

pub(super) fn original_site_operand_layout_advice(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalCompilationLookupAdvice> {
    original_site_layout_advice(
        site,
        tokens,
        snapshot,
        namespace,
        OriginalOperandLayoutPurpose::RegistryMetadata,
        config,
    )
}

#[derive(Clone, Copy)]
enum OriginalOperandLayoutPurpose {
    RegistryMetadata,
    DeclarationTargets,
    LogicalDeclarationTargets,
}

#[cfg(debug_assertions)]
impl RbsOperandTraceScope {
    pub(super) fn enter() -> Self {
        let previous = RBS_OPERAND_TRACE_ACTIVE.with(|active| active.replace(true));
        let _ = trace_rbs_operand_option("scope.begin", Some(()));
        Self(previous)
    }
}

#[cfg(debug_assertions)]
impl Drop for RbsOperandTraceScope {
    fn drop(&mut self) {
        RBS_OPERAND_TRACE_ACTIVE.with(|active| active.set(self.0));
    }
}

pub(super) fn trace_rbs_operand_option<T>(stage: &str, value: Option<T>) -> Option<T> {
    #[cfg(debug_assertions)]
    if RBS_OPERAND_TRACE_ACTIVE.with(std::cell::Cell::get)
        && tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some()
    {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static RECORDS: AtomicUsize = AtomicUsize::new(0);
        if RECORDS.fetch_add(1, Ordering::Relaxed) < 1024 {
            eprintln!(
                "RBS_OPERAND_LAYOUT stage={stage} present={}",
                value.is_some()
            );
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = stage;
    value
}

fn original_site_layout_advice(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &SourceNamespaceKey,
    purpose: OriginalOperandLayoutPurpose,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalCompilationLookupAdvice> {
    if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
        return trace_rbs_operand_option("site.original_argv", None);
    }
    let mut dialect = trace_rbs_operand_option(
        "site.dialect",
        snapshot.state.source_variables.invocation_dialect,
    )?;
    let logical_source = snapshot
        .state
        .logical_source_name_advice_input()
        .is_some_and(|input| {
            input.lexer_config() == config
                && source_analysis_entry::source_input_dialect(input) == dialect
        });
    // Logical declaration targets and Native formal allocations consume distinct
    // purposes, even when the readonly model retains an internal source allocation.
    if (matches!(
        purpose,
        OriginalOperandLayoutPurpose::LogicalDeclarationTargets
    ) && !logical_source)
        || (matches!(purpose, OriginalOperandLayoutPurpose::DeclarationTargets) && logical_source)
    {
        return trace_rbs_operand_option("site.purpose", None);
    }
    // Conditional Logical command layout uses its retained source namespace
    // and lookup world. It does not require or issue a physical variable frame.
    if snapshot.state.current_source_origin.as_ref() != Some(&site.source)
        || (!logical_source
            && matches!(
                snapshot.state.variable_frame,
                crate::var_resolve::VariableExecutionFrame::Unknown
            ))
    {
        return trace_rbs_operand_option("site.origin_or_frame", None);
    }
    let original = trace_rbs_operand_option(
        "site.original_command",
        site.source
            .original_lexical_command(tokens.words(), site.offset, config),
    )?;
    let head = trace_rbs_operand_option("site.original_head", original.native_words().first())?;
    if snapshot
        .state
        .source_variables
        .execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        .is_some_and(|policy| original_standalone_document_naming(snapshot, head, policy))
    {
        // Only returned source metadata adopts the retained Document grammar.
        // The model version, naming policy, baseline and Native entry stay put.
        dialect.lexer_grammar = config.grammar_over(dialect.lexer_grammar);
        dialect.word_values =
            tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
    }
    let projection = trace_rbs_operand_option(
        "site.static_projection",
        original_static_layout_projection(
            snapshot,
            namespace,
            head,
            config,
            dialect,
            logical_source,
        ),
    )?;
    #[cfg(test)]
    trace_original_site_projection(site.offset, logical_source, snapshot, &projection);
    if projection.targets.is_empty()
        || projection.targets.iter().any(|target| {
            !target.registry_backed
                && match purpose {
                    OriginalOperandLayoutPurpose::RegistryMetadata => true,
                    OriginalOperandLayoutPurpose::DeclarationTargets => {
                        target.kind != BindingKind::Proc
                            || target.implementation_allocation.is_none()
                    }
                    OriginalOperandLayoutPurpose::LogicalDeclarationTargets => {
                        target.kind != BindingKind::Proc
                    }
                }
        })
    {
        return trace_rbs_operand_option("site.registry_targets", None);
    }
    Some(OriginalCompilationLookupAdvice {
        snapshot: Arc::clone(snapshot),
        site: site.clone(),
        words: Arc::from(tokens.words()),
        config,
        targets: projection.targets,
        dialect,
        namespace: namespace.clone(),
        unknown: projection.unknown,
        may_be_absent: projection.may_be_absent,
    })
}

#[cfg(test)]
fn trace_original_site_projection(
    offset: u32,
    logical_source: bool,
    snapshot: &SourceLookupSnapshot,
    projection: &SourceInvocationBinding,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
        eprintln!(
            "ORIGINAL_DECLARATION offset={} stage=layout logical={} frame={:?} targets={} unknown={} absent={} opaque={}",
            offset,
            logical_source,
            snapshot.state.variable_frame,
            projection.targets.len(),
            projection.unknown,
            projection.may_be_absent,
            snapshot.state.opaque_binding_mutation,
        );
    }
}

fn original_standalone_document_naming(
    snapshot: &SourceLookupSnapshot,
    head: &tcl_lexer::NativeWord,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> bool {
    let baseline = &snapshot.state.baseline;
    baseline.metadata_context.is_standalone()
        && baseline.native_entry.is_none()
        && baseline.logical_source_input.is_none()
        && baseline.vendor_source_input.is_none()
        && baseline.hosted_execution_context.is_none()
        && !baseline.unknown_entry
        && policy.authority() == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
        && head.image().channel() == tcl_lexer::SourceChannel::Document
}

pub(super) fn original_static_layout_projection(
    snapshot: &SourceLookupSnapshot,
    namespace: &SourceNamespaceKey,
    head: &tcl_lexer::NativeWord,
    config: tcl_lexer::LexerConfig,
    dialect: tcl_registry::InvocationDialect,
    logical_source: bool,
) -> Option<SourceInvocationBinding> {
    if let Some(policy) = snapshot
        .state
        .source_variables
        .execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
    {
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        let key = trace_rbs_operand_option(
            "projection.source_key",
            if original_standalone_document_naming(snapshot, head, policy) {
                // Authored naming and the original Document grammar are separate
                // axes. This key supplies source layout, not Native object units.
                crate::signature_scan::scope::SignatureSourceNameKey::from_original_logical_source_word(
                head, rules, policy,
            )
            } else {
                crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                    head, rules, policy,
                )
            },
        )?;
        trace_rbs_operand_option(
            "projection.native_lookup",
            source_binding_from_original_input_in(
                &snapshot.state,
                &crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key),
                namespace,
                CommandTargetLookup::NamedSlots,
            ),
        )
    } else {
        // Independent authored simulation metadata does not issue a Native slot.
        trace_rbs_operand_option(
            "projection.source_policy",
            (logical_source || dialect.authored_name_policy().is_none()).then_some(()),
        )?;
        let presentation = trace_rbs_operand_option(
            "projection.presentation",
            if logical_source {
                tcl_syntax::word_rules::original_static_word_unicode_value(head)
            } else {
                tcl_syntax::word_rules::original_static_word_ascii_presentation(head)
            },
        )?;
        let head =
            trace_rbs_operand_option("projection.utf8", std::str::from_utf8(&presentation).ok())?;
        Some(source_binding_projection_in(
            &snapshot.state,
            head,
            namespace,
            CommandTargetLookup::NamedSlots,
        ))
    }
}
