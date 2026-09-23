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
use crate::invocation_words::{InvocationArgument, InvocationArguments, InvocationWord};
use crate::value_transfer::inputs::OperandId;
use tcl_dialect::TclVersion;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::surface_admits;

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
/// instances in real Tcl rather than plain assignments.
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

#[derive(Debug, Clone, Copy)]
struct SlotOpSpec {
    name: &'static str,
    operation: SlotOp,
    since_9: bool,
}

const TCLOO_SLOT_OPERATIONS: &[SlotOpSpec] = &[
    SlotOpSpec {
        name: "-append",
        operation: SlotOp::Append,
        since_9: false,
    },
    SlotOpSpec {
        name: "-appendifnew",
        operation: SlotOp::AppendIfNew,
        since_9: true,
    },
    SlotOpSpec {
        name: "-clear",
        operation: SlotOp::Clear,
        since_9: false,
    },
    SlotOpSpec {
        name: "-prepend",
        operation: SlotOp::Prepend,
        since_9: true,
    },
    SlotOpSpec {
        name: "-remove",
        operation: SlotOp::Remove,
        since_9: true,
    },
    SlotOpSpec {
        name: "-set",
        operation: SlotOp::Set,
        since_9: false,
    },
];

impl SlotOp {
    /// Parse an explicit slot-operation word, or `None` when `word` is not
    /// one.  Only the **first** argument of a slot call is ever an
    /// operation — tclsh 9.0.4: `filter a -set b` appends the three literal
    /// items `a`, `-set`, `b`.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        TCLOO_SLOT_OPERATIONS
            .iter()
            .find(|entry| entry.name == word)
            .map(|entry| entry.operation)
    }

    /// Resolve an exact runtime slot method for `version`.
    ///
    /// `TclOO` slot methods do not abbreviate. The miss message uses `TclOO`'s
    /// no-Oxford-comma method list and is filtered before rendering so Tcl 8.6
    /// never advertises the 9.0-only operations.
    ///
    /// # Errors
    /// Tcl's byte-exact `unknown method` message.
    pub fn resolve_runtime(word: &[u8], version: TclVersion) -> Result<Self, Vec<u8>> {
        let available: Vec<&SlotOpSpec> = TCLOO_SLOT_OPERATIONS
            .iter()
            .filter(|entry| !entry.since_9 || version >= TclVersion::V9_0)
            .collect();
        let names: Vec<&str> = available.iter().map(|entry| entry.name).collect();
        match tcl_cmd_core::prefix::scan(&names, word, true) {
            tcl_cmd_core::prefix::Resolution::Exact(index) => Ok(available[index].operation),
            tcl_cmd_core::prefix::Resolution::UniquePrefix(_)
            | tcl_cmd_core::prefix::Resolution::Ambiguous
            | tcl_cmd_core::prefix::Resolution::NoMatch => {
                let mut message = b"unknown method \"".to_vec();
                message.extend_from_slice(word);
                message.extend_from_slice(b"\": must be ");
                message.extend_from_slice(&tcl_cmd_core::prefix::tcloo_choice_list_bytes(&names));
                Err(message)
            }
        }
    }
}

/// The slot behaviour of a list-valued member word: its default operation
/// and whether the slot deduplicates on append.
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
    /// `mixin`, and `variable` channels cannot diverge.
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
    /// Every visibility, in the order the `.tclspec` vocabulary lists them.
    pub const ALL: &'static [Self] = &[Self::Public, Self::Private, Self::Unexported];

    /// The analyser/storage spelling used by existing class facts, and the
    /// `.tclspec` spelling of a wrapper's `-shift {-visibility V}`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Unexported => "unexported",
        }
    }

    /// The visibility `word` spells, or `None` for any other word.
    #[must_use]
    pub fn from_spelling(word: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|visibility| visibility.as_str() == word)
    }
}

/// What one member word of a definition body declares — the member-effect
/// descriptor (`docs/design/compiler/registry-consumer-contracts.md` § *The
/// member-effect descriptor*).
///
/// [`MemberKind`] stays the *layout* fact (`Flat`, `Wrapper`, `FlagKeyed`);
/// this is what the member means. The vocabulary is closed and family-neutral:
/// no variant names `TclOO`, snit or itcl, and [`DefinerFamily`] stays the only
/// place a family is named.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberEffect {
    /// A callable member: `method`, `classmethod`, `typemethod`,
    /// `constructor`, `destructor`, snit's `onconfigure` / `oncget`, a
    /// class-scoped `proc`.
    Callable {
        /// Which dispatch side the member lands on, before any
        /// [`MemberKind::Wrapper`] shift is applied.
        receiver: MemberReceiver,
        /// Its place in the object's lifecycle.
        role: CallableRole,
        /// Slot holding the declared name, 0-based after the keyword; `None`
        /// when the keyword *is* the name (`constructor`).
        name_slot: Option<u8>,
        /// Slot holding the formal parameter list.
        params_slot: Option<u8>,
        /// Slot holding the body.
        body_slot: Option<u8>,
    },
    /// A dispatch redirect: `forward NAME PREFIX ?word …?`.
    Forward {
        /// Slot holding the declared method name.
        name_slot: u8,
        /// Slot holding the command the method delegates to.
        prefix_slot: u8,
    },
    /// Declares state: `variable`, `typevariable`, itcl's `common`, snit's
    /// `option`.
    StateDeclaration {
        /// Whose state it is.
        scope: StateScope,
    },
    /// Contributes to an ancestry or interposition slot: `superclass`,
    /// `mixin`, `filter`, itcl's `inherit`. The operation and dedup rule stay
    /// [`MemberSpec::slot`]; this says which graph the slot feeds.
    Relation {
        /// The graph the member's words feed.
        slot: RelationSlot,
    },
    /// Changes an existing member's visibility. The value stays
    /// [`MemberSpec::visibility_effect`].
    Visibility,
    /// Removes existing members. Which arguments stays
    /// [`MemberSpec::retraction`].
    Retraction,
    /// A script with no member of its own, run at definition or construction
    /// time: snit's `typeconstructor`, `TclOO`'s `initialise`.
    InitScript {
        /// Slot holding the script.
        body_slot: u8,
        /// When it runs.
        timing: InitTiming,
    },
    /// Configures the definition and declares nothing: a wrapper
    /// (`self`, `private`, itcl's access modifiers), `definitionnamespace`,
    /// `property`'s flag-keyed accessors until they are `Callable` rows of
    /// their own, and every row of a declaration document (`.tclspec`,
    /// `SslicTcl`), which opens no method frame.
    Configuration,
}

impl MemberEffect {
    /// Every effect kind's `.tclspec` spelling — the first word of a member
    /// row's `-effect` value — in the order the vocabulary lists them.
    pub const KIND_SPELLINGS: &'static [&'static str] = &[
        "callable",
        "forward",
        "state-declaration",
        "relation",
        "visibility",
        "retraction",
        "init-script",
        "configuration",
    ];

    /// The `.tclspec` spelling of this effect's kind (see
    /// [`Self::KIND_SPELLINGS`]).
    #[must_use]
    pub const fn kind_spelling(self) -> &'static str {
        match self {
            Self::Callable { .. } => "callable",
            Self::Forward { .. } => "forward",
            Self::StateDeclaration { .. } => "state-declaration",
            Self::Relation { .. } => "relation",
            Self::Visibility => "visibility",
            Self::Retraction => "retraction",
            Self::InitScript { .. } => "init-script",
            Self::Configuration => "configuration",
        }
    }

    /// The side the member lands on before any wrapper shift: a callable's
    /// declared receiver; per-type and option state on both sides (itcl's
    /// `common` and snit's `typevariable` are visible from type and instance
    /// bodies alike, and an option is configured through an instance and
    /// declared by the type); a definition-time script on the type object;
    /// everything else on the instances.
    #[must_use]
    pub const fn natural_receiver(self) -> MemberReceiver {
        match self {
            Self::Callable { receiver, .. } => receiver,
            Self::StateDeclaration {
                scope: StateScope::PerType | StateScope::Option,
            } => MemberReceiver::Both,
            Self::InitScript {
                timing: InitTiming::AtDefinition,
                ..
            } => MemberReceiver::TypeObject,
            Self::Forward { .. }
            | Self::StateDeclaration {
                scope: StateScope::PerInstance,
            }
            | Self::Relation { .. }
            | Self::Visibility
            | Self::Retraction
            | Self::InitScript {
                timing: InitTiming::AtConstruction,
                ..
            }
            | Self::Configuration => MemberReceiver::Instance,
        }
    }
}

/// Which dispatch side a member lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberReceiver {
    /// The instances the definition creates.
    Instance,
    /// The class or type object itself (`self method`, `typemethod`).
    TypeObject,
    /// Both sides, which itcl's `common` and snit's `option` need.
    Both,
}

/// A callable member's place in the object's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallableRole {
    /// An ordinary method.
    Method,
    /// Runs when an instance is created.
    Constructor,
    /// Runs when an instance is destroyed.
    Destructor,
    /// Answers an option or property read (snit's `oncget`).
    Accessor,
    /// Handles an option or property write (snit's `onconfigure`).
    Mutator,
}

