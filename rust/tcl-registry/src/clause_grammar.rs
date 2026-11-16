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

//! Clause grammars as data — the clause-grammar descriptor
//! (`docs/design/compiler/registry-consumer-contracts.md` § *The
//! clause-grammar descriptor*).
//!
//! A clause-carrying command — `if` / `elseif` / `else`, `try` / `on` /
//! `trap` / `finally`, `for`, `while`, `foreach`, `lmap`, `catch`, `dict for`,
//! `dict map`, `dict update`, `array for` — declares its word grammar as a
//! [`ClauseGrammarSpec`] on `CommandSpec::clause_grammar` (or a subcommand's).
//! The registry walks a call against it once, [`ClauseGrammarSpec::walk`], and
//! every consumer reads the one answer, a [`ClausePlan`]: the clauses the call
//! supplied, the flat argument roles, and the first structural defect.
//!
//! The descriptor gives locations and grammar and nothing executable:
//! first-match dispatch, list iteration and completion belong to the consumer
//! interface's structural plan, declared beside it and never inferred from the
//! slots.
//!
//! **Where keywords match** (normative). The walk compares a word against a
//! keyword only when it asks "does a clause start here?", and at a `?noise?`
//! slot. Every other slot is filled positionally and consumes whatever word is
//! there, *including one spelled like a keyword*: `if else {a}` is a
//! well-formed `if` whose condition is the bareword `else`, and
//! `if 1 a elseif else b` a well-formed chain whose second condition is the
//! bareword `else` — what C Tcl's `IfConditionCallback` does.
//!
//! **The flat projection.** [`ClausePlan::roles`] is what
//! `CommandRegistry::arg_indices_for_role` folds: every introducing keyword
//! and present noise word as [`ArgRole::Keyword`], and each filled slot's role
//! — except the facts only the clause can carry. A [`ArgRole::LoopVarList`]
//! slot's names are bound per clause (a `try` handler's, a `catch`'s), which
//! the flat `LoopVarList` role — read as an iteration header's binding —
//! cannot say; a [`ArgRole::Value`] slot is the unlisted default; a `Pattern`
//! slot carrying a [`HandlerMatch`] is matched by the handler's vocabulary, not
//! the command's pattern language; a fall-through body is not a script; and a
//! [`ClauseRowShape::Group`] row's words are its cited
//! [`RepeatedArgLayout`]'s, which the registry folds itself. Each is read from
//! [`ClausePlan::clauses`]; a command whose flat table should also carry the
//! role (`dict for`'s `LoopVarList`, `catch`'s `VarWrite`) states it in its own
//! `arg_roles`, as before.
//!
//! **The escape hatch.** `CommandSpec::clause_shape_check` stays only for a
//! chain no grammar can spell; the walk derives [`ClauseShapeError`] from the
//! grammar, so `if`'s resolver and shape check retired into one declaration.

use std::sync::OnceLock;

use tcl_dialect::model::{SpecSurface, SurfaceQuery, surface_admits};

use crate::arg_role::ArgRole;
use crate::clause_shape::{ClauseShapeError, ClauseShapeIssue, ClauseShapeRepair};
use crate::registry::CommandRegistry;
use crate::repeated::RepeatedArgLayout;
use crate::value_transfer::HandlerMatch;

/// The word grammar of a clause chain: `if` / `elseif` / `else`, `try` / `on`
/// / `trap` / `finally`, `for`, `while`, `foreach` / `lmap` / `dict for` /
/// `dict map` / `array for`, `dict update`, `catch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClauseGrammarSpec {
    /// The mandatory leading clause, filled positionally. Its slots are never
    /// keyword-matched, which is what makes `if else {a}` a well-formed `if`
    /// whose condition is the bareword `else`. A row (keyword `None`,
    /// [`ClauseRowShape::Once`]) rather than bare slots, so it carries its own
    /// timing: `try`'s body is protected, `for`'s start is a loop fixture.
    pub head: ClauseRow,
    /// Further clauses in declaration order. The walk compares a word against
    /// a keyword only when asking "does a clause start here?"; a keywordless
    /// row is entered positionally, in declaration order.
    pub rows: &'static [ClauseRow],
    /// At most one trailing clause, last; anything after it is extra.
    pub tail: Option<ClauseRow>,
    /// The body word that runs a following clause's body instead of its own
    /// (`try`'s `-`). `None`: no clause falls through. Only a clause whose
    /// timing is [`ClauseTiming::Selected`] falls through — a protected body
    /// or a `finally` spelled `-` is a script named `-`.
    pub fallthrough_body: Option<&'static str>,
    /// The clause that runs when no earlier clause is selected, and whether it
    /// is legal anywhere but last.
    pub default_clause: Option<DefaultClause>,
    /// How many clauses one call selects.
    pub selection: ClauseSelection,
    /// Releases the whole grammar is available at; `None` is every one.
    pub surface: Option<&'static [SpecSurface]>,
}

/// One clause row of a [`ClauseGrammarSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClauseRow {
    /// The literal introducing word; `None` for a keywordless row or a tail
    /// whose keyword may be omitted (`if`'s implicit final body).
    pub keyword: Option<&'static str>,
    /// Whether that keyword is required when the clause is present.
    pub keyword_required: bool,
    /// What repeats, and how.
    pub shape: ClauseRowShape,
    /// When the clause's body runs relative to the call. Conditional depth
    /// only — never a CFG edge, which keeps the descriptor on the data side of
    /// the `completion` exclusion.
    pub timing: ClauseTiming,
    /// Releases this row is available at; `None` inherits the grammar's.
    pub surface: Option<&'static [SpecSurface]>,
}

