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

//! Class factories: the factory a metaclass publishes, the word layout its manufacturer imposes and the members it injects.

use super::super::super::state::Analyser;
use super::super::super::types::{ClassDef, ClassFactory, FactoryMember, FactoryWord, MemberSide};
use super::{InjectedMember, ManufacturerLayout, ResolvedClassCreation, UserMetaclass};
use crate::parsing::syntax::descend::descend_token;
use crate::parsing::syntax::segment::segments_from_tree;
use tcl_lexer::{Token, TokenType};

impl ManufacturerLayout {
    /// A registry-declared manufacturer's own word layout, with nothing
    /// injected. Anonymous manufacturers (`new`) and ordinary-instance
    /// manufacturers (snit's type `create`) have no statically nameable
    /// class definition and therefore return `None`.
    pub(in super::super) fn builtin(
        method: &tcl_registry::definer::ManufacturerMethod,
    ) -> Option<Self> {
        Some(Self {
            name_arg: usize::from(method.names_instance_at?),
            body_arg: usize::from(method.definition_body_at?),
            injected: Vec::new(),
            inheritance_unknown: false,
        })
    }
}

/// One piece of a manufacturer prologue word, as
/// [`Analyser::manufacturer_injected_template`] accounts for it.
#[derive(Debug, PartialEq, Eq)]
pub(in super::super) enum ProloguePiece<'a> {
    /// A `[…]` command substitution — one prologue member to read.
    Substitution(Token),
    /// A `$name` / `${name}` variable read.
    VarRead(&'a str),
    /// Whitespace, a statement separator, or an escaped separator — text
    /// that joins pieces without contributing any member of its own.
    Separator,
    /// Anything else: literal prologue text, or a fragment the scanner
    /// cannot classify.  Its presence means the prologue is *not* fully
    /// read.
    Opaque,
}

/// Read original executable components without treating literal text as a
/// substituted parameter. Unsupported components withdraw the whole template.
pub(in super::super) fn prologue_pieces(
    original: &tcl_lexer::NativeWord,
) -> Vec<ProloguePiece<'_>> {
    let arena = original.executable_parts();
    let source_map = tcl_lexer::SourceMap::from_image(original.image());
    let mut pieces = Vec::new();
    for part in arena.list(arena.root()) {
        let piece = match part.part {
            tcl_lexer::ExecutablePart::Text(_) => {
                if arena.text(part).is_some_and(|text| {
                    text.iter()
                        .all(|byte| byte.is_ascii_whitespace() || *byte == b';')
                }) {
                    ProloguePiece::Separator
                } else {
                    ProloguePiece::Opaque
                }
            }
            tcl_lexer::ExecutablePart::Variable { name, index: None } => arena
                .bytes(name)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map_or(ProloguePiece::Opaque, ProloguePiece::VarRead),
            tcl_lexer::ExecutablePart::Command { body } => {
                let mut groups = original.tokens().iter().filter(|token| {
                    token.kind == TokenType::Cmd
                        && token.span.start() == part.span.start()
                        && Some(source_map.token_text(**token).as_bytes()) == arena.bytes(body)
                });
                match (groups.next(), groups.next()) {
                    (Some(group), None) => ProloguePiece::Substitution(*group),
                    _ => ProloguePiece::Opaque,
                }
            }
            tcl_lexer::ExecutablePart::Variable { index: Some(_), .. }
            | tcl_lexer::ExecutablePart::Expression { .. }
            | tcl_lexer::ExecutablePart::ParseError(_) => ProloguePiece::Opaque,
        };
        let opaque = piece == ProloguePiece::Opaque;
        if piece != ProloguePiece::Separator || pieces.last() != Some(&piece) {
            pieces.push(piece);
        }
        if opaque {
            break;
        }
    }
    pieces
}

