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

//! The walk over one script body and its list-shaped arguments.

use super::argument_override::{head_is_computed, subspec_content};
use super::argument_tokens::classify_arg_token;
use super::command_roles::{
    command_role_overrides, insert_original_proc_role_overrides,
    merge_list_quoted_command_overrides, unrecorded_head_kind,
};
use super::command_tokens::{
    CommandTokenSources, classify_and_push_if, collect_command_tokens, command_body_context,
    command_deferred_role_starts, deferred_produced_command_words, emit_selected_command_head,
};
use super::emission::{Entry, push_token};
use super::legend::TokenKind;
use super::object_dispatch::{ObjectClassMap, insert_self_method_overrides};
use super::original;
use super::regex_subtokens::{mark_regex_source_words, push_regex_subtokens};
use super::variable_name_roles::VarNameArgRoles;
use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::{AnalysisResult, ClassHierarchy};
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_dialect::NumberSyntax;
use tcl_lexer::{LineIndex, Span, Token, TokenType};
use tcl_registry::CommandRegistry;
use tcl_registry::definer::DefinitionBodyGrammar;

/// Maximum body / expr / command-substitution recursion depth — guards
/// against pathological nesting.
pub(super) const MAX_TOKEN_RECURSION: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(32);

/// Emit the command-head token, splitting a namespace-qualified head
/// (`oo::class`, `::set`) into a `namespace` token for the leading
/// `…::` prefix plus a command token for the final segment.  A bare head
/// is emitted whole, carrying `defaultLibrary` when it resolves to a
/// registry built-in.
/// Sub-tokenise the braced case list of `switch … { pat body … }`.
///
/// The inner script is re-segmented into commands; the words are flattened
/// across all command lines and paired (even index → pattern, odd index →
/// body), since a Tcl `switch` case list is one flat list whose line breaks
/// are insignificant whitespace.  Body words are recursed as scripts.
/// Pattern words (except the literal `default`) are sub-tokenised as regexes
/// when `regexp` is set (`-regexp` mode), otherwise classified as ordinary
/// literals.
/// Immutable context threaded through the recursive script-tokenisation
/// walk.  Bundling these read-only borrows keeps each recursive helper to a
/// small, focused signature (the mutable `entries` sink and the `depth`
/// guard stay explicit parameters).
/// Per-walk facts every [`ScriptCtx`] of one token walk shares.
pub(super) struct WalkFacts<'a> {
    /// The source shell this walk reads. Nothing was interpreted, so the
    /// context's `analysis` is `None` and every command reads its written
    /// words.
    pub(super) shell: Option<&'a AnalysisResult>,
    /// Written command tails the analysed document itself rebinds (a
    /// procedure, an alias, either end of a `rename`). A head the analysis has
    /// no proof for keeps its descriptor's colours unless it is one of these.
    pub(super) rebound_tails: FxHashSet<Vec<u8>>,
    /// The script regions this walk entered, with their nesting depth. Their
    /// commands are coloured as script, so their comments are comments.
    pub(super) entered_scripts: std::cell::RefCell<Vec<(Span, u32)>>,
    /// The analysed formals of every procedure declaration, keyed by the
    /// extent of the first token of its parameter word. Built on first use.
    pub(super) procedure_formals: std::cell::OnceCell<
        FxHashMap<
            (u32, u32),
            tcl_compiler::signature_scan::formal_parameters::SignatureSourceFormalParameters,
        >,
    >,
}

