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

//! The scan of member bodies for `my method` and other self-dispatch call sites.

use super::callback_targets::{
    StoredCallbackTarget, list_built_self_reference_spans, original_self_receiver_word,
    stored_callback_targets,
};
use super::dispatch_regions::{
    MAX_DISPATCH_SCAN_DEPTH, RetainedDispatchContext, nested_dispatch_regions,
    retained_dispatch_context, strip_outer_braces,
};
use super::object_method_sites::SpanSink;
use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;

/// Re-segment each brace-delimited body span in `bodies` and return the
/// name-token span of every `my <method>` invocation whose method name is
/// `method`, skipping the token at `skip` (the declaration site, when the
/// scanned class declares `method`).
///
/// Intra-class dispatch of a `TclOO` method is `my <method>` — the method
/// name is argv[1], not the command head.  A bare head equal to the method
/// name is *not* a call (a `TclOO` method is not a command in the body's
/// namespace; `<method> …` without `my`/an object errors "invalid command
/// name"), so only `my`-headed sites match. Call hierarchy, references,
/// rename and code lenses share this positioned selector matcher.
pub(crate) fn scan_my_method_sites(
    source: &str,
    analysis: &AnalysisResult,
    bodies: &[tcl_lexer::Span],
    method: &str,
    skip: Option<tcl_lexer::Span>,
) -> Vec<tcl_lexer::Span> {
    scan_method_sites(source, analysis, bodies, method, skip, false)
}

/// [`scan_my_method_sites`] with registry-declared deferred callback
/// recognition.  `external_callback_allowed` is the target method's
/// visibility gate: `[self] method` captures the externally-dispatched object
/// command and therefore requires `public`, whereas `[list my method]` stays
/// in the current object's private dispatch path.
pub(crate) fn scan_method_sites(
    source: &str,
    analysis: &AnalysisResult,
    bodies: &[tcl_lexer::Span],
    method: &str,
    skip: Option<tcl_lexer::Span>,
    external_callback_allowed: bool,
) -> Vec<tcl_lexer::Span> {
    scan_method_sites_by_kind(
        source,
        analysis,
        bodies,
        method,
        skip,
        external_callback_allowed,
        false,
    )
}

pub(super) fn scan_method_sites_by_kind(
    source: &str,
    analysis: &AnalysisResult,
    bodies: &[tcl_lexer::Span],
    method: &str,
    skip: Option<tcl_lexer::Span>,
    external_callback_allowed: bool,
    callbacks_only: bool,
) -> Vec<tcl_lexer::Span> {
    let Some(RetainedDispatchContext {
        dialect, config, ..
    }) = retained_dispatch_context(source, analysis)
    else {
        return Vec::new();
    };
    let ctx = MyMethodScan {
        source,
        analysis,
        config,
        dialect,
        method,
        skip,
        external_callback_allowed,
        callbacks_only,
        stored_callbacks: &[],
    };
    let mut out: Vec<tcl_lexer::Span> = Vec::new();
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    for &body_span in bodies {
        let stored_callbacks = stored_callback_targets(ctx, body_span);
        let body_ctx = MyMethodScan {
            stored_callbacks: &stored_callbacks,
            ..ctx
        };
        let mut sink = SpanSink {
            out: &mut out,
            seen: &mut seen,
        };
        scan_my_method_body(body_ctx, body_span, &mut sink);
    }
    out
}

/// Read-only context for the intra-class `my method` call-site scan: the
/// document `source`, its `dialect`, the `method` being looked up, and the
/// declaration span to `skip` (so the method's own name token isn't reported
/// as a call to itself).
#[derive(Clone, Copy)]
pub(super) struct MyMethodScan<'a> {
    pub(super) source: &'a str,
    pub(super) analysis: &'a AnalysisResult,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) dialect: &'static tcl_dialect::DialectProfile,
    pub(super) method: &'a str,
    pub(super) skip: Option<tcl_lexer::Span>,
    /// `[list [self] METHOD ...]` later dispatches through the external
    /// object command and therefore counts only for an exported method.
    /// `[list my METHOD ...]` is always eligible because it retains the
    /// current method frame's private dispatch semantics.
    pub(super) external_callback_allowed: bool,
    /// Suppress direct dispatches while proving a cursor sits on a deferred
    /// callback target rather than an ordinary method reference.
    pub(super) callbacks_only: bool,
    /// At most one same-scope static callback assignment per variable.  This
    /// is deliberately source-local and conservative; ambiguous writes are
    /// omitted before the callback walker sees them.
    pub(super) stored_callbacks: &'a [StoredCallbackTarget],
}

