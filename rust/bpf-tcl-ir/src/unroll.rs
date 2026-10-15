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

//! Bounded-loop unrolling.
//!
//! `loop N VAR { body }` is expanded *before* CFG construction into N copies of
//! the body, each preceded by `setint VAR <k>`. Unrolling keeps the program
//! loop-free (the eBPF verifier's signature constraint) and lets the existing
//! back-edge rejection keep guarding native `while`/`for`. Nested loops are
//! supported (the body is unrolled recursively); a `loop` nested inside an `if`
//! is left in place and rejected later by the typed lowering — loops must
//! appear at the statement level.

use tcl_compiler::{Script, Statement};
use tcl_lexer::Span;
use tcl_registry::registry::CommandRegistry;
use tcl_syntax::number::NumberSyntax;

use crate::diag::{BpfDiag, BpfError};
use crate::lower::parse_int;
pub use crate::source::OriginalBpfSource;
use crate::source::{selected_argument, selected_command, synthetic_operation_call};

/// Maximum iterations a single `loop` may unroll to.
const MAX_UNROLL: i64 = 64;

/// Expand every top-level `loop` in `script` into unrolled statements.
///
/// # Errors
/// Returns a [`BpfError`] for a malformed `loop` (bad arity, non-literal count,
/// or a count exceeding the unroll cap).
pub fn unroll_loops(
    script: &Script,
    registry: &CommandRegistry,
    source: &OriginalBpfSource,
) -> Result<Script, BpfError> {
    let mut out = Vec::with_capacity(script.statements.len());
    for stmt in &script.statements {
        let kind = match stmt {
            Statement::Call { .. } | Statement::Barrier { .. } => selected_command(stmt, registry)?
                .and_then(|spec| spec.bpf_op)
                .map(|op| op.kind),
            _ => None,
        };
        if let Statement::Call { span, .. } = stmt
            && kind == Some(tcl_registry::bpf_op::BpfOpKind::LoopMacro)
        {
            expand_loop(stmt, *span, registry, source, &mut out)?;
        } else {
            out.push(stmt.clone());
        }
    }
    Ok(Script::from_statements(out))
}

fn expand_loop(
    statement: &Statement,
    span: Span,
    registry: &CommandRegistry,
    source: &OriginalBpfSource,
    out: &mut Vec<Statement>,
) -> Result<(), BpfError> {
    let Statement::Call { args, .. } = statement else {
        unreachable!("selected loop call")
    };
    // loop N VAR { body }
    if args.len() != 3 {
        return Err(BpfError::new(
            BpfDiag::BadArity,
            span,
            "`loop` expects: loop N VAR { body }",
        ));
    }
    let count = selected_argument(statement, 0, registry)?
        .and_then(|value| parse_count(&value, registry.numbers()))
        .ok_or_else(|| {
            BpfError::new(
                BpfDiag::BadInt,
                span,
                "`loop` count must be a non-negative integer literal",
            )
        })?;
    if count > MAX_UNROLL {
        return Err(BpfError::new(
            BpfDiag::UnboundedLoop,
            span,
            format!("`loop` count {count} exceeds the v1 unroll cap of {MAX_UNROLL}"),
        ));
    }
    let var = selected_argument(statement, 1, registry)?.ok_or_else(|| {
        BpfError::new(
            BpfDiag::BadArity,
            span,
            "loop variable requires an original static name",
        )
    })?;
    let body_source = source
        .selected_body(statement, 2, registry)
        .ok_or_else(|| {
            BpfError::new(
                BpfDiag::BadArity,
                span,
                "loop body requires unchanged original literal source",
            )
        })?;
    let body_base = body_source
        .base()
        .checked_sub(source.base())
        .ok_or_else(|| {
            BpfError::new(
                BpfDiag::Internal,
                span,
                "loop body is outside its original source owner",
            )
        })?;
    let body = body_source
        .prepared_body(registry)
        .map_err(|error| error.offset(body_base))?;
    let mut body =
        unroll_loops(&body, registry, &body_source).map_err(|error| error.offset(body_base))?;
    tcl_compiler::rebase_script(&mut body, i64::from(body_base));

    for k in 0..count {
        out.push(synthetic_operation_call(
            tcl_registry::bpf_op::BpfOpKind::ScalarSet(tcl_registry::bpf_op::BpfScalarWidth::I64),
            vec![var.clone(), (k).to_string()],
            span,
            registry,
        )?);
        out.extend(body.statements.iter().cloned());
    }
    Ok(())
}

fn parse_count(s: &str, numbers: NumberSyntax) -> Option<i64> {
    let v = parse_int(s, numbers)?;
    (v >= 0).then_some(v)
}
