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

//! The walk of a `TclOO` definition body and of the inline `oo::define` form: each member statement, wrapper expansion, the bodies it collects and the W002 for a member the dialect lacks.

use super::super::state::Analyser;
use super::super::types::ClassDef;
use super::super::utils::parse_param_list;
use super::member_effects::{apply_oo_subcommand_in, collect_property_accessor_bodies};
use super::{
    CollectedDefinitionBodies, CollectedMethodBody, MemberDialect, MemberLanding, MemberStatement,
    MethodBodyClass,
};
use tcl_dialect::model::{SurfaceQuery, surface_admits};
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::{
    DefinitionBodyGrammar, MemberCurrentNamespace, MemberEffect, MemberKind, MemberReceiver,
    MemberSpec,
};
use tcl_syntax::word_rules::WordValueRules;

/// Whether `member` declares variables by name: a
/// [`MemberEffect::StateDeclaration`] whose words name them — every argument
/// (`TclOO`'s `variable a b c`) or its [`ArgRole::VarWrite`] word (snit's
/// `variable` / `typevariable` / `component` / `typecomponent`, itcl's
/// `variable` / `common`). snit's `option` declares state but names an option,
/// not a variable, so it is not one. A callable that binds a value variable
/// (snit 1.x `onconfigure`) is a body, not a declaration.
fn declares_variables(member: &MemberSpec) -> bool {
    matches!(member.effect, MemberEffect::StateDeclaration { .. })
        && (member.all_args_var || member.indices_for(ArgRole::VarWrite).next().is_some())
}

/// Map each declared instance-variable name in `known` to the span of its
/// declaration name-token, scanning the class-body `cmds`.  Used to anchor a
/// seeded object variable's `definition_span` at its `variable v` declaration
/// instead of the whole method body — a whole-body span would let a
/// variable rename overwrite the entire method.  Only names already present in
/// `known` (the authoritative `ClassDef::variables` list) are recorded, so a
/// non-name argument of a declaration member can never be mistaken for a
/// variable; the first declaration of a name wins.
pub(super) fn collect_var_decl_spans(
    grammar: &DefinitionBodyGrammar,
    cmds: &[tcl_source::segment::SegmentedCommand],
    known: &[String],
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    dialect: Option<SurfaceQuery<'_>>,
) -> std::collections::HashMap<String, Span> {
    let mut out: std::collections::HashMap<String, Span> = std::collections::HashMap::new();
    if known.is_empty() {
        return out;
    }
    let known_set: std::collections::HashSet<&str> = known.iter().map(String::as_str).collect();
    for cmd in cmds {
        if cmd.is_partial {
            continue;
        }
        // A statement is a variable declaration when its row declares the
        // instances' state and its member names variables — the same answer
        // the class fold gives it, so the span map and `ClassDef::variables`
        // cannot disagree.  Gating additionally on `known_set` below means a
        // stray non-declaration match cannot leak.
        let Some(statement) = MemberStatement::read(grammar, &cmd.texts, &cmd.argv, dialect) else {
            continue;
        };
        if !declares_variables(statement.member)
            || statement.row.receiver != MemberReceiver::Instance
        {
            continue;
        }
        let groups = tcl_lexer::group_commands_bytes(&cmd.all_tokens, image.bytes(), config);
        let [group] = groups.as_slice() else {
            continue;
        };
        if group.words.len() != cmd.argv.len() || group.words.iter().any(|word| word.expand) {
            continue;
        }
        let args = cmd.args().iter().map(String::as_str).collect::<Vec<_>>();
        for argument in
            grammar.member_arg_indices_for_role_in(cmd.name(), &args, dialect, ArgRole::VarWrite)
        {
            let ordinal = argument + 1;
            let Some((group, tok)) = group.words.get(ordinal).zip(cmd.argv.get(ordinal)) else {
                continue;
            };
            let Ok(word) =
                tcl_lexer::NativeWord::from_group(image.clone(), config, &cmd.all_tokens, group)
            else {
                continue;
            };
            let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(&word)
            else {
                continue;
            };
            let Ok(name) = String::from_utf8(value) else {
                continue;
            };
            let base = tcl_syntax::naming::split_element_ref(&name)
                .map_or(name.as_str(), |(root, _)| root);
            if known_set.contains(base) {
                out.entry(base.to_owned()).or_insert(tok.span);
            }
        }
    }
    out
}

