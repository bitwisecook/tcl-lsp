// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional exact-argv bracing with independent lookup and expression owners.

use super::{
    BindingKind, CommandAllocationSite, ModuleCommandBindings, SourceCommandTarget,
    SourceInvocationBinding,
};
use crate::ir::CommandTokens;
use crate::registry_invocation::InvocationWordOrigin;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span, WordKind};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};
use tcl_syntax::expr::ExprNode;
use tcl_syntax::expr::parser::{CheckedExprParse, ExprParseContext, NativeExprSyntax};
use tcl_syntax::naming::NamePolicyProtocol;

/// A single static original expression operand and an authored braced word
/// which decodes to the same counted native bytes under the same full grammar.
/// The retained checked expression contains no variable, script or function
/// evaluation, and the original handler and observer envelope remain closed.
/// A shared selected-native constant evaluator independently proves finite
/// normal completion. The whole source is one root command, so no retained
/// procedure body or later source observer can distinguish its spelling.
/// This permits this operand rewrite only; it supplies no compiler admission,
/// entered physical frame, object identity, relocation or insertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLiteralExpressionBracing {
    site: CommandAllocationSite,
    original: Vec<NativeWord>,
    native: Vec<u8>,
    replacement: String,
    target: SourceCommandTarget,
    policy: NamePolicyProtocol,
    parser: ExprParseContext,
    tree: ExprNode<Vec<u8>>,
    registry: RegistrySemanticKey,
    normal_dialect: tcl_registry::InvocationDialect,
}

impl OriginalLiteralExpressionBracing {
    /// Complete original operand with its source-channel and word geometry.
    #[must_use]
    pub fn original_operand(&self) -> &NativeWord {
        &self.original[1]
    }

    /// One authored braced source word, separately decode-checked against the
    /// original operand. This word cannot donate an original lexical receipt.
    #[must_use]
    pub fn replacement_source_word(&self) -> &str {
        &self.replacement
    }

    /// Original counted operand bytes, independent of reporting text.
    #[must_use]
    pub fn native_value(&self) -> &[u8] {
        &self.native
    }

    /// Whole original source/channel and full lexical configuration match.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.site.source.source_image() == image
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == config)
    }

    /// The original selected Registry semantics have not changed.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}

impl SourceInvocationBinding {
    /// Issue a bounded bracing rewrite for one complete static operand of the
    /// genuinely selected expression handler. Argument evaluation, original
    /// compiler name effects, observers and the checked native expression must
    /// close independently; unknowns never borrow a reporting command name.
    #[must_use]
    pub fn original_literal_expression_bracing(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<OriginalLiteralExpressionBracing> {
        if tokens.source_binding.as_ref() != Some(self)
            || tokens.words().len() != 2
            || !self.unobserved_native_dispatch()
        {
            return brace_refusal(tokens, "original-dispatch-guard");
        }
        let site = brace_required(tokens, "original-site", self.invocation_site())?;
        let config = brace_required(
            tokens,
            "original-config",
            self.original_lexer_config_for_tokens(tokens),
        )?;
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_LITERAL_BRACING").is_some() {
            eprintln!(
                "ORIGINAL_LITERAL_BRACING_HANDLER offset={:?} execution={} envelope={} lookup={} unknown={} compiler_names={}",
                self.invocation_site().map(|site| site.offset),
                self.proved_execution_target().is_some(),
                self.native_handler_envelope.is_some(),
                self.proved_target().is_some(),
                self.execution_is_unknown(),
                self.original_compiler_preserves_names.is_preserved()
            );
        }
        let target = brace_required(tokens, "selected-handler", self.proved_handler_target())?;
        let state =
            &brace_required(tokens, "original-lookup-state", self.lookup_state.as_ref())?.state;
        if self.proved_target() != Some(target)
            || !target.registry_backed
            || target.kind != BindingKind::Builtin
            || !target.prepended.is_empty()
            || state.current_source_origin.as_ref() != Some(&site.source)
            || state.has_opaque_domain()
            || state.source_step_observed()
            || !state.command_observers.is_quiet()
            || !state.retained_target_is_current(target)
            || state.runtime_execution_observed(target.identity.as_ref())
            || state.baseline.registry_snapshot.as_ref()
                != Some(&registry.snapshot().semantic_key())
        {
            return brace_refusal(tokens, "current-handler-observers");
        }
        let spec = registry.get(target.registry_identity()?)?;
        if spec.lowering_hook != Some(tcl_registry::hooks::LoweringHookId::Expr) {
            return None;
        }
        let selected = brace_required(
            tokens,
            "selected-handler-original-argv",
            crate::registry_invocation::original_selected_handler_layout_invocation(
                registry, tokens,
            ),
        )?;
        // Implementation contract: naming.refactor.original-literal-expression-bracing
        // docs/design/analysis/name-resolution-proofs/original-literal-expression-bracing.md
        // Handler shape and unchanged argv precede the independent checked
        // native-byte evaluation below; source IR expression shape is not it.
        if !selected.facts.arg_roles_complete
            || selected.facts.arity_accepts_frozen_arguments() != Some(true)
            || selected.facts.lowering_hook != spec.lowering_hook
            || selected.facts.successful_handler
                != Some(
                    tcl_registry::native_compilation::SuccessfulHandlerSpec::ExpressionArguments,
                )
            || selected.effective.words.len() != 2
            || selected.effective.origins.get(1) != Some(&InvocationWordOrigin::Written(1))
        {
            return None;
        }
        if !self.original_compiler_names_preserved_for_tokens(tokens) {
            return brace_refusal(tokens, "compiler-name-effects");
        }
        original_literal_bracing_operand(self, tokens, registry, site, config, target, state)
    }
}

