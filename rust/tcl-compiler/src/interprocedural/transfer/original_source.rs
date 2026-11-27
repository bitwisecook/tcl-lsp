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

//! The module's retained original source, read at one function point: call arguments, statement tokens, expressions, substitutions and native operands.

use super::ModuleProcedures;
use super::source_context::{OriginalSummaryExpression, OriginalSummaryScript, ProcedureCall};
use crate::cfg::Function as CfgFunction;
use crate::ir::{CommandTokens, Statement};
use crate::sccp::FoldTrust;
use tcl_registry::CommandRegistry;

impl<'a> ModuleProcedures<'a> {
    pub(super) fn resolve_original_word(
        &self,
        tokens: &CommandTokens,
        word: &str,
        trust: FoldTrust,
    ) -> Option<crate::ir::ProcedureId> {
        let ids = crate::registry_invocation::original_source_procedure_ids_for_word_for_module(
            tokens,
            word,
            self.ir,
            self.registry,
        )?;
        let [id] = ids.as_slice() else {
            return None;
        };
        let procedure = self.source.as_ref()?.procedure(id)?;
        let name = &procedure.qualified_name;
        let stands = !self.redefined.contains(name)
            && match trust {
                FoldTrust::WholeModule => self.mutations.trusts_proc_binding(name),
                FoldTrust::ObservedBindings => {
                    self.projection.trusts_proc_binding(name)
                        && self.mutations.observed_proc_binding(name)
                }
            };
        stands.then_some(*id)
    }

    pub(crate) fn body_for_cfg(&self, cfg: &CfgFunction) -> Option<crate::sccp::AnalysisBody> {
        if std::ptr::eq(cfg, std::ptr::from_ref(&self.cfg.top_level)) {
            return Some(crate::sccp::AnalysisBody::TopLevel);
        }
        let mut candidates = self
            .cfg
            .procedures
            .iter()
            .filter(|(_, row)| std::ptr::eq(cfg, *row))
            .map(|(id, _)| *id)
            .chain(
                self.units
                    .into_iter()
                    .flat_map(|units| units.iter())
                    .filter(|(_, row)| std::ptr::eq(cfg, std::ptr::from_ref(&row.cfg)))
                    .map(|(id, _)| *id),
            );
        let first = candidates.next()?;
        candidates
            .all(|id| id == first)
            .then_some(crate::sccp::AnalysisBody::Procedure(first))
    }

    /// A procedure summary call requires the same retained source point,
    /// original declaration and effective argv. A reporting head is no join.
    pub(crate) fn original_call_arguments(
        &self,
        call: &ProcedureCall<'_>,
    ) -> Option<Vec<Option<String>>> {
        call.literal_arguments(self.ir)
    }

    pub(crate) fn original_statement_tokens(
        &self,
        body_id: Option<crate::ir::ProcedureId>,
        statement: &Statement,
    ) -> Option<CommandTokens> {
        if let Some(source) = &self.source {
            source.statement_tokens(body_id, statement)
        } else {
            self.native_source
                .as_ref()?
                .statement_tokens(body_id, statement)
        }
    }

    pub(crate) fn original_expression(
        &self,
        parent: &CommandTokens,
        expression: &crate::expr_ast::ExprNode,
        base: Option<u32>,
    ) -> Option<OriginalSummaryExpression> {
        if let Some(source) = &self.source {
            source.expression(parent, expression, base)
        } else {
            self.native_source
                .as_ref()?
                .expression(parent, expression, base)
        }
    }

    pub(crate) fn original_expression_substitution(
        &self,
        expression: &OriginalSummaryExpression,
        script: &str,
        start: u32,
        end: u32,
    ) -> Option<OriginalSummaryScript> {
        if let Some(source) = &self.source {
            source.expression_substitution(expression, script, start, end)
        } else {
            self.native_source
                .as_ref()?
                .expression_substitution(expression, script, start, end)
        }
    }

    pub(crate) fn original_substitution(
        &self,
        parent: &CommandTokens,
        site: &tcl_source::words::SourceSite,
        script: &str,
    ) -> Option<OriginalSummaryScript> {
        if let Some(source) = &self.source {
            source.substitution(parent, site, script)
        } else {
            self.native_source
                .as_ref()?
                .substitution(parent, site, script)
        }
    }

    pub(crate) fn original_body(
        &self,
        parent: &CommandTokens,
        argument: usize,
        script: &str,
    ) -> Option<OriginalSummaryScript> {
        self.source.as_ref()?.body(parent, argument, script)
    }