/// Whose state a [`MemberEffect::StateDeclaration`] declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateScope {
    /// One cell per instance (`variable`).
    PerInstance,
    /// One cell for the type (`typevariable`, itcl's `common`).
    PerType,
    /// An option an instance configures (snit's `option`).
    Option,
}

/// The graph a [`MemberEffect::Relation`] member feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationSlot {
    /// The ancestry: `superclass`, itcl's `inherit`.
    Superclass,
    /// The mixed-in classes.
    Mixin,
    /// The interposed filter methods.
    Filter,
}

/// When a [`MemberEffect::InitScript`] runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitTiming {
    /// Once, when the definition is evaluated.
    AtDefinition,
    /// Each time an instance is constructed.
    AtConstruction,
}

/// The `.tclspec` spellings of the member-effect vocabulary's small enums:
/// `ALL` in the order the vocabulary lists them, `spelling` for the renderer
/// and `from_spelling` for the loader, so neither keeps a table of its own.
macro_rules! member_effect_spellings {
    ($($ty:ident { $($variant:ident => $spelling:literal),+ $(,)? })+) => {$(
        impl $ty {
            /// Every value, in the order the `.tclspec` vocabulary lists them.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The `.tclspec` spelling.
            #[must_use]
            pub const fn spelling(self) -> &'static str {
                match self {
                    $(Self::$variant => $spelling),+
                }
            }

            /// The value `word` spells, or `None` for any other word.
            #[must_use]
            pub fn from_spelling(word: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|value| value.spelling() == word)
            }
        }
    )+};
}

member_effect_spellings! {
    MemberReceiver {
        Instance => "instance",
        TypeObject => "type-object",
        Both => "both",
    }
    CallableRole {
        Method => "method",
        Constructor => "constructor",
        Destructor => "destructor",
        Accessor => "accessor",
        Mutator => "mutator",
    }
    StateScope {
        PerInstance => "per-instance",
        PerType => "per-type",
        Option => "option",
    }
    RelationSlot {
        Superclass => "superclass",
        Mixin => "mixin",
        Filter => "filter",
    }
    InitTiming {
        AtDefinition => "at-definition",
        AtConstruction => "at-construction",
    }
}

/// What a [`MemberKind::Wrapper`] does to the member it wraps: `TclOO`'s
/// `self` moves it to the class object, `private` and itcl's access modifiers
/// declare its visibility. `None` in either field keeps the inner member's
/// own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrapperShift {
    /// The side the wrapped member lands on.
    pub receiver: Option<MemberReceiver>,
    /// The visibility the wrapped member is declared with.
    pub visibility: Option<DeclaredMemberVisibility>,
}

impl WrapperShift {
    /// The shift that changes nothing.
    pub const NONE: Self = Self {
        receiver: None,
        visibility: None,
    };

    /// This (inner) shift applied inside `outer`: a field this shift sets
    /// wins, a field it leaves open keeps the outer wrapper's.
    #[must_use]
    pub const fn within(self, outer: Self) -> Self {
        Self {
            receiver: match self.receiver {
                Some(receiver) => Some(receiver),
                None => outer.receiver,
            },
            visibility: match self.visibility {
                Some(visibility) => Some(visibility),
                None => outer.visibility,
            },
        }
    }
}

/// A callable member's parameter shape, read off its parameter-list word.
///
/// Counted the way Tcl binds arguments — positionally — so a parameter with a
/// default that precedes a required one is itself required: `{{a 1} b}` takes
/// exactly two arguments (tclsh 9.0.4 and 8.6: `wrong # args: should be "p ?a?
/// b"` for one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberArity {
    /// Arguments every call must supply: every parameter up to and including
    /// the last one without a default.
    pub required: usize,
    /// Further parameters, each with a default, a call may supply.
    pub optional: usize,
    /// Whether the list ends in `args`, taking any number beyond.
    pub variadic: bool,
}

impl MemberArity {
    /// The arity of the formal parameter list `params`, or `None` when it is
    /// not a well-formed one (the strict `tcl_syntax::formal_params` reading).
    #[must_use]
    pub fn parse(params: &str) -> Option<Self> {
        let parameters = tcl_syntax::formal_params::parse_formal_parameters(params).ok()?;
        let variadic = tcl_syntax::formal_params::has_trailing_args(&parameters);
        let fixed = &parameters[..parameters.len() - usize::from(variadic)];
        let required = fixed
            .iter()
            .rposition(|parameter| parameter.default.is_none())
            .map_or(0, |last| last + 1);
        Some(Self {
            required,
            optional: fixed.len() - required,
            variadic,
        })
    }
}

/// What one member statement of a definition body declares — the answer
/// [`DefinitionBodyGrammar::member_row`] derives from the member's
/// [`MemberEffect`] and the statement's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRow {
    /// Index of the member keyword in the statement's words — the wrapped
    /// member's own keyword for a wrapper's prefix form.
    pub keyword_index: usize,
    /// What the member declares.
    pub effect: MemberEffect,
    /// Side after every wrapper shift is applied.
    pub receiver: MemberReceiver,
    /// The declared name when the effect names one and the word is literal.
    /// A computed word abstains.
    pub name: Option<String>,
    /// Derived from the parameter-list slot, when the effect has one and the
    /// word is a literal list.
    pub arity: Option<MemberArity>,
    /// The family's name-based default, overridden by the member's own option
    /// word or its wrapper.
    pub visibility: DeclaredMemberVisibility,
    /// The body operand (an index into the statement's words), when the
    /// effect has one and the call supplies it.
    pub body: Option<OperandId>,
    /// The slot operation for a slot member (`SlotOp`, unchanged): the
    /// explicit leading operation word, or the slot's default.
    pub slot_op: Option<SlotOp>,
    /// Releases this row is available at ([`MemberSpec::surface`]).
    pub surface: Option<&'static [SpecSurface]>,
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
    pub surface: Option<&'static [SpecSurface]>,
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
        self.value_for_in(args, None)
    }

    fn value_for_in<S: AsRef<str>>(
        self,
        args: &[S],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<MemberOptionValue> {
        self.value_at(args.get(usize::from(self.position))?.as_ref(), dialect)
    }

    /// The spelling `word` names, when it is one available in `dialect`.
    fn value_at(self, word: &str, dialect: Option<SurfaceQuery<'_>>) -> Option<MemberOptionValue> {
        self.values.iter().copied().find(|candidate| {
            candidate.value == word
                && candidate
                    .surface
                    .is_none_or(|available| surface_admits(available, dialect.as_ref()))
        })
    }
}

/// A member call whose argument layout cannot be read: a recognised optional
/// word unavailable in the dialect, or a computed word where the optional
/// word could stand.
struct UnreadableLayout;

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
    pub surface: Option<&'static [SpecSurface]>,
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
    /// slot.
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
    /// What the member declares (see [`MemberEffect`]). Required: every
    /// member states it, so a consumer routes a member by its effect and
    /// never by its keyword.
    pub effect: MemberEffect,
    /// What a [`MemberKind::Wrapper`] does to the member it wraps, or `None`
    /// for a wrapper that changes nothing and for every other kind.
    pub wrapper_shift: Option<WrapperShift>,
}

impl MemberSpec {
    /// An ordinary [`MemberKind::Flat`] member.
    #[must_use]
    const fn flat(
        keyword: &'static str,
        arg_roles: &'static [(u8, ArgRole)],
        effect: MemberEffect,
    ) -> Self {
        Self {
            keyword,
            arg_roles,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            surface: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
            effect,
            wrapper_shift: None,
        }
    }

