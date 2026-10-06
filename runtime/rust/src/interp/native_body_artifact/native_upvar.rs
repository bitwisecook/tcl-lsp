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

//! Ordinary UPVAR keeps its level header while each original target executes.

use super::*;
use tcl_registry::native_upvar_compilation::NativeUpvarInstruction;

pub(super) struct UpvarOperation {
    level: NamespaceOperand,
    bindings: Vec<(NamespaceOperand, usize)>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
    empty: usize,
}

impl Builder<'_> {
    pub(super) fn upvar_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeUpvarInstruction,
        depth: u32,
    ) -> Result<UpvarOperation, ValueError> {
        let mut prepared_words = HashMap::new();
        let level = match recipe.level {
            Some(level) => self.namespace_operand(words, &level, &mut prepared_words, depth)?,
            None => NamespaceOperand::Literal(self.literals.intern_bytes(b"1")),
        };
        let mut bindings = Vec::new();
        for binding in recipe.bindings {
            let other =
                self.namespace_operand(words, &binding.other, &mut prepared_words, depth)?;
            let slot = self
                .local(&binding.local, None)
                .ok_or_else(|| unavailable("original upvar local declaration"))?;
            bindings.push((other, slot));
        }
        Ok(UpvarOperation {
            level,
            bindings,
            prepared_words,
            empty: self.literals.intern_bytes(b""),
        })
    }
}

impl Interp {
    pub(super) fn execute_body_upvar(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        upvar: &UpvarOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let level = self.body_namespace_operand(artifact, command, &upvar.level, execution)?;
        if execution.done {
            return Ok(Code::Ok);
        }
        for (other, slot) in &upvar.bindings {
            let other = self.body_namespace_operand(artifact, command, other, execution)?;
            if execution.done {
                return Ok(Code::Ok);
            }
            let (_, target) = crate::cmd_eval::select_frame(
                self,
                &[level.as_ptr()],
                tcl_registry::FrameEffectSpec::UPVAR,
            )?;
            let code = self.link_original_compiled_upvar(other.as_ptr(), target, *slot);
            if code != Code::Ok || self.host_refusal_pending() {
                return Ok(code);
            }
        }
        drop(level);
        self.set_result(
            artifact
                .literals
                .original(upvar.empty)
                .expect("upvar empty PUSH"),
        );
        Ok(Code::Ok)
    }
}
