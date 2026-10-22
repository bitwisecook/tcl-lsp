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

/// Purpose-specific whitespace compaction of a pure, finite original expression.
/// The underlying original bracing receipt retains its complete source, parser,
/// selected handler, Registry, observer and root-reflection envelope. This is
/// neither a general word rewrite nor permission for variable or child reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalExpressionGapCompaction {
    expression: OriginalLiteralExpressionBracing,
    gaps: Vec<Span>,
}

impl OriginalExpressionGapCompaction {
    /// Original source gaps independently verified by this purpose's issuer.
    #[must_use]
    pub fn removed_gaps(&self) -> &[Span] {
        &self.gaps
    }

    /// Complete unchanged original braced operand and its source configuration.
    #[must_use]
    pub fn original_operand(&self) -> &NativeWord {
        self.expression.original_operand()
    }

    /// Independently verified whole-word rendering of the approved gap edits.
    #[must_use]
    pub fn replacement_source_word(&self) -> &str {
        self.expression.replacement_source_word()
    }

    /// Complete source/channel/configuration correspondence retained by issuer.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.expression.matches_source(image, config)
    }

    /// Exact original Registry semantic generation retained by issuer.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.expression.matches_registry(registry)
    }
}

struct ExpressionRewriteEnvelope<'a> {
    site: &'a CommandAllocationSite,
    config: LexerConfig,
    target: &'a SourceCommandTarget,
    state: &'a ModuleCommandBindings,
}

#[derive(Clone, Copy)]
enum ExpressionRewrite<'a> {
    Brace,
    Gaps(&'a str),
}

// Both token streams retain the exact selected expression grammar. Only gaps
// classified by that lexer may disappear; token bytes and the checked Native
// AST retain exact topology and lexer-issued leaf correspondence. Braced
// decoding correspondence is checked by the caller.
fn same_expression_gaps(
    original: &[u8],
    candidate: &[u8],
    parser: &ExprParseContext,
    config: LexerConfig,
    tree: &ExprNode<Vec<u8>>,
) -> bool {
    let Some((original_tokens, candidate_tokens)) =
        expression_syntax_signature(original, parser, config)
            .zip(expression_syntax_signature(candidate, parser, config))
    else {
        return false;
    };
    if original_tokens.len() != candidate_tokens.len()
        || original_tokens
            .iter()
            .zip(&candidate_tokens)
            .any(|(old, new)| old.kind != new.kind || old.text != new.text)
    {
        return false;
    }
    let mut cursor = 0;
    for &byte in original {
        if candidate.get(cursor) == Some(&byte) {
            cursor += 1;
        } else if !byte.is_ascii_whitespace() {
            return false;
        }
    }
    cursor == candidate.len()
        && matches!(tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(candidate, parser), CheckedExprParse::Parsed(ref candidate) if same_pure_expression_tree(tree, candidate, &original_tokens, &candidate_tokens))
}

fn expression_syntax_signature(
    source: &[u8],
    parser: &ExprParseContext,
    config: LexerConfig,
) -> Option<Vec<tcl_lexer::ExprToken<Vec<u8>>>> {
    let (tokens, unknown) = tcl_lexer::tokenise_expr_bytes_checked_with_expression_grammar(
        source,
        &parser.lexer_grammar,
        parser.expr_grammar_base,
        parser.f5_word_grammar,
    );
    if unknown || tcl_lexer::expression_terms(source, &tokens, config).is_none() {
        return None;
    }
    Some(
        tokens
            .into_iter()
            .filter(|token| !token.kind.is_skipped() && token.kind != tcl_lexer::ExprTokenType::Eof)
            .collect(),
    )
}

fn whitespace_gap_spans(original: &[u8], candidate: &[u8], base: u32) -> Option<Vec<Span>> {
    let mut cursor = 0;
    let mut gaps: Vec<Span> = Vec::new();
    for (index, &byte) in original.iter().enumerate() {
        if candidate.get(cursor) == Some(&byte) {
            cursor += 1;
        } else {
            if !byte.is_ascii_whitespace() {
                return None;
            }
            let start = base.checked_add(u32::try_from(index).ok()?)?;
            let end = start.checked_add(1)?;
            if let Some(last) = gaps.last_mut().filter(|gap| gap.end() == start) {
                *last = Span::new(last.start(), end);
            } else {
                gaps.push(Span::new(start, end));
            }
        }
    }
    (cursor == candidate.len()).then_some(gaps)
}

