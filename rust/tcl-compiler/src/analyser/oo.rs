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
use tcl_dialect::model::surface_admits;
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::Traits;
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::{
    CallableRole, DefinitionBodyGrammar, InitTiming, MemberCurrentNamespace, MemberEffect,
    MemberKind, MemberReceiver, MemberRefKind, MemberRow, MemberSpec, MemberVisibility,
    RelationSlot, StateScope, WrapperShift,
};
use tcl_registry::side_effects::SideEffectTarget;
use tcl_registry::{InvocationArguments, InvocationWord};
use tcl_syntax::word_rules::WordValueRules;

use super::diagnostics::helpers::has_substitution;
use super::scope::scope_at_mut;
use super::state::Analyser;
use super::types::{
    ClassDef, DefinitionAbort, DefinitionAbortKind, MemberSide, MethodDef, PropertyDef,
    RenamedMember, Scope, ScopeKind, UnknownProcInfo,
};
use super::utils::{param_name_spans_for_token, parse_param_list};
use crate::ir::{MethodKind, Statement};
use crate::signature_scan::types::ParamDef;

/// Original-handler chaining follows the retained implementation identity.
/// A saved name chosen by the author is not evidence without a proved rename.
fn chains_original_unknown(facts: &tcl_registry::InvocationFacts) -> bool {
    facts.traits.contains(Traits::UNRESOLVED_COMMAND_HANDLER)
}

/// Effective registry effects, including alias prefixes and subcommand forms.
fn spawns_process(facts: &tcl_registry::InvocationFacts) -> bool {
    facts.traits.contains(Traits::UNSAFE)
        && facts
            .effects
            .legacy()
            .side_effects
            .iter()
            .any(|effect| effect.target == SideEffectTarget::Process)
}

fn loads_external_unit(facts: &tcl_registry::InvocationFacts) -> bool {
    facts.traits.contains(Traits::LOADS_EXTERNAL_UNIT)
}

/// Implicit variable snit injects into `typemethod` / `typeconstructor` bodies.
///
/// The instance-body implicits (`self` / `selfns` / `type` / `options`) come
/// from the definer's registry grammar (`implicit_vars`); this is only the
/// narrower *type*-body subset, which the flat grammar list doesn't
/// distinguish — a `typemethod` body sees `type` but not `self`.
// registry-axis-ok: special_vars — snit's implicit type variable in a typemethod body; until the snit definition grammar declares its implicit variables
const SNIT_TYPE_IMPLICIT: &[&str] = &["type"];

/// A definer's grammar plus the class being populated and where its members
/// live — bundled so the snit / itcl member dispatch + extraction helpers stay
/// under the argument limit.  Shared by both families (the extraction is
/// grammar-driven; only the per-family dispatch differs).
struct OriginalJimClassLists {
    bases_dynamic: bool,
    superclasses: Vec<String>,
    variable_words: Vec<String>,
}

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

/// The snit definer being parsed.  Which implicit variables its member
/// bodies see — including the widget-only `win` / `hull` pair — comes from
/// the grammar's own `implicit_vars` (`snit::widget` / `widgetadaptor` carry
/// `SNIT_WIDGET_GRAMMAR`), so no consumer matches the definer's name suffix.
struct SnitDefiner {
    grammar: &'static DefinitionBodyGrammar,
}

/// Whether `member` declares variables by name: a
/// [`MemberEffect::StateDeclaration`] whose words name them — every argument
/// (`TclOO`'s `variable a b c`) or its [`ArgRole::VarWrite`] word (snit's
/// `variable` / `typevariable` / `component` / `typecomponent`, itcl's
/// `variable` / `common`). snit's `option` declares state but names an option,
/// not a variable, so it is not one. A callable that binds a value variable
/// (snit 1.x `onconfigure`) is a body, not a declaration.
fn declares_variables(member: &MemberSpec) -> bool {
    matches!(member.effect, MemberEffect::StateDeclaration { .. })
        && (member.all_args_var || member.indices_for(ArgRole::VarWrite).next().is_some())
}

