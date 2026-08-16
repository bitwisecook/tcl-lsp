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

//! The template/macro facet of the profile-based top layer.
//!
//! `template NAME {params} {body}` declares a reusable parameterised handler;
//! `use NAME k=v …` expands it inline — binding each parameter to an integer
//! via a synthetic `setint`, then splicing the (already-lowered) body. This is
//! the same IR-splice macro pattern [`crate::unroll::unroll_loops`] uses for
//! `loop`, and runs *before* unrolling and field expansion so a template body
//! may itself contain loops and profile field commands.
//!
//! Templates share the handler's local namespace (no hygiene in v1): a param
//! `key` becomes a local `key`, so a handler local of the same name is
//! overwritten by the expansion.

use tcl_compiler::ir::IfClause;
use tcl_compiler::{Script, Statement};
use tcl_lexer::Span;
use tcl_registry::registry::CommandRegistry;
use tcl_syntax::number::NumberSyntax;

use crate::diag::{BpfDiag, BpfError};
use crate::lower::parse_int;
use crate::source::lower_bpf_source;

/// The deepest a `use` may nest (one template using another), a guard against
/// mutually-recursive templates expanding forever.
const MAX_DEPTH: usize = 16;

/// A parameterised macro: a name, its ordered parameter names, and the lowered
/// body to splice at each `use` site.
#[derive(Debug, Clone)]
pub struct TemplateDef {
    /// Template name (the first word of `use`).
    pub name: String,
    /// Ordered parameter names, each bound by a `key=value` at the `use` site.
    pub params: Vec<String>,
    /// The lowered template body.
    pub body: Script,
}

/// Collect every top-level `template NAME {params} {body}` declaration.
///
/// # Errors
/// Bad arity or a duplicate template name.
pub fn collect_templates(
    top_level: &Script,
    registry: &CommandRegistry,
) -> Result<Vec<TemplateDef>, BpfError> {
    let mut out: Vec<TemplateDef> = Vec::new();
    for stmt in &top_level.statements {
        if let Statement::Call {
            command,
            canonical_command,
            args,
            span,
            ..
        } = stmt
            && canonical_command.as_deref().unwrap_or(command.as_str()) == "template"
        {
            let def = parse_template(args, *span, registry)?;
            if out.iter().any(|t| t.name == def.name) {
                return Err(BpfError::new(
                    BpfDiag::BadTemplate,
                    *span,
                    format!("duplicate template `{}`", def.name),
                ));
            }
            out.push(def);
        }
    }
    Ok(out)
}

fn parse_template(
    args: &[String],
    span: Span,
    registry: &CommandRegistry,
) -> Result<TemplateDef, BpfError> {
    // template NAME {params} {body}
    if args.len() != 3 {
        return Err(BpfError::new(
            BpfDiag::BadTemplate,
            span,
            "`template` expects: template NAME { params } { body }",
        ));
    }
    let params = args[1].split_whitespace().map(str::to_owned).collect();
    let body = lower_bpf_source(&args[2], registry).top_level;
    Ok(TemplateDef {
        name: args[0].clone(),
        params,
        body,
    })
}

/// Expand every `use NAME k=v …` in `script` by splicing the named template,
/// recursing into `if` bodies and nested `use` sites.
///
/// # Errors
/// Unknown template, missing/extra binding, a non-integer binding value, or
/// recursion past [`MAX_DEPTH`].
pub fn expand_uses(
    script: &Script,
    templates: &[TemplateDef],
    numbers: NumberSyntax,
) -> Result<Script, BpfError> {
    expand_script(script, templates, numbers, 0)
}

fn expand_script(
    script: &Script,
    templates: &[TemplateDef],
    numbers: NumberSyntax,
    depth: usize,
) -> Result<Script, BpfError> {
    let mut out = Vec::with_capacity(script.statements.len());
    for stmt in &script.statements {
        match stmt {
            Statement::Call {
                command,
                canonical_command,
                args,
                span,
                ..
            } if canonical_command.as_deref().unwrap_or(command.as_str()) == "use" => {
                expand_use(args, *span, templates, numbers, depth, &mut out)?;
            }
            Statement::If {
                span,
                clauses,
                else_body,
                else_span,
            } => {
                out.push(expand_if(
                    *span,
                    clauses,
                    else_body.as_ref(),
                    *else_span,
                    templates,
                    numbers,
                    depth,
                )?);
            }
            other => out.push(other.clone()),
        }
    }
    Ok(Script::from_statements(out))
}

