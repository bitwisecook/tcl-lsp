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

//! `TclOO` class / method body parsing + unknown-proc detection.
//!
//! Walks the body of an ``oo::class create Name { ... }`` or
//! ``oo::define Name { ... }`` block (and a snit type's or an itcl class's)
//! and populates the [`super::types::ClassDef`] fields: the full field set
//! (``constructors``, ``destructor``, ``variables``,
//! ``properties``, ``filters``, ``exports``, ``unexports``) plus
//! [`Analyser::extract_unknown_proc_info`] — the W123 gating
//! analysis for user-defined ``unknown`` procs.
//!
//! Every member statement is read through its registry row
//! (`DefinitionBodyGrammar::member_row`) and lands by its effect and the side
//! its receiver resolves to, never by its keyword (`member_landing`):
//!
//! - a callable — in the table `MethodKind::from_effect` names: an instance
//!   method in ``ClassDef::methods``, a type-object one in
//!   ``ClassDef::class_methods``, a constructor appended to
//!   ``ClassDef::constructors`` under its synthetic `<keyword>` name, a
//!   destructor in ``ClassDef::destructor``; snit's option handlers and
//!   `typeconstructor` among the methods of their side; snit's namespace
//!   `proc` through the ordinary proc handling;
//! - a forward — ``methods`` with ``kind = "forward"``;
//! - per-instance state — folds into ``ClassDef::variables`` through the
//!   registry slot spec (``-append`` with dedup for `TclOO`'s `variable`);
//! - a relation — the superclass and mixin slots (default ``-set``) and each
//!   side's filter slot (default ``-append``, duplicates kept);
//! - a retraction or visibility word — the members it names, on its side;
//! - flag-keyed accessors (``property NAME ?-get BODY? ?-set BODY? ?-kind
//!   K?``) — a [`super::types::PropertyDef`] per name;
//! - a definition-time script (``initialise`` / ``initialize``) — its body is
//!   walked in a class-keyed scope for variable tracking.

use tcl_dialect::model::SurfaceQuery;
#[cfg(test)]
use tcl_lexer::Span;
use tcl_lexer::{Token, TokenType};
use tcl_registry::definer::{
    CallableRole, DefinitionBodyGrammar, InitTiming, MemberCurrentNamespace, MemberEffect,
    MemberKind, MemberReceiver, MemberRow, MemberSpec, RelationSlot, StateScope, WrapperShift,
};
use tcl_registry::{InvocationArguments, InvocationWord};
use tcl_syntax::word_rules::WordValueRules;

use super::diagnostics::helpers::has_substitution;
#[cfg(test)]
use super::state::Analyser;
use super::types::{ClassDef, MemberSide, MethodDef};
#[cfg(test)]
use super::types::{DefinitionAbort, DefinitionAbortKind, Scope, UnknownProcInfo};
use crate::ir::MethodKind;
#[cfg(test)]
use crate::ir::Statement;
use crate::signature_scan::types::ParamDef;

mod definition_body;
mod itcl;
mod jim_classes;
mod member_effects;
mod member_references;
mod method_bodies;
mod retained_member_source;
mod snit;
mod unknown_proc;

#[cfg(test)]
use definition_body::collect_var_decl_spans;
#[cfg(test)]
use definition_body::expand_wrapper_block_members;
pub(super) use definition_body::parse_oo_define_inline_in;
#[cfg(test)]
pub(super) use member_effects::apply_oo_subcommand;
#[cfg(test)]
use member_effects::extract_method_def;
pub(super) use member_effects::{apply_oo_subcommand_in, extract_method_def_in};
#[cfg(test)]
use member_references::loop_installed_member_shape;
pub(super) use member_references::loop_installer_pair;
#[cfg(test)]
use unknown_proc::walk_unknown_stmt;

struct ClassBodyCtx<'a> {
    grammar: &'static DefinitionBodyGrammar,
    class_def: &'a mut ClassDef,
    class_qualified: &'a str,
    scope_path: &'a [usize],
}

/// A body-bearing member ready to extract: its grammar spec (argument layout),
/// the table its row lands in, and the synthetic name for the nameless forms.
/// Bundled to keep [`Analyser::extract_class_member`] under the argument limit.
struct MemberForm<'a> {
    member: &'a MemberSpec,
    kind: MethodKind,
    label: &'a str,
    /// The member's declared visibility — its row's: the family's name rule
    /// (every snit and itcl member is exported), or the visibility an itcl
    /// access modifier's shift declares.
    visibility: &'a str,
}

