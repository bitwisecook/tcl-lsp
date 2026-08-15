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

//! Definition-body grammars for class/type *definer* commands.
//!
//! A *definer* command's script argument (`oo::class create Name { … }`,
//! `snit::type Name { … }`, the bare `oo::define Target { … }` script form) is
//! a **definition body**: its top-level words are member sub-keywords
//! (`method`, `typemethod`, `constructor`, `variable`, …) rather than ordinary
//! commands.  Those keywords are context-sensitive — they only mean anything
//! inside a definition body, and have no standalone [`CommandSpec`] — so their
//! argument layout is described here, as registry *data*, and consumed
//! generically by the shared definition-body walker
//! ([`tcl_lsp_core::oo_body`], used by folding + semantic tokens).
//!
//! The point is that adding (or completing) a definer — snit, xotcl, a custom
//! class system — is a matter of writing a [`DefinitionBodyGrammar`] and hanging
//! it off the definer's [`CommandSpec::definition_body`], **never** editing the
//! compiler / analyser / LSP with command-specific `match cmd_name` logic.
//!
//! [`CommandSpec`]: crate::CommandSpec
//! [`CommandSpec::definition_body`]: crate::CommandSpec::definition_body

use crate::arg_role::ArgRole;

/// How a member's argument layout is determined — most members are `Flat`
/// (their `arg_roles` give the layout directly), but two irregular shapes recur
/// across class systems and are described structurally so the walker never
/// hardcodes a member name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    /// Ordinary member — `arg_roles` give the argument layout directly
    /// (`method NAME PARAMS BODY`, `variable v`, …).
    Flat,
    /// A prefix wrapper around an inner member keyword at argument 0: `TclOO`'s
    /// `self method …` and itcl's access modifiers `public`/`protected`/
    /// `private method …`.  The inner member's own roles apply shifted one
    /// place right (past the wrapper word).
    Wrapper,
    /// Flag-keyed bodies rather than positional ones: `TclOO`'s
    /// `property NAME ?-get BODY? ?-set BODY?`.
    FlagKeyed,
}

/// What a member's arguments *refer* to, when they are an unbounded list of
/// references rather than declarations (`superclass A B`, `export m n`).
///
/// Distinct from [`ArgRole`], which describes a declaring position: these
/// arguments name an entity defined elsewhere, so a walker types them as a
/// reference to that entity rather than as a definition of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberRefKind {
    /// Each argument names a class (`superclass A B`, `mixin M`).
    Class,
    /// Each argument names a method (`export m`, `unexport m`, `filter f`).
    Method,
}

/// Which of a retracting member's arguments name members it **removes**.
///
/// `deletemethod a b` removes every name it is given; `renamemethod OLD NEW`
/// removes only `OLD` — `NEW` is the name the member *arrives* under, so a
/// consumer that treated it as retracted would suppress a live member (and,
/// across documents, one another file legitimately declares).  Keeping the
/// shape as registry data means the walker never learns that `renamemethod`'s
/// second word is special by matching the keyword.
///
/// Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
///
/// ```tcl
/// oo::class create ::I3 { method old {} {…} ; renamemethod old new }
/// info class methods ::I3    ;# -> new       (`new` is live, `old` is gone)
/// [::I3 new] new             ;# -> o
/// ::I3 old                   ;# -> unknown method "old"
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberRetraction {
    /// Every argument names a member that is removed (`deletemethod a b`).
    EveryArgument,
    /// Only the first argument is removed; the rest name the result
    /// (`renamemethod OLD NEW`).
    FirstArgument,
}

impl MemberRetraction {
    /// Split a retracting member's argument words into the **retracted** names
    /// and the *0-based argument index* of the name the retracted member
    /// **arrives** under, when the word has one.
    ///
    /// The one place the `deletemethod` / `renamemethod` argument shapes are
    /// spelled out, so neither the member walker nor a cross-document consumer
    /// re-derives them by matching a keyword:
    ///
    /// * [`Self::EveryArgument`] — every word is retracted, nothing arrives
    ///   (`deletemethod a b` removes both `a` and `b`).
    /// * [`Self::FirstArgument`] — word 0 is retracted and word 1 is the
    ///   arrival name (`renamemethod OLD NEW` moves `OLD`'s definition to
    ///   `NEW`; `info class definition ::C NEW` really answers with `OLD`'s
    ///   original parameter list and body, byte-identical on tclsh 9.0.4 and
    ///   8.6.14).
    ///
    /// `arrives_at` is `None` when the call is too short to carry one — a
    /// bare `renamemethod old` is a `wrong # args` error in real Tcl, so
    /// there is no arrival to model.
    #[must_use]
    pub fn split(self, args: &[String]) -> RetractionWords<'_> {
        match self {
            Self::EveryArgument => RetractionWords {
                retracted: args,
                arrives_at: None,
            },
            Self::FirstArgument => RetractionWords {
                retracted: &args[..args.len().min(1)],
                arrives_at: (args.len() > 1).then_some(1),
            },
        }
    }
}

/// The words of one retracting member call, split by [`MemberRetraction::split`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetractionWords<'a> {
    /// The argument words naming members this call **removes**.
    pub retracted: &'a [String],
    /// The 0-based argument index of the name the retracted member arrives
    /// under, or `None` when this member word moves nothing.
    pub arrives_at: Option<usize>,
}

/// One operation on a `TclOO` **slot** — the list-valued definition words
/// (`filter`, `superclass`, `mixin`, `variable`) that are `oo::Slot`
/// instances in real Tcl rather than plain assignments (issue #1169).
///
/// A slot call's first argument may name the operation explicitly
/// (`filter -set x`, `mixin -append M`); a bare word list uses the slot's
/// own default operation.  Oracle for the operation set, C Tcl 9.0.4
/// `tclOODefineCmds.c` (`slotMethods[]`), confirmed live on tclsh 9.0.4:
///
/// ```text
/// unknown method "-bogus": must be -append, -appendifnew, -clear,
/// -prepend, -remove or -set
/// ```
///
/// Tcl 8.6 has only `-set` / `-append` / `-clear` (`tclOO.c`'s embedded
/// slot script: `export -set -append -clear`); `-appendifnew` / `-prepend`
/// / `-remove` are 9.0 additions.  The fold below accepts all six under
/// every dialect — modelling an 8.6 script that uses a 9.0-only operation
/// as the operation it names is strictly closer to what the author meant
/// than treating `-prepend` as a member name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotOp {
    /// `-set` — replace the slot's contents with the arguments.
    Set,
    /// `-append` — add the arguments at the end (no deduplication:
    /// tclsh 9.0.4, `filter a ; filter a b` → `a a b`).
    Append,
    /// `-appendifnew` (9.0+) — add each argument not already present.
    AppendIfNew,
    /// `-prepend` (9.0+) — add the arguments at the front
    /// (tclsh 9.0.4: `filter a ; filter -prepend b` → `b a`).
    Prepend,
    /// `-remove` (9.0+) — remove the named items.
    Remove,
    /// `-clear` — empty the slot (takes no further arguments).
    Clear,
}

impl SlotOp {
    /// Parse an explicit slot-operation word, or `None` when `word` is not
    /// one.  Only the **first** argument of a slot call is ever an
    /// operation — tclsh 9.0.4: `filter a -set b` appends the three literal
    /// items `a`, `-set`, `b`.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "-set" => Some(Self::Set),
            "-append" => Some(Self::Append),
            "-appendifnew" => Some(Self::AppendIfNew),
            "-prepend" => Some(Self::Prepend),
            "-remove" => Some(Self::Remove),
            "-clear" => Some(Self::Clear),
            _ => None,
        }
    }
}

/// The slot behaviour of a list-valued member word: its default operation
/// and whether the slot deduplicates on append (issue #1169).
///
/// Defaults pinned against C Tcl — identical in 9.0.4 (`slots[]` in
/// `tclOODefineCmds.c`) and 8.6.16 (`tclOO.c`'s embedded
/// `--default-operation` forwards) — and confirmed live on tclsh 9.0.4:
///
/// | slot                    | default op | dedup |
/// |-------------------------|------------|-------|
/// | `filter` (both sides)   | `-append`  | no (`filter a; filter a b` → `a a b`) |
/// | `variable` (both sides) | `-append`  | yes (`variable a; variable a b` → `a b`) |
/// | `superclass`            | `-set`     | —     |
/// | `mixin` (both sides)    | `-set`     | —     |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotSpec {
    /// The operation a bare word list applies.
    pub default_op: SlotOp,
    /// Whether appending skips items already present (`variable` does,
    /// `filter` does not — oracle above).
    pub dedup: bool,
}

impl SlotSpec {
    /// Split one slot call's arguments into its effective operation and
    /// value words: an explicit leading operation word wins, a bare list
    /// takes the slot default.  Returns `None` for an unrecognised leading
    /// `-word` — real Tcl aborts the whole definition there (`unknown
    /// method "-bogus"`), so a consumer must not fold anything for it.
    #[must_use]
    pub fn split_call<'a>(&self, args: &'a [String]) -> Option<(SlotOp, &'a [String])> {
        match args.first() {
            Some(first) if first.starts_with('-') => {
                SlotOp::parse(first).map(|op| (op, &args[1..]))
            }
            _ => Some((self.default_op, args)),
        }
    }

    /// Fold one slot call into `current` — the single definition of what
    /// each operation does to the slot's list, shared by every consumer so
    /// the instance `filters`, class-object `class_filters`, `superclass`,
    /// `mixin`, and `variable` channels cannot diverge (issue #1169).
    ///
    /// An unrecognised leading operation word leaves the slot unchanged
    /// (see [`Self::split_call`]).
    pub fn apply(&self, current: &mut Vec<String>, args: &[String]) {
        let Some((op, values)) = self.split_call(args) else {
            return;
        };
        match op {
            SlotOp::Set => {
                current.clear();
                self.extend(current, values);
            }
            SlotOp::Append => self.extend(current, values),
            SlotOp::AppendIfNew => {
                for v in values {
                    if !current.iter().any(|c| c == v) {
                        current.push(v.clone());
                    }
                }
            }
            SlotOp::Prepend => {
                let mut fresh: Vec<String> = Vec::with_capacity(values.len() + current.len());
                self.extend(&mut fresh, values);
                fresh.append(current);
                *current = fresh;
            }
            SlotOp::Remove => current.retain(|c| !values.iter().any(|v| v == c)),
            SlotOp::Clear => current.clear(),
        }
    }

    /// Append `values` honouring the slot's dedup rule.
    fn extend(self, current: &mut Vec<String>, values: &[String]) {
        for v in values {
            if !self.dedup || !current.iter().any(|c| c == v) {
                current.push(v.clone());
            }
        }
    }
}

/// The visibility a member word imposes on the members its arguments name.
///
/// The sibling of [`MemberSpec::retraction`] for the *other* kind
/// of effect a [`MemberRefKind::Method`] member can have on a member it names:
/// `deletemethod` / `renamemethod` remove it, `export` / `unexport` change
/// whether it is callable from outside, and `filter` does neither.  Keeping it
/// as registry data means a consumer never matches `"export"` / `"unexport"` by
/// name, so a definition grammar for another class system can declare an
/// equivalent member word and every consumer picks it up unchanged.
///
/// Oracle for the `TclOO` pair, byte-identical on tclsh 9.0.4 and 8.6.14:
///
/// ```tcl
/// oo::class create ::F1 { method m {} {…} ; unexport m }
/// info class methods ::F1                 ;# -> {}          (not callable)
/// info class methods ::F1 -all -private   ;# -> … m …       (still defined)
/// [::F1 new] m                            ;# -> unknown method "m"
/// oo::class create ::H1 { self { method lower {} {…} ; unexport lower ; export lower } }
/// info object methods ::H1                ;# -> lower       (last writer wins)
/// ```
///
/// Naming a member that does not exist on the side the word is scoped to is a
/// **silent no-op**, not the hard error a retracting word raises: `oo::class
/// create ::E { method onlyinst {} {…} } ; oo::define ::E { self unexport
/// onlyinst }` succeeds and leaves `onlyinst` exported on the instance side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberVisibility {
    /// The named members become callable from outside the object (`export`).
    Exported,
    /// The named members become callable only through `my` (`unexport`).
    Unexported,
}

/// The visibility a definition-time option applies to the member being
/// declared.
///
/// This is distinct from [`MemberVisibility`], which describes a separate
/// `export` / `unexport` member word acting on a member that already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredMemberVisibility {
    /// Callable from outside the object.
    Public,
    /// Callable only in the object's defining class.
    Private,
    /// Defined but not exported for outside dispatch.
    Unexported,
}

impl DeclaredMemberVisibility {
    /// The analyser/storage spelling used by existing class facts.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Unexported => "unexported",
        }
    }
}

/// One accepted spelling of an optional definition-member argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberOptionValue {
    /// Exact Tcl spelling accepted at this position.
    pub value: &'static str,
    /// Semantic role of the option word.
    pub role: ArgRole,
    /// Dialects that accept this spelling, or `None` when it is available in
    /// every dialect that supplies the enclosing member.
    pub dialects: Option<crate::dialects::DialectSet>,
    /// Visibility selected for a member declaration, when this option has one.
    pub declared_visibility: Option<DeclaredMemberVisibility>,
}