/// Resolve one injected-member template against the creation call that
/// triggered it.
///
/// The literal words keep the tokens they were templated with; a
/// [`FactoryWord::CallerSplice`] contributes the call argument's literal list
/// elements, each with its own call-site token.  `None` when this call's
/// argument is not a statically-known list — the caller then marks the
/// class's inheritance unknown rather than claiming an injection it cannot
/// spell.
pub(in super::super) fn resolve_factory_member(
    member: &FactoryMember,
    original_arguments: Option<&[tcl_lexer::NativeWord]>,
) -> Option<InjectedMember> {
    let mut member_words: Vec<String> = Vec::new();
    let mut member_tokens: Vec<Token> = Vec::new();
    for word in &member.words {
        match word {
            FactoryWord::Literal { text, token } => {
                member_words.push(text.clone());
                member_tokens.push(*token);
            }
            FactoryWord::CallerSplice(arg_index) => {
                let original = original_arguments?.get(*arg_index)?;
                for element in tcl_syntax::word_rules::original_static_word_list_elements(original)?
                {
                    // This token is a display anchor for an actual list field,
                    // never an independently captured word or naming input.
                    let span = element.source_span()?;
                    member_words.push(element.value().to_owned());
                    member_tokens.push(Token {
                        kind: TokenType::Esc,
                        span,
                        content_offset: 0,
                        in_quote: false,
                    });
                }
            }
        }
    }
    Some(InjectedMember {
        texts: member_words,
        argv: member_tokens,
    })
}

/// The word layout a class factory's `args[0]` manufacturer imposes on this
/// creation call, with its injected members resolved against the call's own
/// arguments.
///
/// A subcommand the factory does not override runs the inherited `oo::class`
/// manufacturer, so the builtin `create Name Body` layout applies with
/// nothing injected.
pub(super) fn manufacturer_layout(
    factory: &ClassFactory,
    builtin: &tcl_registry::definer::ManufacturerMethod,
    args: &[String],
    original_arguments: Option<&[tcl_lexer::NativeWord]>,
) -> Option<ManufacturerLayout> {
    let Some(spec) = factory.overrides.get(&args[0]) else {
        return ManufacturerLayout::builtin(builtin);
    };
    let injected = spec.injected.as_ref().and_then(|members| {
        members
            .iter()
            .map(|m| resolve_factory_member(m, original_arguments))
            .collect::<Option<Vec<_>>>()
    });
    Some(ManufacturerLayout {
        name_arg: spec.name_arg,
        body_arg: spec.body_arg,
        inheritance_unknown: injected.is_none(),
        injected: injected.unwrap_or_default(),
    })
}

