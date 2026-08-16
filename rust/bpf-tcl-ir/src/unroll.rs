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
//! is left in place and rejected later by the typed lowering (a v1 limitation —
//! loops must appear at the statement level).

use tcl_compiler::{Script, Statement};
use tcl_lexer::Span;
use tcl_registry::registry::CommandRegistry;
use tcl_syntax::number::NumberSyntax;

use crate::diag::{BpfDiag, BpfError};
use crate::lower::parse_int;
use crate::source::lower_bpf_source;

/// Maximum iterations a single `loop` may unroll to in v1.
const MAX_UNROLL: i64 = 64;

/// Expand every top-level `loop` in `script` into unrolled statements.
///
/// # Errors
/// Returns a [`BpfError`] for a malformed `loop` (bad arity, non-literal count,
/// or a count exceeding the unroll cap).
pub fn unroll_loops(script: &Script, registry: &CommandRegistry) -> Result<Script, BpfError> {
    let mut out = Vec::with_capacity(script.statements.len());
    for stmt in &script.statements {
        if let Statement::Call {
            command,
            canonical_command,
            args,
            span,
            ..
        } = stmt
            && canonical_command.as_deref().unwrap_or(command.as_str()) == "loop"
        {
            expand_loop(args, *span, registry, &mut out)?;
        } else {
            out.push(stmt.clone());
        }
    }
    Ok(Script::from_statements(out))
}

fn expand_loop(
    args: &[String],
    span: Span,
    registry: &CommandRegistry,
    out: &mut Vec<Statement>,
) -> Result<(), BpfError> {
    // loop N VAR { body }
    if args.len() != 3 {
        return Err(BpfError::new(
            BpfDiag::BadArity,
            span,
            "`loop` expects: loop N VAR { body }",
        ));
    }
    let count = parse_count(&args[0], registry.numbers()).ok_or_else(|| {
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
    let var = &args[1];
    let body_text = &args[2];

    // Lower the loop body, then recursively unroll any loops inside it.
    let body_module = lower_bpf_source(body_text, registry);
    let body = unroll_loops(&body_module.top_level, registry)?;

    for k in 0..count {
        out.push(synth_setint(var, k, span));
        out.extend(body.statements.iter().cloned());
    }
    Ok(())
}

/// Build a synthetic `setint VAR <k>` IR call (the loop-variable binding).
fn synth_setint(var: &str, k: i64, span: Span) -> Statement {
    Statement::Call {
        span,
        command: "setint".to_owned(),
        canonical_command: None,
        args: vec![var.to_owned(), k.to_string()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

fn parse_count(s: &str, numbers: NumberSyntax) -> Option<i64> {
    let v = parse_int(s, numbers)?;
    (v >= 0).then_some(v)
}
