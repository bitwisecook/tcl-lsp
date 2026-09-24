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

//! Analysis inputs over literal words alone: the shape a `const_fold`
//! callback sees, so a shipped folder can be the route it names rather
//! than a second implementation beside it.

use tcl_dialect::{DialectProfile, TclVersion};

use crate::arg_role::ArgRole;
use crate::invocation_words::InvocationWordKind;

use super::CommandSemantics;
use super::answers::{EvalAnswer, ExactValue, ExactValueOrUnavailable};
use super::context::{AnalysisContext, BindingIdentity, Budget};
use super::decline::{AnalysisTier, DeclineReason};
use super::inputs::{
    AnalysisInputs, BodyRegion, EvaluationState, FactDomain, FactView, InvocationLayout, OperandId,
    OperandView, PlaceRef, ResolvedInvocationView, WordPart, WordStructure,
};

/// Inputs over literal words: every operand is exact, no place has a
/// prior fact, and the context is detached under the given profile.
pub struct LiteralInputs<'a> {
    view: ResolvedInvocationView<'a>,
    context: AnalysisContext,
    priors: Vec<(String, ExactValue)>,
    /// The operands written as brace-quoted words.
    braced: Vec<OperandId>,
}

impl<'a> LiteralInputs<'a> {
    /// The invocation `command ?subcommand? args…` over literal words, in
    /// the resolver's coordinate system (the subcommand word is operand
    /// 0 when there is one).
    #[must_use]
    pub fn new(
        command: &'a str,
        subcommand: Option<&'a str>,
        args: &[&'a str],
        profile: Option<&'static DialectProfile>,
    ) -> Self {
        let operands = subcommand
            .into_iter()
            .chain(args.iter().copied())
            .map(|text| OperandView {
                text,
                kind: InvocationWordKind::Literal,
                role: None,
            })
            .collect();
        Self {
            view: ResolvedInvocationView {
                canonical_command: command,
                subcommand,
                form: None,
                layout: InvocationLayout::Source,
                operands,
                argument_offset: usize::from(subcommand.is_some()),
            },
            context: AnalysisContext::detached(profile),
            priors: Vec::new(),
            braced: Vec::new(),
        }
    }

    /// Operand `id` as a brace-quoted word: its structure is its text as one
    /// literal run from offset 1, past the opening brace — what a template
    /// plan decomposes. Any other operand's structure is unavailable.
    #[must_use]
    pub fn with_braced(mut self, id: OperandId) -> Self {
        self.braced.push(id);
        self
    }

    /// The value the scalar place `name` holds before the invocation runs.
    #[must_use]
    pub fn with_prior(mut self, name: &str, value: ExactValue) -> Self {
        self.priors.push((name.to_owned(), value));
        self
    }

    /// The role the resolver gives operand `id` — the registry's
    /// `arg_indices_for_role` answer for the same words — so a
    /// specialisation that finds its places by role can run here.
    #[must_use]
    pub fn with_role(mut self, id: OperandId, role: ArgRole) -> Self {
        if let Some(operand) = self.view.operands.get_mut(id.0) {
            operand.role = Some(role);
        }
        self
    }
}

impl AnalysisInputs for LiteralInputs<'_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        if domain != FactDomain::ExactValue {
            return FactView::Top(DeclineReason::Unavailable(AnalysisTier::Structure));
        }
        match self.view.operand(id) {
            Some(operand) => FactView::Exact(ExactValue::from_literal(operand.text), None),
            None => FactView::Top(DeclineReason::NotExact),
        }
    }

    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        self.view
            .operand(id)
            .map(|operand| PlaceRef::scalar(operand.text))
            .ok_or(DeclineReason::NotExact)
    }

    fn variable(&self, _name: &str, _domain: FactDomain) -> FactView {
        FactView::Top(DeclineReason::NotExact)
    }

    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
        if domain != FactDomain::ExactValue {
            return FactView::Top(DeclineReason::Unavailable(AnalysisTier::Structure));
        }
        self.priors
            .iter()
            .find(|(name, _)| *name == place.name)
            .map_or(FactView::Top(DeclineReason::NotExact), |(_, value)| {
                FactView::Exact(value.clone(), None)
            })
    }

    fn word_structure(&self, id: OperandId) -> Result<WordStructure, DeclineReason> {
        if !self.braced.contains(&id) {
            return Err(DeclineReason::Unsupported);
        }
        let text = self.view.operand(id).ok_or(DeclineReason::NotExact)?.text;
        let end = u32::try_from(text.len())
            .map_err(|_| DeclineReason::NotExact)?
            .saturating_add(1);
        Ok(WordStructure {
            braced: true,
            parts: vec![WordPart::Literal {
                span: tcl_lexer::Span::new(1, end),
                text: text.to_owned(),
            }],
        })
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
        &self.context
    }
}

/// Run a pure route over literal words, as a `const_fold` callback does:
/// the exact result's text when the route evaluates without stores under
/// `version`'s profile — or, with no version, under the answer every
/// modelled release gives — and `None` for every decline.
#[must_use]
pub fn evaluate_literal(
    semantics: &dyn CommandSemantics,
    command: &str,
    subcommand: Option<&str>,
    args: &[&str],
    version: Option<TclVersion>,
) -> Option<String> {
    let profile = version.and_then(|v| DialectProfile::find(v.dialect_profile_name()));
    let inputs = LiteralInputs::new(command, subcommand, args, profile);
    match semantics.evaluate(&inputs, &mut Budget::evaluation()) {
        EvalAnswer::Evaluated(outcome) if !outcome.has_stores() => match outcome.result {
            ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes).ok(),
            ExactValueOrUnavailable::Unavailable(_) => None,
        },
        EvalAnswer::Pending | EvalAnswer::Declined(_) | EvalAnswer::Evaluated(_) => None,
    }
}