impl Analyser {
    /// The [`ClassFactory`] `class` publishes, when `class` is itself a
    /// `TclOO` class factory — a class whose superclass chain reaches a
    /// registry metaclass.
    ///
    /// Derived **once, where the metaclass is written**, so every consumer
    /// (this file's own `Meta create …` calls and, through the workspace
    /// factory index, another file's) classifies a creation call from the
    /// same proved fact instead of re-deriving it — or, cross-file,
    /// abstaining for want of it.
    ///
    /// A user metaclass that does not override a manufacturer subcommand
    /// inherits `oo::class`'s own `create Name Body` shape, so that
    /// subcommand simply has no entry and the builtin layout applies.  One
    /// that *does* override it declares its own shape in the override's
    /// parameter list, and that override is read rather than guessed: Tk's
    /// `self method create {name superclasses body}` puts the body at
    /// argument 3, not 2, and splices a superclass the caller never wrote.
    pub(in super::super) fn class_factory_of(&self, class: &ClassDef) -> Option<ClassFactory> {
        let meta = self.user_metaclass_of_class(class)?;
        let members =
            super::super::super::class_hierarchy::original_metadata::original_definition_members_for_class(
                &self.result,
                class,
                MemberSide::ClassObject,
            )?;
        let mut overrides = std::collections::BTreeMap::new();
        let mut exported_manufacturers = std::collections::BTreeSet::new();
        for builtin in meta.grammar.manufacturers {
            let own = members.iter().find(|method| {
                method.original_name_input().policy() == meta.root_policy
                    && method.original_name_input().bytes() == builtin.keyword.as_bytes()
            });
            let exported = if let Some(method) = own {
                let override_def = method.original_parameter_names_and_body(&self.result)?;
                let lexical =
                    super::super::super::types::OriginalLexicalMemberContext::from_original_definition_method(
                        &self.result,
                        class,
                        method,
                    )?;
                let next = self.manufacturer_next_call(&override_def, &lexical)?;
                let arguments = next.original_direct_argument_words()?;
                let (name_arg, body_arg) =
                    Self::manufacturer_word_positions(&override_def, builtin, &arguments)?;
                let body_argument = usize::from(builtin.definition_body_at?).checked_sub(1)?;
                let prologue = arguments.get(body_argument)?;
                overrides.insert(
                    builtin.keyword.to_owned(),
                    super::super::super::types::ManufacturerSpec {
                        name_arg,
                        body_arg,
                        injected: self.manufacturer_injected_template(
                            &meta,
                            &override_def,
                            prologue,
                        ),
                    },
                );
                method.exported()
            } else {
                class
                    .original_members
                    .inherited_visibility_for_metadata_keyword(
                        MemberSide::ClassObject,
                        meta.root_policy,
                        builtin.keyword,
                        builtin.visibility == tcl_registry::definer::MemberVisibility::Exported,
                    )?
            };
            if exported {
                exported_manufacturers.insert(builtin.keyword.to_owned());
            }
        }
        let unknown_binds_instance = self.unknown_dispatch_binds_instance(&meta, class);
        Some(ClassFactory {
            root_metaclass: meta.root_command,
            overrides,
            exported_manufacturers,
            unknown_binds_instance,
        })
    }

    /// Original formal arguments supplying the selected manufacturer's name
    /// and body roles. Unrepresented or transformed contributions refuse.
    fn manufacturer_word_positions(
        original: &super::super::super::types::OriginalSourceMethodNameBody<'_>,
        builtin: &tcl_registry::definer::ManufacturerMethod,
        arguments: &[&tcl_lexer::NativeWord],
    ) -> Option<(usize, usize)> {
        let name = usize::from(builtin.names_instance_at?).checked_sub(1)?;
        let body = usize::from(builtin.definition_body_at?).checked_sub(1)?;
        let name_arg = original
            .scalar_parameter_read(arguments.get(name)?)?
            .checked_add(1)?;
        let body_arg = original
            .body_parameter_read(arguments.get(body)?)?
            .checked_add(1)?;
        (name_arg != body_arg).then_some((name_arg, body_arg))
    }