    /// A member whose every argument references an entity of `kind`
    /// (`superclass A B`, `export m`).
    #[must_use]
    const fn all_refs(keyword: &'static str, kind: MemberRefKind, effect: MemberEffect) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: Some(kind),
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            surface: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
            effect,
            wrapper_shift: None,
        }
    }

    /// A `variable a b c`-style member: every argument is a declared name.
    #[must_use]
    const fn all_vars(keyword: &'static str, effect: MemberEffect) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            optional_argument: None,
            all_args_var: true,
            all_args_ref: None,
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            surface: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
            effect,
            wrapper_shift: None,
        }
    }

    /// A name-reference / keyword-only member carrying nothing to recurse or
    /// declare (`superclass A B`, `inherit Base`, `option …`).
    #[must_use]
    const fn keyword_only(keyword: &'static str, effect: MemberEffect) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            surface: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
            effect,
            wrapper_shift: None,
        }
    }

    /// A [`MemberKind::Wrapper`] member (itcl's `public`/`protected`/`private`)
    /// — an inner member keyword follows at argument 0, and there is no bare
    /// script-block form.
    #[must_use]
    const fn wrapper(keyword: &'static str, shift: WrapperShift) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Wrapper,
            wrapper_block_body: false,
            surface: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
            effect: MemberEffect::Configuration,
            wrapper_shift: Some(shift),
        }
    }

    /// A [`MemberKind::Wrapper`] member that *also* accepts the bare
    /// script-block form — `TclOO`'s `self` and `private`, which are both
    /// `self method …` / `private method …` (prefix) and `self { … }` /
    /// `private { … }` (a definition script evaluated with altered visibility /
    /// target).  When the following word is not an inner member, argument 0 is
    /// the block [`ArgRole::Body`].
    #[must_use]
    const fn wrapper_or_body(keyword: &'static str, shift: WrapperShift) -> Self {
        Self {
            keyword,
            arg_roles: BODY0_ROLES,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Wrapper,
            wrapper_block_body: true,
            surface: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
            effect: MemberEffect::Configuration,
            wrapper_shift: Some(shift),
        }
    }

    /// Restrict this member to `surface` (a builder over the constructors
    /// above): `property` is 9.0+, so it carries `TCL90_PLUS` while every other
    /// `TclOO` member stays version-independent.
    #[must_use]
    const fn with_surface(mut self, surface: &'static [SpecSurface]) -> Self {
        self.surface = Some(surface);
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
    const fn flag_keyed(keyword: &'static str, effect: MemberEffect) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            optional_argument: None,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::FlagKeyed,
            wrapper_block_body: false,
            surface: None,
            retraction: None,
            visibility_effect: None,
            slot: None,
            effect,
            wrapper_shift: None,
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
        dialect: Option<SurfaceQuery<'_>>,
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
    /// `SslicTcl` — the `.sslictcl` TLS-assurance DSL's declaration bodies
    /// (`certificate NAME { … }`, `endpoint NAME { … }`, `policy NAME { … }`,
    /// …).
    ///
    /// Not a class system, for the same reason [`Self::SpecTcl`] is not: a
    /// `.sslictcl` document declares TLS facts, manufactures no instances and
    /// dispatches nothing, so every "object" field of the grammar is empty
    /// for it. It shares [`DefinitionBodyGrammar`] because it needs exactly
    /// what the class definers need — a *context-sensitive* member vocabulary
    /// — and gets folding, semantic tokens, and body recursion from the same
    /// generic consumers.
    SslicTcl,
}

/// How a definer family selects the current namespace for an executable
/// member body.
///
/// This is distinct from [`DefinitionBodyGrammar::member_body_namespace_path`]:
/// the current namespace is searched first, while that path supplies ordered
/// fallbacks. `TclOO` selects the runtime receiver object's private namespace;
/// snit, itcl, `SpecTcl`, and `SslicTcl` bodies use the statically named
/// definition target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberCurrentNamespace {
    /// The class, type, or declaration target names the member namespace.
    DefinedEntity,
    /// The invoked receiver selects an object namespace at runtime.
    RuntimeReceiver,
}

impl DefinerFamily {
    /// Whether this grammar defines commands/objects that can be invoked by
    /// the Tcl runtime after the declaration completes.
    ///
    /// `SpecTcl` and `SslicTcl` reuse the definition-body grammar for structured
    /// declaration documents; their named blocks are data, not runtime class
    /// or command manufacturers. Compiler consumers use this owner-level
    /// predicate instead of maintaining their own family lists.
    #[must_use]
    pub const fn manufactures_runtime_commands(self) -> bool {
        matches!(self, Self::TclOo | Self::Snit | Self::Itcl)
    }

    /// Registry-owned current-namespace policy for executable member bodies.
    #[must_use]
    pub const fn member_current_namespace(self) -> MemberCurrentNamespace {
        match self {
            Self::TclOo => MemberCurrentNamespace::RuntimeReceiver,
            Self::Snit | Self::Itcl | Self::SpecTcl | Self::SslicTcl => {
                MemberCurrentNamespace::DefinedEntity
            }
        }
    }
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
    /// candidate-list finalisation, so no consumer spells
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
    /// from here, so no walker spells the keyword.  Empty for a
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
    /// The registry half of "is `X create Name Body` a class creation?", so a
    /// family manufacturing under a different word can be added without
    /// hardcoding another keyword into the walker.  Empty for a family whose
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
    /// genuine unknown-method error, so a consumer must not accept it. snit
    /// and [incr Tcl] *do* give every instance `configure` /
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

/// The wrappers a member statement sits inside, as
/// [`DefinitionBodyGrammar::member_row`] crosses them.
#[derive(Clone, Copy)]
struct Wrapping {
    /// Their combined shift, the innermost wrapper's fields winning.
    shift: WrapperShift,
    /// The innermost wrapper's release set, which a wrapped member without one
    /// of its own inherits (`private method` is 9.0+ because `private` is).
    surface: Option<&'static [SpecSurface]>,
}

impl Wrapping {
    /// Outside every wrapper.
    const NONE: Self = Self {
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
    /// Current-namespace policy for executable members of this grammar.
    #[must_use]
    pub const fn member_current_namespace(&self) -> MemberCurrentNamespace {
        self.family.member_current_namespace()
    }

    /// The member grammar for `keyword`, if it is a recognised member.
    #[must_use]
    pub fn member(&self, keyword: &str) -> Option<&'static MemberSpec> {
        // `members` is `&'static`, so the borrow can be handed back as static.
        let idx = self.members.iter().position(|m| m.keyword == keyword)?;
        Some(&self.members[idx])
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
        self.member_indices_where(keyword, args, dialect, ArgRole::carries_script)
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
        self.member_indices_where(keyword, args, dialect, ArgRole::folds_as_block)
    }

