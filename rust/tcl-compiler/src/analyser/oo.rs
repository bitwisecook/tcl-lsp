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
//! ``oo::define Name { ... }`` block and populates the
//! [`super::types::ClassDef`] fields: the full field set
//! (``constructors``, ``destructor``, ``variables``,
//! ``properties``, ``filters``, ``exports``, ``unexports``) plus
//! [`Analyser::extract_unknown_proc_info`] — the W123 gating
//! analysis for user-defined ``unknown`` procs.
//!
//! Subcommand coverage:
//!
//! - ``superclass ?-op? <names>`` — folds into ``ClassDef::superclasses``
//!   through the registry slot spec (default ``-set``; issue #1169).
//! - ``mixin ?-op? <names>`` — folds into ``ClassDef::mixins`` the same
//!   way (default ``-set``).
//! - ``method NAME PARAMS BODY`` — adds to ``ClassDef::methods``.
//! - ``classmethod NAME PARAMS BODY`` — adds to
//!   ``ClassDef::class_methods``.
//! - ``constructor PARAMS BODY`` — appends a synthetic-named
//!   ``MethodDef`` to ``ClassDef::constructors``.
//! - ``destructor BODY`` — sets ``ClassDef::destructor``.
//! - ``forward NAME ?TARGET ARGS?`` — adds to ``methods`` with
//!   ``kind = "forward"``.
//! - ``variable ?-op? <names>`` — folds into ``ClassDef::variables``
//!   (slot default ``-append`` with dedup; issue #1169).
//! - ``filter ?-op? <names>`` — folds into ``ClassDef::filters``
//!   (slot default ``-append``, duplicates kept).
//! - ``export <names>`` / ``unexport <names>`` — extends the
//!   matching ``HashSet`` field.
//! - ``property NAME ?-get BODY? ?-set BODY? ?-kind K?`` —
//!   extracts a [`super::types::PropertyDef`] per name.
//! - ``initialise`` / ``initialize`` — recognised; the body is
//!   walked in the enclosing scope for variable tracking.

use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::{DefinitionBodyGrammar, MemberRefKind, MemberSpec, MemberVisibility};

use super::diagnostics::helpers::has_substitution;
use super::scope::scope_at_mut;
use super::state::Analyser;
use super::types::{
    ClassDef, DefinitionAbort, DefinitionAbortKind, MemberSide, MethodDef, PropertyDef,
    RenamedMember, Scope, ScopeKind, UnknownProcInfo,
};
use super::utils::{param_name_spans_for_token, parse_param_list};
use crate::ir::{Module, Statement, SwitchMode};
use crate::signature_scan::types::ParamDef;

/// Command names that, when called from inside a user-defined
/// ``unknown`` proc, indicate the handler chains to the original
/// Tcl ``unknown`` rather than dispatching itself.
///
/// Names match exactly — when any IR call inside the body
/// resolves to one of these, ``UnknownProcInfo::chains_original``
/// flips to ``true``.
const CHAIN_TARGETS: &[&str] = &[
    "_original_unknown",
    "_orig_unknown",
    "::tcl::unknown",
    "tcl::unknown",
    "original_unknown",
];

/// Implicit variable snit injects into `typemethod` / `typeconstructor` bodies.
///
/// The instance-body implicits (`self` / `selfns` / `type` / `options`) come
/// from the definer's registry grammar (`implicit_vars`); this is only the
/// narrower *type*-body subset, which the flat grammar list doesn't
/// distinguish — a `typemethod` body sees `type` but not `self`.
const SNIT_TYPE_IMPLICIT: &[&str] = &["type"];

/// A definer's grammar plus the class being populated and where its members
/// live — bundled so the snit / itcl member dispatch + extraction helpers stay
/// under the argument limit.  Shared by both families (the extraction is
/// grammar-driven; only the per-family dispatch differs).
struct ClassBodyCtx<'a> {
    grammar: &'static DefinitionBodyGrammar,
    class_def: &'a mut ClassDef,
    class_qualified: &'a str,
    scope_path: &'a [usize],
}

/// A body-bearing member ready to extract: its grammar spec (argument layout),
/// the target [`MethodDef::kind`], and the synthetic name for the nameless
/// forms.  Bundled to keep [`Analyser::extract_class_member`] under the argument
/// limit.
struct MemberForm<'a> {
    member: &'a MemberSpec,
    kind: &'a str,
    label: &'a str,
    /// The member's declared visibility (`"public"` for snit / `TclOO`; the itcl
    /// access modifier `public` / `protected` / `private`).
    visibility: &'a str,
}

/// The snit definer being parsed.  Which implicit variables its member
/// bodies see — including the widget-only `win` / `hull` pair — comes from
/// the grammar's own `implicit_vars` (`snit::widget` / `widgetadaptor` carry
/// `SNIT_WIDGET_GRAMMAR`), so no consumer matches the definer's name suffix.
struct SnitDefiner {
    grammar: &'static DefinitionBodyGrammar,
}

/// Whether `member` is a pure variable/component *declaration* — it names a
/// [`ArgRole::VarWrite`] but carries no recursable body (`variable v`,
/// `typevariable v`, `component c`, `typecomponent c`).  Members that carry
/// both (snit 1.x `onconfigure`'s value var) are method bodies, not
/// declarations, and are excluded.
fn is_var_declaration(member: &MemberSpec) -> bool {
    let has_var = member
        .arg_roles
        .iter()
        .any(|(_, r)| *r == ArgRole::VarWrite);
    let has_body = member.arg_roles.iter().any(|(_, r)| *r == ArgRole::Body);
    has_var && !has_body
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
        let Some((sub, _)) = cmd.texts.split_first() else {
            continue;
        };
        // A command is a variable declaration when the grammar marks it one
        // (snit `typevariable`/`component`, …) OR it is TclOO's `variable` /
        // `typevariable`, which `apply_oo_subcommand` handles with a hardcoded
        // arm rather than through the grammar.  Gating additionally on
        // `known_set` below means a stray non-declaration match cannot leak.
        let is_decl = matches!(sub.as_str(), "variable" | "typevariable")
            || grammar.member(sub).is_some_and(is_var_declaration);
        if !is_decl {
            continue;
        }
        // argv[0] / texts[0] is the member keyword; the remaining words are the
        // declared names (`variable a b c`).
        for (text, tok) in cmd.texts.iter().zip(cmd.argv.iter()).skip(1) {
            let base = crate::naming::normalise_var_name(text);
            if known_set.contains(base) {
                out.entry(base.to_string()).or_insert(tok.span);
            }
        }
    }
    out
}

