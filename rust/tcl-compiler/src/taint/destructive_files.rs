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

//! Destructive `file` subcommands on tainted paths, with normalisation and branch guards.

use super::sink_warnings::{arg_index_span, original_scalar_reference_name};
use super::{TaintColour, TaintLattice, TaintWarning, ordered_argument_candidates};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, ValueKey};
use crate::value_shapes::parse_command_substitution_with_spans_and_config;
use rustc_hash::FxHashSet;
use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;
use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, Traits};
use tcl_source::limits::MAX_EXPR_NODE_DEPTH;

/// Explicit standalone warning utility. Actual Module/FU callers use
/// [`find_destructive_file_warnings_for_module_function`].
///
/// **W313.** Flag destructive `file` operations (`file delete` /
/// `rename` / `mkdir` — the registry's `destructive` subcommands) whose
/// path argument is a variable, since the path may carry user-controlled
/// content (path-traversal).  Suppressed when the path variable is both
/// normalised (`PATH_NORMALISED` colour, or its SSA def is
/// `[file normalize …]`) *and* bounds-checked — the latter via
/// branch-guard analysis ([`compute_branch_guard_map`]) or a
/// `PATH_BOUNDED` colour.  The message is softened (not suppressed) for a
/// normalised-but-unguarded path.
#[must_use]
pub fn find_destructive_file_warnings<S: std::hash::BuildHasher, E: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<TaintWarning> {
    let destructive = destructive_file_subs(registry);
    if destructive.is_empty() {
        return Vec::new();
    }
    // This standalone entry selects the complete grammar from its supplied dialect.
    let config = tcl_lexer::LexerConfig::for_profile(dialect);
    let guard_map = compute_branch_guard_map(cfg, registry, config);

    let mut warnings: Vec<TaintWarning> = Vec::new();
    for bn in cfg_order(cfg) {
        if !executable_blocks.contains(&bn) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&bn) else {
            continue;
        };
        let empty = HashSet::new();
        let block_guarded = guard_map.get(&bn).unwrap_or(&empty);
        for ssa_stmt in &ssa_block.statements {
            let Statement::Call {
                command,
                canonical_command,
                args,
                span,
                ..
            } = &ssa_stmt.statement
            else {
                continue;
            };
            if canonical_command.as_deref() != Some("::file") && command != "file" {
                continue;
            }
            let Some(sub) = args.first() else {
                continue;
            };
            if !destructive.contains(sub.as_str()) {
                continue;
            }
            // Skip `-force` / `--` to find the path arguments.
            let mut path_start = 1;
            while path_start < args.len() && matches!(args[path_start].as_str(), "-force" | "--") {
                path_start += 1;
            }
            // One W313 per statement (first offending path variable).
            // Collect candidate path variables in argument (source) order —
            // tagged with their argument index so the warning can anchor at
            // the offending *path argument*, not the whole command:
            // `ssa_stmt.uses` is a `HashMap`, so iterating it would pick a
            // nondeterministic variable for a multi-path sink like
            // `file rename $a $b` — making the warning's message (and the memo
            // vs whole-module builds) differ run-to-run.
            let mut seen: FxHashSet<String> = FxHashSet::default();
            let mut ordered: Vec<(usize, String)> = Vec::new();
            for (idx, a) in args.iter().enumerate().skip(path_start) {
                for name in ordered_argument_candidates(a, config, ssa, &ssa_stmt.uses) {
                    if seen.insert(name.clone()) {
                        ordered.push((idx, name));
                    }
                }
            }
            for (arg_idx, name) in &ordered {
                let Some(sym) = ssa.var_symbol(name) else {
                    continue;
                };
                let Some(&ver) = ssa_stmt.uses.get(&sym) else {
                    continue;
                };
                let t = taints
                    .get(&(sym, ver))
                    .copied()
                    .unwrap_or(TaintLattice::clean());
                let is_normalised = (t.is_tainted()
                    && t.colours.contains(TaintColour::PATH_NORMALISED))
                    || is_normalised_def(name, ver, ssa);
                let is_bounded = (t.is_tainted() && t.colours.contains(TaintColour::PATH_BOUNDED))
                    || (is_normalised && block_guarded.contains(name));
                if is_bounded {
                    continue;
                }
                let message = if is_normalised {
                    format!(
                        "file {sub} with normalised path (${name}) — verify it stays \
                         within the intended directory (e.g. [string match \"$base/*\" \
                         ${{{name}}}])."
                    )
                } else {
                    format!(
                        "file {sub} with a variable path (${name}) risks path-traversal. \
                         Normalise with [file normalize] and verify it stays within the \
                         intended directory."
                    )
                };
                warnings.push(TaintWarning {
                    source_sink: None,
                    // Anchor at the offending path argument's own token span
                    // (mirroring T102's per-argument targeting); the whole
                    // statement span is the tokens-unavailable fallback.
                    span: arg_index_span(
                        &ssa_stmt.statement,
                        *arg_idx,
                        tcl_lexer::LexerConfig::for_profile(dialect),
                    )
                    .unwrap_or(*span),
                    variable: name.clone(),
                    sink_command: format!("file {sub}"),
                    code: DiagCode::W313,
                    message,
                    replacement: None,
                    fixes: Vec::new(),
                });
                break;
            }
        }
    }
    warnings
}