/// Strip a leading member wrapper (a registry [`MemberKind::Wrapper`] — itcl's
/// `public` / `protected` / `private` access modifiers, `TclOO`'s `self`) from a
/// member call, returning the effective member keyword and its argument texts
/// and tokens (the words *after* the keyword).  Returns `None` for an empty
/// command or a bare wrapper with no inner member keyword (a wrapper's bare
/// script-block form, `self { … }`, has no inner member).
fn unwrap_wrapper_member<'a>(
    grammar: &DefinitionBodyGrammar,
    texts: &'a [String],
    argv: &'a [Token],
) -> Option<(&'a str, &'a [String], &'a [Token])> {
    let (first, rest_texts) = texts.split_first()?;
    let rest_toks = argv.get(1..).unwrap_or(&[]);
    if grammar
        .member(first)
        .is_some_and(|m| m.kind == MemberKind::Wrapper)
    {
        // `<modifier> <member> args…` — the inner member follows.
        let (inner, inner_texts) = rest_texts.split_first()?;
        let inner_toks = rest_toks.get(1..).unwrap_or(&[]);
        Some((inner.as_str(), inner_texts, inner_toks))
    } else {
        Some((first.as_str(), rest_texts, rest_toks))
    }
}

/// The words of one definition-body member statement as the registry reads
/// them: a braced word, or one with no substitution, is the literal it spells;
/// anything else is a computed word ([`has_substitution`], the boundary this
/// walker's other static-word checks draw).
fn member_statement_words<'t>(texts: &'t [String], argv: &[Token]) -> Vec<InvocationWord<'t>> {
    texts
        .iter()
        .enumerate()
        .map(|(index, text)| match argv.get(index) {
            Some(tok) if tok.kind == TokenType::Str || !has_substitution(text, tok) => {
                InvocationWord::Literal(text)
            }
            Some(_) => InvocationWord::Dynamic,
            None => InvocationWord::Opaque,
        })
        .collect()
}

/// One definition-body member statement read through its registry
/// [`MemberRow`]: the row, the member spec at its keyword (the wrapped
/// member's own for a wrapper's prefix form), and the statement's words.
struct MemberStatement<'a> {
    grammar: &'a DefinitionBodyGrammar,
    row: MemberRow,
    member: &'static MemberSpec,
    texts: &'a [String],
    argv: &'a [Token],
}

impl<'a> MemberStatement<'a> {
    /// The statement `texts` / `argv` spell, read under `dialect`; `None`
    /// when the registry cannot read it — an unknown or computed keyword, a
    /// wrapper around nothing it recognises, a recognised option word the
    /// dialect lacks, or a computed word where an optional one may stand.
    fn read(
        grammar: &'a DefinitionBodyGrammar,
        texts: &'a [String],
        argv: &'a [Token],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<Self> {
        let words = member_statement_words(texts, argv);
        let row = grammar.member_row(0, InvocationArguments::structured(&words), dialect)?;
        let member = grammar.member(texts.get(row.keyword_index)?)?;
        Some(Self {
            grammar,
            row,
            member,
            texts,
            argv,
        })
    }

    /// The member keyword as written.
    fn keyword(&self) -> &'a str {
        &self.texts[self.row.keyword_index]
    }

    /// The member keyword's token.
    fn keyword_token(&self) -> Option<Token> {
        self.argv.get(self.row.keyword_index).copied()
    }

    /// The words after the member keyword.
    fn args(&self) -> &'a [String] {
        self.texts.get(self.row.keyword_index + 1..).unwrap_or(&[])
    }

    /// The tokens of [`Self::args`], word for word.
    fn arg_tokens(&self) -> &'a [Token] {
        self.argv.get(self.row.keyword_index + 1..).unwrap_or(&[])
    }

    /// Where the row lands in the class model.
    fn landing(&self) -> MemberLanding {
        member_landing(self.grammar, self.member, &self.row)
    }

    /// What the wrappers written before the member keyword do to it,
    /// innermost first ([`WrapperShift::within`]) — the shift the row's
    /// receiver and visibility already include.
    fn shift(&self) -> WrapperShift {
        self.texts[..self.row.keyword_index]
            .iter()
            .fold(WrapperShift::NONE, |outer, word| {
                self.grammar
                    .member(word)
                    .and_then(|wrapper| wrapper.wrapper_shift)
                    .map_or(outer, |inner| inner.within(outer))
            })
    }

    /// The visibility a declared member is recorded with.
    ///
    /// A literal name takes the row's answer — its option word, then its
    /// wrapper, then the family's name rule. A computed name is still recorded
    /// under its written spelling (the class outline shows `${m}`), so the
    /// same three steps read that spelling: `${m}` fails `TclOO`'s `[a-z]*`
    /// rule, and is unexported unless its option word or wrapper says
    /// otherwise.
    fn recorded_visibility(&self, written_name: &str, dialect: Option<SurfaceQuery<'_>>) -> String {
        let named = matches!(
            self.row.effect,
            MemberEffect::Callable {
                name_slot: Some(_),
                ..
            } | MemberEffect::Forward { .. }
        );
        if self.row.name.is_some() || !named {
            return self.row.visibility.as_str().to_string();
        }
        let fallback = self.shift().visibility.map_or_else(
            || default_visibility(self.grammar, written_name),
            |visibility| visibility.as_str().to_string(),
        );
        declared_member_visibility(self.member, self.args(), fallback, dialect)
    }
}