/// Map each declared instance-variable name in `known` to the span of its
/// declaration name-token, scanning the class-body `cmds`.  Used to anchor a
/// seeded object variable's `definition_span` at its `variable v` declaration
/// instead of the whole method body — a whole-body span would let a
/// variable rename overwrite the entire method.  Only names already present in
/// `known` (the authoritative `ClassDef::variables` list) are recorded, so a
/// non-name argument of a declaration member can never be mistaken for a
/// variable; the first declaration of a name wins.
fn collect_var_decl_spans(
    grammar: &DefinitionBodyGrammar,
    cmds: &[crate::segmenter::SegmentedCommand],
    known: &[String],
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    dialect: Option<SurfaceQuery<'_>>,
) -> std::collections::HashMap<String, Span> {
    let mut out: std::collections::HashMap<String, Span> = std::collections::HashMap::new();
    if known.is_empty() {
        return out;
    }
    let known_set: std::collections::HashSet<&str> = known.iter().map(String::as_str).collect();
    for cmd in cmds {
        if cmd.is_partial {
            continue;
        }
        // A statement is a variable declaration when its row declares the
        // instances' state and its member names variables — the same answer
        // the class fold gives it, so the span map and `ClassDef::variables`
        // cannot disagree.  Gating additionally on `known_set` below means a
        // stray non-declaration match cannot leak.
        let Some(statement) = MemberStatement::read(grammar, &cmd.texts, &cmd.argv, dialect) else {
            continue;
        };
        if !declares_variables(statement.member)
            || statement.row.receiver != MemberReceiver::Instance
        {
            continue;
        }
        let groups = tcl_lexer::group_commands_bytes(&cmd.all_tokens, image.bytes(), config);
        let [group] = groups.as_slice() else {
            continue;
        };
        if group.words.len() != cmd.argv.len() || group.words.iter().any(|word| word.expand) {
            continue;
        }
        let args = cmd.args().iter().map(String::as_str).collect::<Vec<_>>();
        for argument in
            grammar.member_arg_indices_for_role_in(cmd.name(), &args, dialect, ArgRole::VarWrite)
        {
            let ordinal = argument + 1;
            let Some((group, tok)) = group.words.get(ordinal).zip(cmd.argv.get(ordinal)) else {
                continue;
            };
            let Ok(word) =
                tcl_lexer::NativeWord::from_group(image.clone(), config, &cmd.all_tokens, group)
            else {
                continue;
            };
            let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(&word)
            else {
                continue;
            };
            let Ok(name) = String::from_utf8(value) else {
                continue;
            };
            let base = tcl_syntax::naming::split_element_ref(&name)
                .map_or(name.as_str(), |(root, _)| root);
            if known_set.contains(base) {
                out.entry(base.to_owned()).or_insert(tok.span);
            }
        }
    }
    out
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

/// The `(var_idx, list_idx, body_idx)` post-head argument positions of a
/// registry-declared "var/list pairs then a trailing body" loop command
/// (`foreach`/`lmap`), when it has **exactly one** var/list pair — the only
/// shape [`Analyser::record_loop_installed_members`] can read a member name
/// off of. `None` for anything else: no [`tcl_registry::Traits::LOOP_LIST_HEADER`]
/// trait, no single matching [`tcl_registry::repeated::RepeatedArgLayout`],
/// or more (or fewer) than one pair — a multi-list `foreach` has no single
/// variable a member's name word could unambiguously mean.
pub(super) fn loop_installer_pair(
    spec: &tcl_registry::CommandSpec,
    post_head: usize,
) -> Option<(usize, usize, usize)> {
    if !spec.traits.contains(tcl_registry::Traits::LOOP_LIST_HEADER) {
        return None;
    }
    let [layout] = spec.repeated_args else {
        return None;
    };
    if layout.role != ArgRole::LoopVarList || layout.stride != 2 || layout.exclude_trailing != 1 {
        return None;
    }
    let var_indices = layout.indices(post_head);
    let &[var_idx] = var_indices.as_slice() else {
        return None;
    };
    Some((var_idx, var_idx + 1, post_head - 1))
}

/// Split a loop's literal list word into `(element, absolute source span)`
/// pairs, or `None` when the word is not literal (a single brace/quote/bare
/// token with no substitutions — same predicate the parameter-list literal
/// check uses) or fails to parse as a Tcl list.
fn literal_loop_elements(
    list_word: &str,
    list_tok: Token,
    list_single_token: bool,
) -> Option<Vec<(String, Span)>> {
    if !crate::signature_scan::params::param_word_is_literal(list_tok.kind, list_single_token) {
        return None;
    }
    let list_content_start = list_tok.span.start() + u32::from(list_tok.content_offset);
    let mut members = Vec::new();
    let mut pos = 0usize;
    loop {
        match tcl_syntax::list::find_element(list_word, pos) {
            Ok(Some(el)) => {
                let raw = &list_word[el.value.clone()];
                let value = if el.literal {
                    raw.to_string()
                } else {
                    tcl_lexer::backslash_subst(raw).into_owned()
                };
                let span = Span::new(
                    list_content_start + u32::try_from(el.value.start).unwrap_or(0),
                    list_content_start + u32::try_from(el.value.end).unwrap_or(0),
                );
                members.push((value, span));
                pos = el.next;
            }
            Ok(None) => break,
            // A malformed list is left exactly as opaque — no partial guess.
            Err(_) => return None,
        }
    }
    if members.is_empty() {
        None
    } else {
        Some(members)
    }
}

/// Whether a loop's body is exactly one member declaration (per `grammar`,
/// under any `self`/`private` wrapper) that names itself with exactly a
/// reference to the loop variable `var_name` — `foreach`'s installer idiom.
/// Returns the member's `(kind, body_span)` on a match; `None` for anything
/// else (more than one statement, a fixed name, a name built from more than
/// the bare variable, a member that is no method on either side) — left
/// exactly as opaque as before, not a partial guess.
fn loop_installed_member_shape(
    grammar: &DefinitionBodyGrammar,
    body_word: &str,
    body_content_start: u32,
    lexer_config: tcl_lexer::LexerConfig,
    dialect: Option<SurfaceQuery<'_>>,
    var_name: &str,
) -> Option<(MethodKind, Span)> {
    let inner_cmds = crate::segmenter::segment_commands_with_offset_and_config(
        body_word,
        body_content_start,
        lexer_config.nested(),
    );
    let mut real_cmds = inner_cmds
        .iter()
        .filter(|c| !c.is_partial && !c.argv.is_empty());
    let inner = real_cmds.next()?;
    if real_cmds.next().is_some() {
        return None; // more than one statement — not the simple installer shape
    }
    let statement = MemberStatement::read(grammar, &inner.texts, &inner.argv, dialect)?;
    let MemberLanding::Method(kind @ (MethodKind::Method | MethodKind::ClassMethod)) =
        statement.landing()
    else {
        return None;
    };
    let (member, inner_texts, inner_argv) =
        (statement.member, statement.args(), statement.arg_tokens());
    let name_idx = member
        .indices_for_call_in(inner_texts, dialect, ArgRole::Name)
        .next()?;
    let original = tcl_lexer::native_script_words_in_with_purpose(
        tcl_lexer::SourceImage::document(body_word),
        Span::new(0, u32::try_from(body_word.len()).ok()?),
        lexer_config,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    if original.fatal_tail.is_some() || original.commands.len() != 1 {
        return None;
    }
    let words = &original.commands.first()?.words;
    if words.len() != inner.argv.len() {
        return None;
    }
    let ordinal = statement
        .row
        .keyword_index
        .checked_add(1)?
        .checked_add(name_idx)?;
    let name =
        tcl_syntax::word_rules::original_scalar_variable_with_prefix(words.get(ordinal)?, &[b""])?;
    if name != var_name.as_bytes() {
        return None;
    }
    let body_idx = member
        .indices_for_call_in(inner_texts, dialect, ArgRole::Body)
        .next()?;
    let body_span = inner_argv.get(body_idx).map_or(inner.span, |t| t.span);
    Some((kind, body_span))
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

fn original_two_word_proc_name(
    declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<
        super::types::ProcDef,
    >,
) -> Option<(
    crate::signature_scan::scope::SignatureSourceNameInput,
    crate::signature_scan::scope::SignatureSourceNameInput,
)> {
    let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
        declaration.name_input().clone(),
    );
    let children = input.original_list_elements()?;
    let [class, member] = children.as_slice() else {
        return None;
    };
    Some((class.clone(), member.clone()))
}

fn insert_original_two_word_member(
    class_def: &mut ClassDef,
    member: &str,
    proc: &super::types::ProcDef,
) {
    class_def.methods.insert(
        member.to_string(),
        MethodDef {
            name: member.to_string(),
            params: proc.params.clone(),
            params_computed: proc.params_computed,
            formal_count: proc.formal_count.clone(),
            name_span: proc.name_span,
            body_span: proc.body_span,
            kind: "method".to_string(),
            is_self_method: false,
            visibility: "public".to_string(),
            doc: proc.doc.clone(),
            forward_target: None,
        },
    );
}

impl Analyser {
    pub(super) fn original_provider_class_name(
        &self,
        span: Span,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNameOccurrence> {
        let policy = self.declaration_name_policy()?;
        self.head_identities
            .original_source_name_at_span(span, self.file_lexer_config(), self.word_rules(), policy)
            .or_else(|| {
                // The actual walk retains this complete original static word
                // independently of invocation entry. It supplies naming syntax
                // only; the caller still selects the definer and naming scope.
                let original = self.original_static_source_name_at_span(span)?;
                let input = original.name_input();
                (input.lexer_config() == self.file_lexer_config()
                    && input.word_value_rules() == self.word_rules()
                    && input.policy() == policy)
                    .then(|| original.clone())
            })
    }

    pub(super) fn retain_original_provider_class(
        &mut self,
        original: Option<&crate::signature_scan::original_name::SourceOriginalNameOccurrence>,
        class: &ClassDef,
    ) {
        if self.selected_logical_declaration_advice()
            && let Some(image) = self
                .result
                .retained_command_realm()
                .filter(|realm| {
                    realm.matches_resolved_analysis_input(&self.resolved_analysis_input())
                })
                .and_then(|realm| realm.original_source_image())
                .filter(|image| image.bytes() == self.source.as_bytes())
                .cloned()
            && let Some(declaration) = super::types::LogicalSourceClassDeclaration::capture(
                image,
                self.resolved_analysis_input(),
                class,
            )
        {
            self.result.logical_class_declarations.push(declaration);
        }
        if let Some(original) = original
            && let Some(name) = class.source_name.clone()
            && let Some(metadata) =
                crate::signature_scan::original_name::SourceDeclarationMetadata::new(
                    original,
                    name,
                    class.clone(),
                )
        {
            self.result.original_class_metadata.push(metadata);
        }
        // The selected definer supplies the class role. Hosted source words
        // retain their own context and unavailable materialisation; no C/Jim
        // command slot is recovered from the reporting class name.
        if let Some(original) = self.result.original_vendor_source_name_in_source(
            &tcl_lexer::SourceImage::document(&self.source),
            self.file_lexer_config(),
            class.name_span,
        ) {
            let record = crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata::new(
                original,
                tcl_syntax::naming::VendorSourceNamePurpose::SourceName,
                class.clone(),
            );
            if !self.result.original_vendor_class_metadata.contains(&record) {
                self.result.original_vendor_class_metadata.push(record);
            }
        }
    }
    /// Walk the body of a ``oo::class create`` / ``oo::define``
    /// block, populating `class_def` from each subcommand.
    ///
    /// The body is re-segmented via
    /// [`crate::segmenter::segment_commands_with_offset`] (no
    /// recovery — recovery is top-level only).  Dynamic bodies
    /// (non-`Str` tokens) skip the walk because they can't be
    /// statically re-segmented.
    /// Whether the command `cmd_name` is *unavailable* in the active dialect —
    /// its registry spec is dialect-gated and the document's dialect falls
    /// outside the gate.  Used to suppress member-level dialect diagnostics
    /// when the enclosing definer (`oo::configurable`, itself 9.0+) is already
    /// flagged: one diagnostic for the version-only construct, not a cascade.
    pub(super) fn command_dialect_disabled(&self, cmd_name: &str) -> bool {
        let generation = self.analysis_context();
        generation
            .commands()
            .get(cmd_name)
            .is_some_and(|spec| !generation.context().spec_available(spec))
    }

    /// Whether the definition-body member `subcmd` is available in the active
    /// dialect.  A member with no dialect restriction (the common case) and an
    /// unknown member (handled elsewhere, not gated here) both count as
    /// available; a version-gated member — `classmethod`, `private`,
    /// `initialise` / `initialize`, `definitionnamespace`, `property`, all
    /// 9.0+ — is available only when the document's dialect intersects its
    /// set.  An unrecognised dialect never restricts.
    fn oo_member_available(&self, grammar: &DefinitionBodyGrammar, subcmd: &str) -> bool {
        let Some(member) = grammar.member(subcmd) else {
            return true;
        };
        match member.surface {
            None => true,
            Some(allowed) => surface_admits(
                allowed,
                Some(&self.analysis_context().context().authoring_query()),
            ),
        }
    }

    /// Emit W002 for a definition-body member the active dialect's grammar
    /// does not have.
    ///
    /// Shared by the three call shapes a member can take — inside a
    /// metaclass's `create` body, inside an `oo::define Cls { … }` block
    /// (both via [`Self::parse_oo_definition_body`]), and the single-command
    /// `oo::define Cls classmethod …` form (via
    /// [`Self::handle_oo_define_command`]) — so all three report the same way
    /// rather than only the bodies being checked.
    ///
    /// Reports **without** dropping the member: the walk still records it,
    /// so go-to-definition, references, rename, document symbols, and code
    /// lenses keep working on the code the user actually wrote.  That
    /// matches the whole-command W002 ([`Self::emit_w002_disabled_command`]),
    /// which likewise reports a dialect-unavailable command while the
    /// analyser goes on modelling the call.  Erasing the member instead
    /// would answer "this needs Tcl 9.0" by silently breaking every editor
    /// feature over it.
    ///
    /// `definer_disabled` bypasses the gate: when the enclosing definer is
    /// itself unavailable (`oo::configurable` under 8.6) its own diagnostic
    /// already covers the construct, and a member word must never cascade
    /// into a second report.
    pub(super) fn emit_w002_oo_member_disabled(
        &mut self,
        grammar: &DefinitionBodyGrammar,
        subcmd: &str,
        tok: Token,
        definer_disabled: bool,
    ) {
        if definer_disabled || self.oo_member_available(grammar, subcmd) {
            return;
        }
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                tcl_core_types::DiagCode::W002,
                tok.span,
                format!(
                    "'{subcmd}' is disabled in the active dialect profile ('{}')",
                    self.dialect()
                ),
                super::types::Severity::Warning,
            ));
    }

    /// Emit W002 for a recognised optional member argument that exists in the
    /// grammar but is unavailable in this document's selected dialect.
    ///
    /// Unlike a whole-member gate, this preserves Tcl 8.6's ordinary
    /// `method NAME PARAMS BODY` layout while declining a Tcl 9 visibility
    /// flag in that same member. The option vocabulary and position remain in
    /// [`MemberSpec`]; this is only the generic diagnostic presentation.
    pub(super) fn emit_w002_oo_member_option_disabled(
        &mut self,
        member: &MemberSpec,
        args: &[String],
        arg_tokens: &[Token],
        definer_disabled: bool,
    ) -> bool {
        if definer_disabled {
            return false;
        }
        let Some(option) = member.unavailable_option_for(
            args,
            Some(self.analysis_context().context().authoring_query()),
        ) else {
            return false;
        };
        let Some(position) = member.optional_argument.map(|optional| optional.position) else {
            return false;
        };
        let Some(tok) = arg_tokens.get(usize::from(position)) else {
            return false;
        };
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                tcl_core_types::DiagCode::W002,
                tok.span,
                format!(
                    "'{}' is disabled in the active dialect profile ('{}')",
                    option.value,
                    self.dialect()
                ),
                super::types::Severity::Warning,
            ));
        true
    }

    /// Record the command references a definition-body member names as
    /// arguments, driven entirely by the member's registry grammar — never by a
    /// member keyword the walker recognises.  Two grammar facts name a command:
    ///
    /// * `all_args_ref == Some(MemberRefKind::Class)` — every argument names a
    ///   class (`superclass A B`, `mixin ?-append? M …`).  A class is a command
    ///   in `TclOO`, so each is a command reference resolved in the class's
    ///   namespace.
    /// * an [`ArgRole::CommandName`] argument position — one argument names a
    ///   command (`forward NAME TARGET …`: the delegated command's name).
    ///
    /// Both flow through the ordinary invocation machinery, so find-references,
    /// go-to-definition, rename, and call-hierarchy reach the named class /
    /// command across files exactly as a direct call does.  A
    /// `MemberRefKind::Method` member names methods of *this* class rather than
    /// commands, so the method-reference machinery handles it, not this path.  A
    /// dynamic word (`superclass $base`) names no static command and is skipped.
    /// Whether one command of a definition body installs members this walk
    /// cannot read, making the class's recorded member tables a lower bound
    /// ([`super::types::ClassDef::member_set_incomplete`]).
    ///
    /// Two shapes qualify, neither matched by keyword:
    ///
    /// * a **member** word whose declaration is supplied dynamically — a
    ///   `{*}` expansion that survived [`splice_static_member_expansions`]
    ///   (only a braced literal splices statically), or a computed word in
    ///   one of the member's own declaring roles (`method $m …`). Tcl
    ///   resolves both at definition time, so the members are real; the
    ///   analyser simply cannot name them.
    /// * a **non-member** word that either has no registry spec (a helper
    ///   proc, or a class-installer command from another file) or declares
    ///   an [`ArgRole::Body`] argument — a script the member walk does not
    ///   descend into, so any `method` inside it is invisible
    ///   (`foreach m {…} { method $m … }`, `if {…} { method x {} {…} }`).
    ///
    /// Registry data decides both halves: the definer grammar says what a
    /// member word is and which of its arguments declare, and the command
    /// spec says whether a word carries a script. Nothing here names a
    /// command or a keyword.
    ///
    /// A comment line segments to no words at all and is skipped by the
    /// caller, so it never reaches this test.
    fn member_declaration_is_opaque(
        &self,
        grammar: &DefinitionBodyGrammar,
        cmd: &crate::segmenter::SegmentedCommand,
        texts: &[String],
    ) -> bool {
        let Some(keyword) = texts.first() else {
            return false;
        };
        let Some(member) = grammar.member(keyword) else {
            // Not a member word. Unknown to the registry, or script-taking:
            // either way this command can install members out of sight.
            // Roles come from the static table *and* the per-call resolver
            // (`foreach`'s layout depends on how many var/list pairs it was
            // given, so its `Body` index is only knowable from the words).
            let generation = self.analysis_context();
            return generation
                .context()
                .resolve_spec(generation.commands(), keyword)
                .is_none_or(|spec| {
                    // Possible declaration effects use the selected schema's
                    // closed role capabilities, without an execution query.
                    tcl_registry::CommandRegistry::spec_may_have_arg_role(spec, ArgRole::Body)
                });
        };
        // A `{*}` word the splicer would refuse — anything but a single
        // braced literal, whose element list is the only one knowable
        // without running the program.  Tested per word (not "did any
        // splice happen"), so a command mixing a spliceable and an
        // unspliceable expansion is still opaque.
        if let Some(expand) = cmd.expand_word.as_ref()
            && expand.iter().enumerate().any(|(i, &e)| {
                e && !(cmd.argv.get(i).is_some_and(|t| t.kind == TokenType::Str)
                    && cmd.single_token_word.get(i).copied().unwrap_or(false))
            })
        {
            return true;
        }
        // A computed member **name** — `method $m …`, the installer-loop
        // spelling — declares a member the walk cannot name.  `+ 1` because
        // role indices are relative to the member's arguments while `texts`
        // still carries the keyword at 0.
        //
        // Only the name word is tested.  A parameter list or body word is a
        // *script*, and a script routinely contains `$` and `[` without being
        // dynamic in the sense that matters here (`constructor {x} { set a
        // $x }` is entirely readable); the `{*}` test above is what catches
        // the genuinely reflected signature.
        let original =
            self.original_logical_member_words(grammar, cmd)
                .filter(|(statement, _, _)| {
                    cmd.texts.as_slice() == texts && std::ptr::eq(statement.member, member)
                });
        member
            .indices_for_call_in(
                &texts[1..],
                Some(self.analysis_context().context().authoring_query()),
                ArgRole::Name,
            )
            .any(|idx| {
                if let Some((statement, _, words)) = original.as_ref() {
                    return words
                        .get(statement.row.keyword_index + 1 + idx)
                        .is_none_or(|word| {
                            tcl_syntax::word_rules::original_static_word_unicode_value(word)
                                .is_none_or(|value| value.contains(&0))
                        });
                }
                if self.selected_logical_declaration_advice() {
                    return true;
                }
                texts
                    .get(idx + 1)
                    .is_some_and(|w| crate::naming::is_dynamic_word(w))
            })
    }

    /// Record the per-name members a **literal** loop-installer declares,
    /// even though [`Self::member_declaration_is_opaque`] has already (and
    /// correctly) marked the class's member set incomplete because of it.
    ///
    /// `foreach m {alpha beta gamma} { method $m {args} {…} }` computes
    /// each member's *name* from the loop variable, which is why the
    /// ordinary member walk cannot read it — but the loop's own list is
    /// written right there in the source, so the set of names it will bind
    /// `$m` to really is knowable without running the program. This adds
    /// what can honestly be said on top of the existing abstention; it
    /// never narrows `member_set_incomplete` back to "complete", because
    /// knowing three names is not the same as knowing them all (a sibling
    /// `oo::define` elsewhere, or a second, unreadable installer in the
    /// same body, could still add more).
    ///
    /// Deliberately narrow — every condition below is a bail, not a best
    /// effort, and any miss just leaves the loop as opaque as it already
    /// was:
    ///
    /// * the head word must be a registry-declared "var/list pairs then a
    ///   trailing body" loop shape ([`tcl_registry::Traits::LOOP_LIST_HEADER`]
    ///   plus exactly one [`tcl_registry::repeated::RepeatedArgLayout`]
    ///   covering a var/list pair) — `foreach`/`lmap`, decided from registry
    ///   data, never a keyword;
    /// * exactly one var/list pair (a multi-list `foreach` has no single
    ///   variable a member's name word could unambiguously mean);
    /// * the loop variable is one bare name, not a multi-variable group
    ///   (`foreach {a b} $list …`);
    /// * the list word is a **literal** Tcl list (no `$`/`[`) that parses
    ///   cleanly;
    /// * the body is exactly one member declaration (per `grammar`, unwrapped
    ///   of any `self`/`private`) whose `Name` argument is exactly a
    ///   reference to the loop variable — anything else (a fixed name, a
    ///   name built from more than the bare variable, more than one
    ///   statement) is left unread, exactly as before this ran.
    ///
    /// The recorded member's parameter list is always `params_computed`
    /// (never re-derived from the loop body's own written params, even when
    /// they look literal): a reflective installer's signature can itself be
    /// computed per iteration (`method $m {*}[classDef $m]`), so treating
    /// one binding's `{args}` as representative of every other would be a
    /// fabrication for the general shape this exists to cover.
    fn record_loop_installed_members(
        &self,
        grammar: &DefinitionBodyGrammar,
        cmd: &crate::segmenter::SegmentedCommand,
        class_def: &mut ClassDef,
    ) {
        let Some(keyword) = cmd.texts.first().map(String::as_str) else {
            return;
        };
        let generation = self.analysis_context();
        let Some(spec) = generation
            .context()
            .resolve_spec(generation.commands(), keyword)
        else {
            return;
        };
        // Post-head argument count (`texts` still carries the keyword at 0).
        let post_head = cmd.texts.len().saturating_sub(1);
        let Some((var_idx, list_idx, body_idx)) = loop_installer_pair(spec, post_head) else {
            return;
        };
        let (Some(var_word), Some(list_word), Some(body_word)) = (
            cmd.texts.get(var_idx + 1),
            cmd.texts.get(list_idx + 1),
            cmd.texts.get(body_idx + 1),
        ) else {
            return;
        };
        // The loop variable must be one bare name.
        let Ok(var_names) = self.word_rules().split_list(var_word) else {
            return;
        };
        if var_names.len() != 1 {
            return; // a multi-variable group — no single name a `$var` could mean
        }
        let var_name = &var_names[0];
        if !tcl_syntax::naming::is_static_variable_source_word(var_name) {
            return;
        }
        let Some(list_tok) = cmd.argv.get(list_idx + 1) else {
            return;
        };
        let list_single = cmd
            .single_token_word
            .get(list_idx + 1)
            .copied()
            .unwrap_or(false);
        let Some(members) = literal_loop_elements(list_word, *list_tok, list_single) else {
            return;
        };
        let Some(body_tok) = cmd.argv.get(body_idx + 1) else {
            return;
        };
        let body_content_start = body_tok.span.start() + u32::from(body_tok.content_offset);
        let Some((kind, body_span)) = loop_installed_member_shape(
            grammar,
            body_word,
            body_content_start,
            self.file_lexer_config(),
            Some(self.analysis_context().context().authoring_query()),
            var_name,
        ) else {
            return;
        };
        for (name, name_span) in members {
            let md = MethodDef {
                visibility: default_visibility(grammar, &name),
                name,
                params: Vec::new(),
                params_computed: true,
                formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                    tcl_dialect::ParameterGrammar::Tcl,
                ),
                name_span,
                body_span,
                kind: kind.as_str().to_string(),
                is_self_method: false,
                doc: String::new(),
                forward_target: None,
            };
            // A literal, directly-written declaration elsewhere in the class
            // always outranks a name merely *inferred* from the loop's list —
            // this only ever fills a gap, never overrides real data, however
            // the two are ordered in the source.
            let table = if kind == MethodKind::ClassMethod {
                &mut class_def.class_methods
            } else {
                &mut class_def.methods
            };
            table.entry(md.name.clone()).or_insert(md);
        }
    }

    pub(super) fn record_member_command_references(
        &mut self,
        grammar: &DefinitionBodyGrammar,
        texts: &[String],
        argv: &[Token],
        scope_path: &[usize],
    ) {
        // Unwrap a `self …` (or itcl access-modifier) prefix so a wrapped
        // `self mixin -append …` still contributes its class references.  The
        // returned texts/tokens are the words *after* the effective keyword, so
        // grammar arg-role indices (0-based after the keyword) index them
        // directly.
        let Some((keyword, arg_texts, arg_toks)) = unwrap_wrapper_member(grammar, texts, argv)
        else {
            return;
        };
        let Some(spec) = grammar.member(keyword) else {
            return;
        };
        // `skip_flags` drops `-`-prefixed option words — correct only for the
        // class-list members (`mixin -append …`), where the flags aren't class
        // names.  A `forward` TARGET is a plain command name that may legally
        // begin with `-` (`forward f -foo` delegates to a command named
        // `-foo`), so its `ArgRole::CommandName` recording keeps such words.
        let record = |analyser: &mut Self, name: &str, tok: &Token, skip_flags: bool| {
            if name.is_empty() || crate::naming::is_dynamic_word(name) {
                return;
            }
            if skip_flags && name.starts_with('-') {
                return;
            }
            // A named command reference carries no fixed call arity: a
            // superclass/mixin is not invoked here, and a `forward` target is
            // invoked with a variable number of appended arguments.
            let resolved = analyser.resolve_command_qualified_name(name, scope_path);
            analyser.push_command_reference(name.to_owned(), tok.span, resolved, None);
        };
        if spec.all_args_ref == Some(MemberRefKind::Class) {
            for (name, tok) in arg_texts.iter().zip(arg_toks) {
                record(self, name, tok, true);
            }
        }
        for idx in spec.indices_for_call_in(
            arg_texts,
            Some(self.analysis_context().context().authoring_query()),
            ArgRole::CommandName,
        ) {
            if let (Some(name), Some(tok)) = (arg_texts.get(idx), arg_toks.get(idx)) {
                record(self, name, tok, false);
            }
        }
    }

    pub(super) fn parse_oo_definition_body(
        &mut self,
        body_text: &str,
        body_tok: Token,
        class_def: &mut ClassDef,
        scope_path: &[usize],
        grammar: Option<&'static DefinitionBodyGrammar>,
        definer_disabled: bool,
    ) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        // Member recognition + argument layout come from the definer's registry
        // grammar; without one (only possible when the analyser has no registry,
        // e.g. a direct unit-test call) there is nothing to walk.
        let Some(grammar) = grammar else {
            return;
        };
        // A Clay class is a fresh TclOO class whose relation members reach
        // `oo::define` as TclOO's do.
        if matches!(
            grammar.family,
            tcl_registry::definer::DefinerFamily::TclOo
                | tcl_registry::definer::DefinerFamily::Clay
        ) && class_def.source_name.is_some()
            && !class_def.source_name_ambiguous.is_observed()
        {
            class_def.original_relations.begin_declaration();
        }
        // The methods walked below home under the class's qualified name;
        // capture it before the member-collection walk mutates `class_def`.
        let class_qualified = class_def.qualified_name.clone();
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        // A translating Document channel folds CRLF in the cooked value; the
        // member spans below are original offsets, so segment the region.
        let body_text = crate::segmenter::braced_body_source_region(
            &self.source,
            base_offset as usize,
            body_tok.span.end().saturating_sub(1) as usize,
            body_text,
        )
        .to_owned();
        let cmds = crate::segmenter::segment_commands_with_offset_and_config(
            &body_text,
            base_offset,
            self.file_lexer_config().nested(),
        );
        let CollectedDefinitionBodies {
            methods: method_bodies,
            accessors: accessor_bodies,
            initialisers: init_bodies,
        } = self.collect_oo_definition_members(
            grammar,
            &cmds,
            class_def,
            scope_path,
            definer_disabled,
        );
        self.original_class_definition_body = None;
        // The display inventory is separate from original source link authority.
        self.collect_oo_links(&method_bodies, class_def);

        // Walk each method / accessor / class-init body in its own
        // `Method` scope with the formal parameters and the class's instance
        // variables pre-bound.
        let class_variables = class_def.variables.clone();
        // Map each declared instance-variable name to its `variable v`
        // declaration name-token span, so the per-method seeding below anchors
        // the object variable's definition at the declaration rather than the
        // whole method body.
        let generation = self.analysis_context();
        let var_decl_spans = self
            .head_identities
            .original_source_image()
            .filter(|image| image.bytes() == self.source.as_bytes())
            .map_or_else(std::collections::HashMap::new, |image| {
                collect_var_decl_spans(
                    grammar,
                    &cmds,
                    &class_variables,
                    image,
                    self.file_lexer_config().nested(),
                    Some(generation.context().authoring_query()),
                )
            });
        // `TclOO` method bodies resolve bare commands globally (object-ns
        // semantics — see `Scope::oo_global_resolution`); snit / itcl
        // members resolve in the type / class namespace.
        let oo_global =
            grammar.member_current_namespace() == MemberCurrentNamespace::RuntimeReceiver;
        // The class-level `initialise` body runs first and in a scope of its
        // own, so the class-scoped variables it declares are visible to this
        // class's own method / accessor bodies below — and to no other
        // class's.
        let mut class_variables = class_variables;
        let mut var_decl_spans = var_decl_spans;
        for mb in &init_bodies {
            for (name, span) in
                self.walk_class_init_body(&class_variables, &class_qualified, scope_path, mb)
            {
                if !class_variables.contains(&name) {
                    class_variables.push(name.clone());
                }
                var_decl_spans.entry(name).or_insert(span);
            }
        }
        for mb in method_bodies.iter().chain(accessor_bodies.iter()) {
            self.walk_method_body(
                &class_variables,
                &var_decl_spans,
                MethodBodyClass {
                    display: &class_qualified,
                    declaration: Some(class_def),
                },
                scope_path,
                mb,
                oo_global,
            );
        }
    }

    /// Collect an OO definition body's members: update the class facts and
    /// retain body-bearing members for the later scope walk.
    fn collect_oo_definition_members(
        &mut self,
        grammar: &DefinitionBodyGrammar,
        cmds: &[crate::segmenter::SegmentedCommand],
        class_def: &mut ClassDef,
        scope_path: &[usize],
        definer_disabled: bool,
    ) -> CollectedDefinitionBodies {
        let mut bodies = CollectedDefinitionBodies::default();
        for cmd in cmds {
            if !cmd.is_partial && !cmd.argv.is_empty() {
                self.collect_oo_definition_member(
                    grammar,
                    cmd,
                    class_def,
                    scope_path,
                    definer_disabled,
                    &mut bodies,
                );
            }
        }
        bodies
    }

    fn original_logical_member_words<'a>(
        &self,
        grammar: &'a DefinitionBodyGrammar,
        cmd: &'a crate::segmenter::SegmentedCommand,
    ) -> Option<(
        MemberStatement<'a>,
        super::ResolvedAnalysisInput,
        Vec<tcl_lexer::NativeWord>,
    )> {
        if !self.selected_logical_declaration_advice() || cmd.is_partial {
            return None;
        }
        let input = self.resolved_analysis_input();
        let image = self
            .result
            .retained_command_realm()
            .filter(|realm| realm.matches_resolved_analysis_input(&input))
            .and_then(|realm| realm.original_source_image())
            .filter(|image| image.bytes() == self.source.as_bytes())?;
        let config = self.file_lexer_config().nested();
        if config != input.lexer_config().nested() {
            return None;
        }
        let context = self.analysis_context();
        let statement = MemberStatement::read(
            grammar,
            &cmd.texts,
            &cmd.argv,
            Some(context.context().authoring_query()),
        )?;
        let groups = tcl_lexer::group_commands_bytes(&cmd.all_tokens, image.bytes(), config);
        let [group] = groups.as_slice() else {
            return None;
        };
        if group.words.len() != cmd.argv.len() || group.words.iter().any(|word| word.expand) {
            return None;
        }
        let words = group
            .words
            .iter()
            .map(|group| {
                tcl_lexer::NativeWord::from_group(image.clone(), config, &cmd.all_tokens, group)
            })
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        if !words.iter().zip(&cmd.argv).all(|(word, token)| {
            word.group().span == token.span
                && word.tokens().first().is_some_and(|original| {
                    original.kind == token.kind
                        && original.content_offset == token.content_offset
                        && original.in_quote == token.in_quote
                })
        }) || words[..=statement.row.keyword_index]
            .iter()
            .zip(&cmd.texts)
            .any(|(word, text)| {
                tcl_syntax::word_rules::original_static_word_ascii_presentation(word).as_deref()
                    != Some(text.as_bytes())
            })
        {
            return None;
        }
        Some((statement, input, words))
    }

    pub(super) fn retain_logical_member_name_operand(
        &self,
        grammar: &DefinitionBodyGrammar,
        cmd: &crate::segmenter::SegmentedCommand,
        class: &mut ClassDef,
    ) {
        let Some((statement, input, words)) = self.original_logical_member_words(grammar, cmd)
        else {
            return;
        };
        let MemberEffect::Callable {
            name_slot: Some(name_slot),
            ..
        } = statement.row.effect
        else {
            return;
        };
        let (Some(name), Some(side), Some(body)) = (
            statement.row.name.as_ref(),
            member_side(statement.row.receiver),
            statement.row.body,
        ) else {
            return;
        };
        let context = self.analysis_context();
        let surface = Some(context.context().authoring_query());
        let mut names =
            statement
                .member
                .indices_for_call_in(statement.args(), surface, ArgRole::Name);
        let mut bodies =
            statement
                .member
                .indices_for_call_in(statement.args(), surface, ArgRole::Body);
        if names.next() != Some(usize::from(name_slot))
            || names.next().is_some()
            || bodies
                .next()
                .map(|index| statement.row.keyword_index + 1 + index)
                != Some(body.0)
            || bodies.next().is_some()
        {
            return;
        }
        let name_ordinal = statement.row.keyword_index + 1 + usize::from(name_slot);
        let Some(word) = words.get(name_ordinal) else {
            return;
        };
        if tcl_syntax::word_rules::original_static_word_unicode_value(word).as_deref()
            != Some(name.as_bytes())
        {
            return;
        }
        let (Some(name_token), Some(body_token)) =
            (cmd.argv.get(name_ordinal), cmd.argv.get(body.0))
        else {
            return;
        };
        let mut parameters =
            statement
                .member
                .indices_for_call_in(statement.args(), surface, ArgRole::ParamList);
        let first_parameter = parameters.next();
        let parameter_ordinal = if parameters.next().is_none() {
            first_parameter.map(|index| statement.row.keyword_index + 1 + index)
        } else {
            None
        };
        class.original_members.retain_logical_name_operand(
            input,
            words,
            (name_ordinal, body.0, parameter_ordinal),
            side,
            (name_token.span, body_token.span),
            statement.row.clone(),
        );
    }

    pub(super) fn retain_logical_member_visibility_operands(
        &self,
        grammar: &DefinitionBodyGrammar,
        cmd: &crate::segmenter::SegmentedCommand,
        class: &mut ClassDef,
    ) {
        let Some((statement, input, words)) = self.original_logical_member_words(grammar, cmd)
        else {
            return;
        };
        if statement.row.effect != MemberEffect::Visibility
            || statement.member.all_args_ref != Some(MemberRefKind::Method)
        {
            return;
        }
        let (Some(side), Some(effect)) = (
            member_side(statement.row.receiver),
            statement.member.visibility_effect,
        ) else {
            return;
        };
        let first = statement.row.keyword_index + 1;
        if words[first..]
            .iter()
            .zip(statement.args())
            .any(|(word, name)| {
                name.as_bytes().contains(&0)
                    || tcl_syntax::word_rules::original_static_word_unicode_value(word).as_deref()
                        != Some(name.as_bytes())
            })
        {
            return;
        }
        for ordinal in first..words.len() {
            class.original_members.retain_logical_visibility_operand(
                input.clone(),
                words.clone(),
                ordinal,
                side,
                effect,
            );
        }
    }

    /// Selected source-member formal applicability and exact authored ordinals.
    /// The original class ledger supplies any lifecycle body; representative
    /// tokens and reporting strings cannot stand in for that whole word.
    fn member_formal_parameter_indices(
        &self,
        grammar: &DefinitionBodyGrammar,
        member: &MemberSpec,
        args: &[String],
        tokens: &[Token],
        class: &ClassDef,
    ) -> Vec<usize> {
        use tcl_registry::definer::SourceFormalValidationApplicability;
        // naming.tcloo.empty-lifecycle-definition
        // docs/design/analysis/name-resolution-proofs/tcloo-empty-lifecycle-definition.md
        let context = self.analysis_context();
        let surface = Some(context.context().authoring_query());
        let applicability = grammar
            .source_member_formal_validation_applicability(member, None)
            .or_else(|| {
                let kind = grammar.source_special_member_kind(member)?;
                let body_index = member
                    .indices_for_call_in(args, surface, ArgRole::Body)
                    .next()?;
                let body_token = tokens.get(body_index)?;
                let image = tcl_lexer::SourceImage::document(&self.source);
                let config = self.file_lexer_config();
                let mut selected = class
                    .original_special_members
                    .declarations()
                    .filter(|entry| {
                        let word = entry.body_word();
                        entry.kind() == kind
                            && word.image() == &image
                            && word.config() == config
                            && word.group().span.start() == body_token.span.start()
                    });
                let entry = selected.next()?;
                if selected.next().is_some() {
                    return None;
                }
                let values = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                    std::slice::from_ref(entry.body_word()),
                    self.declaration_name_policy()?.string_protocol(),
                )
                .ok()?;
                grammar
                    .source_member_formal_validation_applicability(member, Some(values.literal(0)?))
            });
        if applicability != Some(SourceFormalValidationApplicability::Required) {
            return Vec::new();
        }
        member
            .indices_for_call_in(args, surface, ArgRole::ParamList)
            .collect()
    }

    /// Process one segmented OO definition member using only its registered
    /// grammar, including wrapper expansion and profile-aware argument roles.
    fn collect_oo_definition_member(
        &mut self,
        grammar: &DefinitionBodyGrammar,
        cmd: &crate::segmenter::SegmentedCommand,
        class_def: &mut ClassDef,
        scope_path: &[usize],
        definer_disabled: bool,
        bodies: &mut CollectedDefinitionBodies,
    ) {
        self.retain_original_member_metadata(grammar, cmd, class_def, scope_path);
        self.retain_logical_relation_command(grammar, cmd, class_def);
        let spliced = splice_static_member_expansions(&self.source, cmd, self.file_lexer_config());
        let (texts, argv) = spliced.as_ref().map_or(
            (cmd.texts.as_slice(), cmd.argv.as_slice()),
            |(texts, argv)| (texts.as_slice(), argv.as_slice()),
        );
        if let (Some(subcmd), Some(token)) = (texts.first(), argv.first()) {
            self.emit_w002_oo_member_disabled(grammar, subcmd, *token, definer_disabled);
        }
        if let Some(member) = texts.first().and_then(|subcmd| grammar.member(subcmd))
            && self.emit_w002_oo_member_option_disabled(
                member,
                &texts[1..],
                &argv[1..],
                definer_disabled,
            )
        {
            return;
        }
        if self.member_declaration_is_opaque(grammar, cmd, texts) {
            class_def.member_set_incomplete = true;
            self.record_loop_installed_members(grammar, cmd, class_def);
        }
        let expanded = expand_wrapper_block_members(
            &self.source,
            grammar,
            texts,
            argv,
            self.file_lexer_config(),
        );
        if expanded.is_some() && self.selected_logical_declaration_advice() {
            class_def
                .original_members
                .mark_logical_relation_coverage_unavailable();
        }
        let member_calls: Vec<(&[String], &[Token])> = expanded.as_ref().map_or_else(
            || vec![(texts, argv)],
            |inner| {
                inner
                    .iter()
                    .map(|(texts, argv)| (texts.as_slice(), argv.as_slice()))
                    .collect()
            },
        );
        for (texts, argv) in member_calls {
            if let Some((keyword, args)) = texts.split_first()
                && let Some(member) = grammar.member(keyword)
            {
                let tokens = argv.get(1..).unwrap_or(&[]);
                let parameter_indices =
                    self.member_formal_parameter_indices(grammar, member, args, tokens, class_def);
                super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
                    self,
                    args,
                    tokens,
                    &parameter_indices,
                );
            }
            apply_oo_subcommand_in(
                grammar,
                texts,
                argv,
                class_def,
                MemberDialect {
                    surface: Some(self.analysis_context().context().authoring_query()),
                    rules: self.word_rules(),
                },
            );
            self.retain_class_relation_lookups(texts, class_def, scope_path);
            self.record_member_command_references(grammar, texts, argv, scope_path);
            if let Some(method) = collect_method_body(
                grammar,
                texts,
                argv,
                Some(self.analysis_context().context().authoring_query()),
                self.word_rules(),
            ) {
                bodies.methods.push(method);
            }
        }
        collect_class_level_bodies(
            grammar,
            texts,
            argv,
            Some(self.analysis_context().context().authoring_query()),
            &mut bodies.accessors,
            &mut bodies.initialisers,
        );
    }

    fn retain_logical_relation_command(
        &self,
        grammar: &DefinitionBodyGrammar,
        cmd: &crate::segmenter::SegmentedCommand,
        class: &mut ClassDef,
    ) {
        let Some((statement, input, words)) = self.original_logical_member_words(grammar, cmd)
        else {
            return;
        };
        if matches!(
            statement.row.effect,
            MemberEffect::Configuration | MemberEffect::InitScript { .. }
        ) {
            class
                .original_members
                .mark_logical_relation_coverage_unavailable();
            return;
        }
        let MemberEffect::Relation { slot: relation } = statement.row.effect else {
            return;
        };
        let side = member_side(statement.row.receiver);
        let slot = statement.member.slot;
        class.original_members.retain_logical_relation_command(
            input,
            words,
            (side, relation, slot, statement.row.slot_op),
        );
    }

    pub(super) fn retain_class_relation_lookups(
        &self,
        texts: &[String],
        class: &mut ClassDef,
        scope_path: &[usize],
    ) {
        // These are the same analyser-local field routes as apply_oo_subcommand_in.
        let names = match texts.first().map(String::as_str) {
            // registry-axis-ok: definition_body — class-definition relation keywords; until the definition-body grammar exposes relation kinds
            Some("superclass" | "inherit") => &class.superclasses,
            // registry-axis-ok: definition_body — class-definition relation keywords; until the definition-body grammar exposes relation kinds
            Some("mixin") => &class.mixins,
            _ => return,
        };
        for written in names.iter().filter(|name| texts[1..].contains(name)) {
            let lookup = self.declaration_name_policy().and_then(|policy| {
                crate::signature_scan::scope::SignatureSourceLookup::new(
                    policy,
                    self.declaration_namespace_scope(scope_path)?,
                    written.clone(),
                )
            });
            match class.relation_lookups.entry(written.clone()) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(lookup);
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    if entry.get() != &lookup {
                        entry.insert(None);
                    }
                }
            }
        }
    }

    /// Walk one class-level `initialise` / `initialize` body in a scope
    /// **keyed on the class**, and report the variables it declared there.
    ///
    /// The body is not a method — calling `self` / `classvariable` / `link`
    /// inside it is a runtime error ("`self` may only be called from inside
    /// a method", tclsh 9.0.4) — but it does run in a *per-class* frame:
    /// `namespace current` there is that class object's own namespace
    /// (`::oo::Obj20` for one class, `::oo::Obj22` for the next), with
    /// `namespace path` = `::oo::Helpers ::oo`. Walking every sibling
    /// class's init body in the one shared enclosing scope collided their
    /// declarations: a `classvariable NAME` read in *any* class's property
    /// setter then resolved to whichever class came first in the file, and
    /// find-references merged both classes' declarations into a single
    /// symbol — a definitive wrong answer on a statically decidable case,
    /// and one a rename would act on. tclsh 9.0.4 keeps
    /// the two independent: two `oo::configurable` classes whose setters
    /// each check their own `initialize`-declared list correctly reject the
    /// other's values.
    ///
    /// Returned names are seeded into this class's method bodies by the
    /// caller, which is what makes a setter's `classvariable NAME` read
    /// reach its own class's declaration. Deliberately unconditional, the
    /// same approximation the instance-`variable` seeding beside it already
    /// makes: the analyser does not track which methods actually issued the
    /// `classvariable` link, so a name declared in the init body is treated
    /// as visible (defined, never unused-warned) throughout the class.
    ///
    /// Walked inline rather than through `deferred_bodies` — a class-level
    /// init script is not one of the per-item-rebuildable method bodies,
    /// and this is the same inline walk it always had.
    fn walk_class_init_body(
        &mut self,
        class_variables: &[String],
        class_qualified: &str,
        scope_path: &[usize],
        mb: &CollectedMethodBody,
    ) -> Vec<(String, Span)> {
        if mb.body_tok.kind != TokenType::Str {
            return Vec::new();
        }
        let init_qn = if class_qualified.is_empty() {
            mb.name.clone()
        } else {
            crate::naming::rooted_member_key(class_qualified, &mb.name)
        };
        let Some(init_idx) =
            scope_at_mut(&mut self.result.global_scope, scope_path).map(|parent| {
                let mut child = Scope::new(ScopeKind::Method, init_qn);
                child.body_span = Some(mb.body_tok.span);
                // The object frame resolves bare commands globally, exactly as a
                // method body's does — `namespace path` is `::oo::Helpers ::oo`
                // and the class's *defining* namespace is not searched.
                child.oo_global_resolution = true;
                // ...but it is NOT a method invocation, so `oo_method_frame`
                // stays false. The two facts
                // diverge here and nowhere else: the family *resolves*
                // (`namespace which -command link` → `::oo::Helpers::link`,
                // so W123 must stay silent) yet all but `my` raise `… may
                // only be called from inside a method` when called, so
                // completion and hover must not offer them. `my` is the
                // exception because it is `::oo::ObjN::my` rather than an
                // `::oo::Helpers` member and a class is an object — `my new`
                // in an `initialize` body really does make an instance.
                child.oo_method_frame = false;
                parent.children.push(child);
                parent.children.len() - 1
            })
        else {
            return Vec::new();
        };
        let mut init_path = scope_path.to_vec();
        init_path.push(init_idx);
        let body_start = mb.body_tok.span.start();
        for var in class_variables {
            let base =
                tcl_syntax::naming::split_element_ref(var).map_or(var.as_str(), |(root, _)| root);
            if base.is_empty() {
                continue;
            }
            self.define_var(
                base,
                mb.body_tok,
                &init_path,
                false,
                Some(Span::new(body_start, body_start)),
            );
        }
        let seeded: std::collections::HashSet<String> = class_variables
            .iter()
            .map(|v| {
                tcl_syntax::naming::split_element_ref(v)
                    .map_or(v.as_str(), |(root, _)| root)
                    .to_owned()
            })
            .collect();
        self.analyse_body(&mb.body_text, mb.body_tok, &init_path);
        // Whatever the walk itself recorded in this scope — no second,
        // name-matching parser here; the registry-driven walk already knows
        // which words a `variable` / `const` statement declares.
        super::scope::scope_at(&self.result.global_scope, &init_path).map_or_else(
            Vec::new,
            |scope| {
                scope
                    .variables
                    .iter()
                    .filter(|(name, _)| !seeded.contains(name.as_str()))
                    .map(|(name, def)| (name.clone(), def.definition_span))
                    .collect()
            },
        )
    }

    /// Collect display-only alias/target pairs from the selected catalogue.
    /// Positive source consumers authenticate original invocation receipts.
    fn collect_oo_links(&self, method_bodies: &[CollectedMethodBody], class_def: &mut ClassDef) {
        for mb in method_bodies {
            if mb.body_tok.kind != TokenType::Str {
                continue;
            }
            let base = mb.body_tok.span.start() + u32::from(mb.body_tok.content_offset);
            let cmds = crate::segmenter::segment_commands_with_offset_and_config(
                &mb.body_text,
                base,
                self.file_lexer_config().nested(),
            );
            for cmd in &cmds {
                if cmd.is_partial
                    || !cmd.texts.first().is_some_and(|head| {
                        let generation = self.analysis_context();
                        generation
                            .context()
                            .resolve_spec(generation.commands(), head)
                            .is_some_and(|spec| {
                                spec.traits.contains(Traits::TCLOO_BINDS_METHOD_ALIAS)
                            })
                    })
                {
                    continue;
                }
                for word in cmd.texts.iter().skip(1) {
                    if crate::naming::is_dynamic_word(word) {
                        continue;
                    }
                    match crate::tcl_expr_eval::split_tcl_list(word, self.word_rules()).as_slice() {
                        [name] if !name.is_empty() => {
                            let key = {
                                let written = tcl_syntax::naming::canonical_written_command(name);
                                tcl_syntax::naming::unroot_rooted_key(&written)
                                    .unwrap_or(&written)
                                    .to_owned()
                            };
                            class_def.linked_members.entry(key.clone()).or_insert(key);
                        }
                        [name, target] if !name.is_empty() => {
                            class_def
                                .linked_members
                                .entry({
                                    let written =
                                        tcl_syntax::naming::canonical_written_command(name);
                                    tcl_syntax::naming::unroot_rooted_key(&written)
                                        .unwrap_or(&written)
                                        .to_owned()
                                })
                                .or_insert_with(|| target.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Walk a single `TclOO` method body in a fresh [`ScopeKind::Method`] scope.
    ///
    /// Pre-binds the method's formal parameters and the class's instance
    /// `variable`s as defined-but-not-warned locals (so reads of them do not
    /// false-fire W210 read-before-set / W214 unused), then re-walks the body
    /// through [`Self::analyse_body`] so its `$obj method` / `[cmd] method`
    /// dispatch sites are recorded (with `in_method = true`) for the W307 /
    /// W308 post-pass.
    fn walk_method_body(
        &mut self,
        class_variables: &[String],
        var_decl_spans: &std::collections::HashMap<String, Span>,
        class: MethodBodyClass<'_>,
        scope_path: &[usize],
        mb: &CollectedMethodBody,
        oo_global_resolution: bool,
    ) {
        if mb.body_tok.kind != TokenType::Str {
            return;
        }
        let class_qualified = class.display;
        let original_context = (oo_global_resolution && !mb.class_side)
            .then(|| {
                super::types::OriginalLexicalMemberContext::from_class_body(
                    class.declaration?,
                    &mb.body_tok,
                    mb.params_tok.as_ref(),
                    self.resolved_analysis_input(),
                    &tcl_lexer::SourceImage::document(&self.source),
                    self.file_lexer_config(),
                )
            })
            .flatten()
            .map(Box::new);
        let original_body_declaration = oo_global_resolution
            .then(|| {
                super::types::OriginalSourceReceiverBodyDeclaration::from_class_body(
                    class.declaration?,
                    &mb.body_tok,
                    mb.params_tok.as_ref(),
                    self.resolved_analysis_input(),
                    &tcl_lexer::SourceImage::document(&self.source),
                )
            })
            .flatten();
        let method_qn = if class_qualified.is_empty() {
            mb.name.clone()
        } else {
            crate::naming::rooted_member_key(class_qualified, &mb.name)
        };
        // Instance-side `TclOO` method of a statically-named class:
        // `[self class]` answers the defining class in this frame, so
        // record the fact for the constant command-substitution fold.
        // `oo_global_resolution` is exactly the
        // `TclOO`-family gate (snit / itcl walkers pass `false`);
        // class-side members abstain (`self class` never answers the
        // written class there); a synthetic key (`::@objdefine@…`) is
        // not a class name.
        let oo_defining_class = (oo_global_resolution
            && !mb.class_side
            && !class_qualified.is_empty()
            && !class_qualified.contains('@'))
        .then(|| class_qualified.to_owned());
        let Some(method_idx) = ({
            scope_at_mut(&mut self.result.global_scope, scope_path).map(|parent| {
                let mut child = Scope::new(ScopeKind::Method, method_qn.clone());
                child.body_span = Some(mb.body_tok.span);
                child.original_member_context = original_context;
                child.original_receiver_body_declaration = original_body_declaration;
                child.oo_global_resolution = oo_global_resolution;
                // A real method invocation, unlike the class-level
                // `initialise` frame `walk_class_init_body` opens: the whole
                // `oo::Helpers` family is callable here, not merely
                // resolvable (tclsh 9.0.4 inside a constructor: `link zzz`
                // returns `::oo::ObjN::zzz`).
                child.oo_method_frame = oo_global_resolution;
                child.oo_defining_class.clone_from(&oo_defining_class);
                parent.children.push(child);
                parent.children.len() - 1
            })
        }) else {
            return;
        };
        let mut method_path = scope_path.to_vec();
        method_path.push(method_idx);
        // Formal parameters — defined, never unused-warned.  Anchor each
        // param's definition span at its name in the param-list literal so
        // go-to-definition / references / rename resolve to the
        // parameter, not the whole method body.  Falls back to the body token
        // when the param-list word or a name can't be located.
        let param_spans = mb
            .params_tok
            .map(|pt| param_name_spans_for_token(&self.source, pt));
        for (i, p) in mb.params.iter().enumerate() {
            let def_span = param_spans.as_ref().and_then(|s| s.get(i).copied());
            self.define_var(&p.name, mb.body_tok, &method_path, false, def_span);
        }
        if let Some(pt) = mb.params_tok {
            self.emit_w218_args_not_final(&mb.params, param_spans.as_deref().unwrap_or(&[]), pt);
        }
        // Class instance variables — visible in every method body.  Anchor
        // each one's definition span at its `variable v` declaration token
        // A seeded var must NEVER take the whole method-body span as its
        // `definition_span`, or a rename would rewrite the entire body.  When
        // the declaration span is unknown (e.g. a snit implicit var) fall back
        // to a zero-width span at the body start — harmless, never destructive.
        let body_start = mb.body_tok.span.start();
        for var in class_variables {
            let base =
                tcl_syntax::naming::split_element_ref(var).map_or(var.as_str(), |(root, _)| root);
            if base.is_empty() || mb.params.iter().any(|p| p.name == base) {
                continue;
            }
            let def_span = var_decl_spans
                .get(base)
                .copied()
                .unwrap_or_else(|| Span::new(body_start, body_start));
            self.define_var(base, mb.body_tok, &method_path, false, Some(def_span));
        }
        self.finish_method_body_walk(
            class_variables,
            mb,
            scope_path,
            method_path,
            method_qn,
            (oo_defining_class, oo_global_resolution),
        );
    }

    fn finish_method_body_walk(
        &mut self,
        class_variables: &[String],
        mb: &CollectedMethodBody,
        scope_path: &[usize],
        method_path: Vec<usize>,
        method_qn: String,
        (oo_defining_class, oo_global_resolution): (Option<String>, bool),
    ) {
        // Per-item shell pass: defer the method body for an isolated pass like
        // `handle_proc_command`. Carry the qualified name, defining namespace,
        // formal parameters, and pre-bound class instance variables.
        if self.defer_proc_bodies {
            let namespace = self.command_resolution_namespace(scope_path);
            let safe_interp_ctx = self.safe_interp_ctx_snapshot();
            self.deferred_bodies.push(super::per_item::DeferredBody {
                resolved_input: Some(self.resolved_analysis_input()),
                body_text: std::sync::Arc::from(mb.body_text.as_str()),
                body_tok: mb.body_tok,
                scope_path: method_path,
                is_method: true,
                oo_global_resolution,
                namespace,
                scope_name: method_qn,
                params: mb.params.clone(),
                // The shell walk already seeded real declaration spans; the
                // graft retains them, so this pass needs only variable names.
                seeded_variables: class_variables.to_vec(),
                command_trust: None,
                ensemble_targets: Vec::new(),
                prefixless_ensembles: Vec::new(),
                oo_defining_class,
                safe_interp_ctx,
            });
        } else {
            self.analyse_body(&mb.body_text, mb.body_tok, &method_path);
        }
    }

    /// Handle a snit (tcllib) type/widget definition —
    /// ``snit::type``/``snit::widget``/``snit::widgetadaptor Name { body }``
    /// (and their `::`-qualified forms).  Snit reinterprets its body as a
    /// class description, exactly like an `oo::class` body, so we model it as
    /// a real [`ClassDef`] with method scopes — object dispatch inside method
    /// bodies (`$self foo`, `$component bar`) is then recognised as in-method
    /// dispatch (no false W307) and snit's implicit instance variables don't
    /// surface as read-before-set / unused.
    pub(super) fn handle_snit_type_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        self.handle_snit_type_source(super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            cmd_tok: None,
            original: None,
        })
    }

    pub(super) fn handle_snit_type_source(
        &mut self,
        call: super::original_definer::ClassDefinerCall<'_>,
    ) -> bool {
        let super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            original,
            ..
        } = call;
        // Which commands are snit definers, their member sub-keywords + argument
        // layout, and the variables snit injects into member bodies are all
        // registry data (a `Snit`-family definition-body grammar) — not a
        // hardcoded name / member / implicit-var list.  A new snit-like definer
        // is picked up automatically once its spec carries the grammar.  The
        // grammar is `&'static`, so it outlives the immutable registry borrow
        // released here before the `&mut self` work below.
        let Some(grammar) = original
            .map(super::original_definer::OriginalClassDefinerSource::grammar)
            .or_else(|| self.definition_grammar(cmd_name))
            .filter(|g| g.family == tcl_registry::definer::DefinerFamily::Snit)
        else {
            return false;
        };
        if args.len() < 2 || arg_tokens.len() < 2 {
            return false;
        }
        let raw_name = &args[0];
        let body = &args[1];
        // A relative type name homes to the namespace current at the call —
        // the command-resolution namespace, so a `snit::type` created inside
        // `proc ::ns::p {}` becomes `::ns::T`.
        let ns_prefix = self.command_resolution_namespace(scope_path);
        // Constructed key in, construction-inverse tail out: a colon
        // trim or `rsplit("::")` would collapse a lone-colon name.
        let qualified = super::handlers::qualify(&ns_prefix, raw_name);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = arg_tokens[0].span;
        // **W314** — the class name has no absolute written form.
        self.emit_w314_no_absolute_name(raw_name, name_span);
        let body_tok = arg_tokens[1];
        let doc = std::mem::take(&mut self.last_comment);
        let original_name = original
            .and_then(super::original_definer::OriginalClassDefinerSource::original_name)
            .or_else(|| self.original_provider_class_name(name_span));
        let source_name = original_name.as_ref().and_then(|original| {
            crate::signature_scan::scope::SignatureSourceCommand::provider_advice_from_key(
                &self.declaration_namespace_scope(scope_path)?,
                original.name_input(),
            )
        });
        let mut class = ClassDef {
            source_name,
            name: simple.clone(),
            qualified_name: qualified.clone(),
            name_span,
            body_span: body_tok.span,
            metaclass: cmd_name.to_string(),
            metaclass_provenance: super::types::MetaclassProvenance::Observed,
            doc,
            ..Default::default()
        };
        if !body.is_empty() {
            let definer = SnitDefiner { grammar };
            self.parse_snit_definition_body(
                body, body_tok, &mut class, &qualified, scope_path, &definer,
            );
        }
        // For `my`-dispatch resolution — see
        // `class_body_spans`'s doc.
        self.result
            .class_body_spans
            .push((qualified.clone(), class.body_span));
        self.retain_original_provider_class(original_name.as_ref(), &class);
        self.result
            .retain_class_declaration(qualified, class.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path) {
            scope.classes.insert(simple, class);
        }
        true
    }

    /// Parse a snit type/widget body into methods + variable declarations, in
    /// two passes (so a method can reference any instance/type variable
    /// regardless of declaration order).
    fn parse_snit_definition_body(
        &mut self,
        body: &str,
        body_tok: Token,
        class_def: &mut ClassDef,
        class_qualified: &str,
        scope_path: &[usize],
        definer: &SnitDefiner,
    ) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        let grammar = definer.grammar;
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let cmds = crate::segmenter::segment_commands_with_offset_and_config(
            body,
            base_offset,
            self.file_lexer_config().nested(),
        );

        // The variables snit injects into every member body come from the
        // definer's registry grammar (`implicit_vars`); the widget grammar's
        // list already includes the widget-only `win` / `hull` pair.
        let implicit_vars = grammar.implicit_vars;
        let mut instance_vars: Vec<String> =
            implicit_vars.iter().map(|s| (*s).to_string()).collect();
        let mut type_vars: Vec<String> = SNIT_TYPE_IMPLICIT
            .iter()
            .map(|s| (*s).to_string())
            .collect();

        // First pass: collect declared instance / type variable + component
        // names.  Which members declare state, and whose, is each statement's
        // row (`StateDeclaration`'s scope: per-type state is the type
        // variables', per-instance the instance variables'); which argument
        // holds the name is the member's registry layout.
        let context = self.analysis_context();
        let dialect = Some(context.context().authoring_query());
        for cmd in &cmds {
            if cmd.is_partial {
                continue;
            }
            let Some(statement) = MemberStatement::read(grammar, &cmd.texts, &cmd.argv, dialect)
            else {
                continue;
            };
            let MemberLanding::State(scope) = statement.landing() else {
                continue;
            };
            let args = statement.args();
            let Some(name) = statement
                .member
                .indices_for_call_in(args, dialect, ArgRole::VarWrite)
                .next()
                .and_then(|i| args.get(i))
            else {
                continue;
            };
            match scope {
                StateScope::PerType => type_vars.push(name.clone()),
                StateScope::PerInstance => instance_vars.push(name.clone()),
                // An option is configured through the instance, not read as
                // a variable of its own.
                StateScope::Option => {}
            }
        }

        // Record the *explicit* instance + type variables on the class —
        // method-scope seeding and the W307 dispatch-source suppression both
        // read `ClassDef::variables`.  The base grammar's implicit scalars
        // (`self`/`selfns`/`type`/`options` — `SNIT_GRAMMAR.implicit_vars`,
        // shared by every snit definer) and the type-implicit `type` are
        // filtered; a widget's injected `win`/`hull` (the extra names its
        // own grammar adds on top of the base set) are kept — they are real
        // per-instance variables worth surfacing on the class record.
        let base_implicits = tcl_registry::definer::SNIT_GRAMMAR.implicit_vars;
        class_def.variables = instance_vars
            .iter()
            .filter(|v| !base_implicits.contains(&v.as_str()))
            .chain(
                type_vars
                    .iter()
                    .filter(|v| !SNIT_TYPE_IMPLICIT.contains(&v.as_str())),
            )
            .cloned()
            .collect();

        // Second pass: analyse method-bearing declarations in method scopes.
        for cmd in &cmds {
            if cmd.is_partial || cmd.texts.is_empty() {
                continue;
            }
            self.retain_original_member_metadata(grammar, cmd, class_def, scope_path);
            let mut ctx = ClassBodyCtx {
                grammar,
                class_def,
                class_qualified,
                scope_path,
            };
            self.dispatch_snit_member(cmd, &mut ctx, &instance_vars, &type_vars);
        }
    }

    /// Dispatch one snit body statement by its row: a callable or a
    /// definition-time script to the method extractor, a namespace procedure
    /// to the ordinary proc handler.  Split out of
    /// [`Self::parse_snit_definition_body`] so the two-pass walk stays small.
    ///
    /// Recognition, argument layout (which words are the name / parameter list
    /// / value var / body) and what the statement declares all come from its
    /// registry row ([`member_landing`], the routing the `TclOO` fold shares):
    /// a type-object row's body sees the type variables and lands among the
    /// class methods, an instance row's sees the instance variables.  Only the
    /// synthetic name of the nameless forms is decided here.
    fn dispatch_snit_member(
        &mut self,
        cmd: &crate::segmenter::SegmentedCommand,
        ctx: &mut ClassBodyCtx<'_>,
        instance_vars: &[String],
        type_vars: &[String],
    ) {
        let context = self.analysis_context();
        let dialect = Some(context.context().authoring_query());
        let (sub_args, sub_tokens) = (&cmd.texts[1..], cmd.argv.get(1..).unwrap_or(&[]));
        if let Some(member) = ctx.grammar.member(&cmd.texts[0]) {
            let parameter_indices = self.member_formal_parameter_indices(
                ctx.grammar,
                member,
                sub_args,
                sub_tokens,
                ctx.class_def,
            );
            super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
                self,
                sub_args,
                sub_tokens,
                &parameter_indices,
            );
        }
        let Some(statement) = MemberStatement::read(ctx.grammar, &cmd.texts, &cmd.argv, dialect)
        else {
            return;
        };
        match statement.landing() {
            // A type-private `proc name args body` is a procedure in the
            // type's namespace — analyse it as an ordinary proc in the
            // enclosing scope, not a method.
            MemberLanding::Procedure => {
                // No per-argument single-token info is threaded this deep into
                // the snit member dispatcher; `&[]` is the same safe default
                // `resolve_dynamic_word` already falls back to elsewhere (a
                // dynamic type-private proc name still gets a chance to resolve
                // via `fold_interpolation_single`, just not the single-`$var`
                // fast path).
                self.handle_proc_command(
                    statement.keyword(),
                    statement.args(),
                    statement.arg_tokens(),
                    &[],
                    ctx.scope_path,
                );
            }
            MemberLanding::Method(kind) => {
                let seed_vars = if statement.row.receiver == MemberReceiver::TypeObject {
                    type_vars
                } else {
                    instance_vars
                };
                let label = member_label(statement.member, statement.keyword(), statement.args());
                let form = MemberForm {
                    member: statement.member,
                    kind,
                    label: &label,
                    visibility: statement.row.visibility.as_str(),
                };
                self.extract_class_member(
                    statement.args(),
                    statement.arg_tokens(),
                    ctx,
                    seed_vars,
                    &form,
                );
            }
            _ => {}
        }
    }

    /// Analyse one snit method / constructor / etc. body in a method scope
    /// seeded with `seed_vars` (snit's implicit names + declared instance or
    /// type variables) and the method's formal parameters.  The name /
    /// parameter-list / value-var / body word positions all come from
    /// `member`'s registry arg-roles — a body-less member is a no-op.
    fn extract_class_member(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        ctx: &mut ClassBodyCtx<'_>,
        seed_vars: &[String],
        form: &MemberForm<'_>,
    ) {
        let MemberForm {
            member,
            kind,
            label,
            visibility,
        } = *form;
        let context = self.analysis_context();
        let mask = Some(context.context().authoring_query());
        let Some(body_idx) = crate::registry_invocation::source_structure::trace_jim_source_option(
            "member.body_role",
            member.indices_for_call_in(args, mask, ArgRole::Body).next(),
        ) else {
            return;
        };
        let Some(body_text) = args.get(body_idx).cloned() else {
            return;
        };
        let body_tok = crate::registry_invocation::source_structure::trace_jim_source_option(
            "member.body_token",
            arg_tokens.get(body_idx).copied(),
        );
        let name_idx = member.indices_for_call_in(args, mask, ArgRole::Name).next();
        let params_idx = member
            .indices_for_call_in(args, mask, ArgRole::ParamList)
            .next();

        // A named member (`method NAME …`) uses its name word; the nameless
        // forms fall back to the caller's synthetic label (`<constructor>`,
        // `<oncget -opt>`, …), or `<body>` when even that is empty.
        let name = name_idx
            .and_then(|i| args.get(i))
            .cloned()
            .unwrap_or_else(|| {
                if label.is_empty() {
                    "<body>".to_string()
                } else {
                    label.to_string()
                }
            });
        // Formal parameters: the parameter-list word, plus any `VarWrite` word
        // (snit 1.x `onconfigure`'s value variable) modelled as a bound local.
        let mut params: Vec<ParamDef> = params_idx
            .and_then(|i| args.get(i))
            .map_or_else(Vec::new, |p| parse_param_list(p, self.word_rules()));
        for i in member.indices_for_call_in(
            args,
            Some(self.analysis_context().context().authoring_query()),
            ArgRole::VarWrite,
        ) {
            if let Some(v) = args.get(i) {
                params.push(ParamDef {
                    name: v.clone(),
                    has_default: false,
                    default_value: None,
                });
            }
        }
        let params_tok = params_idx.and_then(|i| arg_tokens.get(i).copied());

        let zero = Span::new(0, 0);
        let name_span = name_idx
            .and_then(|i| arg_tokens.get(i))
            .or_else(|| arg_tokens.first())
            .map_or(zero, |t| t.span);
        let body_span = body_tok.map_or(name_span, |t| t.span);
        let method_def = MethodDef {
            name: name.clone(),
            params: params.clone(),
            params_computed: false,
            formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                tcl_dialect::ParameterGrammar::Tcl,
            ),
            name_span,
            body_span,
            kind: kind.as_str().to_string(),
            is_self_method: false,
            visibility: visibility.to_string(),
            doc: String::new(),
            forward_target: None,
        };
        record_method(ctx.class_def, kind, method_def);

        // Walk the body in a method scope seeded with the params + seed vars,
        // reusing the TclOO method-body walker (it pre-binds the params and the
        // supplied vars as never-warn locals, then analyses the body). snit /
        // itcl members resolve in the type / class namespace, so
        // `oo_global_resolution` stays false here.
        if let Some(bt) = body_tok {
            let mb = CollectedMethodBody {
                name,
                params,
                body_text,
                body_tok: bt,
                params_tok,
                // snit / itcl members are not `TclOO` frames — no
                // `[self class]` fact may be derived from them.
                class_side: true,
            };
            // snit / itcl seed vars are mostly grammar-injected implicits with no
            // source declaration token; an empty span map makes `walk_method_body`
            // fall back to a safe zero-width span (never the body span), so a
            // rename can't overwrite the body.  Precise snit declaration spans
            // are not modelled.
            let no_var_spans = std::collections::HashMap::new();
            self.walk_method_body(
                seed_vars,
                &no_var_spans,
                MethodBodyClass {
                    display: ctx.class_qualified,
                    declaration: None,
                },
                ctx.scope_path,
                &mb,
                false,
            );
        }
    }

    /// Handle an [incr Tcl] `itcl::class Name { body }` (family `Itcl`),
    /// modelling it as a [`ClassDef`] with method scopes exactly like
    /// `oo::class` / `snit::type`, so `$this method` dispatch and reads of the
    /// instance / `common` variables inside method bodies don't false-fire.
    /// The access modifiers `public` / `protected` / `private` are prefix
    /// wrappers (registry `MemberKind::Wrapper`) and are unwrapped here.
    pub(super) fn handle_itcl_class_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        self.handle_itcl_class_source(super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            cmd_tok: None,
            original: None,
        })
    }

    pub(super) fn handle_itcl_class_source(
        &mut self,
        call: super::original_definer::ClassDefinerCall<'_>,
    ) -> bool {
        let super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            original,
            ..
        } = call;
        let Some(grammar) = original
            .map(super::original_definer::OriginalClassDefinerSource::grammar)
            .or_else(|| self.definition_grammar(cmd_name))
            .filter(|g| g.family == tcl_registry::definer::DefinerFamily::Itcl)
        else {
            return false;
        };
        if args.len() < 2 || arg_tokens.len() < 2 {
            return false;
        }
        let raw_name = &args[0];
        let body = &args[1];
        // As for snit: a relative itcl class name homes to the namespace
        // current at the call, not the lexical one.
        let ns_prefix = self.command_resolution_namespace(scope_path);
        // Constructed key in, construction-inverse tail out: a colon
        // trim or `rsplit("::")` would collapse a lone-colon name.
        let qualified = super::handlers::qualify(&ns_prefix, raw_name);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = arg_tokens[0].span;
        // **W314** — the class name has no absolute written form.
        self.emit_w314_no_absolute_name(raw_name, name_span);
        let body_tok = arg_tokens[1];
        let doc = std::mem::take(&mut self.last_comment);
        let original_name = original
            .and_then(super::original_definer::OriginalClassDefinerSource::original_name)
            .or_else(|| self.original_provider_class_name(name_span));
        let source_name = original_name.as_ref().and_then(|original| {
            crate::signature_scan::scope::SignatureSourceCommand::provider_advice_from_key(
                &self.declaration_namespace_scope(scope_path)?,
                original.name_input(),
            )
        });
        let mut class = ClassDef {
            source_name,
            name: simple.clone(),
            qualified_name: qualified.clone(),
            name_span,
            body_span: body_tok.span,
            metaclass: cmd_name.to_string(),
            metaclass_provenance: super::types::MetaclassProvenance::Observed,
            doc,
            ..Default::default()
        };
        if !body.is_empty() {
            self.parse_itcl_definition_body(
                body, body_tok, &mut class, &qualified, scope_path, grammar,
            );
        }
        // For `my`-dispatch resolution — see
        // `class_body_spans`'s doc.
        self.result
            .class_body_spans
            .push((qualified.clone(), class.body_span));
        self.retain_original_provider_class(original_name.as_ref(), &class);
        self.result
            .retain_class_declaration(qualified, class.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path) {
            scope.classes.insert(simple, class);
        }
        true
    }

    /// Handle a `JimClass` definer — `class NAME ?BASES? VARS` — recording the
    /// class as a [`ClassDef`] with its base classes and the instance
    /// variables `VARS` names.
    ///
    /// `VARS` is a dictionary of every instance variable and its initial
    /// value, not a script, so it is read for its keys and never walked as a
    /// body. The class defines no member itself: methods arrive later, through
    /// [`Self::handle_jim_class_member_call`] or a two-word `proc`
    /// ([`Self::record_two_word_proc_member`]).
    pub(super) fn handle_jim_class_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        cmd_tok: Token,
    ) -> bool {
        self.handle_jim_class_source(super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            cmd_tok: Some(cmd_tok),
            original: None,
        })
    }

    pub(super) fn handle_jim_class_source(
        &mut self,
        call: super::original_definer::ClassDefinerCall<'_>,
    ) -> bool {
        let super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            original: _,
            cmd_tok,
        } = call;
        let Some(authored) = cmd_tok
            .and_then(|token| {
                crate::registry_invocation::source_structure::source_authored_jim_class_at(
                    &self.source,
                    &self.result,
                    token.span.start(),
                )
            })
            .filter(|source| {
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "ingress.arguments",
                    source.matches_arguments(args, arg_tokens).then_some(()),
                )
                .is_some()
            })
        else {
            return false;
        };
        // The definer's own arity is 2 or 3; any other shape is left to the
        // arity check.
        if !(2..=3).contains(&args.len()) || arg_tokens.len() != args.len() {
            return false;
        }
        let ns_prefix = self.command_resolution_namespace(scope_path);
        let qualified = super::handlers::qualify(&ns_prefix, &args[0]);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = arg_tokens[0].span;
        // **W314** — the class name has no absolute written form.
        self.emit_w314_no_absolute_name(&args[0], name_span);

        let vars_at = args.len() - 1;
        let OriginalJimClassLists {
            bases_dynamic,
            superclasses,
            variable_words,
        } = Self::original_jim_class_lists(&authored);
        self.report_original_jim_dictionary_shape(cmd_name, &variable_words, arg_tokens[vars_at]);
        let own_variables: Vec<String> = variable_words.into_iter().step_by(2).collect();
        // A derived class's variable dictionary is its bases' merged with its
        // own, so a method body sees every inherited instance variable too.
        let mut variables = authored.inherited_variables(&self.result);
        for variable in own_variables {
            if !variables.contains(&variable) {
                variables.push(variable);
            }
        }

        let doc = std::mem::take(&mut self.last_comment);
        let mut class = ClassDef {
            // The distinct authored source receipt supplies no Native class name.
            source_name: None,
            name: simple,
            qualified_name: qualified.clone(),
            name_span,
            body_span: arg_tokens[vars_at].span,
            metaclass: cmd_name.to_string(),
            metaclass_provenance: super::types::MetaclassProvenance::Observed,
            superclasses,
            variables,
            inheritance_unknown: bases_dynamic,
            doc,
            ..Default::default()
        };
        // Source-only base relations never donate a Native naming lookup.
        for base in &class.superclasses {
            class.relation_lookups.insert(base.clone(), None);
        }
        self.register_defined_class(qualified.clone(), class, scope_path);
        self.record_authored_jim_procedure_members_at(
            authored.factory().site().offset,
            None,
            scope_path,
        );
        self.adopt_recorded_two_word_procs(&qualified, scope_path, None);
        true
    }

    pub(super) fn retained_source_advice_purpose(
        &self,
    ) -> Option<crate::command_binding::OriginalInvocationSourcePurpose> {
        let input = self
            .resolved_input
            .as_ref()
            .or(self.result.resolved_input.as_ref())?;
        let config = self.file_lexer_config();
        if input.lexer_config() != config {
            return None;
        }
        let context = self.analysis_context();
        let registry = context.commands();
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, input,
        )?;
        self.head_identities
            .source_bindings_ref()
            .original_source_entry_purpose(
                &tcl_lexer::SourceImage::document(&self.source),
                config,
                metadata,
                registry,
            )
    }

    fn original_jim_class_lists(
        original: &crate::command_binding::OriginalAuthoredJimClass,
    ) -> OriginalJimClassLists {
        OriginalJimClassLists {
            bases_dynamic: original.bases_unknown(),
            superclasses: original
                .base_fields()
                .unwrap_or_default()
                .into_iter()
                .map(|field| field.value().to_owned())
                .collect(),
            variable_words: original
                .variable_fields()
                .unwrap_or_default()
                .iter()
                .map(|field| field.value().to_owned())
                .collect(),
        }
    }

    fn report_original_jim_dictionary_shape(
        &mut self,
        command: &str,
        words: &[String],
        token: Token,
    ) {
        if words.len().is_multiple_of(2) {
            return;
        }
        let span = super::utils::full_word_span(token, &self.source);
        self.result.diagnostics.push(super::types::Diagnostic::new(
            tcl_core_types::DiagCode::E005,
            span,
            format!("Wrong argument-count shape for '{command}': the variable dictionary has an odd number of elements, so '{}' has no value", words[words.len() - 1]),
            super::types::Severity::Error,
        ));
    }

    /// Attach genuine two-word procedure declarations to this source class.
    /// Native list children retain their own producer and naming scope; only
    /// explicit Logical compatibility may read reporting procedure names.
    fn adopt_recorded_two_word_procs(
        &mut self,
        class_q: &str,
        scope_path: &[usize],
        original: Option<&super::original_definer::OriginalClassDefinerSource>,
    ) {
        if let Some(original) = original {
            let calls: Vec<_> = self
                .result
                .original_procedure_declarations()
                .filter_map(|procedure| {
                    let (input, member) = original_two_word_proc_name(procedure)?;
                    let reference =
                        crate::registry_invocation::source_structure::source_class_reference_at(
                            &self.source,
                            &self.result,
                            original.declaration().factory().site().offset,
                            &input,
                        )?;
                    (reference.class_declaration().factory().site()
                        == original.declaration().factory().site())
                    .then(|| (reference, member, procedure.clone()))
                })
                .collect();
            for (reference, member, procedure) in calls {
                self.add_original_two_word_member(&reference, &member, &procedure, scope_path);
            }
            return;
        }
        if self.retained_source_advice_purpose()
            != Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical)
        {
            return;
        }
        let rules = self.word_rules();
        let calls: Vec<_> = self
            .result
            .all_procs
            .values()
            .filter_map(|procedure| {
                let words = rules.split_list(&procedure.name).ok()?;
                let [class, member] = words.as_slice() else {
                    return None;
                };
                (self.resolve_user_class_in(class, scope_path).as_deref() == Some(class_q))
                    .then(|| (member.to_string(), procedure.clone()))
            })
            .collect();
        for (member, procedure) in calls {
            self.add_two_word_member(class_q, &member, &procedure, scope_path);
        }
    }

    /// Class presentation for a setter whose single substitution is a selected
    /// authored Jim class call naming one of its definer grammar's
    /// manufacturers (`set p [Point new]`). Conditional source metadata only:
    /// no allocation, successful construction or receiver lifetime.
    pub(super) fn jim_class_from_constructor_subst(
        &self,
        site_offset: u32,
    ) -> Option<(String, String)> {
        use crate::registry_invocation::source_structure as structure;
        let construction =
            structure::source_handle_construction_at(&self.source, &self.result, site_offset)?;
        let offset = construction.construction().words.first()?.span().start();
        let call = structure::source_authored_jim_call_at(&self.source, &self.result, offset)?;
        let member = call.original_words().get(1)?.bytes();
        let context = self.result.resolved_input.as_ref()?.context_registry();
        let grammar = context
            .context()
            .resolve_spec(context.commands(), call.class().factory().command())?
            .definition_body?;
        (grammar.family == tcl_registry::definer::DefinerFamily::JimClass
            && std::str::from_utf8(member)
                .ok()
                .and_then(|member| grammar.manufacturer(member))
                .is_some_and(|manufacturer| manufacturer.names_instance_at.is_none()))
        .then_some(())?;
        let class = call.metadata(&self.result)?;
        Some((
            std::str::from_utf8(construction.variable_bytes())
                .ok()?
                .to_owned(),
            class.qualified_name.clone(),
        ))
    }

    /// Handle `CLASS member …` where `CLASS` is a `JimClass` class and `member`
    /// is one of its definer's own members (`CLASS method NAME ARGS BODY`),
    /// recording the member on the class and walking its body with the class
    /// variables and the grammar's implicit variables (`self`) bound.
    pub(super) fn handle_jim_class_member_call(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        command_token: Token,
    ) -> bool {
        if let Some(call) =
            crate::registry_invocation::source_structure::source_authored_jim_call_at(
                &self.source,
                &self.result,
                command_token.span.start(),
            )
        {
            let Some(mut class) =
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "jim.member.metadata",
                    call.metadata(&self.result).cloned(),
                )
            else {
                return false;
            };
            let context = self
                .result
                .resolved_input
                .as_ref()
                .map(super::input::ResolvedAnalysisInput::context_registry);
            let Some(context) = context else {
                return false;
            };
            let Some(grammar) = context
                .context()
                .resolve_spec(context.commands(), call.class().factory().command())
                .and_then(|spec| spec.definition_body)
            else {
                return false;
            };
            if grammar.family != tcl_registry::definer::DefinerFamily::JimClass {
                return false;
            }
            if crate::registry_invocation::source_structure::trace_jim_source_option(
                "jim.member.arguments",
                super::original_definer::logical_arguments_match(
                    call.original_words(),
                    args,
                    arg_tokens,
                )
                .then_some(()),
            )
            .is_none()
            {
                return false;
            }
            if !self.analyse_jim_inline_member(grammar, args, arg_tokens, &mut class, scope_path) {
                return false;
            }
            self.register_defined_class(class.qualified_name.clone(), class, scope_path);
            return true;
        }
        let original = super::original_definer::OriginalClassCallSource::capture(
            &self.source,
            &self.result,
            command_token.span.start(),
        );
        let (args, arg_tokens, mut class_def, grammar) = if let Some(original) = &original {
            let Some(class) = original.call.class_declaration().source_class(&self.result) else {
                return false;
            };
            (
                original.arguments.as_slice(),
                original.tokens.as_slice(),
                class.metadata().clone(),
                original.grammar,
            )
        } else {
            let Some(class_q) = self.resolve_user_class_in(cmd_name, scope_path) else {
                return false;
            };
            let Some(class) = self.result.all_classes.get(&class_q).cloned() else {
                return false;
            };
            let Some(grammar) = self.class_definer_grammar(&class_q) else {
                return false;
            };
            (args, arg_tokens, class, grammar)
        };
        if grammar.family != tcl_registry::definer::DefinerFamily::JimClass {
            return false;
        }
        if !self.analyse_jim_inline_member(grammar, args, arg_tokens, &mut class_def, scope_path) {
            return false;
        }
        let class_q = class_def.qualified_name.clone();
        if let Some(original) = &original {
            self.retain_original_source_class_update(original.call.class_declaration(), &class_def);
        }
        self.register_defined_class(class_q, class_def, scope_path);
        true
    }

    fn analyse_jim_inline_member(
        &mut self,
        grammar: &'static DefinitionBodyGrammar,
        args: &[String],
        arg_tokens: &[Token],
        class_def: &mut ClassDef,
        scope_path: &[usize],
    ) -> bool {
        let Some(keyword) = args.first() else {
            return false;
        };
        if arg_tokens.len() != args.len() {
            return false;
        }
        let Some(member) = crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.member.row",
            grammar.member(keyword),
        ) else {
            return false;
        };
        self.retain_original_inline_member_metadata(
            grammar, args, arg_tokens, class_def, scope_path,
        );
        let class_q = class_def.qualified_name.clone();
        let member_args = &args[1..];
        let member_tokens = &arg_tokens[1..];
        let parameter_indices = self.member_formal_parameter_indices(
            grammar,
            member,
            member_args,
            member_tokens,
            class_def,
        );
        super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
            self,
            member_args,
            member_tokens,
            &parameter_indices,
        );
        let mut seed_vars = class_def.variables.clone();
        seed_vars.extend(grammar.implicit_vars.iter().map(ToString::to_string));
        let mut ctx = ClassBodyCtx {
            grammar,
            class_def,
            class_qualified: &class_q,
            scope_path,
        };
        let form = MemberForm {
            member,
            kind: MethodKind::Method,
            label: "",
            visibility: "public",
        };
        self.extract_class_member(member_args, member_tokens, &mut ctx, &seed_vars, &form);
        true
    }

    /// Record a `proc` whose name is the two-element list `{CLASS member}` as a
    /// method of `CLASS` when `CLASS` is a class of a family that stores its
    /// members as two-word commands
    /// ([`tcl_registry::definer::DefinerFamily::members_are_two_word_commands`]).
    ///
    /// The procedure is itself the two-word command `{CLASS member}` — every
    /// dialect defines that — so this only adds the class-side fact. The
    /// method has no class variables and no `self` in its body: a raw
    /// definition is not wrapped by the class command's `method`. A class
    /// written after the `proc` adopts it when the class is defined
    /// ([`Self::adopt_recorded_two_word_procs`]).
    pub(super) fn record_two_word_proc_member(
        &mut self,
        proc_name: &str,
        proc: &super::types::ProcDef,
        scope_path: &[usize],
    ) {
        let Some(purpose) = crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.raw_member.purpose",
            self.retained_source_advice_purpose(),
        ) else {
            return;
        };
        if purpose != crate::command_binding::OriginalInvocationSourcePurpose::Logical {
            let original_offset =
                self.result
                    .original_procedure_declarations()
                    .find_map(|original| {
                        (original.metadata() == proc).then_some(original.declaration_site().offset)
                    });
            if let Some(offset) = original_offset
                && self.record_authored_jim_procedure_members_at(offset, Some(proc), scope_path)
            {
                return;
            }
            let selected = self
                .result
                .original_procedure_declarations()
                .find_map(|original| {
                    (original.metadata().name_span == proc.name_span
                        && original.metadata().body_span == proc.body_span
                        && original.metadata().source_name == proc.source_name)
                        .then_some(())?;
                    let (input, member) =
                        crate::registry_invocation::source_structure::trace_jim_source_option(
                            "jim.raw_member.name_children",
                            original_two_word_proc_name(original),
                        )?;
                    let reference =
                        crate::registry_invocation::source_structure::trace_jim_source_option(
                            "jim.raw_member.class_reference",
                            crate::registry_invocation::source_structure::source_class_reference_at(
                                &self.source,
                                &self.result,
                                original.declaration_site().offset,
                                &input,
                            ),
                        )?;
                    Some((reference, member, original.clone()))
                });
            if let Some((reference, member, procedure)) =
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "jim.raw_member.selected",
                    selected,
                )
            {
                self.add_original_two_word_member(&reference, &member, &procedure, scope_path);
            }
            return;
        }
        let Ok(words) = self.word_rules().split_list(proc_name) else {
            return;
        };
        let [class_word, member_word] = words.as_slice() else {
            return;
        };
        let Some(class_q) = self.resolve_user_class_in(class_word, scope_path) else {
            return;
        };
        self.add_two_word_member(&class_q, member_word, proc, scope_path);
    }

    fn record_authored_jim_procedure_members_at(
        &mut self,
        offset: u32,
        expected: Option<&super::types::ProcDef>,
        scope_path: &[usize],
    ) -> bool {
        let Some(references) =
            crate::registry_invocation::source_structure::trace_jim_source_option(
                "jim.own_member.recipes",
                crate::registry_invocation::source_structure::source_authored_jim_procedure_members_at(
                    &self.source, &self.result, offset,
                ),
            )
        else {
            return false;
        };
        let _ = crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.own_member.recipes_nonempty",
            (!references.is_empty()).then_some(()),
        );
        let mut recorded = false;
        for reference in references {
            let Some(procedure) =
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "jim.own_member.procedure",
                    reference.source_procedure(&self.result),
                )
            else {
                continue;
            };
            if expected.is_some_and(|expected| procedure.metadata() != expected) {
                continue;
            }
            let Some(canonical) =
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "jim.own_member.class",
                    reference.class_metadata(&self.result),
                )
            else {
                continue;
            };
            let mut class = canonical.clone();
            let Some(metadata) =
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "jim.own_member.association",
                    self.original_authored_jim_procedure_member(
                        &reference,
                        procedure,
                        reference.member(),
                        &class,
                    ),
                )
            else {
                continue;
            };
            let Ok(label) = std::str::from_utf8(reference.member().bytes()) else {
                continue;
            };
            insert_original_two_word_member(&mut class, label, procedure.metadata());
            class.original_members.declare(metadata);
            self.register_defined_class(class.qualified_name.clone(), class, scope_path);
            recorded = true;
        }
        recorded
    }

    fn add_original_two_word_member(
        &mut self,
        reference: &crate::command_binding::OriginalSourceClassReference,
        member: &crate::signature_scan::scope::SignatureSourceNameInput,
        procedure: &crate::signature_scan::original_name::SourceDeclarationMetadata<
            super::types::ProcDef,
        >,
        scope_path: &[usize],
    ) {
        let declaration = reference.class_declaration();
        let context = self.analysis_context();
        if !declaration
            .grammar(&context)
            .is_some_and(|grammar| grammar.family.members_are_two_word_commands())
        {
            return;
        }
        let Some(original) = declaration.source_class(&self.result) else {
            return;
        };
        let mut class = original.metadata().clone();
        let Ok(label) = std::str::from_utf8(member.bytes()) else {
            return;
        };
        insert_original_two_word_member(&mut class, label, procedure.metadata());
        if !self.retain_original_two_word_source_member(reference, procedure, member, &mut class) {
            return;
        }
        self.retain_original_source_class_update(declaration, &class);
        self.register_defined_class(class.qualified_name.clone(), class, scope_path);
    }

    /// Add `proc` to `class_q` as its method `member` when the class's family
    /// stores its members as two-word commands.
    fn add_two_word_member(
        &mut self,
        class_q: &str,
        member: &str,
        proc: &super::types::ProcDef,
        scope_path: &[usize],
    ) {
        if !self
            .class_definer_grammar(class_q)
            .is_some_and(|grammar| grammar.family.members_are_two_word_commands())
        {
            return;
        }
        let Some(mut class_def) = self.result.all_classes.get(class_q).cloned() else {
            return;
        };
        insert_original_two_word_member(&mut class_def, member, proc);
        self.register_defined_class(class_q.to_string(), class_def, scope_path);
    }

    /// Parse an itcl class body through each statement's registry row: a
    /// superclass relation (`inherit`) → superclasses, declared state
    /// (`variable` / `common`) → the variables in scope, a callable (`method` /
    /// `proc` / `constructor` / `destructor`) → a method scope in the table its
    /// row lands in.  Two passes (so a method can reference any variable
    /// regardless of declaration order); an access modifier is a wrapper whose
    /// shift the row already carries — its member's visibility.
    fn parse_itcl_definition_body(
        &mut self,
        body: &str,
        body_tok: Token,
        class_def: &mut ClassDef,
        class_qualified: &str,
        scope_path: &[usize],
        grammar: &'static DefinitionBodyGrammar,
    ) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let cmds = crate::segmenter::segment_commands_with_offset_and_config(
            body,
            base_offset,
            self.file_lexer_config().nested(),
        );

        // itcl injects `this` (the object's own command) into every method body.
        let implicit_vars = grammar.implicit_vars;
        let mut instance_vars: Vec<String> =
            implicit_vars.iter().map(|s| (*s).to_string()).collect();

        // Pass 1: declared state names (`variable` / `common`, under any
        // access modifier) + the ancestry an `inherit` names — each read off
        // the statement's row.
        let context = self.analysis_context();
        let dialect = Some(context.context().authoring_query());
        for cmd in &cmds {
            if cmd.is_partial || cmd.texts.is_empty() {
                continue;
            }
            // A base class an `inherit` names is a command reference (the same
            // registry-driven path TclOO's `superclass`/`mixin` use), so
            // find-references / go-to-definition / rename reach it across files.
            self.record_member_command_references(grammar, &cmd.texts, &cmd.argv, scope_path);
            let Some(statement) = MemberStatement::read(grammar, &cmd.texts, &cmd.argv, dialect)
            else {
                continue;
            };
            let args = statement.args();
            match statement.landing() {
                // Per-instance and per-type (`common`) state are both in scope
                // in an itcl method body.
                MemberLanding::State(_) => {
                    if let Some(name) = statement
                        .member
                        .indices_for_call_in(args, dialect, ArgRole::VarWrite)
                        .next()
                        .and_then(|i| args.get(i))
                    {
                        instance_vars.push(name.clone());
                    }
                }
                MemberLanding::Relation(RelationSlot::Superclass) => {
                    apply_slot_member(Some(statement.member), args, &mut class_def.superclasses);
                }
                _ => {}
            }
        }
        class_def.variables = instance_vars
            .iter()
            .filter(|v| !implicit_vars.contains(&v.as_str()))
            .cloned()
            .collect();

        // Pass 2: method-bearing members in their own method scopes.
        for cmd in &cmds {
            if cmd.is_partial {
                continue;
            }
            self.retain_original_member_metadata(grammar, cmd, class_def, scope_path);
            if let Some((keyword, kw_args, kw_toks)) =
                unwrap_wrapper_member(grammar, &cmd.texts, &cmd.argv)
                && let Some(member) = grammar.member(keyword)
            {
                let parameter_indices: Vec<usize> = member
                    .indices_for_call_in(kw_args, dialect, ArgRole::ParamList)
                    .collect();
                super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
                    self,
                    kw_args,
                    kw_toks,
                    &parameter_indices,
                );
            }
            // Only a body the row lands in a method table is a method scope:
            // `variable` / `common` are declarations (handled in pass 1) even
            // though `variable`'s optional config body carries an
            // `ArgRole::Body` (that body is highlighted by the token walker,
            // not recorded as a method here), and `inherit` carries no body.
            let Some(statement) = MemberStatement::read(grammar, &cmd.texts, &cmd.argv, dialect)
            else {
                continue;
            };
            let MemberLanding::Method(kind) = statement.landing() else {
                continue;
            };
            let label = member_label(statement.member, statement.keyword(), statement.args());
            let mut ctx = ClassBodyCtx {
                grammar,
                class_def,
                class_qualified,
                scope_path,
            };
            let form = MemberForm {
                member: statement.member,
                kind,
                label: &label,
                visibility: statement.row.visibility.as_str(),
            };
            self.extract_class_member(
                statement.args(),
                statement.arg_tokens(),
                &mut ctx,
                &instance_vars,
                &form,
            );
        }
    }

    /// Detect dispatch shape of a user-defined ``unknown`` proc.
    ///
    /// Lowers the proc body to IR, then walks the resulting
    /// top-level [`Statement`]s looking for:
    ///
    /// - Selected original case syntax referencing the first parameter —
    ///   exact labels become possible dispatch targets; glob/regexp modes
    ///   mark pattern dispatch. Canonical body-selection residuals remain
    ///   explicit. Selected case-folding child descriptors retain their own
    ///   original operand before setting ``case_insensitive``.
    /// - `Statement::Call` / `Statement::Barrier` that chains the original
    ///   handler ([`chains_original_unknown`]) — sets ``chains_original``.
    /// - a call that shells out ([`spawns_process`]) — sets ``has_exec``.
    /// - a call that loads more script ([`loads_external_unit`]) — sets
    ///   ``has_auto_load``.
    ///
    /// Empty bodies set ``empty_stub = true`` and skip the IR
    /// walk.  Lowering failures fall back to the conservative
    /// "fully dynamic" shape (every flag set, no targets) so
    /// downstream W123 emission stays suppressed.
    pub fn extract_unknown_proc_info(
        &mut self,
        body: &str,
        params: &[ParamDef],
    ) -> UnknownProcInfo {
        self.extract_unknown_proc_info_at(body, params, None)
    }

    /// Secondary body analysis uses the actual document source execution
    /// points when called by the procedure handler.
    pub(crate) fn extract_unknown_proc_info_at(
        &mut self,
        body: &str,
        params: &[ParamDef],
        base: Option<u32>,
    ) -> UnknownProcInfo {
        if body.trim().is_empty() {
            return UnknownProcInfo {
                empty_stub: true,
                ..Default::default()
            };
        }

        let first_param = params
            .first()
            .map_or_else(|| "cmd".to_string(), |p| p.name.clone());

        // Lower to IR.  On panic, be conservative — assume fully
        // dynamic — by returning an ``UnknownProcInfo`` with
        // every dynamic flag set so the W123 emitter suppresses
        // unresolved-command warnings file-wide (the safe
        // direction when we couldn't analyse the handler body).
        // naming.compiler.original-unknown-handler-source-context
        // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
        let context = self.analysis_context();
        let retained = self
            .resolved_input
            .as_ref()
            .or(self.result.resolved_input.as_ref());
        let config = retained.map_or_else(
            || self.lexer_config(),
            super::ResolvedAnalysisInput::lexer_config,
        );
        let profile = retained.map_or(self.profile, super::ResolvedAnalysisInput::unit_profile);
        let metadata = retained.and_then(|input| {
            crate::registry_invocation::InvocationMetadataContext::for_source_input(
                context.commands(),
                input,
                config,
                Some(profile),
            )
        });
        let Ok(script) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
                .with_dialect(Some(profile))
                .with_context_registry(std::sync::Arc::clone(&context));
            if let Some(input) = retained {
                lowerer = lowerer.with_resolved_analysis_input(input.clone());
            }
            if let Some(entry) = &self.source_analysis_entry {
                lowerer.set_source_analysis_options(entry.options());
            } else if let Some(input) =
                retained.filter(|input| input.has_logical_source_name_context())
            {
                // The standalone source helper requires positive retained
                // Logical input; no reconstructed profile grants that axis.
                if let Some(options) =
                    crate::command_binding::SourceAnalysisOptions::for_logical_source(input)
                {
                    lowerer.set_source_analysis_options(options);
                }
            }
            if let Some(base) = base {
                lowerer.lower_into_script_with_bindings(
                    body,
                    base,
                    "::",
                    self.head_identities.source_bindings(),
                )
            } else {
                lowerer.lower(body).top_level.clone()
            }
        })) else {
            return UnknownProcInfo {
                chains_original: true,
                case_insensitive: true,
                has_pattern_dispatch: true,
                has_exec: true,
                has_auto_load: true,
                ..Default::default()
            };
        };

        let mut info = UnknownProcInfo::default();
        for stmt in &script.statements {
            walk_unknown_stmt(
                stmt,
                &script,
                &context,
                metadata,
                &first_param,
                &mut info,
                0,
            );
        }

        info
    }
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

