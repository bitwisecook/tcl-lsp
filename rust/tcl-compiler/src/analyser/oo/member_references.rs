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

//! The command references a member names, and the members a literal loop installs one per element.

use super::super::state::Analyser;
use super::super::types::{ClassDef, MethodDef};
use super::{MemberLanding, MemberStatement, default_visibility, unwrap_wrapper_member};
use crate::ir::MethodKind;
use tcl_dialect::model::SurfaceQuery;
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::{DefinitionBodyGrammar, MemberRefKind};

/// The `(var_idx, list_idx, body_idx)` post-head argument positions of a
/// registry-declared "var/list pairs then a trailing body" loop command
/// (`foreach`/`lmap`), when it has **exactly one** var/list pair — the only
/// shape [`Analyser::record_loop_installed_members`] can read a member name
/// off of. `None` for anything else: no [`tcl_registry::Traits::LOOP_LIST_HEADER`]
/// trait, no single matching [`tcl_registry::repeated::RepeatedArgLayout`],
/// or more (or fewer) than one pair — a multi-list `foreach` has no single
/// variable a member's name word could unambiguously mean.
pub(in super::super) fn loop_installer_pair(
    spec: &tcl_registry::CommandSpec,
    post_head: usize,
) -> Option<(usize, usize, usize)> {
    if !spec.traits.contains(tcl_registry::Traits::LOOP_LIST_HEADER) {
        return None;
    }
    let [layout] = spec.repeated_args else {
        return None;
    };
    if layout.role != ArgRole::LoopVarList || layout.stride != 2 || layout.exclude_trailing != 1 {
        return None;
    }
    let var_indices = layout.indices(post_head);
    let &[var_idx] = var_indices.as_slice() else {
        return None;
    };
    Some((var_idx, var_idx + 1, post_head - 1))
}

/// Split a loop's literal list word into `(element, absolute source span)`
/// pairs, or `None` when the word is not literal (a single brace/quote/bare
/// token with no substitutions — same predicate the parameter-list literal
/// check uses) or fails to parse as a Tcl list.
fn literal_loop_elements(
    list_word: &str,
    list_tok: Token,
    list_single_token: bool,
) -> Option<Vec<(String, Span)>> {
    if !crate::signature_scan::params::param_word_is_literal(list_tok.kind, list_single_token) {
        return None;
    }
    let list_content_start = list_tok.span.start() + u32::from(list_tok.content_offset);
    let mut members = Vec::new();
    let mut pos = 0usize;
    loop {
        match tcl_syntax::list::find_element(list_word, pos) {
            Ok(Some(el)) => {
                let raw = &list_word[el.value.clone()];
                let value = if el.literal {
                    raw.to_string()
                } else {
                    tcl_lexer::backslash_subst(raw).into_owned()
                };
                let span = Span::new(
                    list_content_start + u32::try_from(el.value.start).unwrap_or(0),
                    list_content_start + u32::try_from(el.value.end).unwrap_or(0),
                );
                members.push((value, span));
                pos = el.next;
            }
            Ok(None) => break,
            // A malformed list is left exactly as opaque — no partial guess.
            Err(_) => return None,
        }
    }
    if members.is_empty() {
        None
    } else {
        Some(members)
    }
}

