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

//! Command heads that are not literal: folded head candidates, variable and command-substitution heads and bareword self-dispatch.

use super::super::state::Analyser;
use super::{BarewordDispatch, VarOrCmdSite};
use tcl_lexer::{Token, TokenType};

impl Analyser {
    /// The command head this call actually dispatches to, folding a
    /// statically-determined dynamic head to the name it names.
    ///
    /// A head written with a substitution is not automatically a runtime
    /// unknown: `set ns ticklecharts; ${ns}::setdef …` dispatches to
    /// `::ticklecharts::setdef` on every execution, and the `TclOO` idiom
    /// `set ns [namespace qualifiers [self class]]; ${ns}::setdef …` is the
    /// same shape one level removed (without the fold, go-to-definition,
    /// find-references, and call-hierarchy silently drop every such call
    /// site).  [`Analyser::resolve_dynamic_word`] does the folding
    /// through the *dominating* constant lattice, so a branch-conditional
    /// or otherwise unprovable binding still abstains and the written text
    /// is kept unchanged — the written text never resolves to anything, so
    /// abstaining is exactly the previous behaviour.
    ///
    /// A folded head that is still dynamic (a partially-resolved
    /// interpolation) is discarded rather than half-applied.
    ///
    /// A head that is a **whole-word** variable read (`$cmd`) is left alone:
    /// that shape belongs to the flow-sensitive const-dispatch engine
    /// (`pending_const_dispatches` ->
    /// [`super::super::diagnostics::const_dispatch`]), which settles it from the
    /// CFG/SSA value model rather than the walk's lexical map and marks the
    /// resulting invocation `indirect`.  Only the composite shapes that
    /// engine explicitly skips are folded here.
    /// The full command-resolution candidate list for a **folded** dynamic
    /// head, or an empty list when the head was written literally (the
    /// ordinary case, which `finalise_invocation_resolutions` rebuilds for
    /// itself).  See the call site for why the folded case cannot be
    /// rebuilt there.
    pub(super) fn folded_head_candidates(
        &self,
        head: &str,
        folded: bool,
        scope_path: &[usize],
    ) -> Vec<String> {
        if !folded {
            return Vec::new();
        }
        let ns = self.command_resolution_namespace(scope_path);
        let path: Vec<&str> = self
            .namespace_paths
            .get(&ns)
            .map_or_else(Vec::new, |p| p.iter().map(String::as_str).collect());
        crate::naming::command_resolution_candidates(&ns, &path, head)
    }

