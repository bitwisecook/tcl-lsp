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

mod object_system_profile;
pub use object_system_profile::{
    CLAY_PROFILE, DECLARATION_PROFILE, DefinitionShape, ITCL_PROFILE, JIM_PROFILE,
    MemberExportRule, MemberStorage, ObjectSystemProfile, SNIT_PROFILE, TCLOO_PROFILE,
};
mod clay;
mod jim_source_stock;
pub use clay::CLAY_GRAMMAR;
pub use jim_source_stock::{
    SourceJimClassHelper, SourceJimObjectCommandFootprint, SourceJimStockClassRecipe,
};

use crate::arg_role::ArgRole;
use crate::value_transfer::inputs::OperandId;
#[cfg(test)]
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

/// Registry-selected nameless lifecycle declaration in a definition grammar.
/// Its keyword is source syntax, never a user-supplied method name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionSpecialMemberKind {
    /// Constructor declaration; execution and allocation remain independent.
    Constructor,
    /// Destructor declaration; teardown dispatch remains independent.
    Destructor,
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
        let (retracted, arrives_at) = self.argument_indices(args.len());
        RetractionWords {
            retracted: &args[retracted],
            arrives_at,
        }
    }

    /// Original argv positions affected by this operation. Counted byte and
    /// authored text consumers share the same Registry-owned argument shape.
    #[must_use]
    pub fn argument_indices(self, count: usize) -> (std::ops::Range<usize>, Option<usize>) {
        match self {
            Self::EveryArgument => (0..count, None),
            Self::FirstArgument => (0..count.min(1), (count > 1).then_some(1)),
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
        Procedure => "procedure",
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

mod builtin_methods;
mod command_installation;
mod itcl;
mod member_arguments;
mod member_effect;
mod native_declarations;
mod property_accessors;
mod slots;
mod snit;
mod spectcl;
mod sslictcl;
mod tcloo;

pub use builtin_methods::{
    BuiltinMethodReceiver, BuiltinObjectMethod, BuiltinObjectMethodOperation, MethodReach,
};
pub use command_installation::{
    BareWordConstructionSourceHint, ConstructionNameResult, DefinitionCommandInstallation,
    DefinitionDispatcher, ManufacturerMethod, SourceFactoryInitialiserPresence,
    SourceTwoWordFactoryInitialiser,
};
pub use itcl::ITCL_GRAMMAR;
pub use member_arguments::{
    SourceDefinitionMemberBodyPresentation, SourceDefinitionMemberScriptArgument,
};
pub use member_effect::{
    CallableRole, DeclaredMemberVisibility, InitTiming, MemberEffect, MemberReceiver,
    MemberVisibility, RelationSlot, StateScope,
};
pub use native_declarations::{
    DefinitionMemberLookup, DefinitionReceiver, MemberConstructionEffect,
    MemberManufactureDispatchEffect, NativeConstructorBoundary, NativeDeferredMethodSetter,
    NativeInheritedClassKind, NativeLifecycleBodyDisposition, ReceiverMethodLayout,
    SourceFormalValidationApplicability,
};
pub use property_accessors::{
    SourcePropertyAccessorAdvice, SourcePropertyAccessorLayout, SourcePropertyAccessorMode,
    SourcePropertyCompletionRole, SourcePropertyDeclaration,
};
pub use slots::{SlotOp, SlotSpec};
pub use snit::{SNIT_GRAMMAR, SNIT_WIDGET_GRAMMAR};
pub use spectcl::{
    SPECTCL_BODY_SCOPE_GRAMMAR, SPECTCL_CASE_LIST_GRAMMAR, SPECTCL_CLAUSE_GRAMMAR_GRAMMAR,
    SPECTCL_COMMAND_GRAMMAR, SPECTCL_DEFINITION_BODY_GRAMMAR, SPECTCL_DOCUMENT_GRAMMAR,
    SPECTCL_EVENT_REQUIRES_GRAMMAR, SPECTCL_GRAMMARS, SPECTCL_HOVER_GRAMMAR,
    SPECTCL_OBJECT_CLASS_GRAMMAR, SPECTCL_PACK_GRAMMAR, SPECTCL_STATE_TRANSITIONS_GRAMMAR,
    SPECTCL_VALUES_GRAMMAR, SPECTCL_WORLD_EFFECTS_GRAMMAR,
};
pub use sslictcl::{
    SSLICTCL_ANCHOR_GRAMMAR, SSLICTCL_CERTIFICATE_GRAMMAR, SSLICTCL_CHAIN_GRAMMAR,
    SSLICTCL_CHECK_GRAMMAR, SSLICTCL_CIPHER_GRAMMAR, SSLICTCL_DOCUMENT_GRAMMAR,
    SSLICTCL_ENDPOINT_GRAMMAR, SSLICTCL_GRADE_GRAMMAR, SSLICTCL_GRAMMARS, SSLICTCL_HSTS_GRAMMAR,
    SSLICTCL_POLICY_GRAMMAR, SSLICTCL_PROTOCOL_GRAMMAR, SSLICTCL_TESTSSL_IMPORT_GRAMMAR,
    SSLICTCL_TRUST_PROGRAM_GRAMMAR,
};
#[cfg(test)]
use tcloo::TCL90_MEMBERS;
use tcloo::TCLOO_METHOD_VISIBILITY_ARGUMENT;
pub use tcloo::{
    TCLOO_ABSTRACT_GRAMMAR, TCLOO_CONFIGURABLE_GRAMMAR, TCLOO_CREATE_MANUFACTURER,
    TCLOO_CREATE_WITH_NAMESPACE_MANUFACTURER, TCLOO_DERIVED_METACLASS_MANUFACTURERS, TCLOO_GRAMMAR,
    TCLOO_MEMBER_BODY_NAMESPACE_PATH, TCLOO_NEW_MANUFACTURER, TCLOO_RECEIVER_NAMESPACE_COMMANDS,
    TCLOO_ROOT_CLASS_MANUFACTURERS, TCLOO_ROOT_NEW_MANUFACTURER,
};

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
        let word = args.get(usize::from(self.position))?.as_ref().as_bytes();
        self.value_for_word_in(word, dialect)
    }

    fn value_for_word_in(
        self,
        word: &[u8],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<MemberOptionValue> {
        self.values.iter().copied().find(|candidate| {
            candidate.value.as_bytes() == word
                && candidate
                    .surface
                    .is_none_or(|available| surface_admits(available, dialect.as_ref()))
        })
    }

    /// Select an available optional word through the byte-aware owner.
    fn value_at(self, word: &str, dialect: Option<SurfaceQuery<'_>>) -> Option<MemberOptionValue> {
        self.value_for_word_in(word.as_bytes(), dialect)
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
}

/// Display and grouping identity of an authored definition grammar.
/// Semantic consumers read [`DefinitionBodyGrammar::profile`] instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefinerFamily {
    /// `TclOO` metaclasses and the `oo::define` / `oo::objdefine` script form.
    TclOo,
    /// snit `type` / `widget` / `widgetadaptor`.
    Snit,
    /// [incr Tcl] `itcl::class` (and the bare `class` alias).
    Itcl,
    /// Clay's conditional source class inventory. Its provider may create or
    /// extend a class; this family supplies no installed dispatcher receipt.
    Clay,
    /// Jim Tcl's `class NAME ?BASES? {vars}` (`oo.tcl`, shipped with every
    /// `jimsh`).
    ///
    /// The class body is a dictionary of the instance variables and their
    /// initial values, not a script of member declarations, so the definer
    /// declares no `Body` word. Members arrive afterwards, as
    /// `NAME method M ARGS BODY` or as a raw `proc {NAME M} ARGS BODY`: a
    /// member is stored as the two-word command `{NAME M}`, and an object is a
    /// command that dispatches to whichever `{NAME word}` exists — so the
    /// member set is open at run time ([`DefinitionBodyGrammar::dynamic_method_dispatch`]).
    /// The grammar's `members` name the class-command calls that declare one
    /// ([`ObjectSystemProfile::members_are_two_word_commands`]).
    JimClass,
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

/// Naming axis retained by source definition metadata. Generic provider
/// values do not borrow `TclOO` method registration or lookup authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionMemberNamePurpose {
    /// Counted native `TclOO` method-name input, on its supported engines.
    TclOoMethod,
    /// Original produced value used only as a source metadata key. Runtime
    /// storage, dispatch and provider-specific transformations remain unknown.
    SourceValue,
}

impl DefinitionMemberNamePurpose {
    /// Admit this source naming axis under its independently selected recipe.
    #[must_use]
    pub fn admits(self, protocol: tcl_syntax::naming::NativeNameProtocol, bytes: &[u8]) -> bool {
        match self {
            Self::TclOoMethod => protocol.oo_method_input(bytes).is_ok(),
            Self::SourceValue => true,
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
    /// Independent semantic facts selected by this authored grammar.
    pub profile: ObjectSystemProfile,
    /// Recognised member sub-keywords.
    pub members: &'static [MemberSpec],
    /// Variables implicitly available in every member body (snit's `self` /
    /// `type` / `selfns` / `options`, a widget's `win` / `hull`).  Consumed by
    /// the analyser's read-before-set / stray-dispatch suppression.
    pub implicit_vars: &'static [&'static str],
    /// Implicit variables whose value is the receiving object in instance bodies.
    /// Consumers independently select the actual body and receiver context.
    pub implicit_receiver_vars: &'static [&'static str],
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
    /// Actual authored source hint, retained separately from legacy callbacks.
    pub bare_word_construction_source_hint: Option<BareWordConstructionSourceHint>,

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
    /// Receiver vocabulary requires both an implicit binding and its explicit role.
    #[must_use]
    pub fn is_implicit_receiver_variable(&self, name: &str) -> bool {
        self.implicit_vars.contains(&name) && self.implicit_receiver_vars.contains(&name)
    }

    /// Selected root-definer operand containing a list of base-class names.
    /// The Jim three-operand form has a base list between name and dictionary;
    /// other families express relations through their member declarations.
    /// This source layout grants no class existence, dispatch or inheritance.
    #[must_use]
    pub const fn source_base_class_list_argument(&self, argument_count: usize) -> Option<usize> {
        match self.profile.source_base_class_list {
            Some((count, index)) if argument_count == count => Some(index),
            _ => None,
        }
    }

    /// Readonly member-name purpose for genuine two-word procedure declarations.
    /// The selected family stores these procedures as source members directly;
    /// this is separate from an entered member-definition worker or method table.
    #[must_use]
    pub const fn two_word_procedure_member_name_purpose(
        &self,
    ) -> Option<DefinitionMemberNamePurpose> {
        if self.profile.members_are_two_word_commands() {
            Some(DefinitionMemberNamePurpose::SourceValue)
        } else {
            None
        }
    }

    /// Source declaration role of a selected nameless body member. This
    /// metadata does not prove installation, absence or lifecycle dispatch.
    #[must_use]
    pub fn source_special_member_kind(
        &self,
        member: &MemberSpec,
    ) -> Option<DefinitionSpecialMemberKind> {
        if !self
            .member(member.keyword)
            .is_some_and(|known| std::ptr::eq(known, member))
            || !member
                .arg_roles
                .iter()
                .any(|(_, role)| *role == ArgRole::Body)
            || member
                .arg_roles
                .iter()
                .any(|(_, role)| *role == ArgRole::Name)
        {
            return None;
        }
        match member.keyword {
            "constructor" => Some(DefinitionSpecialMemberKind::Constructor),
            "destructor" => Some(DefinitionSpecialMemberKind::Destructor),
            _ => None,
        }
    }

    /// Naming purpose of one independently selected declaration or name
    /// effect. Membership in another grammar cannot supply this selection.
    #[must_use]
    pub fn member_name_purpose(&self, member: &MemberSpec) -> Option<DefinitionMemberNamePurpose> {
        if !self
            .member(member.keyword)
            .is_some_and(|known| std::ptr::eq(known, member))
            || !(member
                .arg_roles
                .iter()
                .any(|(_, role)| *role == crate::ArgRole::Name)
                || member.all_args_ref == Some(MemberRefKind::Method)
                || member.retraction.is_some()
                || member.visibility_effect.is_some())
        {
            return None;
        }
        Some(if member.effect == MemberEffect::Configuration {
            self.profile.configuration_name_purpose
        } else {
            self.profile.member_name_purpose
        })
    }

    /// Current-namespace policy for executable members of this grammar.
    #[must_use]
    pub const fn member_current_namespace(&self) -> MemberCurrentNamespace {
        self.profile.current_namespace
    }

    /// Root object dispatch dependencies of an audited default constructor.
    /// Configuration of either object revokes the default construction proof.
    #[must_use]
    pub const fn default_construction_lookup_dependencies(
        &self,
    ) -> Option<&'static [&'static str]> {
        if self.profile.default_construction_dependencies.is_empty() {
            None
        } else {
            Some(self.profile.default_construction_dependencies)
        }
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
        self.member_default_exported_bytes(name.as_bytes())
    }

    /// Name-based declaration visibility from the original counted name.
    /// `TclOO`'s native `[a-z]*` `CString` matcher accepts precisely an initial
    /// ASCII lowercase byte; subsequent opaque units do not alter that test.
    /// This is a visibility rule, separate from member publication/lookup.
    #[must_use]
    pub fn member_default_exported_bytes(&self, name: &[u8]) -> bool {
        match self.profile.export_rule {
            MemberExportRule::AsciiLowercase => name.first().is_some_and(u8::is_ascii_lowercase),
            MemberExportRule::All => true,
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

/// `method NAME PARAMS BODY` on the instances.
const INSTANCE_METHOD: MemberEffect =
    callable(MemberReceiver::Instance, CallableRole::Method, METHOD_SLOTS);
/// `classmethod` / `typemethod` / itcl's class-scoped `proc NAME PARAMS BODY`,
/// on the class or type object.
const TYPE_METHOD: MemberEffect = callable(
    MemberReceiver::TypeObject,
    CallableRole::Method,
    METHOD_SLOTS,
);
/// snit's `proc NAME PARAMS BODY`: a procedure in the type's namespace that
/// sees the type's state and is reached by name, never by dispatch
/// ([`CallableRole::Procedure`]).
const TYPE_PROCEDURE: MemberEffect = callable(
    MemberReceiver::TypeObject,
    CallableRole::Procedure,
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_property_accessor_tests;

#[cfg(test)]
mod original_forward_tests;

#[cfg(test)]
mod original_member_presentation_tests;

#[cfg(test)]
mod implicit_receiver_variable_tests;