    /// The shared walk behind [`Self::member_body_indices_in`] and
    /// [`Self::member_block_indices_in`]: `admit` selects which roles count.
    fn member_indices_where(
        &self,
        keyword: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
        admit: fn(ArgRole) -> bool,
    ) -> Vec<usize> {
        let Some(member) = self.member(keyword) else {
            return Vec::new();
        };
        if member.unavailable_option_for(args, dialect).is_some() {
            return Vec::new();
        }
        let selected = |member: &'static MemberSpec| -> Vec<usize> {
            let mut out: Vec<usize> = ArgRole::ALL
                .iter()
                .filter(|role| admit(**role))
                .flat_map(|&role| {
                    member
                        .indices_for_call_in(args, dialect, role)
                        .filter(|&index| index < args.len())
                })
                .collect();
            out.sort_unstable();
            out.dedup();
            out
        };
        match member.kind {
            MemberKind::Flat => selected(member),
            MemberKind::Wrapper => {
                let Some((inner, rest)) = args.split_first() else {
                    return Vec::new();
                };
                if self.member(inner).is_some() {
                    self.member_indices_where(inner, rest, dialect, admit)
                        .into_iter()
                        .map(|index| index + 1)
                        .collect()
                } else if member.wrapper_block_body {
                    selected(member)
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
            // SpecTcl and SslicTcl declare no members that are ever
            // *dispatched*, so the question is vacuous for them; answering
            // `true` keeps the visible
            // set equal to the declared set, which is the only reading of
            // "exported" a declaration-only family has.
            DefinerFamily::Snit
            | DefinerFamily::Itcl
            | DefinerFamily::SpecTcl
            | DefinerFamily::SslicTcl => true,
        }
    }
}

// TclOO — `oo::class` / `oo::configurable` / `oo::abstract` / `oo::singleton`
// `create` bodies and the bare `oo::define` / `oo::objdefine` script form.
//
// The irregular `self …` (nested member) and `property … -get/-set …`
// (flag-keyed bodies) forms are handled by the walker directly; every flat
// member is described here.

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
        surface: Some(TCL90_MEMBERS),
        declared_visibility: Some(DeclaredMemberVisibility::Public),
    },
    MemberOptionValue {
        value: "-private",
        role: ArgRole::Option,
        surface: Some(TCL90_MEMBERS),
        declared_visibility: Some(DeclaredMemberVisibility::Private),
    },
    MemberOptionValue {
        value: "-unexport",
        role: ArgRole::Option,
        surface: Some(TCL90_MEMBERS),
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
        surface: Some(TCL90_MEMBERS),
        declared_visibility: None,
    },
    MemberOptionValue {
        value: "-instance",
        role: ArgRole::Option,
        surface: Some(TCL90_MEMBERS),
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
const TCL90_MEMBERS: &[SpecSurface] = SpecSurface::TCL90_PLUS;

/// `method NAME PARAMS BODY` on the instances.
const INSTANCE_METHOD: MemberEffect =
    callable(MemberReceiver::Instance, CallableRole::Method, METHOD_SLOTS);
/// `classmethod` / `typemethod` / a class-scoped `proc NAME PARAMS BODY`, on
/// the class or type object.
const TYPE_METHOD: MemberEffect = callable(
    MemberReceiver::TypeObject,
    CallableRole::Method,
    METHOD_SLOTS,
);
/// `constructor PARAMS BODY`.
const CONSTRUCTOR: MemberEffect = callable(
    MemberReceiver::Instance,
    CallableRole::Constructor,
    (None, Some(0), Some(1)),
);
/// `destructor BODY`.
const DESTRUCTOR: MemberEffect = callable(
    MemberReceiver::Instance,
    CallableRole::Destructor,
    (None, None, Some(0)),
);
/// A definition-time script in the first slot (`typeconstructor`,
/// `initialise`).
const INIT_AT_DEFINITION: MemberEffect = MemberEffect::InitScript {
    body_slot: 0,
    timing: InitTiming::AtDefinition,
};
/// The name / parameter-list / body slots of a method-shaped member.
const METHOD_SLOTS: (Option<u8>, Option<u8>, Option<u8>) = (Some(0), Some(1), Some(2));

/// A [`MemberEffect::Callable`] from its receiver, role and
/// `(name, params, body)` slots.
const fn callable(
    receiver: MemberReceiver,
    role: CallableRole,
    (name_slot, params_slot, body_slot): (Option<u8>, Option<u8>, Option<u8>),
) -> MemberEffect {
    MemberEffect::Callable {
        receiver,
        role,
        name_slot,
        params_slot,
        body_slot,
    }
}

/// A [`MemberEffect::StateDeclaration`] of `scope`.
const fn state(scope: StateScope) -> MemberEffect {
    MemberEffect::StateDeclaration { scope }
}

/// A [`MemberEffect::Relation`] feeding `slot`.
const fn relation(slot: RelationSlot) -> MemberEffect {
    MemberEffect::Relation { slot }
}

/// A wrapper that moves nothing and declares `visibility`.
const fn declaring(visibility: DeclaredMemberVisibility) -> WrapperShift {
    WrapperShift {
        receiver: None,
        visibility: Some(visibility),
    }
}

const TCLOO_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES, INSTANCE_METHOD)
        .optional_argument(TCLOO_METHOD_VISIBILITY_ARGUMENT),
    MemberSpec::flat("classmethod", METHOD_ROLES, TYPE_METHOD).with_surface(TCL90_MEMBERS),
    MemberSpec::flat("constructor", CTOR_ROLES, CONSTRUCTOR),
    MemberSpec::flat("destructor", BODY0_ROLES, DESTRUCTOR),
    MemberSpec::flat("initialise", BODY0_ROLES, INIT_AT_DEFINITION).with_surface(TCL90_MEMBERS),
    MemberSpec::flat("initialize", BODY0_ROLES, INIT_AT_DEFINITION).with_surface(TCL90_MEMBERS),
    // `private` is a prefix wrapper (`private method m {} {…}`, `private
    // variable x`) *and* a bare definition-script block (`private { … }`).
    // The wrapped member keeps its side and is declared private.
    MemberSpec::wrapper_or_body("private", declaring(DeclaredMemberVisibility::Private))
        .with_surface(TCL90_MEMBERS),
    // `variable a b c` inside a class body declares every name.  A slot:
    // `-append` default like `filter`, but deduplicating (tclsh 9.0.4:
    // `variable a ; variable a b` → `a b`).
    MemberSpec::all_vars("variable", state(StateScope::PerInstance))
        .slot_spec(SlotOp::Append, true),
    // Reference-only members: they declare nothing and recurse nothing, but
    // their arguments *name* an entity defined elsewhere — a class or a method
    // — so they are references, not free strings.  All three are slots;
    // the defaults are pinned against C Tcl (`slots[]` in
    // 9.0.4's tclOODefineCmds.c, the `--default-operation` forwards in
    // 8.6.16's tclOO.c — identical): `superclass` / `mixin` replace,
    // `filter` appends.
    MemberSpec::all_refs(
        "superclass",
        MemberRefKind::Class,
        relation(RelationSlot::Superclass),
    )
    .slot_spec(SlotOp::Set, false),
    MemberSpec::all_refs("mixin", MemberRefKind::Class, relation(RelationSlot::Mixin))
        .slot_spec(SlotOp::Set, false),
    MemberSpec::all_refs(
        "filter",
        MemberRefKind::Method,
        relation(RelationSlot::Filter),
    )
    .slot_spec(SlotOp::Append, false),
    MemberSpec::all_refs("export", MemberRefKind::Method, MemberEffect::Visibility)
        .visibility(MemberVisibility::Exported),
    MemberSpec::all_refs("unexport", MemberRefKind::Method, MemberEffect::Visibility)
        .visibility(MemberVisibility::Unexported),
    MemberSpec::all_refs(
        "deletemethod",
        MemberRefKind::Method,
        MemberEffect::Retraction,
    )
    .retracting(MemberRetraction::EveryArgument),
    // `forward NAME cmd ?arg…?` declares NAME as a method; the word after it
    // (`cmd`) is the delegated command's name — a first-class command
    // reference the walker records so navigation reaches it, exactly like the
    // command a `superclass`/`mixin` names.  Any baked arguments after it are
    // ordinary values.
    MemberSpec::flat(
        "forward",
        FORWARD_ROLES,
        MemberEffect::Forward {
            name_slot: 0,
            prefix_slot: 1,
        },
    ),
    // `renamemethod FROM TO` — both name methods.
    // Both words name methods; the FROM word is retracted (and the TO word is
    // a member this walker does not record), so the whole call retracts.
    MemberSpec::all_refs(
        "renamemethod",
        MemberRefKind::Method,
        MemberEffect::Retraction,
    )
    .retracting(MemberRetraction::FirstArgument),
    MemberSpec::flat(
        "definitionnamespace",
        DEFINITION_NAMESPACE_ROLES,
        MemberEffect::Configuration,
    )
    .optional_argument(TCLOO_DEFINITION_NAMESPACE_ARGUMENT)
    .with_surface(TCL90_MEMBERS),
    // Structurally irregular — a nested-member wrapper (`self method …`) and a
    // flag-keyed body form (`property … -get/-set …`); their body indices come
    // from the walker's `MemberKind`-driven handling, not a hardcoded name.
    // `self` moves the wrapped member to the class object.
    MemberSpec::wrapper_or_body(
        "self",
        WrapperShift {
            receiver: Some(MemberReceiver::TypeObject),
            visibility: None,
        },
    ),
    // `property` (and its configurable-class accessor machinery) is a 9.0
    // addition; the 8.6 `TclOO` definition grammar has no such member.
    MemberSpec::flag_keyed("property", MemberEffect::Configuration).with_surface(TCL90_MEMBERS),
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

// snit — `snit::type` / `snit::widget` / `snit::widgetadaptor` bodies.

/// `onconfigure -option valueVar BODY` (snit 1.x) — the value var + body.
const ONCONFIGURE_ROLES: &[(u8, ArgRole)] = &[(1, ArgRole::VarWrite), (2, ArgRole::Body)];
/// `oncget -option BODY` (snit 1.x) — the body.
const ONCGET_ROLES: &[(u8, ArgRole)] = &[(1, ArgRole::Body)];

const SNIT_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES, INSTANCE_METHOD),
    MemberSpec::flat("typemethod", METHOD_ROLES, TYPE_METHOD),
    // A type-private `proc NAME ARGS BODY` — same shape as a method.
    MemberSpec::flat("proc", METHOD_ROLES, TYPE_METHOD),
    MemberSpec::flat("constructor", CTOR_ROLES, CONSTRUCTOR),
    MemberSpec::flat("destructor", BODY0_ROLES, DESTRUCTOR),
    MemberSpec::flat("typeconstructor", BODY0_ROLES, INIT_AT_DEFINITION),
    // `onconfigure -option valueVar BODY` / `oncget -option BODY`: the
    // option's write and read handlers, named by the option word rather than a
    // `Name` slot.
    MemberSpec::flat(
        "onconfigure",
        ONCONFIGURE_ROLES,
        callable(
            MemberReceiver::Instance,
            CallableRole::Mutator,
            (None, None, Some(2)),
        ),
    ),
    MemberSpec::flat(
        "oncget",
        ONCGET_ROLES,
        callable(
            MemberReceiver::Instance,
            CallableRole::Accessor,
            (None, None, Some(1)),
        ),
    ),
    MemberSpec::flat("variable", VAR0_ROLES, state(StateScope::PerInstance)),
    MemberSpec::flat("typevariable", VAR0_ROLES, state(StateScope::PerType)),
    MemberSpec::flat("component", VAR0_ROLES, state(StateScope::PerInstance)),
    MemberSpec::flat("typecomponent", VAR0_ROLES, state(StateScope::PerType)),
    // Name-reference / option-declaration members — recognised keywords with
    // nothing to recurse or declare.
    MemberSpec::keyword_only("option", state(StateScope::Option)),
    MemberSpec::keyword_only("delegate", MemberEffect::Configuration),
    MemberSpec::keyword_only("expose", MemberEffect::Configuration),
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
    // own generated type namespace, not an implicit helper path (the
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

// [incr Tcl] — `itcl::class Name { … }` (and the bare `class` alias) bodies.
//
// The access modifiers `public` / `protected` / `private` are prefix wrappers
// (`public method foo {args} {body}`, `private variable x`) — `MemberKind::
// Wrapper`, handled generically like TclOO's `self`.  `inherit` lists base
// classes (multiple inheritance).  `variable` declares an instance variable
// (optionally with an init value + config body); `common` a class/static one.

/// itcl `variable NAME ?init? ?configbody?` — the declared name plus the
/// optional trailing config body (the script run when the public variable is
/// modified via `configure`); the init value between them is left to the
/// default classifier.
const ITCL_VAR_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::VarWrite), (2, ArgRole::Body)];
/// itcl `common NAME ?init?` — a class/static variable; the declared name only
/// (no config body).
const ITCL_COMMON_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::VarWrite)];