fn original_literal_bracing_operand(
    binding: &SourceInvocationBinding,
    tokens: &CommandTokens,
    registry: &CommandRegistry,
    site: &CommandAllocationSite,
    config: LexerConfig,
    target: &SourceCommandTarget,
    state: &ModuleCommandBindings,
) -> Option<OriginalLiteralExpressionBracing> {
    let head = brace_required(
        tokens,
        "original-head",
        binding.original_head_name_input(tokens),
    )?;
    let policy = head.policy();
    let head = head.original_word_key()?;
    let original = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        config,
    )?;
    if original.len() != 2
        || original.first() != Some(head.original_word())
        || original.iter().any(|word| word.group().expand)
        || original[1].group().kind == WordKind::Braced
    {
        return None;
    }
    if !isolated_root_source(binding, tokens, &original, config) {
        return brace_refusal(tokens, "source-reflection-boundary");
    }
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        &original,
        policy.string_protocol(),
    )
    .ok()?;
    captured.literal(0)?;
    let native = brace_required(tokens, "static-native-operand", captured.literal(1))?;
    let advice = brace_required(
        tokens,
        "expression-context",
        crate::registry_invocation::original_literal_expression_context_advice(registry, tokens),
    )?;
    let parser = advice.parser;
    let syntax = match policy.string_protocol() {
        tcl_syntax::native_string::NativeStringProtocol::C(version) => {
            NativeExprSyntax::Tcl(version)
        }
        tcl_syntax::native_string::NativeStringProtocol::Jim084 => NativeExprSyntax::Jim084,
    };
    if !advice.lookup_closed || advice.written != 1 || parser.native_syntax != syntax {
        return brace_refusal(tokens, "expression-context-lookup");
    }
    let CheckedExprParse::Parsed(tree) =
        tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(native, &parser)
    else {
        return None;
    };
    if !pure_expression(&tree) {
        return brace_refusal(tokens, "pure-native-tree");
    }
    let actual = state.baseline.dialect?;
    if actual != advice.dialect {
        return brace_refusal(tokens, "native-evaluation-dialect");
    }
    let normal_policy =
        crate::tcl_expr_eval::FoldPolicy::for_retained_entry(registry, Some(actual), &config);
    if normal_policy.preparation_context().is_none_or(|context| {
        context.native_syntax != parser.native_syntax
            || context.lexer_grammar != parser.lexer_grammar
    }) || !finite_native_literal_result(&tree, normal_policy)
    {
        return brace_refusal(tokens, "normal-native-evaluation");
    }
    let replacement = brace_required(
        tokens,
        "candidate-roundtrip",
        braced_source_word(native, site.source.source_image(), config, policy),
    )?;
    Some(OriginalLiteralExpressionBracing {
        site: site.clone(),
        native: native.to_vec(),
        replacement,
        target: target.clone(),
        policy,
        parser,
        tree,
        registry: registry.snapshot().semantic_key(),
        normal_dialect: actual,
        original,
    })
}

// A changed procedure body can be returned by info body, even when the
// expression's argv and result match. This bounded owner admits only a whole
// isolated root command; nested bodies and future source observers stay open.
fn isolated_root_source(
    binding: &SourceInvocationBinding,
    tokens: &CommandTokens,
    original: &[NativeWord],
    config: LexerConfig,
) -> bool {
    let Some(site) = binding.invocation_site() else {
        return false;
    };
    let Some(snapshot) = binding.lookup_state.as_ref() else {
        return false;
    };
    let state = &snapshot.state;
    if !matches!(site.source.kind(), super::SourceOriginKind::Authored(_))
        || super::declaration_layout::root_diagnostic_namespace(
            state,
            &binding.lookup_namespace_key,
        )
        .is_none()
    {
        return false;
    }
    let image = site.source.source_image();
    let Ok(end) = u32::try_from(image.bytes().len()) else {
        return false;
    };
    let Ok(plan) = tcl_lexer::native_script_words_in(image.clone(), Span::new(0, end), config)
    else {
        return false;
    };
    let [command] = plan.commands.as_slice() else {
        return false;
    };
    plan.fatal_tail.is_none()
        && command.words.as_slice() == original
        && tokens.words().len() == original.len()
}

