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

//! Lowering scripts and the bodies commands evaluate: segmented scripts, original bodies, static `eval` and `uplevel`, and bodies kept as their own unit.

use super::{Lowerer, MAX_LOWER_NEST_DEPTH, WordSpace};
use crate::command_binding::{
    RetainedSourceBodyInventory, SourceAnalysisOptions, SourceCommandBindings,
};
use crate::ir::{Script, Statement};
use crate::segmenter::SegmentedCommand;
use std::collections::HashMap;
use tcl_lexer::{SourceSegmentationPurpose, TokenType};
use tcl_registry::events::IrulesExecutionContext;

mod body_units;
mod const_map;
mod original_bodies;
mod static_eval;

#[cfg(test)]
pub(super) use const_map::{assigned_local_body_literal, invalidate_const_map_for};
pub(super) use original_bodies::unavailable_original_body_script;
#[cfg(test)]
pub(super) use static_eval::{body_has_dynamic_barrier, eval_list_literal_body};

/// Stand-in `Script` for a body past [`MAX_LOWER_NEST_DEPTH`]: a single
/// [`Statement::Barrier`] spanning `[base_offset, base_offset + len)`, so
/// downstream passes treat the unanalysed region as having unknown effects
/// (matching how a dynamic `eval`/`uplevel` body is already modelled)
/// rather than as empty/dead code.
pub(super) fn over_depth_script(base_offset: u32, len: usize) -> Script {
    let end = base_offset.saturating_add(u32::try_from(len).unwrap_or(u32::MAX));
    Script::from_statements(vec![Statement::Barrier {
        span: tcl_lexer::Span::new(base_offset, end),
        reason: "nesting depth exceeds analysis limit".to_owned(),
        command: String::new(),
        canonical_command: None,
        args: Vec::new(),
        tokens: None,
    }])
}

