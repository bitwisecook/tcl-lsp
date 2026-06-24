//! The attach/deployment facet of the profile-based top layer.
//!
//! A top-level `attach KIND TARGET` declaration records *where* and *how* a
//! program is deployed (`attach xdp eth0`, `attach socket_filter lo`). The kind
//! resolves to an eBPF program type via the same table the event space uses, so
//! the attach is validated against the program(s) it would govern: it must
//! match at least one `when` block's type, and there is at most one `attach`
//! per file (v1).
//!
//! The result is pure metadata hung off [`crate::ir::BpfProgramDecl::attach`]
//! for a future ELF/loader — codegen ignores it.

use tcl_compiler::{Script, Statement};
use tcl_lexer::Span;

use crate::diag::{BpfDiag, BpfError};
use crate::event::event_to_prog_type;
use crate::ir::{AttachSpec, BpfProgramDecl, ProgType};

/// Collect the (at most one) `attach KIND TARGET` declaration from a file's top
/// level.
///
/// # Errors
/// Bad arity, more than one `attach`, or an unknown attach kind.
pub fn collect_attach(top_level: &Script) -> Result<Option<AttachSpec>, BpfError> {
    Ok(find_attach(top_level)?.map(|(spec, _)| spec))
}

/// Resolve a file's `attach` onto the programs it governs: every program whose
/// type matches the attach kind gets the deployment target. With no `attach`
/// this is a no-op.
///
/// # Errors
/// A malformed/duplicate/unknown `attach`, or an `attach` that matches no
/// program's type (a deployment that could never apply).
pub fn resolve_attach(top_level: &Script, programs: &mut [BpfProgramDecl]) -> Result<(), BpfError> {
    let Some((spec, span)) = find_attach(top_level)? else {
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
fn find_attach(top_level: &Script) -> Result<Option<(AttachSpec, Span)>, BpfError> {
    let mut found: Option<(AttachSpec, Span)> = None;
    for stmt in &top_level.statements {
        let Statement::Call {
            command,
            canonical_command,
            args,
            span,
            ..
        } = stmt
        else {
            continue;
        };
        if canonical_command.as_deref().unwrap_or(command.as_str()) != "attach" {
            continue;
        }
        if found.is_some() {
            return Err(BpfError::new(
                BpfDiag::BadAttach,
                *span,
                "at most one `attach` per file (v1)",
            ));
        }
        found = Some((parse_attach(args, *span)?, *span));
    }
    Ok(found)
}

fn parse_attach(args: &[String], span: Span) -> Result<AttachSpec, BpfError> {
    // attach KIND TARGET
    if args.len() != 2 {
        return Err(BpfError::new(
            BpfDiag::BadAttach,
            span,
            "`attach` expects: attach KIND TARGET",
        ));
    }
    let kind = &args[0];
    if attach_prog_type(kind).is_none() {
        return Err(BpfError::new(
            BpfDiag::BadAttach,
            span,
            format!("unknown attach kind `{kind}` (known: xdp, socket_filter, socket)"),
        ));
    }
    Ok(AttachSpec {
        kind: kind.clone(),
        target: args[1].clone(),
    })
}
