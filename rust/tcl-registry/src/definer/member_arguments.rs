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

//! Selecting the member row of one call and the argument indices of its bodies, roles, references and options.

use super::member_effect::{DeclaredMemberVisibility, InitTiming, MemberEffect, MemberReceiver};
use super::slots::{SlotOp, SlotSpec, TCLOO_SLOT_OPERATIONS};
use super::{
    DefinitionBodyGrammar, MemberArity, MemberKind, MemberOptionValue, MemberRefKind, MemberRow,
    MemberSpec, WrapperShift,
};
use crate::arg_role::ArgRole;
use crate::invocation_words::{InvocationArgument, InvocationArguments, InvocationWord};
use crate::value_transfer::inputs::OperandId;
use tcl_dialect::model::{SpecSurface, SurfaceQuery, surface_admits};

/// A member call whose argument layout cannot be read: a recognised optional
/// word unavailable in the dialect, or a computed word where the optional
/// word could stand.
struct UnreadableLayout;

impl MemberSpec {
    /// The fixed-layout argument indices (0-based after the keyword) carrying
    /// `role`, without considering optional call arguments.
    pub fn indices_for(&self, role: ArgRole) -> impl Iterator<Item = usize> + '_ {
        self.arg_roles
            .iter()
            .filter(move |(_, declared)| *declared == role)
            .map(|(index, _)| usize::from(*index))
    }

    /// The argument indices (0-based after the keyword) carrying `role` for
    /// this concrete call, including any recognised optional argument.
    pub fn indices_for_call<S: AsRef<str>>(
        &self,
        args: &[S],
        role: ArgRole,
    ) -> impl Iterator<Item = usize> + '_ {
        let option = self
            .optional_argument
            .and_then(|optional| optional.value_for(args));
        self.indices_for_selected_option(option, role)
    }

    pub(super) fn selected_indices_where(
        &self,
        option: Option<MemberOptionValue>,
        count: usize,
        admit: &dyn Fn(ArgRole) -> bool,
    ) -> Vec<usize> {
        let mut indices: Vec<_> = ArgRole::ALL
            .iter()
            .filter(|role| admit(**role))
            .flat_map(|&role| self.indices_for_selected_option(option, role))
            .filter(|&index| index < count)
            .collect();
        indices.sort_unstable();
        indices.dedup();
        indices
    }

    fn indices_for_selected_option(
        &self,
        option: Option<MemberOptionValue>,
        role: ArgRole,
    ) -> impl Iterator<Item = usize> + '_ {
        let option_position = self
            .optional_argument
            .map(|optional| usize::from(optional.position));
        let option_index = option
            .filter(|value| value.role == role)
            .zip(option_position)
            .map(|(_, position)| position);
        self.arg_roles
            .iter()
            .filter(move |(_, declared)| *declared == role)
            .map(move |(index, _)| self.selected_argument_index(usize::from(*index), option))
            .chain(option_index)
    }

    fn selected_argument_index(&self, index: usize, option: Option<MemberOptionValue>) -> usize {
        index
            + usize::from(
                option.is_some()
                    && self
                        .optional_argument
                        .is_some_and(|optional| index >= usize::from(optional.position)),
            )
    }

    fn source_body_presentation(
        &self,
        argument: usize,
        option: Option<MemberOptionValue>,
    ) -> SourceDefinitionMemberBodyPresentation {
        if let MemberEffect::Callable {
            body_slot: Some(slot),
            ..
        } = self.effect
            && self.selected_argument_index(usize::from(slot), option) == argument
        {
            SourceDefinitionMemberBodyPresentation::CallableBlock
        } else {
            SourceDefinitionMemberBodyPresentation::OrdinaryBlock
        }
    }

    /// Like [`Self::indices_for_call`], but selects optional words only when
    /// their registry-declared dialect availability intersects `dialect`.
    ///
    /// Production consumers must use this entry point: otherwise a Tcl 9-only
    /// flag could alter a Tcl 8.6 member layout merely because it looks like a
    /// known word. The compatibility method above deliberately uses all
    /// dialects for grammar-only callers and tests with no selected profile.
    pub fn indices_for_call_in<S: AsRef<str>>(
        &self,
        args: &[S],
        dialect: Option<SurfaceQuery<'_>>,
        role: ArgRole,
    ) -> impl Iterator<Item = usize> + '_ {
        let option = self
            .optional_argument
            .and_then(|optional| optional.value_for_in(args, dialect));
        self.indices_for_selected_option(option, role)
    }

    /// The option value present in this concrete member call, if any.
    #[must_use]
    pub fn option_for<S: AsRef<str>>(&self, args: &[S]) -> Option<MemberOptionValue> {
        self.optional_argument
            .and_then(|optional| optional.value_for(args))
    }

    /// The option value present and available in `dialect`, if any.
    #[must_use]
    pub fn option_for_in<S: AsRef<str>>(
        &self,
        args: &[S],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<MemberOptionValue> {
        self.optional_argument
            .and_then(|optional| optional.value_for_in(args, dialect))
    }

    /// A recognised optional word that is unavailable in `dialect`, if one is
    /// written at this member's optional position.
    #[must_use]
    pub fn unavailable_option_for<S: AsRef<str>>(
        &self,
        args: &[S],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<MemberOptionValue> {
        self.option_for(args)
            .filter(|_| self.option_for_in(args, dialect).is_none())
    }

    /// The declaration visibility selected by this concrete call's optional
    /// argument, when that option carries such semantics.
    #[must_use]
    pub fn declared_visibility_for<S: AsRef<str>>(
        &self,
        args: &[S],
    ) -> Option<DeclaredMemberVisibility> {
        self.option_for(args)
            .and_then(|option| option.declared_visibility)
    }

    /// The declaration visibility selected by an option available in
    /// `dialect`, when present.
    #[must_use]
    pub fn declared_visibility_for_in<S: AsRef<str>>(
        &self,
        args: &[S],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<DeclaredMemberVisibility> {
        self.option_for_in(args, dialect)
            .and_then(|option| option.declared_visibility)
    }

    /// The optional word of a source-aware call — [`Self::option_for_in`]
    /// over [`InvocationArguments`]. `Ok(None)` when the fixed layout applies.
    fn option_in_words(
        &self,
        args: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Result<Option<MemberOptionValue>, UnreadableLayout> {
        let Some(optional) = self.optional_argument else {
            return Ok(None);
        };
        match args.argv_at(usize::from(optional.position)) {
            InvocationArgument::Missing => Ok(None),
            InvocationArgument::Word(InvocationWord::Literal(word)) => {
                match optional.value_at(word, dialect) {
                    Some(value) => Ok(Some(value)),
                    None if optional.value_at(word, None).is_some() => Err(UnreadableLayout),
                    None => Ok(None),
                }
            }
            // A computed word that cannot begin with `-` is none of a
            // `-`-spelled vocabulary.
            InvocationArgument::Word(InvocationWord::DynamicNonOption)
                if optional
                    .values
                    .iter()
                    .all(|value| value.value.starts_with('-')) =>
            {
                Ok(None)
            }
            // Any other computed word could be the option; the layout is
            // known only when the call is too short to hold it.
            _ if args
                .exact_argv_len()
                .is_some_and(|len| len <= self.fixed_layout_len()) =>
            {
                Ok(None)
            }
            _ => Err(UnreadableLayout),
        }
    }

    /// The number of words the fixed layout spans: one past the highest
    /// `arg_roles` index.
    fn fixed_layout_len(&self) -> usize {
        self.arg_roles
            .iter()
            .map(|(index, _)| usize::from(*index) + 1)
            .max()
            .unwrap_or(0)
    }

    /// The call position (0-based after the keyword) of fixed-layout `slot`,
    /// shifted past the optional word when the call writes one — the mapping
    /// [`Self::indices_for_call_in`] applies.
    fn call_index(&self, slot: usize, option_present: bool) -> usize {
        slot + usize::from(
            option_present
                && self
                    .optional_argument
                    .is_some_and(|optional| slot >= usize::from(optional.position)),
        )
    }
}

/// Presentation of a script operand selected by an original definition member.
/// This describes authored source layout, without member installation, execution
/// traits, frame ownership or successful construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceDefinitionMemberBodyPresentation {
    /// Ordinary script presentation may keep a short single-command body inline.
    OrdinaryBlock,
    /// The selected callable's actual body slot has expanded declaration layout.
    CallableBlock,
}

/// How a source member walk reads its surface and which roles it admits.
#[derive(Clone, Copy)]
struct SourceMemberQuery<'q> {
    dialect: Option<SurfaceQuery<'q>>,
    admit: &'q dyn Fn(ArgRole) -> bool,
    complete: bool,
    source_layout: bool,
}

/// One selected source script operand within a definition-member call.
/// The ordinal owns no runtime member, installation or entered body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceDefinitionMemberScriptArgument {
    argument: usize,
    pub(super) definition_body: bool,
    presentation: SourceDefinitionMemberBodyPresentation,
}

