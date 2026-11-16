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

//! Catch/try/error handling control flow emission.
//!
//! Extends [`CodegenCtx`] with methods for emitting `beginCatch4`/`endCatch`
//! bytecodes for `catch` and `try` commands.

use tcl_lexer::Span;

use tcl_compiler::expr_ast::{BinOp, ExprNode};

use super::values::is_qualified;
use super::{CodegenCtx, Op};

mod catch_command;
mod dict_commands;
mod release_compiles;
mod try_command;

// Constant expression error detection

/// Detect compile-time expression errors (e.g. divide by zero).
///
/// Returns `(error_message, error_options_string)` when the AST is a
/// fully constant expression that would raise a known error at runtime.
#[must_use]
pub fn detect_const_expr_error(node: &ExprNode) -> Option<(String, String)> {
    if let ExprNode::Binary { op, left, right } = node
        && matches!(op, BinOp::Div | BinOp::Mod)
        && let (ExprNode::Literal { .. }, ExprNode::Literal { text: rv, .. }) =
            (left.as_ref(), right.as_ref())
        && rv == "0"
    {
        return Some((
            "divide by zero".to_owned(),
            "-code 1 -level 0 -errorcode {ARITH DIVZERO {divide by zero}}".to_owned(),
        ));
    }
    None
}

#[derive(Clone, Copy)]
enum InlineBodyEmitter {
    Catch,
    TryHandler,
}

#[derive(Debug, Clone)]
struct TryHandlerBindings {
    body_index: usize,
    handler_body_index: usize,
    result_var: Option<String>,
    options_var: Option<String>,
}

/// Whether a value already resolved by Tcl's word/list grammar can safely use
/// the procedure-local scalar bytecodes emitted by the narrow catch/try
/// specialisers. Other valid Tcl variable names (qualified names, arrays, and
/// names requiring quoting) stay on the generic runtime path.
fn is_inline_local_scalar_name(name: &str) -> bool {
    tcl_syntax::naming::is_static_variable_source_word(name) && !is_qualified(name)
}

/// Whether the release's compiler binds the variable name `name` to a
/// procedure-local scalar (`TclLocalScalar`): no namespace separator anywhere
/// and no array element.
fn is_release_local_scalar_name(name: &str) -> bool {
    !tcl_syntax::naming::is_qualified(name.as_bytes()) && !super::values::is_array_ref(name)
}

/// One word of a command as the release's compiler sees it.
#[derive(Clone, Copy)]
struct ReleaseWord<'a> {
    text: &'a str,
    braced: bool,
}

impl ReleaseWord<'_> {
    /// Known at compile time (`TclWordKnownAtCompileTime`): no variable or
    /// command substitution.
    fn known(self) -> bool {
        !tcl_syntax::naming::word_is_dynamic(self.text, self.braced)
    }

    /// One text token (`TCL_TOKEN_SIMPLE_WORD`): known, and no backslash
    /// substitution either.
    fn simple(self) -> bool {
        self.known() && (self.braced || !self.text.contains('\\'))
    }
}

/// Parse a `dict for`/`dict map` variable-list word by the Tcl list grammar
/// and return the two loop-variable names when it is exactly a two-element
/// list of *plain* names — not qualified (`::`), not an array element (`(`),
/// and not needing list quoting (whitespace/empty). Returns `None` (inline
/// emitter bails to the runtime invoke) otherwise, so a 1-element list like
/// `{{a b}}` errors at runtime instead of being miscompiled as two vars, and a
/// spaced name like `{{a b} v}` keeps its runtime semantics.
fn is_two_plain_names(vars_text: &str) -> Option<[String; 2]> {
    let elems = super::helpers::split_list_simple(vars_text);
    let [a, b] = <[String; 2]>::try_from(elems).ok()?;
    for v in [&a, &b] {
        if v.is_empty() || is_qualified(v) || v.contains('(') || v.chars().any(char::is_whitespace)
        {
            return None;
        }
    }
    Some([a, b])
}

/// Normalise a parsed `return -code C -level L` into the `(code, level)`
/// immediates `returnImm` carries, exactly as C's `TclMergeReturnOptions`
/// hands them to `CompileReturnInternal`.
///
/// Two rules do all the work (`tclResult.c:866-974`):
///
/// 1. `-level` defaults to **1**, not 0 — a plain `return` is `(0, 1)`, and
///    `(0, 0)` means something else entirely (`TclProcessReturn` at level 0
///    with code OK falls through to the next instruction).
/// 2. `-code return` is rewritten to `-code ok -level L+1`, so `TCL_RETURN`
///    never reaches the operand: `return -code return` is `(0, 2)`, and
///    `return -code return -level 3` is `(0, 4)`.
///
/// Every other code rides its own operand with the level untouched:
/// `-code error` → `(1, 1)`, `-code break` → `(3, 1)`, `-code continue` →
/// `(4, 1)`, `-code N` → `(N, 1)`.
fn merge_return_operands(code: Option<i32>, level: Option<i32>) -> (i32, i32) {
    let mut ret_code = code.unwrap_or(0);
    let mut ret_level = level.unwrap_or(1);
    if ret_code == 2 {
        ret_level = ret_level.saturating_add(1);
        ret_code = 0;
    }
    (ret_code, ret_level)
}

