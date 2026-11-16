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

//! The per-command checks rerun on every command nested in a substitution or an expression.

use super::super::state::Analyser;
use super::nested_invocations::nested_source_bodies;
use super::{DispatchSite, VarOrCmdSite};
use crate::depth_guard::MAX_BRACKET_TEXT_DEPTH;
use crate::parsing::syntax::descend::descend_token;
use crate::parsing::syntax::segment::segments_from_tree;
use crate::segmenter::SegmentedCommand;
use tcl_lexer::{LexerConfig, SourceMap, Token, TokenType};

impl Analyser {
    /// Run the per-command syntactic dispatch
    /// ([`Self::emit_dispatch_site_diagnostics`] — security W101-W312,
    /// bounds W230-W242, W001 / W004 / W304, arity E002-E003, …) on every
    /// command nested in a ``[…]`` substitution of this command's words,
    /// recursing into further nested substitutions and the nested
    /// commands' own bodies.  The main analyser walk descends proc /
    /// control-flow *bodies* (so those commands are already checked) but
    /// never a ``[…]`` substitution, which it treats as an opaque value —
    /// so `set fh [open "|$cmd" r]` / `set x [string index abc 99]` would
    /// otherwise escape the per-command checks.
    ///
    /// Only ``[…]`` regions are entered here; everything reached from
    /// inside one is invisible to the main walk, so the recursion may
    /// freely descend the nested commands' own bodies and substitutions
    /// without double-firing a diagnostic the main walk already emitted.
    /// `scope_path` is the enclosing command's scope — a substitution
    /// runs in the same frame as the command it is embedded in.
    pub(super) fn run_nested_command_diagnostics(
        &mut self,
        arg_tokens_in: &[Token],
        scope_path: &[usize],
    ) {
        // Collect the descended substitution commands first (this borrows
        // `self.source` through the `SourceMap`); run the `&mut self`
        // dispatch afterwards, once the immutable borrow has ended.  Each
        // `SegmentedCommand` is fully owned (absolute spans), so it
        // outlives the borrow.
        let config = self.file_lexer_config();
        let mut nested: Vec<SegmentedCommand> = Vec::new();
        {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            for arg_tok in arg_tokens_in {
                let start = arg_tok.span.start() as usize;
                let end = arg_tok.span.end() as usize;
                if start > self.source.len() || end > self.source.len() || start > end {
                    continue;
                }
                match arg_tok.kind {
                    TokenType::Cmd => {
                        for frag in self.cmd_fragments(*arg_tok, config) {
                            collect_substitution_segments(
                                &sm,
                                &self.result,
                                frag,
                                config,
                                &mut nested,
                            );
                        }
                    }
                    // A quoted / bareword / compound word (`Esc`) may carry
                    // live `[...]` substitutions (`log "got [HTTP::uri]"`).
                    // The bare-`Cmd` walk never enters it, so its substitution
                    // commands escape every per-command check (IRULE3102, W123,
                    // …).  A braced `Str` data word's `[...]` is literal and is
                    // skipped; braced *expr* args are covered by
                    // `run_nested_expr_diagnostics`.
                    TokenType::Esc => {
                        let Some(arg_src) = Analyser::source_slice(&self.source, start, end) else {
                            continue;
                        };
                        if !arg_src.contains('[') {
                            continue;
                        }
                        for (off, inner) in top_level_cmd_subst_regions(arg_src) {
                            let off = u32::try_from(off)
                                .expect("byte offset fits in u32 for in-memory source");
                            let base = arg_tok.span.start() + off;
                            for seg in crate::segmenter::segment_commands_with_offset_and_config(
                                inner,
                                base,
                                config.nested(),
                            ) {
                                collect_segment_recursive(
                                    &sm,
                                    &self.result,
                                    seg,
                                    config,
                                    &mut nested,
                                    0,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        for seg in nested {
            self.dispatch_nested_segment(&seg, scope_path);
        }
    }

    /// Run the per-command syntactic dispatch on every command nested in a
    /// ``[…]`` substitution that appears inside a *braced expression*
    /// argument (`if { [acl_ok] } …`, `while { [done $x] } …`).  Such an
    /// argument is a `Str` (braced) token, so it is opaque to
    /// [`Self::run_nested_command_diagnostics`] (which only descends bare
    /// `Cmd` argument tokens) — yet the expression's `[…]` substitutions
    /// are live commands the main walk never reaches.
    pub(super) fn run_nested_expr_diagnostics(&mut self, site: &DispatchSite<'_>) {
        let Some(original) = self.original_diagnostic_source(site) else {
            return;
        };
        let Some(expressions) = original.expression_arguments() else {
            return;
        };
        let expr_indices = expressions
            .arguments
            .into_iter()
            .filter_map(|argument| original.written_index(argument))
            .collect::<Vec<_>>();
        let arg_tokens = site.arg_tokens;
        let scope_path = site.scope_path;
        if expr_indices.is_empty() {
            return;
        }
        let config = self.file_lexer_config();
        let mut nested: Vec<SegmentedCommand> = Vec::new();
        {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            for idx in expr_indices {
                // Only a *braced* expr arg is opaque to the bare-`Cmd` walk;
                // an unbraced `[…]` expr arg is itself a `Cmd` token already
                // descended by `run_nested_command_diagnostics`.
                let Some(tok) = arg_tokens.get(idx) else {
                    continue;
                };
                if tok.kind != TokenType::Str {
                    continue;
                }
                // Re-lex the braced expression as a script: its operands
                // (`$x`, `+`, literals) are not commands, but each `[…]`
                // substitution still tokenises as a `Cmd` to descend.
                let descended = descend_token(&sm, *tok, config);
                for seg in segments_from_tree(descended.tree(), &sm) {
                    for inner in &seg.all_tokens {
                        if inner.kind == TokenType::Cmd {
                            collect_substitution_segments(
                                &sm,
                                &self.result,
                                *inner,
                                config,
                                &mut nested,
                            );
                        }
                    }
                }
            }
        }
        for seg in nested {
            self.dispatch_nested_segment(&seg, scope_path);
        }
    }

    fn dispatch_nested_package_hook(
        &mut self,
        cmd_name: &str,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        match self.resolve_analyser_hook(cmd_name, args) {
            Some(tcl_registry::hooks::AnalyserHookId::PackageRequire) => {
                self.handle_package_require(cmd_name, cmd_tok, args, arg_tokens);
            }
            Some(tcl_registry::hooks::AnalyserHookId::PackageProvide) => {
                self.handle_package_provide(cmd_tok, args, arg_tokens);
            }
            Some(tcl_registry::hooks::AnalyserHookId::Catch) => {
                self.handle_var_binding_command(cmd_tok, arg_tokens, scope_path);
            }
            _ => {}
        }
    }

    /// Run the full per-command diagnostic dispatch on one command
    /// descended from a ``[…]`` substitution: the syntactic emitters
    /// ([`Self::emit_dispatch_site_diagnostics`]) plus the EXPR-argument
    /// walk ([`Self::dispatch_expr_arguments`], which hosts W100 / W110 /
    /// W114 / W003).  The main walk runs both on a top-level command but
    /// only the syntactic half reaches substitution commands by
    /// default, so an unbraced `expr` inside `[…]`
    /// (`set y [expr $a + $b]`) would otherwise escape W100.
    fn dispatch_nested_segment(&mut self, seg: &SegmentedCommand, scope_path: &[usize]) {
        if seg.texts.is_empty() || seg.argv.is_empty() {
            return;
        }
        let cmd_name = seg.texts[0].clone();
        let cmd_tok = seg.argv[0];
        self.observe_interp_visibility(&seg.argv, scope_path);
        let args = seg.texts.get(1..).unwrap_or(&[]);
        let arg_tokens = seg.argv.get(1..).unwrap_or(&[]);
        let arg_single = seg.single_token_word.get(1..).unwrap_or(&[]);
        self.record_original_variable_receivers(cmd_tok.span.start(), arg_tokens, scope_path);
        self.record_dynamic_variable_name_sites(
            args,
            arg_tokens,
            arg_single,
            scope_path,
            cmd_tok.span.start(),
            false,
        );
        // `emit_arity_diagnostics` expects the expand array parallel to
        // the *full* argv (head at index 0), matching `process_command`.
        let arg_expand = seg.expand_word.as_deref().unwrap_or(&[]);
        let site = DispatchSite {
            cmd_name: &cmd_name,
            args,
            arg_tokens,
            arg_single,
            arg_expand_in: arg_expand,
            cmd_tok,
            scope_path,
            // A command written inside a `[…]` substitution is written
            // source, with its own words: `$x` here really is a substitution
            // the author typed. (The list-built path never reaches this
            // walker — `process_command` skips the nested-substitution scan
            // entirely when its words are pre-substituted.)
            presubstituted_args: false,
        };
        self.emit_dispatch_site_diagnostics(&site);
        self.dispatch_expr_arguments(&site);
        // W216 (broken brace-form array access, `${arr}(idx)` / `${arr($i)}`)
        // must reach substitution commands too: `set v [puts ${arr}(name)]`
        // hides the offending word inside a `[…]`, which the main `walk_body`
        // pass treats as an opaque value.  Without this the nested word escapes
        // the check entirely, so the brace-then-paren emitter must run on
        // substitution commands too.
        self.emit_w216_brace_then_paren(seg);
        // A `VarWrite`-role command nested in a `[…]` substitution still
        // writes its variable arguments into the enclosing scope — the
        // idiomatic `if {[regexp {…} $s m]} {…}` / `set n [scan $s "%d" x]` /
        // `while {[gets $chan line] >= 0} {…}` — so bind them for
        // completion/hover/definition and read-before-set, just as the
        // top-level `process_command` path does.
        self.handle_var_binding_command(cmd_tok, arg_tokens, scope_path);
        // A `CommandName` operand of a substituted command (`puts [info body
        // greet]`) names the command exactly as the top-level form does.
        self.record_command_name_invocations(args, arg_tokens, scope_path, cmd_tok.span.start());
        // Record the nested command's variable-/command-substitution-as-command
        // call site too (`puts [$obj method]`, `if {[$obj ok]} …`).  The main
        // walk treats `[…]` as a value, so without this the W307 multi-dispatch
        // suppression under-counts `$obj` dispatches that live inside command
        // substitutions and the W307/W308 emitters never see them.
        self.record_var_or_cmd_command_site(VarOrCmdSite {
            cmd_name: &cmd_name,
            cmd_tok,
            head_expanded: arg_expand.first().copied().unwrap_or(false),
            args,
            arg_tokens,
            arg_expand: arg_expand.get(1..).unwrap_or(&[]),
            scope_path,
        });
        // W125 (orphaned keyword) and IRULE5005 (direct iRules-proc call
        // without `call`) must reach substitution commands too, retaining their
        // independent lookup purposes: `when HTTP_REQUEST { set x
        // [helper] }` invokes `helper` directly inside a `[…]`, and without this
        // the IRULE5005 emitter never sees it.  Matches the top-level
        // `process_command` ordering (proc-resolution after site recording).
        self.emit_proc_resolution_diagnostics(&cmd_name, args, cmd_tok, scope_path);
        // A namespace-name argument nested in a `[…]` substitution names the
        // same namespace a top-level one would — and this is the *dominant*
        // real shape: `set targets [namespace children ::tomato]` is the very
        // line this was mined from.  The main walk treats `[…]` as an
        // opaque value, so without this the occurrence would be invisible to
        // go-to-definition / hover / find-references exactly where it matters
        // most.
        self.record_namespace_name_references(
            args,
            arg_tokens,
            scope_path,
            cmd_tok.span.start(),
            false,
        );
        // A `package require` nested in a `[…]` substitution still runs — the
        // guarded-optional-dependency idiom puts it exactly there
        // (`if {[catch {package require Tk} err]} { … fallback … }`), and the
        // body collector descends `catch`'s script argument to reach it. Without
        // this, W120 ("requires `package require Tk`") false-positives on every
        // file using the standard guard, and the W123 conservative
        // any-require-seen gate never engages.  Only the two `package`
        // hooks run here — the substitution path deliberately dispatches
        // no other handler family.
        self.dispatch_nested_package_hook(&cmd_name, cmd_tok, args, arg_tokens, scope_path);
        // A definition command (`proc`, a class definer, `oo::define`) or an
        // `apply` lambda nested inside a substitution — the feature-detection
        // idiom `if {![catch {oo::configurable create Greeter {…}}]} {…}`, or
        // the ordinary `set r [apply {{…} {…}} …]` — still defines its
        // procedure/class and walks its body **in the body's own scope**,
        // exactly as the top-level dispatch does.  Without this the definer's
        // member keywords (`property`, `constructor`) and the defined name
        // (`Greeter`) all draw W123 as unknown commands, even though W002
        // already reported the dialect-gated definer once, and a lambda body
        // reached this way is walked by nothing at all.  The generic
        // collector descends none of these
        // bodies — a definer's by `nested_source_bodies`, a lambda's
        // because `apply`'s script argument is `ArgRole::LambdaLiteral`, which
        // the shared script-body owner deliberately does not resolve — so no body is ever
        // also dispatched as a plain script in the *enclosing* scope.  The
        // dispatch mirrors the top-level chain exactly: the stamped `Proc` /
        // `OoDefine` / `Apply` hooks first (the handlers do not name-guard
        // themselves), then the grammar-driven definer trio only on the
        // hookless path.
        // Each handler returns whether it claimed the command; nothing
        // follows this dispatch, so an unclaimed command simply ends the
        // substitution walk the same way a claimed one does.
        if self
            .dispatch_original_class_configuration(cmd_tok, scope_path)
            .is_some()
        {
            return;
        }
        {
            use tcl_registry::hooks::AnalyserHookId as Hook;
            match self.resolve_analyser_hook(&cmd_name, args) {
                Some(Hook::Proc) => {
                    self.handle_proc_command_at(
                        &cmd_name, args, arg_tokens, arg_single, scope_path, cmd_tok,
                    );
                }
                Some(Hook::OptProc) => {
                    self.handle_opt_proc_command(args, arg_tokens, arg_single, scope_path);
                }
                Some(Hook::OoDefine) => {
                    self.handle_oo_define_command(
                        &cmd_name, args, arg_tokens, arg_single, scope_path,
                    );
                }
                // `apply {{params} body ?ns?}` — the handler builds the
                // lambda's own `Proc` scope rooted at the lambda's namespace
                // (element 2, or `::`), binds its parameters there, and walks
                // the body in it.  Routing the substitution-position call
                // through the same handler is what keeps the lambda's frame
                // isolated: a variable the body sets is a local of the lambda,
                // not of the enclosing proc, and a bareword call inside it
                // resolves in the lambda's namespace.
                Some(Hook::Apply) => {
                    self.handle_apply_command(args, arg_tokens, scope_path);
                }
                None => {
                    self.note_pack_definer(&cmd_name);
                    let _claimed = self
                        .handle_oo_class_command(&cmd_name, args, arg_tokens, scope_path, cmd_tok)
                        || self.handle_snit_type_command(&cmd_name, args, arg_tokens, scope_path)
                        || self.handle_itcl_class_command(&cmd_name, args, arg_tokens, scope_path)
                        || self.handle_source_inventory_definition(
                            &cmd_name, args, arg_tokens, scope_path, cmd_tok,
                        )
                        || self.handle_jim_class_command(
                            &cmd_name, args, arg_tokens, scope_path, cmd_tok,
                        )
                        || self.handle_jim_class_member_call(
                            &cmd_name, args, arg_tokens, scope_path, cmd_tok,
                        );
                }
                Some(_) => {}
            }
        }
    }
}

pub(super) fn collect_substitution_segments(
    sm: &SourceMap<'_>,
    analysis: &super::super::types::AnalysisResult,
    cmd_tok: Token,
    config: LexerConfig,
    out: &mut Vec<SegmentedCommand>,
) {
    collect_substitution_segments_at(sm, analysis, cmd_tok, config, out, 0);
}

fn collect_substitution_segments_at(
    sm: &SourceMap<'_>,
    analysis: &super::super::types::AnalysisResult,
    cmd_tok: Token,
    config: LexerConfig,
    out: &mut Vec<SegmentedCommand>,
    depth: u32,
) {
    if cmd_tok.kind != TokenType::Cmd || sm.token_text(cmd_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, cmd_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        collect_segment_recursive(sm, analysis, seg, config, out, depth);
    }
}

pub(super) fn collect_segment_recursive(
    sm: &SourceMap<'_>,
    analysis: &super::super::types::AnalysisResult,
    seg: SegmentedCommand,
    config: LexerConfig,
    out: &mut Vec<SegmentedCommand>,
    depth: u32,
) {
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        out.push(seg);
        return;
    }
    for tok in &seg.all_tokens {
        if tok.kind == TokenType::Cmd {
            collect_substitution_segments_at(sm, analysis, *tok, config, out, depth + 1);
        }
    }
    for inner in nested_source_bodies(
        sm,
        analysis,
        &seg,
        config,
        crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
    ) {
        collect_segment_recursive(sm, analysis, inner, config, out, depth + 1);
    }
    out.push(seg);
}

/// Top-level ``[...]`` command-substitution regions in `text`, as
/// `(inner_byte_offset, inner_text)` — the script *inside* the brackets and
/// the offset of its first byte within `text`.  ``\\[`` / ``\\]`` escapes are
/// honoured.  Only the *outermost* substitutions are returned; the caller's
/// segment recursion descends any nested `[...]`.  Used to reach `[...]`
/// embedded in a quoted / bareword word argument (`log "got [HTTP::uri]"`),
/// which the bare-`Cmd`-token walk misses.
///
/// Braces are **not** treated as opaque here: this only runs on `Esc` words
/// (quoted / bareword / compound).  A braced *word* is a `Str` token (handled
/// elsewhere and excluded by the caller); inside a quoted or bareword context
/// `{` / `}` are ordinary characters that do *not* suppress substitution, so
/// `log "got { [HTTP::uri] }"` still executes — and must still be scanned.
fn top_level_cmd_subst_regions(text: &str) -> Vec<(usize, &str)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'[' => {
                let inner_start = i + 1;
                let mut depth = 1i32;
                let mut j = inner_start;
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        b'\\' if j + 1 < bytes.len() => j += 1,
                        _ => {}
                    }
                    j += 1;
                }
                if depth == 0 && j <= bytes.len() {
                    out.push((inner_start, &text[inner_start..j]));
                    i = j + 1;
                } else {
                    i += 1;
                }
            }
            b'\\' if i + 1 < bytes.len() => i += 2,
            _ => i += 1,
        }
    }
    out
}
