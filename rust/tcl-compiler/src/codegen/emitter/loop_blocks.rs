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

//! Per-block handlers for loop-related CFG patterns.
//!
//! foreach opcode compilation, loop-end result handling, for-init /
//! while startCommand wrapping. Each handler corresponds to one
//! branch of the block-name prefix check in the emitter.

#![allow(dead_code, clippy::implicit_hasher, clippy::doc_markdown)]

use std::collections::{HashMap, HashSet};

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ir::Statement;

/// Metadata about a foreach loop detected in the CFG.
#[derive(Debug, Clone)]
pub struct ForeachInfo {
    /// Body block name.
    pub body: String,
    /// End block name.
    pub end: String,
    /// List arguments to the foreach/lmap command.
    pub list_args: Vec<String>,
    /// Whether each list argument was written as a **braced literal** word
    /// (`foreach e {a[b]c x}`), parallel to `list_args`.
    ///
    /// A braced word is a literal: `TclFindElement`'s brace semantics mean
    /// neither the word nor any braced element inside it takes `$` / `[…]`
    /// substitution. Without this the emitter pushes the list as an ordinary
    /// literal and the VM's `subst_word` runs the `[…]` at loop entry, so
    /// `foreach e {{a[b]c} x}` raises `invalid command name "b"`. The fact is
    /// retained on the typed iterator and checked against the genuine synthetic
    /// header's original operand metadata before codegen.
    pub list_braced: Vec<bool>,
    /// Loop-variable groups from the retained typed iteration owner. Carried onto the
    /// `FOREACH_START` instruction so the VM can bind them (C Tcl `ForeachInfo`).
    pub var_groups: Vec<Vec<String>>,
    /// This is an `lmap` (a collecting loop): the codegen strips the body's
    /// trailing `POP` and emits `LMAP_COLLECT` so each iteration's result is
    /// gathered VM-side, and `FOREACH_END` yields `list(accum)`.
    pub collect: bool,
}

/// Metadata about a complex foreach: one whose body block terminates with a
/// Branch (the body's first/only control structure is an `if`/loop whose
/// condition becomes the body terminator). The body may carry statements — e.g.
/// the `<cond>` placeholder of an inline command substitution in the condition.
///
/// The emitter must (a) emit `FOREACH_STEP`/`FOREACH_END` at the
/// foreach_end block rather than the body, (b) route continue/break
/// through synthetic step/break labels, and (c) suppress back-edge
/// gotos from body blocks to the foreach header.
#[derive(Debug, Clone)]
pub struct ComplexForeach {
    /// foreach_end_N block name.
    pub end: String,
    /// Label placed before `FOREACH_STEP` (continue target).
    pub step_label: String,
    /// Label placed between `FOREACH_STEP` and `FOREACH_END`
    /// (break target — past the step opcode).
    pub break_label: String,
    /// Body blocks reachable from the foreach body (excluding end).
    pub body_blocks: HashSet<String>,
}

