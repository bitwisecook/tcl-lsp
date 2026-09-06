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

//! Interleaved proc definition emission.
//!
//! Top-level `proc` definitions are emitted as a synthetic
//! `proc NAME PARAMS BODY` command call, interleaved with other
//! statements at their original source line.

#![allow(dead_code, clippy::doc_markdown)]

use std::collections::VecDeque;

use crate::ir::Procedure as IrProcedure;

use super::super::{CodegenCtx, Op, Operand};

/// Return `true` if the proc definition is purely literal (no `$` or
/// `[` substitutions in name, params, or body), so it can be emitted
/// at compile time rather than as a runtime command call.
#[must_use]
pub fn is_static_proc(p: &IrProcedure) -> bool {
    let body = p.body_source.as_deref().unwrap_or("");
    for s in [p.name.as_str(), p.params_raw.as_str(), body] {
        if s.contains('$') || s.contains('[') {
            return false;
        }
    }
    true
}

impl CodegenCtx<'_> {
    /// Emit a single proc definition call.
    ///
    /// tclsh emits each `proc` definition at top level as:
    /// ```text
    /// push "proc"
    /// push <name>
    /// push <params>
    /// push <body_source>
    /// invokeStk1 4
    /// pop
    /// ```
    pub fn emit_one_proc_def(&mut self, proc_def: &IrProcedure) {
        self.push_lit("proc");
        self.push_lit(&proc_def.name);
        // Wrap params and body in braces so the VM's push handler
        // strips them without performing variable/command substitution.
        let params = &proc_def.params_raw;
        let body = proc_def.body_source.as_deref().unwrap_or("");
        let wrapped_params = if params.is_empty() {
            String::new()
        } else {
            format!("{{{params}}}")
        };
        let wrapped_body = if body.is_empty() {
            String::new()
        } else {
            format!("{{{body}}}")
        };
        self.push_lit(&wrapped_params);
        self.push_lit(&wrapped_body);
        self.emit(Op::INVOKE_STK1, vec![Operand::Imm(4)]);
        self.emit(Op::POP, vec![]);
        self.cmd_index += 1;
        self.seen_generic_invoke = true;
    }

    /// Emit pending proc definitions whose source offset is before
    /// `before_offset`.
    ///
    /// `pending` is maintained in source-offset order; procs before
    /// the given offset are drained from the front. Using a
    /// [`VecDeque`] keeps `pop_front` O(1), so this is linear in the
    /// number of procs drained rather than quadratic.
    pub fn emit_pending_proc_defs(
        &mut self,
        pending: &mut VecDeque<IrProcedure>,
        before_offset: u32,
    ) {
        while pending
            .front()
            .is_some_and(|p| p.span.start() < before_offset)
        {
            // SAFETY: front() just succeeded, so pop_front() returns Some.
            let p = pending.pop_front().expect("front() was Some");
            self.emit_one_proc_def(&p);
        }
    }

    /// Flush remaining pending proc definitions in order.
    pub fn flush_proc_defs(&mut self, pending: &mut VecDeque<IrProcedure>) {
        while let Some(p) = pending.pop_front() {
            self.emit_one_proc_def(&p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodegenCtx;
    use crate::ir::{Procedure, Script};
    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;

    fn dummy_proc(name: &str, params: &str, body: &str) -> Procedure {
        Procedure {
            name: name.into(),
            qualified_name: format!("::{name}"),
            params: Vec::new(),
            span: Span::new(0, 0),
            body: Script::new(),
            params_raw: params.into(),
            body_source: Some(body.into()),
            body_offset: 0,
            namespace_scoped: false,
            base_priority: 500,
        }
    }

    #[test]
    fn is_static_proc_plain() {
        let p = dummy_proc("foo", "x y", "set x 1");
        assert!(is_static_proc(&p));
    }

    #[test]
    fn is_static_proc_with_subst() {
        let p = dummy_proc("foo", "x", "set x [pwd]");
        assert!(!is_static_proc(&p));
    }

    #[test]
    fn emit_one_proc_def_shape() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let p = dummy_proc("foo", "x y", "set x 1");
        ctx.emit_one_proc_def(&p);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // push "proc", push name, push params, push body, invokeStk1, pop
        assert_eq!(
            ops,
            vec![
                Op::PUSH1,
                Op::PUSH1,
                Op::PUSH1,
                Op::PUSH1,
                Op::INVOKE_STK1,
                Op::POP,
            ]
        );
        assert!(ctx.seen_generic_invoke);
    }
}