    pub(crate) fn resolve_call_at(
        &self,
        tokens: &CommandTokens,
        trust: FoldTrust,
    ) -> Option<ProcedureCall<'a>> {
        let call = self.source.as_ref()?.procedure_call(tokens)?;
        let name = &call.procedure.qualified_name;
        let stands = !self.redefined.contains(name)
            && match trust {
                FoldTrust::WholeModule => self.mutations.trusts_proc_binding(name),
                FoldTrust::ObservedBindings => {
                    self.projection.trusts_proc_binding(name)
                        && self.mutations.observed_proc_binding(name)
                }
            };
        stands.then_some(call)
    }

    /// Borrow the selected original operation and its effective source argv
    /// under this Module's complete point metadata. This is conditional value
    /// modelling, independently of Native execution and observer permissions.
    pub(crate) fn with_original_operation<T>(
        &self,
        tokens: &CommandTokens,
        apply: impl FnOnce(
            &crate::registry_invocation::ResolvedStatementInvocation,
            &tcl_registry::ResolvedInvocation<'_, '_>,
        ) -> Option<T>,
    ) -> Option<T> {
        self.source.as_ref()?.with_operation(tokens, apply)
    }

    pub(crate) fn has_original_native_source(&self) -> bool {
        self.native_source.is_some()
    }

    pub(crate) fn with_original_native_store<T>(
        &self,
        body: Option<crate::ir::ProcedureId>,
        tokens: &CommandTokens,
        defs: &[(String, crate::ssa::ValueKey)],
        ssa: &crate::ssa::SsaFunction,
        apply: impl FnOnce(
            &crate::registry_invocation::ResolvedStatementInvocation,
            &tcl_registry::ResolvedInvocation<'_, '_>,
        ) -> Option<T>,
    ) -> Option<T> {
        self.native_source
            .as_ref()?
            .with_store_operation(body, tokens, defs, ssa, apply)
    }

    pub(crate) fn with_original_native_value<T>(
        &self,
        tokens: &CommandTokens,
        writes_land: bool,
        apply: impl FnOnce(
            &crate::registry_invocation::ResolvedStatementInvocation,
            &tcl_registry::ResolvedInvocation<'_, '_>,
        ) -> Option<T>,
    ) -> Option<T> {
        self.native_source
            .as_ref()?
            .with_value_operation(tokens, writes_land, apply)
    }

    /// Why [`Self::with_original_native_value`] refuses `tokens`; `None`
    /// where it admits the command or no native source is retained.
    pub(crate) fn original_native_value_refusal(
        &self,
        tokens: &CommandTokens,
        writes_land: bool,
    ) -> Option<super::NativeValueRefusal> {
        self.native_source
            .as_ref()?
            .value_refusal(tokens, writes_land)
    }

    /// Retained source Module; callers still own their declaration/input joins.
    pub(crate) const fn original_source_module(&self) -> &'a crate::ir::Module {
        self.ir
    }

    /// Reborrow this function's complete input only under the same Module owner.
    pub(crate) fn invocation_metadata_context_for_function<'f>(
        &self,
        function: &'f crate::compilation_unit::FunctionUnit,
        registry: &CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'f>> {
        function.invocation_metadata_context_for_module(registry, self.ir)
    }

    /// Metadata for an exact original function point under this Module owner.
    /// Availability, whole source and original vector correspondence remain
    /// distinct from a successful command or physical variable operation.
    pub(crate) fn original_invocation_metadata_for_function<'f>(
        &self,
        function: &crate::compilation_unit::FunctionUnit,
        tokens: &'f crate::ir::CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'f>> {
        self.invocation_metadata_context_for_function(function, registry)?;
        tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_module(tokens, self.ir, registry)
    }

    /// A built callee keeps its own source input. Its missing input is not
    /// repaired from a sibling module record; fresh CFGs use the actual module.
    pub(super) fn source_metadata_input_for(
        &self,
        callee: &crate::ir::ProcedureId,
    ) -> Option<&'a crate::analyser::ResolvedAnalysisInput> {
        match self.units {
            Some(units) => units
                .get(callee)?
                .invocation_metadata_context_for_module(self.registry, self.ir)?
                .source_analysis_input(),
            None => crate::registry_invocation::InvocationMetadataContext::for_module(
                self.registry,
                self.ir,
            )?
            .source_analysis_input(),
        }
    }
}
