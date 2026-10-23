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

//! Closed BPF DSL metadata retained through source and typed IR transforms.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use tcl_lexer::{
    LexerConfig, NativeWord, SourceImage, SourceMap, SourceSegmentationPurpose, Span, TokenType,
};
use tcl_registry::bpf_op::BpfOpKind;
use tcl_registry::{CommandRegistry, CommandSpec, RegistrySemanticKey, RegistrySnapshot};

use super::{CommandTokens, Module, WordExpr, WordPart};

/// Actual retained Module source/context before closed-DSL phase construction.
/// This authenticates the original input, independently of mutable IR fields,
/// command inventory, transformation authority and Native execution.
#[derive(Debug, Clone)]
pub struct SourceBpfModuleContext {
    source: SourceImage,
    config: LexerConfig,
    registry: RegistrySnapshot,
    retained: Arc<crate::command_binding::RetainedSourceModuleBindings>,
}

impl SourceBpfModuleContext {
    /// Check the actual retained Module owner, including source channel, whole
    /// configuration, entry and namespace context. Script correspondence and
    /// complete phase coverage remain separate obligations.
    #[must_use]
    pub fn matches_module(&self, module: &Module, registry: &CommandRegistry) -> bool {
        module
            .retained_source_bindings
            .as_ref()
            .is_some_and(|retained| Arc::ptr_eq(retained, &self.retained))
            && self.source == module.source
            && self.config == module.native_lexer_config()
            && self.registry.semantic_key() == registry.snapshot().semantic_key()
            && self
                .retained
                .positioned_for_module(module, registry)
                .is_some()
    }
}

/// Proof that one exact, fully expanded closed BPF script performs no Tcl
/// command dispatch. This does not prove original-script census, transformation
/// correctness, target-program admission, or Native execution/effects.
#[derive(Debug, Clone)]
pub struct SourceBpfClosedEffects {
    context: SourceBpfModuleContext,
    script: super::Script,
    phase_config: LexerConfig,
}

impl SourceBpfModuleContext {
    /// Certify every node of the supplied final phase against actual BPF
    /// selections. The phase owner still proves its original source census and
    /// transformations; this witness only separates closed operations from Tcl.
    #[must_use]
    pub fn closed_effects_for(
        &self,
        module: &Module,
        registry: &CommandRegistry,
        script: &super::Script,
        phase_config: LexerConfig,
    ) -> Option<SourceBpfClosedEffects> {
        self.matches_module(module, registry).then_some(())?;
        let mut bodies = vec![(script, 1)];
        while let Some((body, depth)) = bodies.pop() {
            // Refuse rather than certify a phase the unchanged builder truncates.
            if crate::depth_guard::MAX_SOURCE_NEST_DEPTH.exceeded(depth) {
                return None;
            }
            // Closed phases cannot borrow independent Native advice.
            if body.executed_source.is_some()
                || !body.implicit_math_invocations.is_empty()
                || !body.expression_preparations.is_empty()
                || body.command_binding_sites.iter().next().is_some()
                || body.procedure_binding_requirements.iter().next().is_some()
                || body.native_compilation_failure.is_some()
                || body.native_compilation_admission.is_some()
            {
                return None;
            }
            for statement in &body.statements {
                match statement {
                    super::Statement::Call {
                        args,
                        tokens: Some(tokens),
                        foreach_groups: None,
                        ..
                    } => {
                        let selected = tokens.source_bpf_command.as_ref()?;
                        selected.closed_operation(tokens, args, self, registry)?;
                    }
                    super::Statement::If {
                        clauses, else_body, ..
                    } => {
                        for clause in clauses {
                            pure_bpf_condition(&clause.condition).then_some(())?;
                            bodies.push((&clause.body, depth + 1));
                        }
                        if let Some(body) = else_body {
                            bodies.push((body, depth + 1));
                        }
                    }
                    _ => return None,
                }
            }
        }
        Some(SourceBpfClosedEffects {
            context: self.clone(),
            script: script.clone(),
            phase_config,
        })
    }
}

impl SourceBpfClosedEffects {
    pub(crate) fn matches(
        &self,
        module: &Module,
        registry: &CommandRegistry,
        script: &super::Script,
        config: LexerConfig,
    ) -> bool {
        self.context.matches_module(module, registry)
            && self.phase_config == config
            && self.script == *script
    }
}

