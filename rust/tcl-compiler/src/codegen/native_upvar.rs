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

//! Ordinary UPVAR emits one retained level and ordered original alias targets.

use super::{CodegenCtx, NativeEmissionTask as Task, Op};
use tcl_registry::native_upvar_compilation::NativeUpvarInstruction;

impl CodegenCtx<'_> {
    pub(super) fn append_native_upvar_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeUpvarInstruction,
        operations: &mut Vec<Task>,
    ) -> bool {
        let level = match recipe.level {
            Some(level) => Self::native_namespace_word_task(&command.words, level),
            None => Some(Task::Literal(b"1".to_vec())),
        };
        let Some(level) = level else { return false };
        operations.push(level);
        for binding in recipe.bindings {
            let Some(other) = Self::native_namespace_word_task(&command.words, binding.other)
            else {
                return false;
            };
            operations.push(other);
            operations.push(Task::DeclareNamespaceLocal(binding.local.clone()));
            operations.push(Task::NamespaceLocalOperation(Op::UPVAR, binding.local));
        }
        operations.push(Task::Operation(Op::POP, vec![]));
        operations.push(Task::Literal(Vec::new()));
        true
    }
}