impl Analyser {
    /// Walk the body of a ``oo::class create`` / ``oo::define``
    /// block, populating `class_def` from each subcommand.
    ///
    /// The body is re-segmented via
    /// [`tcl_source::segment::segment_commands_with_offset`] (no
    /// recovery — recovery is top-level only).  Dynamic bodies
    /// (non-`Str` tokens) skip the walk because they can't be
    /// statically re-segmented.
    /// Whether the command `cmd_name` is *unavailable* in the active dialect —
    /// its registry spec is dialect-gated and the document's dialect falls
    /// outside the gate.  Used to suppress member-level dialect diagnostics
    /// when the enclosing definer (`oo::configurable`, itself 9.0+) is already
    /// flagged: one diagnostic for the version-only construct, not a cascade.
    pub(in super::super) fn command_dialect_disabled(&self, cmd_name: &str) -> bool {
        let generation = self.analysis_context();
        generation
            .commands()
            .get(cmd_name)
            .is_some_and(|spec| !generation.context().spec_available(spec))
    }

    /// Whether the definition-body member `subcmd` is available in the active
    /// dialect.  A member with no dialect restriction (the common case) and an
    /// unknown member (handled elsewhere, not gated here) both count as
    /// available; a version-gated member — `classmethod`, `private`,
    /// `initialise` / `initialize`, `definitionnamespace`, `property`, all
    /// 9.0+ — is available only when the document's dialect intersects its
    /// set.  An unrecognised dialect never restricts.
    fn oo_member_available(&self, grammar: &DefinitionBodyGrammar, subcmd: &str) -> bool {
        let Some(member) = grammar.member(subcmd) else {
            return true;
        };
        match member.surface {
            None => true,
            Some(allowed) => surface_admits(
                allowed,
                Some(&self.analysis_context().context().authoring_query()),
            ),
        }
    }