    pub(super) fn resolve_dynamic_command_head<'w>(
        &self,
        cmd_name: &'w str,
        cmd_tok: Token,
        head_is_single_token: bool,
        scope_path: &[usize],
    ) -> std::borrow::Cow<'w, str> {
        if !crate::naming::is_dynamic_word(cmd_name) || self.head_is_whole_word_variable(cmd_tok) {
            return std::borrow::Cow::Borrowed(cmd_name);
        }
        self.resolve_dynamic_word(cmd_name, Some(cmd_tok), head_is_single_token, scope_path)
            .filter(|folded| !crate::naming::is_dynamic_word(folded))
            .map_or(
                std::borrow::Cow::Borrowed(cmd_name),
                std::borrow::Cow::Owned,
            )
    }

    /// Whole variable-read geometry comes from the actual original head and
    /// its configured executable-part owner. Braced inert source, mixed data,
    /// expansion and unavailable original words are not whole variable reads.
    pub(super) fn head_is_whole_word_variable(&self, cmd_tok: Token) -> bool {
        self.original_whole_variable_source_word(cmd_tok, true)
            .is_some()
    }

    pub(super) fn original_whole_variable_source_word(
        &self,
        token: Token,
        head: bool,
    ) -> Option<tcl_lexer::NativeWord> {
        if token.kind != TokenType::Var {
            return None;
        }
        let image = self.command_document_source_image();
        let config = self.file_lexer_config();
        let word = if head {
            self.head_identities
                .original_written_head_at_span_in_source(&image, token.span, config)?
                .0
        } else {
            self.head_identities
                .original_written_word_at_span_in_source(&image, token.span, config)?
                .0
        };
        if word.group().expand || word.group().kind == tcl_lexer::WordKind::Braced {
            return None;
        }
        let arena = word.executable_parts();
        let [part] = arena.list(arena.root()) else {
            return None;
        };
        if !matches!(part.part, tcl_lexer::ExecutablePart::Variable { .. })
            || part.span != word.content_span().ok()?
            || !tcl_syntax::naming::is_whole_variable_reference(
                std::str::from_utf8(arena.bytes(part.span)?).ok()?,
                word.config(),
            )
        {
            return None;
        }
        Some(word)
    }

    /// Record this command as a variable-as-command (``$obj
    /// method ...``) or command-substitution-as-command
    /// (``[expr ...] args``) call site so the post-walk W307 /
    /// W308 emitters can resolve them.
    ///
    /// # Dispatch-site shapes
    ///
    /// Four command-head shapes name an object to dispatch on, and every
    /// one of them is recognised **structurally or from the registry** —
    /// never by spelling a command name here:
    ///
    /// | Written head | Token kind | Recorded as |
    /// |---|---|---|
    /// | `$obj method` | [`TokenType::Var`] | [`DispatchReceiver::Variable`] |
    /// | `[Dog new] method` | [`TokenType::Cmd`] | a [`CmdCommandSite`] |
    /// | `objcmd method` (from `CLASS create objcmd`) | [`TokenType::Esc`] | [`DispatchReceiver::InstanceCommand`] |
    /// | `my method` | [`TokenType::Esc`] | [`DispatchReceiver::SelfDispatch`] |
    ///
    /// The last two share the bareword arm and are told apart by
    /// [`Self::record_bareword_dispatch_site`]: a registry-declared
    /// self-dispatch keyword first, the `CLASS create NAME` binding
    /// otherwise.
    ///
    /// # Span conventions
    ///
    /// Both spans a site carries are the *whole written word*, so a
    /// diagnostic anchored on either underlines exactly what the user
    /// typed. That takes deliberate work in both directions, because a
    /// token's raw span is neither:
    ///
    /// * `method_span` **trims the opening delimiter** — `content_offset`
    ///   bytes of `{` / `"` / `[` — so `{badmethod}` anchors on
    ///   `badmethod`.
    /// * `cmd_span` **adds the closing delimiter back**, via
    ///   [`tcl_lexer::word_span`], because a `Cmd` / `Str` token's span
    ///   stops at the end of its content: the raw span of `[Dog new]` is
    ///   `[Dog new`.
    ///
    /// [`CmdCommandSite`]: super::super::state::CmdCommandSite
    /// [`DispatchReceiver`]: super::super::state::DispatchReceiver
    pub(super) fn record_var_or_cmd_command_site(&mut self, site: VarOrCmdSite<'_>) {
        let VarOrCmdSite {
            cmd_name,
            cmd_tok,
            head_expanded,
            args,
            arg_tokens,
            arg_expand,
            scope_path,
        } = site;
        let in_method = self.scope_path_in_method_body(scope_path);
        // Content span of the method word (delimiters trimmed) — the tight
        // W308 anchor and "did you mean" fix target.
        let method_span = arg_tokens.first().map(|t| {
            tcl_lexer::Span::new(t.span.start() + u32::from(t.content_offset), t.span.end())
        });
        match cmd_tok.kind {
            TokenType::Var => {
                let sm = Analyser::source_map(
                    &self.source,
                    &self.cached_line_index,
                    self.cached_line_index_source_len,
                );
                // A composite head whose first token is a *braced* variable
                // (`${ns}::define::[…]`) merges into one Var word token, so the
                // raw text spans the whole word.  The dispatched variable is
                // only the braced name (`${ns}` → `ns`); the closer ends it and
                // the rest is a literal / substituted suffix.  Where that
                // closer sits is the release's `Tcl_ParseVarName` rule, so
                // [`Analyser::split_braced_head`] asks the shared owner rather
                // than truncating at the first `}` (a simple `$obj` or
                // namespaced `$ns::v` head has no closer at all and is
                // unchanged).
                let raw = sm.token_text(cmd_tok);
                let var_name = self.split_braced_head(raw).0.to_string();
                let method_name = args.first().cloned();
                // A simple-`$cmd` head may be a statically-known
                // dispatch.  Record the *site* for settlement in the
                // CFG/SSA phase, where the flow-sensitive value model can
                // prove (or soundly refuse to prove) the finite set of
                // command names reaching this exact program point —
                // never the walk's lexical constant map, whose
                // last-write-wins view collapses `if`/loop joins.  A
                // braced composite head (`${ns}::tail …`) is the W307
                // ensemble shape, not a whole-command variable, so it is
                // skipped.
                if var_name == raw {
                    let ns = self.command_resolution_namespace(scope_path);
                    self.pending_const_dispatches
                        .push(super::super::state::ConstDispatchSite {
                            var_name: var_name.clone(),
                            span: cmd_tok.span,
                            ns,
                            head_expanded,
                        });
                }
                self.var_command_sites
                    .push(super::super::state::VarCommandSite {
                        var_name,
                        method_name,
                        method_span,
                        // Whole written word, so a braced `${obj}` head anchors
                        // on `${obj}` rather than `${obj` — see this function's
                        // "Span conventions".
                        cmd_span: tcl_lexer::word_span(&sm, cmd_tok),
                        in_method,
                        argc: args.len(),
                        has_expand: arg_expand.iter().any(|&e| e),
                        receiver: super::super::state::DispatchReceiver::Variable,
                    });
            }
            TokenType::Cmd => {
                let sm = Analyser::source_map(
                    &self.source,
                    &self.cached_line_index,
                    self.cached_line_index_source_len,
                );
                let cmd_text = sm.token_text(cmd_tok).to_string();
                let base = cmd_tok.span.start() + u32::from(cmd_tok.content_offset);
                let position = sm.position_at(base);
                let config = self.file_lexer_config();
                let map = tcl_lexer::SourceMap::new(&cmd_text).with_base(
                    base,
                    position.line,
                    position.character.get(),
                );
                let bindings = self.head_identities.source_bindings_ref();
                let commands = crate::segmenter::segment_commands_with_offset_and_config(
                    &cmd_text,
                    base,
                    config.nested(),
                )
                .iter()
                .map(|command| {
                    let mut tokens =
                        crate::ir::CommandTokens::from_segmented(&map, config, command);
                    bindings.stamp_original_tokens(&mut tokens);
                    tokens
                })
                .collect();
                let method_name = args.first().cloned();
                self.cmd_command_sites
                    .push(super::super::state::CmdCommandSite {
                        cmd_text,
                        commands,
                        method_name,
                        method_span,
                        cmd_span: tcl_lexer::word_span(&sm, cmd_tok),
                        in_method,
                    });
            }
            TokenType::Esc => self.record_bareword_dispatch_site(BarewordDispatch {
                cmd_name,
                cmd_tok,
                args,
                method_span,
                in_method,
                arg_expand,
            }),
            _ => {}
        }
    }

    /// Capture readonly bareword dispatch sites with their whole head geometry.
    /// Registry self-dispatch vocabulary supplies a possible source shape;
    /// diagnostics retain their independent positioned member/receiver owner.
    /// Named receivers require the original source-instance issuer rather than
    /// created-command or class-report maps. Deferred scans keep genuine sites
    /// until the canonical original declarations are available after grafting.
    ///
    /// `cmd_name` is the already-extracted reporting head. It never replaces
    /// the original command token or supplies a Native receiver allocation,
    /// current method table, successful dispatch or entered frame.
    pub(super) fn record_bareword_dispatch_site(&mut self, d: BarewordDispatch<'_>) {
        let BarewordDispatch {
            cmd_name,
            cmd_tok,
            args,
            method_span,
            in_method,
            arg_expand,
        } = d;
        let site = |receiver| super::super::state::VarCommandSite {
            var_name: cmd_name.to_string(),
            method_name: args.first().cloned(),
            method_span,
            // A bareword head has no closing delimiter, so its token span
            // already is the whole word.
            cmd_span: cmd_tok.span,
            in_method,
            argc: args.len(),
            has_expand: arg_expand.iter().any(|&e| e),
            receiver,
        };
        if self.head_is_self_dispatch_keyword(cmd_name) {
            // Recorded whether or not a class encloses this offset: the walk
            // does not yet know (an isolated per-item body has no classes at
            // all), and abstaining is the diagnosis-time job of
            // `enclosing_class_at_offset` returning `None`.
            self.var_command_sites
                .push(site(super::super::state::DispatchReceiver::SelfDispatch));
            return;
        }
        self.record_bareword_instance_dispatch_site(site);
    }

    /// Whether `head`, written bare in command position, is a
    /// registry-declared `TclOO` self-dispatch keyword whose next word
    /// therefore names a method on the enclosing object.
    ///
    /// Registry-first and dialect-aware: `method_dispatch_keyword` answers
    /// at the active profile's point, so a `tcl8.4` /
    /// `tcl8.5` document — with no `TclOO` at all — answers `false` for
    /// every spelling, and it resolves the `::`-qualified form itself.
    ///
    /// Purely a *shape* question, deliberately: whether this document goes
    /// on to rename, alias, delete or shadow the keyword is order-dependent
    /// and cannot be settled mid-walk, so it is asked once, post-walk, by
    /// `Analyser::self_dispatch_keyword_disturbed`.
    pub(super) fn head_is_self_dispatch_keyword(&self, head: &str) -> bool {
        let generation = self.analysis_context();
        generation
            .context()
            .resolve_spec(generation.commands(), head)
            .is_some_and(|spec| {
                spec.traits
                    .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
            })
    }

    /// Named receiver inventory comes from the original source-instance issuer.
    /// Deferred scans retain complete source sites until canonical declarations
    /// join; reporting labels cannot admit a receiver or promise its lifetime.
    fn record_bareword_instance_dispatch_site(
        &mut self,
        site: impl Fn(super::super::state::DispatchReceiver) -> super::super::state::VarCommandSite,
    ) {
        let site = site(super::super::state::DispatchReceiver::InstanceCommand);
        if let Some(pending) = self.pending_bareword_dispatch_sites.as_mut() {
            pending.push(site);
        } else if crate::registry_invocation::source_structure::source_class_instance_words_at(
            &self.source,
            &self.result,
            site.cmd_span.start(),
        )
        .is_some()
        {
            self.var_command_sites.push(site);
        }
    }
}