pub(super) fn parse_oo_define_inline_in(
    analyser: &mut Analyser,
    grammar: &DefinitionBodyGrammar,
    args: &[String],
    arg_tokens: &[Token],
    class_def: &mut ClassDef,
    dialect: Option<SurfaceQuery<'_>>,
    scope_path: &[usize],
) {
    if args.is_empty() {
        return;
    }
    if args
        .first()
        .and_then(|subcmd| grammar.member(subcmd))
        .is_some_and(|member| member.unavailable_option_for(&args[1..], dialect).is_some())
    {
        return;
    }
    analyser
        .retain_original_inline_member_metadata(grammar, args, arg_tokens, class_def, scope_path);
    if let Some(member) = args.first().and_then(|subcmd| grammar.member(subcmd)) {
        let parameter_indices = analyser.member_formal_parameter_indices(
            grammar,
            member,
            &args[1..],
            arg_tokens.get(1..).unwrap_or(&[]),
            class_def,
        );
        super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
            analyser,
            &args[1..],
            arg_tokens.get(1..).unwrap_or(&[]),
            &parameter_indices,
        );
    }
    // Synthesise a single fake "command" matching what the
    // body walker would have produced.
    apply_oo_subcommand_in(
        grammar,
        args,
        arg_tokens,
        class_def,
        MemberDialect {
            surface: dialect,
            rules: analyser.word_rules(),
        },
    );
    analyser.retain_class_relation_lookups(args, class_def, scope_path);
}