/// What a [`ClauseRow`] repeats, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseRowShape {
    /// Zero or more clauses, each introduced by the row's keyword, whose slots
    /// are filled positionally after it (`elseif`, `on`, `trap`).
    Repeated {
        /// The clause's slots, in word order.
        slots: &'static [ClauseSlot],
    },
    /// Exactly one clause (`finally`, `else`, `for`'s test).
    Once {
        /// The clause's slots, in word order.
        slots: &'static [ClauseSlot],
    },
    /// A keywordless repeating group whose stride and trailing exclusion are
    /// the `repeated_args` layout at this index — the binder groups of
    /// `foreach` / `lmap` and `dict update`'s key/variable pairs. The stride
    /// stays [`RepeatedArgLayout`]'s fact; this row cites it and never
    /// restates it.
    Group {
        /// Index into the owning spec's `repeated_args`.
        layout: u8,
    },
}

/// One word position inside a clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClauseSlot {
    /// The registry's own [`ArgRole`], so a clause slot and an `arg` row are
    /// one vocabulary. The six a clause uses: `Expr` (a condition), `Body` (a
    /// script; which frame it runs in is `body_kind` / `body_scope`, not this
    /// slot), `LoopVarList` (`try`'s `{msg opts}`, `catch`'s result and options
    /// words, `dict for`'s `{k v}`), `Pattern` (a word that selects its
    /// clause), `Keyword` (a `?noise?` word), and `Value` for anything else.
    pub role: ArgRole,
    /// The literal word an [`ArgRole::Keyword`] slot accepts. Such a slot is
    /// optional by construction and is *not* highlighted as a keyword — the
    /// distinction [`crate::traits::clause_noise_keywords`] draws against
    /// [`crate::traits::clause_keywords_without_command_spec`].
    pub noise: Option<&'static str>,
    /// The match vocabulary of an [`ArgRole::Pattern`] slot that selects its
    /// clause. At most one slot per row carries it, which is what lets the
    /// `.tclspec` row state it as a row flag.
    pub handler: Option<HandlerMatch>,
    /// True when the names an [`ArgRole::LoopVarList`] slot binds are bound
    /// only if a runtime *data* condition holds — [`RepeatedArgLayout`]'s
    /// `conditional_binding` fact, on a clause slot. Such a slot never carries
    /// [`ArgRole::VarWrite`], for the reason `RepeatedArgLayout` gives:
    /// `VarWrite` is read everywhere as an unconditional SSA def.
    pub conditional_binding: bool,
    /// Whether the slot may be absent.
    pub optional: bool,
}

/// When a clause's body runs relative to the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseTiming {
    /// Runs when its own clause is selected: `if`, `elseif`, `else`, `on`,
    /// `trap`, and a `switch` arm.
    Selected,
    /// Runs whatever the outcome: `finally`, and a body that runs once
    /// whenever the call does (`dict update`'s).
    Always,
    /// Runs once per iteration: the body of `while`, `for`, `foreach`,
    /// `lmap`, `dict for`, `dict map`, `array for`.
    PerIteration,
    /// A loop fixture: `for`'s init before the first test, `for`'s next
    /// between iterations.
    LoopFixture(LoopPhase),
    /// Runs unconditionally, and its completion is observed rather than
    /// propagated: `catch`'s and `try`'s protected body.
    Protected,
}

/// Which loop fixture a [`ClauseTiming::LoopFixture`] clause is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopPhase {
    /// Before the first test.
    Init,
    /// Between iterations, after the body.
    Next,
}

/// How many clauses one call selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseSelection {
    /// The first clause whose condition or pattern matches, and no other.
    FirstMatch,
    /// Every clause present runs, in order.
    All,
}

/// The clause that runs when no earlier clause is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultClause {
    /// Index into `rows`, or `None` when `tail` is the default.
    pub row: Option<u8>,
    /// Whether the default is legal only as the last clause.
    pub final_only: bool,
}

/// The one clause answer a consumer asks for: what the walk found in a call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClausePlan {
    /// Every clause the call supplied, in source order — including a partial
    /// one the defect stopped in.
    pub clauses: Vec<ResolvedClause>,
    /// The flat roles the walk assigned, in the `(index, ArgRole)` shape
    /// `CommandRegistry::arg_indices_for_role` already folds (the module's
    /// *flat projection*), in word order.
    pub roles: Vec<(usize, ArgRole)>,
    /// The first structural defect, or `None` for a shape the grammar
    /// accepts — the existing [`ClauseShapeError`], unchanged.
    pub defect: Option<ClauseShapeError>,
}

/// One clause a call supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedClause {
    /// Index of the introducing keyword word, or `None` when omitted.
    pub keyword_index: Option<usize>,
    /// Which row matched.
    pub row: ClauseRowId,
    /// One entry per filled slot, in declaration order, carrying the operand
    /// index and the slot it filled. A present `?noise?` word is an entry; an
    /// absent optional slot is not.
    pub operands: Vec<(usize, ClauseSlot)>,
    /// When the clause's body runs.
    pub timing: ClauseTiming,
    /// Set when this clause's body word is the fall-through marker: the index
    /// (into [`ClausePlan::clauses`]) of the clause whose body it runs.
    pub falls_through_to: Option<usize>,
    /// True for the clause `default_clause` names.
    pub is_default: bool,
}

/// Why a source-aware walk ([`ClauseGrammarSpec::walk_words`]) has no sound
/// plan: a computed word stands where the walk compares one — a keyword, a
/// noise word or the fall-through marker — and Tcl decides those by value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClauseAbstention {
    /// The first such word.
    pub word: usize,
    /// The call read with every computed word matching nothing: what it is
    /// if none of them turns out to spell a keyword, a noise word or the
    /// marker. Never the call's plan — a consumer that defers the call to the
    /// runtime command reads it only to take the same steps before deferring
    /// that it takes for a call whose plan it has.
    pub inert: ClausePlan,
}

/// Which row of a [`ClauseGrammarSpec`] a [`ResolvedClause`] matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClauseRowId {
    /// The positional head.
    Head,
    /// `rows[n]`.
    Row(u8),
    /// The tail.
    Tail,
}

