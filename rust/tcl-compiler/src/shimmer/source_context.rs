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

//! Shared retained metadata and lexical policy for representation consumers.

use crate::registry_invocation::{InvocationMetadataContext, NormalRepresentationInvocation};
use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;

/// Metadata and lexical policy used by representation analysis.
/// Availability never supplies an executed native handler or a physical cache.
#[derive(Clone, Copy)]
pub struct ShimmerContext<'a> {
    registry: &'a CommandRegistry,
    metadata: Option<InvocationMetadataContext<'a>>,
    config: LexerConfig,
    profile: Option<&'static tcl_dialect::DialectProfile>,
}

impl<'a> ShimmerContext<'a> {
    /// Borrow the function's actual retained input and configuration.
    /// Missing or foreign metadata is terminal; profile labels cannot repair it.
    #[must_use]
    pub fn for_function(
        fu: &'a crate::compilation_unit::FunctionUnit,
        registry: &'a CommandRegistry,
    ) -> Option<Self> {
        let input = fu.source_metadata_input()?;
        let metadata = fu.invocation_metadata_context(registry)?;
        if input.lexer_config().normalized() != fu.source_lexer_config().normalized() {
            return None;
        }
        Some(Self {
            registry,
            metadata: Some(metadata),
            config: fu.source_lexer_config(),
            profile: Some(input.unit_profile()),
        })
    }

    /// Explicit standalone compatibility inputs for a separately supplied SSA.
    /// This does not recover missing metadata on a function or compilation unit.
    #[must_use]
    pub fn standalone(registry: &'a CommandRegistry) -> Self {
        let profile = registry.profile();
        Self {
            registry,
            metadata: profile
                .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                .map(Into::into),
            config: LexerConfig::for_profile(profile),
            profile,
        }
    }