/// An optional, closed-vocabulary word embedded in an otherwise fixed member
/// layout.
///
/// `position` is counted in the fixed layout before this optional word is
/// inserted. `position: 0` describes an option prefix plus a fixed tail;
/// `position: 1` describes a fixed name followed by an option and tail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionalMemberArgument {
    /// Number of fixed arguments preceding this optional argument.
    pub position: u8,
    /// Closed vocabulary recognised at this position.
    pub values: &'static [MemberOptionValue],
}

impl OptionalMemberArgument {
    fn value_for<S: AsRef<str>>(self, args: &[S]) -> Option<MemberOptionValue> {
        self.value_for_in(args, crate::dialects::DialectSet::all())
    }

    fn value_for_in<S: AsRef<str>>(
        self,
        args: &[S],
        dialect: crate::dialects::DialectSet,
    ) -> Option<MemberOptionValue> {
        let word = args.get(usize::from(self.position))?.as_ref();
        self.values.iter().copied().find(|candidate| {
            candidate.value == word
                && candidate
                    .dialects
                    .is_none_or(|available| available.intersects(dialect))
        })
    }
}

/// One member sub-keyword of a definition body, with the argument roles a
/// walker should apply to its call.  `arg_roles` indices are 0-based *after*
/// the member keyword itself (`method NAME PARAMS BODY` →
/// `[(0, Name), (1, ParamList), (2, Body)]`).
#[derive(Debug, Clone, Copy)]
pub struct MemberSpec {
    /// The member keyword (`method`, `typemethod`, `constructor`, …).
    pub keyword: &'static str,
    /// Argument roles within the member call, 0-based after the keyword.
    pub arg_roles: &'static [(u8, ArgRole)],
    /// One optional closed-vocabulary argument embedded in the fixed argument
    /// layout, when the member has one. See [`OptionalMemberArgument`].
    pub optional_argument: Option<OptionalMemberArgument>,
    /// When set, *every* argument is a declared variable name (the unbounded
    /// `variable a b c` form).  Overrides `arg_roles` for name collection.
    pub all_args_var: bool,
    /// When set, *every* argument is a **reference** to an entity of this kind
    /// — `superclass A B` names classes, `export m n` names methods.  These
    /// members declare nothing and recurse nothing, but their arguments are not
    /// free strings either; without this they fell through to the default
    /// literal classifier and every `superclass Base` painted as a plain string.
    pub all_args_ref: Option<MemberRefKind>,
    /// The structural shape of the member's arguments (see [`MemberKind`]).
    pub kind: MemberKind,
    /// For a [`MemberKind::Wrapper`], whether the wrapper *also* accepts a bare
    /// script-block form (`private { … }`, `self { … }`) in addition to the
    /// prefix form (`private method m {} {…}`).  `TclOO`'s `private` / `self`
    /// take both; itcl's access modifiers (`public`/`protected`/`private`) only
    /// wrap an inner member.  When the word after the wrapper is not a
    /// recognised inner member and this is set, argument 0 is the member's
    /// [`ArgRole::Body`] script.  Ignored for non-wrapper members.
    pub wrapper_block_body: bool,
    /// The dialects the member keyword is available in, or `None` when it is
    /// version-independent (the common case).  `property` is 9.0+ — it does
    /// not exist in the 8.6 `TclOO` definition grammar — so a document using it
    /// under an older core is flagged rather than silently accepted.
    pub dialects: Option<crate::dialects::DialectSet>,
    /// How this member **removes** the members its arguments name, or `None`
    /// when it merely refers to them.
    ///
    /// Distinguishes `deletemethod m` / `renamemethod old new` from the other
    /// [`MemberRefKind::Method`] members (`export` / `unexport` / `filter`),
    /// which name a method without retracting it.  A consumer that records
    /// members from a definition body must not keep a member some later word
    /// in the same body deletes.  Oracle, identical on tclsh 9.0.4 and 8.6.16:
    ///
    /// ```tcl
    /// oo::class create ::C1 {
    ///     self { method gone {} {…} ; method kept {} {…} ; deletemethod gone }
    /// }
    /// info object methods ::C1   ;# -> kept          (`gone` really is gone)
    /// ::C1 gone                  ;# -> unknown method "gone"
    ///
    /// oo::class create ::C2 { self { method old {} {…} ; renamemethod old new } }
    /// info object methods ::C2   ;# -> new
    /// ::C2 old                   ;# -> unknown method "old"
    /// ```
    ///
    /// Source order is not a consumer concern: naming a member that does not
    /// exist *yet* is a hard error, not a no-op, so the only legal order is
    /// declare-then-retract —
    /// `oo::class create ::C3 { self { deletemethod ghost ; method ghost {} {…} } }`
    /// fails with `method ghost does not exist` and no class is created at all
    /// (same on both interpreters), as does deleting a never-declared name.
    ///
    /// Which arguments a retracting member removes is [`MemberRetraction`]:
    /// `deletemethod` removes every name it is given, `renamemethod` only its
    /// first.
    pub retraction: Option<MemberRetraction>,
    /// The slot behaviour of this member word, or `None` when it is not a
    /// slot (issue #1169).
    ///
    /// A slot member's word list is **not** an assignment: `filter a` then
    /// `filter b` leaves both filters live (`-append` default), while
    /// `superclass` / `mixin` replace (`-set` default), and all four slots
    /// accept explicit `-set` / `-append` / `-prepend` / `-clear` /
    /// `-remove` / `-appendifnew` operation words.  Consumers fold the
    /// call through [`SlotSpec::apply`] instead of overwriting, and skip
    /// the leading operation word when classifying arguments (it names no
    /// class / method / variable).
    pub slot: Option<SlotSpec>,
    /// The visibility this member imposes on the members its arguments name,
    /// or `None` when it has no visibility effect.
    ///
    /// The other half of the "what does this member word do to a method it
    /// names" question [`Self::retraction`] answers: `export` /
    /// `unexport` carry a [`MemberVisibility`], `deletemethod` /
    /// `renamemethod` retract, and `filter` — the third
    /// [`MemberRefKind::Method`] member — does neither, merely referring.  A
    /// consumer that records members reads this instead of matching the
    /// keyword, so the effect travels with the grammar.
    pub visibility_effect: Option<MemberVisibility>,
}

impl MemberSpec {
    /// An ordinary [`MemberKind::Flat`] member.
    #[must_use]
    const fn flat(keyword: &'static str, arg_roles: &'static [(u8, ArgRole)]) -> Self {
        Self {
            keyword,
            arg_roles,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            dialects: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
        }
    }

    /// A member whose every argument references an entity of `kind`
    /// (`superclass A B`, `export m`).
    #[must_use]
    const fn all_refs(keyword: &'static str, kind: MemberRefKind) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: Some(kind),
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            dialects: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
        }
    }

    /// A `variable a b c`-style member: every argument is a declared name.
    #[must_use]
    const fn all_vars(keyword: &'static str) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            optional_argument: None,
            all_args_var: true,
            all_args_ref: None,
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            dialects: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
        }
    }

    /// A name-reference / keyword-only member carrying nothing to recurse or
    /// declare (`superclass A B`, `inherit Base`, `option …`).
    #[must_use]
    const fn keyword_only(keyword: &'static str) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            dialects: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
        }
    }

    /// A [`MemberKind::Wrapper`] member (itcl's `public`/`protected`/`private`)
    /// — an inner member keyword follows at argument 0, and there is no bare
    /// script-block form.
    #[must_use]
    const fn wrapper(keyword: &'static str) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Wrapper,
            wrapper_block_body: false,
            dialects: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
        }
    }

    /// A [`MemberKind::Wrapper`] member that *also* accepts the bare
    /// script-block form — `TclOO`'s `self` and `private`, which are both
    /// `self method …` / `private method …` (prefix) and `self { … }` /
    /// `private { … }` (a definition script evaluated with altered visibility /
    /// target).  When the following word is not an inner member, argument 0 is
    /// the block [`ArgRole::Body`].
    #[must_use]
    const fn wrapper_or_body(keyword: &'static str) -> Self {
        Self {
            keyword,
            arg_roles: BODY0_ROLES,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Wrapper,
            wrapper_block_body: true,
            dialects: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
        }
    }

    /// Restrict this member to `dialects` (a builder over the constructors
    /// above): `property` is 9.0+, so it carries `TCL90_PLUS` while every other
    /// `TclOO` member stays version-independent.
    #[must_use]
    const fn with_dialects(mut self, dialects: crate::dialects::DialectSet) -> Self {
        self.dialects = Some(dialects);
        self
    }

    /// Mark this member as one that **retracts** the members its arguments
    /// name (a builder over the constructors above) — see [`Self::retraction`].
    #[must_use]
    const fn retracting(mut self, retraction: MemberRetraction) -> Self {
        self.retraction = Some(retraction);
        self
    }

    /// Mark this member as one that sets the **visibility** of the members its
    /// arguments name (a builder over the constructors above) — see
    /// [`Self::visibility_effect`].
    #[must_use]
    const fn visibility(mut self, effect: MemberVisibility) -> Self {
        self.visibility_effect = Some(effect);
        self
    }

    /// Mark this member as a `TclOO` **slot** with the given default
    /// operation and dedup rule (a builder over the constructors above) —
    /// see [`Self::slot`] and [`SlotSpec`].
    #[must_use]
    const fn slot_spec(mut self, default_op: SlotOp, dedup: bool) -> Self {
        self.slot = Some(SlotSpec { default_op, dedup });
        self
    }

    /// Insert one optional, closed-vocabulary argument in this member's fixed
    /// layout. Consumers resolve concrete positions through
    /// [`Self::indices_for_call`], so option-bearing members remain registry
    /// data.
    #[must_use]
    const fn optional_argument(mut self, optional: OptionalMemberArgument) -> Self {
        self.optional_argument = Some(optional);
        self
    }

    /// A [`MemberKind::FlagKeyed`] member (`property`).
    #[must_use]
    const fn flag_keyed(keyword: &'static str) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::FlagKeyed,
            wrapper_block_body: false,
            dialects: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
        }
    }

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
            .map(move |(index, _)| {
                let index = usize::from(*index);
                index
                    + usize::from(
                        option.is_some_and(|_| {
                            option_position.is_some_and(|position| index >= position)
                        }),
                    )
            })
            .chain(option_index)
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
        dialect: crate::dialects::DialectSet,
        role: ArgRole,
    ) -> impl Iterator<Item = usize> + '_ {
        let option = self
            .optional_argument
            .and_then(|optional| optional.value_for_in(args, dialect));
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
            .map(move |(index, _)| {
                let index = usize::from(*index);
                index
                    + usize::from(
                        option.is_some_and(|_| {
                            option_position.is_some_and(|position| index >= position)
                        }),
                    )
            })
            .chain(option_index)
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
        dialect: crate::dialects::DialectSet,
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
        dialect: crate::dialects::DialectSet,
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
        dialect: crate::dialects::DialectSet,
    ) -> Option<DeclaredMemberVisibility> {
        self.option_for_in(args, dialect)
            .and_then(|option| option.declared_visibility)
    }
}

/// Which class-system a definer belongs to.  Distinguishes definers that share
/// the `definition_body` marker but need a different analyser body-parser /
/// instance-creation shape (`TclOO`'s `metaclass create Name { … }` vs snit's
/// `snit::type Name { … }`).  Consumers that only walk members (folding,
/// semantic tokens) never read this; the analyser dispatches on it instead of
/// hardcoding definer names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinerFamily {
    /// `TclOO` metaclasses and the `oo::define` / `oo::objdefine` script form.
    TclOo,
    /// snit `type` / `widget` / `widgetadaptor`.
    Snit,
    /// [incr Tcl] `itcl::class` (and the bare `class` alias).
    Itcl,
    /// `SpecTcl` — the `.tclspec` spec-pack DSL's own declaration bodies
    /// (`speclib … { … }`, `command … { … }`, `hover { … }`, …).
    ///
    /// Not a class system: no instances are manufactured and nothing is
    /// dispatched, so every "object" field of the grammar is empty for it.
    /// It shares [`DefinitionBodyGrammar`] because it needs exactly the same
    /// thing the class definers need — a *context-sensitive* member
    /// vocabulary with per-member argument roles — and gets folding,
    /// semantic tokens, and body recursion from the same generic consumers.
    /// Consumers that manufacture instances (the signature scanner, the OO
    /// analyser) must treat this family as claiming nothing.
    SpecTcl,
}

