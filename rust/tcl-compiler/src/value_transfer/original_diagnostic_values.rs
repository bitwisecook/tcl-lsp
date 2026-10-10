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

//! Original Logical source contents for advisory consumers. Construction joins
//! the immutable Module, function and actual semantic-projection inputs; each
//! query additionally joins its original point and read versions. Text obtained
//! here supplies no Native result, representation, frame, Normal or erasure proof.

use tcl_lexer::{LexerConfig, Span};
use tcl_registry::{CommandRegistry, RegistrySnapshot};

use super::{ProvenReadFacts, StatementId, SubstitutionHost};
use crate::cfg::BlockId;
use crate::compilation_unit::FunctionUnit;
use crate::ir::{CommandTokens, Module, Statement};
use crate::registry_invocation::InvocationMetadataContext;
use crate::sccp::DiagnosticValueFacts;
use crate::word_subst::LiftedCall;

/// Borrowed original Logical value contents, separate from executable facts.
/// The only public value projection is checked UTF-8 text for source advice.
pub struct OriginalDiagnosticValues<'a> {
    module: &'a Module,
    function: &'a FunctionUnit,
    registry: RegistrySnapshot,
    facts: DiagnosticValueFacts<'a>,
}

impl<'a> OriginalDiagnosticValues<'a> {
    /// Retain the real supplied producer. Missing or changed construction
    /// inputs refuse even if a cached diagnostic projection is populated.
    #[must_use]
    pub fn for_module_function(
        module: &'a Module,
        function: &'a FunctionUnit,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        module
            .retained_source_bindings
            .as_deref()?
            .matches_module(module, registry)
            .then_some(())?;
        let metadata = function.invocation_metadata_context_for_module(registry, module)?;
        metadata.permits_logical_source_names().then_some(())?;
        function.has_original_semantic_value_owner().then_some(())?;
        let input = metadata.source_analysis_input()?;
        function
            .semantic_value_projection
            .matches_source_input(registry, input)
            .then_some(())?;
        let facts = DiagnosticValueFacts::from_semantic(function.semantic_values()?);
        Some(Self {
            module,
            function,
            registry: registry.snapshot(),
            facts,
        })
    }

    pub(crate) fn reaches(&self, block: BlockId) -> bool {
        self.facts.executable_blocks().contains(&block)
    }

