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

//! Physical direct-read component of one original prepared expression.
//! This is not a complete condition effect footprint or motion licence.

use super::ExpressionMathBindings;
use crate::command_binding::{
    SourceVariableAccess, SourceVariableEvaluationOwner, SourceVariableReadResidual,
};
use crate::place::PlaceKind;

/// Original physical scalar reads, retaining their preparation and read issuers.
/// Commands, arrays, strings and implicit calls remain unsupported here. Even a
/// complete direct-read list does not close object conversions, other effects,
/// normal completion, result allocation, or executable expression equivalence.
pub(crate) struct OriginalPhysicalConditionReads<'a> {
    _preparation: &'a crate::command_binding::SourceExpressionPreparation,
    _accesses: Vec<&'a SourceVariableAccess>,
    cells: Vec<crate::var_resolve::VariableCellKey>,
}

impl OriginalPhysicalConditionReads<'_> {
    /// Actual possible direct scalar cell reads, for positive overlap vetoes.
    /// An empty list never licenses motion or asserts no other effects.
    pub(crate) fn cells(&self) -> &[crate::var_resolve::VariableCellKey] {
        &self.cells
    }
}

impl<'a> ExpressionMathBindings<'a> {
    /// Reborrow the direct physical read component from the actual Native
    /// preparation and per-reference contexts. Source-only frames cannot issue
    /// it. Unsupported or unobserved correspondence remains unavailable.
    pub(crate) fn physical_condition_reads(
        &self,
        module: &crate::ir::Module,
        registry: &tcl_registry::CommandRegistry,
        expected: &crate::expr_ast::ExprNode,
    ) -> Option<OriginalPhysicalConditionReads<'a>> {
        let tokens = self.tokens?;
        let binding = tokens.source_binding.as_ref()?;
        let metadata = binding.original_invocation_metadata_for_module(tokens, module, registry)?;
        (binding.original_invocation_source_purpose(tokens, metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Execution)
            .then_some(())?;
        let preparation = self.preparation()?;
        if preparation.witness.tree() != expected
            || !preparation.has_closed_script_compilation()
            || !self.unobserved_preparation_invocation(preparation)
        {
            return None;
        }
        let (origin, base) = preparation.original_expression_mapping()?;
        if !std::sync::Arc::ptr_eq(origin, &preparation.invocation.source)
            || self.base != Some(base)
        {
            return None;
        }
        let config = binding.original_argument_lexer_config(tokens)?;
        let source = tcl_syntax::expr::substitution::OriginalSourceExpression::capture(
            preparation.source.try_text().ok()?,
            preparation.witness.context(),
            config,
        )?;
        if !source.matches_tree(expected)
            || !source.command_regions()?.is_empty()
            || !source.function_calls().is_empty()
        {
            return None;
        }
        let mut cells = Vec::new();
        let mut retained = Vec::new();
        for node in expected.evaluation_nodes() {
            match node {
                crate::expr_ast::ExprNode::Var { text, .. } => {
                    let (_, site) =
                        preparation.original_variable_source_with_config(node, config)?;
                    let mut matching = self.reads.iter().filter(|access| access.source == site && access.original_spelling == *text
                        && matches!(&access.owner, SourceVariableEvaluationOwner::NativeExpression { invocation, .. }
                            if invocation == &preparation.invocation));
                    let access = matching.next()?;
                    if matching.next().is_some()
                        || access.context_residual() != SourceVariableReadResidual::Closed
                        || access.context_alternatives().is_empty()
                    {
                        return None;
                    }
                    for cell in physical_read_cells(access, registry, config)? {
                        if !cells.contains(&cell) {
                            cells.push(cell);
                        }
                    }
                    retained.push(access);
                }
                crate::expr_ast::ExprNode::Command { .. }
                | crate::expr_ast::ExprNode::Call { .. }
                | crate::expr_ast::ExprNode::String { .. }
                | crate::expr_ast::ExprNode::CompiledWord { .. }
                | crate::expr_ast::ExprNode::Raw { .. } => return None,
                _ => {}
            }
        }
        Some(OriginalPhysicalConditionReads {
            _preparation: preparation,
            _accesses: retained,
            cells,
        })
    }
}

