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
