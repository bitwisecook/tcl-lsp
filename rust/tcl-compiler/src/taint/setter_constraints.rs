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

//! IRULE3101: setter arguments that violate a declared constraint.

use super::sink_warnings::{original_scalar_reference_name, static_word_expr_value};
use super::source_context::TaintSourceContext;
use super::{TaintColour, TaintLattice, TaintWarning, is_irules_dialect};
use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, ValueKey};
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

// IRULE3101 — setter-constraint violations

/// Find setter-constraint violations (IRULE3101). Currently constrains
/// `HTTP::uri` / `HTTP::path` setters to paths beginning with `/`.
///
/// Dialect-gated: returns an empty vector unless `dialect` is
/// `"f5-irules"` / `"irules"`. The gate is applied internally (defence
/// in depth) so a caller outside `compiler_checks::run_all_checks`
/// can't accidentally emit IRULE3101 errors against user-defined
/// commands that happen to be named `HTTP::uri` / `HTTP::path`.
///
/// Original literal operands are checked under their word configuration.
/// A whole scalar read can use only the value or colour represented at that
/// statement; missing geometry or contents cannot mitigate a dynamic operand.
/// Explicit standalone utility; retained Module/FU callers use
/// [`find_setter_constraint_warnings_for_module_function`].
#[must_use]
pub fn find_setter_constraint_warnings<
    S: std::hash::BuildHasher,
    V: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
>(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    values: Option<&HashMap<ValueKey, crate::analyses::LatticeValue, V>>,
    executable_blocks: &HashSet<BlockId, E>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<TaintWarning> {
    let mut out: Vec<TaintWarning> = Vec::new();
    if !is_irules_dialect(dialect) {
        return out;
    }
    let standalone_config = TaintSourceContext::standalone(registry, dialect).config;
    let safe_path_colours =
        TaintColour::PATH_PREFIXED | TaintColour::PATH_NORMALISED | TaintColour::PATH_BOUNDED;

    for bn in cfg_order(cfg) {
        if !executable_blocks.contains(&bn) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&bn) else {
            continue;
        };

        for ssa_stmt in &ssa_block.statements {
            let Statement::Call { command, args, .. } = &ssa_stmt.statement else {
                continue;
            };
            let span = ssa_stmt.statement.span();
            // The setter constraints are command metadata, read straight
            // from the registry spec (`HTTP::uri` / `HTTP::path` declare
            // their `/`-prefix IRULE3101 rule) rather than a hardcoded
            // table here.
            for constraint in tcl_registry::taint::setter_constraints(registry, command) {
                let Some(arg_val) = args.get(constraint.arg_index as usize) else {
                    continue;
                };
                let original = ssa_stmt
                    .statement
                    .tokens()
                    .and_then(|tokens| tokens.word_exprs.get(constraint.arg_index as usize + 1));
                let config = match ssa_stmt.statement.tokens().and_then(|tokens| {
                    tokens
                        .source_binding
                        .as_ref()
                        .map(|binding| (tokens, binding))
                }) {
                    Some((tokens, binding)) => binding.original_lexer_config_for_tokens(tokens),
                    None => Some(standalone_config),
                };
                let warn = |variable: String| TaintWarning {
                    source_sink: None,
                    span,
                    variable,
                    sink_command: command.clone(),
                    code: constraint.code,
                    message: constraint.message.to_owned(),
                    replacement: None,
                    fixes: Vec::new(),
                };

                let Some(config) = config else {
                    out.push(warn(String::new()));
                    continue;
                };
                let Some(original) = original.filter(|word| word.legacy_text() == *arg_val) else {
                    // Independent IR without original word geometry cannot
                    // turn source-looking bytes into a represented read.
                    if arg_val.contains(['$', '['])
                        || !arg_val.starts_with(constraint.required_prefix)
                    {
                        out.push(warn(String::new()));
                    }
                    continue;
                };
                if let Some(literal) = static_word_expr_value(
                    original,
                    config.escapes,
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                ) {
                    if !literal.starts_with(constraint.required_prefix) {
                        out.push(warn(String::new()));
                    }
                    continue;
                }

                if let Some(var_name) = original_scalar_reference_name(original, config) {
                    let represented = ssa.var_symbol(var_name).and_then(|symbol| {
                        ssa_stmt.uses.get(&symbol).map(|version| (symbol, *version))
                    });
                    let proven = represented
                        .and_then(|key| values?.get(&key))
                        .and_then(|value| match value {
                            crate::analyses::LatticeValue::Const(c) => {
                                crate::value_transfer::const_text(c)
                            }
                            _ => None,
                        });
                    if let Some(proven) = proven {
                        if !proven.starts_with(constraint.required_prefix) {
                            out.push(warn(var_name.to_owned()));
                        }
                        continue;
                    }
                    let taint = represented.and_then(|key| taints.get(&key)).copied();
                    if taint.is_some_and(|value| {
                        value.is_tainted() && value.colours.intersects(safe_path_colours)
                    }) {
                        continue;
                    }
                    out.push(warn(var_name.to_owned()));
                    continue;
                }

                // Dynamic (interpolation, command sub, mixed) — always warn.
                out.push(warn(String::new()));
            }
        }
    }

    out
}