/// The grammar of a definer command's definition body: its recognised member
/// sub-keywords plus the variables implicitly in scope inside every member
/// body.
#[derive(Debug, Clone, Copy)]
pub struct DefinitionBodyGrammar {
    /// The class-system this definer belongs to.
    pub family: DefinerFamily,
    /// Recognised member sub-keywords.
    pub members: &'static [MemberSpec],
    /// Variables implicitly available in every member body (snit's `self` /
    /// `type` / `selfns` / `options`, a widget's `win` / `hull`).  Consumed by
    /// the analyser's read-before-set / stray-dispatch suppression.
    pub implicit_vars: &'static [&'static str],
    /// The **implicit `namespace path`** every member body of this class
    /// system runs with — namespaces a bare command word resolves through
    /// after the body's own current namespace and before global, exactly as
    /// a written `namespace path` would.
    ///
    /// `TclOO` puts `::oo::Helpers` there unconditionally (tclsh 8.6.16 and
    /// 9.0.4: inside any method body `namespace path` is `::oo::Helpers`, so
    /// a bare `callback` reaches `::oo::Helpers::callback` — the documented
    /// "`TclOO` Tricks" idiom — before `::callback`).  snit and itcl member
    /// bodies run in the type / class namespace with no injected path, so
    /// theirs is empty.
    ///
    /// This is the data half of [`crate::Traits::TclooMethodContext`]: that
    /// trait says *which words* only resolve in such a frame, this says
    /// *where* a frame looks.  Consumed by the analyser's invocation
    /// candidate-list finalisation (issue #1137), so no consumer spells
    /// `::oo::Helpers` itself.
    pub member_body_namespace_path: &'static [&'static str],

    /// Type-level members every definer of this family provides without the
    /// body declaring them — snit gives every type an `info` and a `destroy`
    /// typemethod.  `create` is deliberately **not** listed: `Type create
    /// inst` is a construction, not a typemethod call, and a consumer
    /// distinguishing the two must keep them apart.  Empty for families with
    /// no such built-ins.
    pub builtin_type_methods: &'static [&'static str],

    /// **Instance**-level methods every object of this family answers to
    /// without its class body declaring them — `TclOO`'s inherited
    /// `oo::object` surface (`destroy`, plus the unexported `eval` /
    /// `variable` / `varname` / `unknown` / `<cloned>`), snit's
    /// `configure` / `cget` / `info`, itcl's `configure` / `cget` / `isa`.
    ///
    /// The instance-side twin of [`Self::builtin_type_methods`], and the
    /// registry answer to "is this method word actually missing, or is it
    /// one the class system supplies?" — the question a consumer emitting an
    /// unknown-method diagnostic has to settle before it fires.  Each entry
    /// carries its own [`MemberVisibility`], because that decides *which
    /// dispatch spellings can reach it*: an unexported builtin is reachable
    /// only through the family's self-dispatch keyword (`my variable v`
    /// works; `$obj variable v` and `[self] variable v` both fail with
    /// `unknown method "variable"` — pinned against tclsh 9.0.4 and
    /// 8.6.16).  Ask through [`Self::builtin_object_method`] rather than
    /// scanning this directly, so that reachability rule stays in one place.
    ///
    /// Empty for a family with no such built-ins.
    pub builtin_object_methods: &'static [BuiltinObjectMethod],

    /// Built-in object methods that cannot complete normally. Consumers use
    /// this only after proving that method dispatch reaches the built-in
    /// implementation; a user override is governed by its own body.
    pub builtin_terminating_methods: &'static [&'static str],

    /// Commands this class system makes available **inside every member
    /// body** and nowhere else — snit's `install NAME using TYPE …`.
    ///
    /// The command counterpart of [`Self::implicit_vars`], and deliberately
    /// *not* a global [`crate::spec::CommandSpec`]: `install` is a snit
    /// instance-namespace alias, so registering it globally would make a
    /// user's own `proc install` look like a built-in everywhere.  A consumer
    /// resolves the layout facts (today: [`MemberBodyCommand::binds_handle`])
    /// from here, so no walker spells the keyword — issue #1185.  Empty for a
    /// family that injects no such commands.
    pub member_body_commands: &'static [MemberBodyCommand],

    /// Whether invoking this family's **type command with a bare instance
    /// name** constructs an instance — snit's `$type $name ?options…?`
    /// shorthand for `$type create $name`, documented in snit(n)'s "The
    /// Type Command" ("if the first argument is not a type method name, it
    /// is assumed to be an instance name and `create` is implied").
    ///
    /// `false` for `TclOO` and [incr Tcl], whose class commands only
    /// construct through an explicit `create` / `new` method.  A consumer
    /// deciding whether `set w [Foo $win.a]` binds an object handle reads
    /// this rather than testing the metaclass name for a `snit::` prefix.
    pub bare_word_construction: bool,

    /// Conservative recogniser for conventional bare instance-name forms
    /// when a consumer knows a class name but does not have that class's
    /// family record. The callback belongs beside
    /// [`Self::bare_word_construction`], so low-level type inference does not
    /// embed snit naming conventions. `None` for families without the
    /// shorthand.
    pub bare_word_construction_hint: Option<fn(&str) -> bool>,

    /// Whether a class body can install methods whose names are not
    /// statically enumerable from its member declarations.  snit's wildcard
    /// delegation does this; `TclOO` and [incr Tcl] do not unless the analyser
    /// separately observes reflective code.  Unknown-method diagnostics must
    /// abstain when this is true.
    pub dynamic_method_dispatch: bool,

    /// The methods of this family's **class command** that manufacture an
    /// instance — `TclOO`'s `create` / `new` / `createWithNamespace`.
    ///
    /// The registry half of "is `X create Name Body` a class creation?".  The
    /// analyser used to carry those three keywords as a literal `matches!`,
    /// so a family manufacturing under a different word could not be added
    /// without editing the walker (issue #1303).  Empty for a family whose
    /// class command manufactures only through
    /// [`Self::bare_word_construction`] or a
    /// [`CommandSpec::creates_instance_at`](crate::spec::CommandSpec::creates_instance_at)
    /// spec.
    pub manufacturers: &'static [ManufacturerMethod],

    /// The method an object of this class system dispatches an **unrecognised
    /// first word** to — `TclOO`'s `unknown` (`object.n`: "if the method is
    /// not found … the `unknown` method is invoked, with the name of the
    /// method as its first argument").
    ///
    /// Declaring one says only that an unrecognised word still reaches code.
    /// Whether such a call *constructs*, and whether its result is the new
    /// object's name, is **not** implied and must be proved from the body —
    /// see `ClassFactory::unknown_binds_instance` in the analyser, which
    /// abstains whenever it cannot.
    ///
    /// `None` for snit and [incr Tcl].  snit's type command does treat an
    /// unrecognised *first* word as an instance name, but that is
    /// [`Self::bare_word_construction`] — a documented property of the type
    /// command, not a user-written fallback member.
    pub unknown_dispatch_method: Option<&'static str>,

    /// The instance methods this definer **generates** from the class's
    /// declared `property` members — Tcl 9.0's `oo::configurable`, whose
    /// instances answer `configure` (`configurable.n`).
    ///
    /// Distinct from [`Self::builtin_object_methods`] in *who* supplies them:
    /// those come from the family's root object and are on every instance of
    /// the family, while these come from one specific metaclass and are only
    /// on the classes it manufactures. Every `TclOO` metaclass shares this
    /// one grammar, so `oo::configurable` carries its own copy
    /// ([`TCLOO_CONFIGURABLE_GRAMMAR`]) rather than handing `configure` to
    /// plain `oo::class` instances, which really do fail with `unknown
    /// method "configure"`.
    ///
    /// **Only `configure`.** `cget` is *not* generated: `configurable.n` for
    /// both 9.0 and 9.1 documents `configure` and the internal
    /// `<ReadProp-name>` / `<WriteProp-name>` accessors and no `cget` at all,
    /// and this workspace's own VM implements the same single method
    /// (`tcl-vm/src/cmd_oo.rs`). A `cget` on a configurable object is a
    /// genuine unknown-method error, so a consumer must not accept it (issue
    /// #1362). snit and [incr Tcl] *do* give every instance `configure` /
    /// `cget`, but those are family built-ins and live in
    /// [`Self::builtin_object_methods`], not here.
    ///
    /// Empty for every family that generates no such methods.
    pub property_accessor_methods: &'static [&'static str],
}

/// One method of a class system's **class command** that manufactures an
/// instance — see [`DefinitionBodyGrammar::manufacturers`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManufacturerMethod {
    /// The method word as invoked on the class command (`create`).
    pub keyword: &'static str,
    /// Whether ordinary dispatch through the class command can reach this
    /// method without an explicit export. `createWithNamespace` is a real
    /// `TclOO` method but is unexported in C Tcl 9.0.4 and 8.6.
    pub visibility: MemberVisibility,
    /// Index — among the call's arguments, argument 0 being the manufacturer
    /// keyword itself — of the word naming the new instance, or `None` when
    /// the manufacturer generates the name itself (`new`).
    pub names_instance_at: Option<u8>,
    /// Index — in the same argument coordinate system — of the definition
    /// body for a class-manufacturing call. `None` when this method creates
    /// an ordinary instance and has no class-definition body.
    pub definition_body_at: Option<u8>,
    /// Index of the first argument passed to the newly created object's
    /// constructor. This keeps constructor parameter flow independent of the
    /// spelling and of structural words such as an explicit object name or
    /// namespace name.
    pub constructor_args_from: u8,
}

/// One method every instance of a class system's objects has without the
/// class body declaring it — see
/// [`DefinitionBodyGrammar::builtin_object_methods`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinObjectMethod {
    /// The method name as dispatched (`variable`, `destroy`, `cget`).
    pub name: &'static str,
    /// Whether the object's own command exposes it.  [`MemberVisibility::
    /// Unexported`] means "reachable only through the family's self-dispatch
    /// keyword" — the same rule an `unexport`ed user method obeys.
    pub visibility: MemberVisibility,
    /// Which receiver actually carries the method.
    pub receiver: BuiltinMethodReceiver,
    /// One-line description, for hover / completion.
    pub detail: &'static str,
}

/// Which kind of object a [`BuiltinObjectMethod`] lives on.
///
/// A class command is itself an object, so the two sets overlap but are not
/// the same: `Foo new` works on the class and never on one of its instances
/// (tclsh 9.0.4: `[Foo new] new` → `unknown method "new"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinMethodReceiver {
    /// Present on every object of the family, instances included.
    AnyObject,
    /// Present only on a **class** object — `TclOO`'s `new` / `create` /
    /// `createWithNamespace`, snit's `create`.
    ClassObject,
}

/// How a dispatch reaches its receiver — the axis that decides whether an
/// unexported [`BuiltinObjectMethod`] is visible at a call site.
///
/// Pinned against tclsh 9.0.4 and 8.6.16 from inside a method body of a
/// class declaring only `probe`:
///
/// ```text
/// my varname v        -> ::oo::Obj22::v      (unexported: reachable)
/// [self] varname v    -> unknown method "varname": must be destroy or probe
/// $obj varname v      -> unknown method "varname": must be destroy or probe
/// ```
///
/// `[self]` substitutes to the object's *command*, so it reaches exactly
/// what an outside caller reaches — it is [`Self::ObjectCommand`], not
/// [`Self::SelfDispatch`], despite naming the same object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodReach {
    /// Through the object's own command — `$obj m`, `[self] m`, or a
    /// bareword instance command bound by `CLASS create NAME`.  Exported
    /// members only.
    ObjectCommand,
    /// Through the family's self-dispatch keyword (`my m`), which bypasses
    /// export filtering.  Exported *and* unexported members.
    SelfDispatch,
}

/// One command a class system injects into every member body — see
/// [`DefinitionBodyGrammar::member_body_commands`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberBodyCommand {
    /// The command word as invoked inside a member body (`install`).
    pub name: &'static str,
    /// One-line description, for hover / completion.
    pub detail: &'static str,
    /// How the call binds a variable to an object handle, when it does —
    /// `install NAME using TYPE …` makes `NAME` a handle of class `TYPE`.
    /// `None` for an injected command that binds no handle.
    pub binds_handle: Option<crate::handle_binding::HandleBindingSpec>,
}

