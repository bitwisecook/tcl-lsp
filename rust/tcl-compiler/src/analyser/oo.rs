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
//! - ``superclass <names>`` — assigns ``ClassDef::superclasses``.
//! - ``mixin ?-append? <names>`` — assigns ``ClassDef::mixins``
//!   (the ``-append`` flag is consumed and ignored —
//!   class-hierarchy state machines belong to the workspace
//!   index, not the per-file analyser).
//! - ``method NAME PARAMS BODY`` — adds to ``ClassDef::methods``.
//! - ``classmethod NAME PARAMS BODY`` — adds to
//!   ``ClassDef::class_methods``.
//! - ``constructor PARAMS BODY`` — appends a synthetic-named
//!   ``MethodDef`` to ``ClassDef::constructors``.
//! - ``destructor BODY`` — sets ``ClassDef::destructor``.
//! - ``forward NAME ?TARGET ARGS?`` — adds to ``methods`` with
//!   ``kind = "forward"``.
//! - ``variable <names>`` — assigns ``ClassDef::variables``.
//! - ``filter <names>`` — assigns ``ClassDef::filters``.
//! - ``export <names>`` / ``unexport <names>`` — extends the
//!   matching ``HashSet`` field.
//! - ``property NAME ?-get BODY? ?-set BODY? ?-kind K?`` —
//!   extracts a [`super::types::PropertyDef`] per name.
//! - ``initialise`` / ``initialize`` — recognised; the body is
//!   walked in the enclosing scope for variable tracking.

use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::{DefinitionBodyGrammar, MemberRefKind, MemberSpec};

use super::scope::scope_at_mut;
use super::state::Analyser;
use super::types::{ClassDef, MethodDef, PropertyDef, Scope, ScopeKind, UnknownProcInfo};
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

/// The snit definer being parsed: its member grammar plus whether it is a
/// `widget` / `widgetadaptor` (which injects the extra `win` / `hull` instance
/// variables).  Bundled to keep [`Analyser::parse_snit_definition_body`] under
/// the argument limit.
struct SnitDefiner {
    grammar: &'static DefinitionBodyGrammar,
    is_widget: bool,
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
        // `initialise`/`initialize { body }` — a class-level script walked in
        // the *enclosing* scope (not a method scope).
        let mut init_bodies: Vec<(String, Token)> = Vec::new();
        for cmd in &cmds {
            if cmd.is_partial || cmd.argv.is_empty() {
                continue;
            }
            // A member keyword gated to a newer core — `classmethod`,
            // `private`, `initialise`/`initialize`, `definitionnamespace`,
            // and `property` are all 9.0+ — does not exist in this dialect's
            // definition grammar.  Flag it with the same disabled-in-dialect
            // diagnostic a command draws, then carry on recording it (see
            // the emitter's doc for why reporting must not erase).
            if let (Some(subcmd), Some(tok)) = (cmd.texts.first(), cmd.argv.first()) {
                self.emit_w002_oo_member_disabled(grammar, subcmd, *tok, definer_disabled);
            }
            apply_oo_subcommand(grammar, &cmd.texts, &cmd.argv, class_def);
            // A member argument that *names* a command — the class a
            // `superclass`/`mixin` extends, the command a `forward` delegates to
            // — is a first-class command reference.  Record it so references /
            // go-to-definition / rename / call-hierarchy reach it the same as a
            // direct call.  Which arguments name commands is registry data (the
            // member grammar's `all_args_ref` / `ArgRole::CommandName`), never a
            // member keyword the walker knows by name.
            self.record_member_command_references(grammar, &cmd.texts, &cmd.argv, scope_path);
            if let Some(mb) = collect_method_body(grammar, &cmd.texts, &cmd.argv) {
                method_bodies.push(mb);
            }
            match cmd.texts.first().map(String::as_str) {
                Some("property") => {
                    collect_property_accessor_bodies(&cmd.texts, &cmd.argv, &mut accessor_bodies);
                }
                Some(kw @ ("initialise" | "initialize")) => {
                    // A class-level init script: unlike a method, its body (the
                    // member's grammar `Body` word) is walked in the *enclosing*
                    // scope, so it is collected here rather than by
                    // `collect_method_body`.
                    if let Some(body_idx) = grammar
                        .member(kw)
                        .and_then(|m| m.indices_for(ArgRole::Body).next())
                        .map(|i| i + 1)
                        && let (Some(body), Some(tok)) =
                            (cmd.texts.get(body_idx), cmd.argv.get(body_idx).copied())
                        && tok.kind == TokenType::Str
                    {
                        init_bodies.push((body.clone(), tok));
                    }
                }
                _ => {}
            }
        }
        // A bareword-callable-from-this-class fact, gathered before Phase 2
        // starts (so it's already populated by the time member-lookup
        // consumers — go-to-definition / hover — read `class_def`, and
        // regardless of *which* method body called `link`).
        self.collect_oo_links(&method_bodies, class_def);

