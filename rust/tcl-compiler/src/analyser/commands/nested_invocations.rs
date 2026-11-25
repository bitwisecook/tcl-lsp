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

//! Invocations inside a command's words: `[...]` substitutions, expression operands and math functions, and the scanners that collect their heads.

use super::super::state::Analyser;
use crate::depth_guard::MAX_BRACKET_TEXT_DEPTH;
use crate::parsing::syntax::descend::descend_token;
use crate::parsing::syntax::segment::segments_from_tree;
use crate::segmenter::SegmentedCommand;
use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType};
use tcl_registry::ArgRole;

/// A command head collected from a `[...]` substitution / expression scan,
/// ready to push as a `command_invocations` entry:
/// `(name, head-span, argc, callback_arity, ensemble_subcommand_candidate)`.
/// `argc` is the nested call's statically-known argument count (`None` when
/// `{*}`-expanded); `callback_arity` is `Some` only for an
/// `ArgRole::CommandPrefix` callback head (so the callback arity check runs)
/// and `None` for an ordinary call head. `ensemble_subcommand_candidate` is
/// `Some((word, span))` when the head's first actual argument is a static,
/// non-`{*}`-expanded word — a *candidate* subcommand
/// [`Analyser::push_collected_heads`] checks against
/// `ensemble_subcommand_targets` once the head's own resolved name is known
/// (a `[widget make hello]` nested call needs the same
/// subcommand-reference recording a top-level `widget make hello` call
/// already gets from [`Analyser::record_ensemble_subcommand_invocation`]).
pub(super) struct CollectedHead {
    pub(super) name: String,
    pub(super) span: Span,
    pub(super) argc: Option<usize>,
    pub(super) callback: Option<crate::command_binding::OriginalCallbackPrefix>,
    pub(super) ensemble: Option<(String, Span)>,
}

impl Analyser {
    /// Walk every argument's source slice for ``[cmd ...]``
    /// substitutions and record each nested head as its own
    /// ``CommandInvocation``.  Extracted from
    /// [`Self::process_command`] for readability — without this,
    /// calls embedded inside argument expressions
    /// (``set x [helper $foo]``, ``puts "got [count $items]"``,
    /// ``if { [HTTP::uri] eq "/foo" }``) aren't tracked, which
    /// breaks workspace usage counts, find-references, rename,
    /// and call-hierarchy.
    pub(super) fn record_nested_invocations_from_args(
        &mut self,
        arg_tokens_in: &[Token],
        scope_path: &[usize],
    ) {
        // Which arguments are *expressions*?  A `[...]` inside a braced
        // expr arg is a real invocation (`if {[acl_ok]} …`), but a `[...]`
        // inside a braced *data* word is literal (`set x {[noeval]}`) — so
        // a braced word is scanned only when it is an `Expr` arg.  A braced
        // *body* arg is covered separately by `analyse_body`.
        let expr_indices: Vec<usize> = arg_tokens_in.first().map_or_default(|head| {
            self.retained_argument_role_assistance(
                head.span.start(),
                arg_tokens_in.get(1..).unwrap_or(&[]),
            )
            .into_iter()
            .filter_map(|(ordinal, role)| (role == ArgRole::Expr).then_some(ordinal))
            .collect()
        });
        // A command whose *name* is itself a substitution (`[x] hi`):
        // descend a `Cmd` head too, since every token including the
        // head can hold a nested invocation (the head's *name* is
        // recorded separately by `process_command`; this records what
        // it substitutes).
        if let Some(head) = arg_tokens_in.first()
            && head.kind == TokenType::Cmd
        {
            self.record_invocations_from_cmd_token(*head, scope_path);
        }
        for (i, arg_tok) in arg_tokens_in.iter().enumerate().skip(1) {
            let arg_start = arg_tok.span.start();
            let arg_end = arg_tok.span.end() as usize;
            let src_len = self.source.len();
            if arg_start as usize >= src_len || arg_end > src_len {
                continue;
            }
            if arg_tok.kind == TokenType::Cmd {
                self.record_invocations_from_cmd_token(*arg_tok, scope_path);
            } else if arg_tok.kind == TokenType::Str {
                // Braced word: scan its `[...]` substitutions only when it
                // is an expression argument (substitutions are then active);
                // a braced data word stays opaque (a non-expr braced word
                // is never walked as a script).
                if expr_indices.contains(&(i - 1)) {
                    self.record_invocations_from_expr_token(*arg_tok, scope_path);
                }
            } else if self
                .cmd_fragments(*arg_tok, self.file_lexer_config())
                .iter()
                .all(|fragment| fragment.kind == TokenType::Cmd)
            {
                // `Esc` (bareword / quoted) holding `[…]` substitutions: walk
                // each one as a bare substitution is walked, so a body word of
                // a nested command (`puts "[catch {thread::id} m]"`) is
                // reached too, not only the substitution's head.
                self.record_invocations_from_cmd_token(*arg_tok, scope_path);
            } else {
                // `Esc` (bareword / quoted): substitutions are active, so
                // scan.  Clone the slice into an owned ``String`` so the
                // helper can take ``&mut self`` without conflicting with the
                // source borrow.
                let Some(arg_src) =
                    Analyser::source_slice(&self.source, arg_start as usize, arg_end)
                        .map(str::to_owned)
                else {
                    continue;
                };
                self.record_invocations_from_word_token(*arg_tok, &arg_src, arg_start, scope_path);
            }
        }
    }