/// The `file` subcommands flagged `destructive` in the registry
/// (`delete` / `rename` / `mkdir`).
fn destructive_file_subs(registry: &CommandRegistry) -> HashSet<&'static str> {
    registry.get("file").map_or_else(HashSet::new, |spec| {
        spec.subcommands
            .iter()
            .filter(|s| s.destructive)
            .map(|s| s.name)
            .collect()
    })
}

/// True when the SSA def of `name`@`ver` is a `[file normalize …]`
/// command substitution.
fn is_normalised_def(name: &str, ver: u32, ssa: &SsaFunction) -> bool {
    let Some(sym) = ssa.var_symbol(name) else {
        return false;
    };
    for ssa_block in ssa.blocks.values() {
        for ssa_stmt in &ssa_block.statements {
            if ssa_stmt.defs.get(&sym) == Some(&ver) {
                if let Statement::AssignValue { value, .. } = &ssa_stmt.statement {
                    return value.trim().starts_with("[file normalize ");
                }
                return false;
            }
        }
    }
    false
}

/// Build a `block → {bounds-checked var}` map: a `Branch` whose condition
/// is `[string match|first|equal … $var]` marks `$var` as `PATH_BOUNDED`
/// in the guarded successor (and its exclusive successors).
pub(super) fn compute_branch_guard_map(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> HashMap<BlockId, HashSet<String>> {
    let mut guarded: HashMap<BlockId, HashSet<String>> = HashMap::new();
    for block in cfg.blocks.values() {
        let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            ..
        }) = &block.terminator
        else {
            continue;
        };
        let (negated, var) = extract_guard_var(condition, false, 0, config);
        let Some(var) = var else {
            continue;
        };
        let (guarded_target, other_target) = if negated {
            (*false_target, *true_target)
        } else {
            (*true_target, *false_target)
        };
        propagate_guard(
            cfg,
            guarded_target,
            other_target,
            &var,
            registry,
            &mut guarded,
        );
    }
    guarded
}