/// Depth cap for [`walk_unknown_stmt`]'s recursion over nested `if`/`for`/
/// `while`/`foreach`/`catch`/`try`/`switch`/`Block`/`UpFrame` bodies —
/// Transitively bounded today via `MAX_LOWER_NEST_DEPTH`
/// (every `Script` this walk sees was built by `crate::lowering`, which
/// already caps its own construction at 256), capped here independently
/// for defence-in-depth and consistency with every other full-tree walker
/// in this crate.
const MAX_UNKNOWN_STMT_WALK_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Inspect a single IR statement for unknown-proc dispatch
/// markers.
///
/// Recurses through control-flow bodies (`if` clauses, `for` /
/// `while` / `foreach` bodies, `try` / `catch` bodies) so a
/// ``switch`` arm or ``exec`` call buried inside a guard or
/// loop is still detected. `depth` is `stmt`'s own nesting level — see
/// [`MAX_UNKNOWN_STMT_WALK_DEPTH`].
fn walk_unknown_stmt(
    stmt: &Statement,
    script: &crate::ir::Script,
    context: &tcl_registry::model::ContextRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    first_param: &str,
    info: &mut UnknownProcInfo,
    depth: u32,
) {
    if MAX_UNKNOWN_STMT_WALK_DEPTH.exceeded(depth) {
        return;
    }
    update_unknown_statement_facts(stmt, script, context, metadata, first_param, info);
    walk_unknown_stmt_children(stmt, context, metadata, first_param, info, depth);
}