    /// Emit W002 for a definition-body member the active dialect's grammar
    /// does not have.
    ///
    /// Shared by the three call shapes a member can take — inside a
    /// metaclass's `create` body, inside an `oo::define Cls { … }` block
    /// (both via [`Self::parse_oo_definition_body`]), and the single-command
    /// `oo::define Cls classmethod …` form (via
    /// [`Self::handle_oo_define_command`]) — so all three report the same way
    /// rather than only the bodies being checked.
    ///
    /// Reports **without** dropping the member: the walk still records it,
    /// so go-to-definition, references, rename, document symbols, and code
    /// lenses keep working on the code the user actually wrote.  That
    /// matches the whole-command W002 ([`Self::emit_w002_disabled_command`]),
    /// which likewise reports a dialect-unavailable command while the
    /// analyser goes on modelling the call.  Erasing the member instead
    /// would answer "this needs Tcl 9.0" by silently breaking every editor
    /// feature over it.
    ///
    /// `definer_disabled` bypasses the gate: when the enclosing definer is
    /// itself unavailable (`oo::configurable` under 8.6) its own diagnostic
    /// already covers the construct, and a member word must never cascade
    /// into a second report.
    pub(in super::super) fn emit_w002_oo_member_disabled(
        &mut self,
        grammar: &DefinitionBodyGrammar,
        subcmd: &str,
        tok: Token,
        definer_disabled: bool,
    ) {
        if definer_disabled || self.oo_member_available(grammar, subcmd) {
            return;
        }
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                tcl_core_types::DiagCode::W002,
                tok.span,
                format!(
                    "'{subcmd}' is disabled in the active dialect profile ('{}')",
                    self.dialect()
                ),
                super::super::types::Severity::Warning,
            ));
    }

    /// Emit W002 for a recognised optional member argument that exists in the
    /// grammar but is unavailable in this document's selected dialect.
    ///
    /// Unlike a whole-member gate, this preserves Tcl 8.6's ordinary
    /// `method NAME PARAMS BODY` layout while declining a Tcl 9 visibility
    /// flag in that same member. The option vocabulary and position remain in
    /// [`MemberSpec`]; this is only the generic diagnostic presentation.
    pub(in super::super) fn emit_w002_oo_member_option_disabled(
        &mut self,
        member: &MemberSpec,
        args: &[String],
        arg_tokens: &[Token],
        definer_disabled: bool,
    ) -> bool {
        if definer_disabled {
            return false;
        }
        let Some(option) = member.unavailable_option_for(
            args,
            Some(self.analysis_context().context().authoring_query()),
        ) else {
            return false;
        };
        let Some(position) = member.optional_argument.map(|optional| optional.position) else {
            return false;
        };
        let Some(tok) = arg_tokens.get(usize::from(position)) else {
            return false;
        };
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                tcl_core_types::DiagCode::W002,
                tok.span,
                format!(
                    "'{}' is disabled in the active dialect profile ('{}')",
                    option.value,
                    self.dialect()
                ),
                super::super::types::Severity::Warning,
            ));
        true
    }

    pub(in super::super) fn parse_oo_definition_body(
        &mut self,
        body_text: &str,
        body_tok: Token,
        class_def: &mut ClassDef,
        scope_path: &[usize],
        grammar: Option<&'static DefinitionBodyGrammar>,
        definer_disabled: bool,
    ) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        // Member recognition + argument layout come from the definer's registry
        // grammar; without one (only possible when the analyser has no registry,
        // e.g. a direct unit-test call) there is nothing to walk.
        let Some(grammar) = grammar else {
            return;
        };
        // A Clay class is a fresh TclOO class whose relation members reach
        // `oo::define` as TclOO's do.
        if grammar.profile.members_use_receiver_methods()
            && class_def.source_name.is_some()
            && !class_def.source_name_ambiguous.is_observed()
        {
            class_def.original_relations.begin_declaration();
        }
        // The methods walked below home under the class's qualified name;
        // capture it before the member-collection walk mutates `class_def`.
        let class_qualified = class_def.qualified_name.clone();
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        // A translating Document channel folds CRLF in the cooked value; the
        // member spans below are original offsets, so segment the region.
        let body_text = tcl_source::segment::braced_body_source_region(
            &self.source,
            base_offset as usize,
            body_tok.span.end().saturating_sub(1) as usize,
            body_text,
        )
        .to_owned();
        let cmds = tcl_source::segment::segment_commands_with_offset_and_config(
            &body_text,
            base_offset,
            self.file_lexer_config().nested(),
        );
        let CollectedDefinitionBodies {
            methods: method_bodies,
            accessors: accessor_bodies,
            initialisers: init_bodies,
        } = self.collect_oo_definition_members(
            grammar,
            &cmds,
            class_def,
            scope_path,
            definer_disabled,
        );
        self.original_class_definition_body = None;
        // The display inventory is separate from original source link authority.
        self.collect_oo_links(&method_bodies, class_def);

        // Walk each method / accessor / class-init body in its own
        // `Method` scope with the formal parameters and the class's instance
        // variables pre-bound.
        let class_variables = class_def.variables.clone();
        // Map each declared instance-variable name to its `variable v`
        // declaration name-token span, so the per-method seeding below anchors
        // the object variable's definition at the declaration rather than the
        // whole method body.
        let generation = self.analysis_context();
        let var_decl_spans = self
            .head_identities
            .original_source_image()
            .filter(|image| image.bytes() == self.source.as_bytes())
            .map_or_else(std::collections::HashMap::new, |image| {
                collect_var_decl_spans(
                    grammar,
                    &cmds,
                    &class_variables,
                    image,
                    self.file_lexer_config().nested(),
                    Some(generation.context().authoring_query()),
                )
            });
        // `TclOO` method bodies resolve bare commands globally (object-ns
        // semantics — see `Scope::oo_global_resolution`); snit / itcl
        // members resolve in the type / class namespace.
        let oo_global =
            grammar.member_current_namespace() == MemberCurrentNamespace::RuntimeReceiver;
        // The class-level `initialise` body runs first and in a scope of its
        // own, so the class-scoped variables it declares are visible to this
        // class's own method / accessor bodies below — and to no other
        // class's.
        let mut class_variables = class_variables;
        let mut var_decl_spans = var_decl_spans;
        for mb in &init_bodies {
            for (name, span) in
                self.walk_class_init_body(&class_variables, &class_qualified, scope_path, mb)
            {
                if !class_variables.contains(&name) {
                    class_variables.push(name.clone());
                }
                var_decl_spans.entry(name).or_insert(span);
            }
        }
        for mb in method_bodies.iter().chain(accessor_bodies.iter()) {
            self.walk_method_body(
                &class_variables,
                &var_decl_spans,
                MethodBodyClass {
                    display: &class_qualified,
                    declaration: Some(class_def),
                },
                scope_path,
                mb,
                oo_global,
            );
        }
    }

    /// Collect an OO definition body's members: update the class facts and
    /// retain body-bearing members for the later scope walk.
    fn collect_oo_definition_members(
        &mut self,
        grammar: &DefinitionBodyGrammar,
        cmds: &[tcl_source::segment::SegmentedCommand],
        class_def: &mut ClassDef,
        scope_path: &[usize],
        definer_disabled: bool,
    ) -> CollectedDefinitionBodies {
        let mut bodies = CollectedDefinitionBodies::default();
        for cmd in cmds {
            if !cmd.is_partial && !cmd.argv.is_empty() {
                self.collect_oo_definition_member(
                    grammar,
                    cmd,
                    class_def,
                    scope_path,
                    definer_disabled,
                    &mut bodies,
                );
            }
        }
        bodies
    }

    /// Process one segmented OO definition member using only its registered
    /// grammar, including wrapper expansion and profile-aware argument roles.
    fn collect_oo_definition_member(
        &mut self,
        grammar: &DefinitionBodyGrammar,
        cmd: &tcl_source::segment::SegmentedCommand,
        class_def: &mut ClassDef,
        scope_path: &[usize],
        definer_disabled: bool,
        bodies: &mut CollectedDefinitionBodies,
    ) {
        self.retain_original_member_metadata(grammar, cmd, class_def, scope_path);
        self.retain_logical_relation_command(grammar, cmd, class_def);
        let spliced = splice_static_member_expansions(&self.source, cmd, self.file_lexer_config());
        let (texts, argv) = spliced.as_ref().map_or(
            (cmd.texts.as_slice(), cmd.argv.as_slice()),
            |(texts, argv)| (texts.as_slice(), argv.as_slice()),
        );
        if let (Some(subcmd), Some(token)) = (texts.first(), argv.first()) {
            self.emit_w002_oo_member_disabled(grammar, subcmd, *token, definer_disabled);
        }
        if let Some(member) = texts.first().and_then(|subcmd| grammar.member(subcmd))
            && self.emit_w002_oo_member_option_disabled(
                member,
                &texts[1..],
                &argv[1..],
                definer_disabled,
            )
        {
            return;
        }
        if self.member_declaration_is_opaque(grammar, cmd, texts) {
            class_def.member_set_incomplete = true;
            self.record_loop_installed_members(grammar, cmd, class_def);
        }
        let expanded = expand_wrapper_block_members(
            &self.source,
            grammar,
            texts,
            argv,
            self.file_lexer_config(),
        );
        if expanded.is_some() && self.selected_logical_declaration_advice() {
            class_def
                .original_members
                .mark_logical_relation_coverage_unavailable();
        }
        let member_calls: Vec<(&[String], &[Token])> = expanded.as_ref().map_or_else(
            || vec![(texts, argv)],
            |inner| {
                inner
                    .iter()
                    .map(|(texts, argv)| (texts.as_slice(), argv.as_slice()))
                    .collect()
            },
        );
        for (texts, argv) in member_calls {
            if let Some((keyword, args)) = texts.split_first()
                && let Some(member) = grammar.member(keyword)
            {
                let tokens = argv.get(1..).unwrap_or(&[]);
                let parameter_indices =
                    self.member_formal_parameter_indices(grammar, member, args, tokens, class_def);
                super::super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
                    self,
                    args,
                    tokens,
                    &parameter_indices,
                );
            }
            apply_oo_subcommand_in(
                grammar,
                texts,
                argv,
                class_def,
                MemberDialect {
                    surface: Some(self.analysis_context().context().authoring_query()),
                    rules: self.word_rules(),
                },
            );
            self.retain_class_relation_lookups(grammar, texts, class_def, scope_path);
            self.record_member_command_references(grammar, texts, argv, scope_path);
            if let Some(method) = collect_method_body(
                grammar,
                texts,
                argv,
                Some(self.analysis_context().context().authoring_query()),
                self.word_rules(),
            ) {
                bodies.methods.push(method);
            }
        }
        collect_class_level_bodies(
            grammar,
            texts,
            argv,
            Some(self.analysis_context().context().authoring_query()),
            &mut bodies.accessors,
            &mut bodies.initialisers,
        );
    }
}