    /// Inner: ``Cmd`` (``[…]``) substitution tokens.  Descend the
    /// substitution into a child CST ([`descend_token`]), segment it,
    /// and record *every* inner command's bareword head, recursing
    /// into nested ``[...]``.
    ///
    /// A flat [`first_command_head`] scan would record only the
    /// *first* head of each ``[...]``, dropping ``;``- / newline-
    /// separated commands (`[foo; bar]` → only `foo`); the CST
    /// descent finds them all.
    fn record_invocations_from_cmd_token(&mut self, arg_tok: Token, scope_path: &[usize]) {
        let config = self.file_lexer_config();
        // Collect the inner heads first (this borrows `self.source`
        // through the `SourceMap`); resolve + push afterwards so the
        // immutable source borrow has ended.
        let (heads, expr_toks) = {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            let mut heads: Vec<CollectedHead> = Vec::new();
            let mut expr_toks: Vec<Token> = Vec::new();
            // `arg_tok` is the *merged* argv token.  For a compound word
            // whose first fragment is a `[…]` substitution (`[foo]bar`,
            // `[foo]$x`, `[foo]bar[baz]`), `segments_from_tree` widens the
            // span from the first fragment's start to the *last* fragment's
            // end.  Descending that merged span would re-lex the trailing
            // literal as a script and record a bogus head (`[foo]bar` →
            // `foo]bar`).  Descend each `[…]` fragment instead, walking the
            // unmerged token stream; a single-fragment `[…]` word yields
            // just itself.
            for frag in self.cmd_fragments(arg_tok, config) {
                collect_substitution_heads(
                    &sm,
                    &self.result,
                    frag,
                    config,
                    &mut heads,
                    &mut expr_toks,
                );
            }
            (heads, expr_toks)
        };
        self.push_collected_heads(heads, scope_path);
        // Math-function applications inside any nested `[expr {…}]` dispatch
        // to `::tcl::mathfunc::<fn>` — recorded here (with `&mut self`) because
        // the free-function collection can't resolve that namespace.
        for expr_tok in expr_toks {
            self.record_expr_function_invocations(expr_tok, scope_path);
        }
    }

