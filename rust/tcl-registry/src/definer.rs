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
    /// Whether this member **removes** the members its arguments name, rather
    /// than merely referring to them.
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
    pub retracts_named_members: bool,
}

impl MemberSpec {
    /// An ordinary [`MemberKind::Flat`] member.
    #[must_use]
    const fn flat(keyword: &'static str, arg_roles: &'static [(u8, ArgRole)]) -> Self {
        Self {
            keyword,
            arg_roles,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            dialects: None,
            retracts_named_members: false,
        }
    }

    /// A member whose every argument references an entity of `kind`
    /// (`superclass A B`, `export m`).
    #[must_use]
    const fn all_refs(keyword: &'static str, kind: MemberRefKind) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            all_args_var: false,
            all_args_ref: Some(kind),
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            dialects: None,
            retracts_named_members: false,
        }
    }

    /// A `variable a b c`-style member: every argument is a declared name.
    #[must_use]
    const fn all_vars(keyword: &'static str) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            all_args_var: true,
            all_args_ref: None,
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            dialects: None,
            retracts_named_members: false,
        }
    }

    /// A name-reference / keyword-only member carrying nothing to recurse or
    /// declare (`superclass A B`, `inherit Base`, `option …`).
    #[must_use]
    const fn keyword_only(keyword: &'static str) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Flat,
            wrapper_block_body: false,
            dialects: None,
            retracts_named_members: false,
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
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Wrapper,
            wrapper_block_body: false,
            dialects: None,
            retracts_named_members: false,
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
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::Wrapper,
            wrapper_block_body: true,
            dialects: None,
            retracts_named_members: false,
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
    /// name (a builder over the constructors above) — see
    /// [`Self::retracts_named_members`].
    #[must_use]
    const fn retracting(mut self) -> Self {
        self.retracts_named_members = true;
        self
    }

    /// A [`MemberKind::FlagKeyed`] member (`property`).
    #[must_use]
    const fn flag_keyed(keyword: &'static str) -> Self {
        Self {
            keyword,
            arg_roles: NO_ROLES,
            all_args_var: false,
            all_args_ref: None,
            kind: MemberKind::FlagKeyed,
            wrapper_block_body: false,
            dialects: None,
            retracts_named_members: false,
        }
    }

    /// The argument indices (0-based after the keyword) carrying `role`.
    pub fn indices_for(&self, role: ArgRole) -> impl Iterator<Item = usize> + '_ {
        self.arg_roles
            .iter()
            .filter(move |(_, r)| *r == role)
            .map(|(i, _)| *i as usize)
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
}

impl DefinitionBodyGrammar {
    /// The member grammar for `keyword`, if it is a recognised member.
    #[must_use]
    pub fn member(&self, keyword: &str) -> Option<&'static MemberSpec> {
        // `members` is `&'static`, so the borrow can be handed back as static.
        let idx = self.members.iter().position(|m| m.keyword == keyword)?;
        Some(&self.members[idx])
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
            DefinerFamily::Snit | DefinerFamily::Itcl => true,
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
    MemberSpec::flat("method", METHOD_ROLES),
    MemberSpec::flat("classmethod", METHOD_ROLES).with_dialects(TCL90_MEMBERS),
    MemberSpec::flat("constructor", CTOR_ROLES),
    MemberSpec::flat("destructor", BODY0_ROLES),
    MemberSpec::flat("initialise", BODY0_ROLES).with_dialects(TCL90_MEMBERS),
    MemberSpec::flat("initialize", BODY0_ROLES).with_dialects(TCL90_MEMBERS),
    // `private` is a prefix wrapper (`private method m {} {…}`, `private
    // variable x`) *and* a bare definition-script block (`private { … }`).
    MemberSpec::wrapper_or_body("private").with_dialects(TCL90_MEMBERS),
    // `variable a b c` inside a class body declares every name.
    MemberSpec::all_vars("variable"),
    // Reference-only members: they declare nothing and recurse nothing, but
    // their arguments *name* an entity defined elsewhere — a class or a method
    // — so they are references, not free strings.
    MemberSpec::all_refs("superclass", MemberRefKind::Class),
    MemberSpec::all_refs("mixin", MemberRefKind::Class),
    MemberSpec::all_refs("filter", MemberRefKind::Method),
    MemberSpec::all_refs("export", MemberRefKind::Method),
    MemberSpec::all_refs("unexport", MemberRefKind::Method),
    MemberSpec::all_refs("deletemethod", MemberRefKind::Method).retracting(),
    // `forward NAME cmd ?arg…?` declares NAME as a method; the word after it
    // (`cmd`) is the delegated command's name — a first-class command
    // reference the walker records so navigation reaches it, exactly like the
    // command a `superclass`/`mixin` names.  Any baked arguments after it are
    // ordinary values.
    MemberSpec::flat("forward", FORWARD_ROLES),
    // `renamemethod FROM TO` — both name methods.
    // Both words name methods; the FROM word is retracted (and the TO word is
    // a member this walker does not record), so the whole call retracts.
    MemberSpec::all_refs("renamemethod", MemberRefKind::Method).retracting(),
    MemberSpec::keyword_only("definitionnamespace").with_dialects(TCL90_MEMBERS),
    // Structurally irregular — a nested-member wrapper (`self method …`) and a
    // flag-keyed body form (`property … -get/-set …`); their body indices come
    // from the walker's `MemberKind`-driven handling, not a hardcoded name.
    MemberSpec::wrapper_or_body("self"),
    // `property` (and its configurable-class accessor machinery) is a 9.0
    // addition; the 8.6 `TclOO` definition grammar has no such member.
    MemberSpec::flag_keyed("property").with_dialects(TCL90_MEMBERS),
];

/// The definition-body grammar for every `TclOO` metaclass and the bare
/// `oo::define` / `oo::objdefine` script form.
pub const TCLOO_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::TclOo,
    members: TCLOO_MEMBERS,
    implicit_vars: &[],
};

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

/// The definition-body grammar for snit `type` / `widget` / `widgetadaptor`.
/// `implicit_vars` is the set snit injects into *every* member body; a
/// widget's extra `win` / `hull` are added by the analyser only for the widget
/// definers (they are not implicit in a plain `snit::type`), so they are not
/// listed in this shared grammar.
pub const SNIT_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Snit,
    members: SNIT_MEMBERS,
    implicit_vars: &["self", "selfns", "type", "options"],
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

/// The definition-body grammar for [incr Tcl] `itcl::class` / bare `class`.
/// Member bodies run in the object's context with the instance/common
/// variables and `this` in scope.
pub const ITCL_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Itcl,
    members: ITCL_MEMBERS,
    implicit_vars: &["this"],
};