pub(in super::super) fn parse_oo_define_inline_in(
    analyser: &mut Analyser,
    grammar: &DefinitionBodyGrammar,
    args: &[String],
    arg_tokens: &[Token],
    class_def: &mut ClassDef,
    dialect: Option<SurfaceQuery<'_>>,
    scope_path: &[usize],
) {
    if args.is_empty() {
        return;
    }
    if args
        .first()
        .and_then(|subcmd| grammar.member(subcmd))
        .is_some_and(|member| member.unavailable_option_for(&args[1..], dialect).is_some())
    {
        return;
    }
    analyser
        .retain_original_inline_member_metadata(grammar, args, arg_tokens, class_def, scope_path);
    if let Some(member) = args.first().and_then(|subcmd| grammar.member(subcmd)) {
        let parameter_indices = analyser.member_formal_parameter_indices(
            grammar,
            member,
            &args[1..],
            arg_tokens.get(1..).unwrap_or(&[]),
            class_def,
        );
        super::super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
            analyser,
            &args[1..],
            arg_tokens.get(1..).unwrap_or(&[]),
            &parameter_indices,
        );
    }
    // Synthesise a single fake "command" matching what the
    // body walker would have produced.
    apply_oo_subcommand_in(
        grammar,
        args,
        arg_tokens,
        class_def,
        MemberDialect {
            surface: dialect,
            rules: analyser.word_rules(),
        },
    );
    analyser.retain_class_relation_lookups(grammar, args, class_def, scope_path);
}