/// Scan a brace-delimited body span for `my method` call sites (stripping the
/// surrounding braces first), mirroring [`scan_obj_method_body`].
fn scan_my_method_body(ctx: MyMethodScan<'_>, body_span: tcl_lexer::Span, sink: &mut SpanSink<'_>) {
    if body_span.is_empty() {
        return;
    }
    let (start, end) = strip_outer_braces(ctx.source, body_span);
    if start >= end {
        return;
    }
    scan_my_method_region(ctx, start, end, 0, sink);
}

/// Segment `source[start..end]` and record the argv[1] span of every
/// `my <method>` invocation whose method name is `ctx.method`, recursing into
/// command-substitution (`[...]`) args **and** every same-frame
/// (`Plain`-`BodyKind`) control-flow / `eval` body argument
/// ([`nested_dispatch_regions`]) so a dispatch nested inside `return [my …]`,
/// an `if` / `while` / `foreach` / `switch` / `try` / `catch` body, or any
/// combination of the two, is found too. This is the same
/// recursion [`scan_obj_method_region`] performs, keeping intra-class `my`
/// dispatch and external `$obj` dispatch at parity.  The declaration span
/// (`ctx.skip`) and already-seen spans are elided.  `depth` guards against
/// runaway recursion on pathological input — see [`MAX_DISPATCH_SCAN_DEPTH`].
///
/// A bare head equal to the method name is *not* a call (a `TclOO` method is
/// not a command in the body's namespace; `<method> …` without `my`/an object
/// errors "invalid command name"), so only `my`-headed sites match.
pub(super) fn scan_my_method_region(
    ctx: MyMethodScan<'_>,
    start: usize,
    end: usize,
    depth: u32,
    sink: &mut SpanSink<'_>,
) {
    use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
    let source = ctx.source;
    if start >= end || end > source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return;
    }
    let region = &source[start..end];
    let commands = segment_commands_with_offset_and_config(
        region,
        u32::try_from(start).unwrap_or(0),
        ctx.config.nested(),
    );
    for cmd in &commands {
        if !ctx.callbacks_only
            && let (Some(head), Some(name_tok)) = (cmd.argv.first(), cmd.argv.get(1))
        {
            let h_start = head.span.start() as usize;
            let h_end = head.span.end() as usize;
            // Readonly source roles retain this actual point's availability
            // and grammar; the written keyword cannot reselect a catalogue row.
            let head_is_self_dispatch =
                h_start < source.len() && h_end <= source.len() && original_self_dispatch(ctx, cmd);
            if head_is_self_dispatch {
                let n_start = name_tok.span.start() as usize;
                let n_end = name_tok.span.end() as usize;
                if n_start < source.len()
                    && n_end <= source.len()
                    && &source[n_start..n_end] == ctx.method
                    && Some(name_tok.span) != ctx.skip
                {
                    let key = (name_tok.span.start(), name_tok.span.end());
                    if sink.seen.insert(key) {
                        sink.out.push(name_tok.span);
                    }
                }
            }
        }
        for span in list_built_self_reference_spans(ctx, cmd) {
            let key = (span.start(), span.end());
            if sink.seen.insert(key) {
                sink.out.push(span);
            }
        }
        for (inner_start, inner_end) in
            nested_dispatch_regions(source, ctx.analysis, ctx.dialect, cmd)
        {
            scan_my_method_region(ctx, inner_start, inner_end, depth + 1, sink);
        }
    }
}

/// Authored self-dispatch at an unchanged original command. A bracketed
/// receiving-object result keeps its own original child lookup; lexical
/// parsing supplies no current object, entered method frame or edit authority.
fn original_self_dispatch(
    ctx: MyMethodScan<'_>,
    command: &tcl_compiler::segmenter::SegmentedCommand,
) -> bool {
    use tcl_compiler::registry_invocation::source_structure;
    let Some(input) = ctx.analysis.resolved_input.as_ref() else {
        return false;
    };
    let context = input.context_registry();
    if let Some(words) = source_structure::source_registry_words(ctx.source, ctx.analysis, command)
        && words.origins().get(1)
            == Some(&tcl_compiler::registry_invocation::InvocationWordOrigin::Written(1))
        && words.with_source_schema(&context, |schema| {
            schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
        }) == Some(true)
    {
        return true;
    }
    let Some(&head) = command.argv.first() else {
        return false;
    };
    if command.single_token_word.first() != Some(&true) || head.kind != tcl_lexer::TokenType::Cmd {
        return false;
    }
    original_self_receiver_word(ctx, head)
}
