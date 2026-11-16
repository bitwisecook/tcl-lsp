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

//! Questions other passes ask of the transfer: proven word values over solved facts, and structure-only questions over source words.

use super::place_named;
use std::cell::RefCell;
use tcl_lexer::LexerConfig;
use tcl_registry::InvocationWordKind;
use tcl_registry::value_transfer::{
    AnalysisContext, AnalysisInputs, AnalysisTier, BindingIdentity, BodyRegion, DeclineReason,
    EvalAnswer, EvaluationState, ExactValue, FactDomain, FactView, OperandId, PlaceRef,
    ResolvedInvocationView, WordStructure,
};

mod dict_bodies;
mod proven_values;
mod resolved_calls;
mod templates_and_selections;

pub(crate) use dict_bodies::{
    DictBinder, dict_body_operand_with_metadata_context, dict_body_with_metadata_context,
};
#[cfg(test)]
pub(crate) use dict_bodies::{dict_body, dict_body_operand};
pub(super) use proven_values::written_before;
pub use proven_values::{
    StatementId, SubstitutionHost, proven_return_word_value, proven_substituted_word_value,
    proven_word_value, return_command_tokens, substitution_calls,
};
pub(crate) use proven_values::{
    proven_return_word_value_with_facts, proven_substituted_word_value_with_facts,
    proven_word_value_with_facts,
};
pub(crate) use resolved_calls::{
    cell_update_range_model, literal_keyed_update, resolved_body_absorbs_completion,
    resolved_cell_update, resolved_iteration_plan, resolved_literal_semantics,
    selected_iteration_plan, typed_incr_creates_absent,
};
#[cfg(test)]
pub(crate) use templates_and_selections::LiteralSelection;
pub use templates_and_selections::{SourceWord, literal_template_plan};
pub(crate) use templates_and_selections::{
    WordForm, literal_selection, source_template_plan, statement_selection,
};

/// The variable a branch condition's `Raw` operand reads (§ `switch`, step
/// 1): the variable-name owner proves the text is exactly one reference
/// under its complete config, and the name holds none of `{`,
/// `}` or `\`, the three characters on which the release close rules and
/// the name readers can disagree — so a backslash-bearing `${…}` subject,
/// which `Raw` exists to carry intact under either rule, stays `Raw` and
/// decides nothing. Any other `Raw` text is never read as a variable.
pub(crate) fn whole_variable_operand(text: &str, config: LexerConfig) -> Option<&str> {
    tcl_syntax::naming::whole_variable_reference_literal_name(text, config)
        .filter(|name| !name.contains(['{', '}', '\\']))
}

/// Inputs for a structure-only question over source words: literal words
/// are exact, dynamic words are not, and a literal name is a place.
pub(crate) struct StructureInputs<'a> {
    pub(super) view: ResolvedInvocationView<'a>,
    pub(super) context: &'a AnalysisContext,
    /// What a prior store reads.
    prior: Prior<'a>,
    /// The operands the plan has read as exact values, in the order it read
    /// them.
    exact_reads: RefCell<Vec<OperandId>>,
}

/// What a structure question reads as the prior value of a place.
#[derive(Clone, Copy)]
pub(super) enum Prior<'a> {
    /// Nothing: the question has no store to read.
    Unavailable,
    /// The value the caller knows, for the one place a plan reads (a body
    /// plan's dictionary).
    Known(&'a str),
    /// The least dictionary holding, each inside the one before, the exact
    /// words the plan read before it: any key path the plan descends
    /// exists and holds no key, so the plan answers its shape and binds
    /// nothing.
    KeyPath,
}

impl<'a> StructureInputs<'a> {
    pub(crate) fn new(view: ResolvedInvocationView<'a>, context: &'a AnalysisContext) -> Self {
        Self {
            view,
            context,
            prior: Prior::Unavailable,
            exact_reads: RefCell::default(),
        }
    }

    /// The texts of the operands the plan has read as exact values, in the
    /// order it read them.
    fn exact_read_texts(&self) -> Vec<&'a str> {
        self.exact_reads
            .borrow()
            .iter()
            .filter_map(|&id| self.view.operand(id).map(|operand| operand.text))
            .collect()
    }
}

impl AnalysisInputs for StructureInputs<'_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        match self.view.operand(id) {
            Some(operand)
                if domain == FactDomain::ExactValue
                    && operand.kind == InvocationWordKind::Literal =>
            {
                self.exact_reads.borrow_mut().push(id);
                FactView::Exact(ExactValue::from_literal(operand.text), None)
            }
            _ => FactView::Top(DeclineReason::NotExact),
        }
    }

    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        match self.view.operand(id) {
            Some(operand) if operand.kind == InvocationWordKind::Literal => {
                Ok(place_named(operand.text))
            }
            Some(_) => Err(DeclineReason::DynamicName),
            None => Err(DeclineReason::NotExact),
        }
    }

    fn variable(&self, _name: &str, _domain: FactDomain) -> FactView {
        FactView::Top(DeclineReason::Unavailable(AnalysisTier::Structure))
    }

    fn prior_store(&self, _place: &PlaceRef, domain: FactDomain) -> FactView {
        match self.prior {
            Prior::Known(prior) if domain == FactDomain::ExactValue => {
                FactView::Exact(ExactValue::from_literal(prior), None)
            }
            Prior::KeyPath if domain == FactDomain::ExactValue => {
                let dictionary = self
                    .exact_read_texts()
                    .into_iter()
                    .rev()
                    .fold(String::new(), |inner, key| {
                        tcl_syntax::list::join_list([key, inner.as_str()])
                    });
                FactView::Exact(ExactValue::from_literal(&dictionary), None)
            }
            _ => FactView::Top(DeclineReason::Unavailable(AnalysisTier::Structure)),
        }
    }

    fn word_structure(&self, _id: OperandId) -> Result<WordStructure, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn body(&self, _id: OperandId) -> Result<BodyRegion, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn nested(&self, _script: &str, _state: &mut EvaluationState) -> EvalAnswer {
        EvalAnswer::Declined(DeclineReason::Unsupported)
    }

    fn math_function(&self, _name: &str) -> Result<BindingIdentity, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn context(&self) -> &AnalysisContext {
        self.context
    }
}