const ITCL_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES, INSTANCE_METHOD),
    // A class-scoped `proc NAME ARGS BODY` — same shape as a method.
    MemberSpec::flat("proc", METHOD_ROLES, TYPE_METHOD),
    MemberSpec::flat("constructor", CTOR_ROLES, CONSTRUCTOR),
    MemberSpec::flat("destructor", BODY0_ROLES, DESTRUCTOR),
    MemberSpec::flat("variable", ITCL_VAR_ROLES, state(StateScope::PerInstance)),
    MemberSpec::flat("common", ITCL_COMMON_ROLES, state(StateScope::PerType)),
    // Base-class list (multiple inheritance) — each argument names a base
    // class, a first-class command reference exactly like TclOO's `superclass`,
    // so navigation reaches the base class across files.
    MemberSpec::all_refs(
        "inherit",
        MemberRefKind::Class,
        relation(RelationSlot::Superclass),
    ),
    // Access modifiers: prefix wrappers around an inner member keyword. A
    // `protected` member is reachable from the class and its heirs but never
    // dispatched from outside — the unexported tier.
    MemberSpec::wrapper("public", declaring(DeclaredMemberVisibility::Public)),
    MemberSpec::wrapper("protected", declaring(DeclaredMemberVisibility::Unexported)),
    MemberSpec::wrapper("private", declaring(DeclaredMemberVisibility::Private)),
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

/// A document-grammar statement: it configures the thing being described
/// (a spec pack's command, a `.sslictcl` endpoint) and declares no callable,
/// state, relation or visibility of a runtime object — the
/// [`MemberEffect::Configuration`] row the `SpecTcl` and `SslicTcl`
/// grammars are built from.
const fn setting(keyword: &'static str, arg_roles: &'static [(u8, ArgRole)]) -> MemberSpec {
    MemberSpec::flat(keyword, arg_roles, MemberEffect::Configuration)
}

/// A document-grammar statement whose words carry no role — see [`setting`].
const fn setting_word(keyword: &'static str) -> MemberSpec {
    MemberSpec::keyword_only(keyword, MemberEffect::Configuration)
}

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
        surface: None,
        declared_visibility: None,
    }],
};

/// The five pack-level statements of a `speclib` body.
const SPECTCL_PACK_MEMBERS: &[MemberSpec] = &[
    setting("command", SPECTCL_NAMED_BLOCK_ROLES).optional_argument(SPECTCL_OVERRIDE_ARGUMENT),
    setting("values", SPECTCL_NAMED_BLOCK_ROLES),
    setting("hook", SPECTCL_NAMED_HOOK_ROLES),
    setting("descriptor", SPECTCL_DESCRIPTOR_ROLES),
    // `default KEY VALUE…` sets one pack-wide availability/identity key; it
    // declares no name of its own and holds no script.
    setting_word("default"),
];

/// The declaration keys of a `command` / `subcommand` body.
///
/// One member per DSL property word of the syntax memo's coverage matrix.
/// The two tables (`CommandSpec` and `SubCommand`) are unioned into one
/// grammar rather than split: they overlap on all but a handful of words,
/// and a `subcommand` block is lexically identical to a `command` block, so
/// splitting would buy nothing a loader does not already have to check.
const SPECTCL_COMMAND_MEMBERS: &[MemberSpec] = &[
    // Nested blocks: each also a `CommandSpec` carrying the inner grammar,
    // so recursing into one switches vocabulary.
    setting("subcommand", SPECTCL_NAMED_BLOCK_ROLES),
    setting("hover", BODY0_ROLES),
    // `values NAME { … }` is deliberately absent: the memo makes the shared
    // value table a *pack-level* declaration, referenced from here by
    // `arg -values-from NAME` / `option -values-from NAME`.
    setting("case_list", BODY0_ROLES),
    setting("clause_grammar", BODY0_ROLES),
    setting("event_requires", BODY0_ROLES),
    setting("world_effects", BODY0_ROLES),
    setting("state_transitions", BODY0_ROLES),
    setting("definition_body", BODY0_ROLES),
    setting("body_scope", BODY0_ROLES),
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
    // Hook bodies: a proc-shaped `{words ctx} { … }` pair.
    setting("arg_role_resolver", SPECTCL_HOOK_ROLES),
    setting("command_prefix_resolver", SPECTCL_HOOK_ROLES),
    setting("const_fold", SPECTCL_HOOK_ROLES),
    setting("const_fold_versioned", SPECTCL_HOOK_ROLES),
    setting("taint_sink_gate", SPECTCL_HOOK_ROLES),
    setting("context_gate", SPECTCL_HOOK_ROLES),
    setting("literal_argument_validator", SPECTCL_HOOK_ROLES),
    setting("clause_shape_check", SPECTCL_HOOK_ROLES),
    // Row statements, under the singular-row rule: a field holding a list
    // of rows gets a singular statement, never a nested block.
    setting_word("arg"),
    setting_word("option"),
    setting_word("option_conflict"),
    setting_word("form"),
    setting_word("side_effect"),
    setting_word("repeat"),
    setting_word("manufacturer"),
    setting_word("setter_constraint"),
    setting_word("sub_subcommand"),
    setting_word("oo_context_fact"),
    setting_word("versioned_arg_value"),
    setting_word("event_requirement_form"),
    setting_word("defines_symbol"),
    setting_word("binds_handle"),
    setting_word("frame_effect"),
    setting_word("byte_array_payload"),
    setting_word("deprecation_fix"),
    setting_word("event_handler_priority"),
    // Scalar property words.
    setting_word("traits"),
    setting_word("dialects"),
    setting_word("arity"),
    setting_word("detail"),
    setting_word("synopsis"),
    setting_word("return_type"),
    setting_word("var_write_typing"),
    setting_word("return_elements"),
    setting_word("var_elements_effect"),
    setting_word("representation_effect"),
    setting_word("allow_unknown_subcommands"),
    setting_word("prefix_matching"),
    setting_word("default_form_first_word"),
    setting_word("semantic_operation"),
    setting_word("assigns_variable_at"),
    setting_word("safe_on_uninit"),
    setting_word("lowering_hook"),
    setting_word("codegen_hook"),
    setting_word("inline_codegen_hook"),
    setting_word("analyser_hook"),
    setting_word("bpf_op"),
    setting_word("data_collection"),
    setting_word("command_table_effect"),
    setting_word("result_stability"),
    setting_word("inferred_storage_type"),
    setting_word("required_package"),
    setting_word("excluded_events"),
    setting_word("unsafe_command"),
    setting_word("side_switch_target"),
    setting_word("reserved_trailing_words"),
    setting_word("body_kind"),
    setting_word("body_arg_implicit_args"),
    setting_word("taint_output_sink"),
    setting_word("taint_output_sink_subcommands"),
    setting_word("taint_log_sink"),
    setting_word("taint_network_sink_args"),
    setting_word("taint_code_sink_args"),
    setting_word("taint_interp_eval_subcommands"),
    setting_word("taint_source"),
    setting_word("taint_transform"),
    setting_word("taint_double_encode_colour"),
    setting_word("taint_sink_safe_colour"),
    setting_word("credential_options"),
    setting_word("credential_arg"),
    setting_word("sensitive_headers"),
    setting_word("pattern_type"),
    setting_word("format_string_type"),
    setting_word("tcllib_package"),
    setting_word("introduced_version"),
    setting_word("deprecated_version"),
    setting_word("retired_version"),
    setting_word("warn_missing_import"),
    setting_word("is_namespace_exported"),
    setting_word("xc_translatable"),
    setting_word("deprecated_replacement"),
    setting_word("deprecated_replacement_drop_in"),
    setting_word("byte_array_effect"),
    setting_word("self_receiver_words"),
    setting_word("creates_instance_at"),
    setting_word("defines_command_at"),
    setting_word("implementation_namespace"),
    // `SubCommand`-only keys.
    setting_word("pure"),
    setting_word("mutator"),
    setting_word("min_abbrev"),
    setting_word("loop_list_header"),
    setting_word("creates_scope_alias"),
    setting_word("arg_values_accept_prefix"),
    setting_word("destructive"),
    setting_word("returns_path"),
    setting_word("is_unescape"),
    setting_word("cfg_rewrite_name"),
    setting_word("max_leading_option_words"),
];

/// The six documentation keys of a `hover { … }` block.  `synopsis` and
/// `example` are repeatable and keep their order; `description` / `example`
/// / `returns` are the three keys the memo deliberately renames from their
/// Rust field names (`snippet` / `examples` / `return_value`).
const SPECTCL_HOVER_MEMBERS: &[MemberSpec] = &[
    setting_word("summary"),
    setting_word("synopsis"),
    setting_word("description"),
    setting_word("source"),
    setting_word("example"),
    setting_word("returns"),
];

/// A row whose first word is the thing it declares (`value V …`).
const SPECTCL_NAME0_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Name)];

/// The one repeatable row of a `values NAME { … }` table:
/// `value V ?-detail {…}? ?-min-tcl VER? ?-code N?`.
const SPECTCL_VALUES_MEMBERS: &[MemberSpec] = &[setting("value", SPECTCL_NAME0_ROLES)];