/// Where one member row lands in the class model — the one `match` on
/// [`MemberEffect`] the `TclOO` fold and the snit and itcl body walkers share,
/// so no walker routes a member by its keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberLanding {
    /// A body recorded as a [`MethodDef`] in the table this kind names.
    Method(MethodKind),
    /// A procedure in the definition's own namespace (snit's `proc`): an
    /// ordinary proc, not a member.
    Procedure,
    /// A definition-time script no member table models (`TclOO`'s
    /// `initialise`, walked as the class's own init script, and a wrapper's
    /// block form, whose members are read one by one).
    InitScript,
    /// A forwarded method.
    Forward,
    /// Declared state.
    State(StateScope),
    /// A contribution to an ancestry or interposition slot.
    Relation(RelationSlot),
    /// Removes the members it names.
    Retraction,
    /// Changes the visibility of the members it names.
    Visibility,
    /// Flag-keyed accessor declarations (`property … ?-get …? ?-set …?`),
    /// until they are `Callable` rows of their own.
    Accessors,
    /// Configures the definition and records nothing.
    Nothing,
}

/// Where `row` — a statement of `member` under `grammar` — lands.
///
/// A callable lands in the table [`MethodKind::from_effect`] names; an
/// option's read or write handler opens no method frame of its own (the IR
/// lifts none), but the class model keeps its body, under its synthetic
/// `<keyword -option>` label, on the side that holds the option. A script run
/// once at definition is a class-side body when the family runs member bodies
/// in the defined entity's own namespace
/// ([`MemberCurrentNamespace::DefinedEntity`]: snit's `typeconstructor` is the
/// proc `${type}::Snit_typeconstructor`, run with `type` bound); under
/// `TclOO`'s runtime-receiver policy it runs in the class object's own
/// namespace, and the walker reads it as the class's init script.
fn member_landing(
    grammar: &DefinitionBodyGrammar,
    member: &MemberSpec,
    row: &MemberRow,
) -> MemberLanding {
    match row.effect {
        MemberEffect::Callable {
            role: CallableRole::Procedure,
            ..
        } => MemberLanding::Procedure,
        MemberEffect::Callable {
            role: CallableRole::Accessor | CallableRole::Mutator,
            ..
        } => MemberLanding::Method(side_kind(row.receiver)),
        MemberEffect::Callable { role, .. } => MethodKind::from_effect(role, row.receiver)
            .map_or(MemberLanding::Nothing, MemberLanding::Method),
        MemberEffect::InitScript {
            timing: InitTiming::AtDefinition,
            ..
        } if member.kind == MemberKind::Flat
            && grammar.member_current_namespace() == MemberCurrentNamespace::DefinedEntity =>
        {
            MemberLanding::Method(side_kind(row.receiver))
        }
        MemberEffect::InitScript { .. } => MemberLanding::InitScript,
        MemberEffect::Forward { .. } => MemberLanding::Forward,
        MemberEffect::StateDeclaration { scope } => MemberLanding::State(scope),
        MemberEffect::Relation { slot } => MemberLanding::Relation(slot),
        MemberEffect::Retraction => MemberLanding::Retraction,
        MemberEffect::Visibility => MemberLanding::Visibility,
        MemberEffect::Configuration if member.kind == MemberKind::FlagKeyed => {
            MemberLanding::Accessors
        }
        MemberEffect::Configuration => MemberLanding::Nothing,
    }
}

/// The method table a body with no lifecycle role of its own lands in: the
/// class object's for a type-object row, the instances' otherwise.
const fn side_kind(receiver: MemberReceiver) -> MethodKind {
    match receiver {
        MemberReceiver::TypeObject => MethodKind::ClassMethod,
        MemberReceiver::Instance | MemberReceiver::Both => MethodKind::Method,
    }
}

/// The method table a row's resolved receiver names, or `None` for a row on
/// both sides at once, which no member table models.
const fn member_side(receiver: MemberReceiver) -> Option<MemberSide> {
    match receiver {
        MemberReceiver::Instance => Some(MemberSide::Instance),
        MemberReceiver::TypeObject => Some(MemberSide::ClassObject),
        MemberReceiver::Both => None,
    }
}