/// Whether a loop's body is exactly one member declaration (per `grammar`,
/// under any `self`/`private` wrapper) that names itself with exactly a
/// reference to the loop variable `var_name` — `foreach`'s installer idiom.
/// Returns the member's `(kind, body_span)` on a match; `None` for anything
/// else (more than one statement, a fixed name, a name built from more than
/// the bare variable, a member that is no method on either side) — left
/// exactly as opaque as before, not a partial guess.
pub(super) fn loop_installed_member_shape(
    grammar: &DefinitionBodyGrammar,
    body_word: &str,
    body_content_start: u32,
    lexer_config: tcl_lexer::LexerConfig,
    dialect: Option<SurfaceQuery<'_>>,
    var_name: &str,
) -> Option<(MethodKind, Span)> {
    let inner_cmds = crate::segmenter::segment_commands_with_offset_and_config(
        body_word,
        body_content_start,
        lexer_config.nested(),
    );
    let mut real_cmds = inner_cmds
        .iter()
        .filter(|c| !c.is_partial && !c.argv.is_empty());
    let inner = real_cmds.next()?;
    if real_cmds.next().is_some() {
        return None; // more than one statement — not the simple installer shape
    }
    let statement = MemberStatement::read(grammar, &inner.texts, &inner.argv, dialect)?;
    let MemberLanding::Method(kind @ (MethodKind::Method | MethodKind::ClassMethod)) =
        statement.landing()
    else {
        return None;
    };
    let (member, inner_texts, inner_argv) =
        (statement.member, statement.args(), statement.arg_tokens());
    let name_idx = member
        .indices_for_call_in(inner_texts, dialect, ArgRole::Name)
        .next()?;
    let original = tcl_lexer::native_script_words_in_with_purpose(
        tcl_lexer::SourceImage::document(body_word),
        Span::new(0, u32::try_from(body_word.len()).ok()?),
        lexer_config,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    if original.fatal_tail.is_some() || original.commands.len() != 1 {
        return None;
    }
    let words = &original.commands.first()?.words;
    if words.len() != inner.argv.len() {
        return None;
    }
    let ordinal = statement
        .row
        .keyword_index
        .checked_add(1)?
        .checked_add(name_idx)?;
    let name =
        tcl_syntax::word_rules::original_scalar_variable_with_prefix(words.get(ordinal)?, &[b""])?;
    if name != var_name.as_bytes() {
        return None;
    }
    let body_idx = member
        .indices_for_call_in(inner_texts, dialect, ArgRole::Body)
        .next()?;
    let body_span = inner_argv.get(body_idx).map_or(inner.span, |t| t.span);
    Some((kind, body_span))
}

impl Analyser {
    /// Record the command references a definition-body member names as
    /// arguments, driven entirely by the member's registry grammar — never by a
    /// member keyword the walker recognises.  Two grammar facts name a command:
    ///
    /// * `all_args_ref == Some(MemberRefKind::Class)` — every argument names a
    ///   class (`superclass A B`, `mixin ?-append? M …`).  A class is a command
    ///   in `TclOO`, so each is a command reference resolved in the class's
    ///   namespace.
    /// * an [`ArgRole::CommandName`] argument position — one argument names a
    ///   command (`forward NAME TARGET …`: the delegated command's name).
    ///
    /// Both flow through the ordinary invocation machinery, so find-references,
    /// go-to-definition, rename, and call-hierarchy reach the named class /
    /// command across files exactly as a direct call does.  A
    /// `MemberRefKind::Method` member names methods of *this* class rather than
    /// commands, so the method-reference machinery handles it, not this path.  A
    /// dynamic word (`superclass $base`) names no static command and is skipped.
    /// Whether one command of a definition body installs members this walk
    /// cannot read, making the class's recorded member tables a lower bound
    /// ([`super::super::types::ClassDef::member_set_incomplete`]).
    ///
    /// Two shapes qualify, neither matched by keyword:
    ///
    /// * a **member** word whose declaration is supplied dynamically — a
    ///   `{*}` expansion that survived [`splice_static_member_expansions`]
    ///   (only a braced literal splices statically), or a computed word in
    ///   one of the member's own declaring roles (`method $m …`). Tcl
    ///   resolves both at definition time, so the members are real; the
    ///   analyser simply cannot name them.
    /// * a **non-member** word that either has no registry spec (a helper
    ///   proc, or a class-installer command from another file) or declares
    ///   an [`ArgRole::Body`] argument — a script the member walk does not
    ///   descend into, so any `method` inside it is invisible
    ///   (`foreach m {…} { method $m … }`, `if {…} { method x {} {…} }`).
    ///
    /// Registry data decides both halves: the definer grammar says what a
    /// member word is and which of its arguments declare, and the command
    /// spec says whether a word carries a script. Nothing here names a
    /// command or a keyword.
    ///
    /// A comment line segments to no words at all and is skipped by the
    /// caller, so it never reaches this test.
    pub(super) fn member_declaration_is_opaque(
        &self,
        grammar: &DefinitionBodyGrammar,
        cmd: &crate::segmenter::SegmentedCommand,
        texts: &[String],
    ) -> bool {
        let Some(keyword) = texts.first() else {
            return false;
        };
        let Some(member) = grammar.member(keyword) else {
            // Not a member word. Unknown to the registry, or script-taking:
            // either way this command can install members out of sight.
            // Roles come from the static table *and* the per-call resolver
            // (`foreach`'s layout depends on how many var/list pairs it was
            // given, so its `Body` index is only knowable from the words).
            let generation = self.analysis_context();
            return generation
                .context()
                .resolve_spec(generation.commands(), keyword)
                .is_none_or(|spec| {
                    // Possible declaration effects use the selected schema's
                    // closed role capabilities, without an execution query.
                    tcl_registry::CommandRegistry::spec_may_have_arg_role(spec, ArgRole::Body)
                });
        };
        // A `{*}` word the splicer would refuse — anything but a single
        // braced literal, whose element list is the only one knowable
        // without running the program.  Tested per word (not "did any
        // splice happen"), so a command mixing a spliceable and an
        // unspliceable expansion is still opaque.
        if let Some(expand) = cmd.expand_word.as_ref()
            && expand.iter().enumerate().any(|(i, &e)| {
                e && !(cmd.argv.get(i).is_some_and(|t| t.kind == TokenType::Str)
                    && cmd.single_token_word.get(i).copied().unwrap_or(false))
            })
        {
            return true;
        }
        // A computed member **name** — `method $m …`, the installer-loop
        // spelling — declares a member the walk cannot name.  `+ 1` because
        // role indices are relative to the member's arguments while `texts`
        // still carries the keyword at 0.
        //
        // Only the name word is tested.  A parameter list or body word is a
        // *script*, and a script routinely contains `$` and `[` without being
        // dynamic in the sense that matters here (`constructor {x} { set a
        // $x }` is entirely readable); the `{*}` test above is what catches
        // the genuinely reflected signature.
        let original =
            self.original_logical_member_words(grammar, cmd)
                .filter(|(statement, _, _)| {
                    cmd.texts.as_slice() == texts && std::ptr::eq(statement.member, member)
                });
        member
            .indices_for_call_in(
                &texts[1..],
                Some(self.analysis_context().context().authoring_query()),
                ArgRole::Name,
            )
            .any(|idx| {
                if let Some((statement, _, words)) = original.as_ref() {
                    return words
                        .get(statement.row.keyword_index + 1 + idx)
                        .is_none_or(|word| {
                            tcl_syntax::word_rules::original_static_word_unicode_value(word)
                                .is_none_or(|value| value.contains(&0))
                        });
                }
                if self.selected_logical_declaration_advice() {
                    return true;
                }
                texts
                    .get(idx + 1)
                    .is_some_and(|w| crate::naming::is_dynamic_word(w))
            })
    }

    /// Record the per-name members a **literal** loop-installer declares,
    /// even though [`Self::member_declaration_is_opaque`] has already (and
    /// correctly) marked the class's member set incomplete because of it.
    ///
    /// `foreach m {alpha beta gamma} { method $m {args} {…} }` computes
    /// each member's *name* from the loop variable, which is why the
    /// ordinary member walk cannot read it — but the loop's own list is
    /// written right there in the source, so the set of names it will bind
    /// `$m` to really is knowable without running the program. This adds
    /// what can honestly be said on top of the existing abstention; it
    /// never narrows `member_set_incomplete` back to "complete", because
    /// knowing three names is not the same as knowing them all (a sibling
    /// `oo::define` elsewhere, or a second, unreadable installer in the
    /// same body, could still add more).
    ///
    /// Deliberately narrow — every condition below is a bail, not a best
    /// effort, and any miss just leaves the loop as opaque as it already
    /// was:
    ///
    /// * the head word must be a registry-declared "var/list pairs then a
    ///   trailing body" loop shape ([`tcl_registry::Traits::LOOP_LIST_HEADER`]
    ///   plus exactly one [`tcl_registry::repeated::RepeatedArgLayout`]
    ///   covering a var/list pair) — `foreach`/`lmap`, decided from registry
    ///   data, never a keyword;
    /// * exactly one var/list pair (a multi-list `foreach` has no single
    ///   variable a member's name word could unambiguously mean);
    /// * the loop variable is one bare name, not a multi-variable group
    ///   (`foreach {a b} $list …`);
    /// * the list word is a **literal** Tcl list (no `$`/`[`) that parses
    ///   cleanly;
    /// * the body is exactly one member declaration (per `grammar`, unwrapped
    ///   of any `self`/`private`) whose `Name` argument is exactly a
    ///   reference to the loop variable — anything else (a fixed name, a
    ///   name built from more than the bare variable, more than one
    ///   statement) is left unread, exactly as before this ran.
    ///
    /// The recorded member's parameter list is always `params_computed`
    /// (never re-derived from the loop body's own written params, even when
    /// they look literal): a reflective installer's signature can itself be
    /// computed per iteration (`method $m {*}[classDef $m]`), so treating
    /// one binding's `{args}` as representative of every other would be a
    /// fabrication for the general shape this exists to cover.
    pub(super) fn record_loop_installed_members(
        &self,
        grammar: &DefinitionBodyGrammar,
        cmd: &crate::segmenter::SegmentedCommand,
        class_def: &mut ClassDef,
    ) {
        let Some(keyword) = cmd.texts.first().map(String::as_str) else {
            return;
        };
        let generation = self.analysis_context();
        let Some(spec) = generation
            .context()
            .resolve_spec(generation.commands(), keyword)
        else {
            return;
        };
        // Post-head argument count (`texts` still carries the keyword at 0).
        let post_head = cmd.texts.len().saturating_sub(1);
        let Some((var_idx, list_idx, body_idx)) = loop_installer_pair(spec, post_head) else {
            return;
        };
        let (Some(var_word), Some(list_word), Some(body_word)) = (
            cmd.texts.get(var_idx + 1),
            cmd.texts.get(list_idx + 1),
            cmd.texts.get(body_idx + 1),
        ) else {
            return;
        };
        // The loop variable must be one bare name.
        let Ok(var_names) = self.word_rules().split_list(var_word) else {
            return;
        };
        if var_names.len() != 1 {
            return; // a multi-variable group — no single name a `$var` could mean
        }
        let var_name = &var_names[0];
        if !tcl_syntax::naming::is_static_variable_source_word(var_name) {
            return;
        }
        let Some(list_tok) = cmd.argv.get(list_idx + 1) else {
            return;
        };
        let list_single = cmd
            .single_token_word
            .get(list_idx + 1)
            .copied()
            .unwrap_or(false);
        let Some(members) = literal_loop_elements(list_word, *list_tok, list_single) else {
            return;
        };
        let Some(body_tok) = cmd.argv.get(body_idx + 1) else {
            return;
        };
        let body_content_start = body_tok.span.start() + u32::from(body_tok.content_offset);
        let Some((kind, body_span)) = loop_installed_member_shape(
            grammar,
            body_word,
            body_content_start,
            self.file_lexer_config(),
            Some(self.analysis_context().context().authoring_query()),
            var_name,
        ) else {
            return;
        };
        for (name, name_span) in members {
            let md = MethodDef {
                visibility: default_visibility(grammar, &name),
                name,
                params: Vec::new(),
                params_computed: true,
                formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                    tcl_dialect::ParameterGrammar::Tcl,
                ),
                name_span,
                body_span,
                kind: kind.as_str().to_string(),
                is_self_method: false,
                doc: String::new(),
                forward_target: None,
            };
            // A literal, directly-written declaration elsewhere in the class
            // always outranks a name merely *inferred* from the loop's list —
            // this only ever fills a gap, never overrides real data, however
            // the two are ordered in the source.
            let table = if kind == MethodKind::ClassMethod {
                &mut class_def.class_methods
            } else {
                &mut class_def.methods
            };
            table.entry(md.name.clone()).or_insert(md);
        }
    }

    pub(in super::super) fn record_member_command_references(
        &mut self,
        grammar: &DefinitionBodyGrammar,
        texts: &[String],
        argv: &[Token],
        scope_path: &[usize],
    ) {
        // Unwrap a `self …` (or itcl access-modifier) prefix so a wrapped
        // `self mixin -append …` still contributes its class references.  The
        // returned texts/tokens are the words *after* the effective keyword, so
        // grammar arg-role indices (0-based after the keyword) index them
        // directly.
        let Some((keyword, arg_texts, arg_toks)) = unwrap_wrapper_member(grammar, texts, argv)
        else {
            return;
        };
        let Some(spec) = grammar.member(keyword) else {
            return;
        };
        // `skip_flags` drops `-`-prefixed option words — correct only for the
        // class-list members (`mixin -append …`), where the flags aren't class
        // names.  A `forward` TARGET is a plain command name that may legally
        // begin with `-` (`forward f -foo` delegates to a command named
        // `-foo`), so its `ArgRole::CommandName` recording keeps such words.
        let record = |analyser: &mut Self, name: &str, tok: &Token, skip_flags: bool| {
            if name.is_empty() || crate::naming::is_dynamic_word(name) {
                return;
            }
            if skip_flags && name.starts_with('-') {
                return;
            }
            // A named command reference carries no fixed call arity: a
            // superclass/mixin is not invoked here, and a `forward` target is
            // invoked with a variable number of appended arguments.
            let resolved = analyser.resolve_command_qualified_name(name, scope_path);
            analyser.push_command_reference(name.to_owned(), tok.span, resolved, None);
        };
        if spec.all_args_ref == Some(MemberRefKind::Class) {
            for (name, tok) in arg_texts.iter().zip(arg_toks) {
                record(self, name, tok, true);
            }
        }
        for idx in spec.indices_for_call_in(
            arg_texts,
            Some(self.analysis_context().context().authoring_query()),
            ArgRole::CommandName,
        ) {
            if let (Some(name), Some(tok)) = (arg_texts.get(idx), arg_toks.get(idx)) {
                record(self, name, tok, false);
            }
        }
    }
}