impl DefinitionBodyGrammar {
    /// The member grammar for `keyword`, if it is a recognised member.
    #[must_use]
    pub fn member(&self, keyword: &str) -> Option<&'static MemberSpec> {
        // `members` is `&'static`, so the borrow can be handed back as static.
        let idx = self.members.iter().position(|m| m.keyword == keyword)?;
        Some(&self.members[idx])
    }

    /// Body argument indices for a concrete definition-member invocation.
    ///
    /// Handles flat members, prefix wrappers, wrapper block forms, and
    /// flag-keyed getter/setter bodies from this grammar's structural data.
    #[must_use]
    pub fn member_body_indices_in(
        &self,
        keyword: &str,
        args: &[&str],
        dialect: crate::dialects::DialectSet,
    ) -> Vec<usize> {
        let Some(member) = self.member(keyword) else {
            return Vec::new();
        };
        if member.unavailable_option_for(args, dialect).is_some() {
            return Vec::new();
        }
        match member.kind {
            MemberKind::Flat => member
                .indices_for_call_in(args, dialect, ArgRole::Body)
                .filter(|&index| index < args.len())
                .collect(),
            MemberKind::Wrapper => {
                let Some((inner, rest)) = args.split_first() else {
                    return Vec::new();
                };
                if self.member(inner).is_some() {
                    self.member_body_indices_in(inner, rest, dialect)
                        .into_iter()
                        .map(|index| index + 1)
                        .collect()
                } else if member.wrapper_block_body {
                    member
                        .indices_for_call_in(args, dialect, ArgRole::Body)
                        .filter(|&index| index < args.len())
                        .collect()
                } else {
                    Vec::new()
                }
            }
            MemberKind::FlagKeyed => args
                .iter()
                .enumerate()
                .take(args.len().saturating_sub(1))
                .filter_map(|(index, word)| matches!(*word, "-get" | "-set").then_some(index + 1))
                .collect(),
        }
    }

    /// Whether `name` is a type-level member this family provides without
    /// the body declaring it (see [`Self::builtin_type_methods`]).
    #[must_use]
    pub fn is_builtin_type_method(&self, name: &str) -> bool {
        self.builtin_type_methods.contains(&name)
    }

    /// The built-in object method `name` names, if this family supplies one
    /// **that a `reach` dispatch can actually call** (see
    /// [`Self::builtin_object_methods`]).
    ///
    /// The whole point of routing through here rather than scanning the
    /// slice is the visibility rule: an unexported builtin answers only for
    /// [`MethodReach::SelfDispatch`], so a consumer cannot accidentally
    /// vouch for `$obj variable v`, which really is an error.
    ///
    /// [`BuiltinMethodReceiver`] is deliberately **not** filtered here —
    /// the caller decides whether it holds a class object or an instance,
    /// and a consumer that cannot tell (a `$var` whose value may be either a
    /// class command or one of its instances) must accept both rather than
    /// guess.  Read [`BuiltinObjectMethod::receiver`] off the answer when
    /// that distinction is available.
    #[must_use]
    pub fn builtin_object_method(
        &self,
        name: &str,
        reach: MethodReach,
    ) -> Option<&'static BuiltinObjectMethod> {
        let idx = self.builtin_object_methods.iter().position(|m| {
            m.name == name
                && (m.visibility == MemberVisibility::Exported
                    || reach == MethodReach::SelfDispatch)
        })?;
        Some(&self.builtin_object_methods[idx])
    }

    /// Whether the family's built-in implementation always terminates with a
    /// non-normal completion.
    #[must_use]
    pub fn builtin_method_terminates(&self, name: &str) -> bool {
        self.builtin_terminating_methods.contains(&name)
    }

    /// The member-body command `name` names, if this family injects one (see
    /// [`Self::member_body_commands`]).
    #[must_use]
    pub fn member_body_command(&self, name: &str) -> Option<&'static MemberBodyCommand> {
        let idx = self
            .member_body_commands
            .iter()
            .position(|c| c.name == name)?;
        Some(&self.member_body_commands[idx])
    }

    /// The manufacturer method `keyword` names, if this family's class
    /// command has one (see [`Self::manufacturers`]).
    #[must_use]
    pub fn manufacturer(&self, keyword: &str) -> Option<&'static ManufacturerMethod> {
        let idx = self
            .manufacturers
            .iter()
            .position(|m| m.keyword == keyword)?;
        Some(&self.manufacturers[idx])
    }

    /// Whether `keyword` is a recognised member sub-keyword.
    #[must_use]
    pub fn is_member(&self, keyword: &str) -> bool {
        self.members.iter().any(|m| m.keyword == keyword)
    }

    /// Whether a member named `name` is **exported by default** under this
    /// family's visibility model — before any explicit `export` /
    /// `unexport`, which a consumer applies on top (and which a later
    /// re-`method` definition *resets* back to this default; pinned
    /// against tclsh 9.0.4).
    ///
    /// `TclOO`'s C rule is `Tcl_StringMatch(name, PUBLIC_PATTERN)` with
    /// `PUBLIC_PATTERN "[a-z]*"` (`tclOODefineCmds.c`, 9.0.4): exported
    /// iff the first character is an ASCII lowercase letter — `Upper`,
    /// `_under`, `9digit`, and non-ASCII initials (`ümlaut`) are all
    /// unexported by default.  snit and itcl members are dispatched by
    /// their own access models (itcl's modifier wrappers carry the
    /// visibility explicitly), so they default to exported here.
    #[must_use]
    pub fn member_default_exported(&self, name: &str) -> bool {
        match self.family {
            DefinerFamily::TclOo => name.starts_with(|c: char| c.is_ascii_lowercase()),
            // SpecTcl declares no members that are ever *dispatched*, so the
            // question is vacuous for it; answering `true` keeps the visible
            // set equal to the declared set, which is the only reading of
            // "exported" a declaration-only family has.
            DefinerFamily::Snit | DefinerFamily::Itcl | DefinerFamily::SpecTcl => true,
        }
    }
}

// ---------------------------------------------------------------------------
// TclOO — `oo::class` / `oo::configurable` / `oo::abstract` / `oo::singleton`
// `create` bodies and the bare `oo::define` / `oo::objdefine` script form.
//
// The irregular `self …` (nested member) and `property … -get/-set …`
// (flag-keyed bodies) forms are handled by the walker directly; every flat
// member is described here.
// ---------------------------------------------------------------------------

/// `method NAME PARAMS BODY` — shared by `TclOO` and snit.
const METHOD_ROLES: &[(u8, ArgRole)] = &[
    (0, ArgRole::Name),
    (1, ArgRole::ParamList),
    (2, ArgRole::Body),
];
/// Tcl 9's `method NAME ?EXPORT-FLAG? PARAMS BODY` declaration flags.
///
/// These are intentionally a closed registry vocabulary rather than arbitrary
/// `-word` parsing: Tcl rejects every other spelling at this position.
const TCLOO_METHOD_VISIBILITY_OPTIONS: &[MemberOptionValue] = &[
    MemberOptionValue {
        value: "-export",
        role: ArgRole::Option,
        dialects: Some(TCL90_MEMBERS),
        declared_visibility: Some(DeclaredMemberVisibility::Public),
    },
    MemberOptionValue {
        value: "-private",
        role: ArgRole::Option,
        dialects: Some(TCL90_MEMBERS),
        declared_visibility: Some(DeclaredMemberVisibility::Private),
    },
    MemberOptionValue {
        value: "-unexport",
        role: ArgRole::Option,
        dialects: Some(TCL90_MEMBERS),
        declared_visibility: Some(DeclaredMemberVisibility::Unexported),
    },
];
const TCLOO_METHOD_VISIBILITY_ARGUMENT: OptionalMemberArgument = OptionalMemberArgument {
    // The method name is fixed; the optional flag appears before params/body.
    position: 1,
    values: TCLOO_METHOD_VISIBILITY_OPTIONS,
};
/// Tcl 9's `definitionnamespace ?-class|-instance? NAMESPACE` selector.
const TCLOO_DEFINITION_NAMESPACE_FACETS: &[MemberOptionValue] = &[
    MemberOptionValue {
        value: "-class",
        role: ArgRole::Option,
        dialects: Some(TCL90_MEMBERS),
        declared_visibility: None,
    },
    MemberOptionValue {
        value: "-instance",
        role: ArgRole::Option,
        dialects: Some(TCL90_MEMBERS),
        declared_visibility: None,
    },
];
const TCLOO_DEFINITION_NAMESPACE_ARGUMENT: OptionalMemberArgument = OptionalMemberArgument {
    // The facet is optional; the namespace is always the final fixed tail.
    position: 0,
    values: TCLOO_DEFINITION_NAMESPACE_FACETS,
};
const DEFINITION_NAMESPACE_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::NamespaceName)];
/// `constructor PARAMS BODY`.
const CTOR_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::ParamList), (1, ArgRole::Body)];
/// A single trailing body (`destructor BODY`, `typeconstructor BODY`, …).
const BODY0_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Body)];
/// `forward NAME TARGET ?arg…?` — the method name at 0, then the delegated
/// command name at 1 (a command reference the walker follows for navigation).
const FORWARD_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Name), (1, ArgRole::CommandName)];
/// A single declared variable name (`typevariable v`, `component c`).
const VAR0_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::VarWrite)];
/// A member keyword that carries no recursable body / parameter list /
/// variable declaration — only a class/method name reference the walker leaves
/// to the default classifier (`superclass A B`, `mixin M`, `export foo`, …).
const NO_ROLES: &[(u8, ArgRole)] = &[];

/// The `TclOO` definition members Tcl 9.0 added (TIP 478 `classmethod` /
/// `initialise` / `initialize` / `private`, TIP 524 `definitionnamespace`).
/// None of them exists in the 8.6 grammar — confirmed live: each of the
/// three call shapes (a member inside `oo::class create`'s body, inside an
/// `oo::define` block, and the single-command `oo::define Cls classmethod …`
/// form) fails on tclsh8.6 with `invalid command name "<member>"` and
/// succeeds on tclsh9.0.
const TCL90_MEMBERS: crate::dialects::DialectSet = crate::dialects::DialectSet::TCL90_PLUS;

const TCLOO_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES).optional_argument(TCLOO_METHOD_VISIBILITY_ARGUMENT),
    MemberSpec::flat("classmethod", METHOD_ROLES).with_dialects(TCL90_MEMBERS),
    MemberSpec::flat("constructor", CTOR_ROLES),
    MemberSpec::flat("destructor", BODY0_ROLES),
    MemberSpec::flat("initialise", BODY0_ROLES).with_dialects(TCL90_MEMBERS),
    MemberSpec::flat("initialize", BODY0_ROLES).with_dialects(TCL90_MEMBERS),
    // `private` is a prefix wrapper (`private method m {} {…}`, `private
    // variable x`) *and* a bare definition-script block (`private { … }`).
    MemberSpec::wrapper_or_body("private").with_dialects(TCL90_MEMBERS),
    // `variable a b c` inside a class body declares every name.  A slot:
    // `-append` default like `filter`, but deduplicating (tclsh 9.0.4:
    // `variable a ; variable a b` → `a b`).
    MemberSpec::all_vars("variable").slot_spec(SlotOp::Append, true),
    // Reference-only members: they declare nothing and recurse nothing, but
    // their arguments *name* an entity defined elsewhere — a class or a method
    // — so they are references, not free strings.  All three are slots
    // (issue #1169); the defaults are pinned against C Tcl (`slots[]` in
    // 9.0.4's tclOODefineCmds.c, the `--default-operation` forwards in
    // 8.6.16's tclOO.c — identical): `superclass` / `mixin` replace,
    // `filter` appends.
    MemberSpec::all_refs("superclass", MemberRefKind::Class).slot_spec(SlotOp::Set, false),
    MemberSpec::all_refs("mixin", MemberRefKind::Class).slot_spec(SlotOp::Set, false),
    MemberSpec::all_refs("filter", MemberRefKind::Method).slot_spec(SlotOp::Append, false),
    MemberSpec::all_refs("export", MemberRefKind::Method).visibility(MemberVisibility::Exported),
    MemberSpec::all_refs("unexport", MemberRefKind::Method)
        .visibility(MemberVisibility::Unexported),
    MemberSpec::all_refs("deletemethod", MemberRefKind::Method)
        .retracting(MemberRetraction::EveryArgument),
    // `forward NAME cmd ?arg…?` declares NAME as a method; the word after it
    // (`cmd`) is the delegated command's name — a first-class command
    // reference the walker records so navigation reaches it, exactly like the
    // command a `superclass`/`mixin` names.  Any baked arguments after it are
    // ordinary values.
    MemberSpec::flat("forward", FORWARD_ROLES),
    // `renamemethod FROM TO` — both name methods.
    // Both words name methods; the FROM word is retracted (and the TO word is
    // a member this walker does not record), so the whole call retracts.
    MemberSpec::all_refs("renamemethod", MemberRefKind::Method)
        .retracting(MemberRetraction::FirstArgument),
    MemberSpec::flat("definitionnamespace", DEFINITION_NAMESPACE_ROLES)
        .optional_argument(TCLOO_DEFINITION_NAMESPACE_ARGUMENT)
        .with_dialects(TCL90_MEMBERS),
    // Structurally irregular — a nested-member wrapper (`self method …`) and a
    // flag-keyed body form (`property … -get/-set …`); their body indices come
    // from the walker's `MemberKind`-driven handling, not a hardcoded name.
    MemberSpec::wrapper_or_body("self"),
    // `property` (and its configurable-class accessor machinery) is a 9.0
    // addition; the 8.6 `TclOO` definition grammar has no such member.
    MemberSpec::flag_keyed("property").with_dialects(TCL90_MEMBERS),
];