fn update_unknown_statement_facts(
    stmt: &Statement,
    script: &crate::ir::Script,
    context: &tcl_registry::model::ContextRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    first_param: &str,
    info: &mut UnknownProcInfo,
) {
    // Select the purpose only after this original carrier agrees with the
    // complete supplied source owner. Availability-only metadata cannot turn
    // an original Logical carrier into the Native route.
    let source_metadata = metadata.and_then(|metadata| {
        let tokens = script.retained_source_tokens_for_statement(stmt)?;
        tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_input(
                tokens,
                metadata.source_analysis_input()?,
                context.commands(),
            )
    });
    let case = source_metadata.and_then(|metadata| {
        if script
            .retained_source_tokens_for_statement(stmt)?
            .source_binding
            .as_ref()?
            .original_invocation_source_purpose(
                script.retained_source_tokens_for_statement(stmt)?,
                metadata,
                context.commands(),
            )?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical
        {
            crate::registry_invocation::source_structure::original_logical_source_case_for_tokens(
                script.retained_source_tokens_for_statement(stmt)?,
                context.commands(),
                metadata,
            )
        } else {
            crate::registry_invocation::source_structure::original_native_source_case_for_statement(
                script,
                stmt,
                context.commands(),
                metadata,
            )
        }
    });
    if let Some(case) = case
        && case.references_parameter(first_param)
    {
        info.case_insensitive |= case.folds_case();
        if case.mode() == tcl_registry::spec::CaseMatchMode::Exact {
            info.dispatch_targets
                .extend(case.patterns().iter().cloned());
        } else {
            info.has_pattern_dispatch = true;
        }
        info.case_body_residuals.push(case.body_operands().clone());
    }
    if matches!(
        stmt,
        Statement::Call { .. } | Statement::Barrier { .. } | Statement::NativeCall { .. }
    ) {
        record_unknown_invocation_facts(stmt, script, context, source_metadata, info);
    }
}