/// Collect the two class-level (non-method) body kinds a definition body
/// can carry: a `property`'s `-get`/`-set` accessor scripts, and an
/// `initialise`/`initialize` block.
///
/// Neither is a method frame, so [`collect_method_body`] — restricted to the
/// four real method-bearing members — does not see them, yet each needs its
/// own scope in the per-body walk. Split out of `parse_oo_definition_body`
/// purely to
/// keep that walker within its line budget.
pub(super) fn collect_class_level_bodies(
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    dialect: Option<SurfaceQuery<'_>>,
    accessor_bodies: &mut Vec<CollectedMethodBody>,
    init_bodies: &mut Vec<CollectedMethodBody>,
) {
    // Written directly in the class body — a wrapped spelling is no
    // class-level script of the class's own.
    let Some(statement) = MemberStatement::read(grammar, texts, argv, dialect)
        .filter(|statement| statement.row.keyword_index == 0)
    else {
        return;
    };
    match statement.landing() {
        MemberLanding::Accessors => collect_property_accessor_bodies(texts, argv, accessor_bodies),
        // A wrapper's block form is an init script too, but its members are
        // read one by one ([`expand_wrapper_block_members`]); only a member
        // of its own runs as the class's init script.
        MemberLanding::InitScript if statement.member.kind == MemberKind::Flat => {
            if let Some(body_idx) = statement.row.body.map(|body| body.0)
                && let (Some(body), Some(tok)) = (texts.get(body_idx), argv.get(body_idx).copied())
                && tok.kind == TokenType::Str
            {
                init_bodies.push(CollectedMethodBody {
                    name: format!("<{}>", statement.keyword()),
                    params: Vec::new(),
                    body_text: body.clone(),
                    body_tok: tok,
                    params_tok: None,
                    // A class-level init script is not a method frame at
                    // all — `self` raises there.
                    class_side: true,
                });
            }
        }
        _ => {}
    }
}