impl CodegenCtx<'_> {
    /// Emit one complete Tcl body without losing its command boundaries.
    ///
    /// Catch, try, and dictionary bodies share this owner. Each complete
    /// command retains its source binding and uses the existing command emitter.
    /// A malformed tail emits the same parse failure as native bracket scripts
    /// after the complete prefix, inside the surrounding live exception range.
    fn emit_segmented_inline_body(&mut self, body: &str, emitter: InlineBodyEmitter) {
        let config = self.lexer_config().nested();
        let Ok(segmented) = tcl_compiler::lowering::command_at_time_script_image(
            &tcl_lexer::SourceImage::native(body.as_bytes()),
            config,
        ) else {
            self.refuse_native_dependency();
            self.push_lit_bytes_exact(b"");
            return;
        };
        if segmented.commands.is_empty() && segmented.fatal_tail.is_none() {
            self.push_lit("");
            return;
        }

        let last_complete = segmented.commands.len().saturating_sub(1);
        for (index, command) in segmented.commands.iter().enumerate() {
            let execution_span = command.execution_span(body);
            let command_text = body
                .get(execution_span.start() as usize..execution_span.end() as usize)
                .expect("execution spans index their source body");
            let emitted_start = self.instructions.len();
            let tokens = self.inline_body_command_tokens(command_text, command.span.start());
            let source_span = self.inline_body_source_base.and_then(|base| {
                let span = Span::new(
                    base.checked_add(execution_span.start())?,
                    base.checked_add(execution_span.end())?,
                );
                (self.source.get(span.as_range()) == Some(command_text.as_bytes())).then_some(span)
            });
            self.with_inline_command_source(source_span, command_text, |ctx| {
                ctx.with_invocation_tokens(tokens.as_ref(), |ctx| match emitter {
                    InlineBodyEmitter::Catch => ctx.emit_catch_body(command_text),
                    InlineBodyEmitter::TryHandler => ctx.emit_try_handler_command(command_text),
                });
                ctx.restamp_emitted_inline_command_boundaries(
                    emitted_start,
                    command_text,
                    ctx.span_line(),
                );
            });
            if segmented.fatal_tail.is_some() || index != last_complete {
                self.emit(Op::POP, vec![]);
            }
        }

        if let Some(tail) = segmented.fatal_tail {
            self.emit_script_parse_error(tail.cut.message.as_bytes());
        }
    }

    /// Preserve the authored coordinate only for the exact literal body.
    fn authored_inline_body_base(&self, body: &str, operand: usize) -> Option<u32> {
        let word = self
            .invocation_tokens
            .as_deref()?
            .words()
            .get(self.original_hook_argument(operand)?.checked_add(1)?)?;
        if !matches!(word, tcl_compiler::ir::WordExpr::BracedLiteral { .. }) {
            return None;
        }
        tcl_compiler::command_binding::ExecutedScriptSource::literal_word_base(
            self.source_unicode().ok()?,
            word,
            body,
            self.lexer_config(),
        )
    }

    /// Scope a literal script operand's own source coordinates. In particular,
    /// a try handler cannot inherit the enclosing catch body's first byte.
    fn with_inline_body_operand<T>(
        &mut self,
        body: &str,
        operand: usize,
        emit: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = self.inline_body_source_base;
        let previous_context = self.native_compilation;
        self.inline_body_source_base = self.authored_inline_body_base(body, operand);
        let context = self.invocation_tokens.as_deref().and_then(|tokens| {
            tcl_compiler::registry_invocation::proved_native_inline_body_context(
                self.registry,
                tokens,
                previous_context,
                self.original_hook_argument(operand)?,
            )
        });
        if let Some(context) = context {
            self.native_compilation = context;
        } else if self.invocation_tokens.as_deref().is_some_and(|tokens| {
            tcl_compiler::registry_invocation::proved_native_admitted_inline_operation(tokens)
                .is_some()
        }) {
            self.refuse_native_dependency();
        }
        let result = emit(self);
        self.inline_body_source_base = previous;
        self.native_compilation = previous_context;
        result
    }

    /// Each body command receives its own retained proof. An authored parent
    /// cannot supply its dispatch proof to a child with a missing source site.
    fn inline_body_command_tokens(
        &self,
        command: &str,
        relative_offset: u32,
    ) -> Option<tcl_compiler::ir::CommandTokens> {
        let offset = self
            .inline_body_source_base
            .and_then(|base| base.checked_add(relative_offset));
        if let Some(tokens) =
            offset.and_then(|offset| self.source_proofs.as_ref()?.tokens.get(&offset))
        {
            return Some(tokens.clone());
        }
        if let Some(offset) = offset {
            let config = self.lexer_config().nested();
            let segments = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
                command, offset, config,
            );
            if let [segment] = segments.as_slice()
                && !segment.is_partial
            {
                let source = tcl_lexer::SourceMap::new(command).with_base(offset, 0, 0);
                let mut tokens =
                    tcl_compiler::ir::CommandTokens::from_segmented(&source, config, segment);
                if let Some(parent) = self.invocation_tokens.as_deref() {
                    tokens.inherit_nested_bindings(parent);
                }
                return Some(tokens);
            }
        }
        let mut tokens = self.original_inline_syntax(command)?;
        if self
            .invocation_tokens
            .as_deref()
            .is_some_and(|parent| parent.source_binding.is_some())
        {
            tokens.source_binding =
                Some(tcl_compiler::command_binding::SourceInvocationBinding::unknown());
        }
        Some(tokens)
    }
}

// Tests

#[cfg(test)]
mod tests;