// Native evaluation is independent of source reflection and argv equivalence.
// Leaf ownership changes only through an exact UTF-8 view; no reparse or
// fallback value/engine is introduced for unavailable native units.
fn finite_native_literal_result(
    tree: &ExprNode<Vec<u8>>,
    policy: crate::tcl_expr_eval::FoldPolicy,
) -> bool {
    let mut exact = true;
    let text = tree.clone().map_text(|bytes| {
        if let Ok(text) = String::from_utf8(bytes) {
            text
        } else {
            exact = false;
            String::new()
        }
    });
    if !exact || policy.invocation_dialect.is_none() {
        return false;
    }
    let Some(evaluation) = crate::tcl_expr_eval::analyse_tcl_expr_with_resolved_math_bindings(
        &text,
        &crate::tcl_expr_eval::Env::new(),
        policy,
        &|_, _| None,
        None,
    ) else {
        return false;
    };
    evaluation.native_value_effects_are_proved()
        && !matches!(evaluation.value,crate::tcl_expr_eval::TclValue::Float(value) if !value.is_finite())
}

fn brace_refusal<T>(tokens: &CommandTokens, stage: &str) -> Option<T> {
    #[cfg(test)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_LITERAL_BRACING").is_some() {
        eprintln!(
            "ORIGINAL_LITERAL_BRACING offset={:?} stage={stage}",
            tokens
                .words()
                .first()
                .map(|word| word.source().span.start())
        );
    }
    let _ = (tokens, stage);
    None
}
fn brace_required<T>(tokens: &CommandTokens, stage: &str, value: Option<T>) -> Option<T> {
    value.or_else(|| brace_refusal(tokens, stage))
}

fn pure_expression(tree: &ExprNode<Vec<u8>>) -> bool {
    let mut pending = vec![tree];
    let mut visited = 0_u32;
    while let Some(node) = pending.pop() {
        visited += 1;
        if visited > 4096 {
            return false;
        }
        match node {
            ExprNode::Literal { .. } => {}
            ExprNode::String { text, .. } => {
                if std::str::from_utf8(text)
                    .ok()
                    .and_then(tcl_syntax::expr::ast::fixed_string_operand)
                    .is_none()
                {
                    return false;
                }
            }
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Binary { left, right, .. } => {
                pending.push(right);
                pending.push(left);
            }
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                pending.push(false_branch);
                pending.push(true_branch);
                pending.push(condition);
            }
            ExprNode::Var { .. }
            | ExprNode::Command { .. }
            | ExprNode::Call { .. }
            | ExprNode::Raw { .. }
            | ExprNode::CompiledWord { .. } => return false,
        }
    }
    true
}

