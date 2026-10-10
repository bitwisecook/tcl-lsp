// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional transfer queries over one unchanged original Logical source world.

use crate::ir::{CommandTokens, Module, Procedure, Script, Statement};
use crate::registry_invocation::{
    EffectiveCommandWords, InvocationMetadataContext, ResolvedStatementInvocation,
};
use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;

/// A selected source call and its same-owner effective operands. Neither the
/// declaration nor this conditional join establishes Native Normal completion.
pub(crate) struct ProcedureCall<'a> {
    pub(crate) procedure: &'a Procedure,
    pub(crate) arguments: EffectiveCommandWords,
}

impl ProcedureCall<'_> {
    pub(crate) fn literal_arguments(&self, module: &Module) -> Option<Vec<Option<String>>> {
        if self.arguments.words.iter().any(|word| {
            matches!(
                word,
                crate::ir::WordExpr::Expand { .. } | crate::ir::WordExpr::Opaque { .. }
            )
        }) {
            return None;
        }
        Some(
            (0..self.arguments.words.len().checked_sub(1)?)
                .map(|index| {
                    self.arguments.argument_literal(
                        index,
                        module.native_lexer_config().escapes,
                        module.word_values(),
                    )
                })
                .collect(),
        )
    }
}

/// Original child words retained by the same Module and parent invocation.
/// This source inventory grants no Native entry or completion authority.
#[derive(Clone)]
pub(crate) struct OriginalSummaryScript {
    text: String,
    base: u32,
    commands: Vec<CommandTokens>,
}

impl OriginalSummaryScript {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
    pub(crate) const fn base(&self) -> u32 {
        self.base
    }
    pub(crate) fn command_at(&self, offset: u32) -> Option<&CommandTokens> {
        let mut found = self.commands.iter().filter(|tokens| {
            tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.invocation_site())
                .is_some_and(|site| site.offset == offset)
        });
        let first = found.next()?;
        found.all(|next| next == first).then_some(first)
    }
}

/// A complete selected original expression and its existing parent point.
/// It grants only child source correspondence, never Native completion.
#[derive(Clone)]
pub(crate) struct OriginalSummaryExpression {
    parent: CommandTokens,
    base: u32,
}

pub(super) struct SourceSummaryContext<'a> {
    module: &'a Module,
    registry: &'a CommandRegistry,
    config: LexerConfig,
}

impl<'a> SourceSummaryContext<'a> {
    pub(super) fn for_module(
        module: &'a Module,
        registry: &'a CommandRegistry,
        config: LexerConfig,
    ) -> Option<Self> {
        if config.normalized() != module.lexer_config.normalized()
            || module.has_dynamic_trace
            || !module.traced_commands.is_empty()
            || !module
                .retained_source_bindings
                .as_ref()?
                .matches_module(module, registry)
            || !InvocationMetadataContext::for_module(registry, module)?
                .permits_logical_source_names()
        {
            return None;
        }
        Some(Self {
            module,
            registry,
            config,
        })
    }