// Operator/program support remains the BPF lowerer's independent obligation.
// Literal operator trees contain no Tcl command execution. Variable nodes
// have no BPF register-owner proof here: scalar traces and indexed substitutions
// cannot be ruled out by syntax alone, so all variable nodes refuse.
fn pure_bpf_condition(condition: &crate::expr_ast::ExprNode) -> bool {
    use crate::expr_ast::ExprNode;
    let mut pending = vec![(condition, 1)];
    while let Some((node, depth)) = pending.pop() {
        if crate::depth_guard::MAX_EXPR_NODE_DEPTH.exceeded(depth) {
            return false;
        }
        match node {
            ExprNode::Literal { .. } => {}
            ExprNode::Binary { left, right, .. } => {
                pending.push((left, depth + 1));
                pending.push((right, depth + 1));
            }
            ExprNode::Unary { operand, .. } => pending.push((operand, depth + 1)),
            _ => return false,
        }
    }
    true
}

impl Module {
    /// Retain the genuine original input owner for closed-DSL source phases.
    /// Missing or stale metadata cannot be reconstructed from Module labels.
    #[must_use]
    pub fn source_bpf_module_context(
        &self,
        registry: &CommandRegistry,
    ) -> Option<SourceBpfModuleContext> {
        let retained = self.retained_source_bindings.as_ref()?;
        retained.positioned_for_module(self, registry)?;
        Some(SourceBpfModuleContext {
            source: self.source.clone(),
            config: self.native_lexer_config(),
            registry: self.registry_snapshot.clone()?,
            retained: Arc::clone(retained),
        })
    }
}

/// A selected closed-DSL source command or explicitly generated operation.
/// An owning phase must separately authenticate its current source/configuration
/// and complete script. This retains source metadata only; it grants no Tcl
/// dispatch, Native entry, effects, or script coverage.
#[derive(Debug, Clone)]
pub struct SourceBpfCommand(Arc<SourceBpfCommandData>);