    pub(super) fn manufacturer_next_call(
        &self,
        original: &super::super::super::types::OriginalSourceMethodNameBody<'_>,
        context: &super::super::super::types::OriginalLexicalMemberContext,
    ) -> Option<super::super::super::types::OriginalLexicalNextCall> {
        if context.body_word() != original.body_word() {
            return None;
        }
        let word = original.body_word();
        let body_span = word.content_span().ok()?;
        let body = word.image().try_text().ok()?.get(body_span.as_range())?;
        crate::segmenter::segment_commands_with_offset_and_config(
            body,
            body_span.start(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript.config(word.config()),
        )
        .into_iter()
        .filter_map(|command| {
            let head = command.argv.first()?;
            super::super::super::types::OriginalLexicalNextCall::from_source(
                context,
                &self.result,
                &self.source,
                head,
            )
        })
        .find(|call| !call.has_target_class())
    }

    /// The definition-body members the manufacturer splices into every class
    /// it makes, as a template over the creation call's arguments.
    ///
    /// Tk's `Megawidget` hands `next` a `[list superclass ::tk::MegawidgetClass
    /// {*}$superclasses]` prologue, so every class it makes really does
    /// inherit `::tk::MegawidgetClass` plus whatever the caller passed —
    /// tclsh 9.0.4 and 8.6.16 both report exactly that from `info class
    /// superclasses`.  Only **reference-only** members (`superclass`,
    /// `mixin`, `filter`, `export`, … — the grammar's `all_args_ref` set)
    /// are injected: they name existing entities, so every injected word
    /// keeps a real source span, either in the manufacturer's own body
    /// (a literal it always splices) or in the call's arguments (a
    /// `{*}$param` splice, resolved per call by
    /// [`resolve_factory_member`]).
    ///
    /// **Reading the whole prologue is a precondition, not a best effort.**
    /// The definition word `next` receives is scanned piece by piece, and
    /// every piece must be one the analyser can account for: a `[list
    /// <member> …]` group it parsed, the manufacturer's own `$param` read
    /// (the caller's body, spliced in verbatim), or list/statement
    /// separator text.  Anything else — most importantly a **string-built**
    /// prologue such as `next $name "superclass Base\n$body"`, which
    /// injects a superclass with no nested command at all (tclsh 9.0.4 /
    /// 8.6.16: `info class superclasses` really reports `::Base`, and its
    /// methods really are inherited) — yields `None`, and the class is
    /// recorded with its inheritance marked unknown.  Returning a
    /// *known-empty* injection for a prologue that was merely unreadable
    /// would claim the class has no superclass and let W308 fire on
    /// perfectly good inherited methods.
    fn manufacturer_injected_template(
        &self,
        meta: &UserMetaclass,
        original: &super::super::super::types::OriginalSourceMethodNameBody<'_>,
        prologue: &tcl_lexer::NativeWord,
    ) -> Option<Vec<FactoryMember>> {
        let params = original.parameter_names().collect::<Vec<_>>();
        let mut injected: Vec<FactoryMember> = Vec::new();
        for piece in prologue_pieces(prologue) {
            match piece {
                ProloguePiece::Separator => {}
                ProloguePiece::VarRead(name) if params.contains(&name) => {}
                ProloguePiece::Substitution(group) => {
                    injected.push(self.injected_member_from_group(meta, group, original)?);
                }
                // Literal prologue text, a read of something that is not a
                // manufacturer parameter, or an unreadable fragment: the
                // prologue is not fully accounted for.
                ProloguePiece::VarRead(_) | ProloguePiece::Opaque => return None,
            }
        }
        Some(injected)
    }

    /// The definition-body member one `[…]` group of a manufacturer
    /// prologue installs, or `None` when the group is not a
    /// reference-only member built by a source-selected list-argument constructor.
    pub(in super::super) fn injected_member_from_group(
        &self,
        meta: &UserMetaclass,
        group: Token,
        original: &super::super::super::types::OriginalSourceMethodNameBody<'_>,
    ) -> Option<FactoryMember> {
        let sm = Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let descended = descend_token(&sm, group, self.file_lexer_config());
        let mut segs = segments_from_tree(descended.tree(), &sm).into_iter();
        let seg = segs.next()?;
        // One group builds one member; a `[a; b]` compound is not a shape
        // the prologue reader accounts for.
        if segs.next().is_some() {
            return None;
        }
        // The registry words confirm the selected list constructor. Their
        // argument vector is the one recorded when the body was entered,
        // where a `{*}` splice has already expanded to that call's element
        // count, so the template itself is read from the written words.
        let source_words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &seg,
        )?;
        let context = self.result.resolved_input.as_ref()?.context_registry();
        source_words.with_source_schema(&context, |schema| {
            schema.authored_source_list_operand_templates()
        })??;
        if source_words.origins().iter().skip(1).any(|origin| {
            !matches!(
                origin,
                crate::registry_invocation::InvocationWordOrigin::Written(_)
            )
        }) {
            return None;
        }
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.file_lexer_config();
        let mut words = Vec::new();
        for token in seg.arg_tokens() {
            let (word, _) = [tcl_lexer::word_span(&sm, *token), token.span]
                .into_iter()
                .find_map(|span| {
                    self.head_identities
                        .original_written_word_at_span_in_source(&image, span, config)
                })?;
            let value = if word.group().expand {
                FactoryWord::CallerSplice(original.scalar_parameter_splice(&word)?.checked_add(1)?)
            } else {
                FactoryWord::Literal {
                    text: String::from_utf8(
                        tcl_syntax::word_rules::original_static_word_unicode_value(&word)?,
                    )
                    .ok()?,
                    token: *token,
                }
            };
            words.push(value);
        }
        let FactoryWord::Literal { text: keyword, .. } = words.first()? else {
            return None;
        };
        meta.grammar.member(keyword)?.all_args_ref?;
        Some(FactoryMember { words })
    }