impl SourceDefinitionMemberScriptArgument {
    /// Effective post-keyword operand ordinal after real wrapper/option words.
    #[must_use]
    pub const fn argument(self) -> usize {
        self.argument
    }
    /// Presentation of this actual selected operand, independent of execution.
    #[must_use]
    pub const fn presentation(self) -> SourceDefinitionMemberBodyPresentation {
        self.presentation
    }
    /// This wrapper block retains the selected parent definition vocabulary.
    /// Ordinary method/lifecycle/accessor scripts do not retain that vocabulary.
    #[must_use]
    pub const fn retains_definition_grammar(self) -> bool {
        self.definition_body
    }
}

/// The wrappers a member statement sits inside, as
/// [`DefinitionBodyGrammar::member_row`] crosses them.
#[derive(Clone, Copy)]
struct Wrapping {
    /// Their combined shift, the innermost wrapper's fields winning.
    shift: WrapperShift,
    /// The innermost wrapper's release set, which a wrapped member without one
    /// of its own inherits (`private method` is 9.0+ because `private` is).
    pub(super) surface: Option<&'static [SpecSurface]>,
}

impl Wrapping {
    /// Outside every wrapper.
    pub(super) const NONE: Self = Self {
        shift: WrapperShift::NONE,
        surface: None,
    };
}

