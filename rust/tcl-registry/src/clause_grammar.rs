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
use crate::clause_shape::ClauseShapeError;
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
    /// [`ArgRole::Body`] — or `None` when no such slot was filled.
    #[must_use]
    pub fn operand(&self, role: ArgRole) -> Option<usize> {
        self.operands
            .iter()
            .find(|(_, slot)| slot.role == role)
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
        self.args.get(index).is_some_and(|word| *word == literal)
    }

    fn is_fallthrough_at(&mut self, index: usize) -> bool {
        if self.is_dynamic(index) {
            self.abstained.get_or_insert(index);
            return false;
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
mod tests {
    use super::*;
    use crate::InvocationWords;
    use tcl_dialect::model::Family;

    use ArgRole::{Body, Expr, Keyword};
    use ClauseShapeError::{ExtraWords, MissingBody, MissingExpr};

    type Roles = &'static [(usize, ArgRole)];

    fn shipped(name: &str) -> &'static ClauseGrammarSpec {
        crate::cache::default_registry()
            .get(name)
            .and_then(|spec| spec.clause_grammar)
            .unwrap_or_else(|| panic!("`{name}` declares a clause grammar"))
    }

    fn shipped_sub(
        name: &str,
        sub: &str,
    ) -> (&'static ClauseGrammarSpec, &'static [RepeatedArgLayout]) {
        let sub = crate::cache::default_registry()
            .get(name)
            .and_then(|spec| spec.subcommands.iter().find(|found| found.name == sub))
            .unwrap_or_else(|| panic!("`{name} {sub}` is shipped"));
        (
            sub.clause_grammar
                .unwrap_or_else(|| panic!("`{name} {}` declares a clause grammar", sub.name)),
            sub.repeated_args,
        )
    }

    fn walk(grammar: &ClauseGrammarSpec, call: &str) -> ClausePlan {
        let words: Vec<&str> = call.split_whitespace().collect();
        grammar.walk(&words, &[])
    }

    /// Every call shape `if_.rs`'s retired walk was tested on — its 23 shape
    /// cases and the full chain of the port's matrix — with the roles and the
    /// defect that walk answered, captured before it retired. Each case was
    /// cross-checked against tclsh 8.6 and Tcl 9.0.4's `TclNRIfObjCmd` /
    /// `IfConditionCallback` (`generic/tclCmdIL.c`): well-formed shapes, and
    /// shapes that fail only at runtime (an invalid-bareword condition), have
    /// no defect.
    const IF_CORPUS: &[(&str, Roles, Option<ClauseShapeError>)] = &[
        ("", &[], Some(MissingExpr { after: None })),
        ("1", &[(0, Expr)], Some(MissingBody { after: 0 })),
        (
            "1 then",
            &[(0, Expr), (1, Keyword)],
            Some(MissingBody { after: 1 }),
        ),
        ("1 a", &[(0, Expr), (1, Body)], None),
        ("1 then a", &[(0, Expr), (1, Keyword), (2, Body)], None),
        ("1 a b", &[(0, Expr), (1, Body), (2, Body)], None),
        (
            "1 a b c",
            &[(0, Expr), (1, Body), (2, Body)],
            Some(ExtraWords { first_extra: 3 }),
        ),
        (
            "1 a b c d",
            &[(0, Expr), (1, Body), (2, Body)],
            Some(ExtraWords { first_extra: 3 }),
        ),
        (
            "1 a else b",
            &[(0, Expr), (1, Body), (2, Keyword), (3, Body)],
            None,
        ),
        (
            "1 a else",
            &[(0, Expr), (1, Body), (2, Keyword)],
            Some(MissingBody { after: 2 }),
        ),
        (
            "1 a else b c",
            &[(0, Expr), (1, Body), (2, Keyword), (3, Body)],
            Some(ExtraWords { first_extra: 4 }),
        ),
        (
            "a x elseif b y",
            &[(0, Expr), (1, Body), (2, Keyword), (3, Expr), (4, Body)],
            None,
        ),
        (
            "a x elseif b y else z",
            &[
                (0, Expr),
                (1, Body),
                (2, Keyword),
                (3, Expr),
                (4, Body),
                (5, Keyword),
                (6, Body),
            ],
            None,
        ),
        (
            "1 a elseif",
            &[(0, Expr), (1, Body), (2, Keyword)],
            Some(MissingExpr { after: Some(2) }),
        ),
        (
            "1 a elseif 2",
            &[(0, Expr), (1, Body), (2, Keyword), (3, Expr)],
            Some(MissingBody { after: 3 }),
        ),
        (
            "1 a elseif 2 then",
            &[(0, Expr), (1, Body), (2, Keyword), (3, Expr), (4, Keyword)],
            Some(MissingBody { after: 4 }),
        ),
        (
            "1 a elseif 2 b c",
            &[
                (0, Expr),
                (1, Body),
                (2, Keyword),
                (3, Expr),
                (4, Body),
                (5, Body),
            ],
            None,
        ),
        (
            "1 a elseif 2 b c d",
            &[
                (0, Expr),
                (1, Body),
                (2, Keyword),
                (3, Expr),
                (4, Body),
                (5, Body),
            ],
            Some(ExtraWords { first_extra: 6 }),
        ),
        ("else a", &[(0, Expr), (1, Body)], None),
        ("elseif a", &[(0, Expr), (1, Body)], None),
        (
            "1 a elseif else b",
            &[(0, Expr), (1, Body), (2, Keyword), (3, Expr), (4, Body)],
            None,
        ),
        (
            "1 a then b",
            &[(0, Expr), (1, Body), (2, Body)],
            Some(ExtraWords { first_extra: 3 }),
        ),
        (
            "1 a else then",
            &[(0, Expr), (1, Body), (2, Keyword), (3, Body)],
            None,
        ),
        (
            "1 then a elseif 2 then b else c",
            &[
                (0, Expr),
                (1, Keyword),
                (2, Body),
                (3, Keyword),
                (4, Expr),
                (5, Keyword),
                (6, Body),
                (7, Keyword),
                (8, Body),
            ],
            None,
        ),
    ];

    #[test]
    fn if_grammar_agrees_with_the_retired_walk_on_its_corpus() {
        assert_eq!(IF_CORPUS.len(), 24);
        let grammar = shipped("if");
        for (call, roles, defect) in IF_CORPUS {
            let plan = walk(grammar, call);
            assert_eq!(plan.roles, *roles, "roles for `if {call}`");
            assert_eq!(plan.defect, *defect, "defect for `if {call}`");
        }
    }

    /// Well-formed `try` calls, with the roles the retired `try_arg_roles`
    /// answered: a handler's pattern and variable list carry no flat role, and
    /// a `-` body carries no `Body`.
    const TRY_CORPUS: &[(&[&str], Roles)] = &[
        (
            &[
                "{...}", "on", "ok", "result", "-", "trap", "NONE", "result", "{...}",
            ],
            &[(0, Body), (1, Keyword), (5, Keyword), (8, Body)],
        ),
        (
            &[
                "{...}", "on", "ok", "a", "{-}", "trap", "NONE", "b", "{...}",
            ],
            &[(0, Body), (1, Keyword), (5, Keyword), (8, Body)],
        ),
        (
            &[
                "{...}", "on", "ok", "a", "\"-\"", "trap", "NONE", "b", "{...}",
            ],
            &[(0, Body), (1, Keyword), (5, Keyword), (8, Body)],
        ),
        (
            &["{...}", "on", "error", "msg", "{puts $msg}"],
            &[(0, Body), (1, Keyword), (4, Body)],
        ),
        (
            &["{b}", "on", "ok", "{}", "{h}"],
            &[(0, Body), (1, Keyword), (4, Body)],
        ),
        (
            &["{b}", "trap", "{POSIX ENOENT}", "{}", "{h}"],
            &[(0, Body), (1, Keyword), (4, Body)],
        ),
        (
            &[
                "{b}",
                "trap",
                "{POSIX ENOENT}",
                "{m o}",
                "{h}",
                "finally",
                "{f}",
            ],
            &[(0, Body), (1, Keyword), (4, Body), (5, Keyword), (6, Body)],
        ),
        (
            &["{b}", "finally", "{f}"],
            &[(0, Body), (1, Keyword), (2, Body)],
        ),
        (&["{b}"], &[(0, Body)]),
        (
            &[
                "{b}", "on", "error", "{m o}", "-", "on", "break", "{}", "{h}", "finally", "{f}",
            ],
            &[
                (0, Body),
                (1, Keyword),
                (5, Keyword),
                (8, Body),
                (9, Keyword),
                (10, Body),
            ],
        ),
    ];

    #[test]
    fn try_grammar_agrees_with_the_retired_walk() {
        let grammar = shipped("try");
        for (call, roles) in TRY_CORPUS {
            let plan = grammar.walk(call, &[]);
            assert_eq!(plan.roles, *roles, "roles for `try {call:?}`");
            assert_eq!(plan.defect, None, "defect for `try {call:?}`");
        }
        // `on ok`: the handler's pattern selects by completion code; the
        // variable list is a clause fact.
        let plan = grammar.walk(&["{b}", "on", "ok", "{r o}", "{h}"], &[]);
        let handler = &plan.clauses[1];
        assert_eq!(handler.keyword_index, Some(1));
        assert_eq!(handler.timing, ClauseTiming::Selected);
        assert_eq!(
            handler.operands[0].1.handler,
            Some(HandlerMatch::CompletionCode)
        );
        assert_eq!(handler.operands[1].1.role, ArgRole::LoopVarList);
        assert_eq!(plan.clauses[0].timing, ClauseTiming::Protected);
        // `trap {POSIX ENOENT}` selects by errorcode prefix.
        let plan = grammar.walk(&["{b}", "trap", "{POSIX ENOENT}", "{}", "{h}"], &[]);
        assert_eq!(
            plan.clauses[1].operands[0].1.handler,
            Some(HandlerMatch::ErrorCodePrefix)
        );
        // A `-` body runs the next handler's body; `finally` always runs.
        let plan = grammar.walk(
            &[
                "{b}", "on", "error", "{}", "-", "trap", "{X}", "{}", "{h}", "finally", "{f}",
            ],
            &[],
        );
        assert_eq!(plan.clauses[1].falls_through_to, Some(2));
        assert_eq!(plan.clauses[2].falls_through_to, None);
        assert_eq!(plan.clauses[3].row, ClauseRowId::Tail);
        assert_eq!(plan.clauses[3].timing, ClauseTiming::Always);
        assert!(plan.clauses.iter().all(|clause| !clause.is_default));
    }

    /// The retired scan skipped a word it did not recognise and kept looking
    /// for a keyword; the grammar stops there, as the chain does in Tcl
    /// (`bad handler type`), and reports where the words stopped making sense.
    #[test]
    fn a_malformed_try_stops_at_its_first_defect() {
        let grammar = shipped("try");
        let plan = grammar.walk(&["{b}", "foo", "on", "e", "m", "h"], &[]);
        assert_eq!(plan.roles, [(0, Body)]);
        assert_eq!(plan.defect, Some(ExtraWords { first_extra: 1 }));
        let plan = grammar.walk(&["{b}", "on", "x", "y"], &[]);
        assert_eq!(plan.roles, [(0, Body), (1, Keyword)]);
        assert_eq!(plan.defect, Some(MissingBody { after: 3 }));
        let plan = grammar.walk(&["{b}", "finally", "f", "on", "e", "m", "h"], &[]);
        assert_eq!(plan.roles, [(0, Body), (1, Keyword), (2, Body)]);
        assert_eq!(plan.defect, Some(ExtraWords { first_extra: 3 }));
        assert_eq!(
            grammar.walk(&[], &[]).defect,
            Some(MissingBody { after: 0 })
        );
    }

    /// A marker runs the next *handler's* body: `finally` runs whatever the
    /// outcome, so a marker with only `finally` after it has no target —
    /// Tcl's "last non-finally clause must not have a body of `-`" — while
    /// [`ClausePlan::falls_through`] still names the clause whose body is the
    /// marker.
    #[test]
    fn a_marker_falls_through_to_the_next_handler_never_to_finally() {
        let grammar = shipped("try");
        let plan = grammar.walk(&["{b}", "on", "ok", "{}", "-", "finally", "{f}"], &[]);
        assert_eq!(plan.defect, None);
        assert_eq!(plan.clauses[1].falls_through_to, None);
        assert!(plan.falls_through(1));
        assert!(!plan.falls_through(2), "`finally` is a script");
        assert!(!plan.falls_through(0), "the protected body is a script");
        let plan = grammar.walk(
            &[
                "{b}", "on", "ok", "{}", "-", "on", "error", "{}", "-", "trap", "{X}", "{}", "{h}",
            ],
            &[],
        );
        assert_eq!(plan.clauses[1].falls_through_to, Some(3));
        assert_eq!(plan.clauses[2].falls_through_to, Some(3));
        assert!(plan.falls_through(1) && plan.falls_through(2));
        assert!(!plan.falls_through(3));
        assert!(!plan.falls_through(9), "no such clause");
    }

    /// The value walk compares the marker exactly: a word whose *value* is
    /// `{-}` (the source `"{-}"`) is a script, where a source spelling `{-}`
    /// is the braced marker.
    #[test]
    fn the_value_walk_compares_the_marker_exactly() {
        let grammar = shipped("try");
        let words = |body: &'static str| ["b", "on", "ok", "", body, "trap", "X", "", "h"];
        let plan = grammar
            .walk_words(&words("-"), &[false; 9], &[], None)
            .expect("every word is a value");
        assert_eq!(plan.clauses[1].falls_through_to, Some(2));
        let plan = grammar
            .walk_words(&words("{-}"), &[false; 9], &[], None)
            .expect("every word is a value");
        assert!(!plan.falls_through(1), "the value `{{-}}` is a script");
        assert!(plan.roles.contains(&(4, Body)));
        assert!(grammar.walk(&words("{-}"), &[]).falls_through(1));
        // A computed body word is where the marker is compared: no plan.
        let mut dynamic = [false; 9];
        dynamic[4] = true;
        assert_eq!(grammar.walk_words(&words(""), &dynamic, &[], None), None);
    }

    #[test]
    fn a_resolved_clause_names_its_operands_and_handler() {
        let plan = shipped("try").walk(&["{b}", "on", "ok", "{r o}", "{h}"], &[]);
        let handler = &plan.clauses[1];
        assert_eq!(handler.operand(Body), Some(4));
        assert_eq!(handler.operand(ArgRole::LoopVarList), Some(3));
        assert_eq!(handler.operand(Expr), None);
        assert_eq!(handler.handler(), Some((2, HandlerMatch::CompletionCode)));
        assert_eq!(plan.clauses[0].handler(), None);
        let plan = walk(shipped("if"), "{$c} then {a} else {b}");
        assert_eq!(plan.clauses[0].operand(Expr), Some(0));
        assert_eq!(plan.clauses[0].operand(Keyword), Some(1), "the noise word");
        assert_eq!(plan.clauses[0].operand(Body), Some(2));
        assert_eq!(plan.clauses[1].operand(Body), Some(4));
        assert!(plan.clauses[1].is_default);
    }

    #[test]
    fn foreach_group_row_cites_layout_zero() {
        let spec = crate::cache::default_registry()
            .get("foreach")
            .expect("foreach is shipped");
        let grammar = spec.clause_grammar.expect("foreach declares a grammar");
        assert_eq!(grammar.rows[0].shape, ClauseRowShape::Group { layout: 0 });
        let layout = spec.repeated_args[0];
        assert_eq!(layout.role, ArgRole::LoopVarList);
        let plan = grammar.walk(&["x", "{a b}", "y", "$l", "{body}"], spec.repeated_args);
        // Two binder groups, then the body; the groups' words are the
        // layout's, so only the body is a flat role.
        assert_eq!(plan.roles, [(4, Body)]);
        assert_eq!(plan.defect, None);
        let groups: Vec<Vec<(usize, ArgRole)>> = plan
            .clauses
            .iter()
            .filter(|clause| clause.row == ClauseRowId::Row(0))
            .map(|clause| {
                clause
                    .operands
                    .iter()
                    .map(|(index, slot)| (*index, slot.role))
                    .collect()
            })
            .collect();
        assert_eq!(
            groups,
            [
                vec![(0, ArgRole::LoopVarList), (1, ArgRole::Value)],
                vec![(2, ArgRole::LoopVarList), (3, ArgRole::Value)],
            ]
        );
        assert!(
            plan.clauses
                .iter()
                .all(|clause| clause.timing == ClauseTiming::PerIteration)
        );
        // The retired resolver's rule: the last word is the body once there
        // are three words, and nothing is a body before that — including
        // the off-by-one shapes.
        for (call, body) in [
            ("", None),
            ("x", None),
            ("x l", None),
            ("x l b", Some(2)),
            ("x l y b", Some(3)),
            ("x l y m b", Some(4)),
        ] {
            let words: Vec<&str> = call.split_whitespace().collect();
            let plan = grammar.walk(&words, spec.repeated_args);
            let bodies: Vec<usize> = plan
                .roles
                .iter()
                .filter(|(_, role)| *role == Body)
                .map(|(index, _)| *index)
                .collect();
            assert_eq!(
                bodies,
                body.into_iter().collect::<Vec<_>>(),
                "`foreach {call}`"
            );
            assert_eq!(
                plan.defect.is_none(),
                matches!(call, "x l b" | "x l y m b"),
                "`foreach {call}`"
            );
        }
    }

    #[test]
    fn array_for_is_absent_below_9_0() {
        let registry = crate::cache::default_registry();
        let words = InvocationWords::literals("array", &["for", "{k v}", "a", "{body}"]);
        let at = |release: &str| {
            registry
                .resolve_structured_invocation(
                    words,
                    Some(SurfaceQuery::core(Family::Tcl, release)),
                )
                .resolved()
                .and_then(|invocation| invocation.clause_plan())
        };
        assert!(at("8.6").is_none(), "`array for` is Tcl 9.0");
        let plan = at("9.0").expect("`array for` walks at 9.0");
        // In the invocation's own coordinates: the subcommand word is 0.
        assert_eq!(plan.roles, [(3, Body)]);
        let (grammar, _) = shipped_sub("array", "for");
        assert!(!grammar.available(Some(SurfaceQuery::core(Family::Tcl, "8.6"))));
        assert!(grammar.available(Some(SurfaceQuery::core(Family::Tcl, "9.0"))));
    }

    #[test]
    fn a_keyword_never_matches_inside_a_slot() {
        let grammar = shipped("if");
        // `if else {a}` — the condition is the bareword `else`.
        let plan = walk(grammar, "else a");
        assert_eq!(plan.roles, [(0, Expr), (1, Body)]);
        assert_eq!(plan.defect, None);
        assert_eq!(plan.clauses.len(), 1);
        assert_eq!(plan.clauses[0].keyword_index, None);
        // `elseif`'s own condition slot is positional too.
        let plan = walk(grammar, "1 a elseif else b");
        assert_eq!(plan.clauses[1].operands[0], (3, ClauseSlot::of(Expr)));
        // `then` is noise only right after a condition.
        let plan = walk(grammar, "1 a else then");
        assert_eq!(plan.clauses[1].operands[0].0, 3);
        assert!(plan.clauses[1].is_default, "the else body is the default");
    }

    #[test]
    fn a_dynamic_word_where_a_keyword_could_be_abstains() {
        let grammar = shipped("if");
        let args = ["$c", "{a}", "$k", "{b}"];
        // The condition slot never compares, so a computed condition is fine.
        assert!(
            grammar
                .walk_words(&args[..2], &[true, false], &[], None)
                .is_some()
        );
        // A computed word where `elseif` / `else` could stand is Tcl's call.
        assert!(
            grammar
                .walk_words(&args, &[true, false, true, false], &[], None)
                .is_none()
        );
        let try_grammar = shipped("try");
        assert!(
            try_grammar
                .walk_words(
                    &["{b}", "on", "error", "{}", "$h"],
                    &[false, false, false, false, true],
                    &[],
                    None
                )
                .is_none(),
            "a computed handler body could be the `-` marker"
        );
    }

    #[test]
    fn positional_grammars_fill_their_rows_in_declaration_order() {
        let for_grammar = shipped("for");
        let plan = walk(for_grammar, "{set i 0} {$i < 3} {incr i} {body}");
        assert_eq!(plan.roles, [(0, Body), (1, Expr), (2, Body), (3, Body)]);
        let timings: Vec<ClauseTiming> = plan.clauses.iter().map(|clause| clause.timing).collect();
        assert_eq!(
            timings,
            [
                ClauseTiming::LoopFixture(LoopPhase::Init),
                ClauseTiming::Selected,
                ClauseTiming::LoopFixture(LoopPhase::Next),
                ClauseTiming::PerIteration,
            ]
        );
        assert_eq!(
            walk(for_grammar, "a b").defect,
            Some(MissingBody { after: 1 })
        );
        assert_eq!(
            walk(for_grammar, "a").defect,
            Some(MissingExpr { after: Some(0) })
        );
        assert_eq!(
            walk(for_grammar, "a b c d e").defect,
            Some(ExtraWords { first_extra: 4 })
        );
        let while_grammar = shipped("while");
        assert_eq!(walk(while_grammar, "1 b").roles, [(0, Expr), (1, Body)]);
        assert_eq!(
            walk(while_grammar, "1").defect,
            Some(MissingBody { after: 0 })
        );
        let catch = shipped("catch");
        for (call, defect) in [
            ("s", None),
            ("s r", None),
            ("s r o", None),
            ("s r o x", Some(ExtraWords { first_extra: 3 })),
            ("", Some(MissingBody { after: 0 })),
        ] {
            let plan = walk(catch, call);
            assert_eq!(plan.defect, defect, "`catch {call}`");
            // The result and options words are clause facts; `catch`'s own
            // `arg_roles` carries their `VarWrite`.
            assert!(
                plan.roles.iter().all(|(_, role)| *role == Body),
                "`catch {call}`"
            );
        }
    }

    #[test]
    fn dict_update_binds_its_pairs_conditionally() {
        let (grammar, layouts) = shipped_sub("dict", "update");
        let plan = grammar.walk(&["d", "k1", "v1", "k2", "v2", "{body}"], layouts);
        assert_eq!(plan.defect, None);
        assert_eq!(plan.roles, [(5, Body)]);
        let bound: Vec<usize> = plan
            .clauses
            .iter()
            .flat_map(|clause| clause.operands.iter())
            .filter(|(_, slot)| slot.role == ArgRole::LoopVarList)
            .map(|(index, slot)| {
                assert!(slot.conditional_binding);
                *index
            })
            .collect();
        assert_eq!(bound, [2, 4]);
        let body = plan.clauses.last().expect("the body clause");
        assert_eq!(body.timing, ClauseTiming::Always);
    }

    #[test]
    fn the_shipped_clause_words_have_their_owners() {
        for (word, owner) in [
            ("else", "if"),
            ("elseif", "if"),
            ("then", "if"),
            ("on", "try"),
            ("trap", "try"),
            ("finally", "try"),
        ] {
            assert_eq!(owner_of_keyword(word), Some(owner), "`{word}`");
        }
        assert_eq!(owner_of_keyword("set"), None);
        let noise: Vec<&str> = shipped_clause_keywords()
            .iter()
            .filter(|keyword| keyword.noise)
            .map(|keyword| keyword.word)
            .collect();
        assert_eq!(noise, ["then"]);
    }
}