    /// Recognise the tcllib `namespace eval $ns [list namespace unknown
    /// $handler]` idiom: the ``[...]`` body is a
    /// `Cmd` token, so [`Self::analyse_body`]'s literal-`{...}`-only gate
    /// never walks it as a script, and the generic nested-substitution
    /// scan resolves the segment's head to `list` (never dispatching
    /// `AnalyserHookId::NamespaceUnknown`) — so the handler installation
    /// is invisible to every existing path. Narrowly recognises the
    /// exact `list namespace unknown ?HANDLER?` shape and, on a match,
    /// calls [`Self::handle_namespace_unknown_command`] unmodified with
    /// the quoted command's `["unknown", HANDLER?]` args slice, reusing its
    /// established empty/query-form gating rather than reimplementing
    /// it. Both halves are registry facts: the `list` build is the command
    /// carrying `BUILDS_COMMAND_PREFIX`, and the quoted command is the one
    /// the `NamespaceUnknown` hook is stamped on.
    ///
    /// Deliberately narrow: does not recognise the same idiom built via
    /// `concat`, `format`, `linsert`, string concatenation, or a
    /// `list`-building helper proc — a documented scope boundary, not
    /// an oversight (no attested real-world instance of those forms).
    pub(in super::super) fn detect_list_wrapped_namespace_unknown(&mut self, body_tok: Token) {
        if body_tok.kind != TokenType::Cmd {
            return;
        }
        let config = self.file_lexer_config();
        // Collect the descended segments first (this borrows
        // `self.source` through the `SourceMap`); call the `&mut self`
        // handler afterwards, once the immutable borrow has ended — the
        // same two-phase shape as `record_invocations_from_cmd_token`.
        let segs: Vec<SegmentedCommand> = {
            let sm = Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            let mut segs = Vec::new();
            for frag in self.cmd_fragments(body_tok, config) {
                if frag.kind != TokenType::Cmd || sm.token_text(frag).is_empty() {
                    continue;
                }
                let descended = descend_token(&sm, frag, config);
                segs.extend(segments_from_tree(descended.tree(), &sm));
            }
            segs
        };
        let Some(input) = self
            .resolved_input
            .as_ref()
            .or(self.result.resolved_input.as_ref())
            .cloned()
        else {
            return;
        };
        let context = input.context_registry();
        for seg in &segs {
            // `list HEAD word …` quotes the command it builds — the
            // registry's `BUILDS_COMMAND_PREFIX` reading — and that command
            // installs the handler when it resolves to the hook `namespace
            // unknown` is stamped with: no spelling is compared here.
            let Some(words) = crate::registry_invocation::source_structure::source_produced_command_prefix_words_in(
                &self.source,
                &input,
                &self.head_identities,
                seg,
            ) else {
                continue;
            };
            let Some(quoted) = crate::script_arg::original_list_built_script_command(&words, seg)
            else {
                continue;
            };
            if !(2..=3).contains(&quoted.texts.len())
                || words.with_source_schema(&context, |schema| schema.semantics.analyser_hook)
                    != Some(Some(tcl_registry::hooks::AnalyserHookId::NamespaceUnknown))
            {
                continue;
            }
            self.handle_namespace_unknown_command(quoted.args());
        }
    }

    /// The `[…]` substitution fragment tokens of a (possibly compound)
    /// `Cmd`-headed word, with absolute spans.  Re-lexing the word slice
    /// recovers the per-fragment boundaries the argv merge erased, so
    /// `[foo]bar` yields its `[foo]` fragment (not the whole word) and
    /// `[foo]bar[baz]` yields both `[foo]` and `[baz]`.  On a lex error or
    /// a degenerate empty/out-of-bounds span, falls back to the token as
    /// given so the caller still descends *something*.
    pub(super) fn cmd_fragments(&self, arg_tok: Token, config: LexerConfig) -> Vec<Token> {
        let start = arg_tok.span.start() as usize;
        let end = arg_tok.span.end() as usize;
        if start >= end {
            return vec![arg_tok];
        }
        let Some(word_src) = Analyser::source_slice(&self.source, start, end) else {
            return vec![arg_tok];
        };
        let base = arg_tok.span.start();
        let frags: Vec<Token> = Lexer::with_source_map(SourceMap::new(word_src), config.nested())
            .tokenise_all()
            .map_or_default(|toks| {
                toks.into_iter()
                    .filter(|t| t.kind == TokenType::Cmd)
                    .map(|t| Token {
                        kind: t.kind,
                        span: Span::new(t.span.start() + base, t.span.end() + base),
                        content_offset: t.content_offset,
                        in_quote: t.in_quote,
                    })
                    .collect()
            });
        if frags.is_empty() {
            vec![arg_tok]
        } else {
            frags
        }
    }

    /// Inner: a braced ``Str`` *expression* argument (`if {[acl_ok]} …`).
    /// Record the command substitutions inside the expression — the
    /// expression's own operands are not commands (see
    /// [`collect_expr_substitutions`]).  Skips the over-recording the
    /// generic word scanner would do on a braced *data* word.
    fn record_invocations_from_expr_token(&mut self, expr_tok: Token, scope_path: &[usize]) {
        let config = self.file_lexer_config();
        let (heads, expr_toks) = {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            let mut heads: Vec<CollectedHead> = Vec::new();
            let mut expr_toks: Vec<Token> = Vec::new();
            collect_expr_substitutions(
                &sm,
                &self.result,
                expr_tok,
                config,
                &mut heads,
                &mut expr_toks,
            );
            (heads, expr_toks)
        };
        self.push_collected_heads(heads, scope_path);
        // This expr's own math functions, plus any nested inside a `[expr {…}]`
        // substitution the collection surfaced (`if {[expr {Pi()}]}`).
        self.record_expr_function_invocations(expr_tok, scope_path);
        for nested in expr_toks {
            self.record_expr_function_invocations(nested, scope_path);
        }
    }

