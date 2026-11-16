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

//! Instantiating a body template: restoring source proofs and relocating variable proofs through an alpha-renaming.

use super::FunctionUnit;
use crate::ir::Statement;
use crate::semantic_analysis::SemanticAnalysisBundle;

fn relocate_statement_variable_proofs(
    statement: &mut Statement,
    relocation: &crate::var_resolve::VariableProofRelocation,
) {
    if let Some(tokens) = statement.tokens_mut() {
        tokens.relocate_variable_proofs(relocation);
    }
    if let Statement::Block { body, .. } = statement {
        for site in body.command_binding_sites.iter_mut() {
            site.relocate_variable_proofs(relocation);
        }
        for statement in &mut body.statements {
            relocate_statement_variable_proofs(statement, relocation);
        }
    }
}

impl FunctionUnit {
    /// Restore actual command and read ownership on an instantiated body template.
    /// A missing exact site declines the complete artifact without mutating this unit.
    #[must_use]
    pub fn restored_source_proofs(
        &self,
        proofs: &crate::command_binding::BodySourceProofs,
    ) -> Option<Self> {
        if !self.sccp.required_math_invocations.is_empty()
            || !self.sccp.required_expression_preparations.is_empty()
        {
            return None;
        }
        let mut unit = self.clone();
        unit.invalidate_semantic_values();
        if !proofs.restore_cfg(&mut unit.cfg) || !unit.ssa.restore_source_tokens(&proofs.tokens) {
            return None;
        }
        for block in unit.ssa.blocks.values_mut() {
            for statement in &mut block.statements {
                if !proofs.restore_statement(&mut statement.statement) {
                    return None;
                }
            }
        }
        Some(unit)
    }

    /// Restore only independently certified readonly source carriers.
    pub(crate) fn restored_readonly_source_proofs(
        &self,
        proofs: &crate::command_binding::BodySourceProofs,
    ) -> Option<Self> {
        if !self.sccp.required_math_invocations.is_empty()
            || !self.sccp.required_expression_preparations.is_empty()
        {
            return None;
        }
        let mut unit = self.clone();
        unit.invalidate_semantic_values();
        if !proofs.restore_readonly_cfg(&mut unit.cfg)
            || !unit.ssa.restore_source_tokens(&proofs.tokens)
        {
            return None;
        }
        for block in unit.ssa.blocks.values_mut() {
            for statement in &mut block.statements {
                if !proofs.restore_readonly_statement(&mut statement.statement) {
                    return None;
                }
            }
        }
        Some(unit)
    }

    /// Instantiate an injective alpha-renaming of physical variable allocations.
    /// Symbols and lattice versions remain stable; lexical source rebasing is separate.
    /// The source owner must rebuild semantic facts from the actual source before use.
    /// Ambiguous identities or invalid state graphs decline the memoised artifact.
    #[must_use]
    pub fn relocated_variable_proofs(
        &self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) -> Option<Self> {
        relocation.inverse()?;
        if self.ssa.source_variable_symbols().next().is_some() {
            return None;
        }
        let mut unit = self.clone();
        unit.invalidate_semantic_values();
        unit.ssa.relocate_variable_proofs(relocation);
        unit.cfg.namespace_context = unit
            .cfg
            .namespace_context
            .as_deref()
            .map(|key| Box::new(relocation.namespace_key(key)));
        let keys: std::collections::HashSet<_> = unit.ssa.cell_keys().iter().collect();
        if keys.len() != unit.ssa.cell_keys().len() {
            return None;
        }
        unit.def_use.relocate_variable_proofs(relocation);
        if let Some(memory) = &mut unit.memory_ssa {
            memory.relocate_variable_proofs(relocation).ok()?;
        }
        for site in unit
            .cfg
            .command_binding_sites
            .iter_mut()
            .chain(unit.cfg.command_boundary_sites.values_mut())
            .chain(unit.cfg.condition_binding_sites.values_mut())
        {
            site.relocate_variable_proofs(relocation);
        }
        for block in unit.cfg.blocks.values_mut() {
            for statement in &mut block.statements {
                relocate_statement_variable_proofs(statement, relocation);
            }
            if let Some(crate::cfg::Terminator::Return {
                tokens: Some(tokens),
                ..
            }) = &mut block.terminator
            {
                tokens.relocate_variable_proofs(relocation);
            }
        }
        for node in unit.cfg.loop_nodes.values_mut() {
            relocate_statement_variable_proofs(&mut node.statement, relocation);
        }
        for block in unit.ssa.blocks.values_mut() {
            for statement in &mut block.statements {
                relocate_statement_variable_proofs(&mut statement.statement, relocation);
            }
        }
        unit.semantic_facts = SemanticAnalysisBundle::unavailable(self.semantic_facts.context());
        Some(unit)
    }
}
