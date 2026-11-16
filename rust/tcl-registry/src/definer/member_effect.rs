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

//! What a member declares: visibility, the effect vocabulary and its receivers, roles and scopes.

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
    /// `constructor`, `destructor`, snit's `onconfigure` / `oncget` and
    /// namespace `proc`, itcl's class-scoped `proc`.
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
    /// A procedure in the definition's own namespace, called by name from the
    /// definition's code and never dispatched through the type or instance
    /// command (snit's `proc`). Its receiver is the side whose state its body
    /// sees.
    ///
    /// snit 2.3.4 (tcllib 2.0) on tclsh 8.6.18 and 9.0.4 alike: `snit::type
    /// ::app::Dog { typevariable count 5; proc helper {a} { expr {$a + $count}
    /// } }` defines the command `::app::Dog::helper` (`::app::Dog::helper 10`
    /// → `15`), while `::app::Dog helper 1` is a construction (`Error in
    /// constructor: …`) and an instance's `helper` is `unknown subcommand
    /// "helper"` — no dispatch side reaches it.
    Procedure,
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
