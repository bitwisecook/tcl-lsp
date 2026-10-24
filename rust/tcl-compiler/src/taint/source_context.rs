// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retained metadata for source lattice projection, independently of execution.

use crate::analyser::ResolvedAnalysisInput;
use crate::compilation_unit::FunctionUnit;
use crate::registry_invocation::InvocationMetadataContext;
use tcl_dialect::DialectProfile;
use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;

/// Original invocation geometry retained by the diagnostic that selected a sink.
/// It supplies source correspondence, independently of rendered sink labels,
/// execution, runtime binding identity or permission to rewrite an evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaintSinkSource {
    words: Vec<tcl_lexer::NativeWord>,
    input: ResolvedAnalysisInput,
}

impl TaintSinkSource {
    pub(super) fn for_original_invocation(
        tokens: &crate::ir::CommandTokens,
        source: TaintSourceContext<'_>,
    ) -> Option<Self> {
        let input = source.metadata?.source_analysis_input()?;
        let binding = tokens.source_binding.as_ref()?;
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        if config != input.lexer_config() || config != source.config {
            return None;
        }
        let metadata = binding.original_invocation_metadata_for_module(
            tokens,
            source.module?,
            input.borrowed_context_registry().commands(),
        )?;
        if metadata.source_analysis_input() != Some(input) {
            return None;
        }
        let image = binding.original_argument_source_image(tokens)?;
        let (segment, original) = binding.original_recorded_command()?;
        if original.words() != tokens.words() {
            return None;
        }
        let words = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            segment.argv.first()?.span.start(),
            config,
        )?;
        Some(Self {
            words,
            input: input.clone(),
        })
    }

    /// Compare the current caller's genuine whole vector and complete input.
    /// Arbitrary spelling, a nearby span or a label cannot reconstruct this subject.
    #[must_use]
    pub fn matches_original_words(
        &self,
        words: &[tcl_lexer::NativeWord],
        input: &ResolvedAnalysisInput,
    ) -> bool {
        &self.input == input && self.words == words
    }

    /// The retained original whole words, without a dispatch or edit grant.
    #[must_use]
    pub fn original_words(&self) -> &[tcl_lexer::NativeWord] {
        &self.words
    }
}

/// Source policy accompanying each projection. Actual missing or mismatched
/// input remains unavailable; only explicitly standalone callers use a label.
#[derive(Clone, Copy)]
pub(crate) struct TaintSourceContext<'a> {
    pub(super) metadata: Option<InvocationMetadataContext<'a>>,
    pub(super) config: LexerConfig,
    pub(super) dialect: Option<&'static DialectProfile>,
    standalone: bool,
    module: Option<&'a crate::ir::Module>,
    body_id: Option<crate::ir::ProcedureId>,
    module_owner: Option<&'a crate::command_binding::RetainedSourceModuleBindings>,
}

