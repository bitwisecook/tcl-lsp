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

//! The attach/deployment facet of the profile-based top layer.
//!
//! A top-level `attach KIND TARGET` declaration records *where* and *how* a
//! program is deployed (`attach xdp eth0`, `attach socket_filter lo`). The kind
//! resolves to an eBPF program type via the same table the event space uses, so
//! the attach is validated against the program(s) it would govern: it must
//! match at least one `when` block's type, and there is at most one `attach`
//! per file.
//!
//! The result is pure metadata hung off [`crate::ir::BpfProgramDecl::attach`]
//! for a future ELF/loader — codegen ignores it.

use crate::source::{OriginalBpfSource, source_declaration_kind};
use tcl_compiler::{Script, Statement};
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

use crate::diag::{BpfDiag, BpfError};
use crate::event::event_to_prog_type;
use crate::ir::{AttachSpec, BpfProgramDecl, ProgType};

/// Collect the (at most one) `attach KIND TARGET` declaration from a file's top
/// level.
///
/// # Errors
/// Missing original static operands, bad arity, more than one `attach`, or an
/// unknown attach kind.
pub fn collect_attach(
    top_level: &Script,
    registry: &CommandRegistry,
    source: &OriginalBpfSource,
) -> Result<Option<AttachSpec>, BpfError> {
    Ok(find_attach(top_level, registry, source)?.map(|(spec, _)| spec))
}

/// Resolve a file's `attach` onto the programs it governs: every program whose
/// type matches the attach kind gets the deployment target. With no `attach`
/// this is a no-op.
///
/// # Errors
/// A malformed/duplicate/unknown `attach`, or an `attach` that matches no
/// program's type (a deployment that could never apply).
pub fn resolve_attach(
    top_level: &Script,
    programs: &mut [BpfProgramDecl],
    registry: &CommandRegistry,
    source: &OriginalBpfSource,
) -> Result<(), BpfError> {
    let Some((spec, span)) = find_attach(top_level, registry, source)? else {
        return Ok(());
    };
    let want = event_to_prog_type(&spec.kind);
    let mut matched = 0usize;
    for decl in programs.iter_mut() {
        if want == Some(decl.program.prog_type) {
            decl.attach = Some(spec.clone());
            matched += 1;
        }
    }
    if matched == 0 {
        return Err(BpfError::new(
            BpfDiag::BadAttach,
            span,
            format!(
                "`attach {}` matches no program (no `when` block of that type)",
                spec.kind
            ),
        ));
    }
    Ok(())
}

/// The program type an attach kind resolves to (shares the event table, so
/// `xdp`/`socket_filter`/`socket` map exactly as the `when` events do).
#[must_use]
pub fn attach_prog_type(kind: &str) -> Option<ProgType> {
    event_to_prog_type(kind)
}

/// Scan the top level for the single `attach` declaration, returning its parsed
/// spec and declaration span.
fn find_attach(
    top_level: &Script,
    registry: &CommandRegistry,
    source: &OriginalBpfSource,
) -> Result<Option<(AttachSpec, Span)>, BpfError> {
    let mut found: Option<(AttachSpec, Span)> = None;
    for stmt in &top_level.statements {
        let Statement::Call {
            args, span, tokens, ..
        } = stmt
        else {
            continue;
        };
        if source_declaration_kind(stmt, registry)
            != Some(tcl_registry::bpf_op::BpfDeclKind::Attach)
        {
            continue;
        }
        if found.is_some() {
            return Err(BpfError::new(
                BpfDiag::BadAttach,
                *span,
                "at most one `attach` per file (v1)",
            ));
        }
        found = Some((parse_attach(args, tokens.as_ref(), source, *span)?, *span));
    }
    Ok(found)
}

fn parse_attach(
    args: &[String],
    tokens: Option<&tcl_compiler::ir::CommandTokens>,
    source: &OriginalBpfSource,
    span: Span,
) -> Result<AttachSpec, BpfError> {
    // attach KIND TARGET
    if args.len() != 2 {
        return Err(BpfError::new(
            BpfDiag::BadAttach,
            span,
            "`attach` expects: attach KIND TARGET",
        ));
    }
    let values = (0..args.len())
        .map(|index| {
            source.scalar(tokens, args, index).ok_or_else(|| {
                BpfError::new(
                    BpfDiag::BadAttach,
                    span,
                    "attach declarations require original static kind and target words",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let kind = &values[0];
    if attach_prog_type(kind).is_none() {
        return Err(BpfError::new(
            BpfDiag::BadAttach,
            span,
            format!("unknown attach kind `{kind}` (known: xdp, socket_filter, socket)"),
        ));
    }
    Ok(AttachSpec {
        kind: kind.clone(),
        target: values[1].clone(),
    })
}

#[cfg(test)]
mod original_operand_tests {
    use super::*;
    use tcl_compiler::ir::WordExpr;

    #[test]
    fn attach_selection_requires_the_actual_original_head_and_each_operand() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        for index in 0..3 {
            let mut module = crate::source::lower_bpf_source(
                if index == 0 {
                    "ignore xdp eth0"
                } else {
                    "attach xdp eth0"
                },
                registry,
            );
            let source = OriginalBpfSource::from_module(&module);
            if index != 0 {
                let spec = collect_attach(&module.top_level, registry, &source)
                    .unwrap()
                    .unwrap();
                assert_eq!(spec.kind, "xdp");
                assert_eq!(spec.target, "eth0");
            }
            let Statement::Call {
                command,
                args,
                tokens: Some(tokens),
                ..
            } = &mut module.top_level.statements[0]
            else {
                panic!("original attach command");
            };
            let replacement = match index {
                0 => "attach",
                1 => "socket",
                _ => "eth1",
            };
            let WordExpr::Literal { text, .. } = &mut tokens.word_exprs[index] else {
                panic!("original literal operand");
            };
            *text = replacement.into();
            tokens.argv_texts[index] = replacement.into();
            if index == 0 {
                *command = replacement.into();
            } else {
                args[index - 1] = replacement.into();
            }
            assert_eq!(
                collect_attach(&module.top_level, registry, &source)
                    .unwrap_err()
                    .code,
                BpfDiag::BadAttach
            );
        }
        let mut module = crate::source::lower_bpf_source("attach xdp eth0", registry);
        let source = OriginalBpfSource::from_module(&module);
        let Statement::Call { tokens, .. } = &mut module.top_level.statements[0] else {
            panic!("original attach command");
        };
        *tokens = None;
        assert_eq!(
            collect_attach(&module.top_level, registry, &source)
                .unwrap_err()
                .code,
            BpfDiag::BadAttach
        );
    }
}