/// Strip a leading member wrapper (a registry [`MemberKind::Wrapper`] — itcl's
/// `public` / `protected` / `private` access modifiers, `TclOO`'s `self`) from a
/// member call, returning the effective member keyword, its argument texts +
/// tokens (the words *after* the keyword), and the declared visibility.  A
/// non-wrapped member reports `"public"` (itcl's members are callable; the
/// precise default is not modelled).  Returns `None` for an empty command or a
/// bare wrapper with no inner member keyword (a wrapper's bare script-block
/// form, `self { … }`, has no inner member).
fn unwrap_wrapper_member<'a>(
    grammar: &DefinitionBodyGrammar,
    texts: &'a [String],
    argv: &'a [Token],
) -> Option<(&'a str, &'a [String], &'a [Token], &'a str)> {
    use tcl_registry::definer::MemberKind;
    let (first, rest_texts) = texts.split_first()?;
    let rest_toks = argv.get(1..).unwrap_or(&[]);
    if grammar
        .member(first)
        .is_some_and(|m| m.kind == MemberKind::Wrapper)
    {
        // `<modifier> <member> args…` — the inner member follows.
        let (inner, inner_texts) = rest_texts.split_first()?;
        let inner_toks = rest_toks.get(1..).unwrap_or(&[]);
        Some((inner.as_str(), inner_texts, inner_toks, first.as_str()))
    } else {
        Some((first.as_str(), rest_texts, rest_toks, "public"))
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
fn loop_installer_pair(
    spec: &'static tcl_registry::CommandSpec,
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
/// unwrapped of any `self`/`private`) that names itself with exactly a
/// reference to the loop variable `var_name` — `foreach`'s installer idiom.
/// Returns the member's `(kind, body_span)` on a match; `None` for anything
/// else (more than one statement, a fixed name, a name built from more than
/// the bare variable, a non-`method`/`classmethod` member) — left exactly as
/// opaque as before, not a partial guess.
fn loop_installed_member_shape(
    grammar: &DefinitionBodyGrammar,
    body_word: &str,
    body_content_start: u32,
    lexer_config: tcl_lexer::LexerConfig,
    var_name: &str,
) -> Option<(&'static str, Span)> {
    let inner_cmds = crate::segmenter::segment_commands_with_offset_and_config(
        body_word,
        body_content_start,
        lexer_config,
    );
    let mut real_cmds = inner_cmds
        .iter()
        .filter(|c| !c.is_partial && !c.argv.is_empty());
    let inner = real_cmds.next()?;
    if real_cmds.next().is_some() {
        return None; // more than one statement — not the simple installer shape
    }
    let (inner_keyword, inner_texts, inner_argv, _modifier) =
        unwrap_wrapper_member(grammar, &inner.texts, &inner.argv)?;
    let kind = match inner_keyword {
        "method" => "method",
        "classmethod" => "classmethod",
        _ => return None,
    };
    let member = grammar.member(inner_keyword)?;
    let name_idx = member.indices_for(ArgRole::Name).next()?;
    let name_word = inner_texts.get(name_idx)?;
    if crate::static_loops::simple_var_ref(name_word).as_deref() != Some(var_name) {
        return None;
    }
    let body_idx = member.indices_for(ArgRole::Body).next()?;
    let body_span = inner_argv.get(body_idx).map_or(inner.span, |t| t.span);
    Some((kind, body_span))
}

/// The synthetic name for a nameless snit member (one with no
/// [`ArgRole::Name`]): `<keyword>`, with a leading roleless option word
/// (snit 1.x `onconfigure`/`oncget`'s `-option`) appended when present so the
/// two option handlers for the same keyword stay distinct
/// (`<onconfigure -foo>` vs `<onconfigure -bar>`).  Unused for named members.
fn snit_member_label(member: &MemberSpec, keyword: &str, args: &[String]) -> String {
    let role_at_zero = member.arg_roles.iter().any(|(i, _)| *i == 0);
    if !role_at_zero && let Some(opt) = args.first() {
        return format!("<{keyword} {opt}>");
    }
    format!("<{keyword}>")
}

impl Analyser {
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
        use tcl_registry::ProfileQueries;
        self.registry
            .and_then(|r| r.get(cmd_name))
            .is_some_and(|spec| !self.profile.is_available(spec))
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
        match member.dialects {
            None => true,
            Some(allowed) => allowed.intersects(self.profile.availability_mask),
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
        self.result.diagnostics.push(super::types::Diagnostic {
            code: tcl_core_types::DiagCode::W002,
            span: tok.span,
            message: format!(
                "'{subcmd}' is disabled in the active dialect profile ('{}')",
                self.dialect()
            ),
            severity: super::types::Severity::Warning,
            fixes: Vec::new(),
        });
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
    /// ([`super::types::ClassDef::member_set_incomplete`], issue #923 idx 53).
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
            return self.registry.as_ref().is_some_and(|registry| {
                registry.get(keyword).is_none_or(|spec| {
                    let args: Vec<&str> = texts.iter().skip(1).map(String::as_str).collect();
                    let resolved = spec
                        .arg_role_resolver
                        .map(|resolve| resolve(&args))
                        .unwrap_or_default();
                    spec.arg_roles
                        .iter()
                        .chain(resolved.iter())
                        .any(|(_, role)| *role == ArgRole::Body)
                })
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
        member.indices_for(ArgRole::Name).any(|idx| {
            texts
                .get(idx + 1)
                .is_some_and(|w| crate::naming::is_dynamic_word(w))
        })
    }

    /// Record the per-name members a **literal** loop-installer declares,
    /// even though [`Self::member_declaration_is_opaque`] has already (and
    /// correctly) marked the class's member set incomplete because of it
    /// (issue #1277).
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
        let Some(spec) = self.registry.and_then(|r| r.get(keyword)) else {
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
        let Ok(var_names) = tcl_syntax::list::split_list(var_word) else {
            return;
        };
        if var_names.len() != 1 {
            return; // a multi-variable group — no single name a `$var` could mean
        }
        let var_name = &var_names[0];
        if !crate::value_shapes::is_static_var_word(var_name) {
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
            self.lexer_config(),
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
                name_span,
                body_span,
                kind: kind.to_string(),
                is_self_method: false,
                doc: String::new(),
                forward_target: None,
            };
            // A literal, directly-written declaration elsewhere in the class
            // always outranks a name merely *inferred* from the loop's list —
            // this only ever fills a gap, never overrides real data, however
            // the two are ordered in the source.
            let table = if kind == "classmethod" {
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
        let Some((keyword, arg_texts, arg_toks, _vis)) =
            unwrap_wrapper_member(grammar, texts, argv)
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
        for idx in spec.indices_for(ArgRole::CommandName) {
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
        // The methods walked below home under the class's qualified name;
        // capture it before the phase-1 walk mutates `class_def`.
        let class_qualified = class_def.qualified_name.clone();
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let cmds = crate::segmenter::segment_commands_with_offset_and_config(
            body_text,
            base_offset,
            self.lexer_config(),
        );
        // Phase 1: populate the `ClassDef` (methods, instance variables,
        // superclasses, …) and collect each method body to walk afterwards.
        // The walk is deferred so every class-level `variable` declaration is
        // visible as a pre-bound local in *every* method body, regardless of
        // source order.
        let mut method_bodies: Vec<CollectedMethodBody> = Vec::new();
        // `property -get`/`-set` accessor bodies — walked in a method scope
        // seeded with the class variables.
        let mut accessor_bodies: Vec<CollectedMethodBody> = Vec::new();
        // `initialise`/`initialize { body }` — a class-level script, walked
        // in a per-class scope of its own (see the walk below).
        let mut init_bodies: Vec<CollectedMethodBody> = Vec::new();
        for cmd in &cmds {
            if cmd.is_partial || cmd.argv.is_empty() {
                continue;
            }
            // `{*}` of a literal list is spliced by the *parser*, so the
            // member's words are the list's elements — `method
            // {*}{foo {} {return 1}}` really defines `foo` (tclsh 9.0.4 and
            // 8.6.16 both run it).  Normalise those words before the
            // grammar's fixed layout is read off them, or the member is one
            // word long and gets dropped as malformed (issue #923 idx 53).
            // A `{*}` over a *substituted* word (`{*}[info class definition
            // …]`) has no statically-known element list, so it is left
            // exactly as written and the member still abstains.
            let spliced = splice_static_member_expansions(cmd);
            let (texts, argv) = spliced
                .as_ref()
                .map_or((cmd.texts.as_slice(), cmd.argv.as_slice()), |(t, a)| {
                    (t.as_slice(), a.as_slice())
                });
            // A member keyword gated to a newer core — `classmethod`,
            // `private`, `initialise`/`initialize`, `definitionnamespace`,
            // and `property` are all 9.0+ — does not exist in this dialect's
            // definition grammar.  Flag it with the same disabled-in-dialect
            // diagnostic a command draws, then carry on recording it (see
            // the emitter's doc for why reporting must not erase).
            if let (Some(subcmd), Some(tok)) = (texts.first(), argv.first()) {
                self.emit_w002_oo_member_disabled(grammar, subcmd, *tok, definer_disabled);
            }
            // …and when it does abstain, say so on the record: the member
            // tables become a lower bound, not the class's whole surface
            // (see [`ClassDef::member_set_incomplete`], issue #923 idx 53).
            if self.member_declaration_is_opaque(grammar, cmd, texts) {
                class_def.member_set_incomplete = true;
                // The class surface stays a lower bound either way — this
                // only ever adds names on top of that abstention when the
                // opaque command turns out to be a literal loop installer
                // (issue #1277).
                self.record_loop_installed_members(grammar, cmd, class_def);
            }
            // A wrapper member's bare *script-block* form (`self { method m …
            // }`, `private { … }`) declares exactly the members its block
            // spells out — so rewrite each inner command into the equivalent
            // prefix form (`self method m …`) and feed the one member walker
            // below.  Which members take a block is registry data
            // (`MemberKind::Wrapper` + `wrapper_block_body`), never a keyword
            // matched here; `None` means "not a block form", i.e. the command
            // stands for itself.
            let expanded = expand_wrapper_block_members(grammar, texts, argv, self.lexer_config());
            let member_calls: Vec<(&[String], &[Token])> = expanded.as_ref().map_or_else(
                || vec![(texts, argv)],
                |inner| {
                    inner
                        .iter()
                        .map(|(t, a)| (t.as_slice(), a.as_slice()))
                        .collect()
                },
            );
            for (texts, argv) in member_calls {
                apply_oo_subcommand(grammar, texts, argv, class_def);
                // A member argument that *names* a command — the class a
                // `superclass`/`mixin` extends, the command a `forward`
                // delegates to — is a first-class command reference.  Record it
                // so references / go-to-definition / rename / call-hierarchy
                // reach it the same as a direct call.  Which arguments name
                // commands is registry data (the member grammar's
                // `all_args_ref` / `ArgRole::CommandName`), never a member
                // keyword the walker knows by name.
                self.record_member_command_references(grammar, texts, argv, scope_path);
                if let Some(mb) = collect_method_body(grammar, texts, argv) {
                    method_bodies.push(mb);
                }
            }
            collect_class_level_bodies(
                grammar,
                texts,
                argv,
                &mut accessor_bodies,
                &mut init_bodies,
            );
        }
        // A bareword-callable-from-this-class fact, gathered before Phase 2
        // starts (so it's already populated by the time member-lookup
        // consumers — go-to-definition / hover — read `class_def`, and
        // regardless of *which* method body called `link`).
        self.collect_oo_links(&method_bodies, class_def);

        // Phase 2: walk each method / accessor / class-init body in its own
        // `Method` scope with the formal parameters and the class's instance
        // variables pre-bound.
        let class_variables = class_def.variables.clone();
        // Map each declared instance-variable name to its `variable v`
        // declaration name-token span, so the per-method seeding below anchors
        // the object variable's definition at the declaration rather than the
        // whole method body.
        let var_decl_spans = collect_var_decl_spans(grammar, &cmds, &class_variables);
        // `TclOO` method bodies resolve bare commands globally (object-ns
        // semantics — see `Scope::oo_global_resolution`); snit / itcl
        // members resolve in the type / class namespace.
        let oo_global = matches!(grammar.family, tcl_registry::definer::DefinerFamily::TclOo);
        // The class-level `initialise` body runs first and in a scope of its
        // own, so the class-scoped variables it declares are visible to this
        // class's own method / accessor bodies below — and to no other
        // class's (issue #923 idx 36).
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
                &class_qualified,
                scope_path,
                mb,
                oo_global,
            );
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
    /// and one a rename would act on (issue #923 idx 36). tclsh 9.0.4 keeps
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
            format!("{class_qualified}::{}", mb.name)
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
                // stays false (Codex review of PR #1084). The two facts
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
            let base = crate::naming::normalise_var_name(var);
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
            .map(|v| crate::naming::normalise_var_name(v).to_string())
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

    /// Scan every collected method/constructor/destructor body's own
    /// top-level statements for a `link`-headed call (`oo::Helpers::link`,
    /// a genuine core `TclOO` builtin since 8.6 — `TclOOLinkObjCmd`, on
    /// every method body's namespace path via `::oo::Helpers`), recording each
    /// alias into [`ClassDef::linked_members`] (issue #923 idx 113).
    ///
    /// `link NAME` installs a per-object-namespace command `NAME` that
    /// dispatches to `my NAME`; `link {NAME TARGET}` dispatches to `my
    /// TARGET` instead. Each argument word of one `link` call is an
    /// independent alias (`link foo bar` installs both `foo` and `bar`),
    /// and a name/target built from a variable or command substitution is
    /// skipped (mirrors [`crate::alias::detect_interp_alias`]'s
    /// literal-only requirement — no lattice lookup here, this runs
    /// before Phase 2's per-method scope walk even exists).
    ///
    /// Deliberately shallow: only a `link` call written directly at a
    /// body's own top level is recognised, not one nested inside an
    /// `if`/`catch`/… body argument — the same scope boundary
    /// `scan_my_method_region`/`scan_obj_method_region` (`references.rs`)
    /// already accept for this class of "scan a method body for one
    /// specific call shape" problem.
    fn collect_oo_links(&self, method_bodies: &[CollectedMethodBody], class_def: &mut ClassDef) {
        for mb in method_bodies {
            if mb.body_tok.kind != TokenType::Str {
                continue;
            }
            let base = mb.body_tok.span.start() + u32::from(mb.body_tok.content_offset);
            let cmds = crate::segmenter::segment_commands_with_offset_and_config(
                &mb.body_text,
                base,
                self.lexer_config(),
            );
            for cmd in &cmds {
                // `link` is deliberately *not* a method-dispatch keyword
                // (issue #1050): it *creates* per-object bareword commands
                // rather than dispatching one, so the barewords it installs
                // are per-class data, not language keywords. That creation
                // is itself a declared behavioural fact —
                // `Traits::TCLOO_BINDS_METHOD_ALIAS` — so the head is
                // recognised through the registry rather than by its
                // spelling (issue #1026); a dialect without `link` (8.5, or
                // 8.6 with no `ooutil`) therefore records no aliases here.
                if cmd.is_partial
                    || !cmd.texts.first().is_some_and(|head| {
                        self.registry.is_some_and(|r| r.binds_method_alias(head))
                    })
                {
                    continue;
                }
                for word in cmd.texts.iter().skip(1) {
                    if crate::naming::is_dynamic_word(word) {
                        continue;
                    }
                    match crate::tcl_expr_eval::split_tcl_list(word).as_slice() {
                        [name] if !name.is_empty() => {
                            let key = name.trim_start_matches("::").to_string();
                            class_def.linked_members.entry(key.clone()).or_insert(key);
                        }
                        [name, target] if !name.is_empty() => {
                            class_def
                                .linked_members
                                .entry(name.trim_start_matches("::").to_string())
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
        class_qualified: &str,
        scope_path: &[usize],
        mb: &CollectedMethodBody,
        oo_global_resolution: bool,
    ) {
        if mb.body_tok.kind != TokenType::Str {
            return;
        }
        let method_qn = if class_qualified.is_empty() {
            mb.name.clone()
        } else {
            format!("{class_qualified}::{}", mb.name)
        };
        // Instance-side `TclOO` method of a statically-named class:
        // `[self class]` answers the defining class in this frame, so
        // record the fact for the constant command-substitution fold
        // (issue #1132). `oo_global_resolution` is exactly the
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
        // param's definition span at its name in the param-list literal (issue
        // #727) so go-to-definition / references / rename resolve to the
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
            let base = crate::naming::normalise_var_name(var);
            if base.is_empty() || mb.params.iter().any(|p| p.name == base) {
                continue;
            }
            let def_span = var_decl_spans
                .get(base)
                .copied()
                .unwrap_or_else(|| Span::new(body_start, body_start));
            self.define_var(base, mb.body_tok, &method_path, false, Some(def_span));
        }
        // Per-item shell pass: defer the method body for an isolated pass like
        // `handle_proc_command`.  Carry the method's qualified name as
        // `scope_name` (so the duplicate detector keys each method distinctly),
        // the class's *defining* namespace (so command/var resolution in the
        // isolated `Method` scope matches the whole-file walk), the formal
        // params, and the class instance variables (pre-bound in every method).
        if self.defer_proc_bodies {
            // Same rule `handle_proc_command` records for a deferred proc body:
            // the command-resolution namespace, so the isolated per-item rebuild
            // and the whole-file walk agree even when the class definer ran
            // inside a qualified-name proc (issue #923 idx 85).
            let namespace = self.command_resolution_namespace(scope_path);
            let safe_interp_ctx = self.safe_interp_ctx_snapshot();
            self.deferred_bodies.push(super::per_item::DeferredBody {
                body_text: std::sync::Arc::from(mb.body_text.as_str()),
                body_tok: mb.body_tok,
                scope_path: method_path,
                is_method: true,
                oo_global_resolution,
                namespace,
                scope_name: method_qn,
                params: mb.params.clone(),
                // The shell walk above already seeded these with their real
                // declaration spans; the graft keeps the shell's span, so
                // the deferred body pass only needs the names.
                class_variables: class_variables.to_vec(),
                // Attached later by `fill_deferred_bodies` for bodies with a
                // fold candidate (issue #1132).
                command_trust: None,
                ensemble_targets: Vec::new(),
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
        // Which commands are snit definers, their member sub-keywords + argument
        // layout, and the variables snit injects into member bodies are all
        // registry data (a `Snit`-family definition-body grammar) — not a
        // hardcoded name / member / implicit-var list.  A new snit-like definer
        // is picked up automatically once its spec carries the grammar.  The
        // grammar is `&'static`, so it outlives the immutable registry borrow
        // released here before the `&mut self` work below.
        let Some(grammar) = self
            .definition_grammar(cmd_name)
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
        // `proc ::ns::p {}` becomes `::ns::T` (issue #923 idx 85).
        let ns_prefix = self.command_resolution_namespace(scope_path);
        // Constructed key in, construction-inverse tail out (#934): a colon
        // trim or `rsplit("::")` would collapse a lone-colon name.
        let qualified = super::handlers::qualify(&ns_prefix, raw_name);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = arg_tokens[0].span;
        // **W314** — the class name has no absolute written form (#934).
        self.emit_w314_no_absolute_name(raw_name, name_span);
        let body_tok = arg_tokens[1];
        let doc = std::mem::take(&mut self.last_comment);
        let mut class = ClassDef {
            name: simple.clone(),
            qualified_name: qualified.clone(),
            name_span,
            body_span: body_tok.span,
            metaclass: cmd_name.to_string(),
            doc,
            ..Default::default()
        };
        if !body.is_empty() {
            let definer = SnitDefiner { grammar };
            self.parse_snit_definition_body(
                body, body_tok, &mut class, &qualified, scope_path, &definer,
            );
        }
        // For `my`-dispatch resolution (issue #923 idx 52) — see
        // `class_body_spans`'s doc.
        self.result
            .class_body_spans
            .push((qualified.clone(), class.body_span));
        self.result.all_classes.insert(qualified, class.clone());
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
            self.lexer_config(),
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
        // names.  Which members are pure declarations (a `VarWrite` arg and no
        // body — `variable` / `typevariable` / `component` / `typecomponent`)
        // and which argument holds the name are read from the registry grammar,
        // not a hardcoded keyword list.  Snit names every *type*-scoped member
        // with a `type` prefix, so that family convention routes the name to the
        // type- vs instance-variable set.
        for cmd in &cmds {
            if cmd.is_partial {
                continue;
            }
            let Some((sub, sub_args)) = cmd.texts.split_first() else {
                continue;
            };
            let Some(member) = grammar.member(sub) else {
                continue;
            };
            if !is_var_declaration(member) {
                continue;
            }
            let Some(name) = member
                .indices_for(ArgRole::VarWrite)
                .next()
                .and_then(|i| sub_args.get(i))
            else {
                continue;
            };
            if sub.starts_with("type") {
                type_vars.push(name.clone());
            } else {
                instance_vars.push(name.clone());
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
            if cmd.is_partial {
                continue;
            }
            if let Some((sub, sub_args)) = cmd.texts.split_first() {
                let sub_tokens = cmd.argv.get(1..).unwrap_or(&[]);
                let mut ctx = ClassBodyCtx {
                    grammar,
                    class_def,
                    class_qualified,
                    scope_path,
                };
                self.dispatch_snit_member(
                    sub,
                    sub_args,
                    sub_tokens,
                    &mut ctx,
                    &instance_vars,
                    &type_vars,
                );
            }
        }
    }

    /// Dispatch one snit body subcommand to the matching method extractor (or,
    /// for `proc`, the ordinary proc handler).  Split out of
    /// [`Self::parse_snit_definition_body`] so the two-pass walk stays small.
    ///
    /// Recognition (is this a member?) and argument layout (which words are the
    /// name / parameter list / value var / body) are read from the registry
    /// grammar member; only the analyser-level *semantics* — the target
    /// [`MethodDef::kind`], whether the body sees instance or type variables,
    /// and the synthetic name of the nameless forms — are decided here.
    fn dispatch_snit_member(
        &mut self,
        sub: &str,
        sub_args: &[String],
        sub_tokens: &[Token],
        ctx: &mut ClassBodyCtx<'_>,
        instance_vars: &[String],
        type_vars: &[String],
    ) {
        // snit allows a type-private `proc name args body` — analyse it as an
        // ordinary proc in the enclosing scope, not a method.
        if sub == "proc" {
            // No per-argument single-token info is threaded this deep into
            // the snit member dispatcher; `&[]` is the same safe default
            // `resolve_dynamic_word` already falls back to elsewhere (a
            // dynamic type-private proc name still gets a chance to resolve
            // via `fold_interpolation_single`, just not the single-`$var`
            // fast path).
            self.handle_proc_command(sub_args, sub_tokens, &[], ctx.scope_path);
            return;
        }
        let Some(member) = ctx.grammar.member(sub) else {
            return;
        };
        // Only body-bearing members define a walkable method scope; pure
        // declarations (`variable` …) and option/delegate members carry none.
        if member.indices_for(ArgRole::Body).next().is_none() {
            return;
        }
        // Snit names every *type*-scoped member with a `type` prefix — the
        // family convention that decides whether the body sees the type or the
        // instance variables, and which `MethodDef` bucket receives it.
        let is_type = sub.starts_with("type");
        let seed_vars = if is_type { type_vars } else { instance_vars };
        let kind = if is_type {
            "classmethod"
        } else if sub == "constructor" {
            "constructor"
        } else if sub == "destructor" {
            "destructor"
        } else {
            "method"
        };
        let label = snit_member_label(member, sub, sub_args);
        let form = MemberForm {
            member,
            kind,
            label: &label,
            visibility: "public",
        };
        self.extract_class_member(sub_args, sub_tokens, ctx, seed_vars, &form);
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
        let Some(body_idx) = member.indices_for(ArgRole::Body).next() else {
            return;
        };
        let Some(body_text) = args.get(body_idx).cloned() else {
            return;
        };
        let body_tok = arg_tokens.get(body_idx).copied();
        let name_idx = member.indices_for(ArgRole::Name).next();
        let params_idx = member.indices_for(ArgRole::ParamList).next();

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
            .map_or_else(Vec::new, |p| parse_param_list(p));
        for i in member.indices_for(ArgRole::VarWrite) {
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
            name_span,
            body_span,
            kind: kind.to_string(),
            is_self_method: false,
            visibility: visibility.to_string(),
            doc: String::new(),
            forward_target: None,
        };
        match kind {
            "constructor" => ctx.class_def.constructors.push(method_def),
            "destructor" => ctx.class_def.destructor = Some(method_def),
            "classmethod" => {
                ctx.class_def.class_methods.insert(name.clone(), method_def);
            }
            _ => {
                ctx.class_def.methods.insert(name.clone(), method_def);
            }
        }

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
            // rename can't overwrite the body.  (Precise snit declaration spans
            // are a follow-up.)
            let no_var_spans = std::collections::HashMap::new();
            self.walk_method_body(
                seed_vars,
                &no_var_spans,
                ctx.class_qualified,
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
        let Some(grammar) = self
            .definition_grammar(cmd_name)
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
        // current at the call, not the lexical one (issue #923 idx 85).
        let ns_prefix = self.command_resolution_namespace(scope_path);
        // Constructed key in, construction-inverse tail out (#934): a colon
        // trim or `rsplit("::")` would collapse a lone-colon name.
        let qualified = super::handlers::qualify(&ns_prefix, raw_name);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = arg_tokens[0].span;
        // **W314** — the class name has no absolute written form (#934).
        self.emit_w314_no_absolute_name(raw_name, name_span);
        let body_tok = arg_tokens[1];
        let doc = std::mem::take(&mut self.last_comment);
        let mut class = ClassDef {
            name: simple.clone(),
            qualified_name: qualified.clone(),
            name_span,
            body_span: body_tok.span,
            metaclass: cmd_name.to_string(),
            doc,
            ..Default::default()
        };
        if !body.is_empty() {
            self.parse_itcl_definition_body(
                body, body_tok, &mut class, &qualified, scope_path, grammar,
            );
        }
        // For `my`-dispatch resolution (issue #923 idx 52) — see
        // `class_body_spans`'s doc.
        self.result
            .class_body_spans
            .push((qualified.clone(), class.body_span));
        self.result.all_classes.insert(qualified, class.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path) {
            scope.classes.insert(simple, class);
        }
        true
    }

    /// Parse an itcl class body: `inherit` → superclasses, `variable` / `common`
    /// → instance/class variables, `method` / `proc` / `constructor` /
    /// `destructor` → method scopes.  Two passes (so a method can reference any
    /// variable regardless of declaration order); access modifiers are unwrapped
    /// via [`unwrap_wrapper_member`].
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
            self.lexer_config(),
        );

        // itcl injects `this` (the object's own command) into every method body.
        let implicit_vars = grammar.implicit_vars;
        let mut instance_vars: Vec<String> =
            implicit_vars.iter().map(|s| (*s).to_string()).collect();

        // Pass 1: declared `variable` / `common` names + `inherit` bases.
        for cmd in &cmds {
            if cmd.is_partial {
                continue;
            }
            let Some((kw, kw_args, _kw_toks, _vis)) =
                unwrap_wrapper_member(grammar, &cmd.texts, &cmd.argv)
            else {
                continue;
            };
            // A base class an `inherit` names is a command reference (the same
            // registry-driven path TclOO's `superclass`/`mixin` use), so
            // find-references / go-to-definition / rename reach it across files.
            self.record_member_command_references(grammar, &cmd.texts, &cmd.argv, scope_path);
            match kw {
                "variable" | "common" => {
                    if let Some(name) = kw_args.first() {
                        instance_vars.push(name.clone());
                    }
                }
                "inherit" => class_def.superclasses.extend(kw_args.iter().cloned()),
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
            let Some((kw, kw_args, kw_toks, vis)) =
                unwrap_wrapper_member(grammar, &cmd.texts, &cmd.argv)
            else {
                continue;
            };
            let Some(member) = grammar.member(kw) else {
                continue;
            };
            // `variable` / `common` are declarations (handled in pass 1) — skip
            // them even though `variable`'s optional config body carries an
            // `ArgRole::Body` (that body is highlighted by the token walker, not
            // recorded as a method here).  `inherit` and the like carry no body.
            if matches!(kw, "variable" | "common")
                || member.indices_for(ArgRole::Body).next().is_none()
            {
                continue;
            }
            // A class-scoped `proc` maps to the class-method bucket; constructor
            // / destructor to their dedicated fields; everything else a method.
            let kind = match kw {
                "proc" => "classmethod",
                "constructor" => "constructor",
                "destructor" => "destructor",
                _ => "method",
            };
            let label = match kw {
                "constructor" => "<constructor>",
                "destructor" => "<destructor>",
                _ => "",
            };
            let mut ctx = ClassBodyCtx {
                grammar,
                class_def,
                class_qualified,
                scope_path,
            };
            let form = MemberForm {
                member,
                kind,
                label,
                visibility: vis,
            };
            self.extract_class_member(kw_args, kw_toks, &mut ctx, &instance_vars, &form);
        }
    }

    /// Detect dispatch shape of a user-defined ``unknown`` proc.
    ///
    /// Lowers the proc body to IR, then walks the resulting
    /// top-level [`Statement`]s looking for:
    ///
    /// - `IRSwitch` whose subject is `$<first_param>` (or
    ///   `${first_param}`) — exact arms become explicit
    ///   dispatch targets; glob/regexp modes flip
    ///   ``has_pattern_dispatch``.  ``string tolower`` /
    ///   ``string toupper`` in the subject sets
    ///   ``case_insensitive``.
    /// - `IRCall` / `IRBarrier` whose command name matches one
    ///   of [`CHAIN_TARGETS`] — sets ``chains_original``.
    /// - ``exec`` calls — set ``has_exec``.
    /// - ``auto_load`` calls — set ``has_auto_load``.
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
        let module: Module = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::lowering::lower_to_ir(body, &tcl_registry::CommandRegistry::build_default())
        })) {
            Ok(module) => module,
            Err(_) => {
                return UnknownProcInfo {
                    chains_original: true,
                    case_insensitive: true,
                    has_pattern_dispatch: true,
                    has_exec: true,
                    has_auto_load: true,
                    ..Default::default()
                };
            }
        };

        let mut info = UnknownProcInfo::default();

        for stmt in &module.top_level.statements {
            walk_unknown_stmt(stmt, &first_param, &mut info, 0);
        }

        info
    }
}

/// Walk an inline ``oo::define Class subcmd ...`` form,
/// dispatching the same per-subcommand logic.
///
/// Reuses [`apply_oo_subcommand`] — the inline form differs
/// from the body form only in how arguments are framed; the
/// per-subcommand handling is identical.
pub(super) fn parse_oo_define_inline(
    grammar: &DefinitionBodyGrammar,
    args: &[String],
    arg_tokens: &[Token],
    class_def: &mut ClassDef,
) {
    if args.is_empty() {
        return;
    }
    // Synthesise a single fake "command" matching what the
    // body walker would have produced.
    apply_oo_subcommand(grammar, args, arg_tokens, class_def);
}

/// Depth cap for [`walk_unknown_stmt`]'s recursion over nested `if`/`for`/
/// `while`/`foreach`/`catch`/`try`/`switch`/`Block`/`UpFrame` bodies —
/// issue #996. Transitively bounded today via `MAX_LOWER_NEST_DEPTH`
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
fn walk_unknown_stmt(stmt: &Statement, first_param: &str, info: &mut UnknownProcInfo, depth: u32) {
    if MAX_UNKNOWN_STMT_WALK_DEPTH.exceeded(depth) {
        return;
    }
    match stmt {
        Statement::Switch {
            subject,
            arms,
            mode,
            ..
        } => {
            // Subject reference: ``$first`` or ``${first}``
            // (both forms are checked).
            let dollar = format!("${first_param}");
            let braced = format!("${{{first_param}}}");
            let subject_refs_first = subject.contains(&dollar) || subject.contains(&braced);

            if subject_refs_first {
                if subject.contains("string tolower") || subject.contains("string toupper") {
                    info.case_insensitive = true;
                }
                if *mode == SwitchMode::Exact {
                    for arm in arms {
                        if arm.pattern != "default" {
                            info.dispatch_targets.insert(arm.pattern.clone());
                        }
                    }
                } else {
                    info.has_pattern_dispatch = true;
                }
            }
            // Recurse into arm bodies (a switch arm may contain
            // an exec or auto_load that should still register).
            for arm in arms {
                if let Some(body) = &arm.body {
                    for inner in &body.statements {
                        walk_unknown_stmt(inner, first_param, info, depth + 1);
                    }
                }
            }
        }
        Statement::Call { command, .. } | Statement::Barrier { command, .. } => {
            if CHAIN_TARGETS.contains(&command.as_str()) {
                info.chains_original = true;
            } else if command == "exec" {
                info.has_exec = true;
            } else if command == "auto_load" {
                info.has_auto_load = true;
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for clause in clauses {
                for inner in &clause.body.statements {
                    walk_unknown_stmt(inner, first_param, info, depth + 1);
                }
            }
            if let Some(body) = else_body {
                for inner in &body.statements {
                    walk_unknown_stmt(inner, first_param, info, depth + 1);
                }
            }
        }
        Statement::For { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => {
            for inner in &body.statements {
                walk_unknown_stmt(inner, first_param, info, depth + 1);
            }
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            for inner in &body.statements {
                walk_unknown_stmt(inner, first_param, info, depth + 1);
            }
            for handler in handlers {
                for inner in &handler.body.statements {
                    walk_unknown_stmt(inner, first_param, info, depth + 1);
                }
            }
            if let Some(body) = finally_body {
                for inner in &body.statements {
                    walk_unknown_stmt(inner, first_param, info, depth + 1);
                }
            }
        }
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Return { .. } => {}
    }
}

/// Collect the two class-level (non-method) body kinds a definition body
/// can carry: a `property`'s `-get`/`-set` accessor scripts, and an
/// `initialise`/`initialize` block.
///
/// Neither is a method frame, so [`collect_method_body`] — restricted to the
/// four real method-bearing members — does not see them, yet each needs its
/// own scope in phase 2. Split out of `parse_oo_definition_body` purely to
/// keep that walker within its line budget.
fn collect_class_level_bodies(
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    accessor_bodies: &mut Vec<CollectedMethodBody>,
    init_bodies: &mut Vec<CollectedMethodBody>,
) {
    match texts.first().map(String::as_str) {
        Some("property") => collect_property_accessor_bodies(texts, argv, accessor_bodies),
        Some(kw @ ("initialise" | "initialize")) => {
            if let Some(body_idx) = grammar
                .member(kw)
                .and_then(|m| m.indices_for(ArgRole::Body).next())
                .map(|i| i + 1)
                && let (Some(body), Some(tok)) = (texts.get(body_idx), argv.get(body_idx).copied())
                && tok.kind == TokenType::Str
            {
                init_bodies.push(CollectedMethodBody {
                    name: format!("<{kw}>"),
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
/// dropped entirely, with no diagnostic (issue #923 idx 53).  Verified
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
/// ([`crate::segmenter::flatten_clause_list_elements`]), so each spliced
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
/// entry, hence no document-symbol node, no dispatch arity, and no body walk
/// (issue #1081).
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
    let cmds =
        crate::segmenter::segment_commands_with_offset_and_config(body_text, base_offset, config);
    let mut out: Vec<(Vec<String>, Vec<Token>)> = Vec::new();
    for cmd in &cmds {
        if cmd.is_partial || cmd.argv.is_empty() {
            continue;
        }
        let spliced = splice_static_member_expansions(cmd);
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
    cmd: &crate::segmenter::SegmentedCommand,
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
            for (element, element_tok) in crate::segmenter::flatten_clause_list_elements(text, *tok)
            {
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
/// A `TclOO` method body collected during the class-body walk, to be analysed
/// in a [`ScopeKind::Method`] scope once the whole `ClassDef` is populated.
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
    /// parameter's definition span at its name (issue #727). `None` for
    /// `destructor` (no parameter list).
    params_tok: Option<Token>,
    /// True for a **class-side** member (`classmethod`, `self method`) —
    /// and for any collected body that is not a real instance-side method
    /// invocation frame (class `initialise` scripts, `property` accessor
    /// bodies). Gates [`super::types::Scope::oo_defining_class`]: `[self
    /// class]` *raises* in a class-side frame, so no defining-class fact may
    /// be recorded for one (issue #1132).
    class_side: bool,
}

/// Recognise a method-defining subcommand in a class body and return its body
/// to walk in a fresh [`ScopeKind::Method`] scope.
///
/// The member's argument layout — which word is the name, the parameter list,
/// and the recursable body — comes entirely from its registry
/// [`MemberSpec`] arg-roles (never hardcoded indices), so a definer that adds
/// or reshapes a method-bearing member is picked up from the grammar alone.
///
/// Restricted to the members whose body is a *method* body: `initialise` /
/// `initialize` are class-level init scripts (walked in the enclosing scope by
/// the caller) and `private` is a visibility wrapper / block, so both are
/// excluded here even though the grammar marks them as carrying a body. The
/// `forward` form has no body; dynamic (non-braced) bodies are filtered
/// downstream by [`Analyser::walk_method_body`].
fn collect_method_body(
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
) -> Option<CollectedMethodBody> {
    // Unwrap a leading `self`/`private` modifier first (issue #923 idx
    // 120): its body would otherwise never be walked at all (no internal
    // diagnostics inside a `self method`/`private method` body — confirmed
    // empirically, a deliberately-wrong-arity call inside one drew
    // nothing, while the identical call in a plain `method` body correctly
    // fired). `unwrap_wrapper_member` is a no-op for an already-bare
    // `method`/`classmethod`/`constructor`/`destructor` keyword — its
    // returned slices start right after the *effective* keyword either
    // way, wrapper word included, so no `+ 1` shift is needed below.
    let (keyword, texts, argv, modifier) = unwrap_wrapper_member(grammar, texts, argv)?;
    if !matches!(
        keyword,
        "method" | "classmethod" | "constructor" | "destructor"
    ) {
        return None;
    }
    // `classmethod` and `self method` define on the class object — a
    // class-side frame, where `[self class]` never answers the written
    // class (tclsh 9.0.4: it raises "method not defined by a class" in a
    // `self method`, and answers the internal `::oo::ObjN:: oo ::delegate`
    // class in a `classmethod`).
    let class_side = keyword == "classmethod" || modifier == "self";
    let member = grammar.member(keyword)?;
    let body_idx = member.indices_for(ArgRole::Body).next()?;
    let params_idx = member.indices_for(ArgRole::ParamList).next();
    let name_idx = member.indices_for(ArgRole::Name).next();

    let body_text = texts.get(body_idx)?.clone();
    let body_tok = *argv.get(body_idx)?;
    // A named member (`method NAME …`) uses its name word; the nameless forms
    // (`constructor` / `destructor`) get a synthetic `<keyword>` name.
    let name = name_idx
        .and_then(|i| texts.get(i))
        .map_or_else(|| format!("<{keyword}>"), String::clone);
    let params = params_idx
        .and_then(|i| texts.get(i))
        .map_or_else(Vec::new, |p| parse_param_list(p));
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

fn apply_oo_private(
    grammar: &DefinitionBodyGrammar,
    sub_args: &[String],
    sub_tokens: &[Token],
    class_def: &mut ClassDef,
) {
    if sub_args.is_empty() {
        return;
    }
    let inner_subcmd = sub_args[0].as_str();
    let inner_args: &[String] = &sub_args[1..];
    let inner_tokens: &[Token] = if sub_tokens.len() > 1 {
        &sub_tokens[1..]
    } else {
        &[]
    };
    let Some(member) = grammar.member(inner_subcmd) else {
        return;
    };
    // `private deletemethod m` removes an instance-side member — `private`'s
    // own side — including one this same block just recorded; `private unexport
    // m` / `private { … export m … }` flip that same side, exactly as the
    // unwrapped spelling does, and `private filter s` fills the *instance*
    // filter slot (`info class filters` — pinned on tclsh 9.0.4; `private` does
    // not exist on 8.6 at all, where the whole body is an `invalid command
    // name` error).
    apply_sided_member_effects(
        member,
        inner_subcmd,
        inner_args,
        inner_tokens,
        class_def,
        MemberSide::Instance,
    );
    match inner_subcmd {
        "method" => {
            if let Some(md) =
                extract_method_def(member, inner_args, inner_tokens, "method", "private", "")
            {
                class_def.methods.insert(md.name.clone(), md);
            }
        }
        "classmethod" => {
            if let Some(md) = extract_method_def(
                member,
                inner_args,
                inner_tokens,
                "classmethod",
                "private",
                "",
            ) {
                class_def.class_methods.insert(md.name.clone(), md);
            }
        }
        _ => {}
    }
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
/// # The rename really moves the member (issue #1121)
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
/// # Definition-aborting shapes (issue #1120)
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
/// workspace advertising a method that sourcing the extension deletes (issue
/// #1101 review). A retraction of a member the same document declares needs no
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
            // a move (issue #1167) — the stub has no `MethodDef` to re-key, so
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
/// one file never reaches another file's class-command dispatch (issue #1119).
/// The two pairs stay strictly apart — `exports`/`unexports` are the
/// *instance*-side record by contract (`workspace_index`'s effective-export
/// union over `instance_method`, `rename_safety`'s "the class declares this
/// name" test), so a class-object-side flip landing there would silently
/// re-state an unrelated instance method's export bit (issue #1098).
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

/// Every effect a member word has on the members it *names*, applied to the one
/// side the word is scoped to — the single decision path all three spellings go
/// through (unwrapped, `self`-scoped, `private`-scoped).
///
/// Keeping it one function is what makes "which side does this word act on" a
/// caller's single argument rather than three near-copies that can drift: the
/// unwrapped and `private` spellings pass [`MemberSide::Instance`], `self`
/// passes [`MemberSide::ClassObject`], and every effect below follows.
///
/// `names` / `arg_tokens` are the word's arguments after the member keyword,
/// aligned index for index.
fn apply_sided_member_effects(
    member: &MemberSpec,
    keyword: &str,
    names: &[String],
    arg_tokens: &[Token],
    class_def: &mut ClassDef,
    side: MemberSide,
) {
    retract_named_members(member, names, arg_tokens, class_def, side);
    apply_visibility_member(member, names, class_def, side);
    apply_filter_member(member, keyword, names, class_def, side);
}

/// `filter f…` / `self filter f…` — fold the call into this side's
/// method-filter slot.
///
/// The two sides are separate slots that intercept different dispatches (see
/// [`ClassDef::filters`] and [`ClassDef::class_filters`] for the oracle), so
/// `filter` was the last member table still landing in one flat list regardless
/// of the wrapper it was written under (issue #1119).
///
/// The keyword is matched here rather than read off the spec because *which
/// `ClassDef` field a member routes to* is analyser-local semantics the registry
/// deliberately does not model — the same judgement as the `superclass` /
/// `mixin` / `variable` arms of [`apply_oo_subcommand`]. What the registry does
/// decide is *what the word does to the slot* ([`MemberSpec::slot`], issue
/// #1169): `filter a; filter b` leaves both live (`-append` default —
/// tclsh 9.0.4 / 8.6.16: `info class filters` → `a b`), and the explicit
/// `-set` / `-clear` / `-prepend` / `-remove` / `-appendifnew` operations
/// fold through [`tcl_registry::definer::SlotSpec::apply`] — the same fold
/// every other slot consumer uses, so they cannot diverge.  The registry
/// also decides that `filter` neither retracts nor flips visibility
/// ([`MemberSpec::retraction`] / [`MemberSpec::visibility_effect`] are both
/// `None` for it), which is why the two calls above leave it alone.
fn apply_filter_member(
    member: &MemberSpec,
    keyword: &str,
    names: &[String],
    class_def: &mut ClassDef,
    side: MemberSide,
) {
    if keyword != "filter" {
        return;
    }
    apply_slot_member(Some(member), names, side.filter_list(class_def));
}

/// Fold one slot-member call (`filter` / `superclass` / `mixin` /
/// `variable`) into `list` through the member's registry [`SlotSpec`]
/// (issue #1169) — the one place the analyser applies slot semantics, so
/// the instance / class-object filter slots, the superclass list, the mixin
/// list, and the declared-variable slot all take the identical fold.
///
/// A member with no slot spec (defensive fallback only — every `TclOO` slot
/// word carries one) keeps the pre-#1169 assignment reading.
fn apply_slot_member(member: Option<&MemberSpec>, args: &[String], list: &mut Vec<String>) {
    match member.and_then(|m| m.slot) {
        Some(slot) => slot.apply(list, args),
        None => *list = args.to_vec(),
    }
}

/// `self method NAME ARGS BODY` / `self classmethod NAME ARGS BODY`
/// (issue #923 idx 120) — `TclOO`'s own spelling for a class-level method,
/// the stock-library counterpart to `ooutil`'s `classmethod` keyword (both
/// end up dispatched through the class's own bound command). Either inner
/// spelling records into `class_methods`, tagged `is_self_method: true` so
/// the class-command MRO walk in `tcl-lsp-core` knows NOT to treat it as
/// inherited the way an `ooutil`-style `classmethod` is (real tclsh: a
/// subclass with no override does not gain a `self method` at all).
///
/// Consumes the **prefix** form only.  `self { method NAME ARGS BODY; … }`'s
/// block form (and `private`'s symmetric one) is normalised *into* this form
/// by [`expand_wrapper_block_members`] before the member walker runs, so both
/// spellings land here (issue #1081).
fn apply_oo_self(
    grammar: &DefinitionBodyGrammar,
    sub_args: &[String],
    sub_tokens: &[Token],
    class_def: &mut ClassDef,
) {
    if sub_args.is_empty() {
        return;
    }
    let inner_subcmd = sub_args[0].as_str();
    let inner_args: &[String] = &sub_args[1..];
    let inner_tokens: &[Token] = if sub_tokens.len() > 1 {
        &sub_tokens[1..]
    } else {
        &[]
    };
    let Some(member) = grammar.member(inner_subcmd) else {
        return;
    };
    // Every effect this word has on the members it names lands on the
    // class-object side — the wrapper's own — and nowhere else. `self
    // deletemethod m` / `self renamemethod old new` remove (or move) class-side
    // members, including ones this same block just recorded (issue #1095
    // review); `self filter f` fills the class object's own filter slot, which
    // intercepts dispatches on the *class command* (`::B cls`, and even `::B
    // new`) while leaving instances unfiltered — `info object filters ::B` ->
    // `f`, `info class filters ::B` -> empty, on tclsh 9.0.4 and 8.6.14 alike
    // (issue #1119). Handled before the declaration arms below so the wrapper's
    // own side is the only table touched.
    //
    // `self unexport m` / `self { … unexport m … }` flip the class-object
    // side's visibility and nothing else (issue #1098). Oracle, byte-identical
    // on tclsh 9.0.4 and 8.6.14:
    //
    //   oo::class create C { method m {} {…}
    //                        self { method m {} {…}; unexport m } }
    //   info object methods ::C   ;# -> (empty)     class-side `m` unexported
    //   info class methods ::C    ;# -> m           instance side untouched
    //   ::C m                     ;# -> unknown method "m"
    //   [::C new] m               ;# -> inst-m      still dispatches
    //
    // and a `self unexport` naming a method that exists only on the *other*
    // side is a silent no-op, not the hard error `deletemethod` raises:
    // `oo::class create E { method onlyinst {} {…} }; oo::define E { self
    // unexport onlyinst }` succeeds and leaves `onlyinst` exported on the
    // instance side. Restricting the flip to this side reproduces both.
    apply_sided_member_effects(
        member,
        inner_subcmd,
        inner_args,
        inner_tokens,
        class_def,
        MemberSide::ClassObject,
    );
    if !matches!(inner_subcmd, "method" | "classmethod") {
        return;
    }
    if let Some(mut md) = extract_method_def(
        member,
        inner_args,
        inner_tokens,
        "classmethod",
        "public",
        "",
    ) {
        md.visibility = default_visibility(grammar, &md.name);
        md.is_self_method = true;
        class_def.class_methods.insert(md.name.clone(), md);
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

/// Set the recorded visibility of each named member on `side` only to
/// `visibility` — the `export` / `unexport` member effect.
///
/// A name with no recorded member **on that side** is skipped: a later `method`
/// definition re-applies the name default anyway (tclsh 9.0.4-pinned), an
/// export-only stub has no declaration to navigate to, and — pinned on 9.0.4 and
/// 8.6.14 alike — naming a method that exists only on the *other* side really is
/// a no-op in Tcl, so skipping it is the faithful answer, not an abstention
/// (issue #1098).
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
/// see `Analyser::resolve_indirect_call_target`.
fn apply_oo_forward(
    grammar: &DefinitionBodyGrammar,
    sub_args: &[String],
    sub_tokens: &[Token],
    class_def: &mut ClassDef,
) {
    if let Some(name) = sub_args.first() {
        let span = sub_tokens
            .first()
            .map_or_else(|| tcl_lexer::Span::new(0, 0), |t| t.span);
        let forward_target = sub_args
            .get(1)
            .map(|target| (target.clone(), sub_args.get(2..).unwrap_or(&[]).to_vec()));
        let md = MethodDef {
            name: name.clone(),
            params: Vec::new(),
            params_computed: false,
            name_span: span,
            body_span: span,
            kind: "forward".to_string(),
            is_self_method: false,
            // A forward is dispatched by the same name rule as a method
            // (C computes `isPublic` identically for both).
            visibility: default_visibility(grammar, name),
            doc: String::new(),
            forward_target,
        };
        class_def.methods.insert(md.name.clone(), md);
    }
}

/// Extract a `constructor`/`destructor` member definition, anchoring its
/// name span on the keyword token (`argv[0]`) — neither has a name word of
/// its own, so editors land on the keyword for go-to-definition/hover.
/// Extracted from [`apply_oo_subcommand`] to keep it within the line
/// budget; the two members share this shape (`kind` param, no name word,
/// synthetic id) exactly, differing only in which `ClassDef` field the
/// caller stores the result into.
fn apply_oo_ctor_or_dtor(
    member: Option<&'static MemberSpec>,
    sub_args: &[String],
    sub_tokens: &[Token],
    argv: &[Token],
    kind: &str,
) -> Option<MethodDef> {
    let synthetic_id = if kind == "constructor" {
        "<constructor>"
    } else {
        "<destructor>"
    };
    let mut md = member
        .and_then(|m| extract_method_def(m, sub_args, sub_tokens, kind, "public", synthetic_id))?;
    if let Some(kw) = argv.first() {
        md.name_span = kw.span;
    }
    Some(md)
}

pub(super) fn apply_oo_subcommand(
    grammar: &DefinitionBodyGrammar,
    texts: &[String],
    argv: &[Token],
    class_def: &mut ClassDef,
) {
    let Some(subcmd) = texts.first().map(String::as_str) else {
        return;
    };
    let sub_args: &[String] = if texts.len() > 1 { &texts[1..] } else { &[] };
    let sub_tokens: &[Token] = if argv.len() > 1 { &argv[1..] } else { &[] };
    // The member's argument layout (name / params / body positions) comes from
    // its registry grammar spec; field routing below stays analyser-local.
    let member = grammar.member(subcmd);

    // A member word that *removes* the members it names — `deletemethod m`,
    // `renamemethod old new` — written with no `self` / `private` wrapper acts
    // on the instance side, so the class must not keep describing what it
    // deleted (issue #1101). Which words retract is registry data
    // ([`MemberSpec::retraction`]), never a keyword matched here;
    // the wrapped spellings route to their wrapper's own side in
    // [`apply_oo_self`] / [`apply_oo_private`]. Neither `deletemethod` nor
    // `renamemethod` has a declaring arm in the match below, so this is their
    // whole effect. Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
    //
    //   oo::class create ::I1 { method gone {} {…}; method kept {} {…}
    //                           deletemethod gone }
    //   info class methods ::I1   ;# -> kept
    //   oo::class create ::I3 { method old {} {…}; renamemethod old new }
    //   info class methods ::I3   ;# -> new          (`old` really is gone)
    //   oo::class create ::I4 { method gone {} {…} }
    //   oo::define ::I4 { deletemethod gone }
    //   info class methods ::I4   ;# -> (empty)
    //
    // Its sibling registry effect — the visibility a member word imposes
    // (`export` / `unexport`, [`MemberSpec::visibility_effect`]) — is applied
    // the same way and on the same side, so neither word needs an arm of its
    // own below: unwrapped, both act on the **instance** side only.
    // `oo::class create E2 { self { method onlyclass {} {…} } }` then
    // `oo::define E2 { unexport onlyclass }` leaves the class-object side's
    // `onlyclass` exported and dispatchable on 9.0.4 and 8.6.14 alike
    // (issue #1098).
    if let Some(m) = member {
        apply_sided_member_effects(
            m,
            subcmd,
            sub_args,
            sub_tokens,
            class_def,
            MemberSide::Instance,
        );
    }

    match subcmd {
        // `superclasses` / `mixins` feed the class-hierarchy graph (inherited
        // methods, MRO).  The navigable *references* to those base classes are
        // recorded separately as `command_invocations` by
        // `record_member_command_references`, so no per-name span is kept here.
        //
        // Both are slots (issue #1169): a bare list applies the slot's
        // C-pinned default operation — `-set` for `superclass` / `mixin`
        // (so the plain spelling still replaces), `-append` and friends
        // fold through the shared registry fold instead of being dropped
        // or, worse, recorded as class names.  tclsh 9.0.4:
        // `superclass A` then `superclass -append B` → `::A ::B`.
        "superclass" => {
            apply_slot_member(member, sub_args, &mut class_def.superclasses);
        }
        "mixin" => {
            apply_slot_member(member, sub_args, &mut class_def.mixins);
        }
        "method" => {
            if let Some(mut md) = member
                .and_then(|m| extract_method_def(m, sub_args, sub_tokens, "method", "public", ""))
            {
                // A method (re)definition applies the family's name-based
                // default export state, discarding any earlier explicit
                // `export`/`unexport` of the name (tclsh 9.0.4-pinned; the
                // rule itself is registry data — `[a-z]*` for TclOO).
                md.visibility = default_visibility(grammar, &md.name);
                class_def.methods.insert(md.name.clone(), md);
            }
        }
        "classmethod" => {
            if let Some(mut md) = member.and_then(|m| {
                extract_method_def(m, sub_args, sub_tokens, "classmethod", "public", "")
            }) {
                md.visibility = default_visibility(grammar, &md.name);
                class_def.class_methods.insert(md.name.clone(), md);
            }
        }
        "constructor" => {
            if let Some(md) =
                apply_oo_ctor_or_dtor(member, sub_args, sub_tokens, argv, "constructor")
            {
                class_def.constructors.push(md);
            }
        }
        "destructor" => {
            if let Some(md) =
                apply_oo_ctor_or_dtor(member, sub_args, sub_tokens, argv, "destructor")
            {
                class_def.destructor = Some(md);
            }
        }
        "variable" => {
            // Additive, not a reset (tclsh9.0-verified: `oo::define Cls
            // variable a b; oo::define Cls variable c` leaves all of `a`,
            // `b`, `c` live simultaneously — the same "always present in
            // every method" declaration `variable` inside a method body
            // itself would make, just issued once for the whole class
            // rather than per-call). A second `variable` statement in the
            // same class body must not silently discard the names the
            // first one declared (issue #923 idx 32, main audit wave).
            //
            // "Additive" because the class `variable` word is a slot whose
            // default operation is `-append` (with dedup — tclsh 9.0.4:
            // `variable a; variable a b` → `a b`); the explicit `-set` /
            // `-clear` / `-remove` operations fold through the same
            // registry fold as every other slot (issue #1169).
            apply_slot_member(member, sub_args, &mut class_def.variables);
        }
        // `filter` has no arm of its own: it is one of the sided member effects
        // above ([`apply_filter_member`]), so the unwrapped and `self` spellings
        // reach their own slot through the one shared path.
        "property" => {
            extract_property_defs(sub_args, sub_tokens, class_def);
        }
        "forward" => apply_oo_forward(grammar, sub_args, sub_tokens, class_def),
        "private" => apply_oo_private(grammar, sub_args, sub_tokens, class_def),
        "self" => apply_oo_self(grammar, sub_args, sub_tokens, class_def),
        // No `ClassDef` mutation here for the remaining subcommands.
        // ``initialise`` / ``initialize`` are class-level initialisation
        // scripts whose bodies are collected and walked separately in
        // [`Analyser::parse_oo_definition_body`]; everything else is ignored.
        _ => {}
    }
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
                "get" => getter_defined = true,
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
fn extract_method_def(
    member: &MemberSpec,
    args: &[String],
    arg_tokens: &[Token],
    kind: &str,
    visibility: &str,
    synthetic_name: &str,
) -> Option<MethodDef> {
    let zero = tcl_lexer::Span::new(0, 0);
    let body_idx = member.indices_for(ArgRole::Body).next()?;
    // A body word must be present for the member to be well-formed.
    args.get(body_idx)?;
    let name_idx = member.indices_for(ArgRole::Name).next();
    let params_idx = member.indices_for(ArgRole::ParamList).next();

    let name = name_idx
        .and_then(|i| args.get(i))
        .map_or_else(|| synthetic_name.to_string(), String::clone);
    let params = params_idx
        .and_then(|i| args.get(i))
        .map_or_else(Vec::new, |p| parse_param_list(p));
    let name_span = name_idx
        .and_then(|i| arg_tokens.get(i))
        .map_or(zero, |t| t.span);
    let body_span = arg_tokens.get(body_idx).map_or(zero, |t| t.span);
    Some(MethodDef {
        name,
        params,
        params_computed: false,
        name_span,
        body_span,
        kind: kind.to_string(),
        // Flipped by the one caller that needs it (`apply_oo_self`) —
        // every other caller means it literally, so `false` is the
        // correct default here, not just a placeholder.
        is_self_method: false,
        visibility: visibility.to_string(),
        doc: String::new(),
        forward_target: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::types::MemberRetractionRecord;

    /// The `TclOO` definition-body grammar the `apply_oo_subcommand` /
    /// `extract_method_def` helpers read their member argument layout from —
    /// the same `&'static` the analyser fetches from the registry at runtime.
    fn tcloo() -> &'static DefinitionBodyGrammar {
        &tcl_registry::definer::TCLOO_GRAMMAR
    }

    fn class() -> ClassDef {
        ClassDef {
            name: "C".to_string(),
            qualified_name: "::C".to_string(),
            name_span: tcl_lexer::Span::new(0, 0),
            body_span: tcl_lexer::Span::new(0, 0),
            ..Default::default()
        }
    }

    /// The `W315` messages an analysis produced, in emission order — the
    /// "this class definition cannot run" reports of issue #1120.
    fn w315_messages(r: &super::super::types::AnalysisResult) -> Vec<String> {
        r.diagnostics
            .iter()
            .filter(|d| d.code == tcl_core_types::DiagCode::W315)
            .map(|d| d.message.clone())
            .collect()
    }

    fn has_w315(r: &super::super::types::AnalysisResult) -> bool {
        !w315_messages(r).is_empty()
    }

    fn tok(span: (u32, u32)) -> Token {
        Token::new(TokenType::Esc, tcl_lexer::Span::new(span.0, span.1))
    }

    fn str_tok(span: (u32, u32)) -> Token {
        Token {
            kind: TokenType::Str,
            span: tcl_lexer::Span::new(span.0, span.1),
            content_offset: 1,
            in_quote: false,
        }
    }

    #[test]
    fn superclass_subcommand_assigns_supers() {
        let mut cd = class();
        let texts: Vec<String> = ["superclass", "::A", "::B"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 10)), tok((11, 14)), tok((15, 18))];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert_eq!(cd.superclasses, vec!["::A", "::B"]);
    }

    #[test]
    fn mixin_subcommand_strips_dash_flags() {
        let mut cd = class();
        let texts: Vec<String> = ["mixin", "-append", "::M1", "::M2"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 5)), tok((6, 13)), tok((14, 18)), tok((19, 23))];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert_eq!(cd.mixins, vec!["::M1", "::M2"]);
    }

    /// Apply a sequence of definition-body words to one class — the shape
    /// slot folding (issue #1169) is about: later words must fold into,
    /// not overwrite, earlier slot state.
    fn apply_words(cd: &mut super::ClassDef, calls: &[&[&str]]) {
        for words in calls {
            let texts: Vec<String> = words.iter().map(|s| (*s).to_string()).collect();
            let argv: Vec<tcl_lexer::Token> = words.iter().map(|_| tok((0, 1))).collect();
            apply_oo_subcommand(tcloo(), &texts, &argv, cd);
        }
    }

    // ---- issue #1169: slot semantics for filter / superclass / mixin /
    //      variable (defaults pinned against tclOODefineCmds.c 9.0.4 and
    //      confirmed live on tclsh 9.0.4) ----

    // TP: `filter a ; filter b` appends — `info class filters` → `a b`,
    // NOT `b` (the pre-#1169 last-writer-wins reading).
    #[test]
    fn filter_slot_appends_across_calls() {
        let mut cd = class();
        apply_words(&mut cd, &[&["filter", "a"], &["filter", "b"]]);
        assert_eq!(cd.filters, vec!["a", "b"]);
    }

    // TP: the explicit operations fold — `-set` replaces, `-clear` empties,
    // `-prepend` front-inserts (tclsh 9.0.4: `filter a; filter -prepend b`
    // → `b a`), `-remove` deletes.
    #[test]
    fn filter_slot_explicit_operations_fold() {
        let mut cd = class();
        apply_words(&mut cd, &[&["filter", "a", "b"], &["filter", "-set", "x"]]);
        assert_eq!(cd.filters, vec!["x"]);
        apply_words(&mut cd, &[&["filter", "-clear"]]);
        assert!(cd.filters.is_empty());
        apply_words(&mut cd, &[&["filter", "a"], &["filter", "-prepend", "b"]]);
        assert_eq!(cd.filters, vec!["b", "a"]);
        apply_words(&mut cd, &[&["filter", "-remove", "b"]]);
        assert_eq!(cd.filters, vec!["a"]);
    }

    // FP guard: an operation word is recognised at argument 0 only —
    // `filter a -set b` appends three literal items (tclsh 9.0.4), it does
    // not replace the slot with `b`.
    #[test]
    fn filter_slot_op_word_only_recognised_first() {
        let mut cd = class();
        apply_words(&mut cd, &[&["filter", "a", "-set", "b"]]);
        assert_eq!(cd.filters, vec!["a", "-set", "b"]);
    }

    // TP: `superclass` defaults to `-set` (replace) — the common single
    // declaration keeps its meaning — and `-append` extends (tclsh 9.0.4:
    // `superclass A` after `superclass B` → `::B`; `superclass -append A`
    // → `::B ::A`).
    #[test]
    fn superclass_slot_replaces_by_default_and_appends_explicitly() {
        let mut cd = class();
        apply_words(&mut cd, &[&["superclass", "::A"], &["superclass", "::B"]]);
        assert_eq!(cd.superclasses, vec!["::B"]);
        apply_words(&mut cd, &[&["superclass", "-append", "::A"]]);
        assert_eq!(cd.superclasses, vec!["::B", "::A"]);
    }

    // TP: `mixin` defaults to `-set` in 8.6 and 9.0 alike (both C sources
    // forward `--default-operation` to `-set`; tclsh 9.0.4:
    // `mixin M1; mixin M2` → `::M2`).
    #[test]
    fn mixin_slot_replaces_by_default() {
        let mut cd = class();
        apply_words(&mut cd, &[&["mixin", "::M1"], &["mixin", "::M2"]]);
        assert_eq!(cd.mixins, vec!["::M2"]);
    }

    // TP: the class `variable` slot appends with dedup (tclsh 9.0.4:
    // `variable a; variable a b` → `a b`) and folds `-set` / `-remove`.
    #[test]
    fn variable_slot_appends_dedups_and_folds_ops() {
        let mut cd = class();
        apply_words(&mut cd, &[&["variable", "a"], &["variable", "a", "b"]]);
        assert_eq!(cd.variables, vec!["a", "b"]);
        apply_words(&mut cd, &[&["variable", "-remove", "a"]]);
        assert_eq!(cd.variables, vec!["b"]);
        apply_words(&mut cd, &[&["variable", "-set", "c"]]);
        assert_eq!(cd.variables, vec!["c"]);
    }

    // TN: an unknown leading `-op` aborts the whole definition in real Tcl
    // (`unknown method "-bogus"`), so the fold must leave the slot state
    // unchanged rather than record `-bogus`/`x` as filters.
    #[test]
    fn unknown_slot_op_leaves_slot_untouched() {
        let mut cd = class();
        apply_words(&mut cd, &[&["filter", "a"], &["filter", "-bogus", "x"]]);
        assert_eq!(cd.filters, vec!["a"]);
    }

    // TP, full pipeline: the fold carries across definition blocks — the
    // issue #1169 oracle transcript verbatim (tclsh 9.0.4 and 8.6.16,
    // byte-identical):
    //   oo::class create C { filter a; filter b } → info class filters → a b
    //   oo::define C { filter -set x }            →                    → x
    //   oo::define C { filter -clear }            →                    → (empty)
    #[test]
    fn filter_slot_folds_across_oo_define_blocks() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { filter a\nfilter b }\n\
             oo::define ::C { filter -set x }\n",
            "tcl9.0",
        );
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert_eq!(c.filters, vec!["x"]);

        let mut a2 = Analyser::new();
        let r2 = a2.analyse(
            "oo::class create ::C { filter a\nfilter b }\n\
             oo::define ::C { filter -append c }\n",
            "tcl9.0",
        );
        let c2 = r2.all_classes.get("::C").expect("::C recorded");
        assert_eq!(c2.filters, vec!["a", "b", "c"]);
    }

    // The class-object side folds through the same slot spec: `self filter`
    // appends across calls, and never leaks into the instance slot.
    #[test]
    fn class_side_filter_slot_folds_and_stays_sided() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { self { filter a }\nself { filter b } }\n",
            "tcl9.0",
        );
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert_eq!(c.class_filters, vec!["a", "b"]);
        assert!(c.filters.is_empty());
    }

    #[test]
    fn method_subcommand_records_method_def() {
        let mut cd = class();
        let texts: Vec<String> = ["method", "greet", "name", "puts $name"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 6)), tok((7, 12)), tok((13, 17)), str_tok((18, 32))];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert!(cd.methods.contains_key("greet"));
        let md = &cd.methods["greet"];
        assert_eq!(md.kind, "method");
        assert_eq!(md.visibility, "public");
    }

    #[test]
    fn classmethod_subcommand_records_class_method() {
        let mut cd = class();
        let texts: Vec<String> = ["classmethod", "build", "args", "return $args"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [
            tok((0, 11)),
            tok((12, 17)),
            tok((18, 22)),
            str_tok((23, 38)),
        ];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert!(cd.class_methods.contains_key("build"));
        assert!(!cd.methods.contains_key("build"));
    }

    #[test]
    fn constructor_appends_to_constructors_list() {
        let mut cd = class();
        let texts: Vec<String> = ["constructor", "args", "puts ctor"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 11)), tok((12, 16)), str_tok((17, 28))];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert_eq!(cd.constructors.len(), 1);
        assert_eq!(cd.constructors[0].kind, "constructor");
        assert_eq!(cd.constructors[0].name, "<constructor>");
        // Name span anchors on the `constructor` keyword token
        // (argv[0] = 0..11), not the default (0, 0).
        assert_eq!(cd.constructors[0].name_span, tcl_lexer::Span::new(0, 11));
        // Constructors are no longer mirrored into the methods map.
        assert!(!cd.methods.contains_key("<constructor>"));
    }

    #[test]
    fn destructor_sets_destructor_field() {
        let mut cd = class();
        let texts: Vec<String> = ["destructor", "puts dtor"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 10)), str_tok((11, 22))];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        let dtor = cd.destructor.as_ref().expect("destructor recorded");
        assert_eq!(dtor.kind, "destructor");
        assert_eq!(dtor.name, "<destructor>");
        // Name span anchors on the `destructor` keyword token
        // (argv[0] = 0..10).
        assert_eq!(dtor.name_span, tcl_lexer::Span::new(0, 10));
        assert!(!cd.methods.contains_key("<destructor>"));
    }

    #[test]
    fn forward_records_method_with_forward_kind() {
        let mut cd = class();
        let texts: Vec<String> = ["forward", "delegate", "::other::cmd"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 7)), tok((8, 16)), tok((17, 29))];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert!(cd.methods.contains_key("delegate"));
        assert_eq!(cd.methods["delegate"].kind, "forward");
    }

    #[test]
    fn private_method_records_with_private_visibility() {
        let mut cd = class();
        let texts: Vec<String> = ["private", "method", "internal", "args", "puts hi"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [
            tok((0, 7)),
            tok((8, 14)),
            tok((15, 23)),
            tok((24, 28)),
            str_tok((29, 37)),
        ];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert!(cd.methods.contains_key("internal"));
        assert_eq!(cd.methods["internal"].visibility, "private");
    }

    #[test]
    fn self_method_subcommand_records_class_method_tagged_is_self_method() {
        // TP — issue #923 idx 120 Part 1: `self method NAME ARGS BODY`
        // (TclOO's own spelling of a class-level method, the stock
        // counterpart to ooutil's `classmethod` keyword) previously had no
        // `apply_oo_subcommand` arm at all — `class_methods` never gained
        // an entry for it. Recorded with `kind: "classmethod"` (both
        // spellings mean "dispatched via the class's own bound command")
        // but tagged `is_self_method` so the class-command MRO walk knows
        // NOT to treat it as inherited the way ooutil's `classmethod` is.
        let mut cd = class();
        let texts: Vec<String> = ["self", "method", "make", "n", "return made"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [
            tok((0, 4)),
            tok((5, 11)),
            tok((12, 16)),
            tok((17, 18)),
            str_tok((19, 30)),
        ];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert!(!cd.methods.contains_key("make"));
        let md = cd
            .class_methods
            .get("make")
            .expect("recorded as a classmethod");
        assert_eq!(md.kind, "classmethod");
        assert!(md.is_self_method);
        assert_eq!(md.visibility, "public");
    }

    #[test]
    fn self_classmethod_subcommand_also_records_into_class_methods() {
        // TP — the `self classmethod` inner spelling (rarer, but valid:
        // `self` always retargets to the class object regardless of which
        // inner member follows) must resolve identically to `self method`.
        let mut cd = class();
        let texts: Vec<String> = ["self", "classmethod", "build", "args", "return $args"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [
            tok((0, 4)),
            tok((5, 16)),
            tok((17, 22)),
            tok((23, 27)),
            str_tok((28, 43)),
        ];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        let md = cd
            .class_methods
            .get("build")
            .expect("recorded as a classmethod");
        assert!(md.is_self_method);
    }

    #[test]
    fn self_block_form_is_a_silent_noop_not_a_crash() {
        // FN guard — the `self { method NAME ARGS BODY }` *block* form is a
        // documented, symmetric (private shares the same gap) follow-up,
        // not fixed here. Must decline cleanly, never panic on the whole
        // braced blob standing in for an inner keyword.
        let mut cd = class();
        let texts: Vec<String> = ["self", "{ method make {n} { return made } }"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 4)), str_tok((5, 42))];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert!(cd.class_methods.is_empty());
        assert!(cd.methods.is_empty());
    }

    #[test]
    fn unrecognised_subcommand_is_silent_noop() {
        let mut cd = class();
        let texts: Vec<String> = ["whatever", "x"].iter().map(|s| (*s).to_string()).collect();
        let argv = [tok((0, 8)), tok((9, 10))];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        // No fields populated; no panic.
        assert!(cd.methods.is_empty());
        assert!(cd.superclasses.is_empty());
        assert!(cd.mixins.is_empty());
    }

    #[test]
    fn variable_subcommand_records_class_variables() {
        let mut cd = class();
        let texts: Vec<String> = ["variable", "x", "y"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 8)), tok((9, 10)), tok((11, 12))];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert_eq!(cd.variables, vec!["x", "y"]);
    }

    #[test]
    fn a_second_variable_subcommand_accumulates_rather_than_replacing() {
        // TP — issue #923 idx 32 (main audit wave): the real corpus shape
        // (georgtree_tclopt's ::tclopt::Mpfit) has TWO separate `variable`
        // statements in the same class body (`variable funct m ftol ...`
        // then, separately, `variable Pars`). tclsh9.0-verified: both
        // statements' names are live, simultaneous instance variables —
        // `variable` inside a class body is additive, never a reset (the
        // same "always present in every method" declaration a `variable`
        // command inside a method body itself would make, just issued
        // once for the whole class). A second statement must not silently
        // discard the first's names.
        let mut cd = class();
        apply_oo_subcommand(
            tcloo(),
            &["variable".to_string(), "funct".to_string(), "m".to_string()],
            &[tok((0, 8)), tok((9, 14)), tok((15, 16))],
            &mut cd,
        );
        apply_oo_subcommand(
            tcloo(),
            &["variable".to_string(), "Pars".to_string()],
            &[tok((20, 28)), tok((29, 33))],
            &mut cd,
        );
        assert_eq!(cd.variables, vec!["funct", "m", "Pars"]);
    }

    #[test]
    fn filter_subcommand_records_filters() {
        let mut cd = class();
        let texts: Vec<String> = ["filter", "log", "trace"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [tok((0, 6)), tok((7, 10)), tok((11, 16))];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert_eq!(cd.filters, vec!["log", "trace"]);
        // …and only the instance slot: the class object stays unfiltered.
        assert!(cd.class_filters.is_empty(), "{:?}", cd.class_filters);
    }

    #[test]
    fn export_and_unexport_record_sets() {
        let mut cd = class();
        let texts1: Vec<String> = ["export", "foo", "bar"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv1 = [tok((0, 6)), tok((7, 10)), tok((11, 14))];
        apply_oo_subcommand(tcloo(), &texts1, &argv1, &mut cd);
        assert!(cd.exports.contains("foo"));
        assert!(cd.exports.contains("bar"));

        let texts2: Vec<String> = ["unexport", "baz"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv2 = [tok((0, 8)), tok((9, 12))];
        apply_oo_subcommand(tcloo(), &texts2, &argv2, &mut cd);
        assert!(cd.unexports.contains("baz"));
    }

    #[test]
    fn property_subcommand_records_property_def() {
        let mut cd = class();
        let texts: Vec<String> = [
            "property",
            "colour",
            "-kind",
            "readable",
            "-get",
            "return red",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
        let argv = [
            tok((0, 8)),
            tok((9, 15)),
            tok((16, 21)),
            tok((22, 30)),
            tok((31, 35)),
            str_tok((36, 47)),
        ];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        let pd = cd.properties.get("colour").expect("colour recorded");
        assert_eq!(pd.kind, "readable");
        assert!(pd.has_getter);
        assert!(!pd.has_setter);
    }

    /// `property` is a 9.0 `TclOO` member: under 8.6 it is flagged
    /// disabled-in-dialect (W002), but from 9.0 on it is accepted silently.
    ///
    /// Either way the member is **recorded** — reporting "this needs Tcl
    /// 9.0" must not also break go-to-definition, references, rename, and
    /// document symbols over the code the user wrote, and the whole-command
    /// W002 behaves the same way (it reports a dialect-unavailable command
    /// while the analyser goes on modelling the call).
    #[test]
    fn property_member_is_gated_to_9_0() {
        let src = "oo::class create C {\n    property color\n}\n";
        let mut a86 = Analyser::new();
        let r86 = a86.analyse(src, "tcl8.6");
        assert!(
            r86.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W002),
            "property should be W002 under 8.6",
        );
        assert!(
            r86.all_classes
                .get("::C")
                .is_some_and(|c| c.properties.contains_key("color")),
            "the member is still recorded, so editor features keep working",
        );

        let mut a90 = Analyser::new();
        let r90 = a90.analyse(src, "tcl9.0");
        assert!(
            !r90.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W002),
            "property is available in 9.0",
        );
        assert!(
            r90.all_classes
                .get("::C")
                .is_some_and(|c| c.properties.contains_key("color")),
            "9.0 records the property",
        );
    }

    /// A configurable class answers `configure`/`cget` for its properties, so
    /// those accessor method words are folded into its known methods even
    /// though no `method` body defines them.
    #[test]
    fn configurable_class_knows_configure_and_cget() {
        let mut a = Analyser::new();
        let r = a
            .analyse(
                "oo::configurable create C {\n    property color\n}\n",
                "tcl9.0",
            )
            .clone();
        let known = r.class_hierarchy().known_methods("::C");
        assert!(known.contains(&"configure".to_owned()), "{known:?}");
        assert!(known.contains(&"cget".to_owned()), "{known:?}");
    }

    /// A command reference the analyser records for a class-body member
    /// argument (`superclass`/`mixin`/`inherit`/`forward` target).  `written`
    /// is the token text; `resolved` the qualified command it denotes.
    fn has_cmd_ref(
        r: &crate::analyser::types::AnalysisResult,
        written: &str,
        resolved: &str,
    ) -> bool {
        r.command_invocations.iter().any(|inv| {
            inv.name == written && inv.resolved_qualified_name.as_deref() == Some(resolved)
        })
    }

    // TP: a `superclass ::ns::Base` names the base class — recorded as a
    // command reference so references / go-to-definition / rename reach it.
    #[test]
    fn superclass_records_a_command_reference_to_the_base_class() {
        let mut a = Analyser::new();
        let r = a
            .analyse(
                "namespace eval ::ns {\n  oo::class create Base {}\n  \
                 oo::class create Sub { superclass ::ns::Base }\n}\n",
                "tcl9.0",
            )
            .clone();
        assert!(
            has_cmd_ref(&r, "::ns::Base", "::ns::Base"),
            "superclass must record a command reference: {:?}",
            r.command_invocations
                .iter()
                .map(|i| &i.name)
                .collect::<Vec<_>>()
        );
    }

    // TP: `mixin ::ns::Role` — the mixed-in class is a command reference.
    #[test]
    fn mixin_records_a_command_reference_to_the_class() {
        let mut a = Analyser::new();
        let r = a
            .analyse(
                "namespace eval ::ns {\n  oo::class create Role {}\n  \
                 oo::class create C { mixin ::ns::Role }\n}\n",
                "tcl9.0",
            )
            .clone();
        assert!(has_cmd_ref(&r, "::ns::Role", "::ns::Role"));
    }

    // Regression: generalising the `forward` target onto the member grammar's
    // `ArgRole::CommandName` must keep recording the delegated command.
    #[test]
    fn forward_target_still_records_a_command_reference() {
        let mut a = Analyser::new();
        let r = a
            .analyse(
                "namespace eval ::ns {\n  proc helper {} {}\n  \
                 oo::class create C { forward f ::ns::helper }\n}\n",
                "tcl9.0",
            )
            .clone();
        assert!(has_cmd_ref(&r, "::ns::helper", "::ns::helper"));
    }

    // TP: a bare `superclass Base` (same namespace) resolves to the enclosing
    // namespace's class — the one-hop call-site resolution, not an ancestor.
    #[test]
    fn bare_superclass_resolves_in_the_class_namespace() {
        let mut a = Analyser::new();
        let r = a
            .analyse(
                "namespace eval ::ns {\n  oo::class create Base {}\n  \
                 oo::class create Sub { superclass Base }\n}\n",
                "tcl9.0",
            )
            .clone();
        assert!(has_cmd_ref(&r, "Base", "::ns::Base"));
    }

    // TP (itcl): `inherit ::ns::Base` names a base class the same way.
    #[test]
    fn itcl_inherit_records_a_command_reference_to_the_base_class() {
        let mut a = Analyser::new();
        let r = a
            .analyse(
                "itcl::class ::ns::Base {}\n\
                 itcl::class ::ns::Sub { inherit ::ns::Base }\n",
                "tcl8.6",
            )
            .clone();
        assert!(
            has_cmd_ref(&r, "::ns::Base", "::ns::Base"),
            "itcl inherit must record a command reference: {:?}",
            r.command_invocations
                .iter()
                .map(|i| &i.name)
                .collect::<Vec<_>>()
        );
    }

    // TN: a dynamic base name (`superclass $base`) names no static command —
    // no reference is recorded (nothing to navigate or rename).
    #[test]
    fn dynamic_superclass_records_no_command_reference() {
        let mut a = Analyser::new();
        let r = a
            .analyse(
                "namespace eval ::ns {\n  \
                 oo::class create Sub { superclass $base }\n}\n",
                "tcl9.0",
            )
            .clone();
        assert!(
            !r.command_invocations
                .iter()
                .any(|inv| inv.name.contains('$')),
            "a dynamic superclass name must not be recorded as a reference"
        );
    }

    // FP guard: `mixin -append ::ns::M` — the `-append` flag is not a class,
    // only `::ns::M` is a reference.
    #[test]
    fn mixin_append_flag_is_not_a_command_reference() {
        let mut a = Analyser::new();
        let r = a
            .analyse(
                "namespace eval ::ns {\n  oo::class create M {}\n  \
                 oo::class create C { mixin -append ::ns::M }\n}\n",
                "tcl9.0",
            )
            .clone();
        assert!(
            has_cmd_ref(&r, "::ns::M", "::ns::M"),
            "the class is a reference"
        );
        assert!(
            !r.command_invocations
                .iter()
                .any(|inv| inv.name == "-append"),
            "the -append flag must not be recorded as a command reference"
        );
    }

    // TP: the inline `oo::define Sub superclass Base` form (no `{body}` block)
    // records the base-class reference too — the same as the braced body form.
    // (Regression guard: the inline path is separate from the body walk.)
    #[test]
    fn inline_oo_define_superclass_records_a_command_reference() {
        let mut a = Analyser::new();
        let r = a
            .analyse(
                "namespace eval ::ns {\n  oo::class create Base {}\n  \
                 oo::class create Sub {}\n  oo::define Sub superclass ::ns::Base\n}\n",
                "tcl9.0",
            )
            .clone();
        assert!(
            has_cmd_ref(&r, "::ns::Base", "::ns::Base"),
            "inline `oo::define … superclass` must record a command reference: {:?}",
            r.command_invocations
                .iter()
                .map(|i| &i.name)
                .collect::<Vec<_>>()
        );
    }

    // TP: a `forward` target may legally begin with `-` (a command named
    // `-foo`).  The `-`-flag filter applies only to class-list members
    // (`mixin -append`), never to the forward command name.
    #[test]
    fn forward_target_beginning_with_dash_is_recorded() {
        let mut a = Analyser::new();
        let r = a
            .analyse(
                "namespace eval ::ns {\n  oo::class create C { forward f -foo }\n}\n",
                "tcl9.0",
            )
            .clone();
        assert!(
            r.command_invocations.iter().any(|inv| inv.name == "-foo"),
            "a hyphen-prefixed forward target must be recorded: {:?}",
            r.command_invocations
                .iter()
                .map(|i| &i.name)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn property_subcommand_with_no_kind_defaults_to_readwrite() {
        let mut cd = class();
        let texts: Vec<String> = ["property", "x"].iter().map(|s| (*s).to_string()).collect();
        let argv = [tok((0, 8)), tok((9, 10))];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        let pd = cd.properties.get("x").expect("x recorded");
        assert_eq!(pd.kind, "readwrite");
        assert!(!pd.has_getter);
        assert!(!pd.has_setter);
    }

    #[test]
    fn property_subcommand_records_multiple_names() {
        let mut cd = class();
        let texts: Vec<String> = ["property", "x", "y", "-kind", "writable"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let argv = [
            tok((0, 8)),
            tok((9, 10)),
            tok((11, 12)),
            tok((13, 18)),
            tok((19, 27)),
        ];
        apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
        assert_eq!(cd.properties.len(), 2);
        assert_eq!(cd.properties["x"].kind, "writable");
        assert_eq!(cd.properties["y"].kind, "writable");
    }

    #[test]
    fn extract_method_def_too_few_args_returns_none() {
        // ``method`` with only 1 arg (just the name) — needs 3 (name, params,
        // body), so the grammar's `Body` word at index 2 is absent.
        let member = tcloo().member("method").expect("method is a member");
        let args: Vec<String> = vec!["foo".to_string()];
        let arg_tokens: Vec<Token> = vec![tok((0, 3))];
        let md = extract_method_def(member, &args, &arg_tokens, "method", "public", "");
        assert!(md.is_none());
    }

    fn param(name: &str) -> ParamDef {
        ParamDef {
            name: name.to_string(),
            has_default: false,
            default_value: None,
        }
    }

    /// Regression coverage for issue #996: `walk_unknown_stmt` recurses
    /// once per nested `if`/`for`/`while`/`foreach`/`catch`/`try`/
    /// `switch`/`Block`/`UpFrame` body, with no depth cap of its own
    /// before this fix. Transitively bounded to `MAX_LOWER_NEST_DEPTH`
    /// (256) by the lowering pass today, so this is defence-in-depth /
    /// consistency with every other full-tree walker in this crate, not a
    /// currently-reproducible crash. 1000 levels of *source* nesting is
    /// comfortably past this new cap; the assertion is that
    /// `extract_unknown_proc_info` returns at all, not what it returns.
    /// Spawns its own big-stack thread since the lexer/CST/segmenter
    /// stages upstream of `walk_unknown_stmt`'s own new cap still walk the
    /// full un-truncated source nesting before lowering's cap trims it —
    /// same rationale as `structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn extract_unknown_proc_info_survives_deep_nesting() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut body = String::new();
        for _ in 0..DEPTH {
            body.push_str("if {1} {\n");
        }
        body.push_str("exec ls\n");
        for _ in 0..DEPTH {
            body.push_str("}\n");
        }
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let mut a = Analyser::new();
                let _ = a.extract_unknown_proc_info(&body, &[param("cmd"), param("args")]);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn extract_unknown_proc_info_empty_body_marks_empty_stub() {
        let mut a = Analyser::new();
        let info = a.extract_unknown_proc_info("", &[param("cmd"), param("args")]);
        assert!(info.empty_stub);
        assert!(info.dispatch_targets.is_empty());
    }

    #[test]
    fn extract_unknown_proc_info_whitespace_body_marks_empty_stub() {
        let mut a = Analyser::new();
        let info = a.extract_unknown_proc_info("   \n  \t ", &[param("cmd"), param("args")]);
        assert!(info.empty_stub);
    }

    #[test]
    fn extract_unknown_proc_info_exact_switch_collects_dispatch_targets() {
        let mut a = Analyser::new();
        let body = r"switch -exact $cmd {
            foo { return 1 }
            bar { return 2 }
            default { return 0 }
        }";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(!info.empty_stub);
        assert!(info.dispatch_targets.contains("foo"));
        assert!(info.dispatch_targets.contains("bar"));
        assert!(!info.dispatch_targets.contains("default"));
        assert!(!info.has_pattern_dispatch);
    }

    #[test]
    fn extract_unknown_proc_info_glob_switch_marks_pattern_dispatch() {
        let mut a = Analyser::new();
        let body = r"switch -glob $cmd {
            foo* { return 1 }
            *bar { return 2 }
        }";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(info.has_pattern_dispatch);
        assert!(info.dispatch_targets.is_empty());
    }

    #[test]
    fn extract_unknown_proc_info_chains_original_via_known_target() {
        let mut a = Analyser::new();
        let body = r"_original_unknown $cmd $args";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(info.chains_original);
    }

    #[test]
    fn extract_unknown_proc_info_detects_exec_call() {
        let mut a = Analyser::new();
        let body = r"exec $cmd {*}$args";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(info.has_exec);
    }

    #[test]
    fn extract_unknown_proc_info_detects_auto_load_call() {
        let mut a = Analyser::new();
        let body = r"auto_load $cmd";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(info.has_auto_load);
    }

    #[test]
    fn extract_unknown_proc_info_case_insensitive_via_string_tolower() {
        let mut a = Analyser::new();
        let body = r"switch -exact [string tolower $cmd] {
            foo { return 1 }
        }";
        let info = a.extract_unknown_proc_info(body, &[param("cmd"), param("args")]);
        assert!(info.case_insensitive);
        assert!(info.dispatch_targets.contains("foo"));
    }

    #[test]
    fn extract_unknown_proc_info_no_first_param_defaults_to_cmd() {
        // Empty params list — the helper should fall back to
        // ``"cmd"`` as the dispatch-subject variable name.
        let mut a = Analyser::new();
        let body = r"switch -exact $cmd { foo { return 1 } }";
        let info = a.extract_unknown_proc_info(body, &[]);
        assert!(info.dispatch_targets.contains("foo"));
    }

    // ---- issue #1081: the `self { … }` / `private { … }` block forms ----

    #[test]
    fn self_block_form_records_every_member_as_a_class_method() {
        // TP. Oracle (tclsh 9.0.4 / 8.6.16, identical):
        //   oo::class create ::C {
        //       self { method make {n} {…} ; method other {} {…} }
        //       method inst {} {…}
        //   }
        //   ::C make 7               -> made-7
        //   info object methods ::C  -> other make   (class-object side)
        //   info class methods ::C   -> inst         (instance side)
        //   oo::class create ::F {superclass ::C} ; ::F make 1
        //                            -> error: unknown method "make"
        // i.e. exactly what the `self method …` prefix form declares — which
        // is why both spellings land on the same recording path.
        let src = "oo::class create ::C {\n\
                   self {\n\
                   method make {n} { return $n }\n\
                   method other {} { return other }\n\
                   }\n\
                   method inst {} { return inst }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        for name in ["make", "other"] {
            let md = c
                .class_methods
                .get(name)
                .unwrap_or_else(|| panic!("`{name}` must record as a class method"));
            assert_eq!(md.kind, "classmethod");
            assert!(
                md.is_self_method,
                "`self`-scoped members are not inherited by subclasses",
            );
            assert_eq!(md.visibility, "public");
        }
        // The instance side is untouched — `inst` stays an instance method and
        // the block's members never leak into it.
        assert!(c.methods.contains_key("inst"));
        assert!(!c.methods.contains_key("make"));
        assert!(!c.methods.contains_key("other"));
    }

    #[test]
    fn self_block_form_records_through_oo_define_too() {
        // TP. Same block, reached through `oo::define` rather than the class
        // creation body (oracle: `oo::define ::D { self { method mk {} {…} } }`
        // → `info object methods ::D` is `mk`).
        let src = "oo::class create ::D {}\n\
                   oo::define ::D {\n\
                   self {\n\
                   method mk {} { return dmk }\n\
                   }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::D").expect("::D recorded");
        assert!(c.class_methods.contains_key("mk"));
    }

    #[test]
    fn self_block_scoped_unexport_flips_the_class_side() {
        // TP, issue #1098. Oracle: `oo::class create ::E { self { method
        // hidden {} {…} ; unexport hidden } }` leaves `info object methods
        // ::E` empty while `-all -private` still lists `hidden`, and
        // `::E hidden` errors "unknown method" — so the block's `unexport`
        // really does apply to the class-object side. The member is recorded
        // (issue #1081) *and* now carries the visibility the block gave it.
        let src = "oo::class create ::E {\n\
                   self {\n\
                   method hidden {} { return h }\n\
                   unexport hidden\n\
                   }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::E").expect("::E recorded");
        assert!(c.class_methods.contains_key("hidden"));
        assert_eq!(c.class_methods["hidden"].visibility, "unexported");
    }

    #[test]
    fn self_scoped_unexport_leaves_a_same_named_instance_method_alone() {
        // TN, issue #1098's whole point — block and prefix spellings both.
        // Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
        //   oo::class create C { method m {} {return inst-m}
        //                        self { method m {} {return class-m}
        //                               unexport m } }
        //   info object methods ::C  ->            (class side unexported)
        //   info class methods ::C   -> m          (instance side untouched)
        //   ::C m                    -> unknown method "m"
        //   [::C new] m              -> inst-m
        for src in [
            "oo::class create ::C {\n\
             method m {} { return inst }\n\
             self {\n\
             method m {} { return class }\n\
             unexport m\n\
             }\n\
             }",
            "oo::class create ::C {\n\
             method m {} { return inst }\n\
             self method m {} { return class }\n\
             self unexport m\n\
             }",
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(src, "tcl9.0");
            let c = r.all_classes.get("::C").expect("::C recorded");
            assert_eq!(
                c.class_methods["m"].visibility, "unexported",
                "class side must be flipped",
            );
            assert_eq!(
                c.methods["m"].visibility, "public",
                "instance side must NOT be flipped by a `self`-scoped unexport",
            );
            assert!(
                !c.unexports.contains("m"),
                "the class-level `unexports` set is the instance-side record; a \
                 class-object-side unexport must not enter it",
            );
        }
    }

    #[test]
    fn unwrapped_unexport_leaves_the_class_side_alone() {
        // TN, the mirror direction. Oracle:
        //   oo::class create E2 { self { method onlyclass {} {…} } }
        //   oo::define E2 { unexport onlyclass }
        //   info object methods ::E2 -> onlyclass   (still exported)
        //   ::E2 onlyclass           -> oc          (still dispatches)
        // The unwrapped word acts on the instance side, where the name does
        // not exist — a silent no-op in real Tcl.
        let src = "oo::class create ::E2 {\n\
                   self { method onlyclass {} { return oc } }\n\
                   }\n\
                   oo::define ::E2 { unexport onlyclass }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::E2").expect("::E2 recorded");
        assert_eq!(c.class_methods["onlyclass"].visibility, "public");
    }

    #[test]
    fn cross_side_unexport_is_a_silent_no_op_not_an_error() {
        // TN / oracle-shape pin. Unlike `deletemethod` (a hard error that
        // aborts the definition), naming the *other* side's method in an
        // `export`/`unexport` is accepted and does nothing:
        //   oo::class create E { method onlyinst {} {…} }
        //   oo::define E { self unexport onlyinst }   ;# no error
        //   info class methods ::E  -> onlyinst       (still exported)
        //   [::E new] onlyinst      -> oi
        // so we record no visibility change and — crucially — no member stub.
        let src = "oo::class create ::E3 { method onlyinst {} { return oi } }\n\
                   oo::define ::E3 { self unexport onlyinst }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::E3").expect("::E3 recorded");
        assert_eq!(c.methods["onlyinst"].visibility, "public");
        assert!(!c.class_methods.contains_key("onlyinst"));
    }

    #[test]
    fn self_block_export_reexports_a_class_side_member() {
        // TP, the export half — last explicit state wins.
        // Oracle: `oo::class create H1 { self { method lower {} {…} ;
        // unexport lower ; export lower } }` -> `info object methods ::H1`
        // is `lower` on 9.0.4 and 8.6.14 alike.
        let src = "oo::class create ::H1 {\n\
                   self {\n\
                   method lower {} { return l }\n\
                   unexport lower\n\
                   export lower\n\
                   }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::H1").expect("::H1 recorded");
        assert_eq!(c.class_methods["lower"].visibility, "public");
    }

    #[test]
    fn private_scoped_visibility_members_flip_the_instance_side() {
        // TP, the third routing. `private` is 9.0-only (`invalid command
        // name "private"` on 8.6), and under it a visibility word acts on the
        // instance side exactly as the unwrapped spelling does. Oracle:
        //   oo::class create G1 { method m {} {…}; private { unexport m } }
        //   info class methods ::G1              ->            (unexported)
        //   info class methods ::G1 -all -private -> … m …
        //   oo::class create G3 { method m {} {…}; private export m }
        //   info class methods ::G3              -> m          (exported)
        for (src, want) in [
            (
                "oo::class create ::G1 {\n\
                 method m {} { return m }\n\
                 private { unexport m }\n\
                 }",
                "unexported",
            ),
            (
                "oo::class create ::G2 {\n\
                 method m {} { return m }\n\
                 private unexport m\n\
                 }",
                "unexported",
            ),
            (
                "oo::class create ::G3 {\n\
                 method m {} { return m }\n\
                 private export m\n\
                 }",
                "public",
            ),
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(src, "tcl9.0");
            let c = r
                .all_classes
                .values()
                .find(|c| c.methods.contains_key("m"))
                .expect("class recorded");
            assert_eq!(c.methods["m"].visibility, want, "{src}");
        }
    }

    #[test]
    fn private_block_form_records_instance_methods() {
        // TP, symmetric half — `private` is the other registry member marked
        // wrapper-with-block-body.
        let src = "oo::class create ::P {\n\
                   private {\n\
                   method secret {k} { return $k }\n\
                   }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::P").expect("::P recorded");
        assert_eq!(c.methods["secret"].visibility, "private");
        assert!(!c.class_methods.contains_key("secret"));
    }

    #[test]
    fn self_block_does_not_declare_instance_variables() {
        // TN / abstention boundary. `self { variable x }` declares a variable
        // on the *class object*, not an instance variable of the class — the
        // prefix form (`self variable x`) is not recorded either, and the
        // block form must not start recording it as one.
        let src = "oo::class create ::V {\n\
                   self {\n\
                   variable classlevel\n\
                   }\n\
                   variable instancelevel\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::V").expect("::V recorded");
        assert_eq!(c.variables, vec!["instancelevel".to_string()]);
    }

    #[test]
    fn self_introspection_prefix_is_not_a_block_member() {
        // TN. Inside a *method body*, `self class` is an introspection call,
        // not a definer member. The body is walked as a script (never as a
        // definition body), so nothing here may reach `class_methods`.
        let src = "oo::class create ::W {\n\
                   method whoami {} { return [self class] }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::W").expect("::W recorded");
        assert!(c.methods.contains_key("whoami"));
        assert!(c.class_methods.is_empty(), "{:?}", c.class_methods);
    }

    #[test]
    fn dynamic_self_block_abstains() {
        // Abstention boundary: a non-literal block word cannot be re-segmented
        // statically, so the expansion declines and the class records nothing
        // rather than guessing.
        let src = "set b {method make {n} { return $n }}\n\
                   oo::class create ::Dyn {\n\
                   self $b\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::Dyn").expect("::Dyn recorded");
        assert!(c.class_methods.is_empty(), "{:?}", c.class_methods);
        assert!(c.methods.is_empty(), "{:?}", c.methods);
    }

    #[test]
    fn self_block_destructive_member_drops_the_member_it_deletes() {
        // TP — issue #1095 review. Oracle (tclsh 9.0.4 / 8.6.16, identical):
        //   oo::class create ::C1 {
        //       self { method gone {} {…} ; method kept {} {…} ; deletemethod gone }
        //   }
        //   info object methods ::C1  ->  kept
        //   ::C1 gone                 ->  unknown method "gone"
        // Retaining `gone` would show a stale document symbol and let
        // navigation resolve a name the interpreter does not have.
        let src = "oo::class create ::C {\n                   self {\n                   method gone {} { return g }\n                   method kept {} { return k }\n                   deletemethod gone\n                   }\n                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(
            !c.class_methods.contains_key("gone"),
            "{:?}",
            c.class_methods
        );
        assert!(c.class_methods.contains_key("kept"));
    }

    #[test]
    fn self_block_renamemethod_moves_the_member_on_the_class_side() {
        // TP, class-side half of issue #1121. Oracle, byte-identical on tclsh
        // 9.0.4 and 8.6.14:
        //   oo::class create ::R2 { self { method old {} {return CLSOLD}
        //                                  renamemethod old new } }
        //   info object methods ::R2  -> new
        //   ::R2 new                  -> CLSOLD   (shadowing the stock `new`!)
        //   ::R2 old                  -> unknown method
        // The move stays on the wrapper's own side: the instance table is not
        // touched in either direction.
        let src = "oo::class create ::C {\n                   self {\n                   method old {} { return o }\n                   renamemethod old new\n                   }\n                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(
            !c.class_methods.contains_key("old"),
            "{:?}",
            c.class_methods
        );
        let md = c.class_methods.get("new").expect("`new` is a class member");
        assert_eq!(md.name, "new");
        assert!(
            src[md.body_span.start() as usize..md.body_span.end() as usize].contains("return o"),
        );
        assert!(
            c.methods.is_empty(),
            "instance side untouched: {:?}",
            c.methods
        );
        assert!(!has_w315(&r), "{:?}", w315_messages(&r));
    }

    #[test]
    fn private_block_destructive_member_drops_the_instance_side_member() {
        // TP, symmetric half. Oracle (9.0 — `private` is a 9.0 member):
        //   oo::class create ::C5 {
        //       private { method secret {} {…} ; method other {} {…} ;
        //                 deletemethod secret }
        //   }
        //   info class methods ::C5 -all -private   ->  no `secret`
        let src = "oo::class create ::C {\n                   private {\n                   method secret {} { return s }\n                   method other {} { return o }\n                   deletemethod secret\n                   }\n                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(!c.methods.contains_key("secret"), "{:?}", c.methods);
        assert!(c.methods.contains_key("other"));
    }

    #[test]
    fn wrapper_block_and_prefix_forms_retract_identically() {
        // The block form is normalised into the prefix form, so both spellings
        // must land on the same result. Oracle for the prefix side:
        //   oo::define ::C7 { self method sgone {} {…} }
        //   info object methods ::C7                  ->  sgone
        //   oo::define ::C7 { self deletemethod sgone }
        //   info object methods ::C7                  ->  (empty)
        let src = "oo::class create ::C {}\n                   oo::define ::C { self method gone {} { return g } }\n                   oo::define ::C { self deletemethod gone }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(c.class_methods.is_empty(), "{:?}", c.class_methods);
    }

    #[test]
    fn a_retraction_does_not_cross_to_the_other_side() {
        // TN. `self deletemethod` touches only the class-object side. Oracle:
        //   oo::class create ::D { method m {} {return inst}
        //                          self { method m {} {return cls} } }
        //   oo::define ::D { self deletemethod m }
        //   info class methods ::D   ->  m        (instance side survives)
        //   info object methods ::D  ->  (empty)
        //   [::D new] m              ->  inst
        let src = "oo::class create ::D {\n                   method m {} { return inst }\n                   self { method m {} { return cls } }\n                   }\n                   oo::define ::D { self deletemethod m }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::D").expect("::D recorded");
        assert!(c.methods.contains_key("m"), "instance side must survive");
        assert!(!c.class_methods.contains_key("m"), "class side must go");
    }

    #[test]
    fn non_retracting_method_reference_members_keep_their_member() {
        // TN, the discrimination the registry flag exists for: `export`,
        // `unexport`, and `filter` are the sibling `MemberRefKind::Method`
        // members — they *name* a method without removing it, so they must not
        // retract. Oracle:
        //   oo::class create ::A { self { method a {} {…} ; unexport a ; export a } }
        //   info object methods ::A  ->  a   ;  ::A a  ->  1
        //   oo::class create ::B { self { method f {} {…} ; filter f } }
        //   info object methods ::B  ->  f
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::A {\n self {\n method a {} { return 1 }\n unexport a\n export a\n }\n}",
            "tcl9.0",
        );
        assert!(
            r.all_classes["::A"].class_methods.contains_key("a"),
            "export/unexport must not retract",
        );
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::B {\n self {\n method f {} { return 1 }\n filter f\n }\n}",
            "tcl9.0",
        );
        assert!(
            r.all_classes["::B"].class_methods.contains_key("f"),
            "filter must not retract",
        );
    }

    #[test]
    fn a_retracted_member_can_be_redeclared_by_a_later_body() {
        // Source-order guard. Oracle:
        //   oo::class create ::C { method m {} {return 1} }
        //   oo::define ::C { self method m {} {return 2} }
        //   oo::define ::C { self deletemethod m }
        //   oo::define ::C { self method m {} {return 3} }
        //   ::C m        ->  3     (class side, redeclared after the delete)
        //   [::C new] m  ->  1     (instance side, never touched)
        // Retraction happens where the body is walked, so a later declaration
        // wins — it is not a whole-document erasure of the name.
        let src = "oo::class create ::C { method m {} { return 1 } }\n                   oo::define ::C { self method m {} { return 2 } }\n                   oo::define ::C { self deletemethod m }\n                   oo::define ::C { self method m {} { return 3 } }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(c.class_methods.contains_key("m"), "redeclaration must win");
        assert!(c.methods.contains_key("m"), "instance side untouched");
    }

    #[test]
    fn unwrapped_deletemethod_drops_the_instance_side_member() {
        // TP, issue #1101. An *unwrapped* `deletemethod` — in a class-creation
        // body or an `oo::define` body, with no `self` / `private` wrapper —
        // deletes on the instance side. Oracle, byte-identical on tclsh 9.0.4
        // and 8.6.14:
        //   oo::class create ::I1 { method gone {} {…}; method kept {} {…}
        //                           deletemethod gone }
        //   info class methods ::I1  ->  kept
        //   oo::class create ::I4 { method gone {} {…} }
        //   oo::define ::I4 { deletemethod gone }
        //   info class methods ::I4  ->            (empty)
        for src in [
            "oo::class create ::C {\n\
             method gone {} { return g }\n\
             method kept {} { return k }\n\
             deletemethod gone\n\
             }",
            "oo::class create ::C {\n\
             method gone {} { return g }\n\
             method kept {} { return k }\n\
             }\n\
             oo::define ::C { deletemethod gone }",
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(src, "tcl9.0");
            let c = r.all_classes.get("::C").expect("::C recorded");
            assert!(!c.methods.contains_key("gone"), "deleted member retained");
            assert!(c.methods.contains_key("kept"), "sibling member dropped");
        }
    }

    #[test]
    fn unwrapped_renamemethod_moves_the_member_to_the_new_name() {
        // TP, issue #1121 (the residual #1118 left deliberately). Oracle,
        // byte-identical on tclsh 9.0.4 and 8.6.14:
        //   oo::class create ::I3 { method old {} {return o}; renamemethod old new }
        //   info class methods ::I3        ->  new
        //   [::I3 new] new                 ->  o        (the old body runs)
        //   info class definition ::I3 new ->  {} { return o }
        //   ::I3 old                       ->  unknown method
        // so `new` is a fully navigable member carrying `old`'s body: `old` goes,
        // `new` arrives with the *same* `MethodDef`, its name span moved onto
        // the `renamemethod` call's destination word (the only place the new
        // name is written) and its body span left on the one real body.
        let mut a = Analyser::new();
        let src = "oo::class create ::C { method old {} { return o }\n\
             renamemethod old new }";
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(!c.methods.contains_key("old"));
        let md = c.methods.get("new").expect("`new` is a member");
        assert_eq!(md.name, "new");
        // The name span is the `renamemethod`'s destination word …
        assert_eq!(
            &src[md.name_span.start() as usize..md.name_span.end() as usize],
            "new"
        );
        // … and it is the *second* `new` in the source (the one in the
        // `renamemethod` call), not any earlier text.
        assert!(md.name_span.start() as usize > src.find("renamemethod").expect("call"));
        // … while the body span still points at the original body.
        assert!(
            src[md.body_span.start() as usize..md.body_span.end() as usize].contains("return o"),
            "body span must stay on the original body",
        );
        // A rename is not a (re)declaration, so the family's name-based export
        // default is not re-applied — the source's visibility travels with it.
        assert_eq!(md.visibility, "public");
        // TN for #1120: a rename onto a fresh name is a legal order.
        assert!(!has_w315(&r), "{:?}", w315_messages(&r));
        // …and the *destination* name leaves no cross-document tombstone: the
        // rename creates `new`, it does not delete it, so another document's
        // `method new` must survive. Registry data decides which arguments a
        // retracting word removes (`MemberRetraction::FirstArgument`).
        assert!(
            !c.retracted_members
                .iter()
                .any(|r| r.member == "new" || r.member == "old"),
            "a locally-declared retraction needs no tombstone: {:?}",
            c.retracted_members,
        );
    }

    // ---- issue #1119: the class-side visibility + filter channels ----

    #[test]
    fn self_scoped_visibility_words_fill_the_class_side_sets() {
        // TP, issue #1119. A `self export` / `self unexport` has to leave a
        // record of its own or the flip never travels to another file's
        // class-command dispatch. Oracle, byte-identical on tclsh 9.0.4/8.6.14:
        //   oo::class create ::X { self { method cm {} { return cm } } }
        //   ::X cm                    ;# -> cm
        //   oo::define X { self unexport cm }
        //   ::X cm                    ;# -> unknown method "cm": must be create,
        //                             ;#    destroy or new
        //   info object methods ::X   ;# -> (empty)
        //   info object methods ::X -all -private ;# -> … cm …  (still defined)
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { self { method cm {} { return c } } }\n\
             oo::define ::C { self unexport cm }",
            "tcl9.0",
        );
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(c.class_unexports.contains("cm"), "{:?}", c.class_unexports);
        assert!(c.class_exports.is_empty(), "{:?}", c.class_exports);
        assert_eq!(c.class_methods["cm"].visibility, "unexported");
        // …and the instance-side pair — the one every existing consumer reads
        // as the instance record — stays untouched in both directions.
        assert!(c.exports.is_empty(), "{:?}", c.exports);
        assert!(c.unexports.is_empty(), "{:?}", c.unexports);
    }

    #[test]
    fn a_rename_carries_the_sources_visibility_not_the_new_names_default() {
        // CRITICAL pin, issue #1121. A rename is not a (re)declaration, so the
        // family's name-based export default (`[a-z]*` is exported for `TclOO`)
        // is NOT re-applied to the destination — the source's own visibility
        // travels with the body. Both directions, byte-identical on tclsh 9.0.4
        // and 8.6.14:
        //   oo::class create ::R4 { method Priv {} {…} ; renamemethod Priv pub }
        //   info class methods ::R4           ;# -> (empty)   `pub` is UNexported
        //   info class methods ::R4 -private  ;# -> pub       despite the name
        //   [::R4 new] pub                    ;# -> unknown method "pub"
        //   oo::class create ::R5 { method low {} {…} ; renamemethod low Up }
        //   info class methods ::R5           ;# -> Up        still EXPORTED
        //   info class methods ::R5 -private  ;# -> Up        despite the name
        //
        // Applying the destination's default instead would flip both of these
        // the wrong way — offering a `pub` no `$obj pub` can call, and hiding an
        // `Up` that dispatches fine.
        for (src_name, dst_name, want) in [("Priv", "pub", "unexported"), ("low", "Up", "public")] {
            let src = format!(
                "oo::class create ::C {{ method {src_name} {{}} {{ return 1 }}\n\
                 renamemethod {src_name} {dst_name} }}"
            );
            let mut a = Analyser::new();
            let r = a.analyse(&src, "tcl9.0");
            let c = r.all_classes.get("::C").expect("::C recorded");
            assert_eq!(
                c.methods[dst_name].visibility, want,
                "{src_name} -> {dst_name}",
            );
        }
    }

    #[test]
    fn the_class_side_visibility_sets_keep_only_the_last_writer() {
        // The class-side pair takes the identical last-writer-exclusive rule as
        // the instance pair, because the workspace consumer reads any `exports`
        // entry as decisive. Oracle: `oo::class create ::H1 { self { method
        // lower {} {…} ; unexport lower ; export lower } }` leaves
        // `info object methods ::H1` -> lower on 9.0.4 and 8.6.14 alike.
        for (body, exported) in [
            ("unexport m\nexport m", true),
            ("export m\nunexport m", false),
        ] {
            let src = format!(
                "oo::class create ::C {{ self {{ method m {{}} {{ return 1 }}\n{body} }} }}"
            );
            let mut a = Analyser::new();
            let r = a.analyse(&src, "tcl9.0");
            let c = r.all_classes.get("::C").expect("::C recorded");
            let (winner, loser) = if exported {
                (&c.class_exports, &c.class_unexports)
            } else {
                (&c.class_unexports, &c.class_exports)
            };
            assert!(winner.contains("m"), "{body}");
            assert!(
                !loser.contains("m"),
                "{body}: superseded set still holds `m`"
            );
        }
    }

    #[test]
    fn an_unwrapped_visibility_word_never_touches_the_class_side_sets() {
        // TN, the mirror of `a_self_scoped_visibility_word_never_touches_the_
        // instance_sets`. The unwrapped spelling is instance-scoped, so an
        // identically-named class-side member keeps its own state — oracle:
        // `oo::class create E2 { self { method onlyclass {} {…} } }` then
        // `oo::define E2 { unexport onlyclass }` leaves `onlyclass` exported and
        // dispatchable via `::E2 onlyclass` on 9.0.4 and 8.6.14.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { self { method m {} { return c } } }\n\
             oo::define ::C { unexport m }",
            "tcl9.0",
        );
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(c.class_exports.is_empty(), "{:?}", c.class_exports);
        assert!(c.class_unexports.is_empty(), "{:?}", c.class_unexports);
        assert_eq!(c.class_methods["m"].visibility, "public");
        // The unwrapped word still records its own (instance-side) intent.
        assert!(c.unexports.contains("m"));
    }

    #[test]
    fn filters_are_sided_by_the_wrapper_they_are_written_under() {
        // TP + TN, issue #1119 item 2 — `filters` was the last un-sided member
        // table. The two slots are genuinely independent in `TclOO` and
        // intercept different dispatches. Oracle, byte-identical on tclsh 9.0.4
        // and 8.6.14:
        //   oo::class create ::A { method real {} {…} ; method logit {args} {…}
        //                          filter logit }
        //   info class filters ::A   ;# -> logit      info object filters ::A -> {}
        //   [::A new] real           ;# logit fires, `self target` -> `::A real`
        //   oo::class create ::B { method inst {} {…}
        //       self { method cls {} {…} ; method logit {args} {…} ; filter logit } }
        //   info object filters ::B  ;# -> logit      info class filters ::B  -> {}
        //   ::B cls                  ;# logit fires, `self target` -> `::B cls`
        //   ::B new                  ;# logit fires too — the constructor path
        //   [::B new] inst           ;# logit does NOT fire
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::A { method real {} { return r }\n\
             method logit {args} { next }\n\
             filter logit }\n\
             oo::class create ::B { method inst {} { return i }\n\
             self { method cls {} { return c }\n\
             method logit {args} { next }\n\
             filter logit } }",
            "tcl9.0",
        );
        let a_cls = r.all_classes.get("::A").expect("::A recorded");
        assert_eq!(a_cls.filters, vec!["logit"]);
        assert!(a_cls.class_filters.is_empty(), "{:?}", a_cls.class_filters);
        let b_cls = r.all_classes.get("::B").expect("::B recorded");
        assert_eq!(b_cls.class_filters, vec!["logit"]);
        assert!(b_cls.filters.is_empty(), "{:?}", b_cls.filters);
    }

    #[test]
    fn a_private_scoped_filter_is_instance_side() {
        // TN for the third spelling: `private` is a *visibility* wrapper over
        // the instance side, not a third side, so its `filter` lands in the
        // instance slot. Oracle (9.0 only — `private` does not exist on 8.6,
        // where the same body fails with `invalid command name "private"`):
        //   oo::class create ::P { private { method s {} {} ; filter s } }
        //   info class filters ::P   ;# -> s
        //   info object filters ::P  ;# -> (empty)
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { private { method s {} { return s }\n\
             filter s } }",
            "tcl9.0",
        );
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert_eq!(c.filters, vec!["s"]);
        assert!(c.class_filters.is_empty(), "{:?}", c.class_filters);
    }

    #[test]
    fn per_object_visibility_lands_in_object_member_state() {
        // Issue #1119 item 3, closed by #1170. `oo::objdefine $o { unexport
        // m }` really works — oracle, 9.0.4 and 8.6.14 alike:
        //   oo::class create ::C { method m {} {…} } ; set o [::C new]
        //   oo::objdefine $o { unexport m } ; $o m
        //   ;# -> unknown method "m": must be destroy or n
        // The flip now lands in the receiver binding's `ObjectMemberState`
        // — the durable per-object home — while the class itself stays
        // untouched (the leak guard from #1119 still holds), and the
        // throwaway holder still never reaches `all_classes`.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { method m {} { return m } }\n\
             set o [::C new]\n\
             oo::objdefine $o { unexport m }",
            "tcl9.0",
        );
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert_eq!(c.methods["m"].visibility, "public");
        assert!(c.unexports.is_empty(), "{:?}", c.unexports);
        assert!(!r.all_classes.contains_key("::@objdefine@::o"));
        let states = r
            .object_member_state
            .get("o")
            .expect("the receiver binding has member state");
        assert_eq!(states.len(), 1, "{states:?}");
        assert!(states[0].unexports.contains("m"), "{states:?}");
        assert!(states[0].exports.is_empty(), "{states:?}");
    }

    #[test]
    fn per_object_export_and_unexport_are_last_writer_exclusive() {
        // The per-object pair keeps the same last-writer-exclusive contract
        // as the class-side pairs: a later `export m` cancels the earlier
        // flip, across blocks of the same binding.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { method m {} { return m } }\n\
             set o [::C new]\n\
             oo::objdefine $o { unexport m }\n\
             oo::objdefine $o { export m }",
            "tcl9.0",
        );
        let states = r.object_member_state.get("o").expect("state recorded");
        assert_eq!(states.len(), 1, "one binding: {states:?}");
        assert!(states[0].exports.contains("m"), "{states:?}");
        assert!(!states[0].unexports.contains("m"), "{states:?}");
    }

    #[test]
    fn per_object_cross_block_retraction_folds_and_stays_silent() {
        // The cross-block hazard that kept W315 out of `oo::objdefine`
        // (issue #1170): a second block retracting what the first declared
        // is legal (tclsh 9.0.4 / 8.6.14 both accept it), so the seeded walk
        // must remove the member silently — no W315 — and the folded state
        // must drop it.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C {}\n\
             set o [::C new]\n\
             oo::objdefine $o { method im {} { return i } }\n\
             oo::objdefine $o { deletemethod im }",
            "tcl9.0",
        );
        assert!(w315_messages(&r).is_empty(), "{:?}", r.diagnostics);
        let states = r.object_member_state.get("o").expect("state recorded");
        assert_eq!(states.len(), 1, "{states:?}");
        assert!(
            states[0].methods.is_empty(),
            "the retraction must fold: {states:?}"
        );
    }

    #[test]
    fn a_retraction_of_a_locally_declared_member_leaves_no_tombstone() {
        // TN for the cross-document channel. Within one document the member is
        // removed outright and the order is known, so exporting a tombstone
        // would wrongly cancel a *different* document's redeclaration of the
        // name — no ordering evidence supports that.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { method gone {} { return g } }\n\
             oo::define ::C { deletemethod gone }",
            "tcl9.0",
        );
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(!c.methods.contains_key("gone"));
        assert!(c.retracted_members.is_empty(), "{:?}", c.retracted_members);
    }

    // ---- issue #1120: W315, "this class definition cannot run" ----

    #[test]
    fn w315_fires_for_every_definition_aborting_retraction() {
        // TP ×4. Real Tcl aborts the whole definition and creates no class at
        // all for each of these — byte-identical on tclsh 9.0.4 and 8.6.14, with
        // `[info object isa class …]` -> 0 in every case:
        //   { deletemethod ghost ; method ghost {} {} }   method ghost does not exist
        //   { self { method cm {} {} } ; deletemethod cm } method cm does not exist
        //   { method a {} {} ; method b {} {} ; renamemethod a b }
        //                                       method called b already exists
        //   { method a {} {} ; renamemethod a a } cannot rename method to itself
        // The message mirrors the interpreter's own text after a fixed preamble.
        for (body, want) in [
            (
                "deletemethod ghost\nmethod ghost {} { return g }",
                "this class definition cannot run: method \"ghost\" does not exist",
            ),
            (
                "self { method cm {} { return c } }\ndeletemethod cm",
                "this class definition cannot run: method \"cm\" does not exist",
            ),
            (
                "method a {} { return a }\nmethod b {} { return b }\nrenamemethod a b",
                "this class definition cannot run: method called \"b\" already exists",
            ),
            (
                "method a {} { return a }\nrenamemethod a a",
                "this class definition cannot run: cannot rename method to itself",
            ),
            // One word earns one report: a rename whose *source* is missing
            // fails on the source in real Tcl (`method ghost does not exist`),
            // never additionally on the destination it never reached.
            (
                "method b {} { return b }\nrenamemethod ghost b",
                "this class definition cannot run: method \"ghost\" does not exist",
            ),
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(&format!("oo::class create ::C {{ {body} }}"), "tcl9.0");
            assert_eq!(w315_messages(&r), vec![want.to_string()], "body: {body}");
            // Navigation resilience: the partial class is still recorded, the
            // same degradation a parse error gets.
            assert!(r.all_classes.contains_key("::C"), "body: {body}");
        }
    }

    #[test]
    fn w315_fires_on_the_class_side_too() {
        // TP, the sided half. A `self deletemethod` of an instance-only member
        // is the same hard error, because the class-object side has no such
        // member: `oo::class create ::E6 { method im {} {} ; self { deletemethod
        // im } }` -> `method im does not exist`, 9.0.4 and 8.6.14 alike. Same
        // for the cross-side rename (`::R3`).
        for body in [
            "method im {} { return i }\nself { deletemethod im }",
            "method im {} { return i }\nself { renamemethod im other }",
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(&format!("oo::class create ::C {{ {body} }}"), "tcl9.0");
            assert_eq!(
                w315_messages(&r),
                vec!["this class definition cannot run: method \"im\" does not exist".to_string()],
                "body: {body}",
            );
            // The member the word could not reach survives on its own side.
            assert!(r.all_classes["::C"].methods.contains_key("im"), "{body}");
        }
    }

    // W315 for `oo::objdefine` bodies (issue #1170) — possible at all only
    // because the per-object walk is seeded with the binding's cross-block
    // state; every reading demands positive, document-wide evidence.

    #[test]
    fn w315_fires_for_a_per_object_retraction_of_a_never_declared_member() {
        // TP. A per-object retraction reaches only the object's *own* table,
        // never a class member: `oo::objdefine $o { deletemethod ghost }`
        // errors `method ghost does not exist` on 9.0.4 and 8.6.14 alike —
        // even when the class provides `ghost`.  The receiver's construction
        // is in view and nothing in the document declares `ghost`
        // per-object, so the report is evidence-backed.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { method ghost {} { return g } }\n\
             set o [::C new]\n\
             oo::objdefine $o { deletemethod ghost }",
            "tcl9.0",
        );
        assert_eq!(
            w315_messages(&r),
            vec!["this object definition cannot run: method \"ghost\" does not exist".to_string()],
        );
    }

    #[test]
    fn w315_fires_for_a_per_object_rename_onto_an_existing_member() {
        // TP, cross-block: the destination's presence comes from an earlier
        // block of the same binding — exactly the state the unseeded walk
        // could not carry.  Oracle: `renamemethod a b` with both per-object
        // -> `method called "b" already exists` (9.0.4 / 8.6.14).
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C {}\n\
             set o [::C new]\n\
             oo::objdefine $o { method a {} { return a }\n method b {} { return b } }\n\
             oo::objdefine $o { renamemethod a b }",
            "tcl9.0",
        );
        assert_eq!(
            w315_messages(&r),
            vec![
                "this object definition cannot run: method called \"b\" already exists".to_string()
            ],
        );
    }

    #[test]
    fn w315_fires_for_a_per_object_rename_to_itself() {
        // TP. `renamemethod x x` errors against *any* table state (present:
        // `cannot rename method to itself`; absent: `method "x" does not
        // exist`), so this one needs no completeness gate at all.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::objdefine $o { method x {} { return x }\n renamemethod x x }",
            "tcl9.0",
        );
        assert_eq!(
            w315_messages(&r),
            vec!["this object definition cannot run: cannot rename method to itself".to_string()],
        );
    }

    #[test]
    fn per_object_w315_abstains_without_the_receivers_construction_in_view() {
        // TN (CRITICAL FP guard). A document that only *extends* an object it
        // never constructs is the per-object analogue of a `via_define` stub:
        // another file may have declared the member per-object, so the
        // retraction is the normal cross-file shape, not an error.
        let mut a = Analyser::new();
        let r = a.analyse("oo::objdefine $o { deletemethod ghost }", "tcl9.0");
        assert!(!has_w315(&r), "{:?}", w315_messages(&r));
    }

    #[test]
    fn per_object_w315_abstains_when_any_key_declares_the_member() {
        // TN (CRITICAL FP guard, the alias shape). The handle flowed through
        // a second spelling that declared the member per-object — tclsh:
        //   set p [::C new]; oo::objdefine $p { method ghost {} {} }
        //   set o $p; oo::objdefine $o { deletemethod ghost }   ;# succeeds
        // The keys are different bindings to the analyser, so the only sound
        // reading is "declared per-object somewhere in this document ⇒ the
        // retraction may be legal".
        // `o`'s own construction *is* in view, so the completeness gate
        // passes and only the declared-anywhere reading keeps this silent.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C {}\n\
             set p [::C new]\n\
             oo::objdefine $p { method ghost {} { return g } }\n\
             set o [::C new]\n\
             set o $p\n\
             oo::objdefine $o { deletemethod ghost }",
            "tcl9.0",
        );
        assert!(!has_w315(&r), "{:?}", w315_messages(&r));
    }

    #[test]
    fn per_object_w315_abstains_when_any_receiver_is_unresolved() {
        // TN (CRITICAL FP guard). An `oo::objdefine` whose receiver resolves
        // to nothing statically may define members on *any* object —
        // including the one another site retracts from — so the whole
        // document abstains.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C {}\n\
             set o [::C new]\n\
             oo::objdefine [pick] { method ghost {} { return g } }\n\
             oo::objdefine $o { deletemethod ghost }",
            "tcl9.0",
        );
        assert!(!has_w315(&r), "{:?}", w315_messages(&r));
    }

    #[test]
    fn w315_stays_silent_for_every_legal_order() {
        // TN ×7, all oracle-pinned as succeeding on tclsh 9.0.4 and 8.6.14.
        // Note especially the last two: rename onto a name *deleted earlier in
        // the same body* is legal, so the check has to read the side's table
        // state at the point the word runs, not the body's final contents.
        for body in [
            // declare-then-retract, both sides
            "method a {} {}\nmethod b {} {}\ndeletemethod a",
            "self { method a {} {}\nmethod b {} {}\ndeletemethod a }",
            // rename onto a fresh name
            "method a {} {}\nrenamemethod a fresh",
            // rename onto a name that exists only on the *other* side — legal:
            // `oo::class create ::R7 { method a {} {} ; self { method b {} {} }
            //  ; renamemethod a b }` -> info class methods ::R7 -> b
            "method a {} {}\nself { method b {} {} }\nrenamemethod a b",
            // delete then redeclare the same name
            "method a {} {}\ndeletemethod a\nmethod a {} {}",
            // rename onto a name deleted earlier in the same body
            "method a {} {}\nmethod b {} {}\ndeletemethod b\nrenamemethod a b",
            // chained renames
            "method a {} {}\nrenamemethod a b\nrenamemethod b c",
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(&format!("oo::class create ::C {{ {body} }}"), "tcl9.0");
            assert!(!has_w315(&r), "body: {body} -> {:?}", w315_messages(&r));
        }
    }

    #[test]
    fn a_cross_side_visibility_word_is_not_a_w315() {
        // TN (CRITICAL). `export`/`unexport` are the words whose cross-side form
        // is a **silent no-op**, not the hard error `deletemethod` raises — the
        // distinction #1118's oracle pinned. Byte-identical on 9.0.4 and 8.6.14:
        //   oo::class create N1 { method onlyinst {} {} }
        //   oo::define N1 { self unexport onlyinst }  ;# succeeds, no effect
        //   oo::class create N2 { self { method onlyclass {} {} } }
        //   oo::define N2 { unexport onlyclass }      ;# succeeds, no effect
        //   oo::define N3 { export ghost }            ;# succeeds even for a
        //   oo::define N4 { self export ghost }       ;# name nothing declares
        for body in [
            "method onlyinst {} {}\nself unexport onlyinst",
            "self { method onlyclass {} {} }\nunexport onlyclass",
            "export ghost",
            "self export ghost",
            "filter ghost",
            "self filter ghost",
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(&format!("oo::class create ::C {{ {body} }}"), "tcl9.0");
            assert!(!has_w315(&r), "body: {body} -> {:?}", w315_messages(&r));
        }
    }

    #[test]
    fn w315_abstains_on_a_dynamic_source_name() {
        // TN. A `$var` / `[cmd]` *source* name resolves to nothing statically:
        // it may or may not name a live member, so neither the removal, the
        // tombstone, nor the abort has evidence behind it — abstain on all
        // three rather than guess.
        for body in [
            "method m {} {}\ndeletemethod $gone",
            "method m {} {}\nrenamemethod $old new",
            "method m {} {}\ndeletemethod [pick]",
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(&format!("oo::class create ::C {{ {body} }}"), "tcl9.0");
            assert!(!has_w315(&r), "body: {body} -> {:?}", w315_messages(&r));
            assert!(
                r.all_classes["::C"].methods.contains_key("m"),
                "body: {body}: a dynamic retraction must not remove a member",
            );
            assert!(
                r.all_classes["::C"].retracted_members.is_empty(),
                "body: {body}: {:?}",
                r.all_classes["::C"].retracted_members,
            );
        }
    }

    #[test]
    fn a_dynamic_rename_destination_still_retracts_the_source() {
        // TN for the *destination* half, which is not symmetric with the source.
        // Whatever `$n` turns out to be, `renamemethod m $n` definitely takes
        // `m` away — `::C m` is an unknown method afterwards on 9.0.4 and
        // 8.6.14 alike — so retracting the source is the faithful answer even
        // though the arrival name is unknowable. Only the arrival abstains, and
        // with no static destination there is nothing to check for a collision.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { method m {} { return m }\n\
             method n {} { return n }\n\
             renamemethod m $n }",
            "tcl9.0",
        );
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(!c.methods.contains_key("m"), "{:?}", c.methods);
        assert!(
            c.methods.contains_key("n"),
            "the collision guess is not made"
        );
        assert!(!has_w315(&r), "{:?}", w315_messages(&r));
    }

    #[test]
    fn a_cross_file_stub_retraction_is_a_tombstone_not_a_w315() {
        // TN (CRITICAL FP guard). This is the *normal* cross-file shape: the
        // class is created in another file, so this record has no member tables
        // to judge against and an absent name is expected, not an error. The
        // retraction travels as a tombstone and nothing is reported.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::define ::C { deletemethod m }\n\
             oo::define ::D { self deletemethod cm }\n\
             oo::define ::E { renamemethod old new }",
            "tcl9.0",
        );
        assert!(!has_w315(&r), "{:?}", w315_messages(&r));
        assert_eq!(
            r.all_classes["::C"].retracted_members,
            vec![MemberRetractionRecord::deletion(
                "m".to_string(),
                MemberSide::Instance
            )],
        );
        assert!(r.all_classes["::C"].via_define);
    }

    #[test]
    fn a_same_file_define_extending_a_local_class_is_not_a_stub() {
        // TP boundary of the `via_define` gate. An `oo::define` on a class this
        // same file created reuses that class's record, member tables included,
        // so its table state is complete and an absent name really is the hard
        // error — oracle: `oo::class create ::C { method kept {} {} }` then
        // `oo::define ::C { deletemethod ghost }` fails `method ghost does not
        // exist` on 9.0.4 and 8.6.14.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { method kept {} { return k } }\n\
             oo::define ::C { deletemethod ghost }",
            "tcl9.0",
        );
        assert_eq!(
            w315_messages(&r),
            vec!["this class definition cannot run: method \"ghost\" does not exist".to_string()],
        );
        assert!(!r.all_classes["::C"].via_define);
        // No tombstone: the two readings are mutually exclusive and the class
        // handler kept the one its knowledge supports.
        assert!(
            r.all_classes["::C"].retracted_members.is_empty(),
            "{:?}",
            r.all_classes["::C"].retracted_members,
        );
    }

    #[test]
    fn a_move_records_the_member_state_it_ran_against() {
        // Issue #1121 review. The moved member's declaration site is the
        // `renamemethod`'s destination word, so a *rename* of it rewrites that
        // word — and the gate that decides whether the result still runs needs
        // the table as it stood at the move. Captured with the source already
        // removed and the destination not yet inserted, which is exactly what
        // the interpreter reads at that point.
        let src = "oo::class create ::C { method old {} { return 1 }\n\
                   method sib {} { return 2 }\n\
                   renamemethod old new }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert_eq!(c.renamed_members.len(), 1, "{:?}", c.renamed_members);
        let moved = &c.renamed_members[0];
        assert_eq!(moved.source, "old");
        assert_eq!(moved.destination, "new");
        assert_eq!(moved.side, MemberSide::Instance);
        assert_eq!(
            &src[moved.destination_span.start() as usize..moved.destination_span.end() as usize],
            "new",
        );
        // `sib` is live at the move; `old` is not (it was just retracted) and
        // `new` is not (it has not arrived yet).
        assert_eq!(moved.blocked, vec!["sib".to_string()]);
    }

    #[test]
    fn the_move_record_answers_which_new_names_would_abort() {
        // The shared fold both consumers use: the walker asks it about the
        // destination actually written (that is how W315 is decided) and the
        // rename gate asks it about the name the user typed. Oracle, identical
        // on tclsh 9.0.4 and 8.6.14 — no class is created in either case:
        //   { method old {} {…} ; renamemethod old old }
        //        -> cannot rename method to itself
        //   { method old {} {…} ; method sib {} {…} ; renamemethod old sib }
        //        -> method called sib already exists
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { method old {} { return 1 }\n\
             method sib {} { return 2 }\n\
             renamemethod old new }",
            "tcl9.0",
        );
        let moved = &r.all_classes["::C"].renamed_members[0];
        assert_eq!(
            moved.abort_if_renamed_to("old"),
            Some(DefinitionAbortKind::RenameToItself),
        );
        assert_eq!(
            moved.abort_if_renamed_to("sib"),
            Some(DefinitionAbortKind::DestinationExists),
        );
        // TN: a fresh name is fine, and so is the destination it already has.
        assert_eq!(moved.abort_if_renamed_to("fresh"), None);
        // The mirror direction, from the same record.
        assert!(moved.collides_with_renaming("sib", "new"));
        assert!(!moved.collides_with_renaming("sib", "other"));
    }

    #[test]
    fn a_cross_side_name_is_not_in_the_move_snapshot() {
        // TN (CRITICAL). Collisions are side-local: `method old {} {…}` +
        // `self { method sib {} {…} }` + `renamemethod old sib` runs fine and
        // leaves `info class methods ::C1` -> sib (9.0.4 / 8.6.14). So a
        // class-side `sib` must not appear in an instance-side move's snapshot.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { method old {} { return 1 }\n\
             self { method sib {} { return 2 } }\n\
             renamemethod old new }",
            "tcl9.0",
        );
        let moved = &r.all_classes["::C"].renamed_members[0];
        assert!(moved.blocked.is_empty(), "{:?}", moved.blocked);
        assert_eq!(moved.abort_if_renamed_to("sib"), None);
    }

    #[test]
    fn the_move_snapshot_honours_body_order_in_both_directions() {
        // TN ×2, both oracle-pinned legal on 9.0.4 and 8.6.14:
        //   { method old {} {} ; method sib {} {} ; deletemethod sib
        //     renamemethod old sib }                 -> info class methods -> sib
        //   { method old {} {} ; renamemethod old sib ; method sib {} {} }
        //                                            -> info class methods -> sib
        // so a name deleted *before* the move is free, and one declared
        // *after* it never collides with it.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { method old {} { return 1 }\n\
             method sib {} { return 2 }\n\
             deletemethod sib\n\
             renamemethod old new }",
            "tcl9.0",
        );
        assert_eq!(
            r.all_classes["::C"].renamed_members[0].abort_if_renamed_to("sib"),
            None,
            "a name deleted before the move is free",
        );
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::D { method old {} { return 1 }\n\
             renamemethod old new\n\
             method sib {} { return 2 } }",
            "tcl9.0",
        );
        assert_eq!(
            r.all_classes["::D"].renamed_members[0].abort_if_renamed_to("sib"),
            None,
            "a name declared after the move never collides with it",
        );
    }

    #[test]
    fn a_rejected_rename_records_no_move() {
        // A move real Tcl refuses never happened, so there is nothing for the
        // rename gate to reason about — only the W315 it already drew.
        for body in [
            "method a {} {}\nrenamemethod a a",
            "method a {} {}\nmethod b {} {}\nrenamemethod a b",
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(&format!("oo::class create ::C {{ {body} }}"), "tcl9.0");
            assert!(
                r.all_classes["::C"].renamed_members.is_empty(),
                "body: {body}: {:?}",
                r.all_classes["::C"].renamed_members,
            );
            assert!(has_w315(&r), "body: {body}");
        }
    }

    #[test]
    fn a_blocked_rename_keeps_both_members_navigable() {
        // Navigation-resilience pin for the aborting shapes. Real Tcl creates no
        // class at all, so there is no "right" answer to mirror — the useful one
        // keeps every declaration the author wrote reachable rather than letting
        // the wreckage take one with it.
        let src = "oo::class create ::C { method a {} { return a }\n\
                   method b {} { return b }\n\
                   renamemethod a b }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(c.methods.contains_key("a"), "source kept: {:?}", c.methods);
        let b = c.methods.get("b").expect("destination kept");
        assert!(
            src[b.body_span.start() as usize..b.body_span.end() as usize].contains("return b"),
            "the destination keeps its OWN body",
        );
    }

    #[test]
    fn w315_is_reported_once_per_offending_word() {
        // A class extended by several `oo::define` blocks drains its aborts
        // after each block, so an earlier block's report is never re-emitted
        // when a later one runs.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create ::C { method kept {} { return k } }\n\
             oo::define ::C { deletemethod ghost }\n\
             oo::define ::C { method extra {} { return e } }\n\
             oo::define ::C { deletemethod other }",
            "tcl9.0",
        );
        assert_eq!(
            w315_messages(&r),
            vec![
                "this class definition cannot run: method \"ghost\" does not exist".to_string(),
                "this class definition cannot run: method \"other\" does not exist".to_string(),
            ],
        );
    }

    #[test]
    fn w315_points_at_the_offending_word() {
        // The span is the argument word, not the whole statement — a squiggle
        // under `ghost` / the destination name, which is what the reader needs.
        let src = "oo::class create ::C { deletemethod ghost }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == tcl_core_types::DiagCode::W315)
            .expect("W315");
        assert_eq!(
            &src[d.span.start() as usize..d.span.end() as usize],
            "ghost"
        );
    }

    #[test]
    fn a_stub_retraction_of_an_undeclared_member_leaves_a_tombstone() {
        // TP for the cross-document channel (issue #1101 review). This document
        // is the `oo::define ::C { deletemethod m }` half of a two-file program:
        // it has no local `m` to remove, so the removal has to travel as a
        // tombstone or the workspace keeps advertising a method that sourcing
        // this file deletes. The side travels with it — a `self`-scoped
        // retraction tombstones the class-object side.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::define ::C { deletemethod m }\n\
             oo::define ::D { self deletemethod cm }",
            "tcl9.0",
        );
        assert_eq!(
            r.all_classes["::C"].retracted_members,
            vec![MemberRetractionRecord::deletion(
                "m".to_string(),
                MemberSide::Instance
            )],
        );
        assert_eq!(
            r.all_classes["::D"].retracted_members,
            vec![MemberRetractionRecord::deletion(
                "cm".to_string(),
                MemberSide::ClassObject
            )],
        );
    }

    #[test]
    fn a_stub_renamemethod_tombstones_only_the_source_name() {
        // TN for the tombstone's shape: `renamemethod old new` removes `old` and
        // *creates* `new` (oracle: `info class methods ::I3` -> new, and
        // `[::I3 new] new` answers), so tombstoning `new` would suppress a live
        // member another document legitimately declares. Which arguments a
        // retracting word removes is registry data
        // ([`tcl_registry::definer::MemberRetraction::FirstArgument`]), not a
        // keyword match here.
        let mut a = Analyser::new();
        let r = a.analyse("oo::define ::C { renamemethod old new }", "tcl9.0");
        assert_eq!(
            r.all_classes["::C"]
                .retracted_members
                .iter()
                .map(|r| (r.member.clone(), r.side))
                .collect::<Vec<_>>(),
            vec![("old".to_string(), MemberSide::Instance)],
        );
        // …and the arrival travels with it (issue #1167): the stub has no
        // `MethodDef` to move, so the destination name is what lets the
        // workspace join re-key the defining file's record.
        assert_eq!(
            r.all_classes["::C"].retracted_members[0].arrival.as_deref(),
            Some("new"),
        );
    }

    #[test]
    fn visibility_sets_keep_only_the_last_writer() {
        // Issue #1101 review finding 3. `exports` / `unexports` each record the
        // *last* explicit writer for a name, and the cross-file consumer
        // (`workspace_index::method_dispatch_chain`) reads any `exports` entry
        // as decisive — so a name must never sit in both. Oracle, identical on
        // tclsh 9.0.4 and 8.6.14:
        //   oo::class create L1 { method m {} {…}; export m; unexport m }
        //   info class methods ::L1  ->            ;  [L1 new] m -> unknown method
        //   oo::class create L2 { method m {} {…}; unexport m; export m }
        //   info class methods ::L2  -> m          ;  [L2 new] m -> 1
        // Every spelling that writes the sets is covered: unwrapped (which had
        // the same double-entry bug before this change) and `private`-scoped.
        for (body, last, want_vis) in [
            ("export m\nunexport m", "unexports", "unexported"),
            ("unexport m\nexport m", "exports", "public"),
            (
                "private export m\nprivate unexport m",
                "unexports",
                "unexported",
            ),
            ("private unexport m\nprivate export m", "exports", "public"),
            (
                "private { export m }\nprivate { unexport m }",
                "unexports",
                "unexported",
            ),
        ] {
            let src = format!("oo::class create ::C {{ method m {{}} {{ return 1 }}\n{body} }}");
            let mut a = Analyser::new();
            let r = a.analyse(&src, "tcl9.0");
            let c = r.all_classes.get("::C").expect("::C recorded");
            let (winner, loser) = if last == "exports" {
                (&c.exports, &c.unexports)
            } else {
                (&c.unexports, &c.exports)
            };
            assert!(winner.contains("m"), "{body}: {last} must hold `m`");
            assert!(
                !loser.contains("m"),
                "{body}: the superseded set still holds `m` — cross-file dispatch \
                 reads any `exports` entry as decisive",
            );
            assert_eq!(c.methods["m"].visibility, want_vis, "{body}");
        }
    }

    #[test]
    fn a_self_scoped_visibility_word_never_touches_the_instance_sets() {
        // TN for the same sets: they are the *instance*-side record, so a
        // class-object-side flip must stay out of them entirely (in either
        // direction) — otherwise it would re-state an unrelated instance
        // method's export bit cross-file.
        let src = "oo::class create ::C {\n\
                   method m {} { return 1 }\n\
                   self { method m {} { return 2 }\n\
                   unexport m\n\
                   export m\n\
                   }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(c.exports.is_empty(), "{:?}", c.exports);
        assert!(c.unexports.is_empty(), "{:?}", c.unexports);
        assert_eq!(c.methods["m"].visibility, "public");
        assert_eq!(c.class_methods["m"].visibility, "public");
    }

    #[test]
    fn unwrapped_deletemethod_does_not_touch_the_class_side() {
        // TN. The unwrapped word is instance-scoped; a class-object-side member
        // of the same name survives it. (Real Tcl goes further and makes the
        // cross-side form a hard definition-aborting error — `oo::define ::I5
        // { deletemethod cm }` over a `self`-only `cm` raises `method cm does
        // not exist` and leaves `info object methods ::I5` -> cm — so a
        // document that reaches this shape never ran; keeping the class-side
        // entry is the abstaining answer either way.)
        let src = "oo::class create ::C {\n\
                   self { method cm {} { return c } }\n\
                   }\n\
                   oo::define ::C { deletemethod cm }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        let c = r.all_classes.get("::C").expect("::C recorded");
        assert!(c.class_methods.contains_key("cm"));
    }

    #[test]
    fn unwrapped_non_destructive_members_do_not_retract() {
        // TN for the registry flag: `export` / `unexport` / `filter` name a
        // method without removing it, so the unwrapped arm must leave them be.
        for member in ["export", "unexport", "filter"] {
            let src =
                format!("oo::class create ::C {{ method m {{}} {{ return 1 }}\n{member} m }}");
            let mut a = Analyser::new();
            let r = a.analyse(&src, "tcl9.0");
            assert!(
                r.all_classes["::C"].methods.contains_key("m"),
                "`{member}` must not retract",
            );
        }
    }

    #[test]
    fn an_unwrapped_retracted_member_can_be_redeclared_by_a_later_body() {
        // Source-order guard, unwrapped half. Retraction happens where the body
        // is walked, so a later `oo::define` redeclaration wins rather than the
        // name being erased document-wide.
        let src = "oo::class create ::C { method m {} { return 1 } }\n\
                   oo::define ::C { deletemethod m }\n\
                   oo::define ::C { method m {} { return 2 } }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        assert!(r.all_classes["::C"].methods.contains_key("m"));
    }

    #[test]
    fn objdefine_deletemethod_drops_the_per_object_member() {
        // The per-object table is the same instance-side routing. Oracle:
        //   oo::objdefine $o { method im {} {…}; deletemethod im }
        //   info object methods $o  ->            (empty)
        // `oo::objdefine` records into a throwaway `ClassDef` that never
        // reaches `all_classes`, so this asserts the walk simply does not
        // panic or resurrect the name anywhere.
        let src = "oo::class create ::C { method m {} { return m } }\n\
                   set o [::C new]\n\
                   oo::objdefine $o { method im {} { return i }\n\
                   deletemethod im }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        assert!(!r.all_classes.contains_key("::@objdefine@::o"));
        assert!(r.all_classes["::C"].methods.contains_key("m"));
    }

    // snit (tcllib) type / widget support.  Verified against real
    // `tclsh8.6` + tcllib.

    #[test]
    fn snit_type_recorded_as_class_with_members() {
        let src = "snit::type ::foo::Bar {\n\
                   variable v1\n\
                   typevariable tv1\n\
                   method m1 {a b} { return [expr {$a+$b+$v1}] }\n\
                   typemethod tm1 {} { return $tv1 }\n\
                   constructor {args} { set v1 0 }\n\
                   destructor { unset v1 }\n\
                   typeconstructor { set tv1 0 }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let c = r.all_classes.get("::foo::Bar").expect("Bar class recorded");
        assert_eq!(c.metaclass, "snit::type");
        assert!(c.methods.contains_key("m1"));
        assert!(c.class_methods.contains_key("tm1"));
        assert!(c.class_methods.contains_key("<typeconstructor>"));
        assert_eq!(c.constructors.len(), 1);
        assert!(c.destructor.is_some());
        assert_eq!(c.variables, vec!["v1".to_string(), "tv1".to_string()]);
    }

    #[test]
    fn snit_widget_keeps_win_and_hull_vars() {
        // A snit::widget injects `win` and `hull` instance variables — both
        // are recorded (only the four implicit scalars are filtered).
        let src = "snit::widget Dial {\n\
                   variable state\n\
                   method draw {} { return $win }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let c = r.all_classes.get("::Dial").expect("Dial recorded");
        assert_eq!(c.metaclass, "snit::widget");
        assert_eq!(
            c.variables,
            vec!["win".to_string(), "hull".to_string(), "state".to_string()]
        );
    }

    #[test]
    fn snit_method_body_suppresses_self_dispatch_and_implicit_vars() {
        // Inside a snit method, `$self`/`$component` dispatch and reads of
        // instance variables must not false-fire W307 (non-literal command),
        // W210 (read-before-set), or W211/W214 (unused).
        let src = "snit::widget mywidget {\n\
                   variable helper\n\
                   component inner\n\
                   method draw {} {\n\
                       $self configure -bg white\n\
                       $inner render\n\
                       $helper compute\n\
                       return $win\n\
                   }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        for code in ["W210", "W211", "W214", "W307", "W308"] {
            assert!(
                !r.diagnostics.iter().any(|d| d.code.as_str() == code),
                "{code} must not fire in a snit method body: {:?}",
                r.diagnostics
            );
        }
    }

    #[test]
    fn snit_widgetadaptor_and_qualified_definer() {
        let mut a = Analyser::new();
        let r = a.analyse("::snit::widgetadaptor Foo { method m {} {} }", "tcl8.6");
        let c = r.all_classes.get("::Foo").expect("Foo recorded");
        assert_eq!(c.metaclass, "::snit::widgetadaptor");
        assert!(c.methods.contains_key("m"));
    }

    #[test]
    fn non_snit_command_is_not_a_class() {
        // A plain command that merely starts with `snit` is not a definer.
        let mut a = Analyser::new();
        let r = a.analyse("snitch foo { bar }", "tcl8.6");
        assert!(r.all_classes.is_empty());
    }

    // [incr Tcl] `itcl::class` — recorded as a `ClassDef` with method scopes,
    // access modifiers unwrapped, `inherit` → superclasses, `this` implicit.

    #[test]
    fn itcl_class_recorded_with_members() {
        let src = "itcl::class ::widgets::Dial {\n\
                   inherit Base\n\
                   variable value 0\n\
                   common count 0\n\
                   constructor {args} { set value 0 }\n\
                   destructor { unset value }\n\
                   method spin {delta} { incr value $delta }\n\
                   proc reset {} { set count 0 }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let c = r
            .all_classes
            .get("::widgets::Dial")
            .expect("Dial class recorded");
        assert_eq!(c.metaclass, "itcl::class");
        assert_eq!(c.superclasses, vec!["Base".to_string()]);
        assert!(c.methods.contains_key("spin"));
        assert!(
            c.class_methods.contains_key("reset"),
            "proc is a class method"
        );
        assert_eq!(c.constructors.len(), 1);
        assert!(c.destructor.is_some());
        // `variable` + `common` declarations are recorded (implicit `this` is
        // filtered).
        assert_eq!(c.variables, vec!["value".to_string(), "count".to_string()]);
    }

    #[test]
    fn itcl_access_modifiers_unwrap_with_visibility() {
        let src = "itcl::class C {\n\
                   public method api {} {}\n\
                   private method helper {} {}\n\
                   protected variable state 0\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let c = r.all_classes.get("::C").expect("C recorded");
        assert_eq!(c.methods["api"].visibility, "public");
        assert_eq!(c.methods["helper"].visibility, "private");
        // The wrapped `protected variable state` is still a declared variable.
        assert!(c.variables.contains(&"state".to_string()));
    }

    #[test]
    fn itcl_method_body_suppresses_this_and_instance_vars() {
        // Inside an itcl method, `$this` dispatch and reads of instance /
        // `common` variables must not false-fire W210/W211/W307.
        let src = "itcl::class C {\n\
                   variable handler\n\
                   common registry\n\
                   method run {} {\n\
                       $this configure\n\
                       $handler process\n\
                       return $registry\n\
                   }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        for code in ["W210", "W211", "W214", "W307", "W308"] {
            assert!(
                !r.diagnostics.iter().any(|d| d.code.as_str() == code),
                "{code} must not fire in an itcl method body: {:?}",
                r.diagnostics
            );
        }
    }

    #[test]
    fn non_itcl_command_is_not_a_class() {
        // A command that merely starts with `itcl` is not a definer.
        let mut a = Analyser::new();
        let r = a.analyse("itclish foo { bar }", "tcl8.6");
        assert!(r.all_classes.is_empty());
    }

    // OO body-walks: `initialise` body, `property -get/-set` accessor bodies,
    // and the `new` / `createWithNamespace` class-command variants.

    #[test]
    fn oo_initialise_body_is_walked() {
        // A `variable` read inside `initialise { … }` must not false-fire
        // W210 read-before-set — the class-level init body is walked with the
        // class's instance variables visible.
        let src = "oo::class create Foo {\n\
                   variable cache\n\
                   initialise { set cache [dict create] }\n\
                   method get {k} { return [dict get $cache $k] }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| matches!(d.code.as_str(), "W210" | "W211")),
            "initialise body should be walked cleanly: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn oo_property_accessor_bodies_are_walked() {
        // `-get`/`-set` accessor bodies are walked with the instance variable
        // `val` and the implicit `value` visible — no false W210 / W307.
        // `property` is a 9.0 member, so the vector runs under a 9.0 dialect.
        let src = "oo::configurable create Bar {\n\
                   variable val\n\
                   property color -get { return $val } -set { set val $value }\n\
                   }";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        assert!(r.all_classes.contains_key("::Bar"));
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| matches!(d.code.as_str(), "W210" | "W307")),
            "property accessor bodies should be walked cleanly: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn oo_class_create_with_namespace_is_recognised() {
        // The `createWithNamespace` class-command variant introduces a class.
        let mut a = Analyser::new();
        let r = a.analyse("oo::class createWithNamespace MyCls ::ns { }", "tcl8.6");
        assert!(
            r.all_classes.keys().any(|k| k.contains("MyCls")),
            "createWithNamespace should record a class: {:?}",
            r.all_classes.keys().collect::<Vec<_>>()
        );
    }
}