/// File a recorded member under the table its kind names.
fn record_method(class_def: &mut ClassDef, kind: MethodKind, md: MethodDef) {
    match kind {
        MethodKind::Method => {
            class_def.methods.insert(md.name.clone(), md);
        }
        MethodKind::ClassMethod => {
            class_def.class_methods.insert(md.name.clone(), md);
        }
        MethodKind::Constructor => class_def.constructors.push(md),
        MethodKind::Destructor => class_def.destructor = Some(md),
    }
}

/// The synthetic name for a nameless member (one with no [`ArgRole::Name`]):
/// `<keyword>`, with a leading roleless option word (snit 1.x
/// `onconfigure`/`oncget`'s `-option`) appended when present so the two
/// option handlers for the same keyword stay distinct (`<onconfigure -foo>` vs
/// `<onconfigure -bar>`).  Unused for named members.
fn member_label(member: &MemberSpec, keyword: &str, args: &[String]) -> String {
    let role_at_zero = member.arg_roles.iter().any(|(i, _)| *i == 0);
    if !role_at_zero && let Some(opt) = args.first() {
        return format!("<{keyword} {opt}>");
    }
    format!("<{keyword}>")
}

/// Walk an inline `oo::define Class subcmd ...` form using the selected
/// dialect's member layout. A recognised member option from another dialect
/// is not reinterpreted using the older positional layout.
/// The two dialect facts extracting a member definition needs, carried
/// together because they are read from the same document: the availability
/// point that resolves the member's argument roles, and the word-value rules
/// that split its parameter list.
#[derive(Clone, Copy)]
pub(super) struct MemberDialect<'a> {
    pub(super) surface: Option<SurfaceQuery<'a>>,
    pub(super) rules: WordValueRules,
}

/// Per-subcommand dispatcher shared by the body-form and
/// inline-form walkers.
///
/// `texts` and `argv` are parallel: `texts[0]` / `argv[0]` is
/// the subcommand name (``superclass`` / ``method`` / etc.).
/// `oo::define Cls private <subcmd> ...` — wraps a method-defining
/// subcommand with `visibility = "private"`.  Extracted from
/// [`apply_oo_subcommand`] to keep the dispatch under threshold.
/// Display text stays separate from the actual source class declaration.
#[derive(Clone, Copy)]
struct MethodBodyClass<'a> {
    display: &'a str,
    declaration: Option<&'a ClassDef>,
}

/// A `TclOO` method body collected during the definition walk, with its actual
/// argument tokens. The source declaration stays separate from display text.
struct CollectedMethodBody {
    /// Method name (`<constructor>` / `<destructor>` for those forms).
    name: String,
    /// Formal parameters (empty for `destructor`).
    params: Vec<ParamDef>,
    /// Inner body text (braces stripped), as `analyse_body` expects.
    body_text: String,
    /// The body word token (carries the absolute span + `content_offset`).
    body_tok: Token,
    /// The raw param-list word token (`{a b}`), used to anchor each formal
    /// parameter's definition span at its name. `None` for
    /// `destructor` (no parameter list).
    params_tok: Option<Token>,
    /// True for a **class-side** member (`classmethod`, `self method`) —
    /// and for any collected body that is not a real instance-side method
    /// invocation frame (class `initialise` scripts, `property` accessor
    /// bodies). Gates [`super::types::Scope::oo_defining_class`]: `[self
    /// class]` *raises* in a class-side frame, so no defining-class fact may
    /// be recorded for one.
    class_side: bool,
}

/// Bodies collected from a definition grammar before their scopes can be
/// walked with the complete class variable set.
#[derive(Default)]
struct CollectedDefinitionBodies {
    methods: Vec<CollectedMethodBody>,
    accessors: Vec<CollectedMethodBody>,
    initialisers: Vec<CollectedMethodBody>,
}

/// The effective default visibility string for a freshly-(re)defined
/// member under `grammar`'s family rule (`"public"` / `"unexported"`).
fn default_visibility(grammar: &DefinitionBodyGrammar, name: &str) -> String {
    if grammar.member_default_exported(name) {
        "public".to_string()
    } else {
        "unexported".to_string()
    }
}

/// The visibility of a newly-declared member, selected by its grammar's
/// optional declaration argument when present and otherwise by the caller's
/// family/wrapper default. No caller needs to parse a member-specific flag.
fn declared_member_visibility(
    member: &MemberSpec,
    args: &[String],
    fallback: String,
    dialect: Option<SurfaceQuery<'_>>,
) -> String {
    member
        .declared_visibility_for_in(args, dialect)
        .map_or(fallback, |visibility| visibility.as_str().to_string())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod lifecycle_formal_tests;