    /// Readonly factory metadata and its same original call arguments.
    /// Foreign headers require the source graph's separate occupancy facet;
    /// neither tier proves that a custom manufacturer executes or completes.
    pub(in super::super) fn class_factory_for_command(
        &self,
        args: &[String],
        arg_tokens: &[Token],
        call_off: u32,
    ) -> Option<(ClassFactory, Vec<tcl_lexer::NativeWord>)> {
        if let Some(call) = crate::registry_invocation::source_structure::source_constructor_call_at(
            &self.source,
            &self.result,
            call_off,
        ) {
            if (0..arg_tokens.len()).any(|index| call.written_argument(index) != Some(index))
                || !super::super::super::original_definer::constructor_arguments_match(
                    &call, args, arg_tokens,
                )
            {
                return None;
            }
            let declaration = call.class_declaration();
            let class = if self.result.allows_retained_logical_declaration_advice() {
                declaration.logical_source_class(&self.result)?
            } else {
                declaration.source_class(&self.result)?.metadata()
            };
            let original_arguments = (0..arg_tokens.len())
                .map(|ordinal| call.argument_word(ordinal).cloned())
                .collect::<Option<Vec<_>>>()?;
            return Some((class.factory.clone()?, original_arguments));
        }
        let request =
            crate::registry_invocation::source_structure::source_logical_command_candidates_at(
                &self.source,
                &self.result,
                call_off,
            )?;
        if request.has_earlier_unknown_mutations()
            || request.has_lookup_operand_effects()
            || request.argument_count() != Some(arg_tokens.len())
            || !super::super::super::original_definer::logical_arguments_match(
                request.original_words(),
                args,
                arg_tokens,
            )
        {
            return None;
        }
        let input = self.result.resolved_input.as_ref()?;
        let policy = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
            .authored_name_policy()?;
        let index = self.workspace_class_factories.as_ref()?;
        let foreign = request
            .external_header_coordinates()?
            .iter()
            .find_map(|coordinate| {
                // Project an existing slot outward to the legacy index only
                // when the shared recipe preserves every component boundary.
                let key = tcl_syntax::naming::native_command_source_spelling(
                    policy.recipe(),
                    coordinate,
                )?;
                index.get(&key)
            })?;
        Some((
            self.foreign_factory_at_call(foreign, arg_tokens)?,
            request.original_words().iter().skip(1).cloned().collect(),
        ))
    }

    pub(super) fn foreign_factory_at_call(
        &self,
        foreign: &ClassFactory,
        arg_tokens: &[Token],
    ) -> Option<ClassFactory> {
        // Tokens in the foreign factory index the metaclass's document. The
        // creation call's own subcommand word (`create`) is the nearest real
        // token in *this* document that the injected member genuinely came
        // with, and it is guaranteed present — the caller has already checked
        // `arg_tokens.len() >= 2`.
        let elsewhere = *arg_tokens.first()?;
        let grammar = self.definition_grammar(&foreign.root_metaclass)?;
        Some(foreign.resolve_in_other_document(elsewhere, &|keyword| {
            grammar
                .member(keyword)
                .is_some_and(|m| m.retraction.is_some())
        }))
    }