    pub(super) const fn registry(&self) -> &'a CommandRegistry {
        self.registry
    }

    pub(super) fn procedure(&self, qname: &str) -> Option<&'a Procedure> {
        let procedure = self.module.procedures.get(qname)?;
        (qname == procedure.qualified_name
            && self
                .module
                .retained_source_bindings
                .as_ref()?
                .matches_original_procedure(procedure))
        .then_some(procedure)
    }

    pub(super) fn metadata<'w>(
        &'w self,
        tokens: &'w CommandTokens,
    ) -> Option<InvocationMetadataContext<'w>> {
        let metadata = tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_module(tokens, self.module, self.registry)?;
        metadata.permits_logical_source_names().then_some(metadata)
    }

    pub(super) fn operation(&self, tokens: &CommandTokens) -> Option<ResolvedStatementInvocation> {
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            self.registry,
            self.metadata(tokens)?,
            tokens,
        )
    }

    pub(super) fn with_operation<T>(
        &self,
        tokens: &CommandTokens,
        apply: impl FnOnce(
            &ResolvedStatementInvocation,
            &tcl_registry::ResolvedInvocation<'_, '_>,
        ) -> Option<T>,
    ) -> Option<T> {
        let operation = self.operation(tokens)?;
        let realm = tokens.source_binding.as_ref()?.invocation_realm()?;
        operation.with_metadata_schema(self.registry, self.metadata(tokens)?, realm, |schema| {
            apply(&operation, schema)
        })
    }

    pub(super) fn procedure_call(&self, tokens: &CommandTokens) -> Option<ProcedureCall<'a>> {
        self.metadata(tokens)?;
        let calls = crate::registry_invocation::original_logical_procedure_calls_for_module(
            tokens,
            self.module,
            self.registry,
        )?;
        let mut selected: Option<ProcedureCall<'a>> = None;
        for call in calls {
            let procedure = self.procedure(&call.procedure().qualified_name)?;
            let next = ProcedureCall {
                procedure,
                arguments: call.effective_arguments().clone(),
            };
            if let Some(previous) = &selected {
                if !std::ptr::eq(previous.procedure, next.procedure)
                    || previous.arguments != next.arguments
                {
                    return None;
                }
            } else {
                selected = Some(next);
            }
        }
        selected
    }

    pub(super) fn expression(
        &self,
        parent: &CommandTokens,
        expression: &crate::expr_ast::ExprNode,
        base: Option<u32>,
    ) -> Option<OriginalSummaryExpression> {
        self.metadata(parent)?;
        let mut pending = vec![parent.clone()];
        let mut selected: Option<OriginalSummaryExpression> = None;
        while let Some(tokens) = pending.pop() {
            self.metadata(&tokens)?;
            for written in 1..tokens.words().len() {
                let Some(advice) =
                    crate::registry_invocation::original_expression_operand_advice_for_word(
                        self.registry,
                        &tokens,
                        written,
                    )
                else {
                    continue;
                };
                if advice.expression != *expression
                    || base.is_some_and(|base| base != advice.expression_base)
                {
                    continue;
                }
                let next = OriginalSummaryExpression {
                    parent: tokens.clone(),
                    base: advice.expression_base,
                };
                if selected.as_ref().is_some_and(|previous| {
                    previous.base != next.base || previous.parent != next.parent
                }) {
                    return None;
                }
                selected = Some(next);
            }
            // Only a supplied original producer base permits descent through
            // that parent's retained child inventory. Text/tree equality alone
            // never locates a producer for an assembled expression.
            if base.is_some() {
                pending.extend(self.children(&tokens)?);
            }
        }
        selected
    }

    pub(super) fn expression_substitution(
        &self,
        expression: &OriginalSummaryExpression,
        script: &str,
        start: u32,
        end: u32,
    ) -> Option<OriginalSummaryScript> {
        let site = crate::ir::SourceSite::source(tcl_lexer::Span::new(
            expression.base.checked_add(start)?,
            expression.base.checked_add(end)?.checked_add(1)?,
        ));
        self.substitution(&expression.parent, &site, script)
    }

    pub(super) fn substitution(
        &self,
        parent: &CommandTokens,
        site: &crate::ir::SourceSite,
        script: &str,
    ) -> Option<OriginalSummaryScript> {
        self.metadata(parent)?;
        if site.provenance != crate::ir::Provenance::Source {
            return None;
        }
        let original = &parent.source_binding.as_ref()?.invocation_site()?.source;
        let spelling = original
            .source_image()
            .try_text()
            .ok()?
            .get(site.span.as_range())?;
        if spelling.strip_prefix('[')?.strip_suffix(']')? != script {
            return None;
        }
        let calls = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            parent,
            self.config.nested(),
            self.registry,
            self.metadata(parent)?,
        )?;
        let mut selected = calls.into_iter().filter(|call| call.span == site.span);
        let first = selected.next()?;
        if selected.next().is_some() {
            return None;
        }
        let child = first.tokens?;
        self.metadata(&child)?;
        Some(OriginalSummaryScript {
            text: script.to_owned(),
            base: site.span.start().checked_add(1)?,
            commands: vec![child],
        })
    }

    pub(super) fn body(
        &self,
        parent: &CommandTokens,
        argument: usize,
        script: &str,
    ) -> Option<OriginalSummaryScript> {
        let operation = self.operation(parent)?;
        if !operation
            .written_argument_roles()
            .contains(&(argument, tcl_registry::ArgRole::Body))
        {
            return None;
        }
        let argument = (0..operation.effective.words.len().checked_sub(1)?)
            .find(|&index| operation.effective.written_argument(index) == Some(argument))?;
        let word = operation.effective.words.get(argument.checked_add(1)?)?;
        if !matches!(word, crate::ir::WordExpr::BracedLiteral { .. }) {
            return None;
        }
        let source = &parent.source_binding.as_ref()?.invocation_site()?.source;
        let base = crate::command_binding::ExecutedScriptSource::literal_word_base(
            source.source_image().try_text().ok()?,
            word,
            script,
            self.config.nested(),
        )?;
        let end = base.checked_add(u32::try_from(script.len()).ok()?)?;
        let mut selected: Option<Vec<CommandTokens>> = None;
        for procedure in self.module.procedures.values() {
            self.procedure(&procedure.qualified_name)?;
            let mut pending = vec![&procedure.body];
            while let Some(body) = pending.pop() {
                for statement in &body.statements {
                    let children = crate::ir_helpers::nested_bodies(statement);
                    if body.retained_source_tokens_for_statement(statement) == Some(parent) {
                        for child in &children {
                            if child.statements.is_empty() {
                                continue;
                            }
                            let mut commands = Vec::new();
                            let mut in_extent = true;
                            for statement in &child.statements {
                                if statement.synthetic_marker().is_some() {
                                    continue;
                                }
                                let tokens =
                                    child.retained_source_tokens_for_statement(statement)?;
                                self.metadata(tokens)?;
                                let site = tokens.source_binding.as_ref()?.invocation_site()?;
                                if &site.source != source
                                    || site.offset < base
                                    || site.offset >= end
                                {
                                    in_extent = false;
                                    break;
                                }
                                commands.push(tokens.clone());
                            }
                            if in_extent && !commands.is_empty() {
                                if selected
                                    .as_ref()
                                    .is_some_and(|previous| previous != &commands)
                                {
                                    return None;
                                }
                                selected = Some(commands);
                            }
                        }
                    }
                    pending.extend(children);
                }
            }
        }
        Some(OriginalSummaryScript {
            text: script.to_owned(),
            base,
            commands: selected?,
        })
    }

    pub(super) fn children(&self, tokens: &CommandTokens) -> Option<Vec<CommandTokens>> {
        crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            tokens,
            self.config.nested(),
            self.registry,
            self.metadata(tokens)?,
        )?
        .into_iter()
        .map(|call| {
            let child = call.tokens?;
            self.metadata(&child)?;
            Some(child)
        })
        .collect()
    }

    /// Obtain existing original words from the retained IR owner. A lowered CFG
    /// statement cannot manufacture that owner from its reporting head or span.
    pub(super) fn statement_tokens(
        &self,
        qname: &str,
        statement: &Statement,
    ) -> Option<CommandTokens> {
        let mut pending = Vec::new();
        if let Some(procedure) = self.procedure(qname) {
            pending.push(&procedure.body);
        }
        // This is the compilation unit's explicit top-level entry identity.
        // A procedure also called ::top still needs its own genuine header;
        // original statement extents and carriers distinguish the two roots.
        if qname == "::top" {
            pending.push(&self.module.top_level);
        }
        if pending.is_empty() {
            return None;
        }
        let mut found = None;
        while let Some(script) = pending.pop() {
            for original in &script.statements {
                if original.span() == statement.span()
                    && let Some(tokens) = script.retained_source_tokens_for_statement(original)
                {
                    self.metadata(tokens)?;
                    if tokens.source_binding.as_ref()?.invocation_site()?.offset
                        != statement.span().start()
                        || statement.tokens().is_some_and(|current| current != tokens)
                    {
                        return None;
                    }
                    if found.as_ref().is_some_and(|previous| previous != tokens) {
                        return None;
                    }
                    found = Some(tokens.clone());
                }
                pending.extend(crate::ir_helpers::nested_bodies(original));
            }
        }
        found
    }

    pub(super) fn body_commands(&self, qname: &str) -> Option<Vec<CommandTokens>> {
        let procedure = self.procedure(qname)?;
        let mut commands = Vec::new();
        let mut pending: Vec<&Script> = vec![&procedure.body];
        while let Some(script) = pending.pop() {
            for statement in &script.statements {
                if statement.synthetic_marker().is_some() {
                    continue;
                }
                let tokens = script.retained_source_tokens_for_statement(statement)?;
                self.metadata(tokens)?;
                if tokens.source_binding.as_ref()?.invocation_site()?.offset
                    != statement.span().start()
                {
                    return None;
                }
                commands.push(tokens.clone());
                commands.extend(self.children(tokens)?);
                pending.extend(crate::ir_helpers::nested_bodies(statement));
            }
        }
        Some(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_expression_fused_top_level_retains_its_existing_parent_record() {
        // naming.expression.original-positioned-analysis-services
        // docs/design/analysis/name-resolution-proofs/expression-original-positioned-analysis-services.md
        // Retained top-level IR ownership, independently of any execution frame.
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 7}\nset answer [expr {[leaf] + 1}]",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .unwrap();
        let (statement, expression, base) = module
            .top_level
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::AssignExpr {
                    expr,
                    expr_base: Some(base),
                    ..
                } => Some((statement, expr, *base)),
                _ => None,
            })
            .expect("genuine source-fused expression");
        let parent = source
            .statement_tokens("::top", statement)
            .expect("retained outer set words");
        let receipt = source
            .expression(&parent, expression, Some(base))
            .expect("genuine retained nested expr producer");
        assert!(
            source.expression(&parent, expression, None).is_none(),
            "tree equality cannot recover an unspecified child producer"
        );
        let child = source
            .expression_substitution(&receipt, "leaf", 0, 5)
            .unwrap();
        assert_eq!(child.base(), base + 1);
        assert!(child.command_at(child.base()).is_some());
    }

    #[test]
    fn original_expression_receipts_join_actual_literal_operand_and_child_extent() {
        // naming.expression.original-positioned-analysis-services
        // docs/design/analysis/name-resolution-proofs/expression-original-positioned-analysis-services.md
        // Software source ancestry and byte extents, not Native Normal completion.
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 7}\ninterp alias {} e {} expr\nproc p {} {e {[leaf] + 1}; expr {\"é:[leaf]\" eq \"é:7\"}}",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .unwrap();
        let expressions: Vec<_> = source
            .body_commands("::p")
            .unwrap()
            .into_iter()
            .filter(|tokens| {
                source
                    .operation(tokens)
                    .is_some_and(|operation| operation.facts.canonical_command == "expr")
            })
            .collect();
        assert_eq!(expressions.len(), 2);
        for (index, parent) in expressions.iter().enumerate() {
            let advice = crate::registry_invocation::original_expression_operand_advice_for_word(
                context.commands(),
                parent,
                1,
            )
            .expect("selected genuine literal expression operand");
            let receipt = source
                .expression(parent, &advice.expression, Some(advice.expression_base))
                .expect("actual parent and complete original expression");
            let relative = advice.expression_text.find("[leaf]").unwrap();
            let start = u32::try_from(relative).unwrap();
            let end = start + 5;
            let child = source
                .expression_substitution(&receipt, "leaf", start, end)
                .expect("same original child and inclusive parser extent");
            assert_eq!(child.base(), advice.expression_base + start + 1);
            assert!(child.command_at(child.base()).is_some());
            assert!(
                source
                    .expression_substitution(&receipt, "leaf", start, end + 1)
                    .is_none()
            );
            assert!(
                source
                    .expression_substitution(&receipt, " leaf", start, end)
                    .is_none()
            );
            assert!(
                source
                    .expression(parent, &advice.expression, Some(advice.expression_base + 1))
                    .is_none()
            );
            assert!(
                source
                    .expression(
                        parent,
                        &crate::expr_parser::parse_expr("0", None),
                        Some(advice.expression_base)
                    )
                    .is_none()
            );
            assert!(
                source
                    .expression(parent, &advice.expression, None)
                    .is_some(),
                "literal assembled operand {index}"
            );
        }
    }

    #[test]
    fn original_expression_receipts_refuse_cooked_and_foreign_producers() {
        // naming.expression.original-positioned-analysis-services
        // docs/design/analysis/name-resolution-proofs/expression-original-positioned-analysis-services.md
        // Whole source receipt refusal; no command-table or execution inference.
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 7}\nproc p {} {expr {[leaf]} + 1}",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .unwrap();
        let parent = source
            .body_commands("::p")
            .unwrap()
            .into_iter()
            .find(|tokens| {
                source
                    .operation(tokens)
                    .is_some_and(|operation| operation.facts.canonical_command == "expr")
            })
            .unwrap();
        let assembled = crate::expr_parser::parse_expr("[leaf] + 1", None);
        assert!(source.expression(&parent, &assembled, None).is_none());
        let (foreign_context, foreign) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 8}\nproc p {} {expr {[leaf] + 1}}",
            "tcl8.6",
        );
        let foreign_source = SourceSummaryContext::for_module(
            &foreign.ir_module,
            foreign_context.commands(),
            foreign.ir_module.lexer_config,
        )
        .unwrap();
        let foreign_parent = foreign_source
            .body_commands("::p")
            .unwrap()
            .into_iter()
            .find(|tokens| {
                foreign_source
                    .operation(tokens)
                    .is_some_and(|operation| operation.facts.canonical_command == "expr")
            })
            .unwrap();
        let foreign_advice =
            crate::registry_invocation::original_expression_operand_advice_for_word(
                foreign_context.commands(),
                &foreign_parent,
                1,
            )
            .unwrap();
        assert!(
            source
                .expression(
                    &foreign_parent,
                    &foreign_advice.expression,
                    Some(foreign_advice.expression_base)
                )
                .is_none(),
            "independent whole source image cannot borrow this Module owner"
        );
    }

    #[test]
    fn original_expression_receipts_refuse_captured_and_shadowed_producers() {
        // naming.expression.original-positioned-analysis-services
        // docs/design/analysis/name-resolution-proofs/expression-original-positioned-analysis-services.md
        // No source producer can be invented from captured argv or a reported head.
        let assembled = crate::expr_parser::parse_expr("[leaf] + 1", None);
        let (captured_context, captured_unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 7}\ninterp alias {} e {} expr {[leaf] + 1}\nproc p {} {e}",
            "tcl8.6",
        );
        let captured = SourceSummaryContext::for_module(
            &captured_unit.ir_module,
            captured_context.commands(),
            captured_unit.ir_module.lexer_config,
        )
        .unwrap();
        let captured_parent = captured
            .body_commands("::p")
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert!(
            captured
                .expression(&captured_parent, &assembled, None)
                .is_none(),
            "captured argv without a retained original expression producer cannot donate a written operand"
        );
        let (shadow_context, shadow_unit) = crate::interprocedural::logical_completion_unit(
            "proc expr {args} {return 0}\nproc p {} {expr {[leaf] + 1}}",
            "tcl8.6",
        );
        let shadow = SourceSummaryContext::for_module(
            &shadow_unit.ir_module,
            shadow_context.commands(),
            shadow_unit.ir_module.lexer_config,
        )
        .unwrap();
        let parent = shadow
            .body_commands("::p")
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert!(shadow.expression(&parent, &assembled, None).is_none());
    }

    #[test]
    fn original_transfer_call_keeps_selected_procedure_and_captured_operands_together() {
        // naming.interprocedural.original-transfer-summary-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
        // Conditional source/argv correspondence, not Native activation or effects.
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {name {by 1} args} {return $name}\ninterp alias {} frozen {} leaf chosen\nproc caller {} {frozen 2 extra}",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .expect("genuine complete Logical source owner");
        let commands = source.body_commands("::caller").unwrap();
        let [original] = commands.as_slice() else {
            panic!("one original call")
        };
        let call = source
            .procedure_call(original)
            .expect("selected original procedure");
        assert_eq!(call.procedure.qualified_name, "::leaf");
        assert_eq!(
            call.literal_arguments(module).unwrap(),
            [
                Some("chosen".into()),
                Some("2".into()),
                Some("extra".into())
            ]
        );
        assert_eq!(call.arguments.written_argument(0), None);
        assert_eq!(call.arguments.written_argument(1), Some(0));
        let mut altered = original.clone();
        let crate::ir::WordExpr::Literal { text, .. } = &mut altered.word_exprs[0] else {
            panic!("authentic literal alias head")
        };
        *text = "leaf".into();
        assert!(source.procedure_call(&altered).is_none());
    }

    #[test]
    fn original_transfer_child_and_body_require_the_genuine_parent_word() {
        // naming.interprocedural.original-transfer-summary-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
        // Existing original child carriers, independently of entered Body authority.
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 9}\ninterp alias {} each {} dict for\nproc caller {} {catch {leaf}; each {k v} {} {leaf}; return [leaf]}",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .unwrap();
        let commands = source.body_commands("::caller").unwrap();
        let catch = commands
            .iter()
            .find(|tokens| {
                source
                    .operation(tokens)
                    .is_some_and(|operation| operation.facts.canonical_command == "catch")
            })
            .unwrap();
        let body = source
            .body(catch, 0, "leaf")
            .expect("existing selected original Body child");
        assert!(body.base() > 0);
        assert!(body.command_at(body.base()).is_some());
        assert!(source.body(catch, 1, "leaf").is_none());
        assert!(source.body(catch, 0, " leaf").is_none());
        let compound = commands
            .iter()
            .find(|tokens| {
                source.operation(tokens).is_some_and(|operation| {
                    operation.facts.canonical_command == "dict"
                        && operation
                            .written_argument_roles()
                            .contains(&(2, tcl_registry::ArgRole::Body))
                })
            })
            .expect("selected compound operation through a captured subcommand");
        let compound_body = source
            .body(compound, 2, "leaf")
            .expect("same original written body after selected subcommand offset");
        assert!(compound_body.command_at(compound_body.base()).is_some());
        assert!(source.body(compound, 1, "leaf").is_none());
        let parent = commands
            .iter()
            .find(|tokens| {
                tokens
                    .words()
                    .last()
                    .is_some_and(|word| word.sole_command_substitution().is_some())
            })
            .unwrap();
        let (_, site) = parent
            .words()
            .last()
            .unwrap()
            .sole_command_substitution()
            .unwrap();
        let child = source
            .substitution(parent, site, "leaf")
            .expect("existing selected original substitution");
        assert!(child.command_at(child.base()).is_some());
        assert!(source.substitution(parent, site, " leaf").is_none());
        let mut detached = site.clone();
        detached.provenance = crate::ir::Provenance::Opaque;
        assert!(source.substitution(parent, &detached, "leaf").is_none());
    }

    #[test]
    fn original_transfer_source_refuses_missing_foreign_config_and_changed_headers() {
        // naming.interprocedural.original-transfer-summary-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
        // Software source-world correspondence; no Native dispatch permission.
        let (context, unit) =
            crate::interprocedural::logical_completion_unit("proc p {x} {return $x}", "tcl8.6");
        let module = &unit.ir_module;
        assert!(
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .is_some()
        );
        let mut config = module.lexer_config;
        config.expand_syntax = !config.expand_syntax;
        assert!(SourceSummaryContext::for_module(module, context.commands(), config).is_none());
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        assert!(
            SourceSummaryContext::for_module(module, foreign.commands(), module.lexer_config)
                .is_none()
        );
        let mut missing = module.clone();
        missing.source_metadata_input = None;
        assert!(
            SourceSummaryContext::for_module(&missing, context.commands(), missing.lexer_config)
                .is_none()
        );
        let mut changed = module.clone();
        changed
            .procedures
            .get_mut("::p")
            .unwrap()
            .params_raw
            .push_str(" injected");
        let source =
            SourceSummaryContext::for_module(&changed, context.commands(), changed.lexer_config)
                .unwrap();
        assert!(source.procedure("::p").is_none());
        let mut moved = module.clone();
        let procedure = moved.procedures.remove("::p").unwrap();
        moved.procedures.insert("::other".into(), procedure);
        let source =
            SourceSummaryContext::for_module(&moved, context.commands(), moved.lexer_config)
                .unwrap();
        assert!(source.procedure("::other").is_none());
    }

    #[test]
    fn original_transfer_shadow_and_command_observers_do_not_borrow_catalogue_closure() {
        // naming.interprocedural.original-transfer-summary-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc set {name value} {return changed}\nproc caller {} {set x 1}",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .unwrap();
        let commands = source.body_commands("::caller").unwrap();
        let [original] = commands.as_slice() else {
            panic!("one original replacement call")
        };
        assert_eq!(
            source
                .procedure_call(original)
                .unwrap()
                .procedure
                .qualified_name,
            "::set"
        );
        assert!(source.operation(original).is_none());
        let mut observed = module.clone();
        observed.traced_commands.insert("::set".into());
        assert!(
            SourceSummaryContext::for_module(&observed, context.commands(), observed.lexer_config)
                .is_none()
        );
        let native_context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let native = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "proc p {} {return 1}",
            native_context.commands(),
            false,
            "tcl8.6",
        );
        assert!(
            SourceSummaryContext::for_module(
                &native.ir_module,
                native_context.commands(),
                native.ir_module.lexer_config
            )
            .is_none()
        );
    }
}