impl<'a> WalkFacts<'a> {
    pub(super) fn new(
        shell: Option<&'a AnalysisResult>,
        analysis: Option<&AnalysisResult>,
    ) -> Self {
        let rebound_tails = analysis.map_or_else(FxHashSet::default, |analysis| {
            analysis
                .all_procs
                .keys()
                .chain(analysis.command_aliases.keys())
                .chain(analysis.renamed_commands.keys())
                .chain(analysis.renamed_commands.values())
                .map(|name| tcl_syntax::naming::written_command_tail(name.as_bytes()).to_vec())
                .collect()
        });
        Self {
            shell,
            rebound_tails,
            entered_scripts: std::cell::RefCell::new(Vec::new()),
            procedure_formals: std::cell::OnceCell::new(),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ScriptCtx<'a> {
    pub(super) full_source: &'a str,
    pub(super) dialect: &'static tcl_dialect::DialectProfile,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) context: &'a tcl_registry::model::ResolvedContext,
    pub(super) generation: &'a tcl_registry::model::ContextRegistry,
    pub(super) operand: Option<&'a crate::original_invocation::OriginalOperandSource>,
    pub(super) original_words: Option<&'a crate::original_invocation::OriginalRegistryWords>,
    pub(super) declared_words:
        Option<&'a tcl_compiler::command_binding::OriginalDeclaredCommandWords>,
    /// The enclosing definition body was opened by a command with no retained
    /// invocation, so its member words are read from the grammar alone.
    pub(super) written_members: bool,
    /// Facts fixed for the whole walk.
    pub(super) walk: &'a WalkFacts<'a>,
    pub(super) lexical: bool,
    /// The dialect's numeric-literal grammar, resolved once from `dialect` so
    /// the per-word `Number` classification does not re-walk the profile
    /// catalogue for every token.  Decides which radix prefixes exist, whether a
    /// bare leading zero is octal, and whether `_` digit separators are
    /// allowed — see [`tcl_syntax::number::is_whole_number`].
    pub(super) numbers: NumberSyntax,
    pub(super) registry: &'a CommandRegistry,
    pub(super) line_index: &'a LineIndex,
    /// The enclosing definition-body grammar, or `None` outside any
    /// definition body.  When `Some`, this script is a class/type definition
    /// body (an `oo::class create … { … }`, `snit::type … { … }`, or bare
    /// `oo::define … { … }` block) and the grammar's member sub-keywords
    /// (`method`, `typemethod`, `constructor`, `variable`, …) carry the script
    /// bodies / parameter lists / variable declarations to recurse and
    /// highlight — see [`crate::oo_body`].  Outside one, a same-named user proc
    /// is never treated as a member.
    pub(super) oo_grammar: Option<&'static DefinitionBodyGrammar>,
    /// Selected source member receiver vocabulary, without an entered frame.
    pub(super) receiver_grammar: Option<&'static DefinitionBodyGrammar>,
    /// Member vocabulary retains its actual source schema and whole body.
    pub(super) original_definition_parent:
        Option<&'a tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
    /// Complete original argv owner for this current definition-source region.
    pub(super) original_definition_members:
        Option<&'a tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion>,
    /// The enclosing scoped command environment, or `None` outside any scoped
    /// body.  When `Some`, this script runs in a context (a `report::defstyle`
    /// style script) that exposes a curated command set (`top`, `data`,
    /// `columns`, …) — the heads highlight as library commands and their
    /// ensemble operations (`top set`) as subcommand keywords, resolved from
    /// registry data (see [`tcl_registry::scoped`]).  Persists into nested
    /// bodies and command substitutions inside the scoped body.
    pub(super) scoped_env: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
    /// Def-site literal value words to highlight as regex (regex-source
    /// tracking), keyed by word start.  Empty when disabled.
    pub(super) regex_sources: &'a FxHashMap<u32, Span>,
    /// Object-handle → class-name provenance for the whole document, so a
    /// `$var method …` dispatch can resolve the method's options through the
    /// registry's object-class model.  Empty when no
    /// [`CompilationUnit`] is available or the document creates no tracked
    /// object handles.
    pub(super) object_classes: &'a ObjectClassMap,
    /// Class hierarchy, when available — the MRO + `ClassDef`s (methods +
    /// `oo::configurable` properties) used to resolve a dispatched method
    /// against a *user* class, not just a registry-modelled one.  This is the
    /// current file's hierarchy, or a workspace-merged project index so a class
    /// defined in another file resolves too.  `None` for
    /// the pure-segmentation path.
    pub(super) classes: Option<&'a ClassHierarchy>,
    pub(super) project_classes: Option<
        &'a [tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards],
    >,
    /// The local analysis when this request has one, used only for document
    /// package floors on lifecycle-gated registry methods.
    pub(super) analysis: Option<&'a AnalysisResult>,
    /// Original collection insertion/extraction occurrences of the same
    /// compilation unit, keyed by source occurrence rather than variable text.
    /// `None` without a [`CompilationUnit`] or a source-receiver consumer.
    pub(super) original_collections:
        Option<&'a tcl_compiler::object_types::OriginalObjectCollectionSources>,
    pub(super) original_roles: &'a original::OriginalTokenRoles,
    pub(super) proc_roles: Option<&'a VarNameArgRoles>,
    /// The class whose definition body we are currently inside (as written at
    /// the `oo::class create NAME` / `oo::define NAME` head), sliced from the
    /// source so it lives as long as the walk.  Lets a `my method …` self-call
    /// in a method body resolve against the enclosing class's MRO — the single
    /// most common `TclOO` dispatch form.  `None` outside any class body.
    pub(super) enclosing_class: Option<&'a str>,
    /// Source offsets admitted by the shared iRules declaration-boundary
    /// owner. Empty outside the iRules overlay.
    pub(super) irules_top_level_declaration_heads: &'a FxHashSet<u32>,
}

/// Emit one clause-list *pattern* element.
///
/// A keyword pattern (`default`; Expect's `timeout` / `eof` / `full_buffer`)
/// matches no text — it is a keyword, never a regex and never a string.
/// Otherwise a regex-mode pattern is sub-tokenised as a regex, and an
/// exact/glob one is classified as an ordinary literal.
fn push_case_pattern(
    ctx: ScriptCtx<'_>,
    pat_tok: Token,
    text: &str,
    spec: &tcl_registry::CaseListSpec,
    regexp: bool,
    entries: &mut Vec<Entry>,
) {
    let line_index = ctx.line_index;
    let full_source = ctx.full_source;
    if spec.keyword_patterns.contains(&text) {
        push_token(
            line_index,
            full_source,
            pat_tok,
            TokenKind::Keyword,
            0,
            entries,
        );
    } else if regexp {
        if !push_regex_subtokens(ctx, pat_tok, entries) {
            push_token(
                line_index,
                full_source,
                pat_tok,
                TokenKind::Regexp,
                0,
                entries,
            );
        }
    } else if let Some(kind) = classify_arg_token(pat_tok, full_source, ctx.numbers) {
        push_token(line_index, full_source, pat_tok, kind, 0, entries);
    }
}

pub(super) fn collect_case_list(
    ctx: ScriptCtx<'_>,
    tok: Token,
    entries: &mut Vec<Entry>,
    depth: u32,
    spec: &tcl_registry::CaseListSpec,
    regexp: bool,
    written: bool,
) {
    if !ctx.lexical
        && !written
        && !ctx
            .operand
            .and_then(|operand| operand.word.as_ref())
            .is_some_and(|word| {
                word.group().kind == tcl_lexer::WordKind::Braced
                    && word
                        .tokens()
                        .first()
                        .is_some_and(|first| first.span == tok.span)
            })
    {
        classify_and_push_if(true, ctx, tok, entries);
        return;
    }
    if MAX_TOKEN_RECURSION.exceeded(depth) {
        return;
    }
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let Some((cstart, inner)) = subspec_content(full_source, tok) else {
        return;
    };

    // The clause split is `tcl-syntax`'s, shared with the iRules
    // object-reference walker: if the two disagreed about where a clause body
    // is, they would disagree about what the code says.  It also handles
    // Expect's clause-leading flags (`-re`, `-timeout 5`), which strict
    // pattern/body alternation would let shift every following element by one.
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: spec.clause_flags,
        clause_value_flags: spec.clause_value_flags,
    };

    // Rebuild each element as a `Token` following the lexer's inner-end +
    // `content_offset` convention (`span.end()` sits at the closing `}`/`"`;
    // `content_offset` strips the opener) so the downstream helpers work
    // unchanged.
    let as_token = |e: tcl_syntax::case_list::Element| {
        let range = e.token_range();
        let (kind, content_offset) = if e.braced {
            (TokenType::Str, 1u8)
        } else if inner.as_bytes().get(e.start) == Some(&b'"') {
            (TokenType::Esc, 1u8)
        } else {
            (TokenType::Esc, 0u8)
        };
        Token::with_content_offset(
            kind,
            tcl_lexer::Span::new(
                u32::try_from(cstart + range.start).unwrap_or(0),
                u32::try_from(cstart + range.end).unwrap_or(0),
            ),
            content_offset,
        )
    };

    for clause in
        tcl_syntax::case_list::split_case_list_with_syntax(inner, &shape, ctx.config.list_parse)
    {
        let mut clause_regexp = regexp;
        for f in &clause.flags {
            let text = inner.get(f.start..f.end).unwrap_or_default();
            // The descriptor permits unique prefixes (`-re` for `-regexp`),
            // so compare the resolved canonical flag rather than the spelling
            // that appeared in the clause list.
            let flag = shape.resolve_flag(text);
            if spec.clause_regex_flag == flag {
                clause_regexp = true;
            }
            let ftok = as_token(*f);
            // A flag word is a decorator; its *value* word (`-timeout 5`) takes
            // its own literal classification.
            let kind = if flag.is_some() {
                Some(TokenKind::Decorator)
            } else {
                classify_arg_token(ftok, full_source, ctx.numbers)
            };
            if let Some(kind) = kind {
                push_token(line_index, full_source, ftok, kind, 0, entries);
            }
        }

        if let Some(p) = clause.pattern {
            let Some(text) = inner.get(p.content_range()) else {
                continue;
            };
            push_case_pattern(ctx, as_token(p), text, spec, clause_regexp, entries);
        }

        // Body element — recurse as a script.
        if let Some(b) = clause.body {
            let btok = as_token(b);
            if let Some((bstart, body)) = subspec_content(full_source, btok) {
                collect_script(
                    ctx,
                    body,
                    u32::try_from(bstart).unwrap_or(0),
                    entries,
                    depth + 1,
                    false,
                );
            } else if let Some(kind) = classify_arg_token(btok, full_source, ctx.numbers) {
                push_token(line_index, full_source, btok, kind, 0, entries);
            }
        }
    }
}

/// Recurse the body of an `ArgRole::LambdaLiteral` `{params body ?ns?}`
/// lambda literal (`apply`'s shape, reached directly or through the shared original
/// produced-prefix source receipt).
///
/// The braced lambda is a Tcl list; its second element is the body script
/// and is re-segmented so its commands / vars / strings tokenise like any
/// other body.  The first element (the argument list) and an optional third
/// (the namespace) are emitted with their default classification, so no part
/// of the lambda is dropped.  Mirrors C Tcl's `apply` lambda shape.
///
/// The body recurses in a *fresh* context (`oo_grammar` / `enclosing_class` /
/// `scoped_env` all cleared), never the enclosing command's: `apply`'s body
/// runs in a new call frame in the global namespace by default (or the
/// lambda's own optional third element, never inherited), so `my foo` inside
/// a bare `apply {{} {my foo}}` called from a method body is not actually a
/// call to that method at runtime — `my` isn't defined in `::`. Leaving the
/// enclosing class/grammar/scoped-env active here would resolve such a call
/// anyway, painting it as live when it would error (mirrors `folding.rs`'s
/// `None` reset for the same recursion, and the same fresh-frame reasoning the
/// interprocedural and param-trait passes apply).
pub(super) fn collect_lambda_literal(
    ctx: ScriptCtx<'_>,
    tok: Token,
    entries: &mut Vec<Entry>,
    depth: u32,
) {
    original::collect_lambda(ctx, tok, entries, depth);
}

/// Emit the name(s) of a `foreach` / `lmap` / `dict for` variable spec as
/// variable declarations.  A single bareword is one name; a braced/quoted list
/// is flattened via the list grammar and each element name emitted separately.
/// A non-name element (a `$`-computed word, an array element) keeps a plain
/// `string` classification so nothing is dropped and it does not masquerade as
/// a variable.
pub(super) fn collect_loop_var_list(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>) {
    if !original::collect_loop_variables(ctx, tok, entries) {
        classify_and_push_if(true, ctx, tok, entries);
    }
}

/// Emit a procedure parameter list's names as variable declarations.  Each
/// top-level list element is either a bareword parameter name or a `{name
/// ?default...?}` pair; the name is a `Variable` declaration and any default
/// words are classified (number / string).  A non-name element is left to the
/// default classifier.
pub(super) fn collect_param_list(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>) {
    if !original::collect_parameter_fields(ctx, tok, entries) {
        classify_and_push_if(true, ctx, tok, entries);
    }
}

/// Emit the command-head token for a *static* head word — a resolvable command
/// name, painted as a single function / keyword / namespace token.
///
/// A *computed* head — `$obj method …`, `[dict get …] method …`, `[Class new]
/// method …`, a multi-fragment `chartV$node` — is not a command name we can
/// resolve, so it must not be painted as one; the caller gates on
/// [`head_is_computed`] and lets those tokens fall through to the ordinary
/// argument path, where a `[…]` recurses into its inner script and a `$var`
/// reads as a variable — an accurate picture of the runtime dispatch rather
/// than a misleading command highlight.
pub(super) fn emit_static_command_head(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    entries: &mut Vec<Entry>,
) {
    let Some(&head_tok) = seg.argv.first() else {
        return;
    };
    let (kind, mods) = if ctx.original_roles.head_names_class(ctx, seg) {
        (TokenKind::Class, 0)
    } else if original::definition_member_head(ctx, seg) {
        (TokenKind::Keyword, 0)
    } else {
        unrecorded_head_kind(ctx, seg)
    };
    push_token(
        ctx.line_index,
        ctx.full_source,
        head_tok,
        kind,
        mods,
        entries,
    );
}

/// Segment `text` (anchored at absolute byte `base_offset` within
/// `full_source`) into commands and push a semantic-token [`Entry`] for each
/// token, recursing into braced bodies (`ArgRole::Body`), braced expressions
/// (`ArgRole::Expr`), and `[…]` command substitutions.  Token spans are
/// already absolute (the segmenter shifts them by `base_offset`), so positions
/// and text are resolved against `full_source` + `line_index`.
///
/// `deferred_role` is `true` only when this call's *entire* `text` is the
/// content of a `[…]` substitution whose own enclosing argument slot (in the
/// command containing it) carries `Body` / `LambdaLiteral` / `CommandPrefix`
/// — i.e. a position whose value is later invoked or sourced, not merely
/// computed. It gates the original produced-prefix projection and is otherwise `false`: every other recursion (the top-level
/// script, a body, a lambda body, a case-list clause, an expression) processes
/// source that is *itself* executed code, not a value that might or might not
/// be invoked later, so list-quoted detection inside it is decided fresh at the
/// next `[…]` hop rather than inherited.
pub(super) fn collect_script(
    ctx: ScriptCtx<'_>,
    text: &str,
    base_offset: u32,
    entries: &mut Vec<Entry>,
    depth: u32,
    deferred_role: bool,
) {
    if MAX_TOKEN_RECURSION.exceeded(depth) {
        return;
    }
    if let Ok(length) = u32::try_from(text.len()) {
        ctx.walk.entered_scripts.borrow_mut().push((
            Span::new(base_offset, base_offset.saturating_add(length)),
            depth,
        ));
    }
    let full_source = ctx.full_source;
    let definition_members = ctx.original_definition_parent.and_then(|parent| {
        let end = base_offset.checked_add(u32::try_from(text.len()).ok()?)?;
        parent.definition_member_region(ctx.generation, Span::new(base_offset, end))
    });
    let ctx = ScriptCtx {
        original_definition_members: definition_members.as_ref(),
        ..ctx
    };
    for seg in
        segment_commands_with_offset_and_config(text, base_offset, ctx.config.at_depth(depth))
    {
        // A cooperative stop point only: a cancelled host read unwinds here.
        tcl_compiler::analysis_control::unwind_point(
            tcl_compiler::analysis_control::AnalysisCheckpoint::Command,
        );
        if seg.argv.is_empty() {
            continue;
        }
        // Classify the command-head token.  A head that resolves to a registry
        // built-in carries the `defaultLibrary` modifier.
        let head_tok = seg.argv[0];
        let head_text = &seg.texts[0];
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        // Source descriptors retain their independently selected domain. Logical
        // advice uses the same original word/context owner as other source roles.
        let original = ctx.analysis.and_then(|analysis| {
            crate::original_invocation::source_registry_words(full_source, analysis, &seg)
        });
        let member_head = original::definition_member_head_word(ctx, &seg)
            .and_then(|word| Some((word, word.content_span().ok()?)));
        // A complete original member word can prove static spelling even when
        // its physical escape tokens form several lexical fragments.
        let computed_head = member_head.is_none()
            && head_is_computed(&seg)
            && original
                .as_ref()
                .is_none_or(|words| original::original_logical_head(ctx, words).is_none());
        let static_head_extent = member_head.map_or(head_tok.span, |(word, _)| word.word_span());
        if !computed_head {
            emit_selected_command_head(ctx, &seg, original.as_ref(), member_head, entries);
        }

        // The command's argument words (head excluded), borrowed once as
        // `&[&str]` and shared by every registry-driven pass below — the
        // override builder and the OO-body context check both need it, and
        // the registry API takes `&[&str]`, so building it here keeps the
        // hot path to a single bridging allocation per command.
        let arg_texts: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();

        let member_bodies = original::definition_member_bodies(ctx, &seg);
        let original_bodies = original
            .as_ref()
            .map_or_else(Vec::new, |words| words.source_script_bodies(ctx.generation));
        let mut overrides =
            command_role_overrides(ctx, &seg, original.as_ref(), member_bodies.as_deref());
        let member_parameter_operands = original::definition_member_parameter_operands(ctx, &seg);
        let declared = ctx.analysis.and_then(|analysis| {
            tcl_compiler::registry_invocation::source_structure::source_declared_command_words(
                full_source,
                analysis,
                &seg,
            )
        });
        if let Some(declared) = &declared {
            overrides.extend(original::declared_overrides(&seg, declared));
        }
        let procedure_role_extents = insert_original_proc_role_overrides(ctx, &seg, &mut overrides);
        // A deferred operand may retain the exact future source words of a
        // selected original list builder. The shared receipt owns both parent
        // and builder schemas; inert data has no deferred parent purpose.
        let produced = deferred_produced_command_words(ctx, &seg, deferred_role);
        if let Some(produced) = &produced {
            merge_list_quoted_command_overrides(&seg, ctx, produced, &mut overrides);
        }
        // `my method …` inside a class body resolves against the enclosing
        // class's MRO (the most common `TclOO` dispatch form).
        if !ctx.lexical || !ctx.original_roles.has_head(head_tok.span.start()) {
            insert_self_method_overrides(ctx, &seg, original.as_ref(), &mut overrides);
        }
        ctx.original_roles
            .insert_overrides(ctx, &seg, &mut overrides);
        // Regex-source tracking: retag a `set` value word that feeds a regexp
        // pattern as a (substitution-aware) regex.
        mark_regex_source_words(&seg, ctx.regex_sources, &mut overrides);

        let body_ctx = command_body_context(ctx, &seg, original.as_ref(), head_text, &arg_texts);

        let deferred_role_starts =
            command_deferred_role_starts(original.as_ref(), declared.as_ref());

        let mut expanded_roles = original
            .as_ref()
            .map_or_else(Vec::new, original::expanded_role_extents);
        expanded_roles.extend(procedure_role_extents);
        collect_command_tokens(
            ctx,
            body_ctx,
            &seg,
            &CommandTokenSources {
                original: &original,
                produced: &produced,
                declared: &declared,
                member_bodies: &member_bodies,
                original_bodies: &original_bodies,
                member_parameter_operands: &member_parameter_operands,
                overrides: &overrides,
                deferred_role_starts: &deferred_role_starts,
                expanded_roles: &expanded_roles,
                computed_head,
                static_head_extent,
            },
            entries,
            depth,
        );
    }
}
