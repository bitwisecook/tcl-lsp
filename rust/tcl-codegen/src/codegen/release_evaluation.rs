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

//! How the emulated release evaluates and binds the command being emitted, and whether it compiles locals there.

use super::CodegenCtx;
use tcl_lexer::Span;

impl CodegenCtx<'_> {
    /// How the release evaluates the command being emitted.
    pub(super) fn command_evaluation(&self) -> tcl_runtime_api::CommandEvaluation {
        if self.in_direct_command() {
            tcl_runtime_api::CommandEvaluation::Direct
        } else {
            tcl_runtime_api::CommandEvaluation::Compiled
        }
    }

    /// Whether the release evaluates the command being emitted without
    /// compiling it, so reads a literal word in it as the command runs.
    #[must_use]
    pub(crate) fn in_direct_command(&self) -> bool {
        self.expr_depth == 0
            && self.current_span_is_command
            && self
                .current_span
                .is_some_and(|span| self.evaluates_directly(span))
    }

    /// Whether the release evaluates the command at `span` without compiling
    /// it: one of [`Self::direct_commands`], or a command of a script body it
    /// evaluates directly ([`Self::direct_body`]).
    pub(crate) fn evaluates_directly(&self, span: Span) -> bool {
        self.direct_body
            || self.direct_commands.contains(&span)
            || self
                .direct_regions
                .iter()
                .any(|region| region.start() <= span.start() && span.end() <= region.end())
    }

    /// How the release binds an invocation of `command` of `shape`, where it
    /// keeps the commands running bytecode compiled; `None` elsewhere.
    pub(crate) fn release_binding_of(
        &self,
        command: &str,
        shape: &tcl_registry::release_compilation::InvocationShape,
    ) -> Option<tcl_runtime_api::ReleaseBinding> {
        tcl_registry::release_compilation::release_binding(self.vm_release(), command, shape)
    }

    /// The binding a literal-headed invocation of `command` with `words`
    /// after it carries to its `INVOKE` ([`tcl_bytecode::Instruction::release_binding`]).
    /// A command the release evaluates directly binds nothing.
    pub(crate) fn release_invoke_binding(
        &self,
        command: &str,
        words: &[(&str, bool)],
    ) -> Option<tcl_runtime_api::ReleaseBinding> {
        if tcl_syntax::naming::word_is_dynamic(command, false) || self.in_direct_command() {
            return None;
        }
        let args: Vec<&str> = words.iter().map(|&(text, _)| text).collect();
        self.release_binding_of(
            command,
            &self.invocation_shape(&args, |index| words[index].1),
        )
    }

    /// The shape of an invocation whose words after the command are `args`,
    /// the word at each index braced where `braced` says so.
    pub(crate) fn invocation_shape<S: AsRef<str>>(
        &self,
        args: &[S],
        braced: impl Fn(usize) -> bool,
    ) -> tcl_registry::release_compilation::InvocationShape {
        tcl_registry::release_compilation::InvocationShape::of(self.vm_release(), args, |index| {
            !tcl_syntax::naming::word_is_dynamic(args[index].as_ref(), braced(index))
        })
    }

    /// Note that the command being emitted retains `command`'s binding, for
    /// the command's [`tcl_bytecode::Instruction::release_hold`].
    pub(super) fn hold_release_binding(
        &mut self,
        command: &str,
        shape: &tcl_registry::release_compilation::InvocationShape,
    ) {
        if let Some(span) = self.current_span.filter(|_| self.current_span_is_command) {
            self.hold_release_binding_at(span, command, shape);
        }
    }

    /// [`Self::hold_release_binding`] for the command at `span`.
    pub(super) fn hold_release_binding_at(
        &mut self,
        span: Span,
        command: &str,
        shape: &tcl_registry::release_compilation::InvocationShape,
    ) {
        let Some(binding) = self.release_binding_of(command, shape) else {
            return;
        };
        self.release_holds
            .entry(span)
            .and_modify(|held| *held = held.meet(binding))
            .or_insert(binding);
    }

    /// The release hold of the command at `span`
    /// ([`tcl_bytecode::Instruction::release_hold`]): where the release keeps
    /// running bytecode, a command that retains no binding holds everywhere.
    pub(crate) fn release_hold(
        &self,
        span: Option<Span>,
    ) -> Option<tcl_runtime_api::ReleaseBinding> {
        if self.vm_release().revalidates_running_bytecode() {
            return None;
        }
        Some(
            span.and_then(|span| self.release_holds.get(&span).copied())
                .unwrap_or(tcl_runtime_api::ReleaseBinding::Compiled),
        )
    }

    /// Whether a variable named here may be addressed as a *compiled local* of
    /// this function.
    ///
    /// A proc body's variables are its frame's locals — except inside a
    /// same-frame script body this compiler folded in
    /// ([`Self::same_frame_eval_spans`]), which C compiles as a separate unit
    /// with no access to the enclosing proc's local table. Every emitter that
    /// chooses between a slot form and its dispatched `*Stk` sibling asks this,
    /// not [`Self::is_proc`].
    #[must_use]
    pub fn compiles_locals(&self) -> bool {
        self.is_proc
            && self
                .compiled_variable_protocol
                .is_some_and(tcl_syntax::naming::NativeCompiledVariableProtocol::has_indexed_locals)
            && !self.inside_same_frame_eval()
    }

    pub(super) fn source_variable_environment(
        &self,
    ) -> tcl_syntax::naming::NativeCompiledVariableEnvironment {
        use tcl_syntax::naming::NativeCompiledVariableEnvironment;
        if self.compiles_locals() {
            NativeCompiledVariableEnvironment::DeclareProcedure
        } else if self.borrowed_local_layout.is_some() {
            NativeCompiledVariableEnvironment::BorrowFrameSlots
        } else {
            NativeCompiledVariableEnvironment::None
        }
    }

    /// Whether the statement being emitted lies inside a folded same-frame
    /// script body. Matched by source containment, exactly as the executor
    /// matches an instruction to its [`tcl_bytecode::ErrorRegion`].
    fn inside_same_frame_eval(&self) -> bool {
        self.current_span.is_some_and(|span| {
            self.same_frame_eval_spans
                .iter()
                .any(|(start, end)| span.start() >= *start && span.end() <= *end)
        })
    }
}
