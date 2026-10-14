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

//! Literal argument data from its actual retained whole source word.

use crate::ir::{CommandTokens, Module, WordExpr};
use tcl_registry::CommandRegistry;

/// Read one unchanged literal argument under its original Module and grammar.
/// This carries syntax data only, independently of target selection, Native
/// entry, variable lookup, physical storage and Normal completion.
pub(crate) fn original_literal_argument_data_for_module(
    registry: &CommandRegistry,
    module: &Module,
    tokens: &CommandTokens,
    argument: usize,
) -> Option<String> {
    original_source_argument_for_module(registry, module, tokens, argument)?.literal_data()
}

/// One immutable original argument under its actual current Module. The view
/// carries source syntax and literal data, without read or execution authority.
pub(crate) struct OriginalSourceArgument<'a> {
    word: &'a WordExpr,
    native: tcl_lexer::NativeWord,
}

impl<'a> OriginalSourceArgument<'a> {
    pub(crate) fn word(&self) -> &'a WordExpr {
        self.word
    }

    pub(crate) fn literal_data(&self) -> Option<String> {
        literal_word_data(&self.native)
    }
}

pub(crate) fn original_source_argument_for_module<'a>(
    registry: &CommandRegistry,
    module: &Module,
    tokens: &'a CommandTokens,
    argument: usize,
) -> Option<OriginalSourceArgument<'a>> {
    let retained = module.retained_source_bindings.as_deref()?;
    retained.positioned_for_module(module, registry)?;
    retained
        .owns_original_analysis_attachment(tokens)
        .then_some(())?;
    let (_, words) = original_command_words(tokens)?;
    let index = argument.checked_add(1)?;
    Some(OriginalSourceArgument {
        word: tokens.words().get(index)?,
        native: words.get(index)?.clone(),
    })
}

/// Literal data under the exact original whole source vector and grammar.
/// This immutable syntax projection carries no current Module or execution proof.
pub(crate) fn original_literal_argument_data(
    tokens: &CommandTokens,
    argument: usize,
) -> Option<String> {
    let (_, words) = original_command_words(tokens)?;
    let word = words.get(argument.checked_add(1)?)?;
    literal_word_data(word)
}

fn literal_word_data(word: &tcl_lexer::NativeWord) -> Option<String> {
    let value = String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
        word,
    )?)
    .ok()?;
    (!value.contains('\0')).then_some(value)
}

/// Original word grammar and any independently retained native/authored name
/// recipe. The immutable source vector supplies no executed variable lookup.
fn original_variable_reference_syntax(
    tokens: &CommandTokens,
) -> Option<(
    tcl_lexer::LexerConfig,
    Option<tcl_syntax::naming::NamePolicyProtocol>,
)> {
    let (config, _) = original_command_words(tokens)?;
    let policy = tokens
        .source_binding
        .as_ref()?
        .original_variable_reference_policy(tokens);
    Some((config, policy))
}

/// Reborrow only the selected original expression operand whose complete AST
/// and source base remain attached to this command or its genuine child.
pub(crate) fn original_lowered_expression_syntax(
    expression: &crate::expr_ast::ExprNode,
    expression_base: Option<u32>,
    tokens: &CommandTokens,
) -> Option<(
    tcl_lexer::LexerConfig,
    Option<tcl_syntax::naming::NamePolicyProtocol>,
)> {
    let base = expression_base?;
    let (config, _) = original_command_words(tokens)?;
    let binding = tokens.source_binding.as_ref()?;
    let registry = binding.retained_registry_for_tokens(tokens)?;
    let matches = |tokens: &CommandTokens| {
        let operand = super::original_expression_operand_advice(registry, tokens)?;
        (operand.expression_base == base && &operand.expression == expression).then_some(())?;
        original_variable_reference_syntax(tokens)
    };
    if let Some(syntax) = matches(tokens) {
        return Some(syntax);
    }
    let metadata = binding
        .original_source_metadata_for_advice(tokens, registry)
        .ok()??;
    let mut matches = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
        tokens, config, registry, metadata,
    )?
    .into_iter()
    .filter_map(|call| matches(call.tokens.as_ref()?));
    let syntax = matches.next()?;
    matches.next().is_none().then_some(syntax)
}