    pub(crate) const fn registry(self) -> &'a CommandRegistry {
        self.registry
    }
    pub(crate) const fn metadata(self) -> Option<InvocationMetadataContext<'a>> {
        self.metadata
    }
    pub(crate) const fn config(self) -> LexerConfig {
        self.config
    }
    pub(crate) fn numbers(self) -> tcl_syntax::number::NumberSyntax {
        tcl_syntax::number::NumberSyntax::of_profile(self.profile)
    }
    pub(crate) fn word_rules(self) -> tcl_syntax::word_rules::WordValueRules {
        tcl_syntax::word_rules::WordValueRules::from_config(&self.config)
    }
    pub(crate) fn expression_parser(self) -> tcl_syntax::expr::parser::ExprParseContext {
        let mut parser = self.profile.map_or_else(
            || tcl_syntax::expr::parser::ExprParseContext {
                lexer_grammar: tcl_dialect::LexerGrammar::default(),
                expr_grammar_base: None,
                f5_word_grammar: None,
                native_syntax: tcl_syntax::expr::parser::NativeExprSyntax::Unknown,
            },
            tcl_syntax::expr::parser::ExprParseContext::for_profile,
        );
        parser.lexer_grammar = self.config.grammar_over(parser.lexer_grammar);
        parser
    }
    pub(crate) fn invocation(
        self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<NormalRepresentationInvocation> {
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            self.registry,
            self.metadata,
            tokens,
        )
    }
    pub(crate) fn statement(
        self,
        statement: &crate::ir::Statement,
    ) -> Option<NormalRepresentationInvocation> {
        crate::registry_invocation::normal_statement_representation_with_metadata_context(
            self.registry,
            self.metadata,
            statement,
        )
    }
    pub(crate) fn expression(
        self,
        tokens: &crate::ir::CommandTokens,
        span: tcl_lexer::Span,
    ) -> Option<crate::word_subst::LiftedSourceExpression> {
        crate::word_subst::representation_expression_with_metadata_context(
            tokens,
            self.registry,
            self.metadata,
            self.expression_parser(),
            span,
        )
    }
    pub(crate) fn lifted_expressions(
        self,
        tokens: Option<&crate::ir::CommandTokens>,
    ) -> Vec<crate::word_subst::LiftedSourceExpression> {
        crate::word_subst::lifted_representation_expressions_with_metadata_context(
            tokens,
            self.registry,
            self.metadata,
            self.config,
            self.expression_parser(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ShimmerContext;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use std::sync::Arc;

    #[test]
    fn original_representation_context_keeps_complete_availability_and_selected_grammar() {
        // naming.compiler.retained-representation-metadata
        // docs/design/analysis/name-resolution-proofs/retained-representation-metadata.md
        let base =
            tcl_registry::model::ingress::resolve_environment("tcl").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let mut commands = base.commands().project_for_profile(profile);
        commands.insert_ambient_package("example", "1.0");
        let context = Arc::new(base.with_command_store(Arc::new(commands)));
        let config = tcl_lexer::LexerConfig {
            braced_var: tcl_dialect::BracedVarStyle::FirstClose,
            list_parse: tcl_dialect::ListParse::Lenient,
            ..tcl_lexer::LexerConfig::for_profile(Some(profile))
        };
        let registry = context.commands();
        let unit = CompilationUnit::build_with_context_registry(
            "proc f {} {set x 1}",
            UnitBuildOptions {
                registry,
                dialect: Some(profile),
                defer_top_level: false,
                config,
                declared_commands: None,
                external_call_sites: None,
            },
            None,
            Arc::clone(&context),
        );
        let function = unit.function("::f").unwrap();
        let selected =
            ShimmerContext::for_function(function, registry).expect("genuine retained metadata");
        assert!(
            selected
                .metadata()
                .unwrap()
                .context()
                .ambient_package("example")
        );
        assert!(core::ptr::eq(
            selected.metadata().unwrap().context(),
            context.context()
        ));
        assert_eq!(selected.config(), config);
        assert_eq!(selected.word_rules().list, tcl_dialect::ListParse::Lenient);
        assert!(selected.word_rules().split_list("{a").is_ok());
        assert!(
            tcl_syntax::word_rules::WordValueRules::TCL
                .split_list("{a")
                .is_err()
        );
        assert_eq!(
            selected.expression_parser().lexer_grammar.braced_var,
            tcl_dialect::BracedVarStyle::FirstClose
        );
        assert_eq!(
            selected.expression_parser().lexer_grammar.numbers,
            profile.grammar.numbers
        );
        assert_eq!(selected.numbers(), profile.grammar.numbers);
        let mut stale = function.clone();
        stale.source_config.braced_var = tcl_dialect::BracedVarStyle::Tcl9Nesting;
        assert!(ShimmerContext::for_function(&stale, registry).is_none());
    }

    #[test]
    fn original_representation_consumers_keep_positive_reads_and_refuse_missing_foreign_input() {
        // naming.compiler.retained-representation-metadata
        // docs/design/analysis/name-resolution-proofs/retained-representation-metadata.md
        let registry = tcl_registry::CommandRegistry::build_default();
        let source = "proc f {lst} {set x [llength $lst]; puts [lindex $x 0]}";
        let mut unit = CompilationUnit::build_for(source, &registry, false);
        assert!(
            !super::super::find_shimmer_warnings_for_cu(&unit, &registry).is_empty(),
            "positive actual input must retain typed nested reads"
        );
        let function = unit.function("::f").unwrap();
        assert!(
            !crate::compiler_checks::shimmer_family_checks(
                function,
                &registry,
                None,
                None::<&std::collections::HashSet<String>>
            )
            .is_empty()
        );
        let mut missing = function.clone();
        missing.source_metadata_input = None;
        assert!(ShimmerContext::for_function(&missing, &registry).is_none());
        assert!(
            crate::compiler_checks::shimmer_family_checks(
                &missing,
                &registry,
                None,
                None::<&std::collections::HashSet<String>>
            )
            .is_empty()
        );
        let mut foreign_registry = tcl_registry::CommandRegistry::build_default();
        let mut unrelated = foreign_registry.get("set").unwrap().clone();
        unrelated.name = "unrelated";
        foreign_registry.insert(unrelated);
        assert!(ShimmerContext::for_function(function, &foreign_registry).is_none());
        assert!(
            crate::compiler_checks::shimmer_family_checks(
                function,
                &foreign_registry,
                None,
                None::<&std::collections::HashSet<String>>
            )
            .is_empty()
        );
        assert!(super::super::find_shimmer_warnings_for_cu(&unit, &foreign_registry).is_empty());
        assert!(super::super::first_use_commitments_for_cu(&unit, &foreign_registry).is_empty());
        assert!(super::super::find_thunking_warnings_for_cu(&unit, &foreign_registry).is_empty());
        assert!(super::super::find_sharing_warnings_for_cu(&unit, &foreign_registry).is_empty());
        assert!(super::super::find_byte_array_warnings_for_cu(&unit, &foreign_registry).is_empty());
        unit.top_level.source_metadata_input = None;
        for function in unit.procedures.values_mut() {
            function.source_metadata_input = None;
        }
        assert!(super::super::find_shimmer_warnings_for_cu(&unit, &registry).is_empty());
        assert!(super::super::first_use_commitments_for_cu(&unit, &registry).is_empty());
        assert!(super::super::find_thunking_warnings_for_cu(&unit, &registry).is_empty());
        assert!(super::super::find_sharing_warnings_for_cu(&unit, &registry).is_empty());
        assert!(super::super::find_byte_array_warnings_for_cu(&unit, &registry).is_empty());
    }

    #[test]
    fn original_representation_known_replacement_cannot_borrow_stock_conversion_hints() {
        // naming.compiler.retained-representation-metadata
        // docs/design/analysis/name-resolution-proofs/retained-representation-metadata.md
        let registry = tcl_registry::CommandRegistry::build_default();
        let stock = CompilationUnit::build_for(
            "proc f {lst} {set x [llength $lst]; puts [::lindex $x 0]}",
            &registry,
            false,
        );
        assert!(!super::super::find_shimmer_warnings_for_cu(&stock, &registry).is_empty());
        let shadow = CompilationUnit::build_for(
            "proc lindex args {return 0}; proc f {lst} {set x [llength $lst]; puts [::lindex $x 0]}",
            &registry,
            false,
        );
        let function = shadow.function("::f").unwrap();
        assert!(
            crate::compiler_checks::shimmer_family_checks(
                function,
                &registry,
                None,
                None::<&std::collections::HashSet<String>>
            )
            .is_empty(),
            "known scripted replacement supplies no stock representation contract"
        );
    }
}