    fn point_tokens(&self, host: SubstitutionHost) -> Option<&'a CommandTokens> {
        let (block, index) = match host {
            SubstitutionHost::Statement(id) => (id.block, id.index),
            SubstitutionHost::Terminator(block) => (block, usize::MAX),
        };
        self.reaches(block).then_some(())?;
        if index != usize::MAX {
            let statement = self
                .function
                .cfg
                .blocks
                .get(&block)?
                .statements
                .get(index)?;
            let at = self
                .function
                .ssa
                .blocks
                .get(&block)?
                .statements
                .get(index)?;
            (statement == &at.statement).then_some(())?;
        }
        let tokens = self.function.cfg.source_tokens_at(block, index)?;
        self.point_metadata(tokens)?;
        Some(tokens)
    }

    fn point_metadata<'t>(
        &self,
        tokens: &'t CommandTokens,
    ) -> Option<InvocationMetadataContext<'t>> {
        let metadata = tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_module(
                tokens,
                self.module,
                self.registry.registry(),
            )?;
        metadata.permits_logical_source_names().then_some(())?;
        let config = metadata.source_analysis_input()?.lexer_config();
        (config.nested().normalized() == self.function.source_lexer_config().nested().normalized())
            .then_some(metadata)
    }

    fn original_calls(&self, tokens: &CommandTokens) -> Option<Vec<LiftedCall>> {
        let metadata = self.point_metadata(tokens)?;
        crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            tokens,
            metadata.source_analysis_input()?.lexer_config(),
            self.registry.registry(),
            metadata,
        )
    }

    fn point_config(&self, tokens: &CommandTokens) -> Option<LexerConfig> {
        Some(
            self.point_metadata(tokens)?
                .source_analysis_input()?
                .lexer_config(),
        )
    }

    /// Locate a whole original call word or its representative token. The
    /// reached-block set belongs to advisory contents, independently of SCCP's
    /// executable replacement projection.
    #[must_use]
    pub fn word_at(&self, span: Span) -> Option<(StatementId, usize)> {
        self.function.cfg.blocks.iter().find_map(|(&block, data)| {
            data.statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    if !matches!(statement, Statement::Call { .. }) {
                        return None;
                    }
                    let id = StatementId { block, index };
                    let tokens = self.point_tokens(SubstitutionHost::Statement(id))?;
                    let word = tokens
                        .words()
                        .iter()
                        .enumerate()
                        .find_map(|(word, original)| {
                            (self.function.abs_span(original.source().span) == span
                                || tokens
                                    .argv
                                    .get(word)
                                    .is_some_and(|&token| self.function.abs_span(token) == span))
                            .then_some(word)
                        })?;
                    Some((id, word))
                })
        })
    }

    /// Checked advisory contents at this original statement's read version.
    /// Earlier substitutions that may write a read keep the value unavailable.
    #[must_use]
    pub fn word_contents(&self, statement: StatementId, word: usize) -> Option<String> {
        let tokens = self.point_tokens(SubstitutionHost::Statement(statement))?;
        let calls = self.original_calls(tokens)?;
        let (value, _) = super::proven_word_value_with_facts(
            self.function,
            statement,
            word,
            self.point_config(tokens)?,
            ProvenReadFacts::diagnostic(self.facts),
            Some((tokens, &calls)),
        )?;
        value.as_str().ok().map(str::to_owned)
    }

    pub(crate) fn substitution_calls(&self, host: SubstitutionHost) -> Option<Vec<LiftedCall>> {
        self.original_calls(self.point_tokens(host)?)
    }

    pub(crate) fn substituted_word_contents(
        &self,
        host: SubstitutionHost,
        calls: &[LiftedCall],
        address: (usize, usize),
    ) -> Option<String> {
        let parent = self.point_tokens(host)?;
        // Equal child spelling or offsets do not authenticate the inventory.
        (self.original_calls(parent)?.as_slice() == calls).then_some(())?;
        let child = calls.get(address.0)?.words.as_ref()?;
        let (value, _) = super::proven_substituted_word_value_with_facts(
            self.function,
            host,
            calls,
            address,
            self.point_config(child)?,
            ProvenReadFacts::diagnostic(self.facts),
        )?;
        value.as_str().ok().map(str::to_owned)
    }

    pub(crate) fn return_tokens(&self, block: BlockId) -> Option<CommandTokens> {
        matches!(
            self.function.cfg.blocks.get(&block)?.terminator,
            Some(crate::cfg::Terminator::Return { .. })
        )
        .then_some(())?;
        Some(
            self.point_tokens(SubstitutionHost::Terminator(block))?
                .clone(),
        )
    }

    pub(crate) fn return_word_contents(
        &self,
        block: BlockId,
        tokens: &CommandTokens,
        word: usize,
    ) -> Option<String> {
        (self.return_tokens(block)?.eq(tokens)).then_some(())?;
        self.original_calls(tokens)?.is_empty().then_some(())?;
        let (value, _) = super::proven_return_word_value_with_facts(
            self.function,
            block,
            tokens,
            word,
            self.point_config(tokens)?,
            ProvenReadFacts::diagnostic(self.facts),
        )?;
        value.as_str().ok().map(str::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::analyser::ResolvedAnalysisInput;
    use crate::analyses::LatticeValue;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};

    fn input(context: Arc<tcl_registry::model::ContextRegistry>) -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            LexerConfig::for_file_grammar(profile.grammar),
        )
    }

    fn unit(source: &str, input: &ResolvedAnalysisInput) -> CompilationUnit {
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: input.borrowed_context_registry().commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        )
    }

    fn read(unit: &CompilationUnit) -> (StatementId, usize) {
        let function = unit
            .function("::f")
            .expect("the original Logical procedure");
        function
            .cfg
            .blocks
            .iter()
            .find_map(|(&block, data)| {
                data.statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        if !matches!(statement, Statement::Call { .. }) {
                            return None;
                        }
                        let tokens = function.cfg.source_tokens_at(block, index)?;
                        (tokens
                            .argv_texts
                            .last()
                            .is_some_and(|word| word.contains('$')))
                        .then_some((StatementId { block, index }, tokens.argv.len() - 1))
                    })
            })
            .expect("the original variable-reading call")
    }

    #[test]
    fn original_logical_diagnostic_contents_keep_executable_values_independent() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Source contents do not establish a Native read, result or replacement.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let input = input(Arc::clone(&context));
        let source = "proc f {} {set p literal.txt; open $p}";
        let unit = unit(source, &input);
        let function = unit.function("::f").unwrap();
        let (statement, word) = read(&unit);
        let values = OriginalDiagnosticValues::for_module_function(
            &unit.ir_module,
            function,
            context.commands(),
        )
        .expect("actual Logical diagnostic projection");
        let symbol = function.ssa.var_symbol("p").unwrap();
        let version =
            function.ssa.blocks[&statement.block].statements[statement.index].uses[&symbol];
        assert_eq!(
            function.sccp.values.get(&(symbol, version)),
            Some(&LatticeValue::Overdefined)
        );
        assert!(
            super::super::proven_word_value(
                function,
                statement,
                word,
                function.source_lexer_config()
            )
            .is_none()
        );
        assert_eq!(
            values.word_contents(statement, word).as_deref(),
            Some("literal.txt")
        );
        let tokens = function
            .cfg
            .source_tokens_at(statement.block, statement.index)
            .unwrap();
        assert_eq!(
            values.word_at(function.abs_span(tokens.argv[word])),
            Some((statement, word))
        );
        assert_eq!(
            function.sccp.values.get(&(symbol, version)),
            Some(&LatticeValue::Overdefined)
        );
    }

    #[test]
    fn original_logical_diagnostic_contents_preserve_aliases_literal_names_and_read_versions() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let input = input(Arc::clone(&context));
        for (source, expected) in [
            (
                "interp alias {} store {} set; proc f {} {store p literal.txt; open $p}",
                "literal.txt",
            ),
            (
                "interp alias {} store {} set p; proc f {} {store literal.txt; open $p}",
                "literal.txt",
            ),
            (
                "proc f {} {set {$p} literal.txt; open ${$p}}",
                "literal.txt",
            ),
            (
                "proc f {} {set scalar(open literal.txt; open ${scalar(open}}",
                "literal.txt",
            ),
            (
                "proc f {} {set café literal.txt; open pre${café}post}",
                "preliteral.txtpost",
            ),
            (
                "proc f {} {set p FIRST; open $p; set p SECOND; open $p}",
                "FIRST",
            ),
        ] {
            let unit = unit(source, &input);
            let function = unit.function("::f").unwrap();
            let (statement, word) = read(&unit);
            let values = OriginalDiagnosticValues::for_module_function(
                &unit.ir_module,
                function,
                context.commands(),
            )
            .unwrap();
            assert_eq!(
                values.word_contents(statement, word).as_deref(),
                Some(expected),
                "{source}"
            );
        }
        for source in [
            "proc f {p} {open $p}",
            "proc f {} {set p literal.txt; set p $unknown; open $p}",
            "proc set args {return CUSTOM}; proc f {} {set p literal.txt; open $p}",
            "proc f {} {set p FIRST; opaque [set p SECOND] $p}",
        ] {
            let unit = unit(source, &input);
            let function = unit.function("::f").unwrap();
            let (statement, word) = read(&unit);
            let values = OriginalDiagnosticValues::for_module_function(
                &unit.ir_module,
                function,
                context.commands(),
            )
            .unwrap();
            assert!(values.word_contents(statement, word).is_none(), "{source}");
        }
    }

    #[test]
    fn original_logical_diagnostic_contents_refuse_changed_availability_and_source_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut registry = CommandRegistry::build_default().project_for_profile(profile);
        let mut setter = registry.get("set").unwrap().clone();
        setter.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(setter);
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.6")
                .with_command_store(Arc::new(registry)),
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        assert!(Arc::ptr_eq(context.commands(), older.commands()));
        let current = input(Arc::clone(&context));
        let old = input(Arc::clone(&older));
        let source = "proc f {} {set p literal.txt; open $p}";
        let unit = unit(source, &current);
        let function = unit.function("::f").unwrap();
        let (statement, word) = read(&unit);
        assert_eq!(
            OriginalDiagnosticValues::for_module_function(
                &unit.ir_module,
                function,
                context.commands()
            )
            .unwrap()
            .word_contents(statement, word)
            .as_deref(),
            Some("literal.txt")
        );
        assert!(
            function
                .semantic_value_projection
                .matches_source_input(context.commands(), &current)
        );
        assert!(
            !function
                .semantic_value_projection
                .matches_source_input(context.commands(), &old)
        );
        let mut changed = unit.ir_module.clone();
        changed.source_metadata_input = Some(old.clone());
        assert!(
            OriginalDiagnosticValues::for_module_function(&changed, function, context.commands())
                .is_none()
        );
        let mut missing = function.clone();
        missing.source_metadata_input = None;
        assert!(
            OriginalDiagnosticValues::for_module_function(
                &unit.ir_module,
                &missing,
                context.commands()
            )
            .is_none()
        );
        let mut missing = unit.ir_module.clone();
        missing.source_metadata_input = None;
        assert!(
            OriginalDiagnosticValues::for_module_function(&missing, function, context.commands())
                .is_none()
        );
        let mut stale = function.clone();
        stale.source_config.strict_quoting = !stale.source_config.strict_quoting;
        assert!(
            OriginalDiagnosticValues::for_module_function(
                &unit.ir_module,
                &stale,
                context.commands()
            )
            .is_none()
        );
        let mut changed = function.clone();
        let Statement::Call {
            tokens: Some(tokens),
            ..
        } = &mut changed
            .cfg
            .blocks
            .get_mut(&statement.block)
            .unwrap()
            .statements[statement.index]
        else {
            panic!("the original source call")
        };
        tokens.argv_texts[word] = "$other".to_owned();
        assert!(
            OriginalDiagnosticValues::for_module_function(
                &unit.ir_module,
                &changed,
                context.commands()
            )
            .unwrap()
            .word_contents(statement, word)
            .is_none()
        );
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        assert!(
            OriginalDiagnosticValues::for_module_function(
                &unit.ir_module,
                function,
                foreign.commands()
            )
            .is_none()
        );
        let old_unit = self::unit(source, &old);
        let old_function = old_unit.function("::f").unwrap();
        let (old_statement, old_word) = read(&old_unit);
        assert!(
            OriginalDiagnosticValues::for_module_function(
                &old_unit.ir_module,
                old_function,
                context.commands()
            )
            .unwrap()
            .word_contents(old_statement, old_word)
            .is_none()
        );
        let native = ResolvedAnalysisInput::new(profile, profile, context, current.lexer_config());
        let native_unit = self::unit(source, &native);
        assert!(
            OriginalDiagnosticValues::for_module_function(
                &native_unit.ir_module,
                &native_unit.top_level,
                native.borrowed_context_registry().commands()
            )
            .is_none()
        );
    }

    #[test]
    fn original_logical_diagnostic_contents_refuse_a_same_input_foreign_populated_projection() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // The complete input alone does not identify another function's producer.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let input = input(Arc::clone(&context));
        let original = unit("proc f {} {set p ORIGINAL; open $p}", &input);
        let other = unit("proc f {} {set p FOREIGN; open $p}", &input);
        let function = original.function("::f").unwrap();
        let foreign = other.function("::f").unwrap();
        assert_eq!(
            function.source_metadata_input(),
            foreign.source_metadata_input()
        );
        assert!(
            foreign
                .semantic_values()
                .unwrap()
                .contents_iter()
                .any(|(_, value)| {
                    *value
                        == LatticeValue::Const(crate::analyses::ConstValue::String(
                            "FOREIGN".to_owned(),
                        ))
                })
        );
        let (statement, word) = read(&original);
        let mut clone = function.clone();
        assert_eq!(
            OriginalDiagnosticValues::for_module_function(
                &original.ir_module,
                &clone,
                context.commands()
            )
            .unwrap()
            .word_contents(statement, word)
            .as_deref(),
            Some("ORIGINAL")
        );
        clone.invalidate_semantic_values();
        assert!(clone.has_original_semantic_value_owner());
        assert!(!Arc::ptr_eq(
            &clone.semantic_value_projection,
            &function.semantic_value_projection
        ));
        assert_eq!(
            OriginalDiagnosticValues::for_module_function(
                &original.ir_module,
                &clone,
                context.commands()
            )
            .unwrap()
            .word_contents(statement, word)
            .as_deref(),
            Some("ORIGINAL")
        );
        crate::lattice_rebase::rebase_function_unit(&mut clone, 7);
        assert!(clone.has_original_semantic_value_owner());
        // Source restoration has not happened: shifted words cannot borrow
        // their previous original point, independently of cache detachment.
        assert!(
            OriginalDiagnosticValues::for_module_function(
                &original.ir_module,
                &clone,
                context.commands()
            )
            .unwrap()
            .word_contents(statement, word)
            .is_none()
        );
        let mut swapped = function.clone();
        swapped.semantic_value_projection = Arc::clone(&foreign.semantic_value_projection);
        assert!(
            swapped
                .semantic_value_projection
                .matches_source_input(context.commands(), &input)
        );
        assert!(!swapped.has_original_semantic_value_owner());
        assert!(swapped.semantic_values().is_none());
        assert!(
            OriginalDiagnosticValues::for_module_function(
                &original.ir_module,
                &swapped,
                context.commands()
            )
            .is_none()
        );
        swapped.invalidate_semantic_values();
        assert!(swapped.semantic_values().is_none());
        assert!(
            OriginalDiagnosticValues::for_module_function(
                &original.ir_module,
                &swapped,
                context.commands()
            )
            .is_none()
        );
    }

    #[test]
    fn original_logical_diagnostic_contents_keep_nested_and_return_source_carriers() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let input = input(Arc::clone(&context));
        let unit = unit(
            "proc f {} {set p literal.txt; opaque [open $p]; return $p}",
            &input,
        );
        let function = unit.function("::f").unwrap();
        let values = OriginalDiagnosticValues::for_module_function(
            &unit.ir_module,
            function,
            context.commands(),
        )
        .unwrap();
        let mut found_child = false;
        for (&block, data) in &function.cfg.blocks {
            for (index, _) in data.statements.iter().enumerate() {
                let host = SubstitutionHost::Statement(StatementId { block, index });
                let Some(calls) = values.substitution_calls(host) else {
                    continue;
                };
                for (call, lifted) in calls.iter().enumerate() {
                    if lifted.words.as_ref().is_some_and(|tokens| {
                        tokens.argv_texts.last().is_some_and(|text| text == "$p")
                    }) {
                        assert_eq!(
                            values
                                .substituted_word_contents(host, &calls, (call, 1))
                                .as_deref(),
                            Some("literal.txt")
                        );
                        let mut altered = calls.clone();
                        altered[call].words.as_mut().unwrap().argv_texts[1] = "$other".to_owned();
                        assert!(
                            values
                                .substituted_word_contents(host, &altered, (call, 1))
                                .is_none()
                        );
                        found_child = true;
                    }
                }
            }
        }
        assert!(
            found_child,
            "the authentic nested substitution was retained"
        );
        // Unknown callees can affect a later read; the return's real carrier
        // remains queryable but cannot borrow the earlier constant.
        let return_tokens = function
            .cfg
            .blocks
            .keys()
            .find_map(|&block| values.return_tokens(block).map(|tokens| (block, tokens)))
            .expect("the original return operand carrier");
        assert!(
            values
                .return_word_contents(return_tokens.0, &return_tokens.1, 1)
                .is_none()
        );
        let mut changed = return_tokens.1.clone();
        changed.argv_texts[1] = "$other".to_owned();
        assert!(
            values
                .return_word_contents(return_tokens.0, &changed, 1)
                .is_none()
        );
    }
}