#[derive(Debug)]
struct SourceBpfCommandData {
    registry: RegistrySemanticKey,
    selected: Option<&'static CommandSpec>,
    head: String,
    origin: BpfCommandOrigin,
    current: BpfWordSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum BpfCommandOrigin {
    Original {
        words: Vec<NativeWord>,
        region: Span,
        purpose: SourceSegmentationPurpose,
    },
    // The actual selected row retains the requested typed operation; these
    // words are DSL data, independently of Tcl substitution or source spans.
    Generated,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BpfWordSnapshot {
    argv: Vec<Span>,
    argv_texts: Vec<String>,
    word_exprs: Vec<WordExpr>,
    argv_kinds: Vec<TokenType>,
    single_token_word: Vec<bool>,
    all_tokens: Vec<Span>,
    expand_word: Option<Vec<bool>>,
}

impl BpfWordSnapshot {
    fn new(tokens: &CommandTokens) -> Self {
        Self {
            argv: tokens.argv.clone(),
            argv_texts: tokens.argv_texts.clone(),
            word_exprs: tokens.word_exprs.clone(),
            argv_kinds: tokens.argv_kinds.clone(),
            single_token_word: tokens.single_token_word.clone(),
            all_tokens: tokens.all_tokens.clone(),
            expand_word: tokens.expand_word.clone(),
        }
    }

    fn matches(&self, tokens: &CommandTokens) -> bool {
        self.argv == tokens.argv
            && self.argv_texts == tokens.argv_texts
            && self.word_exprs == tokens.word_exprs
            && self.argv_kinds == tokens.argv_kinds
            && self.single_token_word == tokens.single_token_word
            && self.all_tokens == tokens.all_tokens
            && self.expand_word == tokens.expand_word
    }

    fn tokens(&self) -> CommandTokens {
        let mut tokens = CommandTokens::from_lossy_parts(
            self.argv.clone(),
            self.argv_texts.clone(),
            self.argv_kinds.clone(),
            self.single_token_word.clone(),
            self.all_tokens.clone(),
            self.expand_word.clone(),
        );
        tokens.word_exprs.clone_from(&self.word_exprs);
        tokens
    }

    fn rebased(&self, delta: i64) -> Option<Self> {
        if !self
            .argv
            .iter()
            .chain(&self.all_tokens)
            .copied()
            .all(|span| span_can_shift(span, delta))
            || !self
                .word_exprs
                .iter()
                .all(|word| word_can_shift(word, delta))
        {
            return None;
        }
        let mut tokens = self.tokens();
        crate::lattice_rebase::rebase_command_tokens(&mut tokens, delta);
        let after = Self::new(&tokens);
        crate::lattice_rebase::rebase_command_tokens(&mut tokens, delta.checked_neg()?);
        // The compatibility rebaser clamps. A source receipt cannot survive
        // any clamped geometry, even if its readable words are unchanged.
        (Self::new(&tokens) == *self).then_some(after)
    }
}

fn span_can_shift(span: Span, delta: i64) -> bool {
    [span.start(), span.end()].into_iter().all(|offset| {
        i64::from(offset)
            .checked_add(delta)
            .and_then(|shifted| u32::try_from(shifted).ok())
            .is_some()
    })
}

fn word_can_shift(word: &WordExpr, delta: i64) -> bool {
    if !span_can_shift(word.source().span, delta) {
        return false;
    }
    match word {
        WordExpr::Template { parts, .. } => parts.iter().all(|part| {
            let source = match part {
                WordPart::Text { source, .. }
                | WordPart::Variable { source, .. }
                | WordPart::CommandSubstitution { source, .. }
                | WordPart::Opaque { source, .. } => source,
            };
            span_can_shift(source.span, delta)
        }),
        WordExpr::Expand { word, .. } => word_can_shift(word, delta),
        _ => true,
    }
}

impl PartialEq for SourceBpfCommand {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.0.registry == other.0.registry
                && selected_identity(self.0.selected) == selected_identity(other.0.selected)
                && self.0.head == other.0.head
                && self.0.origin == other.0.origin
                && self.0.current == other.0.current)
    }
}
impl Eq for SourceBpfCommand {}
impl Hash for SourceBpfCommand {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.registry.hash(state);
        selected_identity(self.0.selected).hash(state);
        self.0.head.hash(state);
        self.0.origin.hash(state);
        self.0.current.hash(state);
    }
}

// These are actual rows selected from the retained immutable registry, rather
// than pointers to independently promoted catalogue constants.
fn selected_identity(selected: Option<&CommandSpec>) -> Option<usize> {
    selected.map(|spec| std::ptr::from_ref(spec) as usize)
}