    /// Every math-function application inside the expression `expr_tok`, as
    /// `(name, name_span, arg_count)` with the function-name span mapped to
    /// absolute source coordinates.  The single extraction the invocation
    /// recorder and the dialect-availability diagnostic both read.
    ///
    /// The expression body starts past the opening delimiter; the AST's
    /// offsets are relative to the *trimmed* text, so each is translated back
    /// through the leading-whitespace trim to a source span.
    pub(in crate::analyser) fn expr_function_calls(
        &self,
        expr_tok: Token,
    ) -> Vec<(String, Span, usize)> {
        if !self.result.allows_lexical_declaration_advice() {
            let generation = self.analysis_context();
            let registry = generation.commands();
            let image = self.command_document_source_image();
            return self
                .head_identities
                .original_math_functions_in_source(
                    registry,
                    &image,
                    self.file_lexer_config(),
                    expr_tok.span,
                )
                .into_iter()
                .map(|call| {
                    (
                        call.function().to_owned(),
                        call.span(),
                        call.argument_count(),
                    )
                })
                .collect();
        }
        let content_start = expr_tok.span.start() + u32::from(expr_tok.content_offset);
        let (start, end) = (content_start as usize, expr_tok.span.end() as usize);
        let Some(expr_text) = Analyser::source_slice(&self.source, start, end) else {
            return Vec::new();
        };
        let trimmed = expr_text.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let trim_base = u32::try_from(expr_text.len() - expr_text.trim_start().len()).unwrap_or(0);
        let Some(context) = self.original_expression_parser_context() else {
            return Vec::new();
        };
        let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(trimmed, &context);
        parsed
            .function_calls()
            .into_iter()
            .map(|(name, rel_start, argc)| {
                let name_start = content_start + trim_base + rel_start;
                let span = Span::new(
                    name_start,
                    name_start + u32::try_from(name.len()).unwrap_or(0),
                );
                (name.to_owned(), span, argc)
            })
            .collect()
    }

    /// Record each math-function application (`sin($x)`, `max($a, $b)`) as an
    /// invocation of the command it dispatches to, `::tcl::mathfunc::<name>`.
    /// A user who defines `proc ::tcl::mathfunc::myfunc { … }` — or, per TIP
    /// 232, a namespace-local `proc ::ns::tcl::mathfunc::myfunc` that shadows
    /// it inside `::ns` — then gets go-to-definition, references, rename, and
    /// arity checking on `myfunc(...)` calls, and the function is no longer
    /// reported unused.
    ///
    /// The written head is the bare function word (`sin`); the resolved name
    /// is the *local-first* candidate `{ns}::tcl::mathfunc::sin` for the
    /// call's own namespace, computed the same way an ordinary bareword
    /// command's walk-time guess is ([`Self::resolve_command_qualified_name`])
    /// so a rename rewrites only the tail token in the expression.
    /// [`Self::finalise_invocation_resolutions`] settles it against the real
    /// two-candidate rule (the caller's `tcl::mathfunc`, else the global
    /// one) — never the generic one-hop `{ns}::{name}` suffix-strip it uses
    /// for an ordinary call, which would misparse the fixed `tcl::mathfunc`
    /// dispatch segment as if it were the calling namespace and could
    /// mis-resolve to an unrelated global command sharing the bare tail name.
    fn record_expr_function_invocations(&mut self, expr_tok: Token, scope_path: &[usize]) {
        for (name, span, argc) in self.expr_function_calls(expr_tok) {
            let resolved = self.resolve_command_qualified_name(
                &crate::naming::unrooted_member_key(
                    tcl_registry::mathfunc::MATHFUNC_NAMESPACE_RELATIVE,
                    &name,
                ),
                scope_path,
            );
            self.push_mathfunc_command_reference(name, span, resolved, Some(argc));
        }
    }

