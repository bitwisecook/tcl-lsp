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

//! Whole-frame effects on the existence rung: the places a statement, a terminator or a body run in this frame may define or destroy.

use super::Clobber;
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::ssa::{SsaFunction, Symbol};
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

/// The clobber one statement performs: every place for a barrier or a
/// static-body `uplevel`, the computed-name facts its words raise, and the
/// places a nested body it keeps inline may define or destroy.
pub(super) fn statement_clobber(
    statement: &Statement,
    ssa: &SsaFunction,
    (cfg, block, index): (&CfgFunction, BlockId, usize),
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Clobber {
    let mut clobber = Clobber::of_names(crate::dynamic_names::statement_barrier_for_cfg(
        statement,
        (cfg, block, index),
        registry,
        config,
    ));
    if matches!(
        statement,
        Statement::Barrier { .. } | Statement::UpFrame { .. }
    ) || crate::ssa::is_unseen_call_marker(statement)
    {
        clobber.all = true;
        return clobber;
    }
    let mut touch = BodyTouch::default();
    for script in crate::ir_helpers::nested_bodies(statement) {
        touch.script(script, registry, config, 0);
    }
    touch.statement_words(statement, registry, config, 0);
    clobber.all |= touch.all;
    clobber.touched = touched_symbols(&touch.names, ssa);
    clobber
}

/// The clobber a block's terminator performs: the computed-name facts its
/// condition or returned word raise, and the places a script body nested
/// in one of its command substitutions may define or destroy.
pub(super) fn terminator_clobber(
    terminator: &Terminator,
    ssa: &SsaFunction,
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Clobber {
    let mut clobber = Clobber::of_names(crate::dynamic_names::terminator_barrier_for_cfg(
        terminator, cfg, registry, config,
    ));
    let mut touch = BodyTouch::default();
    match terminator {
        Terminator::Branch { condition, .. } => touch.expr(condition, registry, config, 0),
        Terminator::Return { value, expr, .. } => {
            if let Some(value) = value {
                touch.word(value, registry, config, 0);
            }
            if let Some(expr) = expr {
                touch.expr(expr, registry, config, 0);
            }
        }
        // Complete records an already observed invocation in this block;
        // its operands and effects are accounted for by the original statement.
        Terminator::Goto { .. } | Terminator::Complete { .. } => {}
    }
    clobber.all |= touch.all;
    clobber.touched = touched_symbols(&touch.names, ssa);
    clobber
}

/// The symbols of every place `named` holds: a place and the array it sits
/// in, and every element of an array touched whole.
fn touched_symbols(named: &HashSet<String>, ssa: &SsaFunction) -> Vec<Symbol> {
    if named.is_empty() {
        return Vec::new();
    }
    let bases: HashSet<&str> = named.iter().map(|name| place_base(name)).collect();
    ssa.var_names()
        .iter()
        .enumerate()
        .filter(|(_, name)| {
            named.contains(name.as_str())
                || bases.contains(name.as_str())
                || bases.contains(place_base(name))
        })
        .map(|(index, _)| Symbol(u32::try_from(index).unwrap_or(u32::MAX)))
        .collect()
}

/// What the script bodies a statement runs in this frame may define or
/// destroy: an inline nested body's (a non-lowered `switch`'s arms), and one
/// nested in a command substitution — `[catch {unset x}]`, `[eval {…}]`,
/// `[lmap v {1} {…}]` run their body here, so the rung clobbers every name
/// it defines or destroys (#2231's consequence). A
/// body lowers to the statements the IR builds for it and each is asked
/// what it defines ([`crate::ssa::defs_of_with_registry`]); a barrier or an
/// up-frame among them, or text nested past the depth cap, touches every
/// place.
#[derive(Default)]
struct BodyTouch {
    /// The places the bodies define or destroy.
    pub(in super::super) names: HashSet<String>,
    /// Whether a body may touch any place.
    pub(super) all: bool,
}

impl BodyTouch {
    /// Every statement of `script`, however nested: what it defines, and
    /// the substitutions its own words run.
    pub(in super::super) fn script(
        &mut self,
        script: &crate::ir::Script,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        depth: u32,
    ) {
        crate::ir::for_each_statement(script, &mut |inner| {
            if matches!(inner, Statement::Barrier { .. } | Statement::UpFrame { .. }) {
                self.all = true;
            }
            self.names
                .extend(crate::ssa::defs_of_with_registry(inner, Some(registry)));
            self.statement_words(inner, registry, config, depth);
        });
    }

    /// The command substitutions one statement's words run.
    fn statement_words(
        &mut self,
        statement: &Statement,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        depth: u32,
    ) {
        match statement {
            Statement::Call { args, .. } | Statement::Barrier { args, .. } => {
                for arg in args {
                    self.word(arg, registry, config, depth);
                }
            }
            Statement::AssignConst { value, .. }
            | Statement::AssignValue { value, .. }
            | Statement::Switch { subject: value, .. } => {
                self.word(value, registry, config, depth);
            }
            Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
                self.expr(expr, registry, config, depth);
            }
            Statement::Return { value, expr, .. } => {
                if let Some(value) = value {
                    self.word(value, registry, config, depth);
                }
                if let Some(expr) = expr {
                    self.expr(expr, registry, config, depth);
                }
            }
            _ => {}
        }
    }

    /// Every `[…]` in a word's spelling.
    pub(in super::super) fn word(
        &mut self,
        text: &str,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        depth: u32,
    ) {
        for inner in crate::var_refs::command_subst_texts_with_config(text, config) {
            self.substitution(&inner, registry, config, depth + 1);
        }
    }

    /// Every `[…]` in an expression.
    pub(in super::super) fn expr(
        &mut self,
        expr: &ExprNode,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        depth: u32,
    ) {
        let mut commands = Vec::new();
        crate::ir_helpers::collect_expr_commands(expr, &mut commands);
        for text in &commands {
            let trimmed = text.trim();
            let inner = trimmed
                .strip_prefix('[')
                .and_then(|inner| inner.strip_suffix(']'))
                .unwrap_or(trimmed);
            self.substitution(inner, registry, config, depth + 1);
        }
    }

    /// One substitution's script: the bodies its commands run in this frame
    /// — never its own commands' definitions, which the statements the
    /// lowering places for a substitution already state — and the
    /// substitutions its words nest. A command the lowering keeps opaque
    /// (`[eval $script]`, `[dict with d {…}]`) may touch any place.
    pub(in super::super) fn substitution(
        &mut self,
        script: &str,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        depth: u32,
    ) {
        if tcl_source::limits::MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            self.all = true;
            return;
        }
        // A script with no braced or quoted word holds no body, and one with
        // no bracket nests no substitution.
        if !script.contains(['{', '"', '[']) {
            return;
        }
        let touched = substitution_touch_memo(script, registry, config, depth, || {
            Self::substitution_touch(script, registry, config, depth)
        });
        self.all |= touched.all;
        self.names.extend(touched.names.iter().cloned());
    }

    /// Kept out of line, with the lowered module boxed, so each nested
    /// substitution's stack frame stays small: the recursion reaches
    /// `MAX_BRACKET_TEXT_DEPTH` levels within `MIN_SOURCE_WALK_STACK`.
    #[inline(never)]
    fn substitution_touch(
        script: &str,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        depth: u32,
    ) -> Self {
        let mut touch = Self::default();
        let module = lower_substitution(script, registry, config);
        for statement in &module.top_level.statements {
            if matches!(
                statement,
                Statement::Barrier { .. } | Statement::UpFrame { .. }
            ) {
                touch.all = true;
            }
            for body in crate::ir_helpers::nested_bodies(statement) {
                touch.script(body, registry, config, depth);
            }
            touch.statement_words(statement, registry, config, depth);
        }
        touch
    }
}