/// The methods every `TclOO` object inherits from `oo::object` (plus the
/// three a *class* object additionally gets from `oo::class`), which no
/// class body declares.
///
/// Pinned against tclsh 9.0.4 **and** 8.6.16 — the sets are byte-identical
/// on both:
///
/// ```text
/// info class methods ::oo::object -all -private  ->  <cloned> destroy eval unknown variable varname
/// info class methods ::oo::class  -all -private  ->  <cloned> create createWithNamespace destroy
///                                                    eval new unknown variable varname
/// ```
///
/// Only `destroy` is exported: `TclOO`'s export test is
/// `Tcl_StringMatch(name, "[a-z]*")` applied at *declaration*, and the rest
/// are explicitly unexported by the core.  `<cloned>` fails the pattern on
/// its leading `<` as well.  That is why every one of them except `destroy`
/// is reachable through `my` and through nothing else — the exact reason
/// `my variable v` must never draw an unknown-method diagnostic while
/// `$obj variable v` legitimately does.
///
/// `configure` / `cget` are **not** listed: they come from `oo::configurable`
/// (Tcl 9.0+) and are declared as real class members on the classes that
/// use it, so they arrive through the ordinary member tables rather than
/// here.
const TCLOO_BUILTIN_OBJECT_METHODS: &[BuiltinObjectMethod] = &[
    BuiltinObjectMethod {
        name: "destroy",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "destroy the object, running its destructors",
    },
    BuiltinObjectMethod {
        name: "eval",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "evaluate a script in the object's own namespace",
    },
    BuiltinObjectMethod {
        name: "unknown",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "handler invoked for an unresolved method name",
    },
    BuiltinObjectMethod {
        name: "variable",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "link object-instance variables into the calling scope",
    },
    BuiltinObjectMethod {
        name: "varname",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "fully-qualified name of an object-instance variable",
    },
    BuiltinObjectMethod {
        name: "<cloned>",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "copy hook invoked by oo::copy on the new object",
    },
    BuiltinObjectMethod {
        name: "new",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::ClassObject,
        detail: "construct an instance with a generated name",
    },
    BuiltinObjectMethod {
        name: "create",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::ClassObject,
        detail: "construct an instance with the given name",
    },
    BuiltinObjectMethod {
        name: "createWithNamespace",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::ClassObject,
        detail: "construct an instance in a named namespace",
    },
];

/// The definition-body grammar for every `TclOO` metaclass and the bare
/// `oo::define` / `oo::objdefine` script form.
pub const TCLOO_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::TclOo,
    members: TCLOO_MEMBERS,
    implicit_vars: &[],
    member_body_namespace_path: TCLOO_MEMBER_BODY_NAMESPACE_PATH,
    // The class command's own built-in, non-manufacturing surface. tclsh
    // 9.0.4 and 8.6.16 agree byte for byte:
    //   oo::class create C {}
    //   info object methods ::C -all   ->  create destroy new
    // `create` / `new` are manufacturers and live in `manufacturers`; what
    // is left is `destroy`. The definition surface proper
    // (`oo::define`/`oo::objdefine`) is a separate command, not a method
    // here.
    builtin_type_methods: &["destroy"],
    builtin_object_methods: TCLOO_BUILTIN_OBJECT_METHODS,
    builtin_terminating_methods: &["unknown"],
    // TclOO injects no extra *commands* into a method body — its helpers
    // (`my` / `next` / `self` / `link` / `classvariable`) are real global
    // commands in `::oo::Helpers` with their own specs, reached through
    // `member_body_namespace_path` above.
    member_body_commands: &[],
    // `Foo $win.a` is not a construction in TclOO: a class command only
    // constructs through `create` / `new` (tclsh 9.0.4: `::C x` →
    // `unknown method "x"`).
    bare_word_construction: false,
    bare_word_construction_hint: None,
    dynamic_method_dispatch: false,
    manufacturers: TCLOO_MANUFACTURERS,
    // `object.n`: an unrecognised method name is dispatched to `unknown`.
    // This is what makes `::C x` above *reachable* rather than fatal once a
    // class (or its metaclass) declares one.
    unknown_dispatch_method: Some("unknown"),
    // Plain `oo::class` / `oo::abstract` / `oo::singleton` instances generate
    // nothing from properties — only `oo::configurable` does, and it carries
    // its own grammar below.
    property_accessor_methods: &[],
};

/// The definition-body grammar for `oo::configurable` — [`TCLOO_GRAMMAR`]
/// plus the one method that metaclass generates for a class's declared
/// `property` members.
///
/// A separate constant precisely because the definition *body* grammar is
/// identical (`property` is a member of `TCLOO_MEMBERS` already): what
/// differs is the instance surface the manufactured class ends up with.
/// Attaching this to the shared grammar would have told every consumer that
/// an `oo::class` instance answers `configure`, which it does not.
pub const TCLOO_CONFIGURABLE_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    // `configurable.n` (9.0.4 and 9.1b0, byte-identical but for the version
    // banner): "making a `configure` method available within the instances".
    // No `cget` — see `property_accessor_methods`' own note.
    property_accessor_methods: &["configure"],
    ..TCLOO_GRAMMAR
};

/// `TclOO`'s class-command manufacturers — see
/// [`DefinitionBodyGrammar::manufacturers`].
///
/// `class.n` (9.0.4): `create name ?definition?`, `new ?definition?`,
/// `createWithNamespace name nsName ?definition?`.  Only `create` and
/// `createWithNamespace` name the instance; `new` generates the name.
pub const TCLOO_CREATE_MANUFACTURER: ManufacturerMethod = ManufacturerMethod {
    keyword: "create",
    visibility: MemberVisibility::Exported,
    names_instance_at: Some(1),
    definition_body_at: Some(2),
    constructor_args_from: 2,
};
/// `new ?definition?`: manufacture with an automatically generated name.
pub const TCLOO_NEW_MANUFACTURER: ManufacturerMethod = ManufacturerMethod {
    keyword: "new",
    visibility: MemberVisibility::Exported,
    names_instance_at: None,
    definition_body_at: Some(1),
    constructor_args_from: 1,
};
/// `new` as inherited by the root `::oo::class` object. The method exists for
/// self-dispatch and can be exported reflectively, but ordinary external
/// dispatch cannot reach it in C Tcl 8.6 or 9.0.
pub const TCLOO_ROOT_NEW_MANUFACTURER: ManufacturerMethod = ManufacturerMethod {
    visibility: MemberVisibility::Unexported,
    ..TCLOO_NEW_MANUFACTURER
};
/// `createWithNamespace name nsName ?definition?`: the unexported
/// namespace-selecting manufacturer.
pub const TCLOO_CREATE_WITH_NAMESPACE_MANUFACTURER: ManufacturerMethod = ManufacturerMethod {
    keyword: "createWithNamespace",
    visibility: MemberVisibility::Unexported,
    names_instance_at: Some(1),
    definition_body_at: Some(3),
    constructor_args_from: 3,
};
const TCLOO_MANUFACTURERS: &[ManufacturerMethod] = &[
    TCLOO_CREATE_MANUFACTURER,
    TCLOO_NEW_MANUFACTURER,
    TCLOO_CREATE_WITH_NAMESPACE_MANUFACTURER,
];

/// Manufacturer surface present on `oo::class` itself. C Tcl 9.0.4 and 8.6
/// expose only `create`; `new` and `createWithNamespace` remain present but
/// private (`info object methods ::oo::class -all -private`).
pub const TCLOO_ROOT_CLASS_MANUFACTURERS: &[ManufacturerMethod] = &[
    TCLOO_CREATE_MANUFACTURER,
    TCLOO_ROOT_NEW_MANUFACTURER,
    TCLOO_CREATE_WITH_NAMESPACE_MANUFACTURER,
];

/// Manufacturer surface exported by the other Tcl 9 metaclass commands.
/// `createWithNamespace` remains unexported on all of them.
pub const TCLOO_DERIVED_METACLASS_MANUFACTURERS: &[ManufacturerMethod] = TCLOO_MANUFACTURERS;

/// snit's single class-command manufacturer — snit(n), "The Type Command":
/// `$type create name ?option value…?`.  There is no `new`: a snit instance
/// is always named, either explicitly or through the `%AUTO%` substitution
/// in the name itself.
const SNIT_MANUFACTURERS: &[ManufacturerMethod] = &[ManufacturerMethod {
    keyword: "create",
    visibility: MemberVisibility::Exported,
    names_instance_at: Some(1),
    definition_body_at: None,
    constructor_args_from: 2,
}];

/// The `namespace path` a `TclOO` member body runs with — see
/// [`DefinitionBodyGrammar::member_body_namespace_path`].
///
/// Exposed on its own so a consumer holding only the analyser's
/// "this frame resolves like a `TclOO` method body" flag (rather than the
/// whole grammar) still reads the path from registry data.
pub const TCLOO_MEMBER_BODY_NAMESPACE_PATH: &[&str] = &["::oo::Helpers"];

// ---------------------------------------------------------------------------
// snit — `snit::type` / `snit::widget` / `snit::widgetadaptor` bodies.
// ---------------------------------------------------------------------------

/// `onconfigure -option valueVar BODY` (snit 1.x) — the value var + body.
const ONCONFIGURE_ROLES: &[(u8, ArgRole)] = &[(1, ArgRole::VarWrite), (2, ArgRole::Body)];
/// `oncget -option BODY` (snit 1.x) — the body.
const ONCGET_ROLES: &[(u8, ArgRole)] = &[(1, ArgRole::Body)];

const SNIT_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES),
    MemberSpec::flat("typemethod", METHOD_ROLES),
    // A type-private `proc NAME ARGS BODY` — same shape as a method.
    MemberSpec::flat("proc", METHOD_ROLES),
    MemberSpec::flat("constructor", CTOR_ROLES),
    MemberSpec::flat("destructor", BODY0_ROLES),
    MemberSpec::flat("typeconstructor", BODY0_ROLES),
    MemberSpec::flat("onconfigure", ONCONFIGURE_ROLES),
    MemberSpec::flat("oncget", ONCGET_ROLES),
    MemberSpec::flat("variable", VAR0_ROLES),
    MemberSpec::flat("typevariable", VAR0_ROLES),
    MemberSpec::flat("component", VAR0_ROLES),
    MemberSpec::flat("typecomponent", VAR0_ROLES),
    // Name-reference / option-declaration members — recognised keywords with
    // nothing to recurse or declare.
    MemberSpec::keyword_only("option"),
    MemberSpec::keyword_only("delegate"),
    MemberSpec::keyword_only("expose"),
];

/// The methods every snit **instance** answers to without its type body
/// declaring them.
///
/// From snit(n), "The Instance Command": every instance supports `configure`,
/// `configurelist`, `cget`, `destroy` and `info` in addition to its declared
/// methods.  snit generates an ordinary Tcl dispatcher proc rather than using
/// `TclOO`'s export machinery, so it has no unexported tier — every one is
/// reachable through the instance command.  `create` belongs to the *type*
/// command, so it is marked [`BuiltinMethodReceiver::ClassObject`]; snit has
/// no `new` at all.
const SNIT_BUILTIN_OBJECT_METHODS: &[BuiltinObjectMethod] = &[
    BuiltinObjectMethod {
        name: "configure",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "set one or more of the instance's options",
    },
    BuiltinObjectMethod {
        name: "configurelist",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "set options from an option/value list",
    },
    BuiltinObjectMethod {
        name: "cget",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "retrieve the value of one of the instance's options",
    },
    BuiltinObjectMethod {
        name: "destroy",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "destroy the instance",
    },
    BuiltinObjectMethod {
        name: "info",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "introspect the instance's type, options and components",
    },
    BuiltinObjectMethod {
        name: "create",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::ClassObject,
        detail: "construct an instance with the given name",
    },
];

/// The commands snit injects into every member body — see
/// [`DefinitionBodyGrammar::member_body_commands`].
///
/// VERIFIED against tcllib snit(n): `install` is available in any snit
/// instance method / constructor and, in its `using` form, "creates the
/// component using the specified command, and assigns the result to the
/// component variable".  The bare `install NAME $widget` form binds a
/// component whose class is only known at run time, so no
/// [`MemberBodyCommand::binds_handle`] fires for it — abstention, not a
/// guess.
const SNIT_MEMBER_BODY_COMMANDS: &[MemberBodyCommand] = &[MemberBodyCommand {
    name: "install",
    detail: "install a snit component into its component variable",
    binds_handle: Some(crate::handle_binding::SNIT_INSTALL_BINDS_HANDLE),
}];

/// The member-body commands a snit **widget** definer injects: `install`, plus
/// the hull installer only a widget has.
///
/// VERIFIED against tcllib snit(n): `installhull` exists for `snit::widget` /
/// `snit::widgetadaptor`, whose instances have a hull; a plain `snit::type`
/// has none, so it is deliberately absent from [`SNIT_MEMBER_BODY_COMMANDS`].
const SNIT_WIDGET_MEMBER_BODY_COMMANDS: &[MemberBodyCommand] = &[
    MemberBodyCommand {
        name: "install",
        detail: "install a snit component into its component variable",
        binds_handle: Some(crate::handle_binding::SNIT_INSTALL_BINDS_HANDLE),
    },
    MemberBodyCommand {
        name: "installhull",
        detail: "install the widget's hull component",
        binds_handle: Some(crate::handle_binding::SNIT_INSTALLHULL_BINDS_HANDLE),
    },
];

/// The two snit instance-name shapes that remain recognisable when a
/// low-level consumer has only a set of class names, not each class's family:
/// `%AUTO%` asks snit to generate a unique suffix, and a leading dot is the
/// conventional Tk widget path. Other plain names are valid too, but require
/// the exact class grammar to distinguish them from a method word.
fn snit_bare_word_construction_hint(word: &str) -> bool {
    word == "%AUTO%" || word.starts_with('.')
}