    /// Resolve each collected `(name, span, argc)` head to a qualified name and
    /// push it as a `command_invocations` entry.  `argc` carries the nested call's
    /// statically-known argument count (`None` when `{*}`-expanded), so a wrong-arg
    /// call to a cross-file proc *inside a substitution* (`set x [helper a b c]`)
    /// still draws the cross-file arity error.
    fn push_collected_heads(&mut self, heads: Vec<CollectedHead>, scope_path: &[usize]) {
        for CollectedHead {
            name,
            span: range,
            argc,
            callback,
            ensemble: sub_candidate,
        } in heads
        {
            let original_lookup = callback
                .as_ref()
                .and_then(|prefix| prefix.lookup().cloned());
            let original_name_input = callback.as_ref().map(|prefix| prefix.name_input().clone());
            let callback_arity = callback
                .as_ref()
                .and_then(crate::command_binding::OriginalCallbackPrefix::appended_arity);
            let callback_baked_args = callback.as_ref().map_or(
                0,
                crate::command_binding::OriginalCallbackPrefix::baked_argument_count,
            );
            let lookup = if callback.is_some() {
                crate::signature_scan::types::SignatureCommandLookup::DeferredReference
            } else {
                crate::signature_scan::types::SignatureCommandLookup::InvocationHead
            };
            let rename_safe = callback.is_none() || original_lookup.is_some();
            let resolved = self.resolve_command_qualified_name(&name, scope_path);
            // `[<ensemble> <subcommand> …]` nested inside a substitution —
            // the same existence-probed subcommand reference a top-level
            // `<ensemble> <subcommand> …` call already gets from
            // `record_ensemble_subcommand_invocation`.
            // `argc` here is "args after the head" (the subcommand word
            // included), so it shifts by one to become "args after the
            // subcommand word" — the same convention that function uses.
            if let Some((sub, sub_span)) = sub_candidate {
                let sub_argc = argc.map(|a| a.saturating_sub(1));
                // A callback prefix dispatches later, from no point this
                // walk can name.
                let head = callback.is_none().then_some(range.start());
                self.record_or_defer_ensemble_subcommand(&resolved, &sub, sub_span, head, sub_argc);
            }
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    original_callback_signature_lookup: None,
                    original_callback_prefix: callback.clone().map(std::sync::Arc::new),
                    original_lookup,
                    original_name_input,
                    lookup,
                    name,
                    range,
                    resolved_qualified_name: Some(resolved),
                    resolved_user_definition: false,
                    resolved_definition: None,
                    resolved_command_reference: None,
                    resolution_candidates: Vec::new(),
                    argc,
                    callback_arity,
                    callback_baked_args,
                    indirect: false,
                    rename_safe,
                    existence_probe: false,
                    is_mathfunc_call: false,
                    ensemble_dispatch: None,
                },
            );
        }
    }

    /// Inner: ``Esc`` (bareword / quoted) and ``Str`` (braced)
    /// tokens.  For ``Str``, strip the surrounding ``{…}`` via
    /// ``content_offset`` first — otherwise the scanner sees the
    /// outer ``{`` and skips the entire braced region opaquely,
    /// missing every nested ``[cmd]`` inside braced expr args.
    fn record_invocations_from_word_token(
        &mut self,
        arg_tok: Token,
        arg_src: &str,
        arg_start: u32,
        scope_path: &[usize],
    ) {
        let (inner_src, inner_base) = if matches!(arg_tok.kind, TokenType::Str) {
            let inner_off = arg_tok.content_offset as usize;
            if inner_off <= arg_src.len() {
                let trimmed = arg_src[inner_off..]
                    .strip_suffix('}')
                    .unwrap_or(&arg_src[inner_off..]);
                let inner_off_u32 = u32::try_from(inner_off)
                    .expect("content_offset fits in u32 for in-memory source");
                (trimmed, arg_start + inner_off_u32)
            } else {
                (arg_src, arg_start)
            }
        } else {
            (arg_src, arg_start)
        };
        for (name, off) in scan_nested_command_heads(inner_src) {
            let abs_start = inner_base + off;
            let abs_end = abs_start
                + u32::try_from(name.len()).expect("token length fits in u32 for in-memory source");
            let resolved = self.resolve_command_qualified_name(&name, scope_path);
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    lookup: crate::signature_scan::types::SignatureCommandLookup::InvocationHead,
                    original_callback_signature_lookup: None,
                    original_callback_prefix: None,
                    original_lookup: None,
                    original_name_input: None,
                    name,
                    range: tcl_lexer::Span::new(abs_start, abs_end),
                    resolved_qualified_name: Some(resolved),
                    resolved_user_definition: false,
                    resolved_definition: None,
                    resolved_command_reference: None,
                    resolution_candidates: Vec::new(),
                    // Nested `[cmd ...]` head, no recorded argument list — arity skip.
                    argc: None,
                    callback_arity: None,
                    callback_baked_args: 0,
                    indirect: false,
                    rename_safe: true,
                    existence_probe: false,
                    is_mathfunc_call: false,
                    ensemble_dispatch: None,
                },
            );
        }
    }
}

