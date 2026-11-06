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

//! Monolithic namespace-upvar emission in original compiler visit order.

use super::{CodegenCtx, NativeEmissionTask as Task, Op};
use tcl_registry::native_namespace_binding_compilation::{
    NativeNamespaceBindingCompilation, NativeNamespaceBindingVisit,
};

#[cfg(test)]
#[path = "native_namespace_upvar_tests.rs"]
mod tests;

impl CodegenCtx<'_> {
    pub(super) fn native_namespace_upvar_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: &NativeNamespaceBindingCompilation,
    ) -> Option<Vec<Task>> {
        let mut tasks = Vec::new();
        let mut binding = recipe.bindings.iter().peekable();
        let mut other = None;
        for visit in &recipe.visits {
            match visit {
                NativeNamespaceBindingVisit::Word(operand) => {
                    tasks.push(Self::native_namespace_word_task(
                        &command.words,
                        operand.clone(),
                    )?);
                    other = Some(operand);
                }
                NativeNamespaceBindingVisit::DeclareLocal(name) => {
                    tasks.push(Task::DeclareNamespaceLocal(name.clone()));
                    if let Some(pair) = binding.peek()
                        && other == Some(&pair.name)
                        && name == &pair.local
                    {
                        tasks.push(Task::NamespaceLocalOperation(Op::NSUPVAR, name.clone()));
                        binding.next();
                        other = None;
                    }
                }
                NativeNamespaceBindingVisit::Literal(_) => return None,
            }
        }
        binding.next().is_none().then_some(tasks)
    }
}