fn physical_read_cells(
    access: &SourceVariableAccess,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<crate::var_resolve::VariableCellKey>> {
    access
        .context_alternatives()
        .iter()
        .map(|context| physical_read_cell_in_context(access, context, registry, config))
        .collect()
}

fn physical_read_cell_in_context(
    access: &SourceVariableAccess,
    context: &crate::var_resolve::ResolveContext,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Option<crate::var_resolve::VariableCellKey> {
    // A compatibility profile that happens to match is not retained input.
    let crate::var_resolve::SourceReadConfiguration::Selected(actual) =
        context.source_read_configuration
    else {
        return None;
    };
    if actual.normalized() != config.normalized()
        || context.execution_name_policy.is_none()
        || context.dynamic_bindings
        || context.dynamic_traces
        || context.binding_identity != crate::var_resolve::BindingIdentity::Bound
        || context.frame_kind == crate::var_resolve::VariableFrameKind::Unknown
    {
        return None;
    }
    let place = access.place_in_context(context, registry);
    if place.kind != PlaceKind::Scalar
        || place.observed
        || place.dynamic
        || place.cell.is_none()
        || context.unenumerated_observers_may_run(&place)
        || !place.name_reads.is_empty()
    {
        return None;
    }
    crate::var_resolve::canonical_place_key(&place)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_native_condition_reads_keep_cells_and_unknown_phases_separate() {
        for (source, expected) in [
            ("expr {1 + 2}", Some(0)),
            ("set value 1; expr {$value > 0}", Some(1)),
            ("set value 1; expr {[incr value] > 0}", None),
            ("set value 1; expr {\"$value\" eq 1}", None),
        ] {
            let fixture = crate::provider_fixtures::original_native_module(source, "tcl8.6");
            let (context, module) = &*fixture;
            let statement = module.top_level.statements.last().unwrap();
            let crate::ir::Statement::ExprEval { expr, .. } = statement else {
                panic!("actual expression statement")
            };
            let bindings =
                ExpressionMathBindings::for_module_statement(module, statement.span()).unwrap();
            assert_eq!(
                bindings
                    .physical_condition_reads(module, context.commands(), expr)
                    .map(|reads| reads.cells().len()),
                expected,
                "{source}"
            );
            let mut foreign = module.clone();
            foreign.lexer_config.strict_quoting = !foreign.lexer_config.strict_quoting;
            assert!(
                bindings
                    .physical_condition_reads(&foreign, context.commands(), expr)
                    .is_none()
            );
        }
    }
    #[test]
    fn physical_read_requires_selected_input_not_equal_profile_fallback() {
        let source = "set value 1; expr {$value > 0}";
        let fixture = crate::provider_fixtures::original_native_module(source, "tcl8.6");
        let (context, module) = &*fixture;
        let statement = module.top_level.statements.last().unwrap();
        let crate::ir::Statement::ExprEval { expr, .. } = statement else {
            panic!("actual expression")
        };
        let bindings =
            ExpressionMathBindings::for_module_statement(module, statement.span()).unwrap();
        let reads = bindings
            .physical_condition_reads(module, context.commands(), expr)
            .unwrap();
        let access = reads._accesses[0];
        let actual = &access.context_alternatives()[0];
        let config = module.lexer_config;
        assert!(
            physical_read_cell_in_context(access, actual, context.commands(), config).is_some()
        );
        let mut fallback = actual.as_ref().clone();
        fallback.source_read_configuration =
            crate::var_resolve::SourceReadConfiguration::Standalone;
        assert!(
            physical_read_cell_in_context(access, &fallback, context.commands(), config).is_none()
        );
        fallback.source_read_configuration =
            crate::var_resolve::SourceReadConfiguration::Unavailable;
        assert!(
            physical_read_cell_in_context(access, &fallback, context.commands(), config).is_none()
        );
    }
}
