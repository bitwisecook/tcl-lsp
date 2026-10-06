//! Original sole-child expression reads at a declaration-local SSA boundary.
//!
//! The receipt references an existing symbolic version only. It supplies no
//! represented read, physical address, successful evaluation or executable edit.

use super::{CommandAllocationSite, SourceInvocationBinding, SourceVariableEvaluationOwner};
use crate::ir::{CommandTokens, Provenance, WordExpr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclarationExpressionReadAdvice {
    parent: CommandAllocationSite,
    child: CommandAllocationSite,
    name: String,
    spelling: String,
    source: crate::ir::SourceSite,
}

impl DeclarationExpressionReadAdvice {
    pub(crate) fn spelling(&self) -> &str {
        &self.spelling
    }

    pub(crate) fn diagnostic_version(
        &self,
        ssa: &crate::ssa::SsaFunction,
        block: crate::cfg::BlockId,
        index: usize,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(crate::ssa::Symbol, crate::ssa::Version)> {
        let tokens = crate::ssa::SsaSourceView::at_statement(ssa, block, index).source_tokens()?;
        let binding = tokens.source_binding.as_ref()?;
        if binding.invocation_site()? != &self.parent
            || !binding
                .declaration_expression_reads(registry, tokens)?
                .contains(self)
        {
            return None;
        }
        super::declaration_layout::symbolic_version(ssa, block, index, &self.name)
    }
}

impl SourceInvocationBinding {
    /// A sole expression child preceded only by original literal words. Exact
    /// parent/child evaluation ownership and own-frame scope are mandatory;
    /// earlier command/index evaluation, formals, aliases and traces withdraw.
    pub(crate) fn declaration_expression_reads(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &CommandTokens,
    ) -> Option<Vec<DeclarationExpressionReadAdvice>> {
        let layout = self.declaration_operand_layout_advice(tokens)?;
        let parent = self.invocation_site()?;
        if !self
            .declaration_flow_report(registry)?
            .declared_argument_entry(parent)
        {
            return None;
        }
        let words = tokens.words();
        if !matches!(words.last()?, WordExpr::CommandSubstitution { .. }) {
            return None;
        }
        for word in &words[..words.len().checked_sub(1)?] {
            if !matches!(
                word,
                WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
            ) {
                return None;
            }
        }
        let config = tcl_lexer::LexerConfig::from_grammar(layout.dialect().lexer_grammar);
        let mut calls = crate::word_subst::checked_lifted_calls(tokens, config)?;
        if calls.len() != 1 {
            return None;
        }
        let child_tokens = calls.pop()?.tokens?;
        let child_binding = child_tokens.source_binding.as_ref()?;
        let child = child_binding.invocation_site()?;
        if parent.source != child.source || self.variable_frame != child_binding.variable_frame {
            return None;
        }
        let advice = crate::registry_invocation::original_expression_operand_advice(
            registry,
            &child_tokens,
        )?;
        if !scalar_expression(&advice.expression) {
            return None;
        }
        let mut reads = Vec::new();
        for node in advice.expression.variable_nodes() {
            let crate::expr_ast::ExprNode::Var { text, .. } = node else {
                return None;
            };
            let span = node.variable_source_span(advice.expression_base, config)?;
            let reference =
                tcl_lexer::word_parts::scan_var_ref(text.as_bytes(), 0, config).ok()??;
            if reference.next != text.len() || reference.index.is_some() {
                return None;
            }
            let name = std::str::from_utf8(reference.name).ok()?;
            if name.contains("::") {
                return None;
            }
            let accesses = child_tokens
                .variable_accesses
                .iter()
                .filter(|access| access.source.span == span && access.original_spelling == *text)
                .collect::<Vec<_>>();
            if accesses.is_empty() {
                return None;
            }
            for access in accesses {
                if access.source.provenance != Provenance::Source
                    || !direct_child_expression_owner(&access.owner, parent, child)
                    || access.context_alternatives().is_empty()
                    || parent
                        .source
                        .source_image()
                        .bytes()
                        .get(span.start() as usize..span.end() as usize)
                        != Some(text.as_bytes())
                    || !declared_local_expression_scope_matches(
                        self, parent, access, name, registry, config,
                    )
                {
                    return None;
                }
                let read = DeclarationExpressionReadAdvice {
                    parent: parent.clone(),
                    child: child.clone(),
                    name: name.to_owned(),
                    spelling: text.clone(),
                    source: access.source.clone(),
                };
                if !reads.contains(&read) {
                    reads.push(read);
                }
            }
        }
        Some(reads)
    }
}

/// Scope of the original declaration-local symbolic value. An unresolved
/// runtime writer or callback remains possible; this receipt is consumed only
/// by candidate diagnostics and cannot supply a successful physical read.
fn declared_local_expression_scope_matches(
    binding: &SourceInvocationBinding,
    parent: &CommandAllocationSite,
    access: &super::SourceVariableAccess,
    name: &str,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let Some(observations) = binding.declaration_layout_observations.as_deref() else {
        return false;
    };
    let Some(originals) = super::declaration_layout::original_declaration_layouts(observations)
    else {
        return false;
    };
    originals.clone().all(|observation| {
        let context = &observation.snapshot.state.source_variables;
        let mut reads = access
            .context_alternatives()
            .iter()
            .filter(|read| observation.entry.owns_original_context(read))
            .peekable();
        reads.peek().is_some()
            && reads.all(|read| {
                read.frame_kind == crate::var_resolve::VariableFrameKind::Local
                    && !super::declaration_layout::local_read_scope_is_excluded(read, name)
            })
            && observation
                .entry
                .owns_source(&parent.source, access.source.span.start())
            && !observation
                .entry
                .parameters()
                .iter()
                .any(|formal| formal.name == name)
            && context.frame_kind == crate::var_resolve::VariableFrameKind::Local
            && !super::declaration_layout::local_read_scope_is_excluded(context, name)
            && !crate::script_binds::script_image_binds_name(
                &observation.entry.source().text,
                name,
                crate::script_binds::Ownership::ScopeAliases,
                registry,
                config,
            )
    })
}

fn direct_child_expression_owner(
    owner: &SourceVariableEvaluationOwner,
    parent: &CommandAllocationSite,
    child: &CommandAllocationSite,
) -> bool {
    let mut pending = vec![owner];
    while let Some(owner) = pending.pop() {
        match owner {
            SourceVariableEvaluationOwner::NativeExpression {
                invocation,
                parent: Some(enclosing),
            } if invocation == child => {
                let enclosing = match enclosing.as_ref() {
                    SourceVariableEvaluationOwner::InvocationBody {
                        invocation,
                        parent: Some(outer),
                    } if invocation == child => outer.as_ref(),
                    enclosing => enclosing,
                };
                if !matches!(enclosing, SourceVariableEvaluationOwner::InvocationArguments {
                    invocation, ..
                } if invocation == parent)
                {
                    return false;
                }
            }
            SourceVariableEvaluationOwner::Alternatives(owners) if !owners.is_empty() => {
                pending.extend(owners);
            }
            _ => return false,
        }
    }
    true
}

fn scalar_expression(expression: &crate::expr_ast::ExprNode) -> bool {
    use crate::expr_ast::ExprNode;
    let mut pending = vec![expression];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { .. } | ExprNode::Var { .. } => {}
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => pending.extend([
                condition.as_ref(),
                true_branch.as_ref(),
                false_branch.as_ref(),
            ]),
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    fn reads(source: &str, statement: &str) -> Option<Vec<super::DeclarationExpressionReadAdvice>> {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
        let base = u32::try_from(source.find(statement).unwrap()).unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(statement, base, config)
                .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceImage::document(source).source_map(),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        tokens
            .source_binding
            .as_ref()?
            .declaration_expression_reads(registry, &tokens)
    }

    #[test]
    fn sole_expression_child_retains_original_local_read_and_withdrawals() {
        let statement = "set y [expr {$x + 0}]";
        let source = format!("proc p {{}} {{set x 0; {statement}}}");
        let receipt = reads(&source, statement).expect("original declaration-local expression");
        assert_eq!(receipt.len(), 1);
        assert_eq!(receipt[0].spelling(), "$x");
        for source in [
            format!("proc p {{x}} {{{statement}}}"),
            format!("proc p {{}} {{global x; {statement}}}"),
            format!("proc p {{}} {{upvar 1 caller x; {statement}}}"),
            format!("proc p {{}} {{return DONE; {statement}}}"),
        ] {
            assert!(reads(&source, statement).is_none(), "{source}");
        }
        for statement in [
            "set [incr x] [expr {$x + 0}]",
            "set y prefix[expr {$x + 0}]",
            "set y [expr {[incr x] + $x}]",
            "set y [expr {$a($x) + 0}]",
        ] {
            let source = format!("proc p {{}} {{set x 0; {statement}}}");
            assert!(reads(&source, statement).is_none(), "{source}");
        }
    }

    #[test]
    fn conditional_loop_expression_advice_keeps_original_child_read() {
        let statement = "set y [expr {$i + 0}]";
        let source = format!(
            "proc f {{n}} {{for {{set i 0}} {{$i < $n}} {{incr i}} {{{statement}; puts $y}}}}; f 3"
        );
        let receipt = reads(&source, statement).expect("conditional original loop declaration");
        assert_eq!(receipt.len(), 1);
        assert_eq!(receipt[0].spelling(), "$i");
        for prefix in ["global i;", "upvar 1 caller i;"] {
            let source = format!(
                "proc f {{n}} {{{prefix} for {{set i 0}} {{$i < $n}} {{incr i}} {{{statement}; puts $y}}}}; f 3"
            );
            assert!(reads(&source, statement).is_none(), "{source}");
        }
    }
}