impl ClauseTiming {
    /// Every timing, in the order the `.tclspec` vocabulary lists them.
    pub const ALL: &'static [Self] = &[
        Self::Selected,
        Self::Always,
        Self::PerIteration,
        Self::LoopFixture(LoopPhase::Init),
        Self::LoopFixture(LoopPhase::Next),
        Self::Protected,
    ];

    /// The `.tclspec` spelling of a row's `-timing` flag.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::Always => "always",
            Self::PerIteration => "per-iteration",
            Self::LoopFixture(LoopPhase::Init) => "init",
            Self::LoopFixture(LoopPhase::Next) => "next",
            Self::Protected => "protected",
        }
    }

    /// The timing a `-timing` word spells, or `None` for any other word.
    #[must_use]
    pub fn from_spelling(word: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|timing| timing.spelling() == word)
    }
}

impl ClauseSelection {
    /// The `.tclspec` spelling of a grammar's `selection` row.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::FirstMatch => "first-match",
            Self::All => "all",
        }
    }

    /// The selection a `selection` word spells, or `None` for any other word.
    #[must_use]
    pub fn from_spelling(word: &str) -> Option<Self> {
        [Self::FirstMatch, Self::All]
            .into_iter()
            .find(|selection| selection.spelling() == word)
    }
}

/// The `.tclspec` spelling of a row's `-pattern` flag: the handler vocabulary
/// its `Pattern` slot selects the clause by.
#[must_use]
pub const fn handler_spelling(handler: HandlerMatch) -> &'static str {
    match handler {
        HandlerMatch::CompletionCode => "completion-code",
        HandlerMatch::ErrorCodePrefix => "error-code-prefix",
    }
}

/// The handler vocabulary a `-pattern` word spells, or `None` for any other
/// word.
#[must_use]
pub fn handler_from_spelling(word: &str) -> Option<HandlerMatch> {
    [HandlerMatch::CompletionCode, HandlerMatch::ErrorCodePrefix]
        .into_iter()
        .find(|handler| handler_spelling(*handler) == word)
}

impl ClausePlan {
    /// Whether clause `index`'s body word is the grammar's fall-through
    /// marker — whether or not a later clause supplies the body it runs
    /// ([`ResolvedClause::falls_through_to`] is `None` for a marker with no
    /// target). The flat projection leaves exactly such a body out of
    /// [`Self::roles`], so a body operand missing there is the marker.
    #[must_use]
    pub fn falls_through(&self, index: usize) -> bool {
        self.clauses.get(index).is_some_and(|clause| {
            clause.falls_through_to.is_some()
                || clause
                    .operand(ArgRole::Body)
                    .is_some_and(|body| !self.roles.contains(&(body, ArgRole::Body)))
        })
    }

    /// The structural defect and a repair addressing selected original word
    /// ordinals. No runtime execution or edit permission follows from this
    /// source grammar; the consumer still validates complete physical spans.
    #[must_use]
    pub fn shape_issue(&self) -> Option<ClauseShapeIssue> {
        let error = self.defect?;
        let last = self.clauses.last();
        let repair = match error {
            ClauseShapeError::MissingExpr { .. } | ClauseShapeError::MissingBody { .. } => last
                .and_then(|clause| clause.keyword_index)
                .map(|keyword| ClauseShapeRepair::RemoveTrailingClause { keyword }),
            ClauseShapeError::ExtraWords { first_extra } => last
                .and_then(|clause| clause.operand(ArgRole::Body))
                .filter(|&body| body + 1 == first_extra)
                .map(|body| ClauseShapeRepair::MergeTrailingWords { body }),
        };
        Some(ClauseShapeIssue::with_repair(error, repair))
    }

    /// This plan with every word index moved `offset` words right — a
    /// subcommand grammar walks the words after the subcommand word, and its
    /// plan is reported in the invocation's own post-head coordinates, the
    /// ones `CommandRegistry::arg_indices_for_role` answers in.
    #[must_use]
    pub fn offset_by(mut self, offset: usize) -> Self {
        if offset == 0 {
            return self;
        }
        for clause in &mut self.clauses {
            clause.keyword_index = clause.keyword_index.map(|index| index + offset);
            for (index, _) in &mut clause.operands {
                *index += offset;
            }
        }
        for (index, _) in &mut self.roles {
            *index += offset;
        }
        self.defect = self.defect.map(|defect| match defect {
            ClauseShapeError::MissingExpr { after } => ClauseShapeError::MissingExpr {
                after: Some(after.map_or(offset - 1, |after| after + offset)),
            },
            ClauseShapeError::MissingBody { after } => ClauseShapeError::MissingBody {
                after: after + offset,
            },
            ClauseShapeError::ExtraWords { first_extra } => ClauseShapeError::ExtraWords {
                first_extra: first_extra + offset,
            },
        });
        self
    }
}

impl ResolvedClause {
    /// The word index of the first operand filling a slot of `role` — the
    /// clause's condition for [`ArgRole::Expr`], its script word for
    /// [`ArgRole::Body`] — or `None` when no such slot was filled. A clause
    /// with two slots of one role (`catch`'s result and options words) names
    /// the rest through [`Self::operands`].
    #[must_use]
    pub fn operand(&self, role: ArgRole) -> Option<usize> {
        self.operands(role).next()
    }

    /// The word index of every operand filling a slot of `role`, in word
    /// order: both of `catch`'s `{LoopVarList optional}` result words, where
    /// [`Self::operand`] names only the first.
    pub fn operands(&self, role: ArgRole) -> impl Iterator<Item = usize> + '_ {
        self.operands
            .iter()
            .filter(move |(_, slot)| slot.role == role)
            .map(|&(index, _)| index)
    }

    /// The pattern operand that selects this clause, with the vocabulary it
    /// selects by (`try`'s `on` code, `trap` prefix), or `None` for a clause
    /// no handler pattern selects.
    #[must_use]
    pub fn handler(&self) -> Option<(usize, HandlerMatch)> {
        self.operands
            .iter()
            .find_map(|(index, slot)| slot.handler.map(|handler| (*index, handler)))
    }
}