    /// The registry metaclass at the root of `seed`'s superclass chain, when
    /// `seed` (recorded, or being recorded, under `qualified`) is **itself a
    /// class factory** — a user-defined `TclOO` metaclass.
    ///
    /// A class is a factory when its (possibly indirect) superclass chain
    /// reaches a command the registry marks [`tcl_registry::Traits::IS_OO_METACLASS`]
    /// with a `TclOo` definition-body grammar — the same seed
    /// [`Self::handle_oo_class_command`] uses for a written-out
    /// `oo::class create`.  Only the inheritance step is added here, and
    /// that is `TclOO` language semantics (`oo::class`'s subclasses
    /// manufacture classes), not per-command knowledge.
    ///
    /// The returned grammar is the *root registry metaclass's* own
    /// definition-body grammar, so the bodies this factory makes are walked
    /// by exactly the grammar the language gives them without the walker
    /// naming a metaclass command.
    ///
    /// Superclass words retain the original definition caller's namespace,
    /// independently of the manufactured class's holder or reported name.
    /// Missing relation context supplies no superclass identity.
    ///
    /// Chain walking is depth-bounded and visited-checked, so a cyclic
    /// `superclass` declaration (rejected by real Tcl, but writable in a
    /// half-edited buffer) terminates.
    pub(in super::super) fn user_metaclass_of_class(
        &self,
        seed: &ClassDef,
    ) -> Option<UserMetaclass> {
        let context = self.analysis_context();
        let provider =
            super::super::super::class_hierarchy::original_metadata::original_registry_metaclass_provider(
                &self.result,
                seed,
                &context,
            )?;
        let descriptor = provider.registry_descriptor(&context)?;
        Some(UserMetaclass {
            root_command: descriptor.name.to_owned(),
            root_policy: provider.policy(),
            root_slot: provider.source_slot().clone(),
            grammar: descriptor.definition_body?,
        })
    }

    /// Classify one possible class-manufacturing call and resolve its layout.
    /// Both registry definers and published user-metaclass factories converge
    /// here, so the main recorder only handles the resulting structural data.
    pub(in super::super) fn resolve_class_creation(
        &self,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) -> Option<ResolvedClassCreation> {
        if args.is_empty() || arg_tokens.is_empty() {
            return None;
        }
        let context = self.analysis_context();
        let original_definer =
            super::super::super::original_definer::OriginalClassDefinerSource::capture(
                &self.source,
                &self.result,
                cmd_tok.span.start(),
            );
        let registry_definer = if let Some(original) = &original_definer {
            original.matches_arguments(args, arg_tokens).then_some(())?;
            let descriptor = original.registry_descriptor(&context)?;
            (descriptor
                .traits
                .contains(tcl_registry::Traits::IS_OO_METACLASS)
                && descriptor.definition_body.is_some_and(|grammar| {
                    grammar.family == tcl_registry::definer::DefinerFamily::TclOo
                }))
            .then_some(())?;
            Some(descriptor)
        } else {
            None
        };
        let user_factory = if registry_definer.is_some() {
            None
        } else {
            Some(
                self.class_factory_for_command(args, arg_tokens, cmd_tok.span.start())
                    .or_else(|| {
                        self.workspace_class_factory_at_release(args, arg_tokens, cmd_tok)
                    })?,
            )
        };
        let manufacturer = match user_factory.as_ref() {
            None => tcl_registry::CommandRegistry::manufacturer_method_for_descriptor(
                registry_definer?,
                &args[0],
            )
            .filter(|method| {
                method.visibility == tcl_registry::definer::MemberVisibility::Exported
            }),
            Some((factory, _)) => self
                .definition_grammar(&factory.root_metaclass)
                .and_then(|grammar| grammar.manufacturer(&args[0])),
        }?;
        if user_factory.as_ref().is_some_and(|(factory, _)| {
            !factory
                .exported_manufacturers
                .contains(manufacturer.keyword)
        }) {
            return None;
        }
        let layout = match user_factory.as_ref() {
            None => ManufacturerLayout::builtin(manufacturer),
            Some((factory, original_arguments)) => {
                manufacturer_layout(factory, manufacturer, args, Some(original_arguments))
            }
        }?;
        Some(ResolvedClassCreation {
            user_factory: user_factory.map(|(factory, _)| factory),
            registry_metaclass: registry_definer,
            layout,
        })
    }
}