fn braced_source_word(
    native: &[u8],
    original: &SourceImage,
    config: LexerConfig,
    policy: NamePolicyProtocol,
) -> Option<String> {
    let text = tcl_syntax::backslash::native_literal_source_text(
        native,
        original.channel(),
        policy.string_protocol(),
    )?;
    let replacement = format!("{{{text}}}");
    let end = u32::try_from(replacement.len()).ok()?;
    // This is an authored candidate, independent of the original word owner.
    let image = SourceImage::from_bytes(replacement.as_bytes(), original.channel());
    let plan = tcl_lexer::native_script_words_in(image, Span::new(0, end), config).ok()?;
    if plan.fatal_tail.is_some() {
        return None;
    }
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    if word.span() != Span::new(0, end)
        || word.group().kind != WordKind::Braced
        || word.group().expand
    {
        return None;
    }
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        &command.words,
        policy.string_protocol(),
    )
    .ok()?;
    (captured.literal(0)? == native).then_some(replacement)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(source: &str, profile: &str) -> Option<OriginalLiteralExpressionBracing> {
        let analysis = crate::analyser::Analyser::new().analyse(source, profile);
        let config = analysis.body_lexer_config?;
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let command = commands.last()?;
        let mut tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, command);
        analysis
            .retained_command_realm()?
            .stamp_original_tokens(&mut tokens);
        tokens
            .source_binding
            .as_ref()?
            .original_literal_expression_bracing(&tokens, analysis.resolved_registry()?)
    }

    #[test]
    fn original_literal_expression_bracing_preserves_native_argv_and_current_owners() {
        // Implementation contract: naming.refactor.original-literal-expression-bracing
        // docs/design/analysis/name-resolution-proofs/original-literal-expression-bracing.md
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let source = "expr \"1 + 2\"";
            let proof = receipt(source, profile)
                .unwrap_or_else(|| panic!("literal bracing missing: {profile}"));
            assert_eq!(proof.native_value(), b"1 + 2");
            assert_eq!(proof.replacement_source_word(), "{1 + 2}");
            assert_eq!(proof.original_operand().bytes(), b"\"1 + 2\"");
            let config = proof.original_operand().config();
            assert!(proof.matches_source(&SourceImage::document(source), config));
            assert!(!proof.matches_source(&SourceImage::native(source.as_bytes()), config));
            assert!(!proof.matches_source(
                &SourceImage::document(&format!("# moved\n{source}")),
                config
            ));
            let mut changed = config;
            changed.strict_quoting = !changed.strict_quoting;
            assert!(!proof.matches_source(&SourceImage::document(source), changed));
        }
    }

    #[test]
    fn original_literal_expression_bracing_declines_substitution_functions_and_handler_changes() {
        // Implementation contract: naming.refactor.original-literal-expression-bracing
        // docs/design/analysis/name-resolution-proofs/original-literal-expression-bracing.md
        for source in [
            "expr {$value}",
            "expr \"$value + 2\"",
            "expr \"\\$value + 2\"",
            "expr \"\\[callback\\]\"",
            "expr \"abs(1)\"",
            // Native captured errorInfo/options differ after changing spelling:
            // naming.expression.braced-error-context-publication.
            r#"expr "1 / 0""#,
            r#"expr """#,
            // Native info-body observer distinguishes these spellings:
            // naming.expression.braced-handler-and-observer-boundaries.
            r#"proc p {} {expr "1 + 2"}; p"#,
            r#"expr "1 + 2"; info frame 0"#,
            "expr 1 + 2",
            "expr {*}{1 2}",
            "expr \"1 +\"",
            "proc expr {args} {return SHADOW}; expr \"1 + 2\"",
            "trace add execution expr enter observer; expr \"1 + 2\"",
            "unknown_mutation; expr \"1 + 2\"",
        ] {
            assert!(receipt(source, "tcl8.6").is_none(), "{source}");
        }
    }

    #[test]
    fn original_literal_bracing_normality_and_source_reflection_are_independent() {
        // Implementation contract: naming.refactor.original-literal-expression-bracing
        // docs/design/analysis/name-resolution-proofs/original-literal-expression-bracing.md
        // Captured natives: naming.expression.braced-error-context-publication
        // and naming.expression.braced-handler-and-observer-boundaries.
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let generation = tcl_registry::model::ingress::static_context_for(profile);
            let registry = generation.commands();
            let dialect = tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap());
            let config = LexerConfig::for_profile(registry.profile());
            let policy = crate::tcl_expr_eval::FoldPolicy::for_retained_entry(
                registry,
                Some(dialect),
                &config,
            );
            let context = dialect.expression_parse_context(None);
            for (bytes, normal) in [(b"1 + 2".as_slice(), true), (b"1 / 0".as_slice(), false)] {
                let CheckedExprParse::Parsed(tree) =
                    tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(
                        bytes, &context,
                    )
                else {
                    panic!("selected native grammar: {profile}");
                };
                assert_eq!(
                    finite_native_literal_result(&tree, policy),
                    normal,
                    "{profile}: {bytes:?}"
                );
            }
            for source in [
                r#"expr "1 / 0""#,
                r#"proc p {} {expr "1 + 2"}; p"#,
                r#"expr "1 + 2"; info frame 0"#,
            ] {
                assert!(receipt(source, profile).is_none(), "{profile}: {source}");
            }
        }
    }

    #[test]
    fn authored_braced_argument_roundtrip_keeps_input_channels_and_escape_boundaries_separate() {
        for version in tcl_dialect::TclVersion::ALL {
            let policy = NamePolicyProtocol::authored_tcl(version);
            let config = LexerConfig::for_dialect(version.dialect_name());
            for image in [
                SourceImage::document(""),
                SourceImage::native(b"".as_slice()),
            ] {
                for value in [b"1 + 2".as_slice(), b"{plain}", b"a\\b"] {
                    assert!(
                        braced_source_word(value, &image, config, policy).is_some(),
                        "{version:?} {:?} {value:?}",
                        image.channel()
                    );
                }
                assert_eq!(
                    braced_source_word(b"a\0b", &image, config, policy).is_some(),
                    image.channel() == tcl_lexer::SourceChannel::NativeValue,
                );
                assert_eq!(
                    braced_source_word(b"a\xc0\x80b", &image, config, policy).is_some(),
                    image.channel() == tcl_lexer::SourceChannel::Document,
                );
                assert!(braced_source_word(b"a}b", &image, config, policy).is_none());
                assert!(braced_source_word(b"a\\\nb", &image, config, policy).is_none());
            }
        }
    }
}