impl ClauseSlot {
    /// A required slot of `role`: no noise word, no handler, bound
    /// unconditionally.
    #[must_use]
    pub const fn of(role: ArgRole) -> Self {
        Self {
            role,
            noise: None,
            handler: None,
            conditional_binding: false,
            optional: false,
        }
    }

    /// A `?noise?` slot accepting the literal `word` — an optional
    /// [`ArgRole::Keyword`] that is never highlighted as a keyword.
    #[must_use]
    pub const fn noise(word: &'static str) -> Self {
        Self {
            role: ArgRole::Keyword,
            noise: Some(word),
            handler: None,
            conditional_binding: false,
            optional: true,
        }
    }

    /// This slot, made optional.
    #[must_use]
    pub const fn optional(self) -> Self {
        Self {
            optional: true,
            ..self
        }
    }

    /// This slot, selecting its clause by `handler`'s vocabulary.
    #[must_use]
    pub const fn selecting(self, handler: HandlerMatch) -> Self {
        Self {
            handler: Some(handler),
            ..self
        }
    }

    /// This slot, binding its names only when a runtime data condition holds.
    #[must_use]
    pub const fn conditional(self) -> Self {
        Self {
            conditional_binding: true,
            ..self
        }
    }

    /// Whether the slot's role reaches the flat projection (the module docs
    /// list what does not).
    #[must_use]
    pub const fn is_flat(self) -> bool {
        let unlisted_or_binding = matches!(self.role, ArgRole::Value | ArgRole::LoopVarList);
        let handler_pattern = matches!(self.role, ArgRole::Pattern) && self.handler.is_some();
        !(unlisted_or_binding || self.conditional_binding || handler_pattern)
    }
}

impl ClauseRow {
    /// The head of a grammar whose first words are a group (`foreach`'s
    /// binders): no slots, so it supplies no clause, and the default timing,
    /// which nothing reads. A `.tclspec` grammar without a `head` row gets
    /// exactly this.
    pub const EMPTY_HEAD: Self = Self::head(&[], ClauseTiming::Selected);

    /// A head row: keyword `None`, [`ClauseRowShape::Once`], filled
    /// positionally.
    #[must_use]
    pub const fn head(slots: &'static [ClauseSlot], timing: ClauseTiming) -> Self {
        Self::once(None, slots, timing)
    }

    /// Zero or more clauses introduced by `keyword`.
    #[must_use]
    pub const fn repeated(
        keyword: &'static str,
        slots: &'static [ClauseSlot],
        timing: ClauseTiming,
    ) -> Self {
        Self {
            keyword: Some(keyword),
            keyword_required: true,
            shape: ClauseRowShape::Repeated { slots },
            timing,
            surface: None,
        }
    }

    /// Exactly one clause, introduced by `keyword` — or keywordless, entered
    /// positionally in declaration order, when `keyword` is `None`.
    #[must_use]
    pub const fn once(
        keyword: Option<&'static str>,
        slots: &'static [ClauseSlot],
        timing: ClauseTiming,
    ) -> Self {
        Self {
            keyword,
            keyword_required: keyword.is_some(),
            shape: ClauseRowShape::Once { slots },
            timing,
            surface: None,
        }
    }

    /// A keywordless repeating group citing `repeated_args[layout]`.
    #[must_use]
    pub const fn group(layout: u8, timing: ClauseTiming) -> Self {
        Self {
            keyword: None,
            keyword_required: false,
            shape: ClauseRowShape::Group { layout },
            timing,
            surface: None,
        }
    }

    /// This row with its keyword optional (`?else?`): the clause may start
    /// without it, which is what makes a bare trailing body legal.
    #[must_use]
    pub const fn optional_keyword(self) -> Self {
        Self {
            keyword_required: false,
            ..self
        }
    }

    /// This row, available only at `surface`.
    #[must_use]
    pub const fn available(self, surface: &'static [SpecSurface]) -> Self {
        Self {
            surface: Some(surface),
            ..self
        }
    }

    /// The row's slots — empty for a [`ClauseRowShape::Group`], whose words
    /// are its layout's.
    #[must_use]
    pub const fn slots(&self) -> &'static [ClauseSlot] {
        match self.shape {
            ClauseRowShape::Repeated { slots } | ClauseRowShape::Once { slots } => slots,
            ClauseRowShape::Group { .. } => &[],
        }
    }

    fn admits(&self, dialect: Option<&SurfaceQuery<'_>>) -> bool {
        self.surface
            .is_none_or(|rows| surface_admits(rows, dialect))
    }
}

impl ClauseGrammarSpec {
    /// Whether each condition is evaluated at most once and before any clause
    /// body: first-match selection whose every row runs only when selected
    /// (`if` / `elseif` / `else`). Loops, fixtures and protected bodies do not.
    #[must_use]
    pub fn evaluates_conditions_once_before_bodies(&self) -> bool {
        self.selection == ClauseSelection::FirstMatch
            && self.head.timing == ClauseTiming::Selected
            && self
                .rows
                .iter()
                .all(|row| row.timing == ClauseTiming::Selected)
            && self
                .tail
                .is_none_or(|row| row.timing == ClauseTiming::Selected)
    }

    /// Whether the whole grammar is available at `dialect` (`None`: every
    /// release).
    #[must_use]
    pub fn available(&self, dialect: Option<SurfaceQuery<'_>>) -> bool {
        self.surface
            .is_none_or(|rows| surface_admits(rows, dialect.as_ref()))
    }