/// The rows of a `clause_grammar { … }` block.  `head` / `repeated` /
/// `once` / `tail` each carry a braced *slot list*, which is a role
/// vocabulary rather than a script, and `group` a layout index, so no member
/// declares a `Body`; the last three state the chain-level rules.
const SPECTCL_CLAUSE_GRAMMAR_MEMBERS: &[MemberSpec] = &[
    setting_word("head"),
    setting_word("repeated"),
    setting_word("once"),
    setting_word("group"),
    setting_word("tail"),
    setting_word("fallthrough_body"),
    setting_word("default_clause"),
    setting_word("selection"),
];

/// The plain-data fields of a `case_list { … }` block. The command-level
/// switches that pick the match mode, fold case, or end the option run are
/// the command's own option rows, each declaring its effect.
const SPECTCL_CASE_LIST_MEMBERS: &[MemberSpec] = &[
    setting_word("subject_args"),
    setting_word("two_arg_optionless_surface"),
    setting_word("fallthrough_body"),
    setting_word("value_options_require_regex"),
    setting_word("clause_flags"),
    setting_word("clause_regex_flag"),
    setting_word("clause_value_flags"),
    setting_word("clause_end_options_flag"),
    setting_word("clause_force_inline_flag"),
    setting_word("clause_force_list_flag"),
    setting_word("clause_force_list_shape"),
    setting_word("allow_omitted_final_body"),
    setting_word("keyword_patterns"),
    setting_word("warn_unbraced_bodies"),
];

/// The six scalars of an `event_requires { … }` block.
const SPECTCL_EVENT_REQUIRES_MEMBERS: &[MemberSpec] = &[
    setting_word("client_side"),
    setting_word("server_side"),
    setting_word("transport"),
    setting_word("profiles"),
    setting_word("also_in"),
    setting_word("flow"),
];

/// The rows of a `world_effects { … }` block.  `resolver` is reference-only
/// (`-native ID`, `none`, or a derivation keyword) — the memo excludes an
/// authored resolver, but the *word* is still part of the block's grammar.
const SPECTCL_WORLD_EFFECTS_MEMBERS: &[MemberSpec] = &[
    setting_word("composition"),
    setting_word("access"),
    setting_word("callback"),
    setting_word("resolver"),
    setting_word("dynamic_fallback"),
];

/// The rows of a `state_transitions { … }` block.
const SPECTCL_STATE_TRANSITIONS_MEMBERS: &[MemberSpec] = &[
    setting_word("composition"),
    setting_word("argument_shape"),
    setting_word("resolver"),
    setting_word("widen"),
    setting_word("covers"),
    setting_word("commit"),
];

/// The rows of a `definition_body { … }` block — `DefinitionBodyGrammar`'s
/// own fields, which is what makes the DSL able to describe *this* module's
/// data structure in itself.
const SPECTCL_DEFINITION_BODY_MEMBERS: &[MemberSpec] = &[
    setting_word("family"),
    setting_word("member"),
    setting_word("member_option"),
    setting_word("implicit_vars"),
    setting_word("member_body_namespace_path"),
    setting_word("builtin_type_methods"),
    setting_word("builtin_object_method"),
    setting_word("builtin_terminating_methods"),
    setting_word("member_body_command"),
    setting_word("bare_word_construction"),
    setting_word("dynamic_method_dispatch"),
    setting_word("manufacturer"),
    setting_word("unknown_dispatch_method"),
    setting_word("property_accessor_methods"),
];

/// The rows of a `body_scope { … }` block.  Its `command NAME { … }` is a
/// *scoped-command* row, not a pack-level command declaration — the same
/// context rule that makes `method` a member row inside `object_class` —
/// which is why it lives in this grammar with its own roles rather than
/// being borrowed from the pack grammar.
const SPECTCL_BODY_SCOPE_MEMBERS: &[MemberSpec] = &[
    setting_word("name"),
    setting_word("include_sibling_definitions"),
    setting_word("allow_unknown_commands"),
    setting("command", SPECTCL_NAMED_BLOCK_ROLES),
];

/// The rows of an `object_class NAME { … }` block: `method NAME { … }`
/// bodies, which reuse the `subcommand` body grammar unchanged.
const SPECTCL_OBJECT_CLASS_MEMBERS: &[MemberSpec] = &[setting("method", SPECTCL_NAMED_BLOCK_ROLES)];

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

/// The one declaration legal at the **root** of a `.tclspec` pack.
///
/// `speclib` is the DSL's only possible top-level word (`spec-packs.md`), so
/// the document grammar is a single row — and that is what lets completion
/// offer it, and only it, at the root of a pack.
const SPECTCL_DOCUMENT_MEMBERS: &[MemberSpec] = &[setting("speclib", SPECTCL_SPECLIB_ROLES)];

/// `speclib NAME DSL-VERSION { … }` — the name first, the body third.
const SPECTCL_SPECLIB_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Name), (2, ArgRole::Body)];

/// The body of a `.tclspec` **document** — see [`SPECTCL_DOCUMENT_MEMBERS`].
pub const SPECTCL_DOCUMENT_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_DOCUMENT_MEMBERS);

/// Every `SpecTcl` grammar, so a sweep can assert the family's invariants
/// without enumerating the constants by hand.
pub const SPECTCL_GRAMMARS: &[&DefinitionBodyGrammar] = &[
    &SPECTCL_DOCUMENT_GRAMMAR,
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

// SslicTcl — the `.sslictcl` TLS-assurance declaration DSL.
//
// Every block statement's member table is exactly the row vocabulary of that
// block, and a *nested* block (`hsts`, `anchor`, `check`, `grade`) appears
// twice: once as a member row of its parent's grammar, and once as its own
// statement carrying its own grammar. That pairing is what lets the shared
// walker switch vocabularies on the way down with no walker changes.

/// A nested bare-block member row: `hsts { … }`, `grade { … }`.
const SSLICTCL_BLOCK_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Body)];

/// A nested named-block member row: `anchor SHA256 { … }`, `check ID { … }`.
const SSLICTCL_NAMED_BLOCK_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Name), (1, ArgRole::Body)];

/// A member row whose one word is a retained, never-evaluated script.
const SSLICTCL_OPAQUE_SCRIPT_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::OpaqueScript)];

/// The rows of a `certificate NAME { … }` block.
const SSLICTCL_CERTIFICATE_MEMBERS: &[MemberSpec] = &[
    setting_word("pem"),
    setting_word("material"),
    setting_word("key"),
];

/// The rows of an `endpoint NAME { … }` block.
const SSLICTCL_ENDPOINT_MEMBERS: &[MemberSpec] = &[
    setting_word("hostname"),
    setting_word("protocols"),
    setting_word("ciphers"),
    setting_word("groups"),
    setting_word("signature-schemes"),
    setting_word("certificate-chain"),
    setting_word("chain"),
    setting_word("policy"),
    setting("hsts", SSLICTCL_BLOCK_ROLES),
];

/// The rows of an `hsts { … }` block.
const SSLICTCL_HSTS_MEMBERS: &[MemberSpec] = &[
    setting_word("enabled"),
    setting_word("max-age"),
    setting_word("include-subdomains"),
    setting_word("preload"),
];

/// The rows of a `testssl-import NAME { … }` block.
const SSLICTCL_TESTSSL_IMPORT_MEMBERS: &[MemberSpec] =
    &[setting_word("schema"), setting_word("raw-json-hex")];

/// The rows of a `trust-program NAME { … }` block.
const SSLICTCL_TRUST_PROGRAM_MEMBERS: &[MemberSpec] = &[
    setting_word("client"),
    setting_word("version"),
    setting_word("generated-at"),
    setting_word("source-name"),
    setting_word("source-url"),
    setting_word("source-revision"),
    setting_word("source-license"),
    setting("anchor", SSLICTCL_NAMED_BLOCK_ROLES),
];

/// The rows of an `anchor SHA256 { … }` block.
const SSLICTCL_ANCHOR_MEMBERS: &[MemberSpec] = &[
    setting_word("subject"),
    setting_word("der-base64"),
    setting_word("purposes"),
    setting_word("trusted"),
    setting_word("distrust-after"),
];

/// The rows of a `protocol VERSION { … }` block.
const SSLICTCL_PROTOCOL_MEMBERS: &[MemberSpec] = &[
    setting_word("status"),
    setting_word("score"),
    setting_word("reference"),
];

/// The rows of a `cipher NAME { … }` block.
const SSLICTCL_CIPHER_MEMBERS: &[MemberSpec] = &[
    setting_word("iana-name"),
    setting_word("openssl-name"),
    setting_word("key-exchange"),
    setting_word("authentication"),
    setting_word("encryption"),
    setting_word("bits"),
    setting_word("forward-secrecy"),
    setting_word("aead"),
    setting_word("status"),
    setting_word("protocols"),
];

/// The one row of a `chain NAME { … }` block.
const SSLICTCL_CHAIN_MEMBERS: &[MemberSpec] = &[setting_word("certificates")];