/// Descend a ``Cmd`` (``[…]``) substitution token and collect every
/// inner command's head as ``(name, head_span)``, recursing into nested
/// ``[...]`` substitutions and registry-resolved body arguments.
///
/// The token is descended into a child CST ([`descend_token`]) and the
/// inner script segmented; each command is then handled by
/// [`record_command_invocations`].  Spans are absolute (the descent
/// anchors the child tree at the substitution's position).
/// Statically-known argument count of a segmented command for the cross-file
/// arity check: `None` when any argument word is `{*}`-expanded (runtime count
/// unknown), else the literal argument count (`argv` minus the head).
fn segment_argc(seg: &SegmentedCommand) -> Option<usize> {
    if seg
        .expand_word
        .as_ref()
        .is_some_and(|e| e.iter().skip(1).any(|&x| x))
    {
        return None;
    }
    Some(seg.argv.len().saturating_sub(1))
}

/// The segment's first argument word, as an ensemble-subcommand candidate
/// `(text, span)` — `None` when there is no such word, it's `{*}`-expanded
/// (so the runtime subcommand isn't known statically), or it's otherwise a
/// dynamic word (the nested-`[...]` counterpart of the top-level check in
/// `record_ensemble_subcommand_invocation`).
fn ensemble_subcommand_candidate(seg: &SegmentedCommand) -> Option<(String, Span)> {
    let expanded = seg
        .expand_word
        .as_ref()
        .and_then(|e| e.get(1))
        .copied()
        .unwrap_or(false);
    if expanded {
        return None;
    }
    let sub = seg.texts.get(1)?;
    if crate::naming::is_dynamic_word(sub) {
        return None;
    }
    let span = seg.argv.get(1)?.span;
    Some((sub.clone(), span))
}

pub(super) fn collect_substitution_heads(
    sm: &SourceMap<'_>,
    analysis: &super::super::types::AnalysisResult,
    cmd_tok: Token,
    config: LexerConfig,
    out: &mut Vec<CollectedHead>,
    expr_tokens: &mut Vec<Token>,
) {
    if cmd_tok.kind != TokenType::Cmd || sm.token_text(cmd_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, cmd_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        record_command_invocations(sm, analysis, &seg, config, out, expr_tokens);
    }
}

fn nested_source_words(
    sm: &SourceMap<'_>,
    analysis: &super::super::types::AnalysisResult,
    segment: &SegmentedCommand,
) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    crate::registry_invocation::source_structure::source_registry_words(
        sm.source(),
        analysis,
        segment,
    )
}

pub(super) fn nested_source_bodies(
    sm: &SourceMap<'_>,
    analysis: &super::super::types::AnalysisResult,
    segment: &SegmentedCommand,
    config: LexerConfig,
    purpose: crate::registry_invocation::OriginalSourceScriptPurpose,
) -> Vec<SegmentedCommand> {
    let Some(words) = nested_source_words(sm, analysis, segment) else {
        return Vec::new();
    };
    let Some(input) = &analysis.resolved_input else {
        return Vec::new();
    };
    let context = input.context_registry();
    if words.with_source_schema(&context, |schema| {
        schema.authored_source_definition_body_grammar().is_some()
            || schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::DEFINES_PROCEDURE)
    }) != Some(false)
    {
        return Vec::new();
    }
    let mut commands = Vec::new();
    for body in words.source_script_bodies_for(&context, purpose) {
        let span = body.content_span();
        if let Some(text) = sm.source().get(span.as_range()) {
            commands.extend(crate::segmenter::segment_commands_with_offset_and_config(
                text,
                span.start(),
                config.nested(),
            ));
        }
    }
    commands
}

fn collected_original_callback(
    prefix: crate::command_binding::OriginalCallbackPrefix,
) -> Option<CollectedHead> {
    let (name, span) = prefix.reported_source_head()?;
    Some(CollectedHead {
        name: name.to_owned(),
        span,
        argc: None,
        callback: Some(prefix),
        ensemble: None,
    })
}

