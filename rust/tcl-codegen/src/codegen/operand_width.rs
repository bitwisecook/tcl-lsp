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

//! Operand widths of the target release: Tcl 9.1 compiles only the four-byte
//! operand forms.

use super::CodegenCtx;

impl CodegenCtx<'_> {
    /// The release this compile targets: the live entry's, else the profile's.
    pub(super) fn target_version(&self) -> Option<tcl_dialect::TclVersion> {
        self.native_entry
            .and_then(|entry| entry.execution_point?.tcl_version())
            .or_else(|| {
                self.dialect
                    .and_then(tcl_dialect::DialectProfile::runtime_version)
            })
    }

    /// Whether the target release compiles only four-byte operand forms.
    pub(super) fn emits_wide_instructions_only(&self) -> bool {
        self.target_version()
            .is_some_and(tcl_dialect::TclVersion::emits_wide_instructions_only)
    }

    /// Give every instruction the wide form the target release compiles in
    /// its place. Runs after jump shortening, so every jump stays four-byte.
    pub(super) fn widen_operands(&mut self) {
        if !self.emits_wide_instructions_only() {
            return;
        }
        for instruction in &mut self.instructions {
            if let Some(wide) = instruction.op.wide_form() {
                instruction.op = wide;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Op;
    use super::*;
    use tcl_bytecode::Operand;

    #[test]
    fn only_a_wide_release_widens() {
        let registry = tcl_registry::CommandRegistry::build_default();
        for (profile, op) in [
            ("tcl9.0", Op::INCR_SCALAR1_IMM),
            ("tcl9.1", Op::INCR_SCALAR4_IMM),
        ] {
            let mut ctx = CodegenCtx::new(false, &[], &registry);
            ctx.dialect = tcl_dialect::DialectProfile::find(profile);
            ctx.emit(Op::INCR_SCALAR1_IMM, vec![Operand::Imm(0), Operand::Imm(1)]);
            ctx.widen_operands();
            assert_eq!(ctx.instructions[0].op, op, "{profile}");
        }
    }
}
