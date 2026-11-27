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

//! Validators that guard a sink: a branch's true edge on a registry
//! validator call proves colours of the variable it tests.

use super::super::{
    BlockId, CfgFunction, ExprNode, TaintColour, TaintLattice, TaintWarning, reg_colour,
};
use super::sink_emission::sink_colour_mitigated;
use crate::ssa::SsaFunction;
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

/// The validator guards of one function, read once per report.
pub(in super::super) struct ValidatorGuards<'a> {
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    registry: &'a CommandRegistry,
    config: tcl_lexer::LexerConfig,
    predecessors: HashMap<BlockId, Vec<BlockId>>,
}

impl<'a> ValidatorGuards<'a> {
    pub(in super::super) fn new(
        cfg: &'a CfgFunction,
        ssa: &'a SsaFunction,
        registry: &'a CommandRegistry,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        let mut predecessors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        for (&id, block) in &cfg.blocks {
            for successor in block
                .terminator
                .iter()
                .flat_map(crate::cfg::Terminator::successors)
            {
                predecessors.entry(successor).or_default().push(id);
            }
        }
        Self {
            cfg,
            ssa,
            registry,
            config,
            predecessors,
        }
    }

    /// The colours proved of `name`, read at `version`, in `block`: every
    /// dominating block entered only by a branch's true edge contributes the
    /// validators its condition calls on that same version, alone or as a
    /// conjunct of `&&`. Any other condition shape proves nothing.
    fn proved(
        &self,
        block: BlockId,
        name: &str,
        version: u32,
        shadowed: &HashSet<String, impl std::hash::BuildHasher>,
    ) -> TaintColour {
        let mut colours = TaintColour::empty();
        let mut current = Some(block);
        while let Some(at) = current {
            if let Some([guard]) = self.predecessors.get(&at).map(Vec::as_slice)
                && let Some(crate::cfg::Terminator::Branch {
                    condition,
                    true_target,
                    false_target,
                    ..
                }) = self
                    .cfg
                    .blocks
                    .get(guard)
                    .and_then(|block| block.terminator.as_ref())
                && *true_target == at
                && *false_target != at
                && self.ssa.blocks.get(guard).is_some_and(|guard| {
                    guard.exit_versions.iter().any(|(&symbol, &held)| {
                        self.ssa.var_name(symbol) == name && held == version
                    })
                })
            {
                colours |= self.condition_colours(condition, name, shadowed);
            }
            current = self.ssa.idom.get(&at).copied().flatten();
        }
        colours
    }

    fn condition_colours(
        &self,
        condition: &ExprNode,
        name: &str,
        shadowed: &HashSet<String, impl std::hash::BuildHasher>,
    ) -> TaintColour {
        match condition {
            ExprNode::Binary {
                op: crate::expr_ast::BinOp::And | crate::expr_ast::BinOp::WordAnd,
                left,
                right,
            } => {
                self.condition_colours(left, name, shadowed)
                    | self.condition_colours(right, name, shadowed)
            }
            ExprNode::Command { text, .. } => self
                .call_colours(text, name, shadowed)
                .unwrap_or_else(TaintColour::empty),
            _ => TaintColour::empty(),
        }
    }

    fn call_colours(
        &self,
        text: &str,
        name: &str,
        shadowed: &HashSet<String, impl std::hash::BuildHasher>,
    ) -> Option<TaintColour> {
        let (command, args) =
            crate::value_shapes::parse_command_substitution_with_config(text, self.config)?;
        if shadowed.contains(command.trim_start_matches("::")) {
            return None;
        }
        let validator = self.registry.get(&command)?.taint_validator?;
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let (index, colours) = validator(&refs)?;
        let word = args.get(index)?;
        let tested = word
            .strip_prefix("${")
            .and_then(|rest| rest.strip_suffix('}'))
            .or_else(|| word.strip_prefix('$'))?;
        (tested == name).then(|| reg_colour(colours))
    }

    /// Keep only the warnings at a statement of `block` that the guards on
    /// their variable do not mitigate.
    pub(in super::super) fn retain_unvalidated(
        &self,
        block: BlockId,
        uses: &HashMap<crate::ssa::Symbol, u32>,
        shadowed: &HashSet<String, impl std::hash::BuildHasher>,
        warnings: &mut Vec<TaintWarning>,
        from: usize,
    ) {
        let mut index = from;
        while index < warnings.len() {
            let warning = &warnings[index];
            let version = uses
                .iter()
                .find(|(symbol, _)| self.ssa.var_name(**symbol) == warning.variable)
                .map(|(_, version)| *version);
            let colours = version.map_or_else(TaintColour::empty, |version| {
                self.proved(block, &warning.variable, version, shadowed)
            });
            if !colours.is_empty() && self.mitigates(warning, colours) {
                warnings.remove(index);
            } else {
                index += 1;
            }
        }
    }

    fn mitigates(&self, warning: &TaintWarning, colours: TaintColour) -> bool {
        let refined = TaintLattice::tainted().with(colours);
        sink_colour_mitigated(warning.code, refined)
            || tcl_registry::taint::taint_sink_safe_colour(self.registry, &warning.sink_command)
                .map(reg_colour)
                .is_some_and(|safe| safe != TaintColour::LIST_CANONICAL && colours.contains(safe))
    }
}