// Exact AST equality also compares source coordinates. Gap removal changes
// those coordinates, so each unchanged leaf must instead address the same
// paired original/candidate lexer token. No leaf extent is discarded.
fn same_pure_expression_tree(
    original: &ExprNode<Vec<u8>>,
    candidate: &ExprNode<Vec<u8>>,
    original_tokens: &[tcl_lexer::ExprToken<Vec<u8>>],
    candidate_tokens: &[tcl_lexer::ExprToken<Vec<u8>>],
) -> bool {
    let mut pending = vec![(original, candidate)];
    let mut visited = 0_u32;
    while let Some((old, new)) = pending.pop() {
        visited += 1;
        if visited > 4096 {
            return false;
        }
        match (old, new) {
            (
                ExprNode::Literal {
                    text: old_text,
                    start: old_start,
                    end: old_end,
                },
                ExprNode::Literal {
                    text: new_text,
                    start: new_start,
                    end: new_end,
                },
            )
            | (
                ExprNode::String {
                    text: old_text,
                    start: old_start,
                    end: old_end,
                },
                ExprNode::String {
                    text: new_text,
                    start: new_start,
                    end: new_end,
                },
            ) => {
                if old_text != new_text
                    || !original_tokens
                        .iter()
                        .zip(candidate_tokens)
                        .any(|(old, new)| {
                            old.start == *old_start
                                && old.end == *old_end
                                && new.start == *new_start
                                && new.end == *new_end
                                && old.text == *old_text
                                && new.text == *new_text
                                && old.kind == new.kind
                        })
                {
                    return false;
                }
            }
            (
                ExprNode::Unary {
                    op: old_op,
                    operand: old_operand,
                },
                ExprNode::Unary {
                    op: new_op,
                    operand: new_operand,
                },
            ) if old_op == new_op => pending.push((old_operand, new_operand)),
            (
                ExprNode::Binary {
                    op: old_op,
                    left: old_left,
                    right: old_right,
                },
                ExprNode::Binary {
                    op: new_op,
                    left: new_left,
                    right: new_right,
                },
            ) if old_op == new_op => {
                pending.push((old_right, new_right));
                pending.push((old_left, new_left));
            }
            (
                ExprNode::Ternary {
                    condition: old_condition,
                    true_branch: old_true,
                    false_branch: old_false,
                },
                ExprNode::Ternary {
                    condition: new_condition,
                    true_branch: new_true,
                    false_branch: new_false,
                },
            ) => {
                pending.push((old_false, new_false));
                pending.push((old_true, new_true));
                pending.push((old_condition, new_condition));
            }
            _ => return false,
        }
    }
    true
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
        let envelope = self.original_expression_rewrite_envelope(tokens, registry)?;
        original_literal_bracing_operand(self, tokens, registry, envelope, ExpressionRewrite::Brace)
    }

    /// Remove only verified token gaps from an already braced, pure original
    /// expression. The existing bracing owner still closes dispatch, observers,
    /// source reflection and finite Native completion independently.
    #[must_use]
    pub fn original_expression_gap_compaction(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
        candidate: &str,
    ) -> Option<OriginalExpressionGapCompaction> {
        let envelope = self.original_expression_rewrite_envelope(tokens, registry)?;
        let expression = original_literal_bracing_operand(
            self,
            tokens,
            registry,
            envelope,
            ExpressionRewrite::Gaps(candidate),
        )?;
        let content = expression.original_operand().content_span().ok()?;
        let gaps = whitespace_gap_spans(&expression.native, candidate.as_bytes(), content.start())?;
        Some(OriginalExpressionGapCompaction { expression, gaps })
    }

    fn original_registry_syntax_envelope(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<ExpressionRewriteEnvelope<'_>> {
        if tokens.source_binding.as_ref() != Some(self) || !self.unobserved_native_dispatch() {
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
        if !self.original_compiler_names_preserved_for_tokens(tokens) {
            return brace_refusal(tokens, "compiler-name-effects");
        }
        Some(ExpressionRewriteEnvelope {
            site,
            config,
            target,
            state,
        })
    }

    /// Exact lexical gaps under the selected quiet handler. Source-role
    /// admission belongs to the caller; no value or completion proof follows.
    pub(crate) fn original_expression_syntax_gaps(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
        operand: &NativeWord,
        candidate: &str,
    ) -> Option<Vec<Span>> {
        let envelope = self.original_registry_syntax_envelope(tokens, registry)?;
        if input.lexer_config() != envelope.config
            || operand.config() != envelope.config
            || operand.image() != envelope.site.source.source_image()
            || operand.group().kind != WordKind::Braced
            || operand.group().expand
        {
            return None;
        }
        let original = crate::registry_invocation::original_native_compiler_words(
            envelope.site.source.source_image(),
            tokens.words(),
            envelope.site.offset,
            envelope.config,
        )?;
        if !original.contains(operand) {
            return None;
        }
        let content = operand.content_span().ok()?;
        let bytes = operand.image().bytes().get(content.as_range())?;
        let mut parser = ExprParseContext::for_profile(input.unit_profile());
        parser.lexer_grammar = envelope.config.grammar_over(parser.lexer_grammar);
        let original_tokens = expression_syntax_signature(bytes, &parser, envelope.config)?;
        let candidate_tokens =
            expression_syntax_signature(candidate.as_bytes(), &parser, envelope.config)?;
        if original_tokens
            .iter()
            .map(|token| (&token.kind, &token.text))
            .ne(candidate_tokens
                .iter()
                .map(|token| (&token.kind, &token.text)))
            || !matches!(
                tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(bytes, &parser),
                CheckedExprParse::Parsed(_)
            )
            || !matches!(
                tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(
                    candidate.as_bytes(),
                    &parser
                ),
                CheckedExprParse::Parsed(_)
            )
        {
            return None;
        }
        whitespace_gap_spans(bytes, candidate.as_bytes(), content.start())
    }

    fn original_expression_rewrite_envelope(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<ExpressionRewriteEnvelope<'_>> {
        let envelope = self.original_registry_syntax_envelope(tokens, registry)?;
        if tokens.words().len() != 2 {
            return None;
        }
        let ExpressionRewriteEnvelope {
            site,
            config,
            target,
            state,
        } = envelope;
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
        Some(ExpressionRewriteEnvelope {
            site,
            config,
            target,
            state,
        })
    }
}