fn original_command_words(
    tokens: &CommandTokens,
) -> Option<(tcl_lexer::LexerConfig, Vec<tcl_lexer::NativeWord>)> {
    if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let site = binding.invocation_site()?;
    let words = super::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        config,
    )?;
    Some((config, words))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn original_module(
        source: &str,
    ) -> (std::sync::Arc<tcl_registry::model::ContextRegistry>, Module) {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
            .with_resolved_analysis_input(input);
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        (context, module)
    }

    fn tokens(module: &Module) -> &CommandTokens {
        module
            .top_level
            .statements
            .iter()
            .find_map(|statement| {
                module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
            })
            .expect("retained complete original source vector")
    }

    #[test]
    fn original_arguments_keep_literal_data_separate_from_variable_reads() {
        for (source, literal, variable) in [
            ("incr n {$delta}", Some("$delta"), None),
            ("incr n $delta", None, Some("$delta")),
            ("incr n \"$delta\"", None, Some("$delta")),
            (r"incr n 0\x33", Some("03"), None),
            (r"incr n {0\x33}", Some(r"0\x33"), None),
        ] {
            let (context, module) = original_module(source);
            let original = tokens(&module);
            let argument =
                original_source_argument_for_module(context.commands(), &module, original, 1)
                    .unwrap();
            assert_eq!(argument.literal_data().as_deref(), literal, "{source}");
            assert_eq!(
                argument
                    .word()
                    .sole_variable_substitution()
                    .map(|(text, _)| text),
                variable,
                "{source}"
            );
            let mut missing = module.clone();
            missing.retained_source_bindings = None;
            assert!(
                original_source_argument_for_module(context.commands(), &missing, original, 1)
                    .is_none()
            );
            let mut changed = original.clone();
            changed.argv_texts[2] = "OTHER".to_owned();
            assert!(
                original_source_argument_for_module(context.commands(), &module, &changed, 1)
                    .is_none()
            );
            let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0");
            assert!(
                original_source_argument_for_module(foreign.commands(), &module, original, 1)
                    .is_none()
            );
        }
    }

    #[test]
    fn original_name_data_keeps_empty_sigils_and_literal_element_keys() {
        for (source, expected) in [
            ("set {} 1", Some("")),
            ("set {$name} 1", Some("$name")),
            ("set {[name]} 1", Some("[name]")),
            (r"set \$name 1", Some("$name")),
            (r"set a\x62 1", Some("ab")),
            ("set {a($key)} 1", Some("a($key)")),
            ("set {($key)} 1", Some("($key)")),
            ("set $name 1", None),
            ("set [name] 1", None),
            ("set a($key) 1", None),
            (r"set \x00 1", None),
        ] {
            let (context, module) = original_module(source);
            let value = original_literal_argument_data_for_module(
                context.commands(),
                &module,
                tokens(&module),
                0,
            );
            assert_eq!(value.as_deref(), expected, "{source}");
            assert!(module.source_entry.native_entry.is_none());
        }
    }

    #[test]
    fn original_name_data_refuses_foreign_or_changed_whole_source_owners() {
        let (context, module) = original_module("set {a($key)} 1");
        let original = tokens(&module);
        assert_eq!(
            original_literal_argument_data_for_module(context.commands(), &module, original, 0,)
                .as_deref(),
            Some("a($key)")
        );
        let mut missing = module.clone();
        missing.retained_source_bindings = None;
        assert!(
            original_literal_argument_data_for_module(context.commands(), &missing, original, 0)
                .is_none()
        );
        let mut changed = module.clone();
        changed.source = tcl_lexer::SourceImage::document("set other 1");
        assert!(
            original_literal_argument_data_for_module(context.commands(), &changed, original, 0)
                .is_none()
        );
        let mut grammar = module.clone();
        grammar.lexer_config.strict_quoting = !grammar.lexer_config.strict_quoting;
        assert!(
            original_literal_argument_data_for_module(context.commands(), &grammar, original, 0)
                .is_none()
        );
        let (_, foreign_module) = original_module("set {a($key)} 1");
        assert!(
            original_literal_argument_data_for_module(
                context.commands(),
                &module,
                tokens(&foreign_module),
                0
            )
            .is_none()
        );
        let foreign_registry = CommandRegistry::build_default();
        assert!(
            original_literal_argument_data_for_module(&foreign_registry, &module, original, 0)
                .is_none()
        );
        let mut no_input = module.clone();
        no_input.source_metadata_input = None;
        assert!(
            original_literal_argument_data_for_module(context.commands(), &no_input, original, 0)
                .is_none()
        );
        let mut synthetic = original.clone();
        synthetic.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedWrapper);
        assert!(
            original_literal_argument_data_for_module(context.commands(), &module, &synthetic, 0)
                .is_none()
        );
        let mut detached = original.clone();
        detached.source_binding = None;
        assert!(
            original_literal_argument_data_for_module(context.commands(), &module, &detached, 0)
                .is_none()
        );
        let mut stale = original.clone();
        stale.argv_texts[1] = "OTHER".to_owned();
        assert!(
            original_literal_argument_data_for_module(context.commands(), &module, &stale, 0)
                .is_none()
        );
        assert!(
            original_literal_argument_data_for_module(
                context.commands(),
                &module,
                original,
                usize::MAX
            )
            .is_none()
        );
    }
}