/// The rows of a `policy NAME { … }` block.
const SSLICTCL_POLICY_MEMBERS: &[MemberSpec] = &[
    setting("check", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("grade", SSLICTCL_BLOCK_ROLES),
];

/// The rows of a `check ID { … }` block. `predicate` carries a braced script
/// the loader retains verbatim and never evaluates, so its word is an
/// [`ArgRole::OpaqueScript`]: it folds like a body, and no analysis enters it.
const SSLICTCL_CHECK_MEMBERS: &[MemberSpec] = &[
    setting_word("severity"),
    setting_word("message"),
    setting_word("require-protocols"),
    setting_word("forbid-protocols"),
    setting_word("forbid-ciphers"),
    setting_word("require-forward-secrecy"),
    setting_word("min-key-bits"),
    setting_word("require-hsts"),
    setting_word("min-hsts-max-age"),
    setting("predicate", SSLICTCL_OPAQUE_SCRIPT_ROLES),
];

/// The one row of a `grade { … }` block.
const SSLICTCL_GRADE_MEMBERS: &[MemberSpec] = &[setting_word("minimum")];

/// Build one `SslicTcl` grammar from its member table. Every field a class
/// system uses is empty: a TLS declaration manufactures nothing, dispatches
/// nothing, and injects no commands or variables into its blocks.
const fn sslictcl_grammar(members: &'static [MemberSpec]) -> DefinitionBodyGrammar {
    DefinitionBodyGrammar {
        family: DefinerFamily::SslicTcl,
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

/// The body of `certificate NAME { … }`.
pub const SSLICTCL_CERTIFICATE_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_CERTIFICATE_MEMBERS);
/// The body of `endpoint NAME { … }`.
pub const SSLICTCL_ENDPOINT_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_ENDPOINT_MEMBERS);
/// The body of `hsts { … }`.
pub const SSLICTCL_HSTS_GRAMMAR: DefinitionBodyGrammar = sslictcl_grammar(SSLICTCL_HSTS_MEMBERS);
/// The body of `testssl-import NAME { … }`.
pub const SSLICTCL_TESTSSL_IMPORT_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_TESTSSL_IMPORT_MEMBERS);
/// The body of `trust-program NAME { … }`.
pub const SSLICTCL_TRUST_PROGRAM_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_TRUST_PROGRAM_MEMBERS);
/// The body of `anchor SHA256 { … }`.
pub const SSLICTCL_ANCHOR_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_ANCHOR_MEMBERS);
/// The body of `protocol VERSION { … }`.
pub const SSLICTCL_PROTOCOL_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_PROTOCOL_MEMBERS);
/// The body of `cipher NAME { … }`.
pub const SSLICTCL_CIPHER_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_CIPHER_MEMBERS);
/// The body of `chain NAME { … }`.
pub const SSLICTCL_CHAIN_GRAMMAR: DefinitionBodyGrammar = sslictcl_grammar(SSLICTCL_CHAIN_MEMBERS);
/// The body of `policy NAME { … }`.
pub const SSLICTCL_POLICY_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_POLICY_MEMBERS);
/// The body of `check ID { … }`.
pub const SSLICTCL_CHECK_GRAMMAR: DefinitionBodyGrammar = sslictcl_grammar(SSLICTCL_CHECK_MEMBERS);
/// The body of `grade { … }`.
pub const SSLICTCL_GRADE_GRAMMAR: DefinitionBodyGrammar = sslictcl_grammar(SSLICTCL_GRADE_MEMBERS);