impl<'a> TaintSourceContext<'a> {
    pub(crate) const fn metadata_context(self) -> Option<InvocationMetadataContext<'a>> {
        self.metadata
    }

    pub(crate) const fn dialect(self) -> Option<&'static DialectProfile> {
        self.dialect
    }

    pub(crate) const fn lexer_config(self) -> LexerConfig {
        self.config
    }

    pub(crate) fn for_input(
        registry: &CommandRegistry,
        input: Option<&'a ResolvedAnalysisInput>,
        config: LexerConfig,
    ) -> Self {
        Self {
            metadata: input.and_then(|input| {
                InvocationMetadataContext::for_source_input(
                    registry,
                    input,
                    config,
                    Some(input.unit_profile()),
                )
            }),
            config,
            dialect: input.map(ResolvedAnalysisInput::unit_profile),
            standalone: false,
            module: None,
            body_id: None,
            module_owner: None,
        }
    }

    /// Reborrow explicit standalone compatibility only for this actual CU's
    /// current top-level graph and individually retained source carriers.
    /// Supplied or withdrawn metadata never selects this compatibility path.
    pub(crate) fn for_unit_function(
        registry: &CommandRegistry,
        unit: &'a crate::compilation_unit::CompilationUnit,
        function: &'a FunctionUnit,
    ) -> Self {
        let source = Self::for_module_function(registry, &unit.ir_module, function);
        let module = &unit.ir_module;
        if function.body_id.is_some()
            || !module.source_entry.metadata_context.is_standalone()
            || !function.cfg.metadata_context.is_standalone()
            || module.dialect_profile != registry.profile()
            || !crate::ssa::current_authored_analysis_function(unit, function, registry)
            || !function.has_original_semantic_value_owner()
            || function
                .semantic_value_projection
                .retained_registry()
                .is_none_or(|held| {
                    held.snapshot().semantic_key() != registry.snapshot().semantic_key()
                })
            || !original_unit_source_graph(function, source.module_owner)
        {
            return source;
        }
        let Some(selection) = module
            .source_entry
            .metadata_context
            .metadata_context(registry)
        else {
            return source;
        };
        Self {
            metadata: selection.context(),
            standalone: true,
            ..source
        }
    }

    pub(crate) fn for_module(registry: &CommandRegistry, module: &'a crate::ir::Module) -> Self {
        let module_owner = module
            .retained_source_bindings
            .as_deref()
            .filter(|owner| owner.matches_module(module, registry));
        Self {
            metadata: module_owner
                .and_then(|_| InvocationMetadataContext::for_module(registry, module)),
            config: module.lexer_config,
            dialect: module.dialect_profile,
            standalone: false,
            module: module_owner.map(|_| module),
            body_id: None,
            module_owner,
        }
    }

    pub(crate) fn for_function(registry: &CommandRegistry, function: &'a FunctionUnit) -> Self {
        Self::for_input(
            registry,
            function.source_metadata_input(),
            function.source_lexer_config(),
        )
    }

    pub(crate) fn for_module_function(
        registry: &CommandRegistry,
        module: &'a crate::ir::Module,
        function: &'a FunctionUnit,
    ) -> Self {
        let source = Self::for_module(registry, module);
        Self {
            metadata: source
                .metadata
                .and_then(|_| function.invocation_metadata_context_for_module(registry, module)),
            module: source.module,
            body_id: function.body_id,
            module_owner: source.module_owner,
            ..Self::for_function(registry, function)
        }
    }

    /// Independently requested scalar analysis; never a missing-input fallback.
    pub(crate) fn standalone(
        registry: &CommandRegistry,
        dialect: Option<&'static DialectProfile>,
    ) -> Self {
        let dialect = dialect.or_else(|| registry.profile());
        Self {
            metadata: dialect
                .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                .map(InvocationMetadataContext::from),
            config: LexerConfig::for_profile(dialect),
            dialect,
            standalone: true,
            module: None,
            body_id: None,
            module_owner: None,
        }
    }

    pub(crate) const fn module(self) -> Option<&'a crate::ir::Module> {
        self.module
    }
    pub(crate) const fn body_id(self) -> Option<crate::ir::ProcedureId> {
        self.body_id
    }
    pub(crate) fn procedure_ids(
        self,
        tokens: &crate::ir::CommandTokens,
        word: &str,
        registry: &CommandRegistry,
    ) -> Option<Vec<crate::ir::ProcedureId>> {
        crate::registry_invocation::original_source_procedure_ids_for_word_for_module(
            tokens,
            word,
            self.module?,
            registry,
        )
    }

    pub(super) const fn module_owner(
        self,
    ) -> Option<&'a crate::command_binding::RetainedSourceModuleBindings> {
        self.module_owner
    }

    pub(super) const fn allows_nominal_metadata(self) -> bool {
        self.standalone
    }
}