/// Join retained typed list loops to their current synthetic CFG headers.
/// A missing correspondence or orphan iteration marker requires a provider.
#[must_use]
pub fn detect_foreach(cfg: &CfgFunction) -> Option<HashMap<String, ForeachInfo>> {
    let mut info: HashMap<String, ForeachInfo> = HashMap::new();
    let mut retained_headers = HashSet::new();
    for (exit, owner) in &cfg.loop_nodes {
        let Statement::Foreach {
            iterators,
            is_lmap,
            is_dict_iteration: false,
            is_array_iteration: false,
            ..
        } = &owner.statement
        else {
            continue;
        };
        let Some(Terminator::Goto { target: header, .. }) = cfg
            .blocks
            .get(&owner.start)
            .and_then(|block| block.terminator.as_ref())
        else {
            return None;
        };
        let Some(block) = cfg.blocks.get(header) else {
            return None;
        };
        let Some(terminator @ Terminator::Branch {
            true_target,
            false_target,
            ..
        }) = &block.terminator
        else {
            return None;
        };
        if owner.start != owner.entry_block
            || owner.span != owner.statement.span()
            || false_target != exit
            || !cfg.blocks.contains_key(true_target)
            || !cfg.blocks.contains_key(exit)
            || cfg.terminator_sources.get(header) != Some(&owner.executed_source)
        {
            return None;
        }
        let [statement] = block.statements.as_slice() else {
            return None;
        };
        if cfg.statement_sources.get(&(*header, 0)) != Some(&owner.executed_source)
            || !crate::cfg_builder::matches_foreach_header(
                &owner.statement,
                statement,
                terminator,
            )
        {
            return None;
        }
        retained_headers.insert((*header, 0));
        info.insert(
            cfg.block_name(*header).to_owned(),
            ForeachInfo {
                body: cfg.block_name(*true_target).to_owned(),
                end: cfg.block_name(*exit).to_owned(),
                list_args: iterators.iter().map(|iterator| iterator.list_arg.clone()).collect(),
                list_braced: iterators.iter().map(|iterator| iterator.list_braced).collect(),
                var_groups: iterators.iter().map(|iterator| iterator.vars.clone()).collect(),
                collect: *is_lmap,
            },
        );
    }
    for (block, contents) in &cfg.blocks {
        for (index, statement) in contents.statements.iter().enumerate() {
            if statement.tokens().is_some_and(|tokens| {
                tokens.synthetic == Some(crate::ir::SyntheticMarker::IterationBindings(Some(
                    tcl_registry::TclType::List,
                )))
            }) && !retained_headers.contains(&(*block, index)) {
                return None;
            }
        }
    }
    Some(info)
}