/// The nine declarations legal at the **root** of a `.sslictcl` document.
///
/// A document is itself a definition body: `hostname` is a member of
/// `endpoint` and of nothing else, and by exactly the same rule `endpoint` is
/// a member of the *document* and `hostname` is not. Giving the root a grammar
/// is what lets the generic consumers answer both halves the same way —
/// completion offers these nine at the top level, and the token walker paints
/// them from membership rather than from a global keyword trait that would
/// also fire for a misplaced member row.
const SSLICTCL_DOCUMENT_MEMBERS: &[MemberSpec] = &[
    // The header names nothing and opens nothing: its word is a version.
    setting_word("sslictcl"),
    setting("certificate", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("endpoint", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("testssl-import", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("trust-program", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("protocol", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("cipher", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("chain", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("policy", SSLICTCL_NAMED_BLOCK_ROLES),
];

/// The body of a `.sslictcl` **document** — see [`SSLICTCL_DOCUMENT_MEMBERS`].
pub const SSLICTCL_DOCUMENT_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_DOCUMENT_MEMBERS);

/// Every `SslicTcl` grammar, so a sweep can assert the family's invariants
/// without enumerating the constants by hand.
pub const SSLICTCL_GRAMMARS: &[&DefinitionBodyGrammar] = &[
    &SSLICTCL_DOCUMENT_GRAMMAR,
    &SSLICTCL_CERTIFICATE_GRAMMAR,
    &SSLICTCL_ENDPOINT_GRAMMAR,
    &SSLICTCL_HSTS_GRAMMAR,
    &SSLICTCL_TESTSSL_IMPORT_GRAMMAR,
    &SSLICTCL_TRUST_PROGRAM_GRAMMAR,
    &SSLICTCL_ANCHOR_GRAMMAR,
    &SSLICTCL_PROTOCOL_GRAMMAR,
    &SSLICTCL_CIPHER_GRAMMAR,
    &SSLICTCL_CHAIN_GRAMMAR,
    &SSLICTCL_POLICY_GRAMMAR,
    &SSLICTCL_CHECK_GRAMMAR,
    &SSLICTCL_GRADE_GRAMMAR,
];

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family, SurfaceQuery};

    use super::{
        CallableRole, DeclaredMemberVisibility, DefinerFamily, ITCL_GRAMMAR, InitTiming,
        MemberArity, MemberEffect, MemberReceiver, MemberRetraction, MemberRow, MemberVisibility,
        RelationSlot, SNIT_GRAMMAR, SlotOp, SlotSpec, StateScope, TCL90_MEMBERS, TCLOO_GRAMMAR,
    };
    use crate::arg_role::ArgRole;
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::value_transfer::inputs::OperandId;

    fn strs(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn declaration_grammars_do_not_manufacture_runtime_commands() {
        for family in [
            DefinerFamily::TclOo,
            DefinerFamily::Snit,
            DefinerFamily::Itcl,
        ] {
            assert!(family.manufactures_runtime_commands());
        }
        for family in [DefinerFamily::SpecTcl, DefinerFamily::SslicTcl] {
            assert!(!family.manufactures_runtime_commands());
        }
    }

    /// The four `TclOO` slot members carry their C-pinned
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
                    Some(SurfaceQuery::core(Family::Tcl, "8.6")),
                    ArgRole::ParamList
                )
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert!(
            method
                .indices_for_call_in(
                    &private,
                    Some(SurfaceQuery::core(Family::Tcl, "8.6")),
                    ArgRole::Option
                )
                .next()
                .is_none()
        );
        assert!(
            method
                .unavailable_option_for(&private, Some(SurfaceQuery::core(Family::Tcl, "8.6")))
                .is_some()
        );
        assert_eq!(
            method
                .indices_for_call_in(
                    &private,
                    Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                    ArgRole::Body
                )
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

    /// The member row of a literal statement under `TclOO`'s grammar.
    fn tcloo_row(words: &[&str]) -> Option<MemberRow> {
        TCLOO_GRAMMAR.member_row(0, InvocationArguments::literals(words), None)
    }

    /// `method m {a b} {…}` — the plan's first row: a `Callable` on the
    /// instances, its name, arity and body read off the statement.
    #[test]
    fn member_row_reads_a_method_statement() {
        let row = tcloo_row(&["method", "m", "a b", "return"]).expect("a member row");
        assert_eq!(
            row,
            MemberRow {
                keyword_index: 0,
                effect: MemberEffect::Callable {
                    receiver: MemberReceiver::Instance,
                    role: CallableRole::Method,
                    name_slot: Some(0),
                    params_slot: Some(1),
                    body_slot: Some(2),
                },
                receiver: MemberReceiver::Instance,
                name: Some("m".to_owned()),
                arity: Some(MemberArity {
                    required: 2,
                    optional: 0,
                    variadic: false,
                }),
                visibility: DeclaredMemberVisibility::Public,
                body: Some(OperandId(3)),
                slot_op: None,
                surface: None,
            }
        );
        // The family's name rule decides an unflagged method's visibility.
        let upper = tcloo_row(&["method", "Helper", "", ""]).expect("a member row");
        assert_eq!(upper.visibility, DeclaredMemberVisibility::Unexported);
    }

    /// `self method` — the wrapper moves the member to the class object and
    /// the row keys on the wrapped keyword; `self { … }` is a definition-time
    /// script whose members land on the class object.
    #[test]
    fn member_row_applies_the_self_wrapper() {
        let row = tcloo_row(&["self", "method", "m", "", "return"]).expect("a member row");
        assert_eq!(row.keyword_index, 1);
        assert_eq!(row.receiver, MemberReceiver::TypeObject);
        assert_eq!(row.name.as_deref(), Some("m"));
        assert_eq!(row.body, Some(OperandId(4)));

        let block = tcloo_row(&["self", "method m {} {}"]).expect("a member row");
        assert_eq!(block.keyword_index, 0);
        assert_eq!(
            block.effect,
            MemberEffect::InitScript {
                body_slot: 0,
                timing: InitTiming::AtDefinition,
            }
        );
        assert_eq!(block.receiver, MemberReceiver::TypeObject);
        assert_eq!(block.body, Some(OperandId(1)));
    }

    /// `private` declares the wrapped member private on its own side, and the
    /// wrapper's 9.0+ release set reaches the row; the member's own option
    /// word still wins.
    #[test]
    fn member_row_applies_the_private_wrapper() {
        let row = tcloo_row(&["private", "method", "m", "", ""]).expect("a member row");
        assert_eq!(row.receiver, MemberReceiver::Instance);
        assert_eq!(row.visibility, DeclaredMemberVisibility::Private);
        assert_eq!(row.surface, Some(TCL90_MEMBERS));

        let flagged = tcloo_row(&["private", "method", "m", "-export", "", ""]).expect("a row");
        assert_eq!(flagged.visibility, DeclaredMemberVisibility::Public);
        assert_eq!(flagged.body, Some(OperandId(5)));
    }

    /// `superclass -append B` — a `Relation` row carrying the explicit slot
    /// operation; a bare list takes the slot's default.
    #[test]
    fn member_row_reads_a_slot_operation() {
        let row = tcloo_row(&["superclass", "-append", "B"]).expect("a member row");
        assert_eq!(
            row.effect,
            MemberEffect::Relation {
                slot: RelationSlot::Superclass,
            }
        );
        assert_eq!(row.slot_op, Some(SlotOp::Append));
        assert_eq!(row.name, None);
        let bare = tcloo_row(&["superclass", "B"]).expect("a member row");
        assert_eq!(bare.slot_op, Some(SlotOp::Set));
        let bogus = tcloo_row(&["filter", "-bogus", "f"]).expect("a member row");
        assert_eq!(bogus.slot_op, None, "real Tcl aborts the definition");
        let dynamic = [InvocationWord::Literal("mixin"), InvocationWord::Dynamic];
        let computed = TCLOO_GRAMMAR
            .member_row(0, InvocationArguments::structured(&dynamic), None)
            .expect("a member row");
        assert_eq!(
            computed.slot_op, None,
            "a computed word could be an operation"
        );
    }

    /// `forward f ::x` — a `Forward` row named by its first word.
    #[test]
    fn member_row_reads_a_forward() {
        let row = tcloo_row(&["forward", "f", "::x"]).expect("a member row");
        assert_eq!(
            row.effect,
            MemberEffect::Forward {
                name_slot: 0,
                prefix_slot: 1,
            }
        );
        assert_eq!(row.name.as_deref(), Some("f"));
        assert_eq!(row.receiver, MemberReceiver::Instance);
        assert_eq!(row.body, None);
    }

    /// A computed name abstains the name, never the row; a computed word
    /// where the optional flag could stand abstains the whole row, because
    /// the params and body positions depend on it.
    #[test]
    fn member_row_abstains_on_computed_words() {
        let words = [
            InvocationWord::Literal("method"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("a"),
            InvocationWord::Literal("return"),
        ];
        let row = TCLOO_GRAMMAR
            .member_row(0, InvocationArguments::structured(&words), None)
            .expect("a member row");
        assert_eq!(row.name, None);
        assert_eq!(row.body, Some(OperandId(3)));
        assert_eq!(
            row.arity,
            Some(MemberArity {
                required: 1,
                optional: 0,
                variadic: false,
            })
        );

        let flagged = [
            InvocationWord::Literal("method"),
            InvocationWord::Literal("m"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("a"),
            InvocationWord::Literal("return"),
        ];
        assert_eq!(
            TCLOO_GRAMMAR.member_row(0, InvocationArguments::structured(&flagged), None),
            None
        );
        let keyword = [InvocationWord::Dynamic, InvocationWord::Literal("m")];
        assert_eq!(
            TCLOO_GRAMMAR.member_row(0, InvocationArguments::structured(&keyword), None),
            None
        );
    }

    /// The optional flag shifts the params and body, declares the
    /// visibility, and — being 9.0-only — makes the row unreadable under an
    /// 8.6 dialect, where the analyser skips the member.
    #[test]
    fn member_row_reads_the_optional_flag_by_dialect() {
        let words = ["method", "m", "-unexport", "a {b 1} args", "return"];
        let tcl90 = Some(SurfaceQuery::core(Family::Tcl, "9.0"));
        let row = TCLOO_GRAMMAR
            .member_row(0, InvocationArguments::literals(&words), tcl90)
            .expect("a member row");
        assert_eq!(row.visibility, DeclaredMemberVisibility::Unexported);
        assert_eq!(row.body, Some(OperandId(4)));
        assert_eq!(
            row.arity,
            Some(MemberArity {
                required: 1,
                optional: 1,
                variadic: true,
            })
        );
        let tcl86 = Some(SurfaceQuery::core(Family::Tcl, "8.6"));
        assert_eq!(
            TCLOO_GRAMMAR.member_row(0, InvocationArguments::literals(&words), tcl86),
            None
        );
    }

    /// The single-command `oo::define CLASS member …` form: the keyword sits
    /// at 1 and every index the row carries is into the same words.
    #[test]
    fn member_row_indexes_the_statement_words() {
        let words = ["::C", "constructor", "x", "return"];
        let row = TCLOO_GRAMMAR
            .member_row(1, InvocationArguments::literals(&words), None)
            .expect("a member row");
        assert_eq!(row.keyword_index, 1);
        assert_eq!(
            row.effect,
            MemberEffect::Callable {
                receiver: MemberReceiver::Instance,
                role: CallableRole::Constructor,
                name_slot: None,
                params_slot: Some(0),
                body_slot: Some(1),
            }
        );
        assert_eq!(row.name, None);
        assert_eq!(row.body, Some(OperandId(3)));
        assert_eq!(
            TCLOO_GRAMMAR.member_row(0, InvocationArguments::literals(&words), None),
            None
        );
    }

    /// Parameter binding is positional: a defaulted parameter before a
    /// required one is required (tclsh 9.0.4 and 8.6: `proc p {{a 1} b}` →
    /// `wrong # args: should be "p ?a? b"` for one argument).
    #[test]
    fn member_arity_counts_positionally() {
        let arity = |params| MemberArity::parse(params).expect("a parameter list");
        assert_eq!(
            arity("{a 1} b"),
            MemberArity {
                required: 2,
                optional: 0,
                variadic: false,
            }
        );
        assert_eq!(
            arity("a {b 2} {c 3} args"),
            MemberArity {
                required: 1,
                optional: 2,
                variadic: true,
            }
        );
        assert_eq!(
            arity(""),
            MemberArity {
                required: 0,
                optional: 0,
                variadic: false,
            }
        );
        assert_eq!(MemberArity::parse("{a"), None);
        assert_eq!(MemberArity::parse("a::b"), None);
    }

    /// snit and itcl rows: state on both sides for type-level declarations,
    /// the option handlers as callables without a name, itcl's modifiers
    /// declaring visibility.
    #[test]
    fn member_row_reads_snit_and_itcl_members() {
        let snit = |words: &[&str]| {
            SNIT_GRAMMAR
                .member_row(0, InvocationArguments::literals(words), None)
                .expect("a snit member row")
        };
        let typevariable = snit(&["typevariable", "count", "0"]);
        assert_eq!(
            typevariable.effect,
            MemberEffect::StateDeclaration {
                scope: StateScope::PerType,
            }
        );
        assert_eq!(typevariable.receiver, MemberReceiver::Both);
        assert_eq!(typevariable.name.as_deref(), Some("count"));
        let onconfigure = snit(&["onconfigure", "-colour", "value", "set x $value"]);
        assert_eq!(onconfigure.name, None);
        assert_eq!(onconfigure.body, Some(OperandId(3)));
        let typeconstructor = snit(&["typeconstructor", "init"]);
        assert_eq!(typeconstructor.receiver, MemberReceiver::TypeObject);
        assert_eq!(typeconstructor.body, Some(OperandId(1)));
        assert_eq!(snit(&["option", "-colour"]).receiver, MemberReceiver::Both);

        let itcl = |words: &[&str]| {
            ITCL_GRAMMAR
                .member_row(0, InvocationArguments::literals(words), None)
                .expect("an itcl member row")
        };
        let protected = itcl(&["protected", "method", "m", "", ""]);
        assert_eq!(protected.visibility, DeclaredMemberVisibility::Unexported);
        assert_eq!(protected.keyword_index, 1);
        let common = itcl(&["public", "common", "shared"]);
        assert_eq!(common.receiver, MemberReceiver::Both);
        assert_eq!(common.visibility, DeclaredMemberVisibility::Public);
        assert_eq!(
            ITCL_GRAMMAR.member_row(0, InvocationArguments::literals(&["public", "{x}"]), None),
            None,
            "itcl's modifiers have no block form"
        );
    }
}