/// Check the current graph against every retained source point, including
/// synthetic operations. Missing or foreign original carriers remain absent.
fn original_unit_source_graph(
    function: &FunctionUnit,
    owner: Option<&crate::command_binding::RetainedSourceModuleBindings>,
) -> bool {
    let Some(owner) = owner else {
        return false;
    };
    let Some(points) = function.ssa.point_contexts.as_ref() else {
        return false;
    };
    if !function.ssa.has_current_source_cfg_topology(&function.cfg)
        || function.ssa.blocks.len() != function.cfg.blocks.len()
        || function.ssa.blocks.iter().any(|(block, ssa)| {
            function.cfg.blocks.get(block).is_none_or(|cfg| {
                ssa.statements.len() != cfg.statements.len()
                    || ssa
                        .statements
                        .iter()
                        .zip(&cfg.statements)
                        .any(|(ssa, cfg)| &ssa.statement != cfg)
            })
        })
    {
        return false;
    }
    let inventory = points.source_token_inventory();
    if function.cfg.blocks.iter().any(|(&block, cfg)| {
        cfg.statements.iter().enumerate().any(|(index, statement)| {
            statement.synthetic_marker().is_none() && !inventory.contains_key(&(block, index))
        })
    }) {
        return false;
    }
    !inventory.is_empty()
        && inventory.iter().all(|(&(block, index), tokens)| {
            function.cfg.source_input_tokens_at(block, index) == Some(tokens)
                && owner.owns_original_tokens(tokens)
        })
}

/// Independently reusable source-lattice inputs. The retained context owns
/// metadata selection; argument and result obligations remain separate.
#[derive(Clone, Copy)]
pub(crate) struct TaintPropagationInputs<'a> {
    pub(crate) registry: &'a CommandRegistry,
    pub(crate) rendered_props: Option<
        &'a std::collections::HashMap<
            crate::ssa::ValueKey,
            crate::rendered_properties::RenderedValueProps,
        >,
    >,
    pub(crate) interproc: Option<&'a crate::interprocedural::InterproceduralAnalysis>,
    pub(crate) source: TaintSourceContext<'a>,
    pub(crate) param_taints: Option<&'a std::collections::HashMap<String, super::TaintLattice>>,
    pub(crate) taint_summaries: Option<
        &'a std::collections::HashMap<
            crate::ir::ProcedureId,
            crate::taint_interproc::ProcTaintSummary,
        >,
    >,
    pub(crate) instance_classes: &'a super::LocalInstanceClasses,
}

#[cfg(test)]
mod original_standalone_unit_tests {
    use super::*;

    #[test]
    fn explicit_standalone_unit_uses_its_current_original_token_owner() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let text = "set x [read $fd]\neval $x";
        let unit = crate::compilation_unit::CompilationUnit::build_for(text, registry, false);
        let source = TaintSourceContext::for_unit_function(registry, &unit, &unit.top_level);
        assert!(unit.ir_module.source_metadata_input.is_none());
        assert!(unit.ir_module.source_entry.metadata_context.is_standalone());
        assert!(unit.ir_module.source_entry.native_entry.is_none());
        assert!(source.allows_nominal_metadata());
        assert!(source.module_owner().is_some());
        assert!(std::ptr::eq(source.module().unwrap(), &unit.ir_module));

        let foreign = crate::compilation_unit::CompilationUnit::build_for(text, registry, false);
        assert!(
            !TaintSourceContext::for_unit_function(registry, &unit, &foreign.top_level,)
                .allows_nominal_metadata()
        );

        let mut swapped = unit.clone();
        swapped.cfg_module.top_level = foreign.cfg_module.top_level.clone();
        swapped.top_level.cfg = foreign.top_level.cfg.clone();
        swapped.top_level.ssa = foreign.top_level.ssa.clone();
        assert!(
            !TaintSourceContext::for_unit_function(registry, &swapped, &swapped.top_level,)
                .allows_nominal_metadata()
        );

        let mut changed = unit.clone();
        let statement = changed
            .top_level
            .ssa
            .blocks
            .values_mut()
            .flat_map(|block| &mut block.statements)
            .find(|statement| {
                statement.statement
                    != crate::ir::Statement::unseen_call_marker(statement.statement.span())
            })
            .expect("the original script retains a non-marker statement");
        let replacement = crate::ir::Statement::unseen_call_marker(statement.statement.span());
        assert_ne!(statement.statement, replacement);
        statement.statement = replacement;
        assert!(
            !TaintSourceContext::for_unit_function(registry, &changed, &changed.top_level,)
                .allows_nominal_metadata()
        );