fn record_unknown_invocation_facts(
    stmt: &Statement,
    script: &crate::ir::Script,
    context: &tcl_registry::model::ContextRegistry,
    source_metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    info: &mut UnknownProcInfo,
) {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    // Logical advice selects the exact operation; Native still needs its
    // independent proved target.
    let invocation = source_metadata.and_then(|metadata| {
        if script
            .retained_source_tokens_for_statement(stmt)?
            .source_binding
            .as_ref()?
            .original_invocation_source_purpose(
                script.retained_source_tokens_for_statement(stmt)?,
                metadata,
                context.commands(),
            )?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical
        {
            crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                context.commands(),
                metadata,
                stmt.tokens()?,
            )
        } else {
            crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
                context.commands(),
                Some(metadata),
                stmt,
            )
        }
    });
    if let Some(invocation) = invocation {
        info.chains_original |= chains_original_unknown(&invocation.facts);
        info.has_exec |= spawns_process(&invocation.facts);
        info.has_auto_load |= loads_external_unit(&invocation.facts);
    }
}

fn walk_unknown_stmt_children(
    stmt: &Statement,
    context: &tcl_registry::model::ContextRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    first_param: &str,
    info: &mut UnknownProcInfo,
    depth: u32,
) {
    match stmt {
        Statement::Switch { arms, .. } => {
            // Recurse into arm bodies (a switch arm may contain
            // an exec or auto_load that should still register).
            for arm in arms {
                if let Some(body) = &arm.body {
                    walk_unknown_script(body, context, metadata, first_param, info, depth + 1);
                }
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for clause in clauses {
                walk_unknown_script(
                    &clause.body,
                    context,
                    metadata,
                    first_param,
                    info,
                    depth + 1,
                );
            }
            if let Some(body) = else_body {
                walk_unknown_script(body, context, metadata, first_param, info, depth + 1);
            }
        }
        Statement::For { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => {
            walk_unknown_script(body, context, metadata, first_param, info, depth + 1);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_unknown_script(body, context, metadata, first_param, info, depth + 1);
            for handler in handlers {
                walk_unknown_script(
                    &handler.body,
                    context,
                    metadata,
                    first_param,
                    info,
                    depth + 1,
                );
            }
            if let Some(body) = finally_body {
                walk_unknown_script(body, context, metadata, first_param, info, depth + 1);
            }
        }
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Call { .. }
        | Statement::Barrier { .. }
        | Statement::NativeCall { .. }
        | Statement::Return { .. } => {}
    }
}

fn walk_unknown_script(
    script: &crate::ir::Script,
    context: &tcl_registry::model::ContextRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    first_param: &str,
    info: &mut UnknownProcInfo,
    depth: u32,
) {
    for stmt in &script.statements {
        walk_unknown_stmt(stmt, script, context, metadata, first_param, info, depth);
    }
}

/// Collect the two class-level (non-method) body kinds a definition body
/// can carry: a `property`'s `-get`/`-set` accessor scripts, and an
/// `initialise`/`initialize` block.
///
/// Neither is a method frame, so [`collect_method_body`] — restricted to the
/// four real method-bearing members — does not see them, yet each needs its
/// own scope in the per-body walk. Split out of `parse_oo_definition_body`
/// purely to
/// keep that walker within its line budget.
fn collect_class_level_bodies(
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    dialect: Option<SurfaceQuery<'_>>,
    accessor_bodies: &mut Vec<CollectedMethodBody>,
    init_bodies: &mut Vec<CollectedMethodBody>,
) {
    // Written directly in the class body — a wrapped spelling is no
    // class-level script of the class's own.
    let Some(statement) = MemberStatement::read(grammar, texts, argv, dialect)
        .filter(|statement| statement.row.keyword_index == 0)
    else {
        return;
    };
    match statement.landing() {
        MemberLanding::Accessors => collect_property_accessor_bodies(texts, argv, accessor_bodies),
        // A wrapper's block form is an init script too, but its members are
        // read one by one ([`expand_wrapper_block_members`]); only a member
        // of its own runs as the class's init script.
        MemberLanding::InitScript if statement.member.kind == MemberKind::Flat => {
            if let Some(body_idx) = statement.row.body.map(|body| body.0)
                && let (Some(body), Some(tok)) = (texts.get(body_idx), argv.get(body_idx).copied())
                && tok.kind == TokenType::Str
            {
                init_bodies.push(CollectedMethodBody {
                    name: format!("<{}>", statement.keyword()),
                    params: Vec::new(),
                    body_text: body.clone(),
                    body_tok: tok,
                    params_tok: None,
                    // A class-level init script is not a method frame at
                    // all — `self` raises there.
                    class_side: true,
                });
            }
        }
        _ => {}
    }
}

/// One definition-body member command's words with every **statically
/// determined** `{*}` expansion already spliced, or `None` when the command
/// has no expansion the analyser can resolve (the overwhelmingly common
/// case, kept allocation-free).
///
/// Tcl expands `{*}` during parsing, so a `{*}`-marked word whose text is a
/// literal is *not* one word — it is the elements of the list it holds, and
/// the member grammar's fixed argument layout applies to those.  Without
/// this normalisation `constructor {*}{args {…}}` / `method
/// {*}{foo {} {…}}` look one word short of their grammar and the member is
/// dropped entirely, with no diagnostic.  Verified
/// against tclsh 9.0.4 and 8.6.16: both forms define a real, callable
/// member, in the `oo::class create` body *and* the `oo::define` body.
///
/// Only a single **braced** (`Str`) word splices: braces suppress
/// substitution, so its element list is exactly what the runtime sees.  A
/// `{*}` over a variable or command substitution (`{*}[info class
/// definition …]`, the ticklecharts `chart3D` reflection idiom) has no
/// statically-knowable element list and is left verbatim — the member then
/// abstains exactly as before rather than being recorded with invented
/// parameters or a body span pointing at the wrong text.
///
/// Element spans come from the segmenter
/// ([`crate::segmenter::flatten_clause_list_elements_with_config`]), so each spliced
/// word carries its own true source range and downstream consumers
/// (document symbols, `my`-dispatch, hover) get real positions rather than
/// the whole-list span.
/// Expand a wrapper member's bare **script-block** form into the equivalent
/// prefix forms, so one walker handles both spellings of the same declaration.
///
/// `TclOO`'s `self` and `private` are the two members the registry marks
/// [`MemberKind::Wrapper`] *with* `wrapper_block_body`: each takes both
/// `self method NAME ARGS BODY` (prefix) and `self { method NAME ARGS BODY; … }`
/// (block, a definition script evaluated against the class object / with
/// private visibility).  Only the prefix form was ever walked, so every member
/// declared in a block was invisible to the whole analysis — no `ClassDef`
/// entry, hence no document-symbol node, no dispatch arity, and no body walk.
///
/// Returns `Some(calls)` — each `(texts, argv)` being the block's inner command
/// with the wrapper keyword (and its token) spliced back on at index 0 — for a
/// block form, and `None` for everything else, including the prefix form, which
/// the caller already handles as written.
///
/// Oracle, identical on tclsh 9.0.4 and 8.6.16 — the block form declares
/// exactly the same class-side methods the prefix form does:
///
/// ```tcl
/// oo::class create ::C {
///     self { method make {n} {return "made-$n"} ; method other {} {return other} }
///     method inst {} {return inst}
/// }
/// ::C make 7                  ;# -> made-7
/// info class methods ::C      ;# -> inst          (instance side untouched)
/// info object methods ::C     ;# -> other make    (class-object side)
/// oo::class create ::F { superclass ::C }
/// ::F make 1                  ;# -> error: unknown method "make"  (not inherited)
/// ```
///
/// and `export`/`unexport` inside the block act on the class-object side:
/// `oo::class create ::E { self { method hidden {} {…} ; unexport hidden } }`
/// leaves `info object methods ::E` empty while `-all -private` still lists
/// `hidden`.
///
/// Abstains — returning `None`, so the command is walked exactly as it is
/// today — on a dynamic (non-`Str`) block word, which cannot be re-segmented
/// statically, and on any shape carrying words after the block.
fn expand_wrapper_block_members(
    source: &str,
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<(Vec<String>, Vec<Token>)>> {
    use tcl_registry::definer::MemberKind;
    let keyword = texts.first()?;
    let keyword_tok = *argv.first()?;
    let member = grammar.member(keyword)?;
    if member.kind != MemberKind::Wrapper || !member.wrapper_block_body {
        return None;
    }
    // The prefix form — the word after the wrapper is itself a member keyword —
    // is already the spelling the walker consumes; leave it alone.
    let inner_first = texts.get(1)?;
    if grammar.member(inner_first).is_some() {
        return None;
    }
    // This is the wrapper's own fixed one-body layout, not an inner member
    // layout. The recursively spliced member goes back through the
    // profile-aware caller, so a Tcl 9 option cannot be accepted here under
    // Tcl 8.6.
    // `+ 1` because a member's registry arg-role indices are relative to its
    // *arguments*, while `texts`/`argv` still carry the keyword at 0.
    let body_idx = member.indices_for(ArgRole::Body).next()? + 1;
    if texts.len() != body_idx + 1 {
        return None;
    }
    let body_text = texts.get(body_idx)?;
    let body_tok = *argv.get(body_idx)?;
    if body_tok.kind != TokenType::Str {
        return None;
    }
    let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
    let cmds = crate::segmenter::segment_commands_with_offset_and_config(
        body_text,
        base_offset,
        config.nested(),
    );
    let mut out: Vec<(Vec<String>, Vec<Token>)> = Vec::new();
    for cmd in &cmds {
        if cmd.is_partial || cmd.argv.is_empty() {
            continue;
        }
        let spliced = splice_static_member_expansions(source, cmd, config);
        let (inner_texts, inner_argv) = spliced
            .as_ref()
            .map_or((cmd.texts.as_slice(), cmd.argv.as_slice()), |(t, a)| {
                (t.as_slice(), a.as_slice())
            });
        let mut wrapped_texts = Vec::with_capacity(inner_texts.len() + 1);
        wrapped_texts.push(keyword.clone());
        wrapped_texts.extend(inner_texts.iter().cloned());
        let mut wrapped_argv = Vec::with_capacity(inner_argv.len() + 1);
        wrapped_argv.push(keyword_tok);
        wrapped_argv.extend(inner_argv.iter().copied());
        out.push((wrapped_texts, wrapped_argv));
    }
    Some(out)
}

fn splice_static_member_expansions(
    source: &str,
    cmd: &crate::segmenter::SegmentedCommand,
    config: tcl_lexer::LexerConfig,
) -> Option<(Vec<String>, Vec<Token>)> {
    let expand = cmd.expand_word.as_ref()?;
    if !expand.iter().any(|&e| e) {
        return None;
    }
    let mut texts: Vec<String> = Vec::with_capacity(cmd.texts.len());
    let mut argv: Vec<Token> = Vec::with_capacity(cmd.argv.len());
    let mut spliced = false;
    for (i, (text, tok)) in cmd.texts.iter().zip(cmd.argv.iter()).enumerate() {
        let is_static_expansion = expand.get(i).copied().unwrap_or(false)
            && tok.kind == TokenType::Str
            && cmd.single_token_word.get(i).copied().unwrap_or(false);
        if is_static_expansion {
            spliced = true;
            for (element, element_tok) in crate::segmenter::flatten_clause_list_elements_with_config(
                source, text, *tok, config,
            ) {
                texts.push(element);
                argv.push(element_tok);
            }
            continue;
        }
        texts.push(text.clone());
        argv.push(*tok);
    }
    spliced.then_some((texts, argv))
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

/// Recognise a method-defining statement in a class body and return its body
/// to walk in a fresh [`ScopeKind::Method`] scope.
///
/// Whether the statement opens a method frame is its registry row's answer (a
/// callable landing in a method table), and the member's argument layout —
/// which word is the name, the parameter list, and the recursable body — comes
/// entirely from its registry [`MemberSpec`] arg-roles (never hardcoded
/// indices), so a definer that adds or reshapes a method-bearing member is
/// picked up from the grammar alone.
///
/// A class-level init script (`initialise` / `initialize`, walked in a scope
/// of its own by the caller) and a wrapper's block form are not method frames,
/// even though the grammar marks each as carrying a body. A forward has no
/// body; dynamic (non-braced) bodies are filtered downstream by
/// [`Analyser::walk_method_body`].
fn collect_method_body(
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    dialect: Option<SurfaceQuery<'_>>,
    rules: WordValueRules,
) -> Option<CollectedMethodBody> {
    // A wrapped member (`self method …`, `private method …`) is read through
    // its wrapper: its body would otherwise never be walked at all (no internal
    // diagnostics inside a `self method`/`private method` body — confirmed
    // empirically, a deliberately-wrong-arity call inside one drew nothing,
    // while the identical call in a plain `method` body correctly fired). The
    // row's keyword is the wrapped member's own, so the slices below start
    // right after the *effective* keyword either way.
    let statement = MemberStatement::read(grammar, texts, argv, dialect)?;
    if !matches!(statement.landing(), MemberLanding::Method(_)) {
        return None;
    }
    // A member on the class object (`classmethod`, `self method`) runs in a
    // class-side frame, where `[self class]` never answers the written class
    // (tclsh 9.0.4: it raises "method not defined by a class" in a
    // `self method`, and answers the internal `::oo::ObjN:: oo ::delegate`
    // class in a `classmethod`).
    let class_side = statement.row.receiver != MemberReceiver::Instance;
    let (member, keyword, texts, argv) = (
        statement.member,
        statement.keyword(),
        statement.args(),
        statement.arg_tokens(),
    );
    let body_idx = member
        .indices_for_call_in(texts, dialect, ArgRole::Body)
        .next()?;
    let params_idx = member
        .indices_for_call_in(texts, dialect, ArgRole::ParamList)
        .next();
    let name_idx = member
        .indices_for_call_in(texts, dialect, ArgRole::Name)
        .next();

    let body_text = texts.get(body_idx)?.clone();
    let body_tok = *argv.get(body_idx)?;
    // A named member (`method NAME …`) uses its name word; the nameless forms
    // (`constructor` / `destructor`) get a synthetic `<keyword>` name.
    let name = name_idx
        .and_then(|i| texts.get(i))
        .map_or_else(|| format!("<{keyword}>"), String::clone);
    let params = params_idx
        .and_then(|i| texts.get(i))
        .map_or_else(Vec::new, |p| parse_param_list(p, rules));
    let params_tok = params_idx.and_then(|i| argv.get(i).copied());
    Some(CollectedMethodBody {
        name,
        params,
        body_text,
        body_tok,
        params_tok,
        class_side,
    })
}

/// Apply a *retracting* member word (`deletemethod m`, `self renamemethod old
/// new`, `private deletemethod m`) to `side`'s method table: remove every name
/// it retracts, and — for a renaming word — re-record the removed member under
/// the name it **arrives** at.
///
/// Which member words retract, and *which of their arguments* they retract or
/// re-key, is registry data ([`MemberSpec::retraction`] /
/// [`tcl_registry::definer::MemberRetraction::split`]), never a keyword matched
/// here — the sibling
/// [`MemberRefKind::Method`] members (`export` / `unexport` / `filter`) name a
/// method without removing it and must not retract.
///
/// `arg_tokens` is the *same slice* as `names`, word for word: the arrival
/// name's own token is the synthetic declaration site the moved member gets
/// (see below), so it has to be reachable from the index the registry hands
/// back.
///
/// # The rename really moves the member
///
/// `renamemethod old new` is not a deletion — it is a move, and the destination
/// is a fully dispatchable member carrying the *source's* body, parameters and
/// visibility. Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
///
/// ```tcl
/// oo::class create ::R1 { method old {} { return OLDBODY } ; renamemethod old new }
/// info class methods ::R1              ;# -> new
/// [::R1 new] new                       ;# -> OLDBODY            (the old body runs)
/// info class definition ::R1 new       ;# -> {} { return OLDBODY }
/// oo::class create ::R4 { method Priv {} {…} ; renamemethod Priv pub }
/// info class methods ::R4              ;# -> (empty)   `pub` is still UNexported…
/// info class methods ::R4 -private     ;# -> pub       …because `Priv` was
/// oo::class create ::R5 { method low {} {…} ; renamemethod low Up }
/// info class methods ::R5              ;# -> Up        …and `Up` is still exported
/// ```
///
/// so the moved [`MethodDef`] keeps its `visibility` verbatim — the family's
/// name-based default (`[a-z]*` is exported for `TclOO`) is applied when a
/// member is *(re)declared*, never when one is renamed. Its `body_span` also
/// stays pointing at the original body, which is the only real body there is;
/// only `name` and `name_span` move, the latter onto the `renamemethod` call's
/// own destination word — the natural go-to-definition target for a member
/// whose sole textual mention of the new name is right there.
///
/// # Definition-aborting shapes
///
/// Real Tcl aborts the *whole* definition — no class is created at all — when a
/// retracting word names a member that does not exist on its own side, or
/// renames onto a name that side already holds. Each is recorded as a
/// [`DefinitionAbort`] for the class handlers to turn into a `W315`; the
/// partial class is still built, so navigation degrades rather than vanishing.
/// This is the one site that can detect them, because it is the only place that
/// knows the side's table state *at the point the word runs* — which is exactly
/// the granularity real Tcl uses (`method a ; method b ; deletemethod b ;
/// renamemethod a b` is legal, and leaves `b`).
///
/// # Tombstones for what this record cannot see
///
/// A name this record does **not** declare leaves a *tombstone* in
/// [`ClassDef::retracted_members`] instead: that is the cross-document shape —
/// `oo::class create ::C { method m … }` in one file, `oo::define ::C {
/// deletemethod m }` in another — where the second file's `via_define` stub has
/// nothing local to remove, and dropping the retraction on the floor leaves the
/// workspace advertising a method that sourcing the extension deletes.
/// A retraction of a member the same document declares needs no
/// tombstone: it removed the entry outright, the order within one body is known,
/// and exporting it would wrongly cancel a *different* document's redeclaration
/// of the name, which no ordering evidence supports.
///
/// Tombstone and abort are the same fact read under different knowledge, so
/// they are mutually exclusive by construction: whether this record is a
/// cross-file stub is not knowable here, so *both* are recorded and the class
/// handler — which does know ([`ClassDef::via_define`]) — keeps exactly one.
fn retract_named_members(
    member: &MemberSpec,
    names: &[String],
    arg_tokens: &[Token],
    class_def: &mut ClassDef,
    side: MemberSide,
) {
    let Some(retraction) = member.retraction else {
        return;
    };
    let words = retraction.split(names);
    // The name the retracted member re-appears under, with the word whose span
    // becomes its synthetic declaration site. A dynamic destination
    // (`renamemethod old $new`) names nothing statically, so the move abstains
    // and the source is simply retracted — a false negative, the direction this
    // campaign abstains toward.
    let arrival = words
        .arrives_at
        .and_then(|i| Some((names.get(i)?, *arg_tokens.get(i)?)))
        .filter(|(name, tok)| !has_substitution(name, tok));
    for (i, name) in words.retracted.iter().enumerate() {
        let tok = arg_tokens.get(i).copied();
        // A dynamic source name (`deletemethod $m`) resolves to nothing
        // statically: it may or may not name a live member, so neither the
        // removal, the tombstone, nor the abort has evidence behind it.
        if tok.is_some_and(|t| has_substitution(name, &t)) {
            continue;
        }
        let Some(removed) = side.table(class_def).remove(name) else {
            // Nothing local to remove: the cross-file stub shape.  The
            // tombstone carries the *arrival* too when the retracting word was
            // a move — the stub has no `MethodDef` to re-key, so
            // recording where the member goes is the only way the workspace
            // join can put it there.  Its own arrival word is the synthetic
            // declaration site, exactly as it is for a same-file move.
            class_def
                .retracted_members
                .push(crate::analyser::types::MemberRetractionRecord {
                    member: name.clone(),
                    side,
                    arrival: arrival.map(|(new_name, _)| new_name.clone()),
                    arrival_span: arrival.map(|(_, tok)| tok.span),
                });
            if let Some(tok) = tok {
                class_def.definition_aborts.push(DefinitionAbort {
                    kind: DefinitionAbortKind::MissingMember,
                    member: name.clone(),
                    span: tok.span,
                });
            }
            // Nothing was removed, so there is nothing to move and no
            // destination question to ask: real Tcl fails on the source word
            // first (`renamemethod ghost b` reports `method ghost does not
            // exist`, never `method called b already exists`), and one word
            // earns one report.
            continue;
        };
        let Some((new_name, new_tok)) = arrival else {
            continue;
        };
        // The member state this move runs against, captured with the source
        // already removed and the destination not yet inserted — exactly the
        // table the interpreter reads at this point in the body.
        let moved = RenamedMember {
            source: name.clone(),
            destination: new_name.clone(),
            side,
            destination_span: new_tok.span,
            blocked: side.table(class_def).keys().cloned().collect(),
        };
        // One fold decides both readings of that state: whether the destination
        // actually written aborts the definition (`W315`, below) and whether
        // some *other* name would (the rename gate, which asks the recorded
        // `RenamedMember` the same question). Deciding them in one place is what
        // stops the diagnostic and the gate from drifting apart.
        //
        // A rename real Tcl rejects moves nothing — and rather than let the
        // wreckage take a declaration with it, the source stays under its own
        // name and an existing destination keeps its own body. Both members the
        // author wrote are then still navigable, which is the whole point of
        // recording a class that cannot run.
        if let Some(kind) = moved.abort_if_renamed_to(new_name) {
            class_def.definition_aborts.push(DefinitionAbort {
                kind,
                member: new_name.clone(),
                span: new_tok.span,
            });
            side.table(class_def).insert(name.clone(), removed);
            continue;
        }
        let mut md = removed;
        // Only the identity moves: the body span still points at the one real
        // body, and the visibility is the source's (oracle above), not the
        // destination name's family default.
        md.name.clone_from(new_name);
        md.name_span = new_tok.span;
        side.table(class_def).insert(new_name.clone(), md);
        class_def.renamed_members.push(moved);
    }
}

/// Apply a member word's **visibility effect** — `export` / `unexport` — to the
/// members it names, scoped to `side`.
///
/// A no-op for a member carrying no effect: which words set visibility is
/// registry data ([`MemberSpec::visibility_effect`]), never a keyword matched
/// here, exactly as [`MemberSpec::retraction`] decides which words
/// retract. The sibling [`MemberRefKind::Method`] member `filter` names a method
/// and does neither, so it falls through untouched, and a definition grammar for
/// another class system gets the same handling by declaring the effect on its
/// own member word.
///
/// The names also update the class-level visibility sets **of that side** —
/// `exports`/`unexports` for the instance side, `class_exports`/
/// `class_unexports` for the class-object side ([`MemberSide::visibility_sets`]).
/// The pair is what carries the flip across documents: a `via_define` stub
/// declares no member to flip, so without a recorded set a `self unexport m` in
/// one file never reaches another file's class-command dispatch.
/// The two pairs stay strictly apart — `exports`/`unexports` are the
/// *instance*-side record by contract (`workspace_index`'s effective-export
/// union over `instance_method`, `rename_safety`'s "the class declares this
/// name" test), so a class-object-side flip landing there would silently
/// re-state an unrelated instance method's export bit.
///
/// Within a pair the two sets are maintained **mutually exclusive**, because
/// each is a record of the *last* writer for the name and the union consumer
/// reads any `exports` entry as decisive: `method m {} {…} ; export m ;
/// unexport m` must leave `m` in `unexports` alone, or cross-file dispatch would
/// keep advertising a method that `[C new] m` rejects with `unknown method "m"`
/// (tclsh 9.0.4 / 8.6.14).
fn apply_visibility_member(
    member: &MemberSpec,
    names: &[String],
    class_def: &mut ClassDef,
    side: MemberSide,
) {
    let Some(effect) = member.visibility_effect else {
        return;
    };
    let visibility = match effect {
        MemberVisibility::Exported => "public",
        MemberVisibility::Unexported => "unexported",
    };
    let (exports, unexports) = side.visibility_sets(class_def);
    let (set, cleared) = match effect {
        MemberVisibility::Exported => (exports, unexports),
        MemberVisibility::Unexported => (unexports, exports),
    };
    for name in names {
        set.insert(name.clone());
        cleared.remove(name);
    }
    set_member_visibility(class_def, names, visibility, side);
}

/// Fold a relation row into the graph its slot feeds, on the side the row
/// resolved to.
///
/// `superclasses` / `mixins` feed the class-hierarchy graph (inherited
/// methods, MRO).  The navigable *references* to those base classes are
/// recorded separately as `command_invocations` by
/// `record_member_command_references`, so no per-name span is kept here.
/// A class object's own ancestry is not the class's (`self mixin M` mixes
/// into the class object, which no instance inherits), so a class-side
/// superclass or mixin row records nothing.
///
/// The two filter slots are separate slots that intercept different
/// dispatches (see [`ClassDef::filters`] and [`ClassDef::class_filters`] for
/// the oracle), so a `filter` call lands in the slot of the side its wrapper
/// put it on, never in one flat list: `self filter f` fills the class
/// object's own filter slot, which intercepts dispatches on the *class
/// command* (`::B cls`, and even `::B new`) while leaving instances unfiltered
/// — `info object filters ::B` -> `f`, `info class filters ::B` -> empty, on
/// tclsh 9.0.4 and 8.6.14 alike — and `private filter s` fills the
/// *instance* slot (`info class filters`, tclsh 9.0.4; `private` does not
/// exist on 8.6).
///
/// What the word does to the slot is registry data ([`MemberSpec::slot`]):
/// `filter a; filter b` leaves both live (`-append` default — tclsh 9.0.4 /
/// 8.6.16: `info class filters` → `a b`), `superclass` / `mixin` replace
/// (`-set` default — tclsh 9.0.4: `superclass A` then `superclass -append B`
/// → `::A ::B`), and the explicit operations fold through
/// [`tcl_registry::definer::SlotSpec::apply`] — the same fold every slot
/// consumer uses, so they cannot diverge.
fn apply_relation_member(
    member: &MemberSpec,
    slot: RelationSlot,
    args: &[String],
    class_def: &mut ClassDef,
    side: MemberSide,
) {
    let list = match (slot, side) {
        (RelationSlot::Superclass, MemberSide::Instance) => &mut class_def.superclasses,
        (RelationSlot::Mixin, MemberSide::Instance) => &mut class_def.mixins,
        (RelationSlot::Filter, side) => side.filter_list(class_def),
        (RelationSlot::Superclass | RelationSlot::Mixin, MemberSide::ClassObject) => return,
    };
    apply_slot_member(Some(member), args, list);
}

/// Fold one slot-member call (`filter` / `superclass` / `mixin` /
/// `variable`) into `list` through the member's registry [`SlotSpec`]
/// — the one place the analyser applies slot semantics, so
/// the instance / class-object filter slots, the superclass list, the mixin
/// list, and the declared-variable slot all take the identical fold.
///
/// A member with no slot spec — a relation that is a plain list of class
/// references, like itcl's `inherit`, which takes no operation words — adds
/// the names it lists.
fn apply_slot_member(member: Option<&MemberSpec>, args: &[String], list: &mut Vec<String>) {
    match member.and_then(|m| m.slot) {
        Some(slot) => slot.apply(list, args),
        None => list.extend(args.iter().cloned()),
    }
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

/// Set the recorded visibility of each named member on `side` only to
/// `visibility` — the `export` / `unexport` member effect.
///
/// A name with no recorded member **on that side** is skipped: a later `method`
/// definition re-applies the name default anyway (tclsh 9.0.4-pinned), an
/// export-only stub has no declaration to navigate to, and — pinned on 9.0.4 and
/// 8.6.14 alike — naming a method that exists only on the *other* side really is
/// a no-op in Tcl, so skipping it is the faithful answer, not an abstention.
fn set_member_visibility(
    class_def: &mut ClassDef,
    names: &[String],
    visibility: &str,
    side: MemberSide,
) {
    let table = side.table(class_def);
    for name in names {
        if let Some(md) = table.get_mut(name) {
            md.visibility = visibility.to_string();
        }
    }
}

/// `oo::define Cls forward name target ...` — records a forward
/// alias as a method, keeping the target command and any prepended
/// arguments (`forward`'s own version of `interp alias` partial
/// application) so a call through the forward can be arity-checked
/// against `target`'s own signature, shifted by the prepended count —
/// see `Analyser::resolve_indirect_call_target`.  Which words are the name and
/// the target is the row's [`MemberEffect::Forward`] slots.
fn apply_oo_forward(
    statement: &MemberStatement<'_>,
    class_def: &mut ClassDef,
    dialect: Option<SurfaceQuery<'_>>,
) {
    let MemberEffect::Forward {
        name_slot,
        prefix_slot,
    } = statement.row.effect
    else {
        return;
    };
    let (args, tokens) = (statement.args(), statement.arg_tokens());
    let (name_slot, prefix_slot) = (usize::from(name_slot), usize::from(prefix_slot));
    if let Some(name) = args.get(name_slot) {
        let span = tokens
            .get(name_slot)
            .map_or(tcl_lexer::Span::new(0, 0), |token| token.span);
        let forward_target = args.get(prefix_slot).map(|target| {
            (
                target.clone(),
                args.get(prefix_slot + 1..).unwrap_or(&[]).to_vec(),
            )
        });
        let md = MethodDef {
            name: name.clone(),
            params: Vec::new(),
            params_computed: false,
            formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                tcl_dialect::ParameterGrammar::Tcl,
            ),
            name_span: span,
            body_span: span,
            kind: "forward".to_string(),
            is_self_method: false,
            // A forward is dispatched by the same name rule as a method
            // (C computes `isPublic` identically for both).
            visibility: statement.recorded_visibility(name, dialect),
            doc: String::new(),
            forward_target,
        };
        class_def.methods.insert(md.name.clone(), md);
    }
}

#[cfg(test)]
pub(super) fn apply_oo_subcommand(
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    class_def: &mut ClassDef,
) {
    apply_oo_subcommand_in(
        grammar,
        texts,
        argv,
        class_def,
        MemberDialect {
            surface: None,
            rules: WordValueRules::TCL,
        },
    );
}

/// Apply one registry member using the selected dialect's concrete layout.
///
/// The statement's [`MemberRow`] decides everything: its effect says what the
/// member declares ([`member_landing`], the routing the snit and itcl walkers
/// share) and its resolved receiver says which side it lands on. A wrapper's
/// shift is already in that receiver, so `self` and `private` have no arm of
/// their own: `self method` lands among the class methods because its row's
/// receiver is the type object, `private method` among the instance methods
/// with the visibility its shift declares, and the block forms reach here
/// member by member, prefixed with their wrapper by
/// [`expand_wrapper_block_members`]. A statement the registry cannot read
/// records nothing.
///
/// A member word that *removes* the members it names — `deletemethod m`,
/// `renamemethod old new` — and one that flips their visibility — `export`
/// / `unexport` — act on their row's side and nowhere else: written with no
/// wrapper, the **instance** side; under `self`, the class-object side;
/// under `private`, `private`'s own (instance) side, including a member the
/// same block just recorded. Oracle, byte-identical on tclsh 9.0.4 and
/// 8.6.14:
///
/// ```tcl
/// oo::class create ::I1 { method gone {} {…}; method kept {} {…}
///                         deletemethod gone }
/// info class methods ::I1   ;# -> kept
/// oo::class create ::I3 { method old {} {…}; renamemethod old new }
/// info class methods ::I3   ;# -> new          (`old` really is gone)
/// oo::class create ::I4 { method gone {} {…} }
/// oo::define ::I4 { deletemethod gone }
/// info class methods ::I4   ;# -> (empty)
/// oo::class create C { method m {} {…}
///                      self { method m {} {…}; unexport m } }
/// info object methods ::C   ;# -> (empty)     class-side `m` unexported
/// info class methods ::C    ;# -> m           instance side untouched
/// ```
///
/// and `oo::class create E2 { self { method onlyclass {} {…} } }` then
/// `oo::define E2 { unexport onlyclass }` leaves the class-object side's
/// `onlyclass` exported and dispatchable, while `oo::define E { self
/// unexport onlyinst }` naming a method only the *other* side has is a
/// silent no-op, not the hard error `deletemethod` raises.
pub(super) fn apply_oo_subcommand_in(
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    class_def: &mut ClassDef,
    dialect: MemberDialect<'_>,
) {
    let Some(statement) = MemberStatement::read(grammar, texts, argv, dialect.surface) else {
        return;
    };
    let Some(side) = member_side(statement.row.receiver) else {
        return;
    };
    let (member, words, tokens) = (statement.member, statement.args(), statement.arg_tokens());
    match statement.landing() {
        MemberLanding::Method(kind) => record_declared_method(&statement, kind, class_def, dialect),
        // `class_methods` holds the `classmethod`-kind entries the
        // class-command dispatch reads, so a class-object forward
        // (`self forward`) stays unrecorded.
        MemberLanding::Forward if side == MemberSide::Instance => {
            apply_oo_forward(&statement, class_def, dialect.surface);
        }
        // Additive, not a reset (tclsh9.0-verified: `oo::define Cls
        // variable a b; oo::define Cls variable c` leaves all of `a`, `b`, `c`
        // live simultaneously), because the class `variable` word is a slot
        // whose default operation is `-append` (with dedup — tclsh 9.0.4:
        // `variable a; variable a b` → `a b`); the explicit `-set` / `-clear`
        // / `-remove` operations fold through the same registry fold as every
        // other slot. A class-object variable (`self variable`) is no instance
        // variable of the class.
        MemberLanding::State(_) if side == MemberSide::Instance => {
            apply_slot_member(Some(member), words, &mut class_def.variables);
        }
        MemberLanding::Relation(slot) => {
            apply_relation_member(member, slot, words, class_def, side);
        }
        MemberLanding::Retraction => retract_named_members(member, words, tokens, class_def, side),
        MemberLanding::Visibility => apply_visibility_member(member, words, class_def, side),
        MemberLanding::Accessors if side == MemberSide::Instance => {
            extract_property_defs(words, tokens, class_def);
        }
        // A class-level init script's body is collected and walked on its
        // own ([`collect_class_level_bodies`]); a procedure, a configuration
        // word and the wrapped spellings of the sided arms above record
        // nothing here.
        _ => {}
    }
}

/// Record a callable member row as the [`MethodDef`] its kind names.
///
/// The name, parameter list and body words come from the member's registry
/// layout ([`extract_method_def_in`]); a nameless member (a constructor or
/// destructor) is recorded under its synthetic `<keyword>` name and anchored on
/// its keyword, where an editor lands for go-to-definition and hover. A method
/// (re)definition takes the visibility its row names — the family's name-based
/// default unless its option word or wrapper says otherwise — discarding any
/// earlier explicit `export`/`unexport` of the name (tclsh 9.0.4-pinned; the
/// rule itself is registry data — `[a-z]*` for `TclOO`).
///
/// A member a receiver-moving wrapper put on the class object (`self method
/// NAME ARGS BODY`, `TclOO`'s own spelling of a class-level method) is tagged
/// `is_self_method`, so the class-command MRO walk in `tcl-lsp-core` knows not
/// to treat it as inherited the way a `classmethod` is (real tclsh: a subclass
/// with no override does not gain a `self method` at all).
fn record_declared_method(
    statement: &MemberStatement<'_>,
    kind: MethodKind,
    class_def: &mut ClassDef,
    dialect: MemberDialect<'_>,
) {
    let synthetic = format!("<{}>", statement.keyword());
    let Some(mut md) = extract_method_def_in(
        statement.member,
        statement.args(),
        statement.arg_tokens(),
        kind.as_str(),
        "public",
        &synthetic,
        dialect,
    ) else {
        return;
    };
    if matches!(
        statement.row.effect,
        MemberEffect::Callable {
            name_slot: None,
            ..
        }
    ) && let Some(keyword) = statement.keyword_token()
    {
        md.name_span = keyword.span;
    }
    md.visibility = statement.recorded_visibility(&md.name, dialect.surface);
    md.is_self_method = kind == MethodKind::ClassMethod && statement.shift().receiver.is_some();
    record_method(class_def, kind, md);
}

/// Extract property definitions from a ``property`` subcommand.
///
/// Walks the args, splitting names from option values
/// (``-get BODY``, ``-set BODY``, ``-kind readable|writable|readwrite``).
/// Each property gets a [`PropertyDef`] entry in
/// ``class_def.properties``.
///
/// All property options take a value (``-get``, ``-set``,
/// ``-kind``); there are no flag-only options.  When ``-kind``
/// is omitted the property defaults to ``"readwrite"``.
///
/// This records only the class-level `PropertyDef` entries; the
/// accessor (`-get` / `-set`) bodies are collected and walked
/// separately by [`collect_property_accessor_bodies`].
fn extract_property_defs(args: &[String], arg_tokens: &[Token], class_def: &mut ClassDef) {
    let zero = Span::new(0, 0);

    // Collect property names + their per-arg index, then the
    // trailing options, in a two-pass shape.
    let mut names: Vec<(String, usize)> = Vec::new();
    let mut kind = "readwrite".to_string();
    let mut getter_defined = false;
    let mut setter_defined = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some(stripped) = arg.strip_prefix('-') {
            // All property options take a value.
            if i + 1 >= args.len() {
                i += 1;
                continue;
            }
            let value = &args[i + 1];
            match stripped {
                "kind" => kind.clone_from(value),
                // registry-axis-ok: definition_body — property accessor keywords of the class body; until the definition-body grammar exposes property accessors
                "get" => getter_defined = true,
                // registry-axis-ok: definition_body — property accessor keywords of the class body; until the definition-body grammar exposes property accessors
                "set" => setter_defined = true,
                _ => {}
            }
            i += 2;
            continue;
        }
        names.push((arg.clone(), i));
        i += 1;
    }

    for (name, idx) in names {
        let span = arg_tokens.get(idx).map_or(zero, |t| t.span);
        class_def.properties.insert(
            name.clone(),
            PropertyDef {
                name,
                name_span: span,
                kind: kind.clone(),
                has_getter: getter_defined,
                has_setter: setter_defined,
            },
        );
    }
}

/// Collect the `-get` / `-set` accessor bodies of a `property` subcommand as
/// walkable method bodies (named `<get>` / `<set>`).  Only braced (`Str`)
/// bodies are walkable.
fn collect_property_accessor_bodies(
    texts: &[String],
    argv: &[Token],
    out: &mut Vec<CollectedMethodBody>,
) {
    let mut i = 0;
    while i < texts.len() {
        if let Some(opt) = texts[i].strip_prefix('-') {
            // Every property option takes a value; only `-get`/`-set` carry a
            // body to analyse.
            if i + 1 < texts.len() {
                // registry-axis-ok: definition_body — property accessor keywords of the class body; until the definition-body grammar exposes property accessors
                if matches!(opt, "get" | "set")
                    && let Some(tok) = argv.get(i + 1).copied()
                    && tok.kind == TokenType::Str
                {
                    out.push(CollectedMethodBody {
                        name: format!("<{opt}>"),
                        params: Vec::new(),
                        body_text: texts[i + 1].clone(),
                        body_tok: tok,
                        params_tok: None,
                        // Conservative: a `property` accessor body is a
                        // 9.0+ `oo::configurable` surface this analysis has
                        // not pinned `[self class]` behaviour for — abstain
                        // from the defining-class fact rather than guess.
                        class_side: true,
                    });
                }
                i += 2;
                continue;
            }
        }
        i += 1;
    }
}

/// Extract a [`MethodDef`] from a method-shaped member's args.
///
/// The name / parameter-list / body word positions come from `member`'s
/// registry arg-roles (`method NAME PARAMS BODY`, `constructor PARAMS BODY`,
/// `destructor BODY`), never hardcoded indices.  A member with no
/// [`ArgRole::Name`] (constructor / destructor) takes `synthetic_name` as its
/// placeholder.  `args` / `arg_tokens` are the words *after* the member
/// keyword.
///
/// Returns `None` when the argument count is too short (no body word) to match
/// the member's shape.
#[cfg(test)]
fn extract_method_def(
    member: &MemberSpec,
    args: &[String],
    arg_tokens: &[Token],
    kind: &str,
    visibility: &str,
    synthetic_name: &str,
) -> Option<MethodDef> {
    extract_method_def_in(
        member,
        args,
        arg_tokens,
        kind,
        visibility,
        synthetic_name,
        MemberDialect {
            surface: None,
            rules: WordValueRules::TCL,
        },
    )
}

pub(super) fn extract_method_def_in(
    member: &MemberSpec,
    args: &[String],
    arg_tokens: &[Token],
    kind: &str,
    visibility: &str,
    synthetic_name: &str,
    dialect: MemberDialect<'_>,
) -> Option<MethodDef> {
    let zero = tcl_lexer::Span::new(0, 0);
    let body_idx = member
        .indices_for_call_in(args, dialect.surface, ArgRole::Body)
        .next()?;
    // A body word must be present for the member to be well-formed.
    args.get(body_idx)?;
    let name_idx = member
        .indices_for_call_in(args, dialect.surface, ArgRole::Name)
        .next();
    let params_idx = member
        .indices_for_call_in(args, dialect.surface, ArgRole::ParamList)
        .next();

    let name = name_idx
        .and_then(|i| args.get(i))
        .map_or_else(|| synthetic_name.to_string(), String::clone);
    let params = params_idx
        .and_then(|i| args.get(i))
        .map_or_else(Vec::new, |p| parse_param_list(p, dialect.rules));
    let name_span = name_idx
        .and_then(|i| arg_tokens.get(i))
        .map_or(zero, |t| t.span);
    let body_span = arg_tokens.get(body_idx).map_or(zero, |t| t.span);
    Some(MethodDef {
        name,
        params,
        params_computed: false,
        formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
            tcl_dialect::ParameterGrammar::Tcl,
        ),
        name_span,
        body_span,
        kind: kind.to_string(),
        // Flipped by the one caller that needs it (`record_declared_method`,
        // for a member a receiver-moving wrapper put on the class object) —
        // every other caller means it literally, so `false` is the
        // correct default here, not just a placeholder.
        is_self_method: false,
        visibility: visibility.to_string(),
        doc: String::new(),
        forward_target: None,
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod lifecycle_formal_tests;