/// The definition-body grammar for a plain snit `type`.  `implicit_vars` is
/// the set snit injects into *every* member body; the widget definers carry
/// [`SNIT_WIDGET_GRAMMAR`], whose `implicit_vars` add the `win` / `hull`
/// pair a widget injects on top (they are not implicit in a plain
/// `snit::type`), so which definers inject them is registry data, not a
/// consumer's name-suffix check.
pub const SNIT_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Snit,
    members: SNIT_MEMBERS,
    implicit_vars: &["self", "selfns", "type", "options"],
    member_body_namespace_path: &[],
    // snit(n): "Every snit type has the following type methods: create,
    // info, destroy."  `create` is left out — see the field's doc comment.
    builtin_type_methods: &["info", "destroy"],
    builtin_object_methods: SNIT_BUILTIN_OBJECT_METHODS,
    builtin_terminating_methods: &[],
    member_body_commands: SNIT_MEMBER_BODY_COMMANDS,
    // snit(n), "The Type Command": `$type name ?args?` with a non-typemethod
    // first word is `$type create name ?args?`.
    bare_word_construction: true,
    bare_word_construction_hint: Some(snit_bare_word_construction_hint),
    dynamic_method_dispatch: true,
    manufacturers: SNIT_MANUFACTURERS,
    // snit dispatches an unrecognised *method* through `delegate method *`
    // when the type declares one, which is a declared delegation target
    // rather than a member body this analysis can read; there is no snit
    // counterpart of TclOO's `unknown`.
    unknown_dispatch_method: None,
    // snit's `configure` / `cget` are on every instance of the family, not
    // generated from property declarations, so they live in
    // `builtin_object_methods`.
    property_accessor_methods: &[],
};

/// The definition-body grammar for snit `widget` / `widgetadaptor`: the same
/// member set as [`SNIT_GRAMMAR`], plus the widget-only implicit instance
/// variables `win` (the widget's window path) and `hull` (the hull
/// component).
pub const SNIT_WIDGET_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Snit,
    members: SNIT_MEMBERS,
    implicit_vars: &["self", "selfns", "type", "options", "win", "hull"],
    // Same as SNIT_GRAMMAR: snit resolves member-body barewords through its
    // own generated type namespace, not an implicit helper path (#1137's
    // `::oo::Helpers` fact is TclOO-only).
    member_body_namespace_path: &[],
    // snit(n): "Every snit type has the following type methods: create,
    // info, destroy."  `create` is left out — see the field's doc comment.
    builtin_type_methods: &["info", "destroy"],
    builtin_object_methods: SNIT_BUILTIN_OBJECT_METHODS,
    builtin_terminating_methods: &[],
    member_body_commands: SNIT_WIDGET_MEMBER_BODY_COMMANDS,
    // snit(n), "The Type Command": `$type name ?args?` with a non-typemethod
    // first word is `$type create name ?args?`.
    bare_word_construction: true,
    bare_word_construction_hint: Some(snit_bare_word_construction_hint),
    dynamic_method_dispatch: true,
    manufacturers: SNIT_MANUFACTURERS,
    // snit dispatches an unrecognised *method* through `delegate method *`
    // when the type declares one, which is a declared delegation target
    // rather than a member body this analysis can read; there is no snit
    // counterpart of TclOO's `unknown`.
    unknown_dispatch_method: None,
    // snit's `configure` / `cget` are on every instance of the family, not
    // generated from property declarations, so they live in
    // `builtin_object_methods`.
    property_accessor_methods: &[],
};

// ---------------------------------------------------------------------------
// [incr Tcl] — `itcl::class Name { … }` (and the bare `class` alias) bodies.
//
// The access modifiers `public` / `protected` / `private` are prefix wrappers
// (`public method foo {args} {body}`, `private variable x`) — `MemberKind::
// Wrapper`, handled generically like TclOO's `self`.  `inherit` lists base
// classes (multiple inheritance).  `variable` declares an instance variable
// (optionally with an init value + config body); `common` a class/static one.
// ---------------------------------------------------------------------------

/// itcl `variable NAME ?init? ?configbody?` — the declared name plus the
/// optional trailing config body (the script run when the public variable is
/// modified via `configure`); the init value between them is left to the
/// default classifier.
const ITCL_VAR_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::VarWrite), (2, ArgRole::Body)];
/// itcl `common NAME ?init?` — a class/static variable; the declared name only
/// (no config body).
const ITCL_COMMON_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::VarWrite)];

const ITCL_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES),
    // A class-scoped `proc NAME ARGS BODY` — same shape as a method.
    MemberSpec::flat("proc", METHOD_ROLES),
    MemberSpec::flat("constructor", CTOR_ROLES),
    MemberSpec::flat("destructor", BODY0_ROLES),
    MemberSpec::flat("variable", ITCL_VAR_ROLES),
    MemberSpec::flat("common", ITCL_COMMON_ROLES),
    // Base-class list (multiple inheritance) — each argument names a base
    // class, a first-class command reference exactly like TclOO's `superclass`,
    // so navigation reaches the base class across files.
    MemberSpec::all_refs("inherit", MemberRefKind::Class),
    // Access modifiers: prefix wrappers around an inner member keyword.
    MemberSpec::wrapper("public"),
    MemberSpec::wrapper("protected"),
    MemberSpec::wrapper("private"),
];

/// The methods every [incr Tcl] **object** answers to without its class body
/// declaring them.
///
/// From the itcl(n) / itclObject documentation: every object supports
/// `configure`, `cget` and `isa`, plus the `info` ensemble extended with the
/// class-introspection subcommands.  Like snit, itcl dispatches through its
/// own generated machinery with no `my`-only tier, so all are exported.
/// Object destruction is `itcl::delete object NAME`, a command rather than a
/// method, so no `destroy` is listed.
const ITCL_BUILTIN_OBJECT_METHODS: &[BuiltinObjectMethod] = &[
    BuiltinObjectMethod {
        name: "configure",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "set one or more of the object's public variables",
    },
    BuiltinObjectMethod {
        name: "cget",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "retrieve the value of one of the object's public variables",
    },
    BuiltinObjectMethod {
        name: "isa",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "test whether the object belongs to the named class",
    },
    BuiltinObjectMethod {
        name: "info",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        detail: "introspect the object's class, variables and methods",
    },
];

/// The definition-body grammar for [incr Tcl] `itcl::class` / bare `class`.
/// Member bodies run in the object's context with the instance/common
/// variables and `this` in scope.
pub const ITCL_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Itcl,
    members: ITCL_MEMBERS,
    implicit_vars: &["this"],
    member_body_namespace_path: &[],
    // [incr Tcl] class commands expose `::itcl::class` introspection rather
    // than built-in typemethods on the class command itself.
    builtin_type_methods: &[],
    builtin_object_methods: ITCL_BUILTIN_OBJECT_METHODS,
    builtin_terminating_methods: &[],
    member_body_commands: &[],
    // itcl constructs through `ClassName objName` *at the class command* —
    // but only via the documented `ClassName objName ?args?` form, which the
    // handle scan reaches through `creates_instance_at`, not through this
    // snit-specific bare-word shorthand.
    bare_word_construction: false,
    bare_word_construction_hint: None,
    dynamic_method_dispatch: false,
    // Reached through `creates_instance_at` (see above), not a named
    // manufacturer method on the class command.
    manufacturers: &[],
    // [incr Tcl] has no user-writable unrecognised-method fallback member.
    unknown_dispatch_method: None,
    // itcl's `configure` / `cget` are family built-ins (every instance has
    // them), so they live in `builtin_object_methods`.
    property_accessor_methods: &[],
};

// ---------------------------------------------------------------------------
// SpecTcl — the `.tclspec` spec-pack DSL's own declaration bodies.
//
// `speclib NAME VERSION { … }` is a definition body in exactly the sense the
// class definers are: its top-level words are declaration keywords, not
// commands, and they mean nothing outside it.  The DSL nests, so each block
// statement is *also* a definer whose own body has its own member vocabulary
// (`command … { … }` → the per-command keys, `hover { … }` → the six hover
// keys, …).  Nesting needs no new machinery: each block statement carries a
// `CommandSpec` whose `definition_body` is the inner grammar, and the shared
// walker's "an outer definer switches grammars" rule does the rest.
//
// The frozen syntax is `docs/design/spec-dsl-examples/README.md`; the member
// vocabulary below is its coverage matrix, one member per property word.
//
// Every field a class system uses to manufacture and dispatch instances is
// empty here — a spec pack creates no objects — so consumers keyed on those
// fields see nothing, and `DefinerFamily::SpecTcl` is the one-word way for a
// consumer that must not treat this as a class system to say so.
// ---------------------------------------------------------------------------

/// A `SpecTcl` block statement that names something and then takes a block:
/// `command NAME { … }`, `values NAME { … }`, `subcommand NAME { … }`.
const SPECTCL_NAMED_BLOCK_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Name), (1, ArgRole::Body)];
/// `hook NAME {params} { … }` — a shared hook body declared pack-wide, and
/// the same shape as a `proc`: name, parameter list, body.
const SPECTCL_NAMED_HOOK_ROLES: &[(u8, ArgRole)] = &[
    (0, ArgRole::Name),
    (1, ArgRole::ParamList),
    (2, ArgRole::Body),
];
/// A hook *property* statement — `const_fold {words ctx} { … }`.  The
/// `-native ID` spelling of the same statement carries no parameter list or
/// body; a walker only ever applies the `ParamList` / `Body` roles to a
/// braced word, so the reference form is left alone by construction.
const SPECTCL_HOOK_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::ParamList), (1, ArgRole::Body)];
/// `descriptor KEY NAME { … }` — the schema key it is a descriptor *for*,
/// the name it is declared under, then the block.
const SPECTCL_DESCRIPTOR_ROLES: &[(u8, ArgRole)] = &[
    (0, ArgRole::Keyword),
    (1, ArgRole::Name),
    (2, ArgRole::Body),
];

/// `command NAME ?-override? { … }` — the one pack-level statement with an
/// optional word between its name and its block.  `-override` claims a name
/// a shipped spec already has (the load-policy section of the syntax memo),
/// and inserting it shifts the block one place right, which is exactly what
/// [`OptionalMemberArgument`] models.
const SPECTCL_OVERRIDE_ARGUMENT: OptionalMemberArgument = OptionalMemberArgument {
    position: 1,
    values: &[MemberOptionValue {
        value: "-override",
        role: ArgRole::Option,
        dialects: None,
        declared_visibility: None,
    }],
};

/// The five pack-level statements of a `speclib` body.
const SPECTCL_PACK_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("command", SPECTCL_NAMED_BLOCK_ROLES)
        .optional_argument(SPECTCL_OVERRIDE_ARGUMENT),
    MemberSpec::flat("values", SPECTCL_NAMED_BLOCK_ROLES),
    MemberSpec::flat("hook", SPECTCL_NAMED_HOOK_ROLES),
    MemberSpec::flat("descriptor", SPECTCL_DESCRIPTOR_ROLES),
    // `default KEY VALUE…` sets one pack-wide availability/identity key; it
    // declares no name of its own and holds no script.
    MemberSpec::keyword_only("default"),
];