impl Lowerer<'_> {
    /// Enter a fresh procedure-like runtime frame for one body lowering.
    ///
    /// A procedure frame must not inherit literal bindings from its caller;
    /// `proc_depth` also enables the body-only const-map paths.  Callers that
    /// have an explicit iRules placement supply it here, so all three pieces
    /// of procedure-frame setup are balanced together.
    pub(super) fn in_procedure_frame<T>(
        &mut self,
        context: Option<IrulesExecutionContext>,
        lower: impl FnOnce(&mut Self) -> T,
    ) -> T {
        self.proc_depth += 1;
        self.const_map_stack.push(HashMap::new());
        let previous_context =
            context.map(|context| std::mem::replace(&mut self.irules_execution_context, context));
        let result = lower(self);
        if let Some(previous) = previous_context {
            self.irules_execution_context = previous;
        }
        self.const_map_stack.pop();
        self.proc_depth -= 1;
        result
    }

    /// Lower a source-text literal into a [`Script`] without
    /// installing it as the module top-level. Used by passes that
    /// need to lower a sub-script (e.g. the
    /// [`crate::inline_uplevel`] rewriter materialising a
    /// brace-literal callsite body).
    pub fn lower_into_script(&mut self, source: &str, namespace: &str) -> Script {
        self.lower_script(source, namespace, SourceSegmentationPurpose::NestedScript)
    }

    /// Lower materialised text under an invocation-time binding proof, retaining
    /// its original document offsets and actual caller activation.
    pub(crate) fn lower_into_script_with_bindings(
        &mut self,
        source: &str,
        base: u32,
        namespace: &str,
        bindings: SourceCommandBindings,
    ) -> Script {
        self.lower_into_script_with_boxed_bindings(
            source,
            base,
            namespace,
            Box::new(bindings.into()),
        )
    }

    fn lower_into_script_with_boxed_bindings(
        &mut self,
        source: &str,
        base: u32,
        namespace: &str,
        bindings: Box<RetainedSourceBodyInventory>,
    ) -> Script {
        let mut positioned_source = " ".repeat(base as usize);
        positioned_source.push_str(source);
        let enclosing_source = std::mem::replace(&mut self.source, positioned_source);
        let enclosing_bindings = self.exchange_boxed_source_bindings(Some(bindings));
        let result = self.lower_body(source, base, namespace);
        drop(self.exchange_boxed_source_bindings(enclosing_bindings));
        self.source = enclosing_source;
        result
    }

    // Whole inventories are constructed and restored outside recursive frames.
    #[inline(never)]
    pub(super) fn exchange_boxed_source_bindings(
        &mut self,
        bindings: Option<Box<RetainedSourceBodyInventory>>,
    ) -> Option<Box<RetainedSourceBodyInventory>> {
        std::mem::replace(
            &mut self.source_bindings,
            bindings.map(|bindings| *bindings),
        )
        .map(Box::new)
    }

    pub(super) fn current_source_channel(&self) -> tcl_lexer::SourceChannel {
        self.source_channel
    }

    pub(super) fn segment_source(
        &self,
        text: &str,
        base: u32,
        purpose: SourceSegmentationPurpose,
    ) -> Option<Vec<SegmentedCommand>> {
        let image =
            tcl_lexer::SourceImage::from_bytes(text.as_bytes(), self.current_source_channel());
        crate::segmenter::segment_commands_image_with_offset_and_config(
            &image,
            base,
            purpose.config(self.config),
        )
    }

    /// Lower a source string to an IR script.
    ///
    /// Depth-guarded entry to the recursive lowering, alongside
    /// [`Self::lower_body`] — every nested body re-enters through one of
    /// these two, so bounding both here caps the whole `lower_script` ↔
    /// `lower_body` ↔ `lower_segmented` ↔ `lower_command` recursion. At the
    /// cap we stop descending and lower the remaining text as a single
    /// opaque [`Statement::Barrier`] rather than silently dropping it —
    /// downstream passes (SCCP, dead-code elimination, …) must treat
    /// unanalysed content as having unknown effects, not as empty/dead.
    pub(super) fn lower_script(
        &mut self,
        source: &str,
        namespace: &str,
        purpose: SourceSegmentationPurpose,
    ) -> Script {
        self.nest_depth += 1;
        if MAX_LOWER_NEST_DEPTH.exceeded(self.nest_depth) {
            self.nest_depth -= 1;
            return over_depth_script(0, source.len());
        }

        // `source` starts a span space: its statements' offsets are relative
        // to it, whether it is the document or a literal materialised inside
        // one.  Swap it in for the descent and restore the enclosing one after
        // (see [`Self::source`]).
        let enclosing_source = std::mem::replace(&mut self.source, source.to_owned());
        // The selected local parser purpose is shared by binding analysis and
        // segmentation. Parent inputs/config remain unchanged for original
        // word and metadata joins; a missing body receipt cannot borrow them.
        let frame = self.source_entry_frame(namespace);
        let image = tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_channel);
        let Some(bindings) = SourceCommandBindings::analyse_image_with_original_native_formals(
            &image,
            &frame,
            self.config,
            self.registry,
            SourceAnalysisOptions {
                analyser_source_settings: self.analyser_source_settings.as_ref(),
                source_point_entry: self
                    .source_point_entry
                    .as_ref()
                    .filter(|_| self.nest_depth == 1),
                metadata_context: self.source_metadata_input(),
                hosted_execution_context: self.hosted_execution_context,
                execution_name_policy: self.execution_name_policy,
                logical_source_input: self.logical_source_input.as_ref(),
                vendor_source_input: self.vendor_source_input.as_ref(),
                compilation_scope: self.compilation_scope,
                unentered: false,
                invocation_realm: self.invocation_realm,
                declared_commands: self
                    .source_declared_commands
                    .as_ref()
                    .or(self.declared_commands),
                trusted_package_loaders: &self.trusted_package_loaders,
                trusted_source_modules: self.effective_source_modules(),
                unknown_entry: self.unknown_entry || self.nest_depth > 1,
                host_extensible_command_table: self.host_command_table.is_extensible(),
                invocation_dialect: self.invocation_dialect,
                compiled_variable_provider: self.compiled_variable_provider,
                expression_parser: self.expression_parser,
                native_entry: self.native_entry.as_deref(),
                stock_tclsh_stdout: self.stock_tclsh_stdout.as_ref(),
                incoming_formals: if self.nest_depth == 1 {
                    &self.incoming_formals
                } else {
                    &[]
                },
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    frame: if self.module.top_level_kind == crate::ir::TopLevelKind::ProcedureBody {
                        tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode
                    } else {
                        self.native_compilation.frame
                    },
                    ..self.native_compilation
                },
            },
            purpose,
            self.original_native_compiler_formals
                .as_ref()
                .filter(|_| self.nest_depth == 1),
        ) else {
            self.source = enclosing_source;
            self.nest_depth -= 1;
            return over_depth_script(0, source.len());
        };

        let enclosing_bindings = self.source_bindings.replace(bindings.into());
        let result = self.lower_script_inner(source, namespace, purpose);

        self.restore_script_bindings(enclosing_bindings);

        self.source = enclosing_source;
        self.nest_depth -= 1;
        result
    }

    #[inline(never)]
    pub(super) fn restore_script_bindings(
        &mut self,
        enclosing: Option<RetainedSourceBodyInventory>,
    ) {
        let completed = std::mem::replace(&mut self.source_bindings, enclosing);
        if self.nest_depth == 1 {
            self.module.procedure_implementation_bodies = completed.as_ref().map_or_else(
                || std::sync::Arc::from([]),
                |bindings| {
                    bindings
                        .procedure_implementation_bodies()
                        .map(
                            |body| crate::command_binding::SourceInstalledProcedureBody {
                                allocation: body.allocation.clone(),
                                command: body.allocation.command.clone(),
                                namespace: body.namespace.to_owned(),
                                namespace_key: body.namespace_key.clone(),
                                parameters: body.parameters.to_vec(),
                                original_parameters: body.original_parameters.cloned(),
                                source: body.source.clone(),
                            },
                        )
                        .collect()
                },
            );
            self.module.future_call_sites = completed.as_ref().map_or_else(Vec::new, |bindings| {
                bindings
                    .future_body_inventories()
                    .iter()
                    .flat_map(crate::command_binding::SourceFutureBodyInventory::call_sites)
                    .collect()
            });
            self.module_source_bindings = completed.map(Box::new);
        }
    }

    /// Run `body` with the words of the commands it lowers read from `text`,
    /// segmented at `base` — see [`WordSpace`]. Restores the enclosing space
    /// after, so a nested body does not outlive its own descent.
    pub(super) fn in_word_space<T>(
        &mut self,
        text: &str,
        base: u32,
        body: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let channel = self.current_source_channel();
        let enclosing =
            std::mem::replace(&mut self.word_space, WordSpace::new(text, base, channel));
        let result = body(self);
        self.word_space = enclosing;
        result
    }

    fn lower_script_inner(
        &mut self,
        source: &str,
        namespace: &str,
        purpose: SourceSegmentationPurpose,
    ) -> Script {
        let Some(commands) = self.segment_source(source, 0, purpose) else {
            return over_depth_script(0, source.len());
        };

        self.const_map_stack.push(HashMap::new());
        self.command_binding_site_stack.push(Vec::new());
        let stmts =
            self.in_word_space(source, 0, |this| this.lower_segmented(&commands, namespace));

        let command_binding_sites = self
            .command_binding_site_stack
            .pop()
            .expect("the script command-binding site scope is balanced");
        self.const_map_stack.pop();
        let script = self.attach_native_compilation_failure(
            Script::from_lowered_parts(stmts, command_binding_sites),
            0,
        );

        self.retain_script_source(script, source, 0)
    }

    /// Lower a body argument (inside braces/brackets) to an IR script.
    ///
    /// Inherits the parent scope's const-map so a child body can
    /// still relax its
    /// `eval` / `uplevel` against literals bound in the enclosing
    /// scope (`set body {literal}; catch {uplevel 1 $body}` is the
    /// canonical example).
    ///
    /// Depth-guarded the same way as [`Self::lower_script`] — see its doc
    /// comment.
    pub(super) fn lower_body(&mut self, text: &str, base_offset: u32, namespace: &str) -> Script {
        let context = self.nested_irules_execution_context();
        self.lower_body_in_irules_context(text, base_offset, namespace, context)
    }

    /// Lower a body while explicitly preserving the registry-owned iRules
    /// execution context.  `when` / `proc` declarations use this to install
    /// their event/procedure frame; ordinary script-bearing commands use
    /// [`Self::lower_body`], which derives the nested context instead.
    pub(super) fn lower_body_in_irules_context(
        &mut self,
        text: &str,
        base_offset: u32,
        namespace: &str,
        context: IrulesExecutionContext,
    ) -> Script {
        let previous = std::mem::replace(&mut self.irules_execution_context, context);
        self.nest_depth += 1;
        let result = if MAX_LOWER_NEST_DEPTH.exceeded(self.nest_depth) {
            self.nest_depth -= 1;
            over_depth_script(base_offset, text.len())
        } else {
            let result = self.lower_body_inner(text, base_offset, namespace);
            self.nest_depth -= 1;
            result
        };
        self.irules_execution_context = previous;
        result
    }

    /// Which iRules placement a nested script inherits.  A control-flow or
    /// callback body within an event/procedure keeps that live frame; a body
    /// reached from a top-level executable command remains invalid rather
    /// than gaining the power to declare a handler or procedure.
    const fn nested_irules_execution_context(&self) -> IrulesExecutionContext {
        match self.irules_execution_context {
            IrulesExecutionContext::TopLevel | IrulesExecutionContext::InvalidNestedBody => {
                IrulesExecutionContext::InvalidNestedBody
            }
            IrulesExecutionContext::EventBody => IrulesExecutionContext::EventBody,
            IrulesExecutionContext::ProcedureBody => IrulesExecutionContext::ProcedureBody,
        }
    }

    /// Run one synthesised script in a known iRules execution context.  A
    /// materialised proc body has no source token to route through
    /// [`Self::lower_body_in_irules_context`], but it still must not treat a
    /// nested declaration as a file-level one.
    pub(super) fn lower_script_in_irules_context(
        &mut self,
        source: &str,
        namespace: &str,
        context: IrulesExecutionContext,
    ) -> Script {
        let previous = std::mem::replace(&mut self.irules_execution_context, context);
        let result = self.lower_script(source, namespace, SourceSegmentationPurpose::NestedScript);
        self.irules_execution_context = previous;
        result
    }

    /// The part of a body word's `text` whose spans may honestly be rebased
    /// by that word's content offset, given the word occupies `tok`'s span in
    /// [`Self::source`].
    ///
    /// Every body lowering below adds one base offset to the spans the
    /// segmenter cuts from a word *value*, which is truthful only while that
    /// value is the source region verbatim.  A compound `{body}x` word breaks
    /// it: the value is the brace content welded to the tail with the closing
    /// `}` dropped, so every token past the drop slides one byte left — an
    /// off-by-one span on ASCII, an offset inside a UTF-8 sequence on
    /// anything else.  [`crate::segmenter::body_text_in_region`]
    /// clamps such a value to the contiguous braced part and passes an
    /// ordinary body through untouched; the analyser's `analyse_body`
    /// applies it too, and this is the lowering side of the same guard.
    ///
    /// A substituting word (`$body`, `[gen]`) is left alone: its value is
    /// run-time data that no source region can be compared against, so there
    /// is nothing here to prove and clamping it would only delete text.  Such
    /// words are the literal gates' business — they barrier rather than
    /// lower.
    ///
    /// A translating Document channel folds CRLF in a braced word's cooked
    /// value while its spans stay original offsets, so a braced body maps
    /// back to its source region first
    /// ([`crate::segmenter::braced_body_source_region`]).
    fn guarded_body_text(&self, tok: tcl_lexer::Token, text: &str) -> String {
        if matches!(tok.kind, TokenType::Var | TokenType::Cmd) {
            return text.to_owned();
        }
        let base = (tok.span.start() + u32::from(tok.content_offset)) as usize;
        let text = crate::segmenter::braced_body_source_region(
            &self.source,
            base,
            tok.span.end().saturating_sub(1) as usize,
            text,
        );
        crate::segmenter::body_text_in_region(&self.source, base, tok.span.end() as usize, text)
            .to_owned()
    }

    fn lower_body_inner(&mut self, text: &str, base_offset: u32, namespace: &str) -> Script {
        let Some(commands) =
            self.segment_source(text, base_offset, SourceSegmentationPurpose::NestedScript)
        else {
            return over_depth_script(base_offset, text.len());
        };

        let inherited = self.const_map_stack.last().cloned().unwrap_or_default();
        self.const_map_stack.push(inherited);
        self.command_binding_site_stack.push(Vec::new());
        let stmts = self.in_word_space(text, base_offset, |this| {
            this.lower_segmented(&commands, namespace)
        });

        let command_binding_sites = self
            .command_binding_site_stack
            .pop()
            .expect("the body command-binding site scope is balanced");
        self.const_map_stack.pop();
        let script = self.attach_native_compilation_failure(
            Script::from_lowered_parts(stmts, command_binding_sites),
            base_offset,
        );

        self.retain_script_source(script, text, base_offset)
    }

    /// Retain only a failure agreed by every reached compilation entry.
    fn attach_native_compilation_failure(&self, mut script: Script, chunk: u32) -> Script {
        if self.readonly_logical_body.is_read_only()
            || self.readonly_hosted_body.is_read_only()
            || self.readonly_authored_body.is_read_only()
            || self.readonly_native_declaration_body.is_read_only()
        {
            return script;
        }
        script.native_compilation_failure = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.native_compilation_failure_at(chunk))
            .cloned()
            .map(Box::new);
        if let Some(bindings) = &self.source_bindings
            && (bindings.native_compilation_provider_required_at(chunk)
                || script.native_compilation_failure.is_some())
        {
            script.native_compilation_admission = Some(std::sync::Arc::new(
                crate::native_compilation_admission::NativeCompilationAdmission {
                    source: bindings.native_compilation_source_at(chunk).cloned(),
                    failure: script
                        .native_compilation_failure
                        .as_deref()
                        .cloned()
                        .map(std::sync::Arc::new),
                    provider_required: true,
                },
            ));
        }
        script
    }

    /// Lower a list of segmented commands to IR statements.
    pub(super) fn lower_segmented(
        &mut self,
        segments: &[SegmentedCommand],
        namespace: &str,
    ) -> Vec<Statement> {
        let mut stmts = Vec::new();
        for seg in segments {
            if seg.is_partial {
                Self::push_partial_command(&mut stmts, seg.span);
                continue;
            }
            if let Some(stmt) = self.lower_command(seg, namespace) {
                self.update_const_map(seg, &stmt);
                Self::push_boxed_statement(&mut stmts, stmt);
            }
        }
        self.debug_assert_spans_in_source(&stmts);
        stmts
    }

    // Materialisation is deliberately outside the recursive descent frame.
    #[inline(never)]
    fn push_partial_command(statements: &mut Vec<Statement>, span: tcl_lexer::Span) {
        statements.push(Statement::Barrier {
            span,
            reason: "incomplete command".into(),
            command: String::new(),
            canonical_command: None,
            args: vec![],
            tokens: None,
        });
    }

    #[inline(never)]
    fn push_boxed_statement(statements: &mut Vec<Statement>, statement: Box<Statement>) {
        statements.push(*statement);
    }

    /// Every statement leaving [`Self::lower_segmented`] must carry a span
    /// that is a real range of the buffer it was lowered from — the contract
    /// downstream consumers rely on when they slice the source with an IR
    /// span (AGENTS.md: "trust it rather than re-deriving").
    ///
    /// This is the producer-side chokepoint: every statement the lowering
    /// emits passes through here, at the nesting level whose segments were
    /// cut from [`Self::source`], so a body rebase that overshoots its
    /// document (a compound `{body}x` word, a non-literal body word)
    /// fails the build's own tests instead of
    /// reaching a consumer.  Debug-only: the guards in
    /// [`Self::lower_body_from_tok`] and codegen's slice clamp are what keep
    /// a release build safe.
    fn debug_assert_spans_in_source(&self, stmts: &[Statement]) {
        debug_assert!(
            self.source.is_empty()
                || stmts
                    .iter()
                    .all(|stmt| self.source.get(stmt.span().as_range()).is_some()),
            "lowering emitted a span outside its {} byte source: {:?}",
            self.source.len(),
            stmts
                .iter()
                .map(Statement::span)
                .filter(|span| self.source.get(span.as_range()).is_none())
                .collect::<Vec<_>>(),
        );
    }
}