    /// Walk a call's argument words (after the command, and after the
    /// subcommand word for a subcommand's grammar) with every row available.
    ///
    /// `layouts` is the owning spec's `repeated_args`, which a
    /// [`ClauseRowShape::Group`] row cites.
    #[must_use]
    pub fn walk(&self, args: &[&str], layouts: &[RepeatedArgLayout]) -> ClausePlan {
        self.walk_at(args, layouts, None)
    }

    /// [`Self::walk`] with the rows gated to `dialect`: a row whose `surface`
    /// does not admit the release is absent.
    #[must_use]
    pub fn walk_at(
        &self,
        args: &[&str],
        layouts: &[RepeatedArgLayout],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> ClausePlan {
        let walk = Walk::new(self, args, &[], layouts, dialect);
        walk.run().0
    }

    /// The source-aware walk over the words' *values* — an
    /// `InvocationWord::Literal`'s, what `ResolvedInvocation::clause_plan`
    /// holds — so the fall-through marker is compared exactly: a value that
    /// merely looks braced (`{-}`, from the source word `"{-}"`) is a script,
    /// not the marker, where [`Self::walk`]'s callers hold source spellings
    /// and strip one layer. `dynamic[i]` marks a word whose value only the
    /// runtime knows (its entry in `args` is a placeholder). `None` when such
    /// a word sits where the walk compares a keyword, a noise word or the
    /// fall-through marker — Tcl decides those by value, so no plan is sound.
    #[must_use]
    pub fn walk_words(
        &self,
        args: &[&str],
        dynamic: &[bool],
        layouts: &[RepeatedArgLayout],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ClausePlan> {
        self.walk_words_or_abstain(args, dynamic, layouts, dialect)
            .ok()
    }

    /// [`Self::walk_words`], saying where it abstained: `Err` names the first
    /// computed word the walk compared and carries the call read with every
    /// computed word matching nothing ([`ClauseAbstention`]).
    pub fn walk_words_or_abstain(
        &self,
        args: &[&str],
        dynamic: &[bool],
        layouts: &[RepeatedArgLayout],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Result<ClausePlan, ClauseAbstention> {
        let mut walk = Walk::new(self, args, dynamic, layouts, dialect);
        walk.values = true;
        match walk.run() {
            (plan, None) => Ok(plan),
            (inert, Some(word)) => Err(ClauseAbstention { word, inert }),
        }
    }

    /// Walk authentic argument knowledge without interpreting an unknown
    /// payload as a value. Known non-NUL bytes compare directly with descriptor
    /// keywords; positional operands require only exact cardinality. Expansion
    /// makes the count unavailable, and an unknown compared word abstains.
    #[must_use]
    pub fn walk_arguments(
        &self,
        arguments: crate::InvocationArguments<'_>,
        layouts: &[RepeatedArgLayout],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<Result<ClausePlan, ClauseAbstention>> {
        if !self.available(dialect) {
            return None;
        }
        let count = arguments.exact_argv_len()?;
        let bytes = (0..count)
            .map(|index| match arguments.get(index) {
                Some(crate::InvocationWord::Literal(value)) => Some(value.as_bytes()),
                Some(crate::InvocationWord::KnownBytes(value)) if !value.contains(&0) => {
                    Some(value)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let values = vec![""; count];
        let dynamic = bytes.iter().map(Option::is_none).collect::<Vec<_>>();
        let mut walk = Walk::new(self, &values, &dynamic, layouts, dialect);
        walk.values = true;
        walk.known_bytes = Some(&bytes);
        Some(match walk.run() {
            (plan, None) => Ok(plan),
            (inert, Some(word)) => Err(ClauseAbstention { word, inert }),
        })
    }

    /// Whether `word` is this grammar's fall-through body marker. Tcl compares
    /// the body's string value, so the braced `{-}` and quoted `"-"` forms are
    /// the marker too: one layer of matched braces or quotes is stripped.
    #[must_use]
    pub fn is_fallthrough_body(&self, word: &str) -> bool {
        self.fallthrough_body
            .is_some_and(|marker| strip_one_layer(word) == marker)
    }

    /// Every row in walk order: the head, the rows, the tail.
    pub fn all_rows(&self) -> impl Iterator<Item = (ClauseRowId, &ClauseRow)> {
        std::iter::once((ClauseRowId::Head, &self.head))
            .chain(self.rows.iter().enumerate().map(|(index, row)| {
                (
                    ClauseRowId::Row(u8::try_from(index).unwrap_or(u8::MAX)),
                    row,
                )
            }))
            .chain(self.tail.iter().map(|row| (ClauseRowId::Tail, row)))
    }

    /// The clauses' introducing keywords, in declaration order.
    pub fn keywords(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.all_rows().filter_map(|(_, row)| row.keyword)
    }

    /// The `?noise?` words the slots accept, in declaration order.
    pub fn noise_words(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.all_rows()
            .flat_map(|(_, row)| row.slots().iter().filter_map(|slot| slot.noise))
    }

    /// Whether the words decide where a clause's operands sit — a keyword
    /// row or tail, a noise slot, or a group row whose count the call sets.
    /// A grammar without any (`for`, `while`, `catch`, `dict for`) lays its
    /// clauses out by position alone, as a static role table would.
    #[must_use]
    pub fn layout_depends_on_words(&self) -> bool {
        self.keywords().next().is_some()
            || self.noise_words().next().is_some()
            || self
                .all_rows()
                .any(|(_, row)| matches!(row.shape, ClauseRowShape::Group { .. }))
    }

    /// Whether the walk's flat projection can carry `role` — the grammar's
    /// share of `CommandRegistry::may_have_arg_role`.
    #[must_use]
    pub fn may_assign(&self, role: ArgRole) -> bool {
        if role == ArgRole::Keyword && (self.keywords().next().is_some()) {
            return true;
        }
        self.all_rows().any(|(_, row)| {
            row.slots()
                .iter()
                .any(|slot| slot.role == role && (slot.is_flat() || slot.noise.is_some()))
        })
    }
}

/// Strip one layer of matched `{}` or `""` — how a caller holding a word's
/// source spelling compares it with a string value.
fn strip_one_layer(word: &str) -> &str {
    word.strip_prefix('{')
        .and_then(|inner| inner.strip_suffix('}'))
        .or_else(|| {
            word.strip_prefix('"')
                .and_then(|inner| inner.strip_suffix('"'))
        })
        .unwrap_or(word)
}

/// One pass of the walk: the cursor, what it found, and whether a dynamic
/// word made the answer unsound.
struct Walk<'a> {
    grammar: &'a ClauseGrammarSpec,
    args: &'a [&'a str],
    dynamic: &'a [bool],
    known_bytes: Option<&'a [Option<&'a [u8]>]>,
    /// Whether `args` hold the words' values rather than their source
    /// spellings — see [`ClauseGrammarSpec::walk_words`].
    values: bool,
    layouts: &'a [RepeatedArgLayout],
    dialect: Option<SurfaceQuery<'a>>,
    at: usize,
    clauses: Vec<ResolvedClause>,
    falls: Vec<bool>,
    roles: Vec<(usize, ArgRole)>,
    defect: Option<ClauseShapeError>,
    /// The first computed word the walk compared, when one was: the answer
    /// is then the inert reading, not a plan.
    abstained: Option<usize>,
}

impl<'a> Walk<'a> {
    fn new(
        grammar: &'a ClauseGrammarSpec,
        args: &'a [&'a str],
        dynamic: &'a [bool],
        layouts: &'a [RepeatedArgLayout],
        dialect: Option<SurfaceQuery<'a>>,
    ) -> Self {
        Self {
            grammar,
            args,
            dynamic,
            known_bytes: None,
            values: false,
            layouts,
            dialect,
            at: 0,
            clauses: Vec::new(),
            falls: Vec::new(),
            roles: Vec::new(),
            defect: None,
            abstained: None,
        }
    }

    /// Walk the whole call, with the first computed word it compared when
    /// the answer is unsound.
    fn run(mut self) -> (ClausePlan, Option<usize>) {
        let head = self.grammar.head;
        let mut entered = vec![false; self.grammar.rows.len()];
        let mut tail_taken = false;
        // The head: positional, never keyword-matched. An empty head (the
        // loops whose first words are a group) supplies no clause.
        let no_head = !self.row_admitted(&head)
            || (head.slots().is_empty() && !matches!(head.shape, ClauseRowShape::Group { .. }));
        if no_head || self.enter(ClauseRowId::Head, &head, None) {
            self.chain(&mut entered, &mut tail_taken);
        }
        self.finish()
    }

    /// Everything after the head: one clause per step until the words run
    /// out or a defect stops the walk. A computed word compared on the way
    /// matches nothing, and the walk goes on — the inert reading.
    fn chain(&mut self, entered: &mut [bool], tail_taken: &mut bool) {
        let grammar = self.grammar;
        let rows = grammar.rows;
        loop {
            if self.at >= self.args.len() {
                self.require_unentered(entered, *tail_taken);
                return;
            }
            // 1. A keyworded row whose keyword the word spells.
            if let Some(index) = self.keyword_row_at(entered) {
                entered[index] = true;
                let keyword = self.at;
                self.roles.push((keyword, ArgRole::Keyword));
                self.at += 1;
                if !self.enter(row_id(index), &rows[index], Some(keyword)) {
                    return;
                }
                continue;
            }
            // 2. The next keywordless row not yet entered, in declaration
            //    order.
            if let Some(index) = (0..rows.len()).find(|&index| {
                !entered[index] && rows[index].keyword.is_none() && self.row_admitted(&rows[index])
            }) {
                entered[index] = true;
                if !self.enter(row_id(index), &rows[index], None) {
                    return;
                }
                continue;
            }
            // 3. The tail — last, so anything after it is extra.
            if !*tail_taken && let Some(tail) = grammar.tail.filter(|tail| self.row_admitted(tail))
            {
                *tail_taken = true;
                self.enter_tail(&tail);
                return;
            }
            // 4. Nothing more is a clause.
            self.stop(ClauseShapeError::ExtraWords {
                first_extra: self.at,
            });
            return;
        }
    }

    /// Enter the tail at the cursor: its keyword when the word spells it, no
    /// keyword when the keyword is optional, and nothing more otherwise.
    fn enter_tail(&mut self, tail: &ClauseRow) {
        let keyword_index = match tail.keyword {
            Some(keyword) => {
                if self.word_is(self.at, keyword) {
                    let index = self.at;
                    self.roles.push((index, ArgRole::Keyword));
                    self.at += 1;
                    Some(index)
                } else if tail.keyword_required {
                    // A tail whose keyword is mandatory does not start here,
                    // so nothing more is a clause.
                    self.stop(ClauseShapeError::ExtraWords {
                        first_extra: self.at,
                    });
                    return;
                } else {
                    // `?else?` — the optional keyword that makes a bare
                    // trailing body legal.
                    None
                }
            }
            None => None,
        };
        if self.enter(ClauseRowId::Tail, tail, keyword_index) && self.at < self.args.len() {
            self.stop(ClauseShapeError::ExtraWords {
                first_extra: self.at,
            });
        }
    }

    /// The words ran out: a keywordless row or tail with a required slot that
    /// no word filled is missing. A keyworded row or tail is an optional
    /// clause.
    fn require_unentered(&mut self, entered: &[bool], tail_taken: bool) {
        let grammar = self.grammar;
        for (index, row) in grammar.rows.iter().enumerate() {
            if entered[index] || row.keyword.is_some() || !self.row_admitted(row) {
                continue;
            }
            if let Some(role) = required_role(row) {
                self.missing(role);
                return;
            }
        }
        if !tail_taken
            && let Some(tail) = grammar.tail
            && tail.keyword.is_none()
            && self.row_admitted(&tail)
            && let Some(role) = required_role(&tail)
        {
            self.missing(role);
        }
    }

    /// The index of a keyworded row whose keyword the cursor's word spells: a
    /// `Repeated` row any number of times, a keyworded `Once` row once.
    fn keyword_row_at(&mut self, entered: &[bool]) -> Option<usize> {
        let rows = self.grammar.rows;
        for (index, row) in rows.iter().enumerate() {
            let Some(keyword) = row.keyword else {
                continue;
            };
            let repeatable = matches!(row.shape, ClauseRowShape::Repeated { .. });
            if (!repeatable && entered[index]) || !self.row_admitted(row) {
                continue;
            }
            if self.word_is(self.at, keyword) {
                return Some(index);
            }
        }
        None
    }

    /// Fill one clause of `row`; `false` when a defect stops the walk.
    fn enter(&mut self, id: ClauseRowId, row: &ClauseRow, keyword_index: Option<usize>) -> bool {
        match row.shape {
            ClauseRowShape::Repeated { slots } | ClauseRowShape::Once { slots } => {
                self.fill(id, row, slots, keyword_index)
            }
            ClauseRowShape::Group { layout } => self.fill_group(id, row, layout),
        }
    }

    /// Fill `slots` positionally from the cursor.
    fn fill(
        &mut self,
        id: ClauseRowId,
        row: &ClauseRow,
        slots: &[ClauseSlot],
        keyword_index: Option<usize>,
    ) -> bool {
        let mut operands = Vec::with_capacity(slots.len());
        let mut falls = false;
        for slot in slots {
            if let Some(noise) = slot.noise {
                if self.at < self.args.len() && self.word_is(self.at, noise) {
                    operands.push((self.at, *slot));
                    self.roles.push((self.at, ArgRole::Keyword));
                    self.at += 1;
                }
                continue;
            }
            if self.at >= self.args.len() {
                if slot.optional {
                    continue;
                }
                self.push_clause(id, row, keyword_index, operands, falls);
                self.missing(slot.role);
                return false;
            }
            let index = self.at;
            self.at += 1;
            operands.push((index, *slot));
            let marker = slot.role == ArgRole::Body
                && row.timing == ClauseTiming::Selected
                && self.grammar.fallthrough_body.is_some()
                && self.is_fallthrough_at(index);
            if marker {
                falls = true;
            } else if slot.is_flat() {
                self.roles.push((index, slot.role));
            }
        }
        self.push_clause(id, row, keyword_index, operands, falls);
        true
    }

    /// Fill a [`ClauseRowShape::Group`] row: the cited layout's stride at a
    /// time, up to the words its trailing exclusion leaves.
    ///
    /// At least one whole group is required; a partial last group is a defect
    /// the walk records and passes, so the excluded trailing words still reach
    /// the rest of the grammar (`foreach a b c body`'s body is its last word).
    fn fill_group(&mut self, id: ClauseRowId, row: &ClauseRow, layout: u8) -> bool {
        let Some(layout) = self.layouts.get(usize::from(layout)).copied() else {
            // A row citing a layout the spec does not declare consumes
            // nothing; `clause_grammar_group_rows_cite_a_real_layout` keeps
            // the shipped grammars honest.
            return true;
        };
        let n = self.args.len();
        let stride = usize::from(layout.stride.max(1));
        let end = n
            .saturating_sub(usize::from(layout.exclude_trailing))
            .max(self.at);
        let covered = layout.indices(n);
        let slot_at = |index: usize| {
            let bound = covered.contains(&index);
            ClauseSlot {
                role: if bound { layout.role } else { ArgRole::Value },
                noise: None,
                handler: None,
                conditional_binding: bound && layout.conditional_binding,
                optional: false,
            }
        };
        if end - self.at < stride {
            let operands = (self.at..end)
                .map(|index| (index, slot_at(index)))
                .collect();
            self.push_clause(id, row, None, operands, false);
            self.at = end;
            self.missing(ArgRole::Value);
            return false;
        }
        while self.at + stride <= end {
            let operands = (self.at..self.at + stride)
                .map(|index| (index, slot_at(index)))
                .collect();
            self.push_clause(id, row, None, operands, false);
            self.at += stride;
        }
        if self.at < end {
            let operands = (self.at..end)
                .map(|index| (index, slot_at(index)))
                .collect();
            self.push_clause(id, row, None, operands, false);
            self.at = end;
            if self.defect.is_none() {
                self.defect = Some(ClauseShapeError::MissingBody { after: end - 1 });
            }
        }
        true
    }

    fn push_clause(
        &mut self,
        id: ClauseRowId,
        row: &ClauseRow,
        keyword_index: Option<usize>,
        operands: Vec<(usize, ClauseSlot)>,
        falls: bool,
    ) {
        self.clauses.push(ResolvedClause {
            keyword_index,
            row: id,
            operands,
            timing: row.timing,
            falls_through_to: None,
            is_default: false,
        });
        self.falls.push(falls);
    }

    /// Record the defect of a required slot no word filled.
    fn missing(&mut self, role: ArgRole) {
        let after = self.at.checked_sub(1);
        self.stop(if role == ArgRole::Expr {
            ClauseShapeError::MissingExpr { after }
        } else {
            // `after` is the last present word; a clause other than the
            // head is never entered with nothing before it.
            ClauseShapeError::MissingBody {
                after: after.unwrap_or(0),
            }
        });
    }

    fn stop(&mut self, defect: ClauseShapeError) {
        if self.defect.is_none() {
            self.defect = Some(defect);
        }
    }

    fn row_admitted(&self, row: &ClauseRow) -> bool {
        row.admits(self.dialect.as_ref())
    }

    /// Whether the word at `index` is `literal`; a dynamic word matches
    /// nothing and makes the walk unsound.
    fn word_is(&mut self, index: usize, literal: &str) -> bool {
        if self.is_dynamic(index) {
            self.abstained.get_or_insert(index);
            return false;
        }
        if let Some(words) = self.known_bytes {
            return words.get(index).copied().flatten() == Some(literal.as_bytes());
        }
        self.args.get(index).is_some_and(|word| *word == literal)
    }

    fn is_fallthrough_at(&mut self, index: usize) -> bool {
        if self.is_dynamic(index) {
            self.abstained.get_or_insert(index);
            return false;
        }
        if let Some(words) = self.known_bytes {
            return self.grammar.fallthrough_body.is_some_and(|marker| {
                words.get(index).copied().flatten() == Some(marker.as_bytes())
            });
        }
        self.args.get(index).is_some_and(|word| {
            if self.values {
                self.grammar.fallthrough_body == Some(*word)
            } else {
                self.grammar.is_fallthrough_body(word)
            }
        })
    }

    fn is_dynamic(&self, index: usize) -> bool {
        self.dynamic.get(index).copied().unwrap_or(false)
    }

    /// Link fall-throughs, mark the default clause, and hand back the plan.
    ///
    /// A marker runs the body of the next selected clause that supplies one
    /// of its own. A clause that runs whatever the outcome (`try`'s
    /// `finally`) is never that clause, so a marker with no selected clause
    /// after it has no target — Tcl's "last non-finally clause must not have
    /// a body of `-`".
    fn finish(mut self) -> (ClausePlan, Option<usize>) {
        let supplies_body = |clause: &ResolvedClause| {
            clause.timing == ClauseTiming::Selected
                && clause
                    .operands
                    .iter()
                    .any(|(_, slot)| slot.role == ArgRole::Body)
        };
        for index in 0..self.clauses.len() {
            if self.falls[index] {
                self.clauses[index].falls_through_to = (index + 1..self.clauses.len())
                    .find(|&next| !self.falls[next] && supplies_body(&self.clauses[next]));
            }
        }
        if let Some(default) = self.grammar.default_clause {
            let row = default.row.map_or(ClauseRowId::Tail, ClauseRowId::Row);
            if let Some(position) = self.clauses.iter().position(|clause| clause.row == row) {
                self.clauses[position].is_default = true;
                if default.final_only
                    && let Some(next) = self.clauses.get(position + 1)
                {
                    let first_extra = next
                        .keyword_index
                        .or_else(|| next.operands.first().map(|(index, _)| *index));
                    if let Some(first_extra) = first_extra {
                        self.stop(ClauseShapeError::ExtraWords { first_extra });
                    }
                }
            }
        }
        (
            ClausePlan {
                clauses: self.clauses,
                roles: self.roles,
                defect: self.defect,
            },
            self.abstained,
        )
    }
}

fn row_id(index: usize) -> ClauseRowId {
    ClauseRowId::Row(u8::try_from(index).unwrap_or(u8::MAX))
}

/// The role of a row's first required slot, or `None` when every slot may be
/// absent. A group row always requires one whole group.
fn required_role(row: &ClauseRow) -> Option<ArgRole> {
    match row.shape {
        ClauseRowShape::Group { .. } => Some(ArgRole::Value),
        ClauseRowShape::Repeated { slots } | ClauseRowShape::Once { slots } => slots
            .iter()
            .find(|slot| !slot.optional && slot.noise.is_none())
            .map(|slot| slot.role),
    }
}

/// One word a clause grammar matches by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClauseKeyword {
    /// The literal word.
    pub word: &'static str,
    /// The command whose grammar declares it.
    pub owner: &'static str,
    /// Whether the word is a `?noise?` word — accepted as filler, never
    /// highlighted — rather than a clause's introducing keyword.
    pub noise: bool,
}

/// Every word the clause grammars of `registry` match by value: each row's
/// introducing keyword and each slot's noise word, with the command that
/// declares it. The first declaration (in command-name order) owns a word
/// two grammars share.
#[must_use]
pub fn clause_keywords(registry: &CommandRegistry) -> Vec<ClauseKeyword> {
    let mut names: Vec<&str> = registry.command_names().collect();
    names.sort_unstable();
    let mut out: Vec<ClauseKeyword> = Vec::new();
    for name in names {
        let Some(spec) = registry.get(name) else {
            continue;
        };
        let grammars = spec
            .clause_grammar
            .into_iter()
            .chain(spec.subcommands.iter().filter_map(|sub| sub.clause_grammar));
        for grammar in grammars {
            let keywords = grammar.keywords().map(|word| (word, false));
            let noise = grammar.noise_words().map(|word| (word, true));
            for (word, noise) in keywords.chain(noise) {
                if !out
                    .iter()
                    .any(|found| found.word == word && found.noise == noise)
                {
                    out.push(ClauseKeyword {
                        word,
                        owner: spec.name,
                        noise,
                    });
                }
            }
        }
    }
    out
}

/// [`clause_keywords`] over the shipped registry, derived once.
pub(crate) fn shipped_clause_keywords() -> &'static [ClauseKeyword] {
    static KEYWORDS: OnceLock<Vec<ClauseKeyword>> = OnceLock::new();
    KEYWORDS.get_or_init(|| clause_keywords(crate::cache::default_registry()))
}

/// The command whose shipped clause grammar matches `word` by value — `if`
/// for `else`, `elseif` and `then`; `try` for `on`, `trap` and `finally` — or
/// `None` for any other word. What a stray clause word (a misplaced newline
/// before `else`) is reported against.
#[must_use]
pub fn owner_of_keyword(word: &str) -> Option<&'static str> {
    shipped_clause_keywords()
        .iter()
        .find(|keyword| keyword.word == word)
        .map(|keyword| keyword.owner)
}

#[cfg(test)]
mod tests;