        // Phase 2: walk each method / accessor body in its own `Method` scope
        // with the formal parameters and the class's instance variables
        // pre-bound; the `initialise` body walks in the enclosing scope.
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
        for (body, tok) in init_bodies {
            self.analyse_body(&body, tok, scope_path);
        }
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
                // `link` is deliberately *not* a method-dispatch keyword and
                // carries none of the `TCLOO_*` traits (issue #1050): it
                // *creates* per-object bareword commands rather than
                // dispatching one, so the barewords it installs are per-class
                // data, not language keywords. Recognising the `link` call
                // itself is a spec-name match with no behavioural fact behind
                // it to query — the migration to a registry-modelled member
                // grammar for `oo::Helpers` is tracked by issue #1026.
                if cmd.is_partial || cmd.texts.first().map(String::as_str) != Some("link") {
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
        let Some(method_idx) = ({
            scope_at_mut(&mut self.result.global_scope, scope_path).map(|parent| {
                let mut child = Scope::new(ScopeKind::Method, method_qn.clone());
                child.body_span = Some(mb.body_tok.span);
                child.oo_global_resolution = oo_global_resolution;
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
            let namespace = self.namespace_from_scope_path(scope_path);
            let safe_interp_ctx = self.safe_interp_ctx_snapshot();
            self.deferred_bodies.push(super::per_item::DeferredBody {
                body_text: mb.body_text.clone(),
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
        let ns_prefix = self.namespace_from_scope_path(scope_path);
        // Constructed key in, construction-inverse tail out (#934): a colon
        // trim or `rsplit("::")` would collapse a lone-colon name.
        let qualified = super::handlers::qualify(&ns_prefix, raw_name);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = arg_tokens[0].span;
        // **W314** — the class name has no absolute written form (#934).
        self.emit_w314_no_absolute_name(raw_name, name_span);
        let body_tok = arg_tokens[1];
        let is_widget = cmd_name.ends_with("widget") || cmd_name.ends_with("widgetadaptor");
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
            let definer = SnitDefiner { grammar, is_widget };
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
        // definer's registry grammar (`implicit_vars`); a widget adds `win` /
        // `hull` on top.
        let implicit_vars = grammar.implicit_vars;
        let mut instance_vars: Vec<String> =
            implicit_vars.iter().map(|s| (*s).to_string()).collect();
        if definer.is_widget {
            instance_vars.push("win".to_string());
            instance_vars.push("hull".to_string());
        }
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
        // read `ClassDef::variables`.  The grammar's implicit scalars
        // (`self`/`selfns`/`type`/`options`) and the type-implicit `type` are
        // filtered; a widget's injected `win`/`hull` are kept.
        class_def.variables = instance_vars
            .iter()
            .filter(|v| !implicit_vars.contains(&v.as_str()))
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
        let ns_prefix = self.namespace_from_scope_path(scope_path);
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
    let (keyword, texts, argv, _modifier) = unwrap_wrapper_member(grammar, texts, argv)?;
    if !matches!(
        keyword,
        "method" | "classmethod" | "constructor" | "destructor"
    ) {
        return None;
    }
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

/// `self method NAME ARGS BODY` / `self classmethod NAME ARGS BODY`
/// (issue #923 idx 120) — `TclOO`'s own spelling for a class-level method,
/// the stock-library counterpart to `ooutil`'s `classmethod` keyword (both
/// end up dispatched through the class's own bound command). Either inner
/// spelling records into `class_methods`, tagged `is_self_method: true` so
/// the class-command MRO walk in `tcl-lsp-core` knows NOT to treat it as
/// inherited the way an `ooutil`-style `classmethod` is (real tclsh: a
/// subclass with no override does not gain a `self method` at all).
///
/// Scoped to the prefix form only; `self { method NAME ARGS BODY; … }`'s
/// block form is a documented, symmetric (`private { … }` shares the same
/// gap) follow-up, not fixed here.
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

/// Set the recorded visibility of each named member (instance or
/// class-side) to `visibility` — the `export` / `unexport` member effect.
/// A name with no recorded member yet is skipped: a later `method`
/// definition re-applies the name default anyway (tclsh 9.0.4-pinned),
/// and an export-only stub has no declaration to navigate to.
fn set_member_visibility(class_def: &mut ClassDef, names: &[String], visibility: &str) {
    for name in names {
        if let Some(md) = class_def.methods.get_mut(name) {
            md.visibility = visibility.to_string();
        }
        if let Some(md) = class_def.class_methods.get_mut(name) {
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

fn apply_oo_subcommand(
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

    match subcmd {
        // `superclasses` / `mixins` feed the class-hierarchy graph (inherited
        // methods, MRO).  The navigable *references* to those base classes are
        // recorded separately as `command_invocations` by
        // `record_member_command_references`, so no per-name span is kept here.
        "superclass" => {
            class_def.superclasses = sub_args.to_vec();
        }
        "mixin" => {
            // Skip ``-append`` and similar flags.
            class_def.mixins = sub_args
                .iter()
                .filter(|a| !a.starts_with('-'))
                .cloned()
                .collect();
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
            class_def.variables.extend(sub_args.iter().cloned());
        }
        "filter" => {
            class_def.filters = sub_args.to_vec();
        }
        "export" => {
            class_def.exports.extend(sub_args.iter().cloned());
            // Flip already-recorded members to exported (last explicit
            // state wins; a later re-`method` resets to the default —
            // tclsh 9.0.4-pinned).
            set_member_visibility(class_def, sub_args, "public");
        }
        "unexport" => {
            class_def.unexports.extend(sub_args.iter().cloned());
            set_member_visibility(class_def, sub_args, "unexported");
        }
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