/// Extract `(negated, path-var)` from a branch condition: a unary
/// operator flips negation; a binary operator checks both sides; a
/// `[string …]` command sub yields the bounds-checked variable.
pub(super) fn extract_guard_var(
    expr: &ExprNode,
    negated: bool,
    depth: u32,
    config: tcl_lexer::LexerConfig,
) -> (bool, Option<String>) {
    // Native-stack safety net: this walks the `ExprNode` tree,
    // one native frame per level. Past the cap, report "no guard variable
    // found" — the same conservative result as the leaf `_` arm, so no
    // read-narrowing is applied where the condition was too deep to analyse.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return (negated, None);
    }
    match expr {
        ExprNode::Unary { operand, .. } => extract_guard_var(operand, !negated, depth + 1, config),
        ExprNode::Binary { left, right, .. } => {
            let (n, r) = extract_guard_var(left, negated, depth + 1, config);
            if r.is_some() {
                return (n, r);
            }
            extract_guard_var(right, negated, depth + 1, config)
        }
        ExprNode::Command { text, .. } => (negated, guard_var_from_string_command(text, config)),
        _ => (negated, None),
    }
}

/// Parse `[string match|first|equal … $var]` and return the path
/// variable it bounds-checks.
pub(super) fn guard_var_from_string_command(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let (command, args) = parse_command_substitution_with_spans_and_config(text, config)?;
    if command != "string" || args.is_empty() {
        return None;
    }
    let name =
        |argument: &(String, Span)| extract_var_name(text.get(argument.1.as_range())?, config);
    match args[0].0.as_str() {
        "match" if args.len() >= 3 => name(args.last()?),
        "first" if args.len() >= 3 => name(&args[2]),
        "equal" => {
            let mut skip_next = false;
            for argument in &args[1..] {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                if argument.0 == "-length" {
                    skip_next = true;
                    continue;
                }
                if argument.0.starts_with('-') {
                    continue;
                }
                if let Some(name) = name(argument) {
                    return Some(name);
                }
            }
            None
        }
        _ => None,
    }
}

pub(super) fn extract_var_name(arg: &str, config: tcl_lexer::LexerConfig) -> Option<String> {
    let original = crate::value_shapes::value_word_with_config(arg, config)?;
    original_scalar_reference_name(&original, config).map(str::to_owned)
}

/// True when block `id` executes a block-terminating command
/// (`error` / `return` / `exit` — the `TERMINATES_BLOCK` trait) so its
/// successors are unreachable from this path.
fn is_dead_end_block(cfg: &CfgFunction, id: BlockId, registry: &CommandRegistry) -> bool {
    let Some(block) = cfg.blocks.get(&id) else {
        return false;
    };
    block.statements.iter().any(|stmt| {
        matches!(stmt, Statement::Call { command, .. }
            if registry.get(command).is_some_and(|s| s.traits.contains(Traits::TERMINATES_BLOCK)))
    })
}

/// Mark `guarded_target` and its successors that aren't also reachable
/// from `other_target` (i.e. before the merge point) as guarding `var`.
/// When `other_target` is a dead-end (error/return), the merge is only
/// reachable through the guard, so the guard extends through it.
fn propagate_guard(
    cfg: &CfgFunction,
    guarded_target: BlockId,
    other_target: BlockId,
    var: &str,
    registry: &CommandRegistry,
    guarded: &mut HashMap<BlockId, HashSet<String>>,
) {
    let mut other_reachable: HashSet<BlockId> = HashSet::new();
    if !is_dead_end_block(cfg, other_target, registry) {
        let mut stack = vec![other_target];
        while let Some(b) = stack.pop() {
            if other_reachable.contains(&b) {
                continue;
            }
            let Some(block) = cfg.blocks.get(&b) else {
                continue;
            };
            other_reachable.insert(b);
            for succ in block.terminator.iter().flat_map(Terminator::successors) {
                stack.push(succ);
            }
        }
    }
    let mut visit = vec![guarded_target];
    let mut visited: HashSet<BlockId> = HashSet::new();
    while let Some(b) = visit.pop() {
        if visited.contains(&b) {
            continue;
        }
        if other_reachable.contains(&b) && b != guarded_target {
            continue;
        }
        let Some(block) = cfg.blocks.get(&b) else {
            continue;
        };
        visited.insert(b);
        guarded.entry(b).or_default().insert(var.to_owned());
        for succ in block.terminator.iter().flat_map(Terminator::successors) {
            visit.push(succ);
        }
    }
}