fn original_literal_bracing_operand(
    binding: &SourceInvocationBinding,
    tokens: &CommandTokens,
    registry: &CommandRegistry,
    envelope: ExpressionRewriteEnvelope<'_>,
    rewrite: ExpressionRewrite<'_>,
) -> Option<OriginalLiteralExpressionBracing> {
    let ExpressionRewriteEnvelope {
        site,
        config,
        target,
        state,
    } = envelope;
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
        || match rewrite {
            ExpressionRewrite::Brace => original[1].group().kind == WordKind::Braced,
            ExpressionRewrite::Gaps(_) => original[1].group().kind != WordKind::Braced,
        }
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
    let replacement_bytes = match rewrite {
        ExpressionRewrite::Brace => native,
        ExpressionRewrite::Gaps(candidate) => {
            let content = original[1].content_span().ok()?;
            if original[1].image().bytes().get(content.as_range())? != native
                || !same_expression_gaps(native, candidate.as_bytes(), &parser, config, &tree)
            {
                return None;
            }
            candidate.as_bytes()
        }
    };
    let replacement = brace_required(
        tokens,
        "candidate-roundtrip",
        braced_source_word(
            replacement_bytes,
            site.source.source_image(),
            config,
            policy,
        ),
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

    fn gap_receipt(source: &str, candidate: &str) -> Option<OriginalExpressionGapCompaction> {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config?;
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        crate::registry_invocation::source_structure::original_expression_gap_compaction(
            source,
            &analysis,
            commands.last()?,
            candidate,
        )
    }

    #[test]
    fn syntax_expression_gaps_keep_token_data_and_dispatch_guards() {
        let receipt = |source: &str, candidate: &str| {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            let config = analysis.body_lexer_config?;
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            crate::registry_invocation::source_structure::original_expression_syntax_gap_compaction(
                source,
                &analysis,
                commands.last()?,
                0,
                candidate,
            )
        };
        let source = "set a 2; expr {$a + 1}";
        let proof = receipt(source, "$a+1").expect("selected syntax gaps without erasing reads");
        assert!(!proof.removed_gaps().is_empty());
        assert!(proof.matches_source(
            &SourceImage::document(source),
            proof.original_operand().config()
        ));
        for (source, candidate) in [
            ("expr {{a  b} eq {a  b}}", "{a b} eq {a b}"),
            ("expr {1 + 2}", "1+3"),
            ("expr {1 +}", "1+"),
            ("proc expr args {}; expr {1 + 2}", "1+2"),
            ("unknown_mutation; expr {1 + 2}", "1+2"),
            (
                "trace add execution expr enter callback; expr {1 + 2}",
                "1+2",
            ),
        ] {
            assert!(
                receipt(source, candidate).is_none(),
                "{source} -> {candidate}"
            );
        }
    }

    #[test]
    fn original_expression_gap_tree_checks_issued_leaf_positions() {
        let parser =
            ExprParseContext::for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let config = LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
        let original = b" 1 + 2 * 3 ";
        let CheckedExprParse::Parsed(tree) =
            tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(original, &parser)
        else {
            panic!("original checked Native tree");
        };
        assert!(same_expression_gaps(
            original, b"1+2*3", &parser, config, &tree
        ));
        for candidate in [b"1+2*4".as_slice(), b"1-2*3", b"1+2* 3 + 0"] {
            assert!(!same_expression_gaps(
                original, candidate, &parser, config, &tree
            ));
        }
        let mut wrong_position = tree.clone();
        let ExprNode::Binary { left, .. } = &mut wrong_position else {
            panic!("original binary");
        };
        let ExprNode::Literal { start, .. } = left.as_mut() else {
            panic!("original literal");
        };
        *start += 1;
        assert!(!same_expression_gaps(
            original,
            b"1+2*3",
            &parser,
            config,
            &wrong_position
        ));
        let strings = b"{a  b} eq {a  b}";
        let CheckedExprParse::Parsed(tree) =
            tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(strings, &parser)
        else {
            panic!("original checked strings");
        };
        assert!(!same_expression_gaps(
            strings,
            b"{a b}eq{a b}",
            &parser,
            config,
            &tree
        ));
    }

    #[test]
    fn original_expression_gaps_retain_pure_native_completion_and_exact_input() {
        let source = "expr { 1 + 2 * 3 }";
        let proof = gap_receipt(source, "1+2*3").expect("pure original finite expression");
        assert_eq!(proof.replacement_source_word(), "{1+2*3}");
        assert_eq!(proof.original_operand().bytes(), b"{ 1 + 2 * 3 }");
        assert!(!proof.removed_gaps().is_empty());
        let config = proof.original_operand().config();
        assert!(proof.matches_source(&SourceImage::document(source), config));
        let mut changed = config;
        changed.strict_quoting = !changed.strict_quoting;
        assert!(!proof.matches_source(&SourceImage::document(source), changed));
        assert!(!proof.matches_source(&SourceImage::document("expr {1+2*3}"), config));
        let mut changed_registry = CommandRegistry::build_default();
        changed_registry.insert_ambient_package("gap-test", "1");
        assert!(!proof.matches_registry(&changed_registry));
        for candidate in ["1+3*3", "1 + 2 * 3 + 0", "1+2*"] {
            assert!(gap_receipt(source, candidate).is_none(), "{candidate}");
        }
        // Whitespace within an expression string is token data, never a gap.
        assert!(gap_receipt("expr {{a  b} eq {a  b}}", "{a b}eq{a b}").is_none());
    }

    #[test]
    fn original_expression_gaps_decline_reads_callbacks_errors_and_source_reflection() {
        for (source, candidate) in [
            ("expr {$a + 1}", "$a+1"),
            ("expr {[callback] + 1}", "[callback]+1"),
            ("expr {abs( 1 )}", "abs(1)"),
            ("expr {1 / 0}", "1/0"),
            ("expr {1 +}", "1+"),
            ("expr {1 + 2}; info frame 0", "1+2"),
            ("proc p {} {expr {1 + 2}}; p", "1+2"),
            (
                "trace add execution expr enter callback; expr {1 + 2}",
                "1+2",
            ),
            ("unknown_mutation; expr {1 + 2}", "1+2"),
            ("proc expr args {}; expr {1 + 2}", "1+2"),
            (
                "expr {1\\
 + 2}",
                "1+2",
            ),
        ] {
            assert!(gap_receipt(source, candidate).is_none(), "{source}");
        }
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
        // docs/design/analysis/name-resolution-proofs/braced-error-context-publication.md
        // docs/design/analysis/name-resolution-proofs/braced-handler-and-observer-boundaries.md

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
        // docs/design/analysis/name-resolution-proofs/braced-error-context-publication.md
        // docs/design/analysis/name-resolution-proofs/braced-handler-and-observer-boundaries.md

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