/// The literal value of argv position `index`, when it is one.
fn literal_argument(words: InvocationArguments<'_>, index: usize) -> Option<&str> {
    match words.argv_at(index) {
        InvocationArgument::Word(InvocationWord::Literal(word)) => Some(word),
        _ => None,
    }
}

/// The operation a slot call applies — [`SlotSpec::split_call`] over
/// source-aware words: an explicit leading operation word, or the slot's
/// default for a bare list. `None` for an unrecognised `-word` (real Tcl
/// aborts the definition) and for a computed first word that could be one.
fn slot_op_at(slot: SlotSpec, args: InvocationArguments<'_>) -> Option<SlotOp> {
    match args.argv_at(0) {
        InvocationArgument::Word(InvocationWord::Literal(word)) if word.starts_with('-') => {
            SlotOp::parse(word)
        }
        InvocationArgument::Word(InvocationWord::Literal(_) | InvocationWord::DynamicNonOption)
        | InvocationArgument::Missing => Some(slot.default_op),
        _ => None,
    }
}

impl DefinitionBodyGrammar {
    /// The member grammar for `keyword`, if it is a recognised member.
    #[must_use]
    pub fn member(&self, keyword: &str) -> Option<&'static MemberSpec> {
        // `members` is `&'static`, so the borrow can be handed back as static.
        let idx = self.members.iter().position(|m| m.keyword == keyword)?;
        Some(&self.members[idx])
    }

    /// Member keyword admitted by this selected definition vocabulary and the
    /// actual source availability query. This grants no ordinary command lookup.
    #[must_use]
    pub fn source_member_in(
        &self,
        keyword: &[u8],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static MemberSpec> {
        let member = self.member(std::str::from_utf8(keyword).ok()?)?;
        member
            .surface
            .is_none_or(|available| surface_admits(available, dialect.as_ref()))
            .then_some(member)
    }

    /// What one member statement declares: its [`MemberEffect`] read against
    /// the statement's words (`registry-consumer-contracts.md` § *The
    /// member-effect descriptor*). One statement at a time — the analyser
    /// segments a definition body and folds the rows.
    ///
    /// `words` are the statement's words and `keyword_index` the member
    /// keyword's position among them: `0` inside a class body, `1` for the
    /// single-command `oo::define CLASS method …` form. The row's
    /// `keyword_index` and `body` index the same words.
    ///
    /// A wrapper's prefix form answers the wrapped member's row with the
    /// wrapper's [`WrapperShift`] applied; its bare block form (`self { … }`,
    /// `private { … }`) answers an [`MemberEffect::InitScript`] run at
    /// definition, whose receiver and visibility are the ones the block's own
    /// members take.
    ///
    /// `None` when the keyword is not a literal member of this grammar, when a
    /// wrapper wraps nothing it recognises, and when the call's layout cannot
    /// be read — a recognised optional word unavailable in `dialect`, or a
    /// computed word where the optional word could stand. A computed *name*
    /// does not abstain the row: its `name` is `None`.
    #[must_use]
    pub fn member_row(
        &self,
        keyword_index: usize,
        words: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<MemberRow> {
        self.member_row_within(keyword_index, words, dialect, Wrapping::NONE)
    }

    /// [`Self::member_row`] inside the wrappers already crossed.
    fn member_row_within(
        &self,
        keyword_index: usize,
        words: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
        outer: Wrapping,
    ) -> Option<MemberRow> {
        let member = self.member(literal_argument(words, keyword_index)?)?;
        let first = keyword_index + 1;
        let args = words.slice_from(first);
        if member.kind == MemberKind::Wrapper {
            let shift = member
                .wrapper_shift
                .unwrap_or(WrapperShift::NONE)
                .within(outer.shift);
            let surface = member.surface.or(outer.surface);
            if literal_argument(args, 0).is_some_and(|inner| self.is_member(inner)) {
                return self.member_row_within(first, words, dialect, Wrapping { shift, surface });
            }
            if !member.wrapper_block_body || args.exact_argv_len() != Some(1) {
                return None;
            }
            return Some(MemberRow {
                keyword_index,
                effect: MemberEffect::InitScript {
                    body_slot: 0,
                    timing: InitTiming::AtDefinition,
                },
                receiver: shift.receiver.unwrap_or(MemberReceiver::Instance),
                name: None,
                arity: None,
                visibility: shift.visibility.unwrap_or(DeclaredMemberVisibility::Public),
                body: Some(OperandId(first)),
                slot_op: None,
                surface,
            });
        }
        let option = member.option_in_words(args, dialect).ok()?;
        let at = |slot: usize| member.call_index(slot, option.is_some());
        let effect = member.effect;
        let name_slot = match effect {
            MemberEffect::Callable { name_slot, .. } => name_slot.map(usize::from),
            MemberEffect::Forward { name_slot, .. } => Some(usize::from(name_slot)),
            MemberEffect::StateDeclaration { .. } if !member.all_args_var => {
                member.indices_for(ArgRole::VarWrite).next()
            }
            _ => None,
        };
        let name = name_slot
            .and_then(|slot| literal_argument(args, at(slot)))
            .map(str::to_owned);
        let arity = match effect {
            MemberEffect::Callable {
                params_slot: Some(slot),
                ..
            } => literal_argument(args, at(usize::from(slot))).and_then(MemberArity::parse),
            _ => None,
        };
        let body_slot = match effect {
            MemberEffect::Callable { body_slot, .. } => body_slot,
            MemberEffect::InitScript { body_slot, .. } => Some(body_slot),
            _ => None,
        };
        let body = body_slot
            .map(|slot| at(usize::from(slot)))
            .filter(|&index| matches!(args.argv_at(index), InvocationArgument::Word(_)))
            .map(|index| OperandId(first + index));
        let visibility = option
            .and_then(|value| value.declared_visibility)
            .or(outer.shift.visibility)
            .unwrap_or_else(|| match &name {
                Some(name) if !self.member_default_exported(name) => {
                    DeclaredMemberVisibility::Unexported
                }
                _ => DeclaredMemberVisibility::Public,
            });
        Some(MemberRow {
            keyword_index,
            effect,
            receiver: outer
                .shift
                .receiver
                .unwrap_or_else(|| effect.natural_receiver()),
            name,
            arity,
            visibility,
            body,
            slot_op: member.slot.and_then(|slot| slot_op_at(slot, args)),
            surface: member.surface.or(outer.surface),
        })
    }

    /// Body argument indices for a concrete definition-member invocation —
    /// the words a consumer walking **executable code** must descend into.
    ///
    /// Handles flat members, prefix wrappers, wrapper block forms, and
    /// flag-keyed getter/setter bodies from this grammar's structural data.
    ///
    /// Deliberately narrower than [`Self::member_block_indices_in`]: a member
    /// word that is script-*shaped* data ([`ArgRole::OpaqueScript`]) is a
    /// block a reader folds, not code an analysis enters.
    #[must_use]
    pub fn member_body_indices_in(
        &self,
        keyword: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<usize> {
        self.member_indices_where(keyword, args, dialect, &ArgRole::carries_script)
    }

    /// Braced-block argument indices for a concrete definition-member
    /// invocation — the words a **reader** can collapse.
    ///
    /// [`Self::member_body_indices_in`] plus every member word that is a
    /// braced block without being executable, so folding covers a retained
    /// script (`SslicTcl`'s `predicate`) without any consumer deciding for
    /// itself what counts as one.
    #[must_use]
    pub fn member_block_indices_in(
        &self,
        keyword: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<usize> {
        self.member_indices_where(keyword, args, dialect, &ArgRole::folds_as_block)
    }

    /// Arguments carrying one role in the selected member grammar. Nested
    /// wrappers, optional words and unbounded variable declarations share the
    /// same layout as executable and presentation body projections.
    #[must_use]
    pub fn member_arg_indices_for_role_in(
        &self,
        keyword: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
        role: ArgRole,
    ) -> Vec<usize> {
        self.member_indices_where(keyword, args, dialect, &|candidate| candidate == role)
    }

    /// Source operands carrying one role in this independently selected
    /// definition vocabulary. Dynamic selectors and expanded argv remain
    /// unavailable; operand values keep their original byte/dynamic state.
    #[must_use]
    pub fn source_member_arg_indices_for_role_in(
        &self,
        keyword: &[u8],
        args: crate::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
        role: ArgRole,
    ) -> Option<Vec<usize>> {
        Some(
            self.source_member_indices_where(
                std::str::from_utf8(keyword).ok()?,
                args,
                dialect,
                &|candidate| candidate == role,
                false,
                false,
            )?
            .into_iter()
            .map(SourceDefinitionMemberScriptArgument::argument)
            .collect(),
        )
    }

    /// Complete source-role layout for a purpose-specific readonly consumer.
    /// This preserves exact argv, selector and availability checks without
    /// converting an opaque role into an executable script argument.
    #[must_use]
    pub fn checked_source_member_arg_indices_for_role_in(
        &self,
        keyword: &[u8],
        args: crate::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
        role: ArgRole,
    ) -> Option<Vec<usize>> {
        Some(
            self.source_member_indices_where(
                std::str::from_utf8(keyword).ok()?,
                args,
                dialect,
                &|candidate| candidate == role,
                true,
                false,
            )?
            .into_iter()
            .map(SourceDefinitionMemberScriptArgument::argument)
            .collect(),
        )
    }

    /// Reference-list arguments and their entity kind, with wrapper operands
    /// and the leading slot operation mapped by this grammar.
    #[must_use]
    pub fn member_ref_indices_in(
        &self,
        keyword: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<(MemberRefKind, Vec<usize>)> {
        self.source_member_ref_indices_in(
            keyword.as_bytes(),
            crate::InvocationArguments::literals(args),
            dialect,
        )
    }

    /// Reference operands from the selected source vocabulary and exact argv.
    /// Only selector and leading slot words need static bytes. Other values
    /// retain their own dynamic/byte states and supply no installed member.
    #[must_use]
    pub fn source_member_ref_indices_in(
        &self,
        keyword: &[u8],
        args: crate::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<(MemberRefKind, Vec<usize>)> {
        self.checked_source_member_ref_indices_in(keyword, args, dialect)?
    }

    /// Complete selected source reference-role inventory. The outer `None`
    /// retains unavailable argv/selector grammar; `Some(None)` is a selected
    /// member with no reference role. Neither result selects a live worker.
    #[must_use]
    pub fn checked_source_member_ref_indices_in(
        &self,
        keyword: &[u8],
        args: crate::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<Option<(MemberRefKind, Vec<usize>)>> {
        // naming.core.original-member-reference-hazard
        // docs/design/analysis/name-resolution-proofs/core-original-member-reference-hazard.md
        let count = args.exact_argv_len()?;
        let member = self.source_member_in(keyword, dialect)?;
        let value = |index| {
            args.get(index).and_then(|word| {
                word.literal()
                    .map(str::as_bytes)
                    .or_else(|| word.native_bytes())
            })
        };
        if let Some(optional) = member.optional_argument
            && usize::from(optional.position) < count
        {
            let word = value(usize::from(optional.position))?;
            if optional.value_for_word_in(word, None).is_some()
                && optional.value_for_word_in(word, dialect).is_none()
            {
                return None;
            }
        }
        match member.kind {
            MemberKind::Flat => {
                let start = if member.slot.is_some() && count != 0 {
                    let first = value(0)?;
                    usize::from(
                        TCLOO_SLOT_OPERATIONS
                            .iter()
                            .any(|entry| entry.name.as_bytes() == first),
                    )
                } else {
                    0
                };
                Some(
                    member
                        .all_args_ref
                        .map(|kind| (kind, (start..count).collect())),
                )
            }
            MemberKind::Wrapper => {
                let selected = self.checked_source_member_ref_indices_in(
                    value(0)?,
                    args.slice_from(1),
                    dialect,
                )?;
                Some(selected.map(|(kind, indices)| {
                    (kind, indices.into_iter().map(|index| index + 1).collect())
                }))
            }
            MemberKind::FlagKeyed => Some(None),
        }
    }

    /// The shared walk behind [`Self::member_body_indices_in`] and
    /// [`Self::member_block_indices_in`]: `admit` selects which roles count.
    fn member_indices_where(
        &self,
        keyword: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
        admit: &dyn Fn(ArgRole) -> bool,
    ) -> Vec<usize> {
        self.source_member_indices_where(
            keyword,
            crate::InvocationArguments::literals(args),
            dialect,
            admit,
            false,
            false,
        )
        .unwrap_or_default()
        .into_iter()
        .map(SourceDefinitionMemberScriptArgument::argument)
        .collect()
    }

    /// Source body ordinals under this independently selected definition grammar.
    /// Real byte/literal selectors share the static layout kernel. Dynamic
    /// options, wrapper selectors or argv shape remain unavailable; unrelated
    /// names, formals and body values do not have to become logical strings.
    #[must_use]
    pub fn source_member_body_indices_in(
        &self,
        keyword: &[u8],
        args: crate::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<Vec<usize>> {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        Some(
            self.source_member_script_arguments_in(keyword, args, dialect)?
                .into_iter()
                .map(SourceDefinitionMemberScriptArgument::argument)
                .collect(),
        )
    }

    /// Original body operands and wrapper vocabulary from the shared layout.
    /// Unknown selector/value shape cannot supply a guessed nested grammar.
    #[must_use]
    pub fn source_member_script_arguments_in(
        &self,
        keyword: &[u8],
        args: crate::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<Vec<SourceDefinitionMemberScriptArgument>> {
        self.source_member_indices_where(
            std::str::from_utf8(keyword).ok()?,
            args,
            dialect,
            &ArgRole::carries_script,
            false,
            false,
        )
    }

    /// Complete selected script layout. Unsupported arity, availability or
    /// selectors remain unavailable rather than an empty body inventory.
    #[must_use]
    pub fn checked_source_member_script_arguments_in(
        &self,
        keyword: &[u8],
        args: crate::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<Vec<SourceDefinitionMemberScriptArgument>> {
        self.source_member_indices_where(
            std::str::from_utf8(keyword).ok()?,
            args,
            dialect,
            &ArgRole::carries_script,
            true,
            false,
        )
    }

    /// Complete source script layout from this grammar's authored member geometry,
    /// independent of whether the selected interpreter release installs that
    /// member. This reports operand roles only; it grants no dispatch or
    /// execution authority.
    #[must_use]
    pub fn source_member_script_layout_arguments(
        &self,
        keyword: &[u8],
        args: crate::InvocationArguments<'_>,
    ) -> Option<Vec<SourceDefinitionMemberScriptArgument>> {
        self.source_member_indices_where(
            std::str::from_utf8(keyword).ok()?,
            args,
            None,
            &ArgRole::carries_script,
            true,
            true,
        )
    }

    fn source_member_indices_where(
        &self,
        keyword: &str,
        args: crate::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
        admit: &dyn Fn(ArgRole) -> bool,
        complete: bool,
        source_layout: bool,
    ) -> Option<Vec<SourceDefinitionMemberScriptArgument>> {
        let count = args.exact_argv_len()?;
        let member = if source_layout {
            self.member(keyword)
        } else {
            self.source_member_in(keyword.as_bytes(), dialect)
        };
        let Some(member) = member else {
            return (!complete).then(Vec::new);
        };
        let word_at = |index| {
            args.get(index).and_then(|word| {
                word.literal()
                    .map(str::as_bytes)
                    .or_else(|| word.native_bytes())
            })
        };
        let option = if let Some(optional) = member.optional_argument {
            if usize::from(optional.position) < count {
                let word = word_at(usize::from(optional.position))?;
                let known = optional.value_for_word_in(word, None);
                let admitted = optional.value_for_word_in(word, dialect);
                if !source_layout && known.is_some() && admitted.is_none() {
                    return (!complete).then(Vec::new);
                }
                if source_layout { known } else { admitted }
            } else {
                None
            }
        } else {
            None
        };
        if complete
            && member.kind == MemberKind::Flat
            && member.arg_roles.iter().any(|(_, role)| admit(*role))
        {
            let required = ArgRole::ALL
                .iter()
                .flat_map(|&role| member.indices_for_selected_option(option, role))
                .max()?
                .checked_add(1)?;
            if required != count {
                return None;
            }
        }
        if complete && member.kind == MemberKind::FlagKeyed && count == 0 {
            return None;
        }
        let selected = || member.selected_indices_where(option, count, admit);
        let script_argument = |argument| SourceDefinitionMemberScriptArgument {
            argument,
            definition_body: false,
            presentation: member.source_body_presentation(argument, option),
        };
        let indices = match member.kind {
            MemberKind::Flat if member.all_args_var && admit(ArgRole::VarWrite) => {
                let start = if member.slot.is_some() && count != 0 {
                    let first = word_at(0)?;
                    usize::from(
                        TCLOO_SLOT_OPERATIONS
                            .iter()
                            .any(|entry| entry.name.as_bytes() == first),
                    )
                } else {
                    0
                };
                let mut indices = selected();
                indices.extend(start..count);
                indices.sort_unstable();
                indices.dedup();
                indices.into_iter().map(script_argument).collect()
            }
            MemberKind::Flat => selected().into_iter().map(script_argument).collect(),
            MemberKind::Wrapper => {
                let query = SourceMemberQuery {
                    dialect,
                    admit,
                    complete,
                    source_layout,
                };
                return self.source_wrapper_indices_where(member, option, args, query);
            }
            MemberKind::FlagKeyed => self
                .source_property_declarations_where(count, &word_at, dialect, source_layout)?
                .into_iter()
                .flat_map(|declaration| declaration.selected_indices_where(admit))
                .map(script_argument)
                .collect(),
        };
        Some(indices)
    }

    /// A wrapper member's script arguments: its delegated inner member, shifted
    /// past the wrapper keyword, or its own block body.
    fn source_wrapper_indices_where(
        &self,
        member: &MemberSpec,
        option: Option<MemberOptionValue>,
        args: crate::InvocationArguments<'_>,
        query: SourceMemberQuery<'_>,
    ) -> Option<Vec<SourceDefinitionMemberScriptArgument>> {
        let SourceMemberQuery {
            dialect,
            admit,
            complete,
            source_layout,
        } = query;
        let count = args.exact_argv_len()?;
        if count == 0 {
            return (!complete).then(Vec::new);
        }
        let inner = args.get(0).and_then(|word| {
            word.literal()
                .map(str::as_bytes)
                .or_else(|| word.native_bytes())
        })?;
        if let Ok(inner) = std::str::from_utf8(inner)
            && self.member(inner).is_some()
        {
            if !source_layout && self.source_member_in(inner.as_bytes(), dialect).is_none() {
                return (!complete).then(Vec::new);
            }
            let mut indices = self
                .source_member_indices_where(
                    inner,
                    args.slice_from(1),
                    dialect,
                    admit,
                    complete,
                    source_layout,
                )?
                .into_iter()
                .map(|mut argument| {
                    argument.argument += 1;
                    argument
                })
                .collect::<Vec<_>>();
            if admit(ArgRole::Keyword) {
                indices.insert(
                    0,
                    SourceDefinitionMemberScriptArgument {
                        argument: 0,
                        definition_body: false,
                        presentation: member.source_body_presentation(0, option),
                    },
                );
            }
            Some(indices)
        } else if member.wrapper_block_body {
            if complete && count != 1 {
                return None;
            }
            Some(
                member
                    .selected_indices_where(option, count, admit)
                    .into_iter()
                    .map(|argument| SourceDefinitionMemberScriptArgument {
                        argument,
                        definition_body: true,
                        presentation: SourceDefinitionMemberBodyPresentation::OrdinaryBlock,
                    })
                    .collect(),
            )
        } else {
            (!complete).then(Vec::new)
        }
    }
}