impl SourceBpfCommand {
    /// Reborrow the originally selected row under the same complete registry
    /// and unchanged current argv. `Some(None)` means the retained static head
    /// has no selected catalogue descriptor under that registry. This grants
    /// no runtime absence. `None` means representation/context is unavailable.
    #[must_use]
    pub fn descriptor(
        &self,
        tokens: &CommandTokens,
        args: &[String],
        registry: &CommandRegistry,
    ) -> Option<Option<&'static CommandSpec>> {
        self.is_current(tokens, args, registry)
            .then_some(self.0.selected)
    }

    /// Actual static source value or generated descriptor label for display
    /// and closed-DSL field selection, without reconstructing command identity.
    #[must_use]
    pub fn head<'a>(
        &'a self,
        tokens: &CommandTokens,
        args: &[String],
        registry: &CommandRegistry,
    ) -> Option<&'a str> {
        self.is_current(tokens, args, registry)
            .then_some(self.0.head.as_str())
    }

    /// Original lexical words retain their original image, channel and full
    /// configuration even when an actual affine IR rebase has moved the sites.
    #[must_use]
    pub fn original_words<'a>(
        &'a self,
        tokens: &CommandTokens,
        args: &[String],
        registry: &CommandRegistry,
    ) -> Option<&'a [NativeWord]> {
        self.is_current(tokens, args, registry).then_some(())?;
        match &self.0.origin {
            BpfCommandOrigin::Original { words, .. } => Some(words),
            BpfCommandOrigin::Generated => None,
        }
    }

    fn closed_operation(
        &self,
        tokens: &CommandTokens,
        args: &[String],
        context: &SourceBpfModuleContext,
        registry: &CommandRegistry,
    ) -> Option<()> {
        let selected = self.descriptor(tokens, args, registry)??;
        match selected.bpf_op?.kind {
            BpfOpKind::ScalarSet(_)
            | BpfOpKind::BindPacket
            | BpfOpKind::PacketLoad { .. }
            | BpfOpKind::PacketLen
            | BpfOpKind::MapDeclare
            | BpfOpKind::MapGet
            | BpfOpKind::MapSet
            | BpfOpKind::MapHas
            | BpfOpKind::Verdict(_) => {}
            BpfOpKind::Conditional | BpfOpKind::LoopMacro | BpfOpKind::Framework(_) => return None,
        }
        if tokens.source_binding.is_some()
            || !tokens.nested_bindings.is_empty()
            || !tokens.variable_accesses.is_empty()
            || tokens.evaluated_body.is_some()
        {
            return None;
        }
        match &self.0.origin {
            BpfCommandOrigin::Original {
                words,
                purpose,
                region,
            } => words
                .iter()
                .all(|word| {
                    word.image() == &context.source
                        && word.config() == purpose.config(context.config)
                        && region.start() <= word.span().start()
                        && word.span().end() <= region.end()
                        && tcl_syntax::word_rules::original_static_word_unicode_value(word)
                            .is_some()
                })
                .then_some(()),
            BpfCommandOrigin::Generated => Some(()),
        }
    }

    fn is_current(
        &self,
        tokens: &CommandTokens,
        args: &[String],
        registry: &CommandRegistry,
    ) -> bool {
        self.0.registry == registry.snapshot().semantic_key()
            && self.matches_tokens(tokens)
            && self.0.current.argv_texts.get(1..) == Some(args)
    }

    pub(crate) fn matches_tokens(&self, tokens: &CommandTokens) -> bool {
        tokens.synthetic.is_none() && self.0.current.matches(tokens)
    }

    pub(crate) fn rebased(&self, delta: i64) -> Option<Self> {
        Some(Self(Arc::new(SourceBpfCommandData {
            registry: self.0.registry.clone(),
            selected: self.0.selected,
            head: self.0.head.clone(),
            origin: self.0.origin.clone(),
            current: self.0.current.rebased(delta)?,
        })))
    }
}

impl CommandTokens {
    /// Project a complete actual command from the selected original region and
    /// grammar. This carries source syntax only, never execution or storage.
    #[must_use]
    pub fn original_configured_command_words(
        &self,
        image: SourceImage,
        region: Span,
        config: LexerConfig,
        purpose: SourceSegmentationPurpose,
        args: &[String],
    ) -> Option<Vec<NativeWord>> {
        if self.synthetic.is_some()
            || !self.original_words_match_argv_text()
            || self.words().len() != args.len().checked_add(1)?
            || self.argv_texts.get(1..) != Some(args)
            || self
                .expand_word
                .as_ref()
                .is_some_and(|flags| flags.iter().any(|flag| *flag))
        {
            return None;
        }
        let text = std::str::from_utf8(image.bytes().get(region.as_range())?).ok()?;
        let first = self
            .words()
            .first()?
            .original_written_span(text.as_bytes())?;
        let expected_head = Span::new(
            region.start().checked_add(first.start())?,
            region.start().checked_add(first.end())?,
        );
        let plan =
            tcl_lexer::native_script_words_in_with_purpose(image.clone(), region, config, purpose)
                .ok()?;
        let command = plan.commands.into_iter().find(|command| {
            command
                .words
                .first()
                .is_some_and(|word| word.span() == expected_head)
        })?;
        if command.words.len() != self.words().len() {
            return None;
        }
        let map = SourceMap::new(image.try_text().ok()?);
        for (actual, supplied) in command.words.iter().zip(self.words()) {
            let local = supplied.original_written_span(text.as_bytes())?;
            let expected = Span::new(
                region.start().checked_add(local.start())?,
                region.start().checked_add(local.end())?,
            );
            if actual.span() != expected {
                return None;
            }
            let mut parsed = WordExpr::from_word(
                &map,
                config,
                actual.tokens(),
                &supplied.legacy_text(),
                false,
                None,
            );
            crate::lattice_rebase::rebase_source_word(&mut parsed, -i64::from(region.start()));
            if parsed != *supplied {
                return None;
            }
        }
        Some(command.words)
    }

