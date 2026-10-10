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
        module: &crate::ir::Module,
        function: &'a FunctionUnit,
    ) -> Self {
        Self {
            metadata: function.invocation_metadata_context_for_module(registry, module),
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
        }
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
    pub(crate) taint_summaries:
        Option<&'a std::collections::HashMap<String, crate::taint_interproc::ProcTaintSummary>>,
    pub(crate) instance_classes: &'a super::LocalInstanceClasses,
}