pub(super) fn nested_source_prefixes(
    sm: &SourceMap<'_>,
    analysis: &super::super::types::AnalysisResult,
    segment: &SegmentedCommand,
    config: LexerConfig,
) -> Vec<CollectedHead> {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let Some(input) = &analysis.resolved_input else {
        return Vec::new();
    };
    let context = input.context_registry();
    let Some(offset) = segment.argv.first().map(|head| head.span.start()) else {
        return Vec::new();
    };
    let mut tokens = crate::ir::CommandTokens::from_segmented(sm, config, segment);
    let Some(realm) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    realm.stamp_original_tokens(&mut tokens);
    if let Some(words) = nested_source_words(sm, analysis, segment) {
        return words.with_source_schema(&context, |schema| {
            let facts = schema.facts();
            let Some(dialect) = schema.words.arguments().dialect() else { return Vec::new(); };
            let mut prefixes = Vec::new();
            for (argument, appended) in schema.authored_source_command_prefix_arguments().unwrap_or_default() {
                let Some(native_input) = words.original_argument_value_input(argument) else { continue; };
                let Some(word) = native_input.original_word_key().map(crate::signature_scan::scope::SignatureSourceNameKey::original_word) else { continue; };
                let original = match argument.checked_add(1).and_then(|ordinal| words.origins().get(ordinal)) {
                    Some(crate::registry_invocation::InvocationWordOrigin::Written(written)) => tokens.source_binding.as_ref()
                        .and_then(|binding| binding.original_callback_prefix_in_context(&tokens, *written, &context)),
                    _ => None,
                };
                let prefix = original.or_else(|| crate::command_binding::OriginalCallbackPrefix::from_original_static_operand(
                    word, &facts, argument, dialect));
                let prefix = prefix.map(|prefix| {
                    crate::registry_invocation::source_structure::source_callback_prefix_at(
                        sm.source(), analysis, offset, &prefix,
                    ).unwrap_or(prefix)
                });
                if let Some(head) = prefix.filter(|prefix| prefix.appended_arity() == Some(appended))
                    .and_then(collected_original_callback) { prefixes.push(head); }
            }
            prefixes
        }).unwrap_or_default();
    }
    let Some(instance) =
        crate::registry_invocation::source_structure::source_registered_instance_words_at(
            sm.source(),
            analysis,
            offset,
        )
    else {
        return Vec::new();
    };
    instance.with_source_schema(&context, |schema| {
        let facts = schema.facts();
        let Some(dialect) = schema.words.arguments().dialect() else { return Vec::new(); };
        schema.authored_source_command_prefix_arguments().unwrap_or_default().into_iter().filter_map(|(argument, appended)| {
            instance.argument_input(argument)?.native_input()?;
            let prefix = crate::command_binding::OriginalCallbackPrefix::from_original_static_operand(
                instance.argument_word(argument)?, &facts, argument, dialect)?;
            (prefix.appended_arity() == Some(appended)).then_some(())?;
            let prefix = crate::registry_invocation::source_structure::source_callback_prefix_at(
                sm.source(), analysis, offset, &prefix,
            ).unwrap_or(prefix);
            collected_original_callback(prefix)
        }).collect()
    }).unwrap_or_default()
}

fn record_command_invocations(
    sm: &SourceMap<'_>,
    analysis: &super::super::types::AnalysisResult,
    seg: &SegmentedCommand,
    config: LexerConfig,
    out: &mut Vec<CollectedHead>,
    expr_tokens: &mut Vec<Token>,
) {
    if let (Some(&head), Some(name)) = (seg.argv.first(), seg.texts.first())
        && !name.is_empty()
    {
        out.push(CollectedHead {
            name: name.clone(),
            span: head.span,
            argc: segment_argc(seg),
            callback: None,
            ensemble: ensemble_subcommand_candidate(seg),
        });
    }
    out.extend(nested_source_prefixes(sm, analysis, seg, config));
    for tok in &seg.all_tokens {
        if tok.kind == TokenType::Cmd {
            collect_substitution_heads(sm, analysis, *tok, config, out, expr_tokens);
        }
    }
    for inner in nested_source_bodies(
        sm,
        analysis,
        seg,
        config,
        crate::registry_invocation::OriginalSourceScriptPurpose::Syntax,
    ) {
        record_command_invocations(sm, analysis, &inner, config, out, expr_tokens);
    }
    let Some(words) = nested_source_words(sm, analysis, seg) else {
        return;
    };
    for (index, role) in words.written_argument_roles() {
        if role == ArgRole::Expr
            && let Some(&tok) = seg.argv.get(index + 1)
        {
            expr_tokens.push(tok);
            collect_expr_substitutions(sm, analysis, tok, config, out, expr_tokens);
        }
    }
}

