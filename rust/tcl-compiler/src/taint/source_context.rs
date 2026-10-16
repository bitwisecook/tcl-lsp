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

/// Independently reusable source-lattice inputs. The retained context owns
/// metadata selection; argument and result obligations remain separate.
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