        let mut topology = unit.clone();
        topology
            .top_level
            .cfg
            .blocks
            .get_mut(&topology.top_level.cfg.entry)
            .unwrap()
            .terminator = None;
        topology.cfg_module.top_level = topology.top_level.cfg.clone();
        assert!(
            !TaintSourceContext::for_unit_function(registry, &topology, &topology.top_level,)
                .allows_nominal_metadata()
        );

        let wrong_registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(
            !TaintSourceContext::for_unit_function(wrong_registry, &unit, &unit.top_level,)
                .allows_nominal_metadata()
        );

        let mut withdrawn = unit.clone();
        withdrawn.ir_module.source_entry.metadata_context =
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
        assert!(
            !TaintSourceContext::for_unit_function(registry, &withdrawn, &withdrawn.top_level,)
                .allows_nominal_metadata()
        );

        let mut replaced = unit.clone();
        let tokens = replaced
            .top_level
            .cfg
            .blocks
            .values_mut()
            .flat_map(|block| &mut block.statements)
            .filter_map(crate::ir::Statement::tokens_mut)
            .find(|tokens| tokens.source_binding.is_some())
            .expect("the original statement retains its source binding");
        let original_tokens = tokens.clone();
        tokens.source_binding.take().unwrap();
        assert_ne!(*tokens, original_tokens);
        assert!(
            !TaintSourceContext::for_unit_function(registry, &replaced, &replaced.top_level,)
                .allows_nominal_metadata()
        );
    }

    #[test]
    fn coordinated_same_span_payload_edit_cannot_reuse_original_statement_custody() {
        use crate::ir::Statement;
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "set x [read $fd]\neval $x",
            registry,
            false,
        );
        assert!(
            TaintSourceContext::for_unit_function(registry, &unit, &unit.top_level)
                .allows_nominal_metadata()
        );
        let (block, index, mut payload) = unit
            .top_level
            .ssa
            .blocks
            .iter()
            .find_map(|(&block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .find(|(_, statement)| {
                        matches!(
                            statement.statement,
                            Statement::Call { .. }
                                | Statement::AssignValue { .. }
                                | Statement::AssignConst { .. }
                        ) && statement.statement.synthetic_marker().is_none()
                    })
                    .map(|(index, statement)| (block, index, statement.statement.clone()))
            })
            .expect("original evaluated value or call payload");
        let span = payload.span();
        match &mut payload {
            Statement::Call { args, .. } => args.push("FORGED".to_owned()),
            Statement::AssignValue { value, .. } | Statement::AssignConst { value, .. } => {
                value.push_str("FORGED")
            }
            _ => unreachable!(),
        }
        assert_eq!(payload.span(), span);
        let mut changed = unit.clone();
        changed
            .top_level
            .cfg
            .blocks
            .get_mut(&block)
            .unwrap()
            .statements[index] = payload.clone();
        changed
            .top_level
            .ssa
            .blocks
            .get_mut(&block)
            .unwrap()
            .statements[index]
            .statement = payload;
        changed.cfg_module.top_level = changed.top_level.cfg.clone();
        let original = crate::ssa::SsaSourceView::at_statement(&unit.top_level.ssa, block, index)
            .source_tokens()
            .unwrap();
        assert_eq!(
            crate::ssa::SsaSourceView::at_statement(&changed.top_level.ssa, block, index)
                .source_tokens(),
            Some(original)
        );
        assert!(
            unit.ir_module
                .retained_source_bindings
                .as_ref()
                .unwrap()
                .owns_original_tokens(original)
        );
        assert!(
            !TaintSourceContext::for_unit_function(registry, &changed, &changed.top_level)
                .allows_nominal_metadata()
        );
    }

    #[test]
    fn supplied_source_keeps_its_existing_strict_metadata_purpose() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "set x [read $fd]\neval $x",
            registry,
            false,
            "tcl8.6",
        );
        let source = TaintSourceContext::for_unit_function(registry, &unit, &unit.top_level);
        assert!(!source.allows_nominal_metadata());
        assert!(unit.ir_module.source_metadata_input.is_some());
    }
}