    /// Capture actual original source and the catalogue row selected by its
    /// static head under this registry. Unknown static heads retain lexical
    /// provenance; dynamic heads and missing ownership refuse publication.
    pub fn install_source_bpf_command(
        &mut self,
        image: SourceImage,
        region: Span,
        config: LexerConfig,
        purpose: SourceSegmentationPurpose,
        args: &[String],
        registry: &CommandRegistry,
    ) -> bool {
        self.source_bpf_command = None;
        let Some(words) =
            self.original_configured_command_words(image, region, config, purpose, args)
        else {
            return false;
        };
        let Some(head) = words
            .first()
            .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)
            .and_then(|bytes| String::from_utf8(bytes).ok())
        else {
            return false;
        };
        self.source_bpf_command = Some(SourceBpfCommand(Arc::new(SourceBpfCommandData {
            registry: registry.snapshot().semantic_key(),
            selected: registry.get_for_surface(&head, registry.own_surface_query()),
            head,
            origin: BpfCommandOrigin::Original {
                words,
                region,
                purpose,
            },
            current: BpfWordSnapshot::new(self),
        })));
        true
    }

    /// Build an explicitly generated BPF operation from its typed operation
    /// and immutable data arguments. This grants no original source geometry
    /// and does not apply Tcl substitution to the generated data.
    #[must_use]
    pub fn generated_bpf_operation(
        operation: BpfOpKind,
        args: &[String],
        span: Span,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let head = registry.bpf_command_for(operation)?;
        let selected = registry.get_for_surface(head, registry.own_surface_query())?;
        (selected.bpf_op?.kind == operation).then_some(())?;
        let words = std::iter::once(head.to_owned())
            .chain(args.iter().cloned())
            .collect::<Vec<_>>();
        let count = words.len();
        let mut tokens = Self::from_lossy_parts(
            vec![span; count],
            words,
            vec![TokenType::Esc; count],
            vec![false; count],
            vec![span; count],
            None,
        );
        tokens.source_bpf_command = Some(SourceBpfCommand(Arc::new(SourceBpfCommandData {
            registry: registry.snapshot().semantic_key(),
            selected: Some(selected),
            head: head.to_owned(),
            origin: BpfCommandOrigin::Generated,
            current: BpfWordSnapshot::new(&tokens),
        })));
        Some(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Statement;
    use tcl_registry::bpf_op::{BpfScalarWidth, BpfVerdictKind};

    fn closed_phase_fixture(registry: &CommandRegistry) -> (Module, super::super::Script) {
        let config = LexerConfig::for_profile(registry.profile());
        let module = crate::lowering::lower_to_ir_with_dialect(
            "accept",
            registry,
            config,
            registry.profile(),
        );
        let args = Vec::new();
        let tokens = CommandTokens::generated_bpf_operation(
            BpfOpKind::Verdict(BpfVerdictKind::Accept),
            &args,
            Span::empty(0),
            registry,
        )
        .unwrap();
        let script = super::super::Script::from_statements(vec![Statement::Call {
            span: Span::empty(0),
            command: "accept".to_owned(),
            canonical_command: None,
            args,
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(tokens),
            foreach_groups: None,
        }]);
        (module, script)
    }

    #[test]
    fn closed_bpf_effects_keep_script_module_registry_and_config_current() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let (module, script) = closed_phase_fixture(registry);
        let config = module.native_lexer_config();
        let context = module.source_bpf_module_context(registry).unwrap();
        let effects = context
            .closed_effects_for(&module, registry, &script, config)
            .unwrap();
        let build = |script, module, registry, config| {
            crate::cfg_builder::build_cfg_function_in_bpf_selected_scope(
                "generated-label",
                script,
                module,
                registry,
                config,
                &effects,
            )
        };
        assert!(build(&script, &module, registry, config).is_some());
        let mut changed = script.clone();
        changed.statements.push(script.statements[0].clone());
        assert!(build(&changed, &module, registry, config).is_none());
        let (foreign, _) = closed_phase_fixture(registry);
        assert!(build(&script, &foreign, registry, config).is_none());
        assert!(build(&script, &module, &CommandRegistry::build_default(), config).is_none());
        let mut changed_config = config;
        changed_config.strict_quoting = !changed_config.strict_quoting;
        assert!(build(&script, &module, registry, changed_config).is_none());
    }

    #[test]
    fn closed_bpf_effects_refuse_missing_selection_and_effectful_conditions() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let (module, script) = closed_phase_fixture(registry);
        let config = module.native_lexer_config();
        let context = module.source_bpf_module_context(registry).unwrap();
        assert!(
            context
                .closed_effects_for(&module, registry, &script, config)
                .is_some()
        );
        let mut unselected = script.clone();
        unselected.statements[0]
            .tokens_mut()
            .unwrap()
            .source_bpf_command = None;
        assert!(
            context
                .closed_effects_for(&module, registry, &unselected, config)
                .is_none()
        );
        let conditional = super::super::Script::from_statements(vec![Statement::If {
            span: Span::empty(0),
            clauses: vec![super::super::IfClause {
                condition: crate::parse_expr("[unknown]", None),
                condition_span: Span::empty(0),
                condition_base: None,
                body: script,
                body_span: Span::empty(0),
            }],
            else_body: None,
            else_span: None,
        }]);
        assert!(
            context
                .closed_effects_for(&module, registry, &conditional, config)
                .is_none()
        );
    }

    #[test]
    fn closed_bpf_effects_refuse_over_budget_scalar_tree() {
        use crate::expr_ast::{ExprNode, UnaryOp};
        let mut condition = crate::parse_expr("1", None);
        assert!(pure_bpf_condition(&condition));
        for source in ["$arr([step])", "$value"] {
            assert!(
                !pure_bpf_condition(&crate::parse_expr(source, None)),
                "{source}"
            );
        }
        for _ in 0..crate::depth_guard::MAX_EXPR_NODE_DEPTH.0 {
            condition = ExprNode::Unary {
                op: UnaryOp::Pos,
                operand: Box::new(condition),
            };
        }
        assert!(!pure_bpf_condition(&condition));
    }

    struct Fixture {
        tokens: CommandTokens,
        args: Vec<String>,
        image: SourceImage,
        config: LexerConfig,
        region: Span,
    }

    fn fixture(source: &str, registry: &CommandRegistry) -> Fixture {
        let module = crate::lowering::lower_to_ir_with_dialect(
            source,
            registry,
            LexerConfig::for_profile(registry.profile()),
            registry.profile(),
        );
        let Statement::Call {
            tokens: Some(tokens),
            args,
            ..
        } = &module.top_level.statements[0]
        else {
            panic!("actual source command");
        };
        Fixture {
            tokens: tokens.clone(),
            args: args.clone(),
            image: module.source.clone(),
            config: module.native_lexer_config(),
            region: Span::new(0, u32::try_from(source.len()).unwrap()),
        }
    }

    fn install(fixture: &mut Fixture, registry: &CommandRegistry) -> bool {
        fixture.tokens.install_source_bpf_command(
            fixture.image.clone(),
            fixture.region,
            fixture.config,
            SourceSegmentationPurpose::WholeInput,
            &fixture.args,
            registry,
        )
    }

    #[test]
    fn original_bpf_module_context_refuses_replaced_source_configuration_or_registry() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let module = crate::lowering::lower_to_ir_with_dialect(
            "drop",
            registry,
            LexerConfig::for_profile(registry.profile()),
            registry.profile(),
        );
        let context = module.source_bpf_module_context(registry).unwrap();
        assert!(context.matches_module(&module, registry));
        for mutate in [
            (|module: &mut Module| module.source = SourceImage::document("pass"))
                as fn(&mut Module),
            |module| module.source = SourceImage::native(module.source.bytes()),
            |module| module.lexer_config.strict_quoting = !module.lexer_config.strict_quoting,
            |module| module.registry_snapshot = None,
            |module| module.retained_source_bindings = None,
        ] {
            let mut changed = module.clone();
            mutate(&mut changed);
            assert!(!context.matches_module(&changed, registry));
            assert!(changed.source_bpf_module_context(registry).is_none());
        }
        assert!(!context.matches_module(&module, &CommandRegistry::build_default()));
    }

    #[test]
    fn original_bpf_selection_retains_actual_source_and_refuses_changed_vectors_or_registry() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let mut original = fixture("::drop", registry);
        assert!(install(&mut original, registry));
        let receipt = original.tokens.source_bpf_command.clone().unwrap();
        assert!(matches!(
            receipt
                .descriptor(&original.tokens, &original.args, registry)
                .unwrap()
                .unwrap()
                .bpf_op
                .unwrap()
                .kind,
            BpfOpKind::Verdict(BpfVerdictKind::Drop)
        ));
        assert_eq!(
            receipt
                .original_words(&original.tokens, &original.args, registry)
                .unwrap()[0]
                .image(),
            &original.image
        );
        let mut changed = original.tokens.clone();
        let WordExpr::Literal { text, .. } = &mut changed.word_exprs[0] else {
            panic!("actual literal head");
        };
        *text = "pass".into();
        changed.argv_texts[0] = "pass".into();
        assert!(
            receipt
                .descriptor(&changed, &original.args, registry)
                .is_none()
        );
        assert!(
            receipt
                .descriptor(
                    &original.tokens,
                    &original.args,
                    &CommandRegistry::build_default()
                )
                .is_none()
        );
        assert!(!changed.install_source_bpf_command(
            original.image.clone(),
            original.region,
            original.config,
            SourceSegmentationPurpose::WholeInput,
            &original.args,
            registry
        ));

        let mut unknown = fixture("custom_field destination", registry);
        assert!(install(&mut unknown, registry));
        assert!(
            unknown
                .tokens
                .source_bpf_command
                .as_ref()
                .unwrap()
                .descriptor(&unknown.tokens, &unknown.args, registry)
                .unwrap()
                .is_none()
        );
        let mut forged = fixture("pass", registry);
        forged.tokens.word_exprs[0] =
            WordExpr::from_lossy_snapshot("pass", Span::new(0, 4), TokenType::Esc, true, false);
        assert!(!install(&mut forged, registry));
    }

    #[test]
    fn bpf_origin_rebase_is_affine_and_refuses_changed_or_clamped_geometry() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let mut original = fixture("drop", registry);
        assert!(install(&mut original, registry));
        let old_word = original
            .tokens
            .source_bpf_command
            .as_ref()
            .unwrap()
            .original_words(&original.tokens, &original.args, registry)
            .unwrap()[0]
            .clone();
        crate::lattice_rebase::rebase_command_tokens(&mut original.tokens, 11);
        let receipt = original.tokens.source_bpf_command.as_ref().unwrap();
        assert!(
            receipt
                .descriptor(&original.tokens, &original.args, registry)
                .is_some()
        );
        assert_eq!(
            receipt
                .original_words(&original.tokens, &original.args, registry)
                .unwrap()[0],
            old_word
        );
        assert_eq!(original.tokens.words()[0].source().span.start(), 11);
        let mut changed = original.tokens.clone();
        changed.argv[0] = Span::empty(0);
        crate::lattice_rebase::rebase_command_tokens(&mut changed, 1);
        assert!(changed.source_bpf_command.is_none());
        crate::lattice_rebase::rebase_command_tokens(&mut original.tokens, i64::from(u32::MAX) + 1);
        assert!(original.tokens.source_bpf_command.is_none());
    }

    #[test]
    fn generated_bpf_operation_owns_its_typed_selection_and_data_without_original_words() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let operation = BpfOpKind::ScalarSet(BpfScalarWidth::I64);
        let args = vec!["with space\0$literal".into(), "3".into()];
        let mut tokens =
            CommandTokens::generated_bpf_operation(operation, &args, Span::new(8, 12), registry)
                .unwrap();
        let receipt = tokens.source_bpf_command.clone().unwrap();
        assert_eq!(
            receipt
                .descriptor(&tokens, &args, registry)
                .unwrap()
                .unwrap()
                .bpf_op
                .unwrap()
                .kind,
            operation
        );
        assert!(receipt.original_words(&tokens, &args, registry).is_none());
        let mut changed_args = args.clone();
        changed_args[1] = "4".into();
        assert!(
            receipt
                .descriptor(&tokens, &changed_args, registry)
                .is_none()
        );
        tokens.argv_texts[1] = "different".into();
        assert!(receipt.descriptor(&tokens, &args, registry).is_none());
    }
}
