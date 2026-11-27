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

//! One variable read inside an expression: its representation, place, incoming slot, contents and native numeric form.

use super::super::SsaReadReference;
use super::SsaSourceView;
use super::word_reads::{
    SsaReadContents, SsaReadRepresentationAdvice, source_read_place, source_read_representations,
};
use tcl_registry::CommandRegistry;
use tcl_source::words::SourceSite;

/// Ordinary incoming slot reached in one or more actual activations.
/// This is a logical input projection, not a scalar SSA value or a physical
/// alias between its cells. Callers must validate the formal binding protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsaIncomingSlotRead {
    /// Exact authored read whose reached activations supplied these cells.
    pub source: SourceSite,
    /// Literal ordinary slot selected in every retained activation.
    pub slot: String,
    /// Distinct physical incoming cells; none is collapsed into another owner.
    pub cells: Vec<tcl_names::place::CellIdentity>,
}

impl<'a> SsaSourceView<'a> {
    pub(in super::super) fn expression_variable_access(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
    ) -> Option<&'a crate::command_binding::SourceVariableAccess> {
        let crate::expr_ast::ExprNode::Var { text, .. } = node else {
            return None;
        };
        let tokens = self.source_tokens()?;
        let config = tokens
            .source_binding
            .as_ref()?
            .original_argument_lexer_config(tokens)?;
        let span = node.variable_source_span(expr_base?, config)?;
        let mut accesses = tokens
            .variable_accesses
            .iter()
            .filter(|access| access.source.span == span && access.original_spelling == *text);
        let access = accesses.next()?;
        accesses.next().is_none().then_some(access)
    }

    fn captured_expression_variable_access(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        expression: Option<&crate::command_binding::ExecutedExpressionSource>,
    ) -> Option<&'a crate::command_binding::SourceVariableAccess> {
        let Some(expression) = expression else {
            return self.expression_variable_access(node, expr_base);
        };
        let (origin, source) = expression.variable_source(node)?;
        let tokens = self.source_tokens()?;
        if tokens.source_binding.as_ref()?.source_origin()? != origin {
            return None;
        }
        let crate::expr_ast::ExprNode::Var { text, .. } = node else {
            return None;
        };
        tokens
            .variable_access_for_site(&source)
            .filter(|access| access.original_spelling == *text)
    }

    /// Representation at one original expression read, retaining the exact
    /// parser extent or evaluated-expression operand origin. Missing, observed,
    /// unknown or disagreeing physical reads yield no representation proof.
    #[must_use]
    pub fn read_expression_representation(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        expression: Option<&crate::command_binding::ExecutedExpressionSource>,
        registry: &CommandRegistry,
    ) -> Option<tcl_syntax::value::ValueRepresentation> {
        self.read_expression_representation_advice(node, expr_base, expression, registry)
            .representation
    }

    /// Closed List/Dict possibilities for conversion-cost advice at the exact
    /// expression read. This grants no single representation or erasure proof.
    #[must_use]
    pub fn read_expression_representation_alternatives(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        expression: Option<&crate::command_binding::ExecutedExpressionSource>,
        registry: &CommandRegistry,
    ) -> Option<crate::native_numeric::ClosedContainerRepresentations> {
        self.read_expression_representation_advice(node, expr_base, expression, registry)
            .container_alternatives
    }

    pub(crate) fn read_expression_representation_advice(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        expression: Option<&crate::command_binding::ExecutedExpressionSource>,
        registry: &CommandRegistry,
    ) -> SsaReadRepresentationAdvice {
        self.captured_expression_variable_access(node, expr_base, expression)
            .map_or_else(SsaReadRepresentationAdvice::default, |access| {
                source_read_representations(access, registry)
            })
    }

    /// Resolve an expression variable using its absolute source extent.
    /// The caller supplies the parser's original expression base; absent bases,
    /// transformed extents and conflicting read inventories yield no proof.
    #[must_use]
    pub fn read_expression_variable(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
    ) -> Option<SsaReadReference> {
        let access = self.expression_variable_access(node, expr_base)?;
        self.read_reference(&access.source, &access.original_spelling)
    }

    /// Resolve one variable through exact evaluated-expression operand origins.
    /// A mapped source from another invocation origin cannot donate a read.
    #[must_use]
    pub fn read_executed_expression_variable(
        self,
        node: &crate::expr_ast::ExprNode,
        expression: &crate::command_binding::ExecutedExpressionSource,
    ) -> Option<SsaReadReference> {
        let (origin, source) = expression.variable_source(node)?;
        if self
            .source_tokens()?
            .source_binding
            .as_ref()?
            .source_origin()?
            != origin
        {
            return None;
        }
        let crate::expr_ast::ExprNode::Var { text, .. } = node else {
            return None;
        };
        self.read_reference(&source, text)
    }

    /// Physical address of one exact expression reference, independently of
    /// its represented value. Missing authored extents or read carriers decline.
    #[must_use]
    pub fn read_expression_variable_place(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        registry: &CommandRegistry,
    ) -> Option<tcl_names::place::Place> {
        let access = self.expression_variable_access(node, expr_base)?;
        Some(source_read_place(access, registry))
    }

    /// Whether an exact read selects this activation's unobserved scalar slot.
    /// `slot` is an already validated literal slot name. Namespace, persistent,
    /// selected-frame and element bindings cannot inherit an ordinary formal's
    /// caller-supplied value evidence through a matching source spelling.
    #[must_use]
    pub fn read_expression_is_current_activation_slot(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        slot: &str,
        registry: &CommandRegistry,
    ) -> bool {
        let Some(access) = self.expression_variable_access(node, expr_base) else {
            return false;
        };
        let place = source_read_place(access, registry);
        !place.observed
            && !place.dynamic
            && place.kind == tcl_names::place::PlaceKind::Scalar
            && place.index.is_none()
            && place.name == slot
            && place.cell.as_ref().is_some_and(|cell| {
                cell.name == slot
                    && matches!(&cell.owner, tcl_names::place::CellOwner::Activation(identity)
                        if access.variable_context.activation.as_ref() == Some(identity))
            })
    }

    /// Prove an ordinary incoming slot across all reached physical activations.
    /// A compatibility join may lose a unique physical symbol while this
    /// purpose-specific input proof remains valid. Unknown residuals, changed
    /// contents, observers and static/reference targets decline atomically.
    #[must_use]
    pub fn read_expression_incoming_slot(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        slot: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaIncomingSlotRead> {
        let access = self.expression_variable_access(node, expr_base)?;
        if access.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed {
            return None;
        }
        let mut cells = Vec::new();
        for context in access.context_alternatives() {
            let cell =
                context.incoming_activation_slot(&access.original_spelling, slot, registry)?;
            if !cells.contains(&cell) {
                cells.push(cell);
            }
        }
        (!cells.is_empty()).then(|| SsaIncomingSlotRead {
            source: access.source.clone(),
            slot: slot.to_owned(),
            cells,
        })
    }

    /// Read contents alternatives for this exact expression occurrence.
    /// A physical binding without a single represented version may still have
    /// bounded reaching writes. Unknown residuals remain explicit.
    #[must_use]
    pub fn read_expression_variable_contents(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        registry: &CommandRegistry,
    ) -> Option<SsaReadContents> {
        let access = self.expression_variable_access(node, expr_base)?;
        self.read_contents_at(&access.source, &access.original_spelling, registry)
    }

    /// Already numeric source-produced object at this exact original read.
    /// This is representation evidence, not an invented scalar SSA version.
    #[must_use]
    pub fn read_expression_native_numeric(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        registry: &CommandRegistry,
    ) -> Option<crate::tcl_expr_eval::RetainedNativeOperandProof> {
        crate::native_numeric::source_read_operand(
            self.expression_variable_access(node, expr_base)?,
            registry,
        )
    }
}