/// One definition-body member command's words with every **statically
/// determined** `{*}` expansion already spliced, or `None` when the command
/// has no expansion the analyser can resolve (the overwhelmingly common
/// case, kept allocation-free).
///
/// Tcl expands `{*}` during parsing, so a `{*}`-marked word whose text is a
/// literal is *not* one word — it is the elements of the list it holds, and
/// the member grammar's fixed argument layout applies to those.  Without
/// this normalisation `constructor {*}{args {…}}` / `method
/// {*}{foo {} {…}}` look one word short of their grammar and the member is
/// dropped entirely, with no diagnostic.  Verified
/// against tclsh 9.0.4 and 8.6.16: both forms define a real, callable
/// member, in the `oo::class create` body *and* the `oo::define` body.
///
/// Only a single **braced** (`Str`) word splices: braces suppress
/// substitution, so its element list is exactly what the runtime sees.  A
/// `{*}` over a variable or command substitution (`{*}[info class
/// definition …]`, the ticklecharts `chart3D` reflection idiom) has no
/// statically-knowable element list and is left verbatim — the member then
/// abstains exactly as before rather than being recorded with invented
/// parameters or a body span pointing at the wrong text.
///
/// Element spans come from the segmenter
/// ([`tcl_source::segment::flatten_clause_list_elements_with_config`]), so each spliced
/// word carries its own true source range and downstream consumers
/// (document symbols, `my`-dispatch, hover) get real positions rather than
/// the whole-list span.
/// Expand a wrapper member's bare **script-block** form into the equivalent
/// prefix forms, so one walker handles both spellings of the same declaration.
///
/// `TclOO`'s `self` and `private` are the two members the registry marks
/// [`MemberKind::Wrapper`] *with* `wrapper_block_body`: each takes both
/// `self method NAME ARGS BODY` (prefix) and `self { method NAME ARGS BODY; … }`
/// (block, a definition script evaluated against the class object / with
/// private visibility).  Only the prefix form was ever walked, so every member
/// declared in a block was invisible to the whole analysis — no `ClassDef`
/// entry, hence no document-symbol node, no dispatch arity, and no body walk.
///
/// Returns `Some(calls)` — each `(texts, argv)` being the block's inner command
/// with the wrapper keyword (and its token) spliced back on at index 0 — for a
/// block form, and `None` for everything else, including the prefix form, which
/// the caller already handles as written.
///
/// Oracle, identical on tclsh 9.0.4 and 8.6.16 — the block form declares
/// exactly the same class-side methods the prefix form does:
///
/// ```tcl
/// oo::class create ::C {
///     self { method make {n} {return "made-$n"} ; method other {} {return other} }
///     method inst {} {return inst}
/// }
/// ::C make 7                  ;# -> made-7
/// info class methods ::C      ;# -> inst          (instance side untouched)
/// info object methods ::C     ;# -> other make    (class-object side)
/// oo::class create ::F { superclass ::C }
/// ::F make 1                  ;# -> error: unknown method "make"  (not inherited)
/// ```
///
/// and `export`/`unexport` inside the block act on the class-object side:
/// `oo::class create ::E { self { method hidden {} {…} ; unexport hidden } }`
/// leaves `info object methods ::E` empty while `-all -private` still lists
/// `hidden`.
///
/// Abstains — returning `None`, so the command is walked exactly as it is
/// today — on a dynamic (non-`Str`) block word, which cannot be re-segmented
/// statically, and on any shape carrying words after the block.
pub(super) fn expand_wrapper_block_members(
    source: &str,
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<(Vec<String>, Vec<Token>)>> {
    use tcl_registry::definer::MemberKind;
    let keyword = texts.first()?;
    let keyword_tok = *argv.first()?;
    let member = grammar.member(keyword)?;
    if member.kind != MemberKind::Wrapper || !member.wrapper_block_body {
        return None;
    }
    // The prefix form — the word after the wrapper is itself a member keyword —
    // is already the spelling the walker consumes; leave it alone.
    let inner_first = texts.get(1)?;
    if grammar.member(inner_first).is_some() {
        return None;
    }
    // This is the wrapper's own fixed one-body layout, not an inner member
    // layout. The recursively spliced member goes back through the
    // profile-aware caller, so a Tcl 9 option cannot be accepted here under
    // Tcl 8.6.
    // `+ 1` because a member's registry arg-role indices are relative to its
    // *arguments*, while `texts`/`argv` still carry the keyword at 0.
    let body_idx = member.indices_for(ArgRole::Body).next()? + 1;
    if texts.len() != body_idx + 1 {
        return None;
    }
    let body_text = texts.get(body_idx)?;
    let body_tok = *argv.get(body_idx)?;
    if body_tok.kind != TokenType::Str {
        return None;
    }
    let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
    let cmds = tcl_source::segment::segment_commands_with_offset_and_config(
        body_text,
        base_offset,
        config.nested(),
    );
    let mut out: Vec<(Vec<String>, Vec<Token>)> = Vec::new();
    for cmd in &cmds {
        if cmd.is_partial || cmd.argv.is_empty() {
            continue;
        }
        let spliced = splice_static_member_expansions(source, cmd, config);
        let (inner_texts, inner_argv) = spliced
            .as_ref()
            .map_or((cmd.texts.as_slice(), cmd.argv.as_slice()), |(t, a)| {
                (t.as_slice(), a.as_slice())
            });
        let mut wrapped_texts = Vec::with_capacity(inner_texts.len() + 1);
        wrapped_texts.push(keyword.clone());
        wrapped_texts.extend(inner_texts.iter().cloned());
        let mut wrapped_argv = Vec::with_capacity(inner_argv.len() + 1);
        wrapped_argv.push(keyword_tok);
        wrapped_argv.extend(inner_argv.iter().copied());
        out.push((wrapped_texts, wrapped_argv));
    }
    Some(out)
}

pub(super) fn splice_static_member_expansions(
    source: &str,
    cmd: &tcl_source::segment::SegmentedCommand,
    config: tcl_lexer::LexerConfig,
) -> Option<(Vec<String>, Vec<Token>)> {
    let expand = cmd.expand_word.as_ref()?;
    if !expand.iter().any(|&e| e) {
        return None;
    }
    let mut texts: Vec<String> = Vec::with_capacity(cmd.texts.len());
    let mut argv: Vec<Token> = Vec::with_capacity(cmd.argv.len());
    let mut spliced = false;
    for (i, (text, tok)) in cmd.texts.iter().zip(cmd.argv.iter()).enumerate() {
        let is_static_expansion = expand.get(i).copied().unwrap_or(false)
            && tok.kind == TokenType::Str
            && cmd.single_token_word.get(i).copied().unwrap_or(false);
        if is_static_expansion {
            spliced = true;
            for (element, element_tok) in
                tcl_source::segment::flatten_clause_list_elements_with_config(
                    source, text, *tok, config,
                )
            {
                texts.push(element);
                argv.push(element_tok);
            }
            continue;
        }
        texts.push(text.clone());
        argv.push(*tok);
    }
    spliced.then_some((texts, argv))
}

/// Recognise a method-defining statement in a class body and return its body
/// to walk in a fresh [`ScopeKind::Method`] scope.
///
/// Whether the statement opens a method frame is its registry row's answer (a
/// callable landing in a method table), and the member's argument layout —
/// which word is the name, the parameter list, and the recursable body — comes
/// entirely from its registry [`MemberSpec`] arg-roles (never hardcoded
/// indices), so a definer that adds or reshapes a method-bearing member is
/// picked up from the grammar alone.
///
/// A class-level init script (`initialise` / `initialize`, walked in a scope
/// of its own by the caller) and a wrapper's block form are not method frames,
/// even though the grammar marks each as carrying a body. A forward has no
/// body; dynamic (non-braced) bodies are filtered downstream by
/// [`Analyser::walk_method_body`].
fn collect_method_body(
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    dialect: Option<SurfaceQuery<'_>>,
    rules: WordValueRules,
) -> Option<CollectedMethodBody> {
    // A wrapped member (`self method …`, `private method …`) is read through
    // its wrapper: its body would otherwise never be walked at all (no internal
    // diagnostics inside a `self method`/`private method` body — confirmed
    // empirically, a deliberately-wrong-arity call inside one drew nothing,
    // while the identical call in a plain `method` body correctly fired). The
    // row's keyword is the wrapped member's own, so the slices below start
    // right after the *effective* keyword either way.
    let statement = MemberStatement::read(grammar, texts, argv, dialect)?;
    if !matches!(statement.landing(), MemberLanding::Method(_)) {
        return None;
    }
    // A member on the class object (`classmethod`, `self method`) runs in a
    // class-side frame, where `[self class]` never answers the written class
    // (tclsh 9.0.4: it raises "method not defined by a class" in a
    // `self method`, and answers the internal `::oo::ObjN:: oo ::delegate`
    // class in a `classmethod`).
    let class_side = statement.row.receiver != MemberReceiver::Instance;
    let (member, keyword, texts, argv) = (
        statement.member,
        statement.keyword(),
        statement.args(),
        statement.arg_tokens(),
    );
    let body_idx = member
        .indices_for_call_in(texts, dialect, ArgRole::Body)
        .next()?;
    let params_idx = member
        .indices_for_call_in(texts, dialect, ArgRole::ParamList)
        .next();
    let name_idx = member
        .indices_for_call_in(texts, dialect, ArgRole::Name)
        .next();

    let body_text = texts.get(body_idx)?.clone();
    let body_tok = *argv.get(body_idx)?;
    // A named member (`method NAME …`) uses its name word; the nameless forms
    // (`constructor` / `destructor`) get a synthetic `<keyword>` name.
    let name = name_idx
        .and_then(|i| texts.get(i))
        .map_or_else(|| format!("<{keyword}>"), String::clone);
    let params = params_idx
        .and_then(|i| texts.get(i))
        .map_or_else(Vec::new, |p| parse_param_list(p, rules));
    let params_tok = params_idx.and_then(|i| argv.get(i).copied());
    Some(CollectedMethodBody {
        name,
        params,
        body_text,
        body_tok,
        params_tok,
        class_side,
    })
}