#[inline(never)]
fn lower_substitution(
    script: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Box<crate::ir::Module> {
    Box::new(crate::lowering::lower_to_ir_with_dialect(
        script,
        registry,
        config,
        registry.profile(),
    ))
}

/// What one substitution's text touches is a pure function of the text, the
/// command store, the grammar and the nesting depth, and the same `[expr …]`
/// recurs across rungs, solves and edits. Lowering it again runs the whole
/// source-binding walk, so the answer is shared, under a bound.
fn substitution_touch_memo(
    script: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    depth: u32,
    compute: impl FnOnce() -> BodyTouch,
) -> std::sync::Arc<BodyTouch> {
    use std::sync::{Arc, Mutex, OnceLock};
    type Key = (
        String,
        tcl_registry::RegistrySemanticKey,
        tcl_lexer::LexerConfig,
        u32,
    );
    static MEMO: OnceLock<Mutex<HashMap<Key, Arc<BodyTouch>>>> = OnceLock::new();
    const MAX_RETAINED: usize = 4096;
    let key = (
        script.to_owned(),
        registry.snapshot().semantic_key(),
        config,
        depth,
    );
    let memo = MEMO.get_or_init(Mutex::default);
    if let Some(touched) = memo
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&key)
    {
        return Arc::clone(touched);
    }
    let touched = Arc::new(compute());
    let mut retained = memo
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if retained.len() >= MAX_RETAINED {
        retained.clear();
    }
    retained.insert(key, Arc::clone(&touched));
    touched
}

/// The variable that holds the place `name`: the array for an element
/// `base(key)`, the name itself otherwise.
pub(crate) fn place_base(name: &str) -> &str {
    tcl_syntax::naming::split_element_ref(name).map_or(name, |(base, _)| base)
}
