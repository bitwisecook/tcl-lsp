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

//! BPF-profile-pinned Tcl source lowering.

use tcl_compiler::ir::{IfClause, WordExpr};
use tcl_compiler::lowering::{lower_to_ir_with_dialect, source_nesting_limit};
use tcl_compiler::{Module, Script, Statement, parse_expr_for_profile, rebase_script};
use tcl_lexer::{LexerConfig, Span};
use tcl_registry::registry::CommandRegistry;

/// Lower BPF Tcl source under the BPF profile for every top-level and nested
/// body re-segmentation.
pub(crate) fn lower_bpf_source(source: &str, registry: &CommandRegistry) -> Module {
    let mut module = lower_to_ir_with_dialect(
        source,
        registry,
        // The same profile the lowering is given, so the re-segmentation
        // grammar and the dialect axis can never disagree.
        LexerConfig::for_profile(registry.profile()),
        // The BPF profile as the registry itself resolved it — this crate
        // deliberately has no compile-time path to `tcl-dialect`, and reaches a
        // profile only through `CommandRegistry::profile()`.
        registry.profile(),
    );
    let config = module.native_lexer_config();
    expand_conditionals(&mut module.top_level, source, registry, config);
    module
}

/// BPF is a closed static language: unknown commands, command factories and
/// replacements are rejected by its frontend. Its authored control syntax is
/// consequently selected independently of Tcl's live execution admission.
fn expand_conditionals(
    root: &mut Script,
    source: &str,
    registry: &CommandRegistry,
    config: LexerConfig,
) {
    let mut pending = vec![(root, 0)];
    while let Some((script, depth)) = pending.pop() {
        if source_nesting_limit().exceeded(depth) {
            continue;
        }
        for statement in &mut script.statements {
            if let Some(conditional) = conditional_statement(statement, source, registry, config) {
                *statement = conditional;
            }
            pending.extend(
                statement
                    .child_scripts_mut()
                    .into_iter()
                    .map(|body| (body, depth + 1)),
            );
        }
    }
}

fn conditional_statement(
    statement: &Statement,
    source: &str,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<Statement> {
    let (args, span, tokens) = match statement {
        Statement::Call {
            args,
            span,
            tokens: Some(tokens),
            ..
        }
        | Statement::Barrier {
            args,
            span,
            tokens: Some(tokens),
            ..
        } => (args, span, tokens),
        _ => return None,
    };
    let WordExpr::Literal { text: head, .. } = tokens.word_exprs.first()? else {
        return None;
    };
    if tokens.word_exprs.len() != args.len() + 1
        || tokens
            .expand_word
            .as_ref()
            .is_some_and(|words| words.iter().any(|expanded| *expanded))
    {
        return None;
    }
    let words = args.iter().map(String::as_str).collect::<Vec<_>>();
    let plan = registry.bpf_conditional_operands(head, &words)?;
    // Require every original word to be literal, including the noise words.
    for (word, value) in tokens.word_exprs[1..].iter().zip(args) {
        original_literal(word, value, source)?;
    }
    let mut clauses = Vec::new();
    let mut else_body = None;
    let mut else_span = None;
    for (condition, body_index) in plan {
        let body_word = tokens.word_exprs.get(body_index + 1)?;
        let (body_text, body_base, body_span) =
            original_literal(body_word, &args[body_index], source)?;
        let mut body =
            lower_to_ir_with_dialect(body_text, registry, config, registry.profile()).top_level;
        discard_execution_proofs(&mut body);
        rebase_script(&mut body, i64::from(body_base));
        if let Some(condition_index) = condition {
            let (text, base, condition_span) = original_literal(
                tokens.word_exprs.get(condition_index + 1)?,
                &args[condition_index],
                source,
            )?;
            clauses.push(IfClause {
                condition: parse_expr_for_profile(text, registry.profile()),
                condition_span,
                condition_base: Some(base),
                body,
                body_span,
            });
        } else {
            else_body = Some(body);
            else_span = Some(body_span);
        }
    }
    Some(Statement::If {
        span: *span,
        clauses,
        else_body,
        else_span,
    })
}

/// Validate the affine mapping against the actual source bytes; decoded or
/// reconstructed text cannot receive an invented condition/body base.
fn original_literal<'a>(
    word: &WordExpr,
    value: &str,
    source: &'a str,
) -> Option<(&'a str, u32, Span)> {
    let (text, site, prefix) = match word {
        WordExpr::Literal { text, source } => (text, source, 0),
        WordExpr::BracedLiteral { text, source } => (text, source, 1),
        _ => return None,
    };
    if text != value {
        return None;
    }
    let base = site.span.start().checked_add(prefix)?;
    let end = base.checked_add(u32::try_from(text.len()).ok()?)?;
    let original = source.get(base as usize..end as usize)?;
    (original == text).then_some((original, base, Span::new(base, end)))
}

/// Independently lowered body receipts belong to their original Tcl entry.
/// Keep their syntax and spans, never import those execution permissions into
/// a BPF-derived control region.
fn discard_execution_proofs(script: &mut Script) {
    tcl_compiler::ir::for_each_script_mut(script, &mut |script| {
        script.executed_source = None;
        script.implicit_math_invocations.clear();
        script.expression_preparations.clear();
        script.command_binding_sites = Default::default();
        script.procedure_binding_requirements = Default::default();
        script.native_compilation_failure = None;
        script.native_compilation_admission = None;
        for statement in &mut script.statements {
            if let Some(tokens) = statement.tokens_mut() {
                tokens.source_binding = None;
                tokens.nested_bindings.clear();
                tokens.variable_accesses.clear();
                tokens.evaluated_body = None;
            }
        }
    });
}
