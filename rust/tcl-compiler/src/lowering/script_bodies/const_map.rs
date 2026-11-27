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

//! The locals a body assigns a literal to, and the written value of an argument read through them.

use super::super::Lowerer;
use crate::ir::Statement;
use std::collections::HashMap;
use tcl_lexer::TokenType;
use tcl_source::segment::SegmentedCommand;

/// Return `(name, literal)` when *seg* is `set name {literal}`.
///
/// The LHS must be a plain bareword `Esc` token (no substitutions,
/// array index, or namespace qualifier) so we only ever track
/// proc-local scalars. The RHS must be a single brace-string `Str`
/// token.
pub(in super::super) fn assigned_local_body_literal(
    seg: &SegmentedCommand,
    stmt: &Statement,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, String)> {
    let Statement::AssignConst { name, value, .. } = stmt else {
        return None;
    };
    if seg.args().len() != 2
        || seg.single_token_word.get(..3)? != [true, true, true]
        || name != &seg.args()[0]
        || value != &seg.args()[1]
    {
        return None;
    }
    let arg_tokens = seg.arg_tokens();
    let name_tok = *arg_tokens.first()?;
    let value_tok = *arg_tokens.get(1)?;
    if name_tok.kind != TokenType::Esc || value_tok.kind != TokenType::Str {
        return None;
    }
    let name = crate::value_transfer::literal_token_value(name, name_tok.kind, &config)?;
    if name.is_empty()
        || name.contains('$')
        || name.contains('[')
        || name.contains('(')
        || tcl_syntax::naming::is_qualified(name.as_bytes())
    {
        return None;
    }
    let value = crate::value_transfer::literal_token_value(value, value_tok.kind, &config)?;
    Some((name.into_owned(), value.into_owned()))
}

pub(in super::super) fn invalidate_const_map_for(
    stmt: &Statement,
    scope: &mut HashMap<String, String>,
    config: tcl_lexer::LexerConfig,
) {
    match stmt {
        Statement::AssignConst {
            name, name_braced, ..
        } => {
            if let Some(literal) =
                crate::value_transfer::recorded_word_value(name, *name_braced, &config)
            {
                let root = tcl_syntax::naming::split_element_ref(&literal)
                    .map_or(literal.as_ref(), |(root, _)| root);
                scope.remove(root);
            } else {
                scope.clear();
            }
        }
        // Calls, substitutions and read traces can write outside their reported target.
        Statement::Call { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Barrier { .. }
        | Statement::NativeCall { .. }
        | Statement::Block { .. }
        | Statement::UpFrame { .. }
        | Statement::If { .. }
        | Statement::For { .. }
        | Statement::While { .. }
        | Statement::Foreach { .. }
        | Statement::Catch { .. }
        | Statement::Try { .. }
        | Statement::Switch { .. } => scope.clear(),
        Statement::Return { .. } => {}
    }
}

impl Lowerer<'_> {
    /// Maintain the per-scope const-map after a command lowers.
    ///
    /// Retain a quiet Logical scalar assignment's original literal value
    /// under its decoded local name; unclosed writes withdraw retained entries.
    /// Gated on `proc_depth > 0` — globals / namespace vars at
    /// top-level or inside `namespace eval` cannot be safely
    /// const-propagated.
    ///
    /// Kept out of [`Self::lower_segmented`]'s frame: its carrier locals are
    /// several KiB, and that frame recurs once per nested body level.
    #[inline(never)]
    pub(in super::super) fn update_const_map(&mut self, seg: &SegmentedCommand, stmt: &Statement) {
        if self.proc_depth == 0 {
            return;
        }
        let quiet_store = if matches!(stmt, Statement::AssignConst { .. }) {
            let tokens = self.cmd_tokens(seg);
            tokens.source_binding.as_ref().and_then(|binding| {
                binding.original_conditional_logical_scalar_store(
                    &tokens,
                    self.invocation_metadata_context()?,
                    self.registry,
                )
            })
        } else {
            None
        };
        let literal = assigned_local_body_literal(seg, stmt, self.config).filter(|(_, value)| {
            quiet_store
                .as_ref()
                .is_some_and(|store| store.value() == Some(value.as_str()))
        });
        let Some(scope) = self.const_map_stack.last_mut() else {
            return;
        };
        if let Some((name, value)) = literal {
            scope.insert(name, value);
        } else if matches!(stmt, Statement::AssignConst { .. }) && quiet_store.is_none() {
            scope.clear();
        } else {
            invalidate_const_map_for(stmt, scope, self.config);
        }
    }

    pub(in super::super) fn original_native_written_argument_value(
        &self,
        seg: &SegmentedCommand,
        argument: usize,
    ) -> Option<String> {
        self.native_entry.as_ref()?;
        let tokens = self.cmd_tokens(seg);
        let binding = tokens.source_binding.as_ref()?;
        (binding.original_invocation_source_purpose(
            &tokens,
            self.invocation_metadata_context()?,
            self.registry,
        )? == crate::command_binding::OriginalInvocationSourcePurpose::Execution)
            .then_some(())?;
        binding
            .evaluated_written_argument_value(argument)
            .map(str::to_owned)
    }

    pub(in super::super) fn original_authored_written_argument_value(
        &self,
        seg: &SegmentedCommand,
        argument: usize,
    ) -> Option<String> {
        let tokens = self.cmd_tokens(seg);
        tokens
            .source_binding
            .as_ref()?
            .original_authored_written_argument_value(
                &tokens,
                argument,
                self.invocation_metadata_context()?,
                self.registry,
            )
            .map(str::to_owned)
    }

    /// Read a simple body reference from the quiet Logical local-value map.
    /// Unknown, array and compound references retain runtime dispatch.
    pub(in super::super) fn const_map_lookup(&self, word: &str) -> Option<String> {
        if self.proc_depth == 0 {
            return None;
        }
        let name = tcl_syntax::naming::whole_scalar_variable_reference_name(word, self.config)?;
        self.const_map_stack.last()?.get(name).cloned()
    }
}