/// Detect complex foreach loops from the foreach info map.
///
/// A complex foreach has an empty body block whose terminator is a
/// `Branch` (not a `Goto`). Returns a map keyed by the foreach header
/// block name.
#[must_use]
pub fn detect_complex_foreach(
    cfg: &CfgFunction,
    foreach_info: &HashMap<String, ForeachInfo>,
    label_counter: &mut u32,
) -> HashMap<String, ComplexForeach> {
    let mut result: HashMap<String, ComplexForeach> = HashMap::new();
    for (header, info) in foreach_info {
        let Some(body_blk) = cfg.block_by_name(&info.body) else {
            continue;
        };
        // A *simple* foreach is a single straight-line body block that loops
        // directly back to the header (`Goto header`); its `FOREACH_STEP`/
        // `FOREACH_END` are emitted inline after the body. Anything else is
        // complex — the step/end move to the foreach_end block and body
        // back-edges route to the step label. This covers both a branching body
        // (an `if`, terminator `Branch`) *and* a body that flows through further
        // blocks before looping (a nested `for`/`while`/`foreach`, whose tail
        // `Goto`s the header from a *different* block — keying only on `Branch`
        // here mis-classified that as simple and emitted the outer step/end
        // before the inner loop ran, an infinite loop).
        let is_simple = matches!(
            &body_blk.terminator,
            Some(Terminator::Goto { target, .. }) if cfg.block_name(*target) == header
        );
        if is_simple {
            continue;
        }
        let step_label = format!("foreach_continue_{}", *label_counter);
        *label_counter += 1;
        let break_label = format!("foreach_break_{}", *label_counter);
        *label_counter += 1;

        // Collect body blocks (reachable from body, excluding end).
        let mut body_blocks: HashSet<String> = HashSet::new();
        super::ordering::collect_loop_body(
            cfg,
            &info.body,
            header,
            &mut body_blocks,
            Some(&info.end),
        );

        result.insert(
            header.clone(),
            ComplexForeach {
                end: info.end.clone(),
                step_label,
                break_label,
                body_blocks,
            },
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loop_cfg(source: &str) -> CfgFunction {
        let registry = tcl_registry::CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(source, &registry);
        crate::cfg_builder::build_cfg_codegen_with_registry(&module, false, &registry).top_level
    }

    #[test]
    fn foreach_detection_uses_typed_loop_owners_instead_of_reported_command_names() {
        for (source, collect) in [
            ("foreach value {one two} {puts $value}", false),
            ("lmap value {one two} {set out $value}", true),
        ] {
            let mut cfg = loop_cfg(source);
            let original = detect_foreach(&cfg).unwrap();
            assert_eq!(original.len(), 1, "{source}");
            assert_eq!(original.values().next().unwrap().collect, collect);
            let header = cfg.block_id(original.keys().next().unwrap()).unwrap();
            let statement = cfg.blocks.get_mut(&header).unwrap().statements.iter_mut().find(
                |statement| matches!(statement, Statement::Call { tokens: Some(tokens), .. }
                    if matches!(tokens.synthetic, Some(crate::ir::SyntheticMarker::IterationBindings(_)))),
            ).unwrap();
            let Statement::Call { command, tokens: Some(tokens), .. } = statement else {
                unreachable!("selected synthetic header");
            };
            *command = "unrelated reported command".to_owned();
            tokens.argv_texts[0] = "another reported head".to_owned();
            let selected = detect_foreach(&cfg).unwrap();
            assert_eq!(selected.len(), 1);
            assert_eq!(selected.values().next().unwrap().collect, collect);
            assert_eq!(selected.values().next().unwrap().list_args, original.values().next().unwrap().list_args);
            assert_eq!(selected.values().next().unwrap().var_groups, original.values().next().unwrap().var_groups);
        }
    }

    #[test]
    fn foreach_detection_refuses_missing_or_changed_operation_correspondence() {
        let cfg = loop_cfg("foreach value {one two} {puts $value}");
        let selected = detect_foreach(&cfg).unwrap();
        assert_eq!(selected.len(), 1);
        let header = cfg.block_id(selected.keys().next().unwrap()).unwrap();
        let index = cfg.blocks[&header].statements.iter().position(
            |statement| matches!(statement, Statement::Call { tokens: Some(tokens), .. }
                if matches!(tokens.synthetic, Some(crate::ir::SyntheticMarker::IterationBindings(_)))),
        ).unwrap();
        let mut changed = cfg.clone();
        changed.loop_nodes.clear();
        assert!(detect_foreach(&changed).is_none());
        let mut changed = cfg.clone();
        let extra = changed.blocks[&header].statements[index].clone();
        changed.blocks.get_mut(&header).unwrap().statements.push(extra);
        assert!(detect_foreach(&changed).is_none());
        let mut changed = cfg.clone();
        changed.statement_sources.remove(&(header, index));
        assert!(detect_foreach(&changed).is_none());
        let mut changed = cfg.clone();
        changed.terminator_sources.remove(&header);
        assert!(detect_foreach(&changed).is_none());
        let mut changed = cfg.clone();
        let Statement::Call { args, .. } = &mut changed.blocks.get_mut(&header).unwrap().statements[index] else {
            unreachable!("selected synthetic header");
        };
        args[0] = "different list".to_owned();
        assert!(detect_foreach(&changed).is_none());
        let mut changed = cfg.clone();
        let Statement::Call { tokens: Some(tokens), .. } = &mut changed.blocks.get_mut(&header).unwrap().statements[index] else {
            unreachable!("selected synthetic header");
        };
        tokens.synthetic = None;
        assert!(detect_foreach(&changed).is_none());
        let mut changed = cfg;
        let Some(Terminator::Branch { condition, .. }) = &mut changed.blocks.get_mut(&header).unwrap().terminator else {
            unreachable!("selected iterator condition");
        };
        *condition = crate::expr_ast::ExprNode::Raw { text: "$condition".to_owned() };
        assert!(detect_foreach(&changed).is_none());
    }
}
