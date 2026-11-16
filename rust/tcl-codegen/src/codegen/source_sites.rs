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

//! The source the context emits from and the command boundary each instruction is stamped with.

use super::{CodegenCtx, Op, Operand, SourceCommandBoundary, namespace_path_for_binding};
use tcl_lexer::Span;

impl CodegenCtx<'_> {
    /// Set the module source text (see [`Self::source`]) so emitted instructions
    /// carry their command's surface text for `errorInfo`.
    pub fn set_source(&mut self, source: &str) {
        self.exact_command_source = None;
        let image = tcl_lexer::SourceImage::document(source);
        self.invalidate_unbound_original_lexical_words(&image);
        self.source = image;
        self.line_index = (!source.is_empty()).then(|| tcl_lexer::LineIndex::new(source));
    }

    /// Retain original native source bytes and channel before emitting spans.
    pub fn set_source_image(&mut self, source: tcl_lexer::SourceImage) {
        self.invalidate_unbound_original_lexical_words(&source);
        self.exact_command_source = None;
        self.line_index =
            (!source.is_empty()).then(|| tcl_lexer::LineIndex::from_bytes(source.bytes()));
        self.source = source;
    }

    /// Borrow the original source image for byte word and nested-body emission.
    #[must_use]
    pub fn source_image(&self) -> &tcl_lexer::SourceImage {
        &self.source
    }

    /// Check an unchanged Unicode advisory view without substituting a buffer.
    pub(crate) fn source_unicode(&self) -> Result<&str, std::str::Utf8Error> {
        self.source.try_text()
    }

    /// Install the module source and its already-built line index. The module
    /// emitter uses this path so procedure contexts share both allocations.
    pub(in super::super) fn set_indexed_source(
        &mut self,
        source: tcl_lexer::SourceImage,
        line_index: tcl_lexer::LineIndex,
    ) {
        self.invalidate_unbound_original_lexical_words(&source);
        self.exact_command_source = None;
        self.line_index = (!source.is_empty()).then_some(line_index);
        self.source = source;
    }

    /// Select the executable Tcl command whose source metadata subsequent
    /// instructions inherit.
    ///
    /// A source span and its command/control ownership are one piece of
    /// emitter state: updating only the span would retain the right line while
    /// silently dropping the command text needed by `errorInfo` and runtime
    /// command-table revalidation. Keep that invariant behind this method
    /// rather than exposing the two fields independently.
    pub fn set_command_source_span(&mut self, span: impl Into<Option<Span>>) {
        self.exact_command_source = None;
        self.current_span = span.into();
        self.current_span_is_command = true;
        self.current_command_namespace = self.resolution_namespace.path.clone();
        self.current_command_namespace_context = self.resolution_namespace_context();
    }

    /// Select compiler control which has a useful diagnostic span but is not
    /// itself a replayable Tcl command (for example, a CFG branch condition).
    pub(crate) fn set_control_source_span(&mut self, span: Option<Span>) {
        self.exact_command_source = None;
        self.current_span = span;
        self.current_span_is_command = false;
        self.current_command_namespace = tcl_runtime_api::ByteNamespacePath::root();
        self.current_command_namespace_context = None;
    }

    /// Clear source ownership before emitting compiler-generated block
    /// machinery. A later `START_CMD` remains a non-boundary until its exact
    /// owning Tcl command is supplied explicitly.
    pub(crate) fn clear_source_site(&mut self) {
        self.set_control_source_span(None);
    }

    /// Select the explicit structured-command owner for a synthetic runtime
    /// boundary. No site means the marker is compiler control only and must not
    /// be used for plain-dispatch replay.
    pub(crate) fn set_command_boundary_site(
        &mut self,
        site: Option<&tcl_compiler::ir::CommandBindingSite>,
    ) {
        self.clear_source_site();
        if let Some(site) = site {
            self.set_command_source_span(site.span);
            self.current_command_namespace = namespace_path_for_binding(&site.binding);
            self.current_command_namespace_context
                .clone_from(&site.binding.namespace_context);
            if !self.plain_command_dispatch {
                self.require_command_site(site);
            }
        }
    }

    /// Restamp an already-emitted synthetic `START_CMD` with its explicit
    /// structured owner without changing the source metadata of the wrapped
    /// clause instructions.
    pub(crate) fn stamp_command_boundary(
        &mut self,
        instruction: usize,
        site: Option<&tcl_compiler::ir::CommandBindingSite>,
    ) {
        let (span, text, line) =
            site.map_or((None, tcl_lexer::SourceImage::default(), 0), |site| {
                if !self.plain_command_dispatch {
                    // The owner's command, not the clause the marker wraps.
                    self.require_command_site(site);
                }
                (
                    Some(site.span),
                    self.source_text(site.span),
                    self.source_line(site.span),
                )
            });
        // The owner's evaluation, not that of the clause the marker was
        // emitted under: a directly evaluated `for` keeps its init compiled.
        let evaluation = if span.is_some_and(|span| self.evaluates_directly(span)) {
            tcl_runtime_api::CommandEvaluation::Direct
        } else {
            tcl_runtime_api::CommandEvaluation::Compiled
        };
        if let Some(instr) = self.instructions.get_mut(instruction) {
            instr.evaluation = evaluation;
            instr.source_span = span;
            instr.source_cmd_text = text;
            instr.source_line = line;
            instr.source_command_namespace =
                site.map_or_default(|site| namespace_path_for_binding(&site.binding));
            instr.source_command_namespace_context =
                site.and_then(|site| site.binding.namespace_context.clone());
            instr.source_command_boundary = site.is_some().into();
        }
    }

    /// Scope one inline command's diagnostic and replay source together.
    /// An absent module coordinate stays absent; its exact command bytes still
    /// belong to every emitted instruction. Nested commands restore this owner.
    pub(super) fn with_inline_command_source<T>(
        &mut self,
        span: Option<Span>,
        text: &str,
        emit: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous_span = self.current_span;
        let previous_command = self.current_span_is_command;
        let previous_exact = self.exact_command_source.take();
        let previous_namespace = self.current_command_namespace.clone();
        let previous_namespace_context = self.current_command_namespace_context.clone();
        self.set_command_source_span(span);
        self.exact_command_source = Some(tcl_lexer::SourceImage::from_bytes(
            text.as_bytes(),
            self.source.channel(),
        ));
        let result = emit(self);
        self.current_span = previous_span;
        self.current_span_is_command = previous_command;
        self.exact_command_source = previous_exact;
        self.current_command_namespace = previous_namespace;
        self.current_command_namespace_context = previous_namespace_context;
        result
    }

    /// Give every synthetic `START_CMD` just emitted for an inline body command
    /// its exact replay text, without overwriting a more deeply nested command
    /// boundary that was already restamped while those instructions were
    /// produced.
    ///
    /// The scoped command source supplies its module span and diagnostic text.
    /// This helper marks only its own replay points; deeper command owners keep
    /// their separate boundaries and source coordinates.
    pub(crate) fn restamp_emitted_inline_command_boundaries(
        &mut self,
        start: usize,
        text: &str,
        line: u32,
    ) {
        let enclosing_span = self.current_span;
        let enclosing_text = self.command_span_text();
        let enclosing_line = self.span_line();
        for instruction in self.instructions.iter_mut().skip(start) {
            if instruction.op == Op::START_CMD
                && instruction.source_span == enclosing_span
                && instruction.source_cmd_text == enclosing_text
                && instruction.source_line == enclosing_line
            {
                instruction.source_cmd_text =
                    tcl_lexer::SourceImage::from_bytes(text.as_bytes(), self.source.channel());
                instruction.source_line = line;
                // This START_CMD is a nested inline replay point, not the
                // boundary of the enclosing IR/source command used to locate
                // that outer command's continuation.
                instruction.source_command_boundary = SourceCommandBoundary::InlineReplay;
            } else if instruction.source_cmd_text == enclosing_text
                && instruction
                    .error_info_command
                    .as_ref()
                    .is_none_or(|command| !command.nested)
            {
                // errorInfo names the body command, which an enclosing
                // substitution's stamp then keeps.
                instruction.error_info_command = Some(tcl_bytecode::ErrorInfoCommand {
                    text: text.to_owned(),
                    line,
                    nested: true,
                    enclosing: Vec::new(),
                });
            }
        }
    }

    /// Begin an executable boundary for an inline command whose specialised
    /// instructions consume the invocation completely.
    ///
    /// Most inline command substitutions pass through `emit_cmd_word`, which
    /// already retains a nested `START_CMD`. A compile-time constant fold is
    /// different: it replaces the entire invocation with a literal push, so
    /// without this boundary a command-table mutation in an earlier argument
    /// of the same active command can leave the later fold executing stale
    /// semantics. The explicit source text lets the VM replay ordinary Tcl
    /// dispatch and resume after the stale folded instructions.
    ///
    /// This is a nested boundary, not a new source-command owner. Its absolute
    /// span/line continue to identify the enclosing command for diagnostics,
    /// while `source_cmd_text` is the exact bracket interior to replay.
    pub(crate) fn begin_consumed_inline_command(&mut self, text: &str) -> String {
        let end = self.fresh_label("inline_cmd_end");
        let start = self.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(end.clone()), Operand::Imm(1)],
            "",
        );
        if let Some(instruction) = self.instructions.get_mut(start) {
            instruction.source_cmd_text =
                tcl_lexer::SourceImage::from_bytes(text.as_bytes(), self.source.channel());
            instruction.source_command_boundary = SourceCommandBoundary::InlineReplay;
        }
        self.cmd_index += 1;
        end
    }
}