/// The declaration keys of a `command` / `subcommand` body.
///
/// One member per DSL property word of the syntax memo's coverage matrix.
/// The two tables (`CommandSpec` and `SubCommand`) are unioned into one
/// grammar rather than split: they overlap on all but a handful of words,
/// and a `subcommand` block is lexically identical to a `command` block, so
/// splitting would buy nothing a loader does not already have to check.
const SPECTCL_COMMAND_MEMBERS: &[MemberSpec] = &[
    // --- nested blocks (each also a `CommandSpec` carrying the inner
    //     grammar, so recursing into one switches vocabulary) --------------
    MemberSpec::flat("subcommand", SPECTCL_NAMED_BLOCK_ROLES),
    MemberSpec::flat("hover", BODY0_ROLES),
    // `values NAME { … }` is deliberately absent: the memo makes the shared
    // value table a *pack-level* declaration, referenced from here by
    // `arg -values-from NAME` / `option -values-from NAME`.
    MemberSpec::flat("case_list", BODY0_ROLES),
    MemberSpec::flat("clause_grammar", BODY0_ROLES),
    MemberSpec::flat("event_requires", BODY0_ROLES),
    MemberSpec::flat("world_effects", BODY0_ROLES),
    MemberSpec::flat("state_transitions", BODY0_ROLES),
    MemberSpec::flat("definition_body", BODY0_ROLES),
    MemberSpec::flat("body_scope", BODY0_ROLES),
    // `object_class NAME ?-superclass {…}? ?-allow-unknown? { … }` is
    // deliberately NOT a member row. Its block's index is a function of two
    // optional flags of *different widths*, which `OptionalMemberArgument`
    // (one optional word, one position) cannot express — and a member layout
    // that guessed index 1 would paint `-superclass`'s braced class list as a
    // definition block. It is instead a plain registered statement carrying
    // an `arg_role_resolver`, which reads the whole call; the word still
    // paints as a keyword (`Traits::LANGUAGE_KEYWORD`) and its block still
    // switches grammars (`CommandSpec::definition_body`), so nothing is lost
    // but the wrong answer.
    // --- hook bodies: a proc-shaped `{words ctx} { … }` pair -------------
    MemberSpec::flat("arg_role_resolver", SPECTCL_HOOK_ROLES),
    MemberSpec::flat("command_prefix_resolver", SPECTCL_HOOK_ROLES),
    MemberSpec::flat("const_fold", SPECTCL_HOOK_ROLES),
    MemberSpec::flat("const_fold_versioned", SPECTCL_HOOK_ROLES),
    MemberSpec::flat("taint_sink_gate", SPECTCL_HOOK_ROLES),
    MemberSpec::flat("context_gate", SPECTCL_HOOK_ROLES),
    MemberSpec::flat("literal_argument_validator", SPECTCL_HOOK_ROLES),
    MemberSpec::flat("clause_shape_check", SPECTCL_HOOK_ROLES),
    // --- row statements (the singular-row rule: a field holding a list of
    //     rows gets a singular statement, never a nested block) -----------
    MemberSpec::keyword_only("arg"),
    MemberSpec::keyword_only("option"),
    MemberSpec::keyword_only("option_conflict"),
    MemberSpec::keyword_only("form"),
    MemberSpec::keyword_only("side_effect"),
    MemberSpec::keyword_only("repeat"),
    MemberSpec::keyword_only("manufacturer"),
    MemberSpec::keyword_only("setter_constraint"),
    MemberSpec::keyword_only("sub_subcommand"),
    MemberSpec::keyword_only("oo_context_fact"),
    MemberSpec::keyword_only("versioned_arg_value"),
    MemberSpec::keyword_only("event_requirement_form"),
    MemberSpec::keyword_only("defines_symbol"),
    MemberSpec::keyword_only("binds_handle"),
    MemberSpec::keyword_only("frame_effect"),
    MemberSpec::keyword_only("byte_array_payload"),
    MemberSpec::keyword_only("deprecation_fix"),
    MemberSpec::keyword_only("event_handler_priority"),
    // --- scalar property words -------------------------------------------
    MemberSpec::keyword_only("traits"),
    MemberSpec::keyword_only("dialects"),
    MemberSpec::keyword_only("arity"),
    MemberSpec::keyword_only("detail"),
    MemberSpec::keyword_only("synopsis"),
    MemberSpec::keyword_only("return_type"),
    MemberSpec::keyword_only("var_write_typing"),
    MemberSpec::keyword_only("return_elements"),
    MemberSpec::keyword_only("var_elements_effect"),
    MemberSpec::keyword_only("representation_effect"),
    MemberSpec::keyword_only("allow_unknown_subcommands"),
    MemberSpec::keyword_only("prefix_matching"),
    MemberSpec::keyword_only("default_form_first_word"),
    MemberSpec::keyword_only("semantic_operation"),
    MemberSpec::keyword_only("assigns_variable_at"),
    MemberSpec::keyword_only("safe_on_uninit"),
    MemberSpec::keyword_only("lowering_hook"),
    MemberSpec::keyword_only("codegen_hook"),
    MemberSpec::keyword_only("inline_codegen_hook"),
    MemberSpec::keyword_only("analyser_hook"),
    MemberSpec::keyword_only("bpf_op"),
    MemberSpec::keyword_only("data_collection"),
    MemberSpec::keyword_only("command_table_effect"),
    MemberSpec::keyword_only("result_stability"),
    MemberSpec::keyword_only("inferred_storage_type"),
    MemberSpec::keyword_only("required_package"),
    MemberSpec::keyword_only("excluded_events"),
    MemberSpec::keyword_only("unsafe_command"),
    MemberSpec::keyword_only("side_switch_target"),
    MemberSpec::keyword_only("reserved_trailing_words"),
    MemberSpec::keyword_only("body_kind"),
    MemberSpec::keyword_only("body_arg_implicit_args"),
    MemberSpec::keyword_only("taint_output_sink"),
    MemberSpec::keyword_only("taint_output_sink_subcommands"),
    MemberSpec::keyword_only("taint_log_sink"),
    MemberSpec::keyword_only("taint_network_sink_args"),
    MemberSpec::keyword_only("taint_code_sink_args"),
    MemberSpec::keyword_only("taint_interp_eval_subcommands"),
    MemberSpec::keyword_only("taint_source"),
    MemberSpec::keyword_only("taint_transform"),
    MemberSpec::keyword_only("taint_double_encode_colour"),
    MemberSpec::keyword_only("taint_sink_safe_colour"),
    MemberSpec::keyword_only("credential_options"),
    MemberSpec::keyword_only("credential_arg"),
    MemberSpec::keyword_only("sensitive_headers"),
    MemberSpec::keyword_only("pattern_type"),
    MemberSpec::keyword_only("format_string_type"),
    MemberSpec::keyword_only("tcllib_package"),
    MemberSpec::keyword_only("introduced_version"),
    MemberSpec::keyword_only("deprecated_version"),
    MemberSpec::keyword_only("retired_version"),
    MemberSpec::keyword_only("warn_missing_import"),
    MemberSpec::keyword_only("is_namespace_exported"),
    MemberSpec::keyword_only("xc_translatable"),
    MemberSpec::keyword_only("xc_operation"),
    MemberSpec::keyword_only("deprecated_replacement"),
    MemberSpec::keyword_only("deprecated_replacement_drop_in"),
    MemberSpec::keyword_only("byte_array_effect"),
    MemberSpec::keyword_only("self_receiver_words"),
    MemberSpec::keyword_only("creates_instance_at"),
    MemberSpec::keyword_only("defines_command_at"),
    MemberSpec::keyword_only("implementation_namespace"),
    // --- `SubCommand`-only keys ------------------------------------------
    MemberSpec::keyword_only("pure"),
    MemberSpec::keyword_only("mutator"),
    MemberSpec::keyword_only("min_abbrev"),
    MemberSpec::keyword_only("loop_list_header"),
    MemberSpec::keyword_only("creates_scope_alias"),
    MemberSpec::keyword_only("arg_values_accept_prefix"),
    MemberSpec::keyword_only("destructive"),
    MemberSpec::keyword_only("returns_path"),
    MemberSpec::keyword_only("is_unescape"),
    MemberSpec::keyword_only("cfg_rewrite_name"),
    MemberSpec::keyword_only("max_leading_option_words"),
];

/// The six documentation keys of a `hover { … }` block.  `synopsis` and
/// `example` are repeatable and keep their order; `description` / `example`
/// / `returns` are the three keys the memo deliberately renames from their
/// Rust field names (`snippet` / `examples` / `return_value`).
const SPECTCL_HOVER_MEMBERS: &[MemberSpec] = &[
    MemberSpec::keyword_only("summary"),
    MemberSpec::keyword_only("synopsis"),
    MemberSpec::keyword_only("description"),
    MemberSpec::keyword_only("source"),
    MemberSpec::keyword_only("example"),
    MemberSpec::keyword_only("returns"),
];

/// A row whose first word is the thing it declares (`value V …`).
const SPECTCL_NAME0_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Name)];

/// The one repeatable row of a `values NAME { … }` table:
/// `value V ?-detail {…}? ?-min-tcl VER? ?-code N?`.
const SPECTCL_VALUES_MEMBERS: &[MemberSpec] = &[MemberSpec::flat("value", SPECTCL_NAME0_ROLES)];

/// The three clause rows of a `clause_grammar { … }` block.  `head` /
/// `repeated` / `tail` each carry a braced *slot list*, which is a role
/// vocabulary rather than a script, so no member declares a `Body`.
const SPECTCL_CLAUSE_GRAMMAR_MEMBERS: &[MemberSpec] = &[
    MemberSpec::keyword_only("head"),
    MemberSpec::keyword_only("repeated"),
    MemberSpec::keyword_only("tail"),
];

/// The nineteen plain-data fields of a `case_list { … }` block.
const SPECTCL_CASE_LIST_MEMBERS: &[MemberSpec] = &[
    MemberSpec::keyword_only("subject_args"),
    MemberSpec::keyword_only("two_arg_optionless_dialects"),
    MemberSpec::keyword_only("exact_option"),
    MemberSpec::keyword_only("glob_option"),
    MemberSpec::keyword_only("regex_option"),
    MemberSpec::keyword_only("nocase_option"),
    MemberSpec::keyword_only("end_options_option"),
    MemberSpec::keyword_only("fallthrough_body"),
    MemberSpec::keyword_only("value_options_require_regex"),
    MemberSpec::keyword_only("clause_flags"),
    MemberSpec::keyword_only("clause_regex_flag"),
    MemberSpec::keyword_only("clause_value_flags"),
    MemberSpec::keyword_only("clause_end_options_flag"),
    MemberSpec::keyword_only("clause_force_inline_flag"),
    MemberSpec::keyword_only("clause_force_list_flag"),
    MemberSpec::keyword_only("clause_force_list_shape"),
    MemberSpec::keyword_only("allow_omitted_final_body"),
    MemberSpec::keyword_only("keyword_patterns"),
    MemberSpec::keyword_only("warn_unbraced_bodies"),
];

/// The eight scalars of an `event_requires { … }` block.
const SPECTCL_EVENT_REQUIRES_MEMBERS: &[MemberSpec] = &[
    MemberSpec::keyword_only("client_side"),
    MemberSpec::keyword_only("server_side"),
    MemberSpec::keyword_only("transport"),
    MemberSpec::keyword_only("profiles"),
    MemberSpec::keyword_only("also_in"),
    MemberSpec::keyword_only("init_only"),
    MemberSpec::keyword_only("flow"),
    MemberSpec::keyword_only("capability"),
];

/// The rows of a `world_effects { … }` block.  `resolver` is reference-only
/// (`-native ID`, `none`, or a derivation keyword) — the memo excludes an
/// authored resolver, but the *word* is still part of the block's grammar.
const SPECTCL_WORLD_EFFECTS_MEMBERS: &[MemberSpec] = &[
    MemberSpec::keyword_only("composition"),
    MemberSpec::keyword_only("access"),
    MemberSpec::keyword_only("callback"),
    MemberSpec::keyword_only("resolver"),
    MemberSpec::keyword_only("dynamic_fallback"),
];

/// The rows of a `state_transitions { … }` block.
const SPECTCL_STATE_TRANSITIONS_MEMBERS: &[MemberSpec] = &[
    MemberSpec::keyword_only("composition"),
    MemberSpec::keyword_only("argument_shape"),
    MemberSpec::keyword_only("resolver"),
    MemberSpec::keyword_only("widen"),
    MemberSpec::keyword_only("covers"),
    MemberSpec::keyword_only("commit"),
];

/// The rows of a `definition_body { … }` block — `DefinitionBodyGrammar`'s
/// own fields, which is what makes the DSL able to describe *this* module's
/// data structure in itself.
const SPECTCL_DEFINITION_BODY_MEMBERS: &[MemberSpec] = &[
    MemberSpec::keyword_only("family"),
    MemberSpec::keyword_only("member"),
    MemberSpec::keyword_only("member_option"),
    MemberSpec::keyword_only("implicit_vars"),
    MemberSpec::keyword_only("member_body_namespace_path"),
    MemberSpec::keyword_only("builtin_type_methods"),
    MemberSpec::keyword_only("builtin_object_method"),
    MemberSpec::keyword_only("builtin_terminating_methods"),
    MemberSpec::keyword_only("member_body_command"),
    MemberSpec::keyword_only("bare_word_construction"),
    MemberSpec::keyword_only("dynamic_method_dispatch"),
    MemberSpec::keyword_only("manufacturer"),
    MemberSpec::keyword_only("unknown_dispatch_method"),
    MemberSpec::keyword_only("property_accessor_methods"),
];

/// The rows of a `body_scope { … }` block.  Its `command NAME { … }` is a
/// *scoped-command* row, not a pack-level command declaration — the same
/// context rule that makes `method` a member row inside `object_class` —
/// which is why it lives in this grammar with its own roles rather than
/// being borrowed from the pack grammar.
const SPECTCL_BODY_SCOPE_MEMBERS: &[MemberSpec] = &[
    MemberSpec::keyword_only("name"),
    MemberSpec::keyword_only("include_sibling_definitions"),
    MemberSpec::keyword_only("allow_unknown_commands"),
    MemberSpec::flat("command", SPECTCL_NAMED_BLOCK_ROLES),
];

/// The rows of an `object_class NAME { … }` block: `method NAME { … }`
/// bodies, which reuse the `subcommand` body grammar unchanged.
const SPECTCL_OBJECT_CLASS_MEMBERS: &[MemberSpec] =
    &[MemberSpec::flat("method", SPECTCL_NAMED_BLOCK_ROLES)];