fn expand_if(
    span: Span,
    clauses: &[IfClause],
    else_body: Option<&Script>,
    else_span: Option<Span>,
    templates: &[TemplateDef],
    numbers: NumberSyntax,
    depth: usize,
) -> Result<Statement, BpfError> {
    let clauses = clauses
        .iter()
        .map(|c| {
            Ok(IfClause {
                condition: c.condition.clone(),
                condition_span: c.condition_span,
                condition_base: c.condition_base,
                body: expand_script(&c.body, templates, numbers, depth)?,
                body_span: c.body_span,
            })
        })
        .collect::<Result<Vec<_>, BpfError>>()?;
    let else_body = else_body
        .map(|b| expand_script(b, templates, numbers, depth))
        .transpose()?;
    Ok(Statement::If {
        span,
        clauses,
        else_body,
        else_span,
    })
}

fn expand_use(
    args: &[String],
    span: Span,
    templates: &[TemplateDef],
    numbers: NumberSyntax,
    depth: usize,
    out: &mut Vec<Statement>,
) -> Result<(), BpfError> {
    if depth >= MAX_DEPTH {
        return Err(BpfError::new(
            BpfDiag::BadTemplate,
            span,
            "template expansion too deep (recursive `use`?)",
        ));
    }
    let Some((name, bindings)) = args.split_first() else {
        return Err(BpfError::new(
            BpfDiag::BadTemplate,
            span,
            "`use` expects: use NAME ?param=value …?",
        ));
    };
    let tmpl = templates.iter().find(|t| &t.name == name).ok_or_else(|| {
        BpfError::new(
            BpfDiag::BadTemplate,
            span,
            format!("unknown template `{name}`"),
        )
    })?;

    let pairs = parse_bindings(bindings, span, numbers)?;
    // Every declared parameter must be bound; bind it via a synthetic `setint`.
    for p in &tmpl.params {
        let (_, val) = pairs.iter().find(|(k, _)| k == p).ok_or_else(|| {
            BpfError::new(
                BpfDiag::BadTemplate,
                span,
                format!("template `{name}` is missing a binding for `{p}`"),
            )
        })?;
        out.push(synth_setint(p, *val, span));
    }
    // Reject bindings that don't name a parameter.
    if let Some((k, _)) = pairs.iter().find(|(k, _)| !tmpl.params.contains(k)) {
        return Err(BpfError::new(
            BpfDiag::BadTemplate,
            span,
            format!("template `{name}` has no parameter `{k}`"),
        ));
    }

    // Splice the body, expanding any nested `use` one level deeper.
    let expanded = expand_script(&tmpl.body, templates, numbers, depth + 1)?;
    out.extend(expanded.statements);
    Ok(())
}

/// Parse `key=value` words into `(key, value)` pairs; the value must be an
/// integer literal (the DSL's only binding type in v1).
fn parse_bindings(
    words: &[String],
    span: Span,
    numbers: NumberSyntax,
) -> Result<Vec<(String, i64)>, BpfError> {
    let mut pairs = Vec::with_capacity(words.len());
    for w in words {
        let (k, v) = w.split_once('=').ok_or_else(|| {
            BpfError::new(
                BpfDiag::BadTemplate,
                span,
                format!("`use` binding `{w}` must be key=value"),
            )
        })?;
        let n = parse_int(v, numbers).ok_or_else(|| {
            BpfError::new(
                BpfDiag::BadTemplate,
                span,
                format!("`use` binding `{k}` must be an integer literal"),
            )
        })?;
        pairs.push((k.to_owned(), n));
    }
    Ok(pairs)
}

/// Build a synthetic `setint VAR <n>` IR call (a parameter binding).
fn synth_setint(var: &str, n: i64, span: Span) -> Statement {
    Statement::Call {
        span,
        command: "setint".to_owned(),
        canonical_command: None,
        args: vec![var.to_owned(), n.to_string()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}