fn collect_expr_substitutions(
    sm: &SourceMap<'_>,
    analysis: &super::super::types::AnalysisResult,
    expr_tok: Token,
    config: LexerConfig,
    out: &mut Vec<CollectedHead>,
    expr_tokens: &mut Vec<Token>,
) {
    if expr_tok.kind != TokenType::Str || sm.token_text(expr_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, expr_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        for tok in &seg.all_tokens {
            if tok.kind == TokenType::Cmd {
                collect_substitution_heads(sm, analysis, *tok, config, out, expr_tokens);
            }
        }
    }
}

/// Walk `text` looking for ``[cmd args...]`` command
/// substitutions and return the head name + the offset of the
/// head's first byte within `text` for each substitution found.
/// Nested substitutions are reported in depth-first order
/// (outer first, then inner).  Braced regions are skipped
/// opaquely; backslash-escaped ``\\[`` / ``\\]`` are skipped.
///
/// Returns ``(name, byte_offset_in_text)`` pairs.  The caller
/// adds the enclosing token's source-span start to obtain an
/// absolute offset.
pub(crate) fn scan_nested_command_heads(text: &str) -> Vec<(String, u32)> {
    // Entry point: the outermost word's raw text is bracket-nesting depth 0
    // (the recursion cap lives in [`scan_nested_command_heads_at`]).
    scan_nested_command_heads_at(text, 0)
}

fn scan_nested_command_heads_at(text: &str, rec_depth: u32) -> Vec<(String, u32)> {
    // Native-stack safety net: this self-recurses once per
    // nested `[…]` substitution inside a single word's raw text — a genuinely
    // unbounded axis. Past the cap, return what's been found so far: nested
    // heads buried deeper than the cap go unreported, never a crash.
    // (`rec_depth` is named apart from the local `[`/`{`-matching `depth`
    // counters below, which track bracket balance, not native recursion.)
    if MAX_BRACKET_TEXT_DEPTH.exceeded(rec_depth) {
        return Vec::new();
    }
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                // Skip braced region opaquely.
                let mut depth = 1i32;
                i += 1;
                while i < bytes.len() && depth > 0 {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b'\\' if i + 1 < bytes.len() => i += 1,
                        _ => {}
                    }
                    i += 1;
                }
            }
            b'[' => {
                // Find the matching closing ``]`` honouring
                // nesting + backslash escapes.
                let inner_start = i + 1;
                let mut depth = 1i32;
                let mut j = inner_start;
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        b'[' => depth += 1,
                        b']' => depth -= 1,
                        b'\\' if j + 1 < bytes.len() => j += 1,
                        _ => {}
                    }
                    if depth == 0 {
                        break;
                    }
                    j += 1;
                }
                if depth == 0 && j < bytes.len() {
                    let inner = &text[inner_start..j];
                    let inner_start_u32 = u32::try_from(inner_start)
                        .expect("byte offset fits in u32 for in-memory source");
                    if let Some((name, head_offset_in_inner)) = first_command_head(inner) {
                        let abs_offset = inner_start_u32
                            + u32::try_from(head_offset_in_inner)
                                .expect("inner offset fits in u32 for in-memory source");
                        out.push((name.to_string(), abs_offset));
                    }
                    // Recurse into the inner text — nested ``[...]``
                    // substitutions inside this one also produce
                    // invocations.
                    for (name, off_in_inner) in scan_nested_command_heads_at(inner, rec_depth + 1) {
                        out.push((name, inner_start_u32 + off_in_inner));
                    }
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

/// Find the first command-head token in `text` (skipping
/// leading whitespace and comments) and return its ``(name,
/// offset)`` pair.  Conservative — any non-bareword leading
/// token returns ``None``.
fn first_command_head(text: &str) -> Option<(&str, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b';' {
            i += 1;
            continue;
        }
        // Skip comment lines if at the start of a logical command.
        if c == b'#' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        break;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' '
            || c == b'\t'
            || c == b'\n'
            || c == b'\r'
            || c == b';'
            || c == b'['
            || c == b']'
        {
            break;
        }
        i += 1;
    }
    if i == start {
        return None;
    }
    let head = &text[start..i];
    // Reject heads that look like substitution / quoting markers
    // (``$foo``, ``"abc"``, ``{...}``) — these aren't command
    // names.
    if head.starts_with('$') || head.starts_with('"') || head.starts_with('{') {
        return None;
    }
    Some((head, start))
}