/// Build one `SpecTcl` grammar from its member table.  Every field a class
/// system uses is empty: a spec pack manufactures nothing, dispatches
/// nothing, and injects no commands or variables into its blocks.
const fn spectcl_grammar(members: &'static [MemberSpec]) -> DefinitionBodyGrammar {
    DefinitionBodyGrammar {
        family: DefinerFamily::SpecTcl,
        members,
        implicit_vars: &[],
        member_body_namespace_path: &[],
        builtin_type_methods: &[],
        builtin_object_methods: &[],
        builtin_terminating_methods: &[],
        member_body_commands: &[],
        bare_word_construction: false,
        bare_word_construction_hint: None,
        dynamic_method_dispatch: false,
        manufacturers: &[],
        unknown_dispatch_method: None,
        property_accessor_methods: &[],
    }
}

/// The body of `speclib NAME VERSION { … }` — the pack itself.
pub const SPECTCL_PACK_GRAMMAR: DefinitionBodyGrammar = spectcl_grammar(SPECTCL_PACK_MEMBERS);
/// The body of `command NAME { … }` and of `subcommand NAME { … }`.
pub const SPECTCL_COMMAND_GRAMMAR: DefinitionBodyGrammar = spectcl_grammar(SPECTCL_COMMAND_MEMBERS);
/// The body of `hover { … }`.
pub const SPECTCL_HOVER_GRAMMAR: DefinitionBodyGrammar = spectcl_grammar(SPECTCL_HOVER_MEMBERS);
/// The body of `values NAME { … }`.
pub const SPECTCL_VALUES_GRAMMAR: DefinitionBodyGrammar = spectcl_grammar(SPECTCL_VALUES_MEMBERS);
/// The body of `clause_grammar { … }`.
pub const SPECTCL_CLAUSE_GRAMMAR_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_CLAUSE_GRAMMAR_MEMBERS);
/// The body of `case_list { … }`.
pub const SPECTCL_CASE_LIST_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_CASE_LIST_MEMBERS);
/// The body of `event_requires { … }`.
pub const SPECTCL_EVENT_REQUIRES_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_EVENT_REQUIRES_MEMBERS);
/// The body of `world_effects { … }`.
pub const SPECTCL_WORLD_EFFECTS_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_WORLD_EFFECTS_MEMBERS);
/// The body of `state_transitions { … }`.
pub const SPECTCL_STATE_TRANSITIONS_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_STATE_TRANSITIONS_MEMBERS);
/// The body of `definition_body { … }`.
pub const SPECTCL_DEFINITION_BODY_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_DEFINITION_BODY_MEMBERS);
/// The body of `body_scope { … }`.
pub const SPECTCL_BODY_SCOPE_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_BODY_SCOPE_MEMBERS);
/// The body of `object_class NAME { … }`.
pub const SPECTCL_OBJECT_CLASS_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_OBJECT_CLASS_MEMBERS);

/// Every `SpecTcl` grammar, so a sweep can assert the family's invariants
/// without enumerating the constants by hand.
pub const SPECTCL_GRAMMARS: &[&DefinitionBodyGrammar] = &[
    &SPECTCL_PACK_GRAMMAR,
    &SPECTCL_COMMAND_GRAMMAR,
    &SPECTCL_HOVER_GRAMMAR,
    &SPECTCL_VALUES_GRAMMAR,
    &SPECTCL_CLAUSE_GRAMMAR_GRAMMAR,
    &SPECTCL_CASE_LIST_GRAMMAR,
    &SPECTCL_EVENT_REQUIRES_GRAMMAR,
    &SPECTCL_WORLD_EFFECTS_GRAMMAR,
    &SPECTCL_STATE_TRANSITIONS_GRAMMAR,
    &SPECTCL_DEFINITION_BODY_GRAMMAR,
    &SPECTCL_BODY_SCOPE_GRAMMAR,
    &SPECTCL_OBJECT_CLASS_GRAMMAR,
];

#[cfg(test)]
mod tests {
    use super::{
        DeclaredMemberVisibility, MemberRetraction, MemberVisibility, SlotOp, SlotSpec,
        TCLOO_GRAMMAR,
    };
    use crate::arg_role::ArgRole;

    fn strs(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    /// Issue #1169: the four `TclOO` slot members carry their C-pinned
    /// default operations (`slots[]` in 9.0.4's tclOODefineCmds.c; identical
    /// forwards in 8.6.16's tclOO.c): `filter`/`variable` append,
    /// `superclass`/`mixin` replace — and only `variable` deduplicates
    /// (tclsh 9.0.4: `filter a; filter a b` → `a a b`, `variable a;
    /// variable a b` → `a b`).
    #[test]
    fn tcloo_slot_members_carry_their_c_pinned_defaults() {
        let want = [
            ("filter", SlotOp::Append, false),
            ("variable", SlotOp::Append, true),
            ("superclass", SlotOp::Set, false),
            ("mixin", SlotOp::Set, false),
        ];
        for (keyword, default_op, dedup) in want {
            let m = TCLOO_GRAMMAR.member(keyword).expect("member exists");
            let slot = m.slot.unwrap_or_else(|| panic!("`{keyword}` is a slot"));
            assert_eq!(slot.default_op, default_op, "{keyword} default op");
            assert_eq!(slot.dedup, dedup, "{keyword} dedup");
        }
    }

    /// TN for the slot flag: members that declare, retract, or flip
    /// visibility are not slots — folding their words through slot ops
    /// would corrupt real declarations.
    #[test]
    fn non_slot_members_carry_no_slot_spec() {
        for keyword in [
            "method",
            "constructor",
            "destructor",
            "forward",
            "export",
            "unexport",
            "deletemethod",
            "renamemethod",
        ] {
            let m = TCLOO_GRAMMAR.member(keyword).expect("member exists");
            assert_eq!(m.slot, None, "{keyword} must not be a slot");
        }
    }

    /// The fold itself, one op per case, each pinned live on tclsh 9.0.4.
    #[test]
    fn slot_fold_matches_the_oracle() {
        let filter = SlotSpec {
            default_op: SlotOp::Append,
            dedup: false,
        };
        // `filter a ; filter a b` → `a a b` (append, no dedup).
        let mut list = Vec::new();
        filter.apply(&mut list, &strs(&["a"]));
        filter.apply(&mut list, &strs(&["a", "b"]));
        assert_eq!(list, strs(&["a", "a", "b"]));
        // `filter -set x` → `x`; `filter -clear` → empty.
        filter.apply(&mut list, &strs(&["-set", "x"]));
        assert_eq!(list, strs(&["x"]));
        filter.apply(&mut list, &strs(&["-clear"]));
        assert!(list.is_empty());
        // `filter a ; filter -prepend b` → `b a`.
        filter.apply(&mut list, &strs(&["a"]));
        filter.apply(&mut list, &strs(&["-prepend", "b"]));
        assert_eq!(list, strs(&["b", "a"]));
        // `filter -remove b` → `a`; `-appendifnew a c` → `a c`.
        filter.apply(&mut list, &strs(&["-remove", "b"]));
        assert_eq!(list, strs(&["a"]));
        filter.apply(&mut list, &strs(&["-appendifnew", "a", "c"]));
        assert_eq!(list, strs(&["a", "c"]));
        // The op word is recognised at argument 0 only: `filter a -set b`
        // appends the three literal items (tclsh 9.0.4).
        let mut mid = Vec::new();
        filter.apply(&mut mid, &strs(&["a", "-set", "b"]));
        assert_eq!(mid, strs(&["a", "-set", "b"]));
        // An unknown leading `-op` aborts the definition in real Tcl —
        // the fold must leave the slot untouched, never treat it as data.
        let mut untouched = strs(&["keep"]);
        filter.apply(&mut untouched, &strs(&["-bogus", "x"]));
        assert_eq!(untouched, strs(&["keep"]));
        // The dedup rule (`variable`): appending an existing name is a no-op.
        let variable = SlotSpec {
            default_op: SlotOp::Append,
            dedup: true,
        };
        let mut vars = Vec::new();
        variable.apply(&mut vars, &strs(&["a"]));
        variable.apply(&mut vars, &strs(&["a", "b"]));
        assert_eq!(vars, strs(&["a", "b"]));
    }

    /// Every `TclOO` member word that *names* a method carries the effect it has
    /// on that method as registry data, so no consumer matches the keyword.
    /// Split three ways, all pinned against tclsh 9.0.4 / 8.6.14 (identical):
    ///   `export` / `unexport` change visibility and remove nothing;
    ///   `deletemethod` / `renamemethod` remove and change no visibility;
    ///   `filter` does neither — it merely refers.
    #[test]
    fn method_naming_members_declare_their_effect() {
        let want = [
            ("export", Some(MemberVisibility::Exported), None),
            ("unexport", Some(MemberVisibility::Unexported), None),
            ("deletemethod", None, Some(MemberRetraction::EveryArgument)),
            ("renamemethod", None, Some(MemberRetraction::FirstArgument)),
            ("filter", None, None),
        ];
        for (keyword, visibility, retraction) in want {
            let m = TCLOO_GRAMMAR
                .member(keyword)
                .unwrap_or_else(|| panic!("`{keyword}` is a TclOO member"));
            assert_eq!(m.visibility_effect, visibility, "{keyword} visibility");
            assert_eq!(m.retraction, retraction, "{keyword} retraction");
        }
    }

    /// TN for the flags: a member that declares or recurses must carry neither
    /// effect, or a walker keyed on them would retract / re-export real
    /// declarations.
    #[test]
    fn declaring_members_carry_no_member_effect() {
        for keyword in ["method", "constructor", "destructor", "variable", "forward"] {
            let m = TCLOO_GRAMMAR.member(keyword).expect("member exists");
            assert_eq!(m.visibility_effect, None, "{keyword}");
            assert_eq!(m.retraction, None, "{keyword}");
        }
    }

    /// Tcl 9 inserts a closed export-flag vocabulary after the method name,
    /// not before it and not in Tcl 8.6's fixed `name params body` layout.
    /// The grammar resolves both layouts without a consumer recognising
    /// `method` or parsing a `-word` itself.
    #[test]
    fn tcloo_method_visibility_option_shifts_only_the_fixed_tail() {
        let method = TCLOO_GRAMMAR.member("method").expect("method exists");
        let plain = strs(&["m", "{x}", "{return $x}"]);
        assert_eq!(
            method
                .indices_for_call(&plain, ArgRole::ParamList)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            method
                .indices_for_call(&plain, ArgRole::Body)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(method.declared_visibility_for(&plain), None);

        let private = strs(&["m", "-private", "{x}", "{return $x}"]);
        assert_eq!(
            method
                .indices_for_call(&private, ArgRole::Option)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            method
                .indices_for_call(&private, ArgRole::ParamList)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(
            method
                .indices_for_call(&private, ArgRole::Body)
                .collect::<Vec<_>>(),
            vec![3]
        );
        assert_eq!(
            method.declared_visibility_for(&private),
            Some(DeclaredMemberVisibility::Private)
        );
        // The selected profile is authoritative: Tcl 8.6 keeps the original
        // fixed layout and exposes the word as an unavailable registry option,
        // while Tcl 9.0 accepts and shifts it.
        assert_eq!(
            method
                .indices_for_call_in(
                    &private,
                    crate::dialects::DialectSet::TCL86,
                    ArgRole::ParamList
                )
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert!(
            method
                .indices_for_call_in(
                    &private,
                    crate::dialects::DialectSet::TCL86,
                    ArgRole::Option
                )
                .next()
                .is_none()
        );
        assert!(
            method
                .unavailable_option_for(&private, crate::dialects::DialectSet::TCL86)
                .is_some()
        );
        assert_eq!(
            method
                .indices_for_call_in(&private, crate::dialects::DialectSet::TCL90, ArgRole::Body)
                .collect::<Vec<_>>(),
            vec![3]
        );
        // Live Tcl 9.0 rejects the same flags on `classmethod`; its grammar
        // deliberately has no optional argument despite sharing the fixed
        // method-shaped roles.
        let classmethod = TCLOO_GRAMMAR
            .member("classmethod")
            .expect("classmethod exists");
        assert!(classmethod.optional_argument.is_none());
    }

    /// `definitionnamespace` is a namespace-reference tail with an optional,
    /// closed `-class` / `-instance` facet—not a keyword-only member whose
    /// target consumers have to rediscover.
    #[test]
    fn definitionnamespace_has_a_facet_option_and_final_namespace_name() {
        let member = TCLOO_GRAMMAR
            .member("definitionnamespace")
            .expect("definitionnamespace exists");
        let plain = strs(&["::definitions"]);
        assert_eq!(
            member
                .indices_for_call(&plain, ArgRole::NamespaceName)
                .collect::<Vec<_>>(),
            vec![0]
        );
        assert!(
            member
                .indices_for_call(&plain, ArgRole::Option)
                .next()
                .is_none()
        );

        let instance = strs(&["-instance", "::definitions"]);
        assert_eq!(
            member
                .indices_for_call(&instance, ArgRole::Option)
                .collect::<Vec<_>>(),
            vec![0]
        );
        assert_eq!(
            member
                .indices_for_call(&instance, ArgRole::NamespaceName)
                .collect::<Vec<_>>(),
            vec![1]
        );
    }
}
