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

//! Lower Tcl source to structured analysis IR.
//!
//! Translates a flat token stream (via the segmenter) into the tree of
//! IR nodes defined in [`crate::ir`]. Each Tcl command is pattern-matched by
//! name and converted to a typed IR statement.

use std::collections::{HashMap, HashSet};

use tcl_lexer::TokenType;
use tcl_registry::definer::{
    DefinitionBodyGrammar, InitTiming, MemberCurrentNamespace, MemberEffect, MemberReceiver,
    MemberRow,
};
use tcl_registry::events::{IrulesCommandPlacement, IrulesExecutionContext};
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::{ArgRole, CommandRegistry, InvocationArguments, InvocationWord};

use crate::alias::CommandAliasMap;
use crate::command_binding::{
    RetainedSourceBodyInventory, SourceAnalysisOptions, SourceCommandBindings, TrustedPackageLoader,
};
use crate::expr_parser::parse_expr_for_profile_with_lexer_config;
use crate::ir::{
    CommandBindingSite, CommandTokens, ForeachIterator, MethodDef, MethodKind, Module, Procedure,
    Script, Statement,
};
use crate::lowering_hooks::{
    ArgTokenKind, LoweringCommand, ResolvedLowering, extract_proved_expr_arg,
};
use crate::segmenter::SegmentedCommand;
use tcl_dialect::model::surface_admits;
use tcl_syntax::word_rules::WordValueRules;

mod authored_source_bodies;
#[doc(hidden)]
pub mod execution_regions;
pub(crate) mod hooks;
mod hosted_source_bodies;
mod installed_bodies;
#[doc(hidden)]
pub mod source_body_syntax;
pub use source_body_syntax::{
    SourceBodySyntaxCache, SourceBodySyntaxRequest, SourceBodySyntaxTemplate,
};
#[cfg(test)]
mod native_body_context_tests;
#[cfg(test)]
mod original_body_word_geometry_tests;
#[cfg(test)]
mod passive_metadata_tests;
pub use execution_regions::{declared_surface_package_loader, stock_body_provider_loader};
// `pub(crate)` for one item: `structured::parse_switch_options`, which the
// dispatch chain's lowering asks where a `switch`'s options end, and whether
// `--` closed them, rather than carrying a second copy of that rule.
pub(crate) mod structured;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoweringMetadataOrigin {
    Standalone,
    Supplied,
}

/// Stand-in `Script` for a body past [`MAX_LOWER_NEST_DEPTH`]: a single
/// [`Statement::Barrier`] spanning `[base_offset, base_offset + len)`, so
/// downstream passes treat the unanalysed region as having unknown effects
/// (matching how a dynamic `eval`/`uplevel` body is already modelled)
/// rather than as empty/dead code.
fn over_depth_script(base_offset: u32, len: usize) -> Script {
    let end = base_offset.saturating_add(u32::try_from(len).unwrap_or(u32::MAX));
    Script::from_statements(vec![Statement::Barrier {
        span: tcl_lexer::Span::new(base_offset, end),
        reason: "nesting depth exceeds analysis limit".to_owned(),
        command: String::new(),
        canonical_command: None,
        args: Vec::new(),
        tokens: None,
    }])
}

fn unavailable_original_body_script(base_offset: u32, len: usize, reason: &str) -> Script {
    let end = base_offset.saturating_add(u32::try_from(len).unwrap_or(u32::MAX));
    Script::from_statements(vec![Statement::Barrier {
        span: tcl_lexer::Span::new(base_offset, end),
        reason: reason.to_owned(),
        command: String::new(),
        canonical_command: None,
        args: Vec::new(),
        tokens: None,
    }])
}

/// Map token kind to the simplified `ArgTokenKind` used by lowering hooks.
fn arg_token_kind(kind: TokenType) -> ArgTokenKind {
    match kind {
        TokenType::Str => ArgTokenKind::Str,
        TokenType::Esc => ArgTokenKind::Esc,
        TokenType::Cmd => ArgTokenKind::Cmd,
        TokenType::ExprSugar => ArgTokenKind::ExprSugar,
        TokenType::Var => ArgTokenKind::Var,
        _ => ArgTokenKind::Other,
    }
}

/// Join a parent namespace with a child name.
fn join_namespace(parent: &str, child: &str) -> String {
    tcl_syntax::naming::qualify(parent, child)
}

/// Recursive dispatch retains only the selected contract, never the registry's
/// full materialization frame while lowering another body.
struct StructuredHookSelection<'a> {
    hook: LoweringHookId,
    requirement: crate::registry_invocation::SelectedLoweringBindingIdentity<'a>,
    error_context: Option<tcl_registry::InlineBodyErrorContext>,
}

#[inline(never)]
fn resolved_top_level_priority(
    registry: &CommandRegistry,
    command: &str,
    args: &[Option<String>],
) -> Option<u16> {
    let references = args
        .iter()
        .map(|argument| argument.as_deref())
        .collect::<Option<Vec<_>>>()?;
    match registry.irules_top_level_effect(command, &references)? {
        tcl_registry::events::IrulesTopLevelDeclaration::Priority { value } => Some(value),
        _ => None,
    }
}

/// One statically-extractable definer invocation, classified from the
/// command's **registry spec** — never a spelling match: the
/// definer's member grammar plus which argv words carry the definition
/// target and the static braced body.
///
/// Covers every command whose spec hangs a
/// [`tcl_registry::definer::DefinitionBodyGrammar`] off
/// `CommandSpec::definition_body`: the `TclOO` metaclasses (`oo::class`,
/// `oo::configurable`, `oo::abstract`, `oo::singleton`), the
/// `oo::define` / `oo::objdefine` script forms, snit's `type` / `widget` /
/// `widgetadaptor`, and itcl's `itcl::class`.  A new definer added to the
/// registry is picked up here with no lowering change.
#[derive(Clone, Copy)]
struct DefinerCall {
    definition_scope: Option<crate::ir::ProcedureId>,
    descriptor: tcl_registry::CanonicalCommandDescriptorKey<'static>,
    /// The definer's member grammar (registry data).
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    /// Full-argv index of the class / type / object-handle word.
    name_idx: usize,
    /// Full-argv index of the static braced definition body.
    body_idx: usize,
    /// `oo::objdefine`: the target is an *object handle*, so members and
    /// `variable` declarations are **per-object** state: after
    /// `oo::objdefine $k { variable z }`, `my variable z; info exists z`
    /// yields `0` — per-object, not per-class.
    /// They home under a synthetic `::@objdefine@::…` class name and never
    /// join the real class's cross-block `class_instance_vars` union.
    per_object: bool,
}

impl Lowerer<'_> {
    /// Whether this concrete invocation is declared to initialise an unset
    /// target under the lowerer's active dialect profile.
    ///
    /// This is registry-driven: command form and Tcl release decide the
    /// result, not a consumer-side command-name or read-modify-write rule.
    fn safe_on_uninit(&self, command: &str, args: &[String]) -> bool {
        let Some(context) = self
            .invocation_metadata_context()
            .filter(|context| context.matches_registry(self.registry))
        else {
            return false;
        };
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let dialect = Some(context.context().authoring_query());
        self.registry
            .resolve_invocation(command, &arg_refs, dialect)
            .is_some_and(|resolved| {
                resolved
                    .semantics
                    .safe_on_uninit
                    .is_none_or(|allowed| surface_admits(allowed, dialect.as_ref()))
            })
    }

    /// Read an exact definition layout from the selected original invocation.
    /// These source units do not establish class presence or entered execution.
    fn classify_definer_call(&self, tokens: &CommandTokens) -> Option<DefinerCall> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context = self.invocation_metadata_context()?;
        if let Some(binding) = tokens.source_binding.as_ref()
            && binding.has_original_logical_source_name_role_owner()
            && !binding.has_original_source_layout_owner()
        {
            let selected = crate::registry_invocation::original_logical_source_definition_layout(
                self.registry,
                context.source_analysis_input()?,
                tokens,
            )?;
            return Some(DefinerCall {
                definition_scope: None,
                descriptor: selected.descriptor,
                grammar: selected.grammar,
                name_idx: selected.name_word,
                body_idx: selected.body_word,
                per_object: selected.per_object,
            });
        }
        let invocation = self.logical_structured_invocation(tokens)?;
        let mut bodies = invocation
            .written_roles()
            .into_iter()
            .filter_map(|(index, role)| (role == ArgRole::Body).then_some(index + 1));
        let body_idx = bodies.next()?;
        if bodies.next().is_some()
            || !word_is_static_braced(&tokens.argv_kinds, &tokens.single_token_word, body_idx)
        {
            return None;
        }
        let realm = tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
            .unwrap_or(self.invocation_realm);
        invocation.with_metadata_schema(self.registry, context, realm, |selected| {
            let grammar = selected.authored_source_definition_body_grammar()?;
            if !grammar.family.manufactures_runtime_commands()
                || !selected
                    .authored_source_script_arguments()?
                    .contains(&(body_idx - 1))
            {
                return None;
            }
            let name_arg = selected
                .state_transitions()
                .facts()
                .iter()
                .find_map(|fact| match &fact.transition {
                    tcl_registry::StateTransition::ObjectDispatch(
                        tcl_registry::ObjectDispatchTransition::Configure { target, .. },
                    ) => target.argument_index(),
                    _ => None,
                })
                .or_else(|| {
                    selected
                        .authored_source_command_publication()
                        .map(|name| name.argument)
                })?;
            let name_idx = invocation.written_argument(name_arg)?.checked_add(1)?;
            (name_idx < tokens.argv.len()).then_some(DefinerCall {
                definition_scope: None,
                descriptor: selected.descriptor_key(),
                grammar,
                name_idx,
                body_idx,
                per_object: selected.authored_source_descriptors().command.analyser_hook
                    == Some(tcl_registry::hooks::AnalyserHookId::OoObjdefine),
            })
        })
    }

    /// Retain possible definition roots from original candidate identities.
    /// Unknown availability or lookup withdraws completeness, never supplies
    /// a nominal grammar or a class declaration from the written command name.
    fn definer_invocation_may_supply_body(&self, tokens: &CommandTokens) -> bool {
        let Some(context) = self
            .invocation_metadata_context()
            .filter(|context| context.matches_registry(self.registry))
        else {
            return true;
        };
        let Some(advice) = crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            self.registry, Some(context), tokens,
        ) else { return true; };
        if advice.unknown_residual || advice.candidates.is_empty() {
            return true;
        }
        let realm = tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
            .unwrap_or(self.invocation_realm);
        advice.candidates.iter().any(|candidate| {
            let Some(spec) =
                context
                    .context()
                    .resolve_spec_in_realm(self.registry, &candidate.command, realm)
            else {
                return true;
            };
            spec.definition_body.is_some_and(|grammar| {
                grammar.family.manufactures_runtime_commands()
                    && (!candidate.roles_complete
                        || candidate
                            .roles
                            .iter()
                            .any(|&(_, role)| role == ArgRole::Body))
            })
        })
    }

    /// A selected namespace wrapper with unchanged original source operands.
    fn is_namespace_eval_shape(&self, tokens: &CommandTokens) -> bool {
        tokens.argv.len() >= 4
            && word_is_static_braced(&tokens.argv_kinds, &tokens.single_token_word, 3)
            && self
                .logical_structured_invocation(tokens)
                .is_some_and(|invocation| {
                    invocation.lowering_hook() == Some(LoweringHookId::NamespaceEval)
                        && invocation.written_roles().contains(&(2, ArgRole::Body))
                })
    }

    fn source_definition_member(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        keyword: &str,
    ) -> Option<&'static tcl_registry::definer::MemberSpec> {
        let context = self
            .invocation_metadata_context()
            .filter(|context| context.matches_registry(self.registry))?;
        grammar.source_member_in(
            keyword.as_bytes(),
            Some(context.context().authoring_query()),
        )
    }

    /// Shared vocabulary and layout over original semantic source words.
    /// Dynamic control words cannot become literal optional member selectors.
    fn source_member_body_indices(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        keyword: &str,
        segment: &SegmentedCommand,
        base: usize,
    ) -> Option<Vec<usize>> {
        let context = self
            .invocation_metadata_context()
            .filter(|context| context.matches_registry(self.registry))?;
        let dialect = self.invocation_dialect?;
        let tokens = self.cmd_tokens(segment);
        let source = tokens.words().get(base..)?;
        let values: Vec<_> = source
            .iter()
            .map(|word| {
                crate::registry_invocation::effective_invocation_word(
                    word,
                    self.config.escapes,
                    dialect.word_values,
                )
            })
            .collect();
        let words: Vec<_> = source
            .iter()
            .zip(&values)
            .map(|(source, value)| {
                crate::registry_invocation::invocation_word_with_source(
                    source,
                    value,
                    self.config.escapes,
                )
            })
            .collect();
        grammar.source_member_body_indices_in(
            keyword.as_bytes(),
            tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect),
            Some(context.context().authoring_query()),
        )
    }

    fn member_invocation_supplies_body(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        segment: &SegmentedCommand,
    ) -> bool {
        self.source_member_body_indices(grammar, &segment.texts[0], segment, 1)
            .is_none_or(|indices| !indices.is_empty())
    }

    /// A definition created only when this method runs is not part of the
    /// post-pass's source-root walk. Detect it recursively and retain explicit
    /// module evidence rather than pretending its callable bodies do not exist.
    fn method_body_may_install_definition(&self, script: &Script) -> bool {
        fn walk(lowerer: &Lowerer<'_>, script: &Script, depth: u32) -> bool {
            if MAX_LOWER_NEST_DEPTH.exceeded(depth) {
                return true;
            }
            script.statements.iter().any(|stmt| {
                let installs = matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. })
                    && stmt
                        .tokens()
                        .is_none_or(|tokens| lowerer.definer_invocation_may_supply_body(tokens));
                installs
                    || crate::ir_helpers::nested_bodies(stmt)
                        .into_iter()
                        .any(|body| walk(lowerer, body, depth + 1))
            })
        }

        walk(self, script, 0)
    }
}

/// True iff word `idx` (full argv index) is a single braced-literal
/// (`Str`) token.
fn word_is_static_braced(kinds: &[TokenType], single: &[bool], idx: usize) -> bool {
    idx < kinds.len() && idx < single.len() && single[idx] && kinds[idx] == TokenType::Str
}

/// `SegmentedCommand` variant of [`word_is_static_braced`]: word
/// `idx` is a single braced-literal (`Str`) token.
fn seg_word_is_static_braced(seg: &SegmentedCommand, idx: usize) -> bool {
    seg.single_token_word.get(idx).copied().unwrap_or(false)
        && seg.argv.get(idx).is_some_and(|t| t.kind == TokenType::Str)
}

/// Word `idx` is a single substitution-free literal: a `{braced}` (`Str`) word
/// or a plain bareword (`Esc`) — exactly the body words C's `TclCompileIfCmd`
/// inlines. A word carrying `$var` / `[cmd]` substitution, or a multi-token
/// concatenation like `$x1$x2`, is not (it must be substituted then evaluated
/// as a script by the runtime `if` command).
fn seg_word_is_static_literal(seg: &SegmentedCommand, idx: usize) -> bool {
    seg.single_token_word.get(idx).copied().unwrap_or(false)
        && seg
            .argv
            .get(idx)
            .is_some_and(|t| matches!(t.kind, TokenType::Str | TokenType::Esc))
}

/// Word `idx` is C's `TCL_TOKEN_SIMPLE_WORD`: a [`seg_word_is_static_literal`]
/// word whose value is its text, so a bareword or quoted word with a
/// backslash is not (`\"a\"` is `"a"`). A condition or body that is not
/// simple is substituted and then evaluated by the runtime command.
fn seg_word_is_simple(seg: &SegmentedCommand, idx: usize) -> bool {
    seg_word_is_static_literal(seg, idx)
        && (seg.argv.get(idx).is_some_and(|t| t.kind == TokenType::Str)
            || seg.texts.get(idx).is_some_and(|text| !text.contains('\\')))
}

/// Whether word `idx` runs a `$var` / `[cmd]` substitution when it is
/// evaluated — so its value is run-time data rather than the text as written.
///
/// Weaker than the negation of [`seg_word_is_static_literal`], and
/// deliberately so: a **compound** literal word (`{body}x`, two literal tokens
/// welded together) is still spelled out in the source, so a body walk that
/// clamps it with [`crate::segmenter::body_text_in_region`] rebases honest
/// spans. `{*}$n` under an expansionless grammar looks the same
/// from the representative token alone — braced, multi-token — yet its second
/// fragment substitutes, so its value appears nowhere in the document.
///
/// Falls back to the representative token kind when the per-word fragments are
/// unavailable, matching the coarse view the other word gates here take.
fn seg_word_substitutes(seg: &SegmentedCommand, idx: usize) -> bool {
    match seg.word_fragments.get(idx) {
        Some(fragments) if !fragments.is_empty() => fragments
            .iter()
            .any(|fragment| matches!(fragment.token.kind, TokenType::Var | TokenType::Cmd)),
        _ => seg
            .argv
            .get(idx)
            .is_some_and(|tok| matches!(tok.kind, TokenType::Var | TokenType::Cmd)),
    }
}

/// Qualify a procedure name relative to a namespace.
fn qualify_proc_name(namespace: &str, proc_name: &str) -> String {
    tcl_syntax::naming::qualify(namespace, proc_name)
}

/// The namespace a `proc`'s **body** resolves names against: the namespace the
/// proc is *defined in* (the qualifier prefix of its own qualified name), not
/// the lexical namespace the `proc` command was written in.
///
/// The two agree for every proc whose name word carries no qualifier — the
/// overwhelmingly common case, including `proc outer {} { proc inner … }` at
/// top level — and diverge exactly when the name word is itself qualified, so
/// the definition site and the defining namespace are different namespaces.
///
/// Observed identically on tclsh 9.0.4 and 8.6.16 (`self class`-style
/// introspection is not involved; this is plain `info commands` after
/// running the outer proc):
///
/// ```tcl
/// namespace eval ::a {}
/// proc a::outer {} { proc helper {x} { return $x } ; return [namespace current] }
/// a::outer                        ;# -> ::a           (the body's frame namespace)
/// info commands ::helper          ;# -> {}            (NOT the lexical ::)
/// info commands ::a::helper       ;# -> ::a::helper
///
/// namespace eval ::b::c {}
/// proc b::c::o2 {} { proc h2 {y} { return $y } }
/// b::c::o2 ; info commands ::b::c::h2   ;# -> ::b::c::h2, and ::h2 is empty
///
/// # The *lexical* enclosing `namespace eval` loses to the name's own qualifier:
/// namespace eval ::e {}
/// namespace eval ::d { proc ::e::o3 {} { proc h3 {z} { return $z } } }
/// ::e::o3 ; info commands ::e::h3       ;# -> ::e::h3, ::d::h3 and ::h3 empty
///
/// # Nesting composes — each level re-homes to its own defining namespace:
/// namespace eval ::g {}
/// proc g::o7 {} { proc o7b {} { proc h7 {u} { return $u } } }
/// g::o7 ; ::g::o7b ; info commands ::g::h7   ;# -> ::g::h7, ::h7 empty
/// ```
///
/// An absolutely-written nested name (`proc ::abs {q} {…}`) is unaffected —
/// [`qualify_proc_name`] already short-circuits on the leading `::`.
///
/// `qualified` is always rooted (it comes from [`qualify_proc_name`], which
/// routes every result through `normalise_qualified_name`), so the holder is
/// never the empty "relative" marker [`tcl_syntax::naming::key_holder_and_tail`]
/// returns for a bare simple name; `lexical_fallback` covers that
/// impossible-by-construction case rather than silently producing `""`.
#[cfg(test)]
fn proc_body_namespace(qualified: &str, lexical_fallback: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qualified);
    if holder.is_empty() {
        lexical_fallback.to_string()
    } else {
        holder.to_string()
    }
}

/// Configured list names from a brace-collapsed compatibility word.
#[cfg(test)]
fn parse_var_list_names(list_text: &str, rules: WordValueRules) -> Option<Vec<String>> {
    rules.split_word_names(list_text)
}

/// Split a Tcl *formal parameter* list word into its parameter names.
///
/// This is the `proc` / `apply`-lambda / TclOO-method parameter word, a
/// different grammar from a loop variable list: it has two list levels, the
/// outer one holding a specifier per parameter and each specifier itself a
/// `name ?default?` list. So `{x {y 2} args}` is the three parameters `x`, `y`
/// (defaulting to `2`), and `args`; and `proc p {a\ b} {}` is the *single*
/// parameter `a` defaulting to `b` (tclsh 9.0: `info args p` → `a`), not the
/// two names a whitespace split would produce.
/// [`tcl_syntax::formal_params::parse_formal_parameters_in`] owns the selected
/// engine grammar and
/// is what the VM, the WASM runtime, and `signature_scan` all decode with.
///
/// Only the names reach the IR — [`Procedure::params`] is name-only, and
/// `params_raw` keeps the source text for consumers that need the defaults.
///
/// An unavailable grammar or a parameter list refused by that engine declines.
/// Jim keeps its own lenient list and name grammar; original link spelling
/// in a name-only header supplies no caller-link or ordinary-cell activation.
fn parse_formal_param_names(
    param_text: &str,
    rules: WordValueRules,
    grammar: Option<tcl_dialect::ParameterGrammar>,
) -> Option<Vec<String>> {
    let collapsed = rules.collapse_braced_word(param_text);
    let parameters =
        tcl_syntax::formal_params::parse_formal_parameters_in(&collapsed, grammar?).ok()?;
    Some(
        parameters
            .into_iter()
            .map(|parameter| parameter.name)
            .collect(),
    )
}

/// Retain original creation/publication geometry without borrowing compiler policy.
fn original_procedure_publication(
    tokens: &CommandTokens,
) -> Option<crate::signature_scan::scope::SignatureSourceCommand> {
    use crate::signature_scan::scope::{SignatureSourceCommand, SignatureSourceNameInput};
    let binding = tokens.source_binding.as_ref()?;
    let input = binding.original_written_name_input(tokens, 1)?;
    let SignatureSourceNameInput::OriginalWord(key) = &input else {
        return None;
    };
    let scope = binding.original_operand_naming_scope(&input)?;
    SignatureSourceCommand::procedure_from_key(&scope, key)
}

/// Return `Some(true)` / `Some(false)` if *`expr_text`* is a
/// literal Tcl boolean, or `None` when the condition is not a
/// simple literal. Tolerates surrounding whitespace and is
/// case-insensitive (matches `Tcl_GetBoolean`).
pub(crate) fn static_bool(expr_text: &str) -> Option<bool> {
    let stripped = expr_text.trim();
    match stripped {
        "0" => Some(false),
        "1" => Some(true),
        // Boolean words, incl. unique-prefix spellings (`tr`, `ye`, `of`).
        _ => tcl_syntax::boolean::parse_boolean_word(stripped),
    }
}

/// An `uplevel` level argument, parsed.
///
/// The two forms address different frames and must not be conflated:
/// `#N` counts *down* from the global frame, `N` counts *up* from the
/// current one. They coincide only when the current frame is the
/// global frame, which static analysis cannot assume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UplevelLevel {
    /// The level number as written, without the `#`.
    shift: i32,
    /// `true` for the `#N` absolute form, `false` for the relative form.
    absolute: bool,
}

/// Parse the level argument of an `uplevel` call. Accepts the
/// canonical relative form (`uplevel 1 body`, `uplevel 3 body`) and
/// the absolute form (`#0` / `#N`), returning `None` when the argument
/// is dynamic (`$lvl`, `[expr {...}]`) or otherwise unparseable.
///
/// `#0` and `0` both yield a `shift` of `0` and are told apart by
/// `absolute` — `uplevel #0 {…}` runs the body in the global frame
/// while `uplevel 0 {…}` runs it in the current one
/// (tclsh8.6/9.0-confirmed).
fn parse_uplevel_level(text: &str) -> Option<UplevelLevel> {
    if let Some(rest) = text.strip_prefix('#') {
        return rest.parse::<i32>().ok().map(|shift| UplevelLevel {
            shift,
            absolute: true,
        });
    }
    text.parse::<i32>().ok().map(|shift| UplevelLevel {
        shift,
        absolute: false,
    })
}

/// Return `(name, literal)` when *seg* is `set name {literal}`.
///
/// The LHS must be a plain bareword `Esc` token (no substitutions,
/// array index, or namespace qualifier) so we only ever track
/// proc-local scalars. The RHS must be a single brace-string `Str`
/// token.
fn assigned_local_body_literal(
    seg: &SegmentedCommand,
    stmt: &Statement,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, String)> {
    let Statement::AssignConst { name, value, .. } = stmt else {
        return None;
    };
    if seg.args().len() != 2
        || seg.single_token_word.get(..3)? != [true, true, true]
        || name != &seg.args()[0]
        || value != &seg.args()[1]
    {
        return None;
    }
    let arg_tokens = seg.arg_tokens();
    let name_tok = *arg_tokens.first()?;
    let value_tok = *arg_tokens.get(1)?;
    if name_tok.kind != TokenType::Esc || value_tok.kind != TokenType::Str {
        return None;
    }
    let name = crate::value_transfer::literal_token_value(name, name_tok.kind, &config)?;
    if name.is_empty()
        || name.contains('$')
        || name.contains('[')
        || name.contains('(')
        || tcl_syntax::naming::is_qualified(name.as_bytes())
    {
        return None;
    }
    let value = crate::value_transfer::literal_token_value(value, value_tok.kind, &config)?;
    Some((name.into_owned(), value.into_owned()))
}

/// If *`cmd_text`* is `list lit1 lit2 ...` with all-literal
/// arguments, return the body text the list would evaluate to —
/// otherwise `None`.
///
/// Literal means `Esc` / `Str` token only: no `$var` substitution,
/// no nested command substitution. `Str` (`{...}`) tokens are
/// re-braced in the synthesised body so list-canonicalisation
/// stays correct (we trust the segmenter's `single_token_word`
/// flag plus the absence of `$` / `[` in `Esc` text).
fn eval_list_literal_body(cmd_text: &str, config: tcl_lexer::LexerConfig) -> Option<String> {
    let image = tcl_lexer::SourceImage::native(cmd_text.as_bytes());
    let inner = crate::segmenter::segment_commands_image_with_offset_and_config(
        &image,
        0,
        config.nested(),
    )?;
    if inner.len() != 1 {
        return None;
    }
    let inner_cmd = &inner[0];
    if inner_cmd.texts.is_empty() || inner_cmd.texts[0] != "list" {
        return None;
    }
    let argv = inner_cmd.arg_tokens();
    let texts = inner_cmd.args();
    let single = &inner_cmd.single_token_word;
    // Each element after ``list`` must be a single-token literal.
    for (i, tok) in argv.iter().enumerate() {
        // ``single`` is per-word over the full argv (including
        // the command word at index 0) — argv here starts at
        // index 1 of the original argv, so the matching single-
        // token-word index is ``i + 1``.
        if !single.get(i + 1).copied().unwrap_or(false) {
            return None;
        }
        if !matches!(tok.kind, TokenType::Esc | TokenType::Str) {
            return None;
        }
        if tok.kind == TokenType::Esc {
            let text = &texts[i];
            if text.contains('$') || text.contains('[') {
                return None;
            }
        }
    }
    let mut parts: Vec<String> = Vec::with_capacity(argv.len());
    for (i, tok) in argv.iter().enumerate() {
        let text = &texts[i];
        if tok.kind == TokenType::Str {
            parts.push(format!("{{{text}}}"));
        } else {
            parts.push(text.clone());
        }
    }
    Some(parts.join(" "))
}

/// Drop const-map entries that *stmt* may have overwritten.
/// Straight-line assignments pop just the named variable;
/// structured IR and barriers conservatively clear the whole map
/// because their child scopes (or runtime side effects) could
/// touch any tracked name.
/// Token-level check that a relaxed-eval /
/// relaxed-uplevel body is free of nested dynamic-shape barriers.
///
/// When the eval/uplevel hooks consider relaxing a braced-literal
/// body to inline IR, they first walk the body's command words and
/// reject any nested `eval`/`uplevel` whose own body argument is
/// substitution-bearing (`$var` / `[cmd]` / multi-token).  Without
/// this gate, a static braced `eval {uplevel 1 $x}` would relax to
/// IR that runs a compiled `uplevel` with a still-dynamic body.
///
/// The walk is deliberately shallow and token-based:
///
/// 1. Segment the body into commands.
/// 2. For each command the registry marks as evaluating code, ask the
///    registry which words its script is made of and poison unless
///    every one of them is a braced literal (`Str`) word.
/// 3. Recurse into nested braced bodies and into braced-arg shapes
///    of non-barrier commands so a nested
///    `if { … } { eval $x }` still trips the gate.
///
/// Which word is the script is **never** re-derived here:
/// [`CommandRegistry::arg_indices_for_role`] answers it, so `uplevel`'s
/// optional-level shape is resolved by the one contract-tested
/// `uplevel_arg_roles` resolver instead of a second, subtly different
/// level-word sniff.  The barrier test likewise composes subcommand
/// traits ([`CommandRegistry::invocation_traits`]), so the compound
/// eval-family members — `namespace eval`, `namespace inscope`,
/// `interp eval`, all of which carry the eval-family bits on the
/// *subcommand* — flow through the same path a bare `eval` does.
///
/// Returns `true` when the body contains a nested dynamic-shape
/// barrier (the caller should fall back to [`Statement::Barrier`]); `false`
/// when the body is safe to relax.
fn body_has_dynamic_barrier(
    body_text: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> bool {
    use tcl_lexer::TokenType;
    use tcl_registry::prelude::Traits;
    let image = tcl_lexer::SourceImage::native(body_text.as_bytes());
    let Some(commands) =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config.nested())
    else {
        return true;
    };
    for sc in &commands {
        if sc.argv.is_empty() || sc.texts.is_empty() {
            continue;
        }
        let raw_name = sc.texts[0].as_str();
        let selected = registry
            .get(raw_name)
            .map(tcl_registry::CommandSpec::descriptor_key);
        let name = selected.map_or(raw_name, tcl_registry::CanonicalCommandDescriptorKey::label);
        let args: Vec<&str> = sc.texts[1..].iter().map(String::as_str).collect();
        // A "dynamic barrier" command evaluates a script (`eval`,
        // `uplevel`, `namespace eval`, `interp eval`, …).  Sourced from
        // the registry's `EVALUATES_CODE` trait rather than a hardcoded
        // name list.  `None` because the question is the
        // command's *shape*, not its availability: a barrier is a barrier
        // whichever dialect the file is analysed as.  Deliberate under
        // invariant I4 — a barrier answer *widens* (falls back to
        // `Statement::Barrier`), never specialises on an unproved
        // binding.
        let traits = registry.invocation_traits(name, &args, None);
        if !traits.contains(Traits::EVALUATES_CODE) {
            // Recurse into braced args of non-barrier commands so
            // nested barriers still trip the gate.
            for (i, tok) in sc.argv.iter().enumerate() {
                if i == 0 {
                    continue;
                }
                if tok.kind != TokenType::Str {
                    continue;
                }
                let arg_text = sc.texts.get(i).map_or("", String::as_str);
                if body_has_dynamic_barrier(arg_text, registry, config) {
                    return true;
                }
            }
            continue;
        }
        // Name is a barrier — inspect the words its script is made of.
        let body_indices = registry.arg_indices_for_role(name, &args, ArgRole::Body);
        let Some(&first_body) = body_indices.first() else {
            // The registry can point at no script word: a malformed call
            // (`uplevel 1` — a wrong-#args error) or an evaluator whose
            // code argument is a command *prefix* rather than a script
            // (`coroprobe` / `coroinject`).  Poison so the outer hook
            // falls back to Statement::Barrier and the runtime decides.
            return true;
        };
        // `SCRIPT_CONCATENATES_ARGS`: the trailing words concatenate into
        // the one script Tcl evaluates, so `uplevel 1 {set x 1} $tail`
        // is dynamic even though the marked body word is a literal.  Check
        // the whole tail from the first script word, not just the marked
        // ones.
        let script_words: Vec<usize> = if traits.contains(Traits::SCRIPT_CONCATENATES_ARGS) {
            (first_body..args.len()).collect()
        } else {
            body_indices
        };
        for idx in script_words {
            // `sc.argv` / `sc.texts` include the command name at 0, so the
            // arg-relative index shifts by one.
            let (Some(tok), Some(text)) = (sc.argv.get(idx + 1), sc.texts.get(idx + 1)) else {
                return true;
            };
            if tok.kind != TokenType::Str {
                return true;
            }
            // Recurse into the literal nested script word.
            if body_has_dynamic_barrier(text, registry, config) {
                return true;
            }
        }
    }
    false
}

fn invalidate_const_map_for(
    stmt: &Statement,
    scope: &mut HashMap<String, String>,
    config: tcl_lexer::LexerConfig,
) {
    match stmt {
        Statement::AssignConst {
            name, name_braced, ..
        } => {
            if let Some(literal) =
                crate::value_transfer::recorded_word_value(name, *name_braced, &config)
            {
                let root = tcl_syntax::naming::split_element_ref(&literal)
                    .map_or(literal.as_ref(), |(root, _)| root);
                scope.remove(root);
            } else {
                scope.clear();
            }
        }
        // Calls, substitutions and read traces can write outside their reported target.
        Statement::Call { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Barrier { .. }
        | Statement::NativeCall { .. }
        | Statement::Block { .. }
        | Statement::UpFrame { .. }
        | Statement::If { .. }
        | Statement::For { .. }
        | Statement::While { .. }
        | Statement::Foreach { .. }
        | Statement::Catch { .. }
        | Statement::Try { .. }
        | Statement::Switch { .. } => scope.clear(),
        Statement::Return { .. } => {}
    }
}

/// A memoised per-procedure body-lowering callback: `(offset-0 body text,
/// namespace) → offset-0 body [`Script`]`.  See [`Lowerer::with_body_cache`].
type BodyCacheFn<'a> = dyn Fn(&str, &str) -> Script + 'a;

/// Actual isolated-body worker output and complete lowering recipe. The body is
/// private; independently supplied script values cannot create this template.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginalBodyLoweringTemplate {
    text: String,
    namespace: String,
    registry: tcl_registry::RegistrySnapshot,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    body: Script,
}
impl OriginalBodyLoweringTemplate {
    /// Run the existing isolated lowerer once with the exact requested inputs.
    #[must_use]
    pub fn lower(
        text: &str,
        namespace: &str,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Self {
        let body = lower_proc_body_isolated(text, namespace, registry, config, dialect);
        Self {
            text: text.to_owned(),
            namespace: namespace.to_owned(),
            registry: registry.snapshot(),
            config,
            dialect,
            body,
        }
    }
    /// Reporting projection, without source-build or execution admission.
    #[must_use]
    pub fn body(&self) -> &Script {
        &self.body
    }
    fn matches(
        &self,
        text: &str,
        namespace: &str,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> bool {
        self.text == text
            && self.namespace == namespace
            && self.registry == registry.snapshot()
            && self.config == config
            && self.dialect == dialect
    }
}
/// Memo callback returning an actual immutable isolated-body worker artifact.
pub type OriginalBodyLoweringCache<'a> = dyn Fn(&str, &str) -> OriginalBodyLoweringTemplate + 'a;

/// Which lowering pass a [`Lowerer`] performs.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CompileTarget {
    /// The analysis IR: structured control flow kept everywhere, every
    /// registry-driven hook active. What LSP consumers (diagnostics,
    /// navigation) lower against.
    #[default]
    Analysis,
    /// The VM-faithful bytecode pass ([`lower_to_ir_for_bytecode`]):
    /// constructs the bytecode backend can't compile correctly (a bare
    /// `try`, a directly-nested `foreach`/`lmap`) barrier to a runtime
    /// dispatch instead of their structured IR; every other hook active.
    Bytecode,
    /// The bytecode pass with every registry-driven inline/structured
    /// lowering hook ALSO suppressed ([`lower_to_ir_traced`]): every command
    /// falls straight to [`Lowerer::lower_default`] — a plain runtime
    /// dispatch (`Statement::Call`/`Statement::Barrier` with a non-empty
    /// `command`, both of which `tcl_codegen::codegen` emits as `push words;
    /// INVOKE`), never a typed statement (`AssignConst`/`Incr`/`Return`/…)
    /// or a structured control-flow block (`If`/`While`/`Foreach`/`Switch`/
    /// `Catch`/`Try`/…). Used to recompile a proc/script body so execution
    /// traces (`enterstep`/`leavestep`) observe every command in it; see
    /// [`tcl_runtime_api::CompileService::compile_traced`]. Never set for
    /// the primary module compile — only for a standalone "compile this body
    /// text as a script" call, so a proc's own `proc name args body`
    /// DEFINITION always module-compiles normally; only its *body* is
    /// affected when recompiled under this target.
    BytecodeTraced,
}

impl CompileTarget {
    /// Whether this pass targets the bytecode backend (barriers
    /// backend-incompatible structured forms) — [`Self::Bytecode`] or
    /// [`Self::BytecodeTraced`].
    pub(crate) fn is_bytecode(self) -> bool {
        matches!(self, Self::Bytecode | Self::BytecodeTraced)
    }

    /// Whether this pass suppresses every inline/structured hook —
    /// [`Self::BytecodeTraced`] only.
    pub(crate) fn is_trace_visible(self) -> bool {
        self == Self::BytecodeTraced
    }
}

/// Origin of the execution entry contract used to initialise source resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceEntryOrigin {
    Authoring,
    Driver,
}

/// Complete source metadata retained independently of an explicitly selected
/// Standalone authoring driver. It supplies no runtime or Native entry.
#[derive(Clone, Copy)]
pub(crate) struct StandaloneSourceInput<'a> {
    registry: &'a CommandRegistry,
    input: &'a crate::analyser::ResolvedAnalysisInput,
}

impl<'a> StandaloneSourceInput<'a> {
    pub(crate) fn retain(
        registry: &'a CommandRegistry,
        input: &'a crate::analyser::ResolvedAnalysisInput,
        config: tcl_lexer::LexerConfig,
        profile: Option<&tcl_dialect::DialectProfile>,
    ) -> Option<Self> {
        if config != input.lexer_config() {
            return None;
        }
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry, input, config, profile,
        )?;
        Some(Self { registry, input })
    }
}

use tcl_lexer::SourceSegmentationPurpose;

#[derive(Clone, Copy)]
enum SourceBodyMode {
    Ordinary,
    ReadOnly,
}

impl SourceBodyMode {
    fn is_read_only(self) -> bool {
        matches!(self, Self::ReadOnly)
    }
}

/// Whether the catalogue describes every command the host can register.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HostCommandTable {
    Catalogue,
    Extensible,
}

impl HostCommandTable {
    const fn of(extensible: bool) -> Self {
        if extensible {
            Self::Extensible
        } else {
            Self::Catalogue
        }
    }

    const fn is_extensible(self) -> bool {
        matches!(self, Self::Extensible)
    }
}

#[cfg(test)]
#[derive(Clone, Copy)]
enum SourceBodySyntaxCacheVerification {
    Disabled,
    Enabled,
}

/// The lowering engine — accumulates procedures and IR statements.
pub struct Lowerer<'r> {
    /// Output module being built.
    pub module: Module,
    /// Compatibility hook argument; semantic aliases are owned by source bindings.
    aliases: CommandAliasMap,
    /// Point-specific command state in the current source span space.
    source_bindings: Option<RetainedSourceBodyInventory>,
    original_native_compiler_formals:
        Option<std::sync::Arc<crate::command_binding::OriginalNativeCompilerFormals>>,
    /// Written geometry only while lowering an authentic body with its
    /// independently selected namespace; never an execution or lookup owner.
    original_body_word_source: Option<crate::command_binding::OriginalBodyWrittenWordOwner>,
    /// Exact entry inventory retained for deferred method extraction.
    module_source_bindings: Option<Box<RetainedSourceBodyInventory>>,
    compilation_scope: tcl_runtime_api::SourceCompilationScope,
    /// Explicit driver-supplied standard-distribution loader contracts.
    trusted_package_loaders: Vec<TrustedPackageLoader>,
    trusted_source_modules: Vec<crate::command_binding::TrustedSourceModuleLoader>,
    /// Whether prior interpreter history prevents a fresh entry proof.
    unknown_entry: bool,
    /// The host may register commands outside the catalogue.
    host_command_table: HostCommandTable,
    /// An explicit entry contract replaces the default authoring assumption.
    source_entry_origin: SourceEntryOrigin,
    supplied_source: Option<crate::command_binding::SuppliedSourceAnalysisIssuance>,
    /// Availability phase supplied by the source entry owner.
    invocation_realm: tcl_dialect::model::InvocationRealm,
    /// Invocation policy snapshot supplied by the driver.
    invocation_dialect: Option<tcl_registry::InvocationDialect>,
    hosted_execution_context: Option<tcl_registry::f5::BigIpExecutionContext>,
    execution_name_policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    metadata_input: Option<crate::analyser::ResolvedAnalysisInput>,
    logical_source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    vendor_source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    compiled_variable_provider:
        Option<tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider>,
    expression_parser: Option<tcl_registry::invocation_words::LogicalExpressionParseProvider>,
    /// Evaluation protocol, independent of the runtime variable frame.
    native_compilation: tcl_registry::native_compilation::NativeCompilationContext,
    native_entry: Option<std::sync::Arc<tcl_runtime_api::NativeCompilationEntry>>,
    source_point_entry: Option<crate::command_binding::OriginalSourcePointEntry>,
    analyser_source_settings:
        Option<std::sync::Arc<crate::analyser::OriginalAnalyserSourceSettings>>,
    source_channel: tcl_lexer::SourceChannel,
    incoming_formals: Vec<String>,
    /// `class qualified name -> every instance variable declared for it`,
    /// accumulated across **all** of that class's definition blocks.
    ///
    /// A class's state is not confined to the block that created it: `oo::class
    /// create C { method m … }` followed by `oo::define C { variable x }`
    /// makes `x` instance state for `m`, which was extracted before the
    /// declaration was seen.  Each block's [`Self::extract_oo_methods`] run
    /// computes only its *own* block's declarations, so the union is
    /// accumulated here and merged into every method of the class by
    /// [`Self::extract_oo_methods_pass`] once all blocks have been walked.
    /// Declaring `variable x` anywhere makes it instance state for every
    /// method of that class, so the union is order-free.
    class_instance_vars: HashMap<String, HashSet<String>>,
    /// Event handler occurrence counts (for `when` numbering).
    when_counts: HashMap<String, u32>,
    /// The registry-owned iRules execution placement of the script currently
    /// being lowered.  This is deliberately distinct from `proc_depth`:
    /// control-flow bodies inherit an event/procedure context, whereas a body
    /// opened by a top-level executable statement is invalid declaration
    /// territory and must not manufacture `when` / `proc` procedures.
    irules_execution_context: IrulesExecutionContext,
    /// Priority inherited by subsequent top-level iRules event declarations.
    irules_priority: u16,
    /// Monotonic counter for synthetic *body unit* names (`apply` lambdas and
    /// `namespace eval` bodies), so each registers under a unique qualified
    /// name in [`Module::body_units`]. Deterministic in source order, so the
    /// fresh and incremental lowerings produce identical names.
    body_unit_count: u32,
    /// Whether we're inside a `namespace eval` body.
    in_namespace_eval: bool,
    /// Command registry for arg-role queries.
    registry: &'r CommandRegistry,
    /// The document's own command declarations (`# tcl-lsp: stub` blocks and
    /// `<dialect>.tcl.stubs` sidecars), unioned with the catalogue's answer
    /// through [`Self::command_surface`]. `None` for a document that
    /// declares nothing, and for every non-document caller (tests, the
    /// bytecode path).
    declared_commands: Option<&'r tcl_registry::model::DeclaredSurface>,
    source_declared_commands: Option<tcl_registry::model::DeclaredSurface>,
    /// Per-script const-map stack. Each scope tracks
    /// proc-local variables assigned a brace-string literal so
    /// later `eval $var` / `uplevel 1 $var` calls can fold the
    /// body in at lowering time. Active only when
    /// `proc_depth > 0` — top-level / `namespace eval` scopes
    /// write globals or namespace vars whose values can be
    /// observed and mutated by other code, so const-propagating
    /// them is unsound.
    const_map_stack: Vec<HashMap<String, String>>,
    /// Per-recursive-script structured command dependencies. Each body owns
    /// its sites, so CFG construction receives the exact bindings for the
    /// script it flattens rather than a module-global approximation.
    command_binding_site_stack: Vec<Vec<CommandBindingSite>>,
    /// Exact original parent across structured body descent; no entry is inferred.
    structured_source_parent: Option<Box<CommandTokens>>,
    /// Depth of `proc` / `when` body lowerings currently in
    /// flight. A positive value enables the const-map.
    proc_depth: u32,
    /// `namespace import` directives observed at lowering time.
    /// Literal `(context_namespace, pattern)` dependency pairs preserve source
    /// order independently of the typed import resolver.
    namespace_imports: Vec<(String, String)>,
    /// `namespace export` directives observed at lowering time.
    /// Recorded as `(context_namespace, pattern)` pairs
    /// and copied into `Module::namespace_exports` at the end of
    /// lowering. Order is preserved.
    namespace_exports: Vec<(String, String)>,
    /// A retained body may be lowered into both a unit and a generic region.
    /// Record each original directive once per actual namespace.
    namespace_directive_sites: HashSet<(crate::command_binding::CommandAllocationSite, String)>,
    /// Depth of statically-dead branches currently being lowered.
    /// `if {0} {…}` / `if {1} {…} else {…}` bump this
    /// around the dead body so any `namespace import` /
    /// `namespace export` directives found inside don't register
    /// with the module-level tables. The IR for the dead code is
    /// still produced so consumers that walk the tree by syntactic
    /// offset see the original structure.
    pub(crate) dead_code_depth: u32,
    /// `true` while lowering a `TclOO` method body. A `proc`
    /// (or `namespace eval`-lifted proc) defined inside a method
    /// body is created at method-call time in the runtime receiver namespace,
    /// NOT at class-definition time — so it must not be lifted into
    /// `module.procedures` (codegen would otherwise emit it
    /// unconditionally at script load). The method body is still
    /// lowered for analysis; only the premature static registration is
    /// suppressed.
    suppress_proc_register: bool,
    /// Genuine readonly Logical body preparation cannot issue executed-source metadata.
    readonly_logical_body: SourceBodyMode,
    readonly_native_declaration_body: SourceBodyMode,
    native_source_children:
        Option<std::sync::Arc<crate::command_binding::OriginalNativeSourceBodyChildren>>,
    /// Hosted source cards keep their independent declaration owner.
    readonly_hosted_body: SourceBodyMode,
    readonly_authored_body: SourceBodyMode,
    authored_source_scope:
        Option<std::sync::Arc<crate::registry_invocation::OriginalAuthoredSourceBodyScope>>,
    /// Lexer config for the document's dialect, threaded into every
    /// body re-segmentation so `{*}` expansion (off for Tcl 8.4 /
    /// iRules) and the iRules `}{` ghost SEP are honoured rather than
    /// always assuming the Tcl-8.5+ default.  Defaults to
    /// `LexerConfig::default()`; production callers thread the active
    /// dialect via [`Lowerer::with_config`] / [`lower_to_ir_with_config`].
    config: tcl_lexer::LexerConfig,
    /// The document's analysis dialect (`None` for plain Tcl), threaded into
    /// every `expr` / condition parse so a dialect-only operator — an iRules
    /// word operator such as `contains` or `starts_with` — is parsed as that
    /// operator instead of degrading to
    /// [`crate::expr_ast::ExprNode::Raw`], which no downstream fold can
    /// evaluate.  Separate from `config`: the lexer config carries the
    /// dialect's *word* tokenisation, this carries its *expression* grammar.
    /// Set by [`Lowerer::with_dialect`] / [`lower_to_ir_with_dialect`].
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    /// The dialect's resolved registry generation, derived beside
    /// [`Self::dialect`] — the context [`try_lower_hook`]'s selection
    /// primitive reads for the I4 binding proof (P1a): a head the
    /// environment does not provide selects no lowering hook and falls to
    /// the default `Statement::Call` path.
    dialect_context: Option<std::sync::Arc<tcl_registry::model::ContextRegistry>>,
    /// Explicit source context survives a driver transition; static authoring
    /// compatibility cannot fill an absent driver's input.
    metadata_origin: LoweringMetadataOrigin,
    /// Which lowering pass this instance performs — folds what would
    /// otherwise be two related bool fields (`for_bytecode`, `trace_visible`)
    /// into one three-state enum (`clippy::struct_excessive_bools`); see
    /// [`CompileTarget`].
    pub(crate) target: CompileTarget,
    /// Optional memoised per-procedure body lowering.
    /// When set, a **top-level** `proc`'s static literal body is lowered through
    /// this callback `(offset-0 body text, namespace) -> offset-0 body Script`
    /// (the caller rebases by the body offset) instead of `lower_body`, so an
    /// unchanged proc's body IR is reused across edits.  Set only for
    /// **context-free** files (no `namespace eval`/`import`/`export`, alias,
    /// `oo::`, `when`, or nested `proc`) where isolated lowering is byte-identical;
    /// `None` ⇒ the normal whole-file lowering.  See [`lower_to_ir_with_body_cache`].
    body_cache: Option<&'r BodyCacheFn<'r>>,
    source_body_syntax_cache: Option<&'r SourceBodySyntaxCache<'r>>,
    original_body_cache: Option<&'r OriginalBodyLoweringCache<'r>>,
    optimiser_lowering_integrity: Option<&'r std::cell::Cell<bool>>,
    #[cfg(test)]
    verify_source_body_syntax_cache: SourceBodySyntaxCacheVerification,
    /// Current `lower_script` / `lower_body` recursion depth, bounded by
    /// [`MAX_LOWER_NEST_DEPTH`] so deeply-nested bodies cannot overflow the
    /// stack. Mirrors [`crate::cfg_builder::CfgBuilder`]'s `depth` field —
    /// lowering runs *before* CFG construction, so an unguarded recursion
    /// here reaches the same crash first and makes the CFG builder's own
    /// cap moot.
    nest_depth: u32,
    /// The buffer every span this lowering emits indexes into — the document
    /// text for an ordinary lowering, and the *materialised literal* while a
    /// synthesised script is lowered at offset `0` inside one (the `eval
    /// $const` / factory-body paths that re-enter [`Self::lower_script`]).
    /// [`Self::lower_script`] swaps it for the duration of each such
    /// re-entry, because that is exactly where a new span space begins;
    /// [`Self::lower_body`] does not, since a body's spans are rebased into
    /// the space it inherits.
    ///
    /// Held so the lowerer can *check* a body word's rebase against the text
    /// it claims to slice ([`crate::segmenter::body_text_in_region`] in
    /// [`Self::lower_body_from_tok`]) rather than trusting the arithmetic —
    /// the same guard the analyser applies.  Empty until a
    /// script is lowered, which the guard reads as "nothing to compare
    /// against" and leaves every body text alone.
    source: String,
    /// The buffer each word model is read from — see [`WordSpace`].
    word_space: WordSpace,
}

/// The text a [`crate::ir::WordExpr`] is decomposed from, and the offset its
/// first byte sits at in the span space the segmentation used.
///
/// Deliberately *not* [`Lowerer::source`]: a word is read from the very text
/// that was segmented into it, which is not always a slice of the enclosing
/// buffer.  A `{body}x` word's body is clamped to the part that does map;
/// an `eval $const` / `eval [list …]` body is a literal
/// materialised at lowering time and lives in a span space of its own; and
/// the `TclOO` method extraction runs as a post-pass, after the document's
/// span space has been restored to the (empty) enclosing one.  Reading the
/// segmented text itself makes the word model right in all three, and the
/// base keeps every span it reports in the space the statements' spans are
/// in.
struct WordSpace {
    text: String,
    base: u32,
    /// `text`'s line index, held so the per-command [`tcl_lexer::SourceMap`]
    /// is a refcount bump (`LineIndex` is `Arc`-backed) rather than a rescan.
    lines: tcl_lexer::LineIndex,
    channel: tcl_lexer::SourceChannel,
}

impl WordSpace {
    fn new(text: &str, base: u32, channel: tcl_lexer::SourceChannel) -> Self {
        Self {
            text: text.to_owned(),
            base,
            lines: tcl_lexer::LineIndex::new(text),
            channel,
        }
    }

    /// A [`tcl_lexer::SourceMap`] over this text, carrying its base so a word
    /// built from it reports spans in the enclosing span space.
    fn map(&self) -> tcl_lexer::SourceMap<'_> {
        tcl_lexer::SourceMap::from_bytes_with_line_index(
            self.text.as_bytes(),
            self.channel,
            self.lines.clone(),
        )
        .with_base(self.base, 0, 0)
    }
}

/// Maximum nesting depth for the recursive `lower_script` / `lower_body`
/// descent (`lower_script` ↔ `lower_body` ↔ `lower_segmented` ↔
/// `lower_command` are mutually recursive with one Rust frame group per
/// braced-body nesting level). Matches
/// [`crate::cfg_builder::MAX_LOWER_DEPTH`] and
/// `tcl_compiler::analyser::commands::MAX_BODY_DEPTH` — all three
/// independently-recursive walkers over the same source cap at the same
/// depth, so no consumer of this crate depends on one pass reaching a
/// deeper nesting level than another. All three read that depth from
/// [`crate::depth_guard::MAX_SOURCE_NEST_DEPTH`], so it is a shared
/// *budget* rather than a shared *convention*.
///
/// This walk is the one the budget is sized by: at eight Rust frames per
/// nesting level it costs about 18.4 KiB a level, more than twice either
/// sibling, and it is the walk whose recursion the ~400-nested-`foreach`
/// reproducer aborted on.
const MAX_LOWER_NEST_DEPTH: tcl_core_types::RecursionLimit =
    crate::depth_guard::MAX_SOURCE_NEST_DEPTH;

/// Shared construction bound for consumers deriving nested source bodies.
/// A subset frontend must decline further descent when this limit is exceeded.
#[must_use]
pub const fn source_nesting_limit() -> tcl_core_types::RecursionLimit {
    MAX_LOWER_NEST_DEPTH
}

/// The substitutions [`crate::subst_nocommands`] performs: `[cmd]` off,
/// `$var` and backslashes on.
///
/// A `subst` call the registry answers with exactly this set is one that
/// evaluator reproduces, however the call spells it —
/// `subst -nocommands {…}` and Tcl 9.1's `subst -variables -backslashes {…}`
/// both land here (tclsh 9.1b0: `subst -variables -backslashes {hello
/// $name\n[format X]}` with `name` set to `world` yields "hello world",
/// a newline, then the untouched `[format X]`). The registry answers a call
/// it cannot read with every kind, which is not this set, so a computed
/// switch word folds nothing.
pub(crate) const SUBST_NOCOMMANDS_KINDS: tcl_registry::substitution::SubstitutionKinds =
    tcl_registry::substitution::SubstitutionKinds {
        backslashes: true,
        commands: false,
        variables: true,
    };

/// [`ContextRegistry::with_command_store`] is a pure function of the
/// context generation and the store, and a lowering rebuilds it for every
/// snippet; reuse a few recent assemblies. Held handles keep both addresses
/// from being reused while an entry remains.
fn context_with_command_store(
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
    commands: std::sync::Arc<CommandRegistry>,
) -> std::sync::Arc<tcl_registry::model::ContextRegistry> {
    type Entry = (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        std::sync::Arc<CommandRegistry>,
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
    );
    const MAX_RETAINED: usize = 8;
    static MEMO: std::sync::OnceLock<std::sync::Mutex<Vec<Entry>>> = std::sync::OnceLock::new();
    let memo = MEMO.get_or_init(Default::default);
    let lock = || {
        memo.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    };
    if let Some((_, _, assembled)) = lock().iter().find(|(base, store, _)| {
        std::sync::Arc::ptr_eq(base, context) && std::sync::Arc::ptr_eq(store, &commands)
    }) {
        return std::sync::Arc::clone(assembled);
    }
    let assembled =
        std::sync::Arc::new(context.with_command_store(std::sync::Arc::clone(&commands)));
    let mut entries = lock();
    if entries.len() >= MAX_RETAINED {
        entries.remove(0);
    }
    entries.push((
        std::sync::Arc::clone(context),
        commands,
        std::sync::Arc::clone(&assembled),
    ));
    assembled
}

impl<'r> Lowerer<'r> {
    /// Create a new lowerer with the default (Tcl-8.5+) lexer config.
    #[must_use]
    pub fn new(registry: &'r CommandRegistry) -> Self {
        // dialect-drift-ok: the test-only constructor — every production
        // construction now goes through `Lowerer::with_config` with the
        // unit's own config; the crate's (and downstream crates') test
        // fixtures still spell the plain-Tcl default this way.
        Self::with_config(registry, tcl_lexer::LexerConfig::default())
    }

    /// Create a lowerer with an explicit dialect [`tcl_lexer::LexerConfig`]
    /// (see the `config` field).
    #[must_use]
    pub fn with_config(registry: &'r CommandRegistry, config: tcl_lexer::LexerConfig) -> Self {
        Self {
            module: Module::default(),
            aliases: CommandAliasMap::new(),
            source_bindings: None,
            original_native_compiler_formals: None,
            original_body_word_source: None,
            module_source_bindings: None,
            compilation_scope: tcl_runtime_api::SourceCompilationScope::WholeModule,
            trusted_package_loaders: Vec::new(),
            trusted_source_modules: Vec::new(),
            unknown_entry: false,
            host_command_table: HostCommandTable::Catalogue,
            source_entry_origin: SourceEntryOrigin::Authoring,
            supplied_source: None,
            invocation_realm: tcl_dialect::model::InvocationRealm::RuleLoader,
            invocation_dialect: Some(crate::environment_ingress::authoring_invocation_dialect(
                registry, None, config,
            )),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            compiled_variable_provider: None,
            expression_parser: None,
            hosted_execution_context: None,
            execution_name_policy: None,
            metadata_input: None,
            logical_source_input: None,
            vendor_source_input: None,
            native_entry: None,
            source_point_entry: None,
            analyser_source_settings: None,
            source_channel: tcl_lexer::SourceChannel::Document,
            incoming_formals: Vec::new(),
            class_instance_vars: HashMap::new(),
            when_counts: HashMap::new(),
            irules_execution_context: IrulesExecutionContext::TopLevel,
            irules_priority: 500,
            body_unit_count: 0,
            in_namespace_eval: false,
            registry,
            declared_commands: None,
            source_declared_commands: None,
            const_map_stack: Vec::new(),
            command_binding_site_stack: Vec::new(),
            structured_source_parent: None,
            proc_depth: 0,
            namespace_imports: Vec::new(),
            namespace_exports: Vec::new(),
            namespace_directive_sites: HashSet::new(),
            dead_code_depth: 0,
            suppress_proc_register: false,
            readonly_logical_body: SourceBodyMode::Ordinary,
            readonly_native_declaration_body: SourceBodyMode::Ordinary,
            native_source_children: None,
            readonly_hosted_body: SourceBodyMode::Ordinary,
            readonly_authored_body: SourceBodyMode::Ordinary,
            authored_source_scope: None,
            config,
            dialect: None,
            dialect_context: None,
            metadata_origin: LoweringMetadataOrigin::Standalone,
            target: CompileTarget::Analysis,
            body_cache: None,
            original_body_cache: None,
            optimiser_lowering_integrity: None,
            source_body_syntax_cache: None,
            #[cfg(test)]
            verify_source_body_syntax_cache: SourceBodySyntaxCacheVerification::Disabled,
            nest_depth: 0,
            source: String::new(),
            word_space: WordSpace::new("", 0, tcl_lexer::SourceChannel::Document),
        }
    }

    pub(crate) fn with_original_optimiser_lowering(
        mut self,
        cache: Option<&'r OriginalBodyLoweringCache<'r>>,
        integrity: &'r std::cell::Cell<bool>,
    ) -> Self {
        self.original_body_cache = cache;
        self.optimiser_lowering_integrity = Some(integrity);
        self
    }

    /// Install the document's own command declarations (see
    /// [`declared_commands`](Self::declared_commands)).
    ///
    /// A stub declares the same kind of fact a `CommandSpec` states, so it
    /// reaches lowering through the same argument-role query the catalogue
    /// does — [`Self::command_surface`] — rather than a parallel table.
    #[must_use]
    pub fn with_declared_commands(
        mut self,
        declared: Option<&'r tcl_registry::model::DeclaredSurface>,
    ) -> Self {
        self.declared_commands = declared;
        self
    }

    /// The driver's module contracts, else the workspace modules the document
    /// input offers for auto-loading.
    fn effective_source_modules(&self) -> &[crate::command_binding::TrustedSourceModuleLoader] {
        if self.trusted_source_modules.is_empty() {
            self.metadata_input
                .as_ref()
                .or(self.logical_source_input.as_ref())
                .or(self.vendor_source_input.as_ref())
                .map_or(
                    &[],
                    crate::analyser::ResolvedAnalysisInput::workspace_source_modules,
                )
        } else {
            &self.trusted_source_modules
        }
    }

    /// Set the document's analysis dialect (see [`Lowerer::dialect`]).
    ///
    /// `None` means the caller named *no* dialect — deliberately distinct
    /// from `Some(plain_tcl)`, which is an explicit plain-Tcl document. The
    /// two select different numeral sources downstream (`None` defers to the
    /// thread-ambient target; `Some` pins the profile's grammar), so a caller
    /// holding an optional ingress spelling must map the unstated case to
    /// `None` rather than defaulting to the plain profile. Matches
    /// [`crate::compilation_unit::UnitBuildOptions::dialect`].
    #[must_use]
    pub fn with_dialect(mut self, dialect: Option<&'static tcl_dialect::DialectProfile>) -> Self {
        self.retain_authoring_dialect(dialect);
        if self.source_entry_origin == SourceEntryOrigin::Authoring
            && self.metadata_origin == LoweringMetadataOrigin::Standalone
        {
            self.dialect_context = dialect.map(|profile| {
                let context = crate::environment_ingress::context_for_profile(profile);
                if context.commands().snapshot().semantic_key()
                    == self.registry.snapshot().semantic_key()
                {
                    context
                } else {
                    context_with_command_store(&context, self.registry.snapshot().shared_registry())
                }
            });
        }
        self
    }

    fn retain_authoring_dialect(&mut self, dialect: Option<&'static tcl_dialect::DialectProfile>) {
        self.dialect = dialect;
        if self.source_entry_origin == SourceEntryOrigin::Authoring {
            self.invocation_dialect =
                Some(crate::environment_ingress::authoring_invocation_dialect(
                    self.registry,
                    dialect,
                    self.config,
                ));
        }
    }

    /// Explicit authoring ingress: a profile supplies grammar and metadata
    /// recipes, independently of any supplied availability or runtime entry.
    pub(crate) fn for_standalone_source(
        registry: &'r CommandRegistry,
        config: tcl_lexer::LexerConfig,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Self {
        let mut lowerer = Self::with_config(registry, config);
        lowerer.retain_authoring_dialect(dialect);
        lowerer
    }

    /// Explicit Standalone driver with complete, independently retained metadata.
    /// Supplied-document ingress never calls this constructor or obtains its model.
    pub(crate) fn for_standalone_source_with_input(owner: StandaloneSourceInput<'r>) -> Self {
        let input = owner.input.clone();
        let mut lowerer = Self::for_standalone_source(
            owner.registry,
            input.lexer_config(),
            Some(input.unit_profile()),
        );
        lowerer.dialect_context = Some(input.context_registry());
        // A rewrite of standalone source must hold wherever it later runs,
        // including under a host that registers its own commands.
        lowerer.host_command_table = HostCommandTable::Extensible;
        lowerer.retain_standalone_source_metadata(input);
        lowerer
    }

    fn retain_standalone_source_metadata(&mut self, input: crate::analyser::ResolvedAnalysisInput) {
        if input.has_logical_source_name_context() || input.has_hosted_source_name_context() {
            self.retain_resolved_analysis_input(&input);
            drop(input);
        } else {
            // The explicit driver already selected authoring independently. Metadata
            // retains its exact store/configuration without replacing that purpose.
            self.metadata_input = Some(input);
            self.metadata_origin = LoweringMetadataOrigin::Supplied;
        }
    }

    /// The host may register commands the catalogue does not describe, so an
    /// undefined head is not bounded by the stock `unknown`.
    #[must_use]
    pub const fn with_host_extensible_command_table(mut self) -> Self {
        self.host_command_table = HostCommandTable::Extensible;
        self
    }

    /// Use the caller's complete availability context for conditional source
    /// metadata. A foreign command generation declines instead of resolving
    /// a replacement from the profile label.
    #[must_use]
    pub fn with_context_registry(
        mut self,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> Self {
        // Preserve supplied ownership even when it is foreign. Shared metadata
        // consumers then refuse it instead of rebuilding a profile fallback.
        self.dialect_context = Some(context);
        self.metadata_origin = LoweringMetadataOrigin::Supplied;
        self
    }

    /// Replace the default fresh entry of the retained input with an explicit
    /// unknown prior history.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_explicit_unknown_entry(mut self) -> Self {
        let input = self
            .metadata_input
            .clone()
            .expect("retained analysis input");
        let entry = crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input);
        self.set_source_analysis_options(entry.options());
        self
    }

    /// Retain the complete caller input independently of execution entry.
    /// Missing or foreign source metadata cannot acquire a profile fallback.
    #[must_use]
    pub fn with_resolved_analysis_input(
        mut self,
        input: crate::analyser::ResolvedAnalysisInput,
    ) -> Self {
        self.retain_resolved_analysis_input(&input);
        drop(input);
        self
    }

    fn retain_resolved_analysis_input(&mut self, input: &crate::analyser::ResolvedAnalysisInput) {
        self.dialect = self.dialect.or(Some(input.unit_profile()));
        let fresh = self.source_entry_origin == SourceEntryOrigin::Authoring;
        self.dialect_context = Some(input.context_registry());
        self.metadata_input = Some(input.clone());
        self.metadata_origin = LoweringMetadataOrigin::Supplied;
        if fresh {
            self.set_supplied_source_analysis_input(input, None);
        }
    }

    /// Choose the fresh supplied-document factory, independently of effective naming policy.
    pub(crate) fn set_supplied_source_analysis_input(
        &mut self,
        input: &crate::analyser::ResolvedAnalysisInput,
        settings: Option<&std::sync::Arc<crate::analyser::OriginalAnalyserSourceSettings>>,
    ) {
        let (entry, issued) = crate::command_binding::SuppliedSourceAnalysisIssuance::issue(
            self.registry,
            input,
            self.config,
            self.dialect,
            settings,
            self.declared_commands,
        );
        self.set_source_analysis_options(entry.options());
        self.supplied_source = Some(issued);
    }

    /// Install a memoised per-procedure body-lowering callback (see
    /// [`body_cache`](Self::body_cache) and [`lower_to_ir_with_body_cache`]).
    #[must_use]
    pub fn with_body_cache(mut self, cache: &'r BodyCacheFn<'r>) -> Self {
        self.body_cache = Some(cache);
        self
    }

    /// Cache certified conditional source syntax without borrowing executed-body proofs.
    #[must_use]
    pub fn with_source_body_syntax_cache(mut self, cache: &'r SourceBodySyntaxCache<'r>) -> Self {
        self.source_body_syntax_cache = Some(cache);
        self
    }

    /// Mark this as the bytecode/VM compile path, barriering constructs the
    /// backend can't compile (see [`CompileTarget::Bytecode`]). A no-op if
    /// [`Self::trace_visible`] already upgraded this to
    /// [`CompileTarget::BytecodeTraced`] (still bytecode-backend, just also
    /// trace-visible) — order-independent whichever builder call comes first.
    #[must_use]
    pub fn for_bytecode_backend(mut self) -> Self {
        if self.target == CompileTarget::Analysis {
            self.target = CompileTarget::Bytecode;
            self.select_bytecode_entry();
        }
        self
    }

    /// Suppress every inline/structured lowering hook so the whole lowering
    /// pass produces plain runtime dispatches (see
    /// [`CompileTarget::BytecodeTraced`]). Implies the bytecode backend.
    #[must_use]
    pub fn trace_visible(mut self) -> Self {
        self.target = CompileTarget::BytecodeTraced;
        self.select_bytecode_entry();
        self
    }

    fn select_bytecode_entry(&mut self) {
        if self.source_entry_origin == SourceEntryOrigin::Authoring {
            self.native_compilation.mode =
                tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject;
        }
    }

    /// Lower a complete source string to an IR module.
    pub fn lower(&mut self, source: &str) -> &Module {
        self.start_module();
        self.module.source =
            tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_channel);

        if self.source_point_entry.as_ref().is_some_and(|point| {
            self.target.is_bytecode()
                || self.module.source_entry.unknown_entry
                || self
                    .module
                    .source_entry
                    .metadata_context
                    .source_analysis_input()
                    != Some(point.input())
        }) && self.refuse_inherited_runtime_target(source)
        {
            return &self.module;
        }
        let namespace = self.runtime_target_namespace("");
        self.module.top_level_namespace.clone_from(&namespace);
        self.module.top_level =
            self.lower_script(source, &namespace, SourceSegmentationPurpose::WholeInput);

        self.retain_module_source_bindings();
        &self.module
    }

    /// Lower a runtime procedure body as this module's top-level script.
    ///
    /// This is the procedure-target counterpart of [`Self::lower`].  It uses
    /// the same fresh frame as a static `proc` body, while retaining all
    /// module-wide side effects (nested procedures, aliases, namespaces, OO
    /// definitions, and traces) for the bytecode backend.
    pub fn lower_procedure_target(&mut self, source: &str, namespace: &str) -> &Module {
        self.start_module();
        self.module.source =
            tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_channel);
        // The runtime ABI carries an unrooted constructed namespace key while
        // compiler IR records rooted constructed keys.  Convert at the shared
        // naming owner without re-parsing the key as a written Tcl word: a
        // leading `::` here may belong to a literal-colon namespace segment.
        if self.refuse_inherited_runtime_target(source) {
            return &self.module;
        }
        let namespace = self.runtime_target_namespace(namespace);
        self.module.top_level_namespace.clone_from(&namespace);
        // This module's top level *is* a procedure body, which the `::top`
        // name cannot convey to anything downstream (#2207).
        self.module.top_level_kind = crate::ir::TopLevelKind::ProcedureBody;
        self.module.source_entry.native_compilation.frame =
            tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
        self.module.top_level =
            self.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                lowerer.lower_script(source, &namespace, SourceSegmentationPurpose::NestedScript)
            });
        self.retain_module_source_bindings();
        &self.module
    }

    /// Lower a runtime script in an explicit constructed namespace.
    ///
    /// Unlike [`Self::lower_procedure_target`], this keeps script-frame
    /// variable and return semantics. Both target forms share the exact same
    /// namespace conversion and module provenance field.
    pub fn lower_script_target(&mut self, source: &str, namespace: &str) -> &Module {
        self.start_module();
        self.module.source =
            tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_channel);
        if self.refuse_inherited_runtime_target(source) {
            return &self.module;
        }
        let namespace = self.runtime_target_namespace(namespace);
        self.module.top_level_namespace.clone_from(&namespace);
        self.module.top_level =
            self.lower_script(source, &namespace, SourceSegmentationPurpose::WholeInput);
        self.retain_module_source_bindings();
        &self.module
    }

    fn retain_module_source_bindings(&mut self) {
        self.module.retained_source_bindings =
            self.module_source_bindings.as_deref().and_then(|bindings| {
                crate::command_binding::RetainedSourceModuleBindings::retain(
                    bindings,
                    &self.module,
                    self.registry,
                    self.supplied_source.as_ref(),
                )
            });
    }

    fn refuse_inherited_runtime_target(&mut self, source: &str) -> bool {
        if self.source_point_entry.is_none() {
            return false;
        }
        self.module.top_level = unavailable_original_body_script(
            0,
            source.len(),
            "inherited source view has no runtime target",
        );
        true
    }

    fn runtime_target_namespace(&mut self, namespace: &str) -> String {
        if let Some(entry) = &self.source_point_entry {
            self.module.top_level_namespace_context = Some(entry.namespace().clone());
            return entry.namespace().display().unwrap_or_default();
        }
        let key = self.native_entry.as_ref().map_or_else(
            || {
                Some(crate::command_binding::SourceNamespaceKey::authored(
                    tcl_syntax::naming::root_unrooted_key(namespace),
                ))
            },
            |entry| crate::command_binding::SourceNamespaceKey::from_native_entry(entry).ok(),
        );
        let display = key
            .as_ref()
            .and_then(crate::command_binding::SourceNamespaceKey::display)
            .unwrap_or_else(|| tcl_syntax::naming::root_unrooted_key(namespace));
        self.module.top_level_namespace_context = key;
        display
    }

    fn source_entry_frame(&self, namespace: &str) -> crate::var_resolve::VariableExecutionFrame {
        if self.nest_depth == 1
            && let Some(entry) = &self.source_point_entry
        {
            return entry.frame().clone();
        }
        crate::command_binding::source_analysis_frame(
            self.native_entry
                .as_deref()
                .filter(|_| self.nest_depth == 1),
            namespace,
            self.module.top_level_kind,
        )
    }

    fn start_module(&mut self) {
        if self.source_entry_origin == SourceEntryOrigin::Authoring
            && self.metadata_origin == LoweringMetadataOrigin::Standalone
            && self.metadata_input.is_none()
            && self.logical_source_input.is_none()
            && self.vendor_source_input.is_none()
            && self.native_entry.is_none()
            && let Some(profile) = self.dialect.or_else(|| self.registry.profile())
            && let Some(context) = self.dialect_context.clone()
        {
            let input =
                crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, self.config);
            self.retain_standalone_source_metadata(input);
        }
        self.module.top_level_namespace_context = None;
        self.module_source_bindings = None;
        self.module.lexer_config = self.config;
        self.module.dialect_profile = self.dialect.or_else(|| self.registry.profile());
        self.module.registry_snapshot = Some(self.registry.snapshot());
        self.module.source_metadata_input = self
            .metadata_input
            .clone()
            .or_else(|| self.logical_source_input.clone())
            .or_else(|| self.vendor_source_input.clone());
        self.module.dialect = self
            .module
            .dialect_profile
            .map(|profile| profile.name.to_owned());
        // Static authoring uses a fresh standard-distribution entry. Runtime
        // compilation has no authority to assert an installed package loader.
        if self.source_entry_origin == SourceEntryOrigin::Authoring && !self.target.is_bytecode() {
            let profile = self.dialect.or_else(|| self.registry.profile());
            if profile.is_some_and(|profile| {
                !profile.is_irules()
                    && profile.package_protocol() == Some(tcl_dialect::PackageProtocol::Tcl)
            }) {
                let mut providers = std::collections::BTreeMap::new();
                for name in self.registry.command_names() {
                    if let Some(tcl_registry::body_execution::BodyExecutionSpec::CapturedLifecycle(
                        contract,
                    )) = self.registry.get(name).and_then(|spec| spec.body_execution)
                        && let Some(loader) = stock_body_provider_loader(contract.provider, None)
                    {
                        providers.insert(loader.package.clone(), loader);
                    }
                }
                self.trusted_package_loaders = providers.into_values().collect();
            }
        }
        self.module.plain_command_dispatch = self.target.is_trace_visible();
        self.module.source_entry = crate::command_binding::SourceAnalysisEntry {
            source_point_entry: self.source_point_entry.clone(),
            analyser_source_settings: self.analyser_source_settings.clone(),
            metadata_context: self.source_metadata_input().retain(),
            hosted_execution_context: self.hosted_execution_context,
            execution_name_policy: self.execution_name_policy,
            logical_source_input: self.logical_source_input.clone(),
            vendor_source_input: self.vendor_source_input.clone(),
            compilation_scope: self.compilation_scope,
            invocation_realm: self.invocation_realm,
            declared_commands: self
                .source_declared_commands
                .as_ref()
                .or(self.declared_commands)
                .cloned(),
            trusted_package_loaders: self.trusted_package_loaders.clone(),
            trusted_source_modules: self.effective_source_modules().to_vec(),
            unknown_entry: self.unknown_entry,
            host_extensible_command_table: self.host_command_table.is_extensible(),
            invocation_dialect: self.invocation_dialect,
            compiled_variable_provider: self.compiled_variable_provider,
            expression_parser: self.expression_parser,
            native_compilation: self.native_compilation,
            native_entry: self.native_entry.clone(),
            incoming_formals: self.incoming_formals.clone(),
        };
    }

    /// Enter a fresh procedure-like runtime frame for one body lowering.
    ///
    /// A procedure frame must not inherit literal bindings from its caller;
    /// `proc_depth` also enables the body-only const-map paths.  Callers that
    /// have an explicit iRules placement supply it here, so all three pieces
    /// of procedure-frame setup are balanced together.
    fn in_procedure_frame<T>(
        &mut self,
        context: Option<IrulesExecutionContext>,
        lower: impl FnOnce(&mut Self) -> T,
    ) -> T {
        self.proc_depth += 1;
        self.const_map_stack.push(HashMap::new());
        let previous_context =
            context.map(|context| std::mem::replace(&mut self.irules_execution_context, context));
        let result = lower(self);
        if let Some(previous) = previous_context {
            self.irules_execution_context = previous;
        }
        self.const_map_stack.pop();
        self.proc_depth -= 1;
        result
    }

    /// Lower a source-text literal into a [`Script`] without
    /// installing it as the module top-level. Used by passes that
    /// need to lower a sub-script (e.g. the
    /// [`crate::inline_uplevel`] rewriter materialising a
    /// brace-literal callsite body).
    pub fn lower_into_script(&mut self, source: &str, namespace: &str) -> Script {
        self.lower_script(source, namespace, SourceSegmentationPurpose::NestedScript)
    }

    /// Lower materialised text under an invocation-time binding proof, retaining
    /// its original document offsets and actual caller activation.
    pub(crate) fn lower_into_script_with_bindings(
        &mut self,
        source: &str,
        base: u32,
        namespace: &str,
        bindings: SourceCommandBindings,
    ) -> Script {
        self.lower_into_script_with_boxed_bindings(
            source,
            base,
            namespace,
            Box::new(bindings.into()),
        )
    }

    fn lower_into_script_with_boxed_bindings(
        &mut self,
        source: &str,
        base: u32,
        namespace: &str,
        bindings: Box<RetainedSourceBodyInventory>,
    ) -> Script {
        let mut positioned_source = " ".repeat(base as usize);
        positioned_source.push_str(source);
        let enclosing_source = std::mem::replace(&mut self.source, positioned_source);
        let enclosing_bindings = self.exchange_boxed_source_bindings(Some(bindings));
        let result = self.lower_body(source, base, namespace);
        drop(self.exchange_boxed_source_bindings(enclosing_bindings));
        self.source = enclosing_source;
        result
    }

    // Whole inventories are constructed and restored outside recursive frames.
    #[inline(never)]
    fn exchange_boxed_source_bindings(
        &mut self,
        bindings: Option<Box<RetainedSourceBodyInventory>>,
    ) -> Option<Box<RetainedSourceBodyInventory>> {
        std::mem::replace(
            &mut self.source_bindings,
            bindings.map(|bindings| *bindings),
        )
        .map(Box::new)
    }

    #[inline(never)]
    fn selected_source_bindings_boxed_in_context(
        &self,
        source: &crate::command_binding::ExecutedScriptSource,
        namespace: &crate::command_binding::SourceNamespaceKey,
    ) -> Option<Box<RetainedSourceBodyInventory>> {
        self.source_bindings
            .as_ref()?
            .selected_source_in_context(source, namespace)
            .map(Box::new)
    }

    /// Lower an original body in its retained namespace incarnation. The
    /// presentation label is independent of command lookup and replay context.
    fn lower_executed_script_in_context(
        &mut self,
        source: crate::command_binding::ExecutedScriptSource,
        namespace: &crate::command_binding::SourceNamespaceKey,
    ) -> Script {
        #[cfg(any(test, debug_assertions))]
        let started = tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
            .is_some()
            .then(std::time::Instant::now);
        #[cfg(any(test, debug_assertions))]
        let trace = |phase: &str| {
            if let Some(started) = started {
                eprintln!(
                    "LOWER_ORIGINAL_BODY_PHASE bytes={} base={} phase={phase} scope=body-inclusive ms={}",
                    source.text.len(),
                    source.base(),
                    started.elapsed().as_millis()
                );
            }
        };
        let (Ok(text), Some(bindings)) = (
            source.try_text(),
            self.selected_source_bindings_boxed_in_context(&source, namespace),
        ) else {
            #[cfg(any(test, debug_assertions))]
            trace("unavailable-projection");
            let mut script = unavailable_original_body_script(
                source.base(),
                source.text.len(),
                "original body source admission is unavailable",
            );
            script.executed_source = Some(std::sync::Arc::new(source));
            script.namespace_context = Some(Box::new(namespace.clone()));
            return script;
        };
        #[cfg(any(test, debug_assertions))]
        trace("selected-projection");
        let implicit_math_invocations =
            bindings.math_invocations_for_script(self.registry, &source);
        #[cfg(any(test, debug_assertions))]
        trace("math-inventory");
        let expression_preparations = bindings.expression_preparations_for_script(&source);
        #[cfg(any(test, debug_assertions))]
        trace("expression-inventory");
        let label = namespace.display().unwrap_or_default();
        let enclosing_channel = std::mem::replace(&mut self.source_channel, source.text.channel());
        // Keep the authentic pre-selection owner. A namespace projection memo
        // can filter observations but cannot issue written source geometry.
        let word_source = self.source_bindings.as_ref().and_then(|owner| {
            let input = self
                .invocation_metadata_context()?
                .source_analysis_input()?;
            match self.structured_source_parent.as_deref() {
                Some(parent) => owner.original_body_written_word_owner_at_parent(
                    &source,
                    self.config,
                    self.registry,
                    input,
                    parent,
                ),
                None => owner.original_body_written_word_owner(
                    &source,
                    self.config,
                    self.registry,
                    input,
                ),
            }
        });
        let enclosing_word_source =
            std::mem::replace(&mut self.original_body_word_source, word_source);
        let mut script =
            self.lower_into_script_with_boxed_bindings(text, source.base(), &label, bindings);
        self.original_body_word_source = enclosing_word_source;
        #[cfg(any(test, debug_assertions))]
        trace("lowered");
        self.source_channel = enclosing_channel;
        script.implicit_math_invocations = implicit_math_invocations;
        script.expression_preparations = expression_preparations;
        script.executed_source = Some(std::sync::Arc::new(source));
        script.namespace_context = Some(Box::new(namespace.clone()));
        script
    }

    /// Project an original body from the same actual Module and parent vector.
    /// Only its retained child source and entry namespace select lowering.
    /// This readonly Script provides no new native compiler, frame or Normal
    /// receipt; absent child/origin/stage correspondence is terminal.
    pub(crate) fn original_body_for_module(
        module: &Module,
        registry: &'r CommandRegistry,
        parent: &CommandTokens,
        argument: usize,
    ) -> Option<Script> {
        // naming.source.original-collection-body-owner
        // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
        #[cfg(test)]
        let trace_enabled =
            tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some();
        #[cfg(test)]
        let trace = |stage: &str, passed: bool| {
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some() {
                let offset = parent.argv.first().map(|span| span.start());
                eprintln!("ORIGINAL_FACTORY_GUARD stage={stage} offset={offset:?} passed={passed}");
            }
        };
        let metadata =
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, module);
        #[cfg(test)]
        if trace_enabled {
            trace("original_body.module_metadata", metadata.is_some());
        }
        metadata?;
        let input = module.source_metadata_input.as_ref()?;
        #[cfg(test)]
        if trace_enabled {
            trace(
                "original_body.retained",
                module.retained_source_bindings.is_some(),
            );
        }
        let retained = module.retained_source_bindings.as_deref()?;
        let bindings = retained.positioned_for_module(module, registry);
        #[cfg(test)]
        if trace_enabled {
            trace("original_body.positioned", bindings.is_some());
        }
        let bindings = bindings?;
        #[cfg(test)]
        if trace_enabled {
            trace(
                "original_body.parent_vector",
                retained.owns_original_tokens(parent),
            );
        }
        if !retained.owns_original_tokens(parent) {
            return None;
        }
        let body_word = parent.argv.get(argument.checked_add(1)?)?;
        let source = bindings.executed_script_for_word(*body_word);
        #[cfg(test)]
        if trace_enabled {
            trace("original_body.selected_script", source.is_some());
        }
        let source = source?.clone();
        source.try_text().ok()?;
        let mut lowerer = Self::with_config(registry, module.lexer_config)
            .with_resolved_analysis_input(input.clone());
        lowerer.set_source_analysis_options(module.source_entry.options());
        lowerer.source_bindings = Some(bindings.clone().into());
        let namespace = lowerer.original_body_namespace_context(parent, argument, &source);
        #[cfg(test)]
        if trace_enabled {
            trace("original_body.namespace", namespace.is_some());
        }
        let namespace = namespace?;
        // Preserve the independently selected child stage and namespace issuer
        // before delegating the same original-body semantic owner.
        let selected = lowerer
            .source_bindings
            .as_ref()?
            .selected_source_in_context(&source, &namespace);
        #[cfg(test)]
        if trace_enabled {
            trace("original_body.stage", selected.is_some());
        }
        selected?;
        Some(lowerer.lower_original_body(source, Some(&namespace)))
    }

    /// Readonly Logical child lowering under the actual complete Module owner.
    /// Original body geometry and selected current/root source namespace stay
    /// independent of reached body-entry and Native compilation receipts.
    pub(crate) fn original_logical_body_for_module(
        module: &Module,
        registry: &'r CommandRegistry,
        parent: &CommandTokens,
        argument: usize,
    ) -> Option<Script> {
        let input = module.source_metadata_input.as_ref()?;
        if !input.has_logical_source_name_context() {
            return None;
        }
        let retained = module.retained_source_bindings.as_deref()?;
        let positioned = retained.positioned_for_module(module, registry)?;
        let body = crate::registry_invocation::original_logical_body_geometry_for_module(
            module, registry, parent, argument,
        )?;
        let namespace = crate::command_binding::original_logical_frame_body_namespace_for_module(
            module, registry, parent, &body,
        )?;
        let source = module.source.try_text().ok()?;
        let text = source.get(body.content_span().as_range())?;
        let mut lowerer = Self::with_config(registry, module.lexer_config)
            .with_resolved_analysis_input(input.clone());
        lowerer.set_source_analysis_options(module.source_entry.options());
        source.clone_into(&mut lowerer.source);
        lowerer.source_channel = module.source.channel();
        lowerer.source_bindings = Some(positioned.clone().into());
        lowerer.structured_source_parent = Some(Box::new(parent.clone()));
        lowerer.suppress_proc_register = true;
        lowerer.readonly_logical_body = SourceBodyMode::ReadOnly;
        let mut script = lowerer.lower_body(
            text,
            body.content_span().start(),
            &namespace.advisory_key()?,
        );
        script.namespace_context = Some(Box::new(namespace));
        Some(script)
    }

    /// Retain an original body without inventing a namespace when its entry
    /// context is unknown. Known contexts survive unsupported text/admission.
    fn lower_original_body(
        &mut self,
        source: crate::command_binding::ExecutedScriptSource,
        namespace: Option<&crate::command_binding::SourceNamespaceKey>,
    ) -> Script {
        if let Some(namespace) = namespace {
            return self.lower_executed_script_in_context(source, namespace);
        }
        let mut script = unavailable_original_body_script(
            source.base(),
            source.text.len(),
            "original body namespace context is unavailable",
        );
        script.executed_source = Some(std::sync::Arc::new(source));
        script
    }

    fn original_body_namespace_context(
        &self,
        tokens: &CommandTokens,
        argument: usize,
        source: &crate::command_binding::ExecutedScriptSource,
    ) -> Option<crate::command_binding::SourceNamespaceKey> {
        let bindings = self.source_bindings.as_ref()?;
        bindings.executed_script_entry_namespace_key_at(
            tokens.source_binding.as_ref()?.invocation_site()?,
            argument,
            source,
        )
    }

    /// Original retained contexts alone select a body inventory when its
    /// caller has no separate invocation/argument context receipt.
    fn lower_retained_script(
        &mut self,
        source: crate::command_binding::ExecutedScriptSource,
    ) -> Script {
        let namespace = self.source_bindings.as_ref().and_then(|bindings| {
            bindings
                .executed_script_namespace_context(&source)
                .or_else(|| bindings.compiled_script_namespace_context(&source))
                .or_else(|| bindings.declared_script_namespace_context(&source))
                .or_else(|| {
                    let parent = self.structured_source_parent.as_deref()?;
                    parent
                        .source_binding
                        .as_ref()?
                        .original_body_namespace_context(
                            self.registry,
                            self.invocation_metadata_context()?,
                            parent,
                            &source,
                        )
                })
        });
        self.lower_original_body(source, namespace.as_ref())
    }

    /// Retain an unchanged body slice independently of compiler admission.
    fn retain_script_source(&self, mut script: Script, text: &str, base: u32) -> Script {
        if self.readonly_logical_body.is_read_only()
            || self.readonly_hosted_body.is_read_only()
            || self.readonly_authored_body.is_read_only()
            || self.readonly_native_declaration_body.is_read_only()
        {
            return script;
        }
        script.executed_source = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.source_origin())
            .cloned()
            .and_then(|origin| {
                crate::command_binding::ExecutedScriptSource::contiguous(origin, text, base)
            })
            .map(std::sync::Arc::new);
        if let Some(source) = &script.executed_source {
            script.implicit_math_invocations = self
                .source_bindings
                .as_ref()
                .map_or_else(Vec::new, |bindings| {
                    bindings.math_invocations_for_script(self.registry, source)
                });
            script.expression_preparations = self
                .source_bindings
                .as_ref()
                .map_or_else(Vec::new, |bindings| {
                    bindings.expression_preparations_for_script(source)
                });
        }
        script
    }

    /// Supply the execution entry contract before lowering. A previously used
    /// interpreter must pass `unknown_entry`; package advertisements alone do
    /// not establish loader implementation identity.
    pub fn set_source_analysis_options(&mut self, options: SourceAnalysisOptions<'_>) {
        // Explicit replacement withdraws factory intent, even for equal options.
        self.supplied_source = None;
        let options = if options.source_point_entry.is_some_and(|point| {
            point
                .child_state(
                    self.registry,
                    options,
                    Some(options.native_lexer_config(self.config)),
                )
                .is_none()
        }) {
            SourceAnalysisOptions {
                source_point_entry: options.source_point_entry,
                analyser_source_settings: options.analyser_source_settings,
                metadata_context:
                    crate::registry_invocation::InvocationMetadataInput::SuppliedSource(None),
                unknown_entry: true,
                host_extensible_command_table: options.host_extensible_command_table,
                ..SourceAnalysisOptions::default()
            }
        } else {
            options
        };
        self.source_point_entry = options.source_point_entry.cloned();
        self.analyser_source_settings = options.analyser_source_settings.cloned();
        self.compilation_scope = options.compilation_scope;
        self.invocation_realm = options.invocation_realm;
        self.incoming_formals = options.incoming_formals.to_vec();
        self.source_declared_commands = options.declared_commands.cloned();
        self.trusted_package_loaders = options.trusted_package_loaders.to_vec();
        self.trusted_source_modules = options.trusted_source_modules.to_vec();
        self.unknown_entry = options.unknown_entry;
        self.host_command_table = HostCommandTable::of(options.host_extensible_command_table);
        self.source_entry_origin = SourceEntryOrigin::Driver;
        self.config = options.native_lexer_config(self.config);
        self.invocation_dialect = options.source_invocation_dialect(self.config);
        self.native_compilation = options.native_compilation;
        self.compiled_variable_provider = options.compiled_variable_provider;
        self.expression_parser = options.expression_parser;
        self.hosted_execution_context = options.hosted_execution_context;
        self.execution_name_policy = options.execution_name_policy();
        self.logical_source_input = options.logical_source_input.cloned();
        self.vendor_source_input = options.vendor_source_input.cloned();
        if let Some(input) = self
            .logical_source_input
            .as_ref()
            .or(self.vendor_source_input.as_ref())
        {
            self.dialect = self.dialect.or(Some(input.unit_profile()));
            self.dialect_context = Some(input.context_registry());
            self.metadata_origin = LoweringMetadataOrigin::Supplied;
        } else if self.metadata_origin == LoweringMetadataOrigin::Standalone {
            self.dialect_context = None;
        }
        self.native_entry = options
            .native_entry
            .map(|entry| std::sync::Arc::new(entry.clone()));
    }

    /// Retain the original entry channel for checked structural lowering.
    /// The module and source-origin owners retain the image bytes separately;
    /// this policy never grants native compiler admission.
    pub fn set_source_image_channel(&mut self, image: &tcl_lexer::SourceImage) {
        self.source_channel = image.channel();
    }

    fn current_source_channel(&self) -> tcl_lexer::SourceChannel {
        self.source_channel
    }

    fn segment_source(
        &self,
        text: &str,
        base: u32,
        purpose: SourceSegmentationPurpose,
    ) -> Option<Vec<SegmentedCommand>> {
        let image =
            tcl_lexer::SourceImage::from_bytes(text.as_bytes(), self.current_source_channel());
        crate::segmenter::segment_commands_image_with_offset_and_config(
            &image,
            base,
            purpose.config(self.config),
        )
    }

    /// Lower a source string to an IR script.
    ///
    /// Depth-guarded entry to the recursive lowering, alongside
    /// [`Self::lower_body`] — every nested body re-enters through one of
    /// these two, so bounding both here caps the whole `lower_script` ↔
    /// `lower_body` ↔ `lower_segmented` ↔ `lower_command` recursion. At the
    /// cap we stop descending and lower the remaining text as a single
    /// opaque [`Statement::Barrier`] rather than silently dropping it —
    /// downstream passes (SCCP, dead-code elimination, …) must treat
    /// unanalysed content as having unknown effects, not as empty/dead.
    fn lower_script(
        &mut self,
        source: &str,
        namespace: &str,
        purpose: SourceSegmentationPurpose,
    ) -> Script {
        self.nest_depth += 1;
        if MAX_LOWER_NEST_DEPTH.exceeded(self.nest_depth) {
            self.nest_depth -= 1;
            return over_depth_script(0, source.len());
        }

        // `source` starts a span space: its statements' offsets are relative
        // to it, whether it is the document or a literal materialised inside
        // one.  Swap it in for the descent and restore the enclosing one after
        // (see [`Self::source`]).
        let enclosing_source = std::mem::replace(&mut self.source, source.to_owned());
        // The selected local parser purpose is shared by binding analysis and
        // segmentation. Parent inputs/config remain unchanged for original
        // word and metadata joins; a missing body receipt cannot borrow them.
        let frame = self.source_entry_frame(namespace);
        let image = tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_channel);
        let Some(bindings) = SourceCommandBindings::analyse_image_with_original_native_formals(
            &image,
            &frame,
            self.config,
            self.registry,
            SourceAnalysisOptions {
                analyser_source_settings: self.analyser_source_settings.as_ref(),
                source_point_entry: self
                    .source_point_entry
                    .as_ref()
                    .filter(|_| self.nest_depth == 1),
                metadata_context: self.source_metadata_input(),
                hosted_execution_context: self.hosted_execution_context,
                execution_name_policy: self.execution_name_policy,
                logical_source_input: self.logical_source_input.as_ref(),
                vendor_source_input: self.vendor_source_input.as_ref(),
                compilation_scope: self.compilation_scope,
                unentered: false,
                invocation_realm: self.invocation_realm,
                declared_commands: self
                    .source_declared_commands
                    .as_ref()
                    .or(self.declared_commands),
                trusted_package_loaders: &self.trusted_package_loaders,
                trusted_source_modules: self.effective_source_modules(),
                unknown_entry: self.unknown_entry || self.nest_depth > 1,
                host_extensible_command_table: self.host_command_table.is_extensible(),
                invocation_dialect: self.invocation_dialect,
                compiled_variable_provider: self.compiled_variable_provider,
                expression_parser: self.expression_parser,
                native_entry: self.native_entry.as_deref(),
                incoming_formals: if self.nest_depth == 1 {
                    &self.incoming_formals
                } else {
                    &[]
                },
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    frame: if self.module.top_level_kind == crate::ir::TopLevelKind::ProcedureBody {
                        tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode
                    } else {
                        self.native_compilation.frame
                    },
                    ..self.native_compilation
                },
            },
            purpose,
            self.original_native_compiler_formals
                .as_ref()
                .filter(|_| self.nest_depth == 1),
        ) else {
            self.source = enclosing_source;
            self.nest_depth -= 1;
            return over_depth_script(0, source.len());
        };

        let enclosing_bindings = self.source_bindings.replace(bindings.into());
        let result = self.lower_script_inner(source, namespace, purpose);

        self.restore_script_bindings(enclosing_bindings);

        self.source = enclosing_source;
        self.nest_depth -= 1;
        result
    }

    #[inline(never)]
    fn restore_script_bindings(&mut self, enclosing: Option<RetainedSourceBodyInventory>) {
        let completed = std::mem::replace(&mut self.source_bindings, enclosing);
        if self.nest_depth == 1 {
            self.module.procedure_implementation_bodies = completed.as_ref().map_or_else(
                || std::sync::Arc::from([]),
                |bindings| {
                    bindings
                        .procedure_implementation_bodies()
                        .map(
                            |body| crate::command_binding::SourceInstalledProcedureBody {
                                allocation: body.allocation.clone(),
                                command: body.allocation.command.clone(),
                                namespace: body.namespace.to_owned(),
                                namespace_key: body.namespace_key.clone(),
                                parameters: body.parameters.to_vec(),
                                original_parameters: body.original_parameters.cloned(),
                                source: body.source.clone(),
                            },
                        )
                        .collect()
                },
            );
            self.module.future_call_sites = completed.as_ref().map_or_else(Vec::new, |bindings| {
                bindings
                    .future_body_inventories()
                    .iter()
                    .flat_map(crate::command_binding::SourceFutureBodyInventory::call_sites)
                    .collect()
            });
            self.module_source_bindings = completed.map(Box::new);
        }
    }

    /// Run `body` with the words of the commands it lowers read from `text`,
    /// segmented at `base` — see [`WordSpace`]. Restores the enclosing space
    /// after, so a nested body does not outlive its own descent.
    fn in_word_space<T>(&mut self, text: &str, base: u32, body: impl FnOnce(&mut Self) -> T) -> T {
        let channel = self.current_source_channel();
        let enclosing =
            std::mem::replace(&mut self.word_space, WordSpace::new(text, base, channel));
        let result = body(self);
        self.word_space = enclosing;
        result
    }

    fn lower_script_inner(
        &mut self,
        source: &str,
        namespace: &str,
        purpose: SourceSegmentationPurpose,
    ) -> Script {
        let Some(commands) = self.segment_source(source, 0, purpose) else {
            return over_depth_script(0, source.len());
        };

        self.const_map_stack.push(HashMap::new());
        self.command_binding_site_stack.push(Vec::new());
        let stmts =
            self.in_word_space(source, 0, |this| this.lower_segmented(&commands, namespace));

        let command_binding_sites = self
            .command_binding_site_stack
            .pop()
            .expect("the script command-binding site scope is balanced");
        self.const_map_stack.pop();
        let script = self.attach_native_compilation_failure(
            Script::from_lowered_parts(stmts, command_binding_sites),
            0,
        );

        self.retain_script_source(script, source, 0)
    }

    /// Lower a body argument (inside braces/brackets) to an IR script.
    ///
    /// Inherits the parent scope's const-map so a child body can
    /// still relax its
    /// `eval` / `uplevel` against literals bound in the enclosing
    /// scope (`set body {literal}; catch {uplevel 1 $body}` is the
    /// canonical example).
    ///
    /// Depth-guarded the same way as [`Self::lower_script`] — see its doc
    /// comment.
    fn lower_body(&mut self, text: &str, base_offset: u32, namespace: &str) -> Script {
        let context = self.nested_irules_execution_context();
        self.lower_body_in_irules_context(text, base_offset, namespace, context)
    }

    /// Lower a body while explicitly preserving the registry-owned iRules
    /// execution context.  `when` / `proc` declarations use this to install
    /// their event/procedure frame; ordinary script-bearing commands use
    /// [`Self::lower_body`], which derives the nested context instead.
    fn lower_body_in_irules_context(
        &mut self,
        text: &str,
        base_offset: u32,
        namespace: &str,
        context: IrulesExecutionContext,
    ) -> Script {
        let previous = std::mem::replace(&mut self.irules_execution_context, context);
        self.nest_depth += 1;
        let result = if MAX_LOWER_NEST_DEPTH.exceeded(self.nest_depth) {
            self.nest_depth -= 1;
            over_depth_script(base_offset, text.len())
        } else {
            let result = self.lower_body_inner(text, base_offset, namespace);
            self.nest_depth -= 1;
            result
        };
        self.irules_execution_context = previous;
        result
    }

    /// Which iRules placement a nested script inherits.  A control-flow or
    /// callback body within an event/procedure keeps that live frame; a body
    /// reached from a top-level executable command remains invalid rather
    /// than gaining the power to declare a handler or procedure.
    const fn nested_irules_execution_context(&self) -> IrulesExecutionContext {
        match self.irules_execution_context {
            IrulesExecutionContext::TopLevel | IrulesExecutionContext::InvalidNestedBody => {
                IrulesExecutionContext::InvalidNestedBody
            }
            IrulesExecutionContext::EventBody => IrulesExecutionContext::EventBody,
            IrulesExecutionContext::ProcedureBody => IrulesExecutionContext::ProcedureBody,
        }
    }

    /// Run one synthesised script in a known iRules execution context.  A
    /// materialised proc body has no source token to route through
    /// [`Self::lower_body_in_irules_context`], but it still must not treat a
    /// nested declaration as a file-level one.
    fn lower_script_in_irules_context(
        &mut self,
        source: &str,
        namespace: &str,
        context: IrulesExecutionContext,
    ) -> Script {
        let previous = std::mem::replace(&mut self.irules_execution_context, context);
        let result = self.lower_script(source, namespace, SourceSegmentationPurpose::NestedScript);
        self.irules_execution_context = previous;
        result
    }

    /// The part of a body word's `text` whose spans may honestly be rebased
    /// by that word's content offset, given the word occupies `tok`'s span in
    /// [`Self::source`].
    ///
    /// Every body lowering below adds one base offset to the spans the
    /// segmenter cuts from a word *value*, which is truthful only while that
    /// value is the source region verbatim.  A compound `{body}x` word breaks
    /// it: the value is the brace content welded to the tail with the closing
    /// `}` dropped, so every token past the drop slides one byte left — an
    /// off-by-one span on ASCII, an offset inside a UTF-8 sequence on
    /// anything else.  [`crate::segmenter::body_text_in_region`]
    /// clamps such a value to the contiguous braced part and passes an
    /// ordinary body through untouched; the analyser's `analyse_body`
    /// applies it too, and this is the lowering side of the same guard.
    ///
    /// A substituting word (`$body`, `[gen]`) is left alone: its value is
    /// run-time data that no source region can be compared against, so there
    /// is nothing here to prove and clamping it would only delete text.  Such
    /// words are the literal gates' business — they barrier rather than
    /// lower.
    ///
    /// A translating Document channel folds CRLF in a braced word's cooked
    /// value while its spans stay original offsets, so a braced body maps
    /// back to its source region first
    /// ([`crate::segmenter::braced_body_source_region`]).
    fn guarded_body_text(&self, tok: tcl_lexer::Token, text: &str) -> String {
        if matches!(tok.kind, TokenType::Var | TokenType::Cmd) {
            return text.to_owned();
        }
        let base = (tok.span.start() + u32::from(tok.content_offset)) as usize;
        let text = crate::segmenter::braced_body_source_region(
            &self.source,
            base,
            tok.span.end().saturating_sub(1) as usize,
            text,
        );
        crate::segmenter::body_text_in_region(&self.source, base, tok.span.end() as usize, text)
            .to_owned()
    }

    fn lower_body_inner(&mut self, text: &str, base_offset: u32, namespace: &str) -> Script {
        let Some(commands) =
            self.segment_source(text, base_offset, SourceSegmentationPurpose::NestedScript)
        else {
            return over_depth_script(base_offset, text.len());
        };

        let inherited = self.const_map_stack.last().cloned().unwrap_or_default();
        self.const_map_stack.push(inherited);
        self.command_binding_site_stack.push(Vec::new());
        let stmts = self.in_word_space(text, base_offset, |this| {
            this.lower_segmented(&commands, namespace)
        });

        let command_binding_sites = self
            .command_binding_site_stack
            .pop()
            .expect("the body command-binding site scope is balanced");
        self.const_map_stack.pop();
        let script = self.attach_native_compilation_failure(
            Script::from_lowered_parts(stmts, command_binding_sites),
            base_offset,
        );

        self.retain_script_source(script, text, base_offset)
    }

    /// Retain only a failure agreed by every reached compilation entry.
    fn attach_native_compilation_failure(&self, mut script: Script, chunk: u32) -> Script {
        if self.readonly_logical_body.is_read_only()
            || self.readonly_hosted_body.is_read_only()
            || self.readonly_authored_body.is_read_only()
            || self.readonly_native_declaration_body.is_read_only()
        {
            return script;
        }
        script.native_compilation_failure = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.native_compilation_failure_at(chunk))
            .cloned()
            .map(Box::new);
        if let Some(bindings) = &self.source_bindings
            && (bindings.native_compilation_provider_required_at(chunk)
                || script.native_compilation_failure.is_some())
        {
            script.native_compilation_admission = Some(std::sync::Arc::new(
                crate::native_compilation_admission::NativeCompilationAdmission {
                    source: bindings.native_compilation_source_at(chunk).cloned(),
                    failure: script
                        .native_compilation_failure
                        .as_deref()
                        .cloned()
                        .map(std::sync::Arc::new),
                    provider_required: true,
                },
            ));
        }
        script
    }

    /// Lower a list of segmented commands to IR statements.
    fn lower_segmented(
        &mut self,
        segments: &[SegmentedCommand],
        namespace: &str,
    ) -> Vec<Statement> {
        let mut stmts = Vec::new();
        for seg in segments {
            if seg.is_partial {
                Self::push_partial_command(&mut stmts, seg.span);
                continue;
            }
            if let Some(stmt) = self.lower_command(seg, namespace) {
                self.update_const_map(seg, &stmt);
                Self::push_boxed_statement(&mut stmts, stmt);
            }
        }
        self.debug_assert_spans_in_source(&stmts);
        stmts
    }

    // Materialisation is deliberately outside the recursive descent frame.
    #[inline(never)]
    fn push_partial_command(statements: &mut Vec<Statement>, span: tcl_lexer::Span) {
        statements.push(Statement::Barrier {
            span,
            reason: "incomplete command".into(),
            command: String::new(),
            canonical_command: None,
            args: vec![],
            tokens: None,
        });
    }

    #[inline(never)]
    fn push_boxed_statement(statements: &mut Vec<Statement>, statement: Box<Statement>) {
        statements.push(*statement);
    }

    /// Every statement leaving [`Self::lower_segmented`] must carry a span
    /// that is a real range of the buffer it was lowered from — the contract
    /// downstream consumers rely on when they slice the source with an IR
    /// span (AGENTS.md: "trust it rather than re-deriving").
    ///
    /// This is the producer-side chokepoint: every statement the lowering
    /// emits passes through here, at the nesting level whose segments were
    /// cut from [`Self::source`], so a body rebase that overshoots its
    /// document (a compound `{body}x` word, a non-literal body word)
    /// fails the build's own tests instead of
    /// reaching a consumer.  Debug-only: the guards in
    /// [`Self::lower_body_from_tok`] and codegen's slice clamp are what keep
    /// a release build safe.
    fn debug_assert_spans_in_source(&self, stmts: &[Statement]) {
        debug_assert!(
            self.source.is_empty()
                || stmts
                    .iter()
                    .all(|stmt| self.source.get(stmt.span().as_range()).is_some()),
            "lowering emitted a span outside its {} byte source: {:?}",
            self.source.len(),
            stmts
                .iter()
                .map(Statement::span)
                .filter(|span| self.source.get(span.as_range()).is_none())
                .collect::<Vec<_>>(),
        );
    }

    /// Maintain the per-scope const-map after a command lowers.
    ///
    /// Retain a quiet Logical scalar assignment's original literal value
    /// under its decoded local name; unclosed writes withdraw retained entries.
    /// Gated on `proc_depth > 0` — globals / namespace vars at
    /// top-level or inside `namespace eval` cannot be safely
    /// const-propagated.
    ///
    /// Kept out of [`Self::lower_segmented`]'s frame: its carrier locals are
    /// several KiB, and that frame recurs once per nested body level.
    #[inline(never)]
    fn update_const_map(&mut self, seg: &SegmentedCommand, stmt: &Statement) {
        if self.proc_depth == 0 {
            return;
        }
        let quiet_store = if matches!(stmt, Statement::AssignConst { .. }) {
            let tokens = self.cmd_tokens(seg);
            tokens.source_binding.as_ref().and_then(|binding| {
                binding.original_conditional_logical_scalar_store(
                    &tokens,
                    self.invocation_metadata_context()?,
                    self.registry,
                )
            })
        } else {
            None
        };
        let literal = assigned_local_body_literal(seg, stmt, self.config).filter(|(_, value)| {
            quiet_store
                .as_ref()
                .is_some_and(|store| store.value() == Some(value.as_str()))
        });
        let Some(scope) = self.const_map_stack.last_mut() else {
            return;
        };
        if let Some((name, value)) = literal {
            scope.insert(name, value);
        } else if matches!(stmt, Statement::AssignConst { .. }) && quiet_store.is_none() {
            scope.clear();
        } else {
            invalidate_const_map_for(stmt, scope, self.config);
        }
    }

    fn original_native_written_argument_value(
        &self,
        seg: &SegmentedCommand,
        argument: usize,
    ) -> Option<String> {
        self.native_entry.as_ref()?;
        let tokens = self.cmd_tokens(seg);
        let binding = tokens.source_binding.as_ref()?;
        (binding.original_invocation_source_purpose(
            &tokens,
            self.invocation_metadata_context()?,
            self.registry,
        )? == crate::command_binding::OriginalInvocationSourcePurpose::Execution)
            .then_some(())?;
        binding
            .evaluated_written_argument_value(argument)
            .map(str::to_owned)
    }

    fn original_authored_written_argument_value(
        &self,
        seg: &SegmentedCommand,
        argument: usize,
    ) -> Option<String> {
        let tokens = self.cmd_tokens(seg);
        tokens
            .source_binding
            .as_ref()?
            .original_authored_written_argument_value(
                &tokens,
                argument,
                self.invocation_metadata_context()?,
                self.registry,
            )
            .map(str::to_owned)
    }

    /// Read a simple body reference from the quiet Logical local-value map.
    /// Unknown, array and compound references retain runtime dispatch.
    fn const_map_lookup(&self, word: &str) -> Option<String> {
        if self.proc_depth == 0 {
            return None;
        }
        let name = tcl_syntax::naming::whole_scalar_variable_reference_name(word, self.config)?;
        self.const_map_stack.last()?.get(name).cloned()
    }

    fn logical_structured_invocation(
        &self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::SourceStructuredInvocation> {
        if self.allows_standalone_registry_metadata() {
            return crate::registry_invocation::original_authored_control_invocation(
                tokens,
                self.registry,
            )
            .or_else(|| {
                crate::registry_invocation::original_authored_scalar_invocation(
                    tokens,
                    self.registry,
                )
                .map(|selected| {
                    crate::registry_invocation::SourceStructuredInvocation::Authored(Box::new(
                        selected,
                    ))
                })
            });
        }
        let metadata = self.invocation_metadata_context()?;
        if self.readonly_native_declaration_body.is_read_only() {
            return crate::registry_invocation::original_native_source_structured_invocation(
                self.registry,
                metadata,
                tokens,
            );
        }
        if metadata
            .source_analysis_input()
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_hosted_source_name_context)
        {
            return crate::registry_invocation::original_hosted_structured_invocation(
                self.registry,
                metadata,
                tokens,
            );
        }
        crate::registry_invocation::logical_structured_invocation_with_metadata_context(
            self.registry,
            self.invocation_metadata_context()?,
            tokens,
            self.source_bindings.as_deref(),
        )
        .map(|selected| {
            crate::registry_invocation::SourceStructuredInvocation::Logical(Box::new(selected))
        })
        .or_else(|| {
            crate::registry_invocation::original_logical_source_structured_invocation(
                self.registry,
                metadata,
                tokens,
            )
        })
        .or_else(|| {
            if self.target.is_bytecode() {
                return None;
            }
            crate::registry_invocation::original_native_source_structured_invocation(
                self.registry,
                metadata,
                tokens,
            )
        })
    }

    /// Build a `CommandTokens` snapshot from a segmented command, decomposing
    /// each word under the document's own grammar (`self.config`) from the
    /// text that was segmented into it (see [`WordSpace`]).
    fn cmd_tokens(&self, seg: &SegmentedCommand) -> CommandTokens {
        let mut tokens = CommandTokens::from_segmented(&self.word_space.map(), self.config, seg);
        if self.readonly_authored_body.is_read_only() {
            if let Some(scope) = &self.authored_source_scope {
                let mut scalar = None;
                if let Some(bindings) = &self.source_bindings
                    && let Some(mut declared) = bindings
                        .original_authored_declaration_source_tokens(&tokens, scope, self.registry)
                {
                    let original = declared.clone();
                    let retained = declared.source_binding.as_mut().and_then(|binding| {
                        binding.retain_original_authored_scalar_body_scope(
                            std::sync::Arc::clone(scope),
                            &original,
                        )
                    });
                    if retained.is_some()
                        && crate::registry_invocation::original_authored_scalar_invocation(
                            &declared,
                            self.registry,
                        )
                        .is_some()
                    {
                        scalar = Some(declared);
                    }
                }
                let original = tokens.clone();
                let _ = tokens
                    .source_binding
                    .get_or_insert_with(Default::default)
                    .retain_original_authored_source_body_scope(
                        std::sync::Arc::clone(scope),
                        &original,
                    );
                if let Some(declared) = scalar {
                    tokens = declared;
                }
            }
        } else if let Some(bindings) = &self.source_bindings {
            bindings.stamp_original_tokens(&mut tokens);
            if let (Some(body), Some(metadata)) = (
                self.original_body_word_source.as_ref(),
                self.invocation_metadata_context(),
            ) && let Some(input) = metadata.source_analysis_input()
            {
                let _ = body.stamp_original_tokens(self.registry, input, &mut tokens);
            }
        }
        if let Some(owner) = &self.native_source_children {
            let _ = owner.stamp_tokens(&mut tokens);
        }
        // Implementation contract: naming.consumer.hosted-source-taint-descriptor
        // docs/design/analysis/name-resolution-proofs/hosted-source-taint-descriptor.md
        // The invocation owns the original whole image. A retained body's
        // positioned lowering buffer can contain padding before this command;
        // those bytes are not the source owner of conditional hosted metadata.
        tokens.hosted_taint_context = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.invocation_site())
            .zip(self.dialect_context.as_ref())
            .and_then(|(site, context)| {
                crate::registry_invocation::HostedSourceTaintContext::from_lowering(
                    site.source.source_image().clone(),
                    self.config,
                    std::sync::Arc::clone(context),
                )
                .map(|owner| {
                    owner.retain_source_structure_input(
                        self.invocation_metadata_context()
                            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input),
                    )
                })
            })
            .map(std::sync::Arc::new);
        tokens
    }

    /// Construct the full source carrier outside recursive lowering frames.
    #[inline(never)]
    fn cmd_tokens_boxed(&self, seg: &SegmentedCommand) -> Box<CommandTokens> {
        Box::new(self.cmd_tokens(seg))
    }

    /// Extract arg token kinds for the lowering hooks.
    fn arg_kinds(seg: &SegmentedCommand) -> Vec<ArgTokenKind> {
        seg.argv
            .iter()
            .skip(1)
            .map(|t| arg_token_kind(t.kind))
            .collect()
    }

    /// Collect proved handler directives as metadata, independently of opcode
    /// eligibility. Dead branches and unknown operands supply no literal row.
    fn record_namespace_directives(&mut self, tokens: &CommandTokens, namespace: &str) {
        use tcl_registry::{NamespaceTransition, NamespaceTransitionTarget, TransitionSubject};
        if self.dead_code_depth != 0 {
            return;
        }
        let footprint = match self.invocation_metadata_context() {
            Some(context) => {
                crate::registry_invocation::namespace_directive_footprint_with_metadata_context(
                    self.registry,
                    Some(context),
                    tokens,
                )
            }
            None if self.allows_standalone_registry_metadata() => {
                crate::registry_invocation::namespace_directive_footprint(
                    self.registry,
                    None,
                    tokens,
                )
            }
            None => None,
        };
        let Some(footprint) = footprint else { return };
        let namespace = tokens
            .source_binding
            .as_ref()
            .map_or(namespace, |binding| binding.lookup_namespace.as_str());
        if let Some(site) = tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::invocation_site)
            && !self
                .namespace_directive_sites
                .insert((site.clone(), namespace.to_owned()))
        {
            return;
        }
        for directive in footprint.transitions() {
            match directive {
                NamespaceTransition::Import {
                    namespace: NamespaceTransitionTarget::Current,
                    patterns,
                    ..
                } => {
                    for pattern in patterns.iter().filter_map(TransitionSubject::literal) {
                        self.namespace_imports
                            .push((namespace.to_owned(), pattern.to_owned()));
                    }
                }
                NamespaceTransition::Export {
                    namespace: NamespaceTransitionTarget::Current,
                    patterns,
                } => {
                    let Some((_, patterns)) =
                        NamespaceTransition::export_pattern_operands(patterns)
                    else {
                        continue;
                    };
                    self.namespace_exports.extend(
                        patterns
                            .iter()
                            .filter_map(TransitionSubject::literal)
                            .map(|pattern| (namespace.to_owned(), pattern.to_owned())),
                    );
                }
                _ => {}
            }
        }
    }

    /// Whether a resolved lowering hook commits to a **positional argv
    /// shape** — its lowerer reads "the body is word N", "the loop variables
    /// are word 0", "the result variable is word 1" — so a `{*}` expansion
    /// among the words makes the shape unknowable and the call must barrier
    /// instead.
    ///
    /// Registry-derived by construction: the answer is keyed on the typed
    /// [`LoweringHookId`] that [`Self::try_dispatch_structured_hook`] already
    /// dispatches on, not on a command-name list: a name list drifts against
    /// the structured hook IDs, letting a form such as `lmap`, `dict for`,
    /// `array for`, `foreachLine`, `catch`, `try`, `eval`, `uplevel` or
    /// `apply` commit to its un-expanded argv.  The match is exhaustive with
    /// no wildcard arm, so a new hook ID cannot be added without classifying
    /// it here.
    ///
    /// The `false` arm is the non-structured group: those hooks run in
    /// [`try_lower_hook`] before this dispatcher is reached, and each already
    /// declines an expanded call itself via `has_expansion`.
    fn hook_commits_to_argv_shape(hook: LoweringHookId) -> bool {
        match hook {
            LoweringHookId::Proc
            | LoweringHookId::When
            | LoweringHookId::NamespaceEval
            | LoweringHookId::If
            | LoweringHookId::Switch
            | LoweringHookId::For
            | LoweringHookId::While
            | LoweringHookId::Foreach
            | LoweringHookId::Lmap
            | LoweringHookId::ForeachLine
            | LoweringHookId::Catch
            | LoweringHookId::Try
            | LoweringHookId::Dict
            | LoweringHookId::Eval
            | LoweringHookId::Uplevel
            | LoweringHookId::Apply
            | LoweringHookId::ArrayFor => true,
            LoweringHookId::Expr
            | LoweringHookId::Return
            | LoweringHookId::Set
            | LoweringHookId::Incr
            | LoweringHookId::AppendOrLappend
            | LoweringHookId::Unset
            | LoweringHookId::Global
            | LoweringHookId::Variable
            | LoweringHookId::Upvar => false,
        }
    }

    /// A proved builtin whose lowering commits to its argv shape keeps the
    /// opaque runtime barrier when a `{*}` word leaves no structured
    /// selection: the expanded argv decides what the command runs.
    fn expanded_structured_barrier(
        &self,
        seg: &SegmentedCommand,
        tokens: &CommandTokens,
    ) -> Option<Box<Statement>> {
        if self.target.is_trace_visible()
            || !seg
                .expand_word
                .as_ref()
                .is_some_and(|expanded| expanded.iter().any(|&e| e))
        {
            return None;
        }
        let target = tokens
            .source_binding
            .as_ref()?
            .proved_execution_target()
            .filter(|target| target.registry_backed && target.prepended.is_empty())?;
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let hook = self
            .registry
            .resolve_call(&target.command, &args, self.registry.own_surface_query())?
            .lowering_hook?;
        Self::hook_commits_to_argv_shape(hook)
            .then(|| Box::new(self.structured_expand_barrier(seg.name(), seg.args(), seg)))
    }

    /// `{*}` expansion on a structured command lowers to a barrier so
    /// downstream analyses can't reason about the expanded form.
    fn structured_expand_barrier(
        &self,
        cmd_name: &str,
        args: &[String],
        seg: &SegmentedCommand,
    ) -> Statement {
        Statement::Barrier {
            span: seg.span,
            reason: format!("{cmd_name} with argument expansion"),
            command: cmd_name.into(),
            canonical_command: None,
            args: args.to_vec(),
            tokens: Some(self.cmd_tokens(seg)),
        }
    }

    /// Try to dispatch *`cmd_name`* through the registry's typed
    /// [`LoweringHookId`] for structured commands dispatched from
    /// [`lower_command`](Self::lower_command).  Returns
    /// `Some(stmt)` when the hook ID matches a structured form;
    /// returns `None` otherwise so `lower_command` falls back to
    /// [`lower_default`](Self::lower_default).
    ///
    /// Every structured form routes through
    /// `resolve_call().lowering_hook`, and unmatched commands flow
    /// straight to `lower_default`.  Forms with shape preconditions
    /// (`proc`, `namespace eval`, `foreach`, `lmap`, `dict`,
    /// `when`, `foreachLine`) return `None` on a failed
    /// precondition so `lower_default` catches them.
    ///
    /// Dispatching through the hook ID picks up two benefits over a
    /// bare name match:
    ///
    /// 1. Canonical resolution via [`CommandRegistry::resolve_call`]
    ///    instead of bare name comparison, so any future spec
    ///    that aliases an existing form (e.g. an iRules-specific
    ///    `eval` variant) automatically dispatches correctly
    ///    when its `lowering_hook` is stamped.
    /// 2. The hook ID is the canonical key consumed by downstream
    ///    audit / LSP / compiler-explorer surfaces.
    // Registry-hook dispatcher keeps uniform fallback policy in one place.
    #[allow(clippy::too_many_lines)]
    fn try_dispatch_structured_hook(
        &mut self,
        cmd_name: &str,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Box<Statement>> {
        let args = seg.args();
        // Release availability is selected before recursive body lowering.
        let tokens = self.cmd_tokens_boxed(seg);
        if self.target.is_bytecode()
            && tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_structured_compilation(&tokens))
                .is_some_and(|preparation| {
                    matches!(
                        preparation.recipe(),
                        tcl_registry::native_instruction_plan::NativeInstructionPlan::Control(_)
                    )
                })
        {
            // The selected compiler owns expression preparation and body visits.
            // Retain its original vector until the native emission work list.
            return Some(self.lower_default_boxed(seg, namespace));
        }
        let admitted = self.target.is_bytecode().then(|| {
            crate::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                &tokens,
            )
        });
        let logical =
            (!self.target.is_bytecode()).then(|| self.logical_structured_invocation(&tokens));
        let resolved = if let Some(admitted) = &admitted {
            let admitted = admitted.as_ref()?;
            StructuredHookSelection {
                hook: admitted.lowering_hook()?,
                requirement: crate::registry_invocation::SelectedLoweringBindingIdentity::Native(
                    admitted,
                ),
                error_context: admitted.compiler_operand_error_context(),
            }
        } else {
            let logical = logical.as_ref()?.as_ref()?;
            StructuredHookSelection {
                hook: logical.lowering_hook()?,
                requirement: logical.lowering_identity(),
                error_context: logical.error_context(),
            }
        };
        let hook = resolved.hook;
        // The expansion gate lives here, keyed on the same typed hook the
        // dispatch below uses, so it can never name a different set of
        // commands than the lowerers it protects.
        if Self::hook_commits_to_argv_shape(hook)
            && seg
                .expand_word
                .as_ref()
                .is_some_and(|ew| ew.iter().any(|&e| e))
        {
            return Some(self.box_lowered_statement(|this| {
                this.structured_expand_barrier(cmd_name, args, seg)
            }));
        }
        self.with_original_body_parent(tokens, |this| {
            let inline_body_error_context = resolved.error_context;
            let statement = match hook {
                // Static-body uplevel.  Match `uplevel 1 {body}`,
                // `uplevel #0 {body}`, and the canonical no-level form
                // `uplevel {body}` (level defaults to 1) when the body
                // arg is a brace-string literal token.  Dynamic forms
                // (`uplevel 1 $body` / `uplevel $lvl {body}`) fall
                // through to the default lowering so a runtime
                // `Call` / `Barrier` carries the unresolved arguments.
                LoweringHookId::Uplevel => this.box_lowered_statement(|this| {
                    this.try_lower_uplevel_static(seg, namespace)
                        .unwrap_or_else(|| this.lower_default(seg, namespace))
                }),

                // `eval $body` / `eval {body}` with a literal /
                // const-folded body relaxes to a `Statement::Block` so
                // downstream analyses see the inlined script.  Dynamic
                // bodies (`eval $dyn` with no const-map binding,
                // `eval [cmd]`) fall through to the default barrier
                // dispatch.
                LoweringHookId::Eval => this.box_lowered_statement(|this| {
                    this.lower_eval_structured(
                        seg,
                        namespace,
                        inline_body_error_context,
                        resolved.requirement,
                    )
                }),

                // `apply {{params} body ?ns?} …` — walk the braced body so nested
                // definitions register (like `namespace eval`), keeping the call a
                // runtime barrier because the body runs in a separate frame.
                LoweringHookId::Apply => this.box_lowered_statement(|this| this.lower_apply(seg)),

                // `array for {k v} arr body` (Tcl 9.0) — iterate array entries.
                // Like `apply`, the call stays a runtime barrier (C Tcl invokes
                // `::tcl::array::for` with the body as an unparsed literal), but a
                // braced literal body is walked in a fresh frame bound to the two
                // loop variables so it is analysable.
                LoweringHookId::ArrayFor => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_array_for(seg, namespace))
                }

                // Straightforward control-flow forms.  Each is a
                // single-method dispatch with no arity / shared-method /
                // subcommand complications.
                LoweringHookId::If => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_if(seg, namespace))
                }
                LoweringHookId::Switch => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_switch(seg, namespace))
                }
                LoweringHookId::For => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_for(seg, namespace))
                }
                LoweringHookId::While => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_while(seg, namespace))
                }
                // A count the release's `catch` refuses is its `wrong # args`, which
                // the command raises before it runs the body.
                LoweringHookId::Catch => {
                    if !this
                        .release_admits_arity(resolved.requirement.binding_identity(), args.len())
                    {
                        return None;
                    }
                    this.box_lowered_statement(|this| {
                        this.lower_catch_with_binding(seg, namespace, resolved.requirement)
                    })
                }
                LoweringHookId::Try => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_try(seg, namespace))
                }

                // Forms with shape preconditions.  Calls with the wrong
                // arity / shape must fall through to `lower_default`
                // instead of crashing the dedicated lowerer, so each arm
                // returns `None` on precondition failure and
                // `lower_command` then falls through to `lower_default`.
                //
                // `proc name params body` — exactly three args and
                // at least three token slices (the body needs to
                // be a real token, not synthesised whitespace).
                LoweringHookId::Proc => {
                    return this.box_optional_lowered_statement(|this| {
                        this.try_lower_proc_declaration(
                            seg,
                            namespace,
                            resolved.requirement.descriptor_key().label(),
                        )
                    });
                }
                // `namespace eval ns body` — the subcommand match
                // is already handled by `resolve_call`, so the
                // hook only fires for the `eval` subcommand.  Keep
                // the arity / token-count guards (a tokenless body
                // would derail body lowering).
                LoweringHookId::NamespaceEval => {
                    if args.len() >= 3 && seg.arg_tokens().len() >= 3 {
                        this.box_lowered_statement(|this| this.lower_namespace_eval(seg, namespace))
                    } else {
                        return None;
                    }
                }
                // `foreach vars list body` / `lmap vars list body`
                // — share `lower_foreach(... is_lmap)`.  The
                // dedicated lowerer handles its own shape errors,
                // so no precondition here.
                LoweringHookId::Foreach => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_foreach(seg, namespace, false))
                }
                LoweringHookId::Lmap => {
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_foreach(seg, namespace, true))
                }
                // `dict <subcommand> ...` — must have at least one
                // arg so the subcommand can be picked.  Bare `dict`
                // falls through to `lower_default`.
                LoweringHookId::Dict => {
                    if args.is_empty() {
                        return None;
                    }
                    this.record_consumed_command_binding(seg, namespace, resolved.requirement);
                    this.box_lowered_statement(|this| this.lower_dict(seg, namespace))
                }

                // `when EVENT ?priority N? body` — iRules event
                // handler.  The hook stamp lives on the `when` spec
                // in `tcl-registry/src/commands/irules/when.rs`,
                // which `load_dialect(SpecSurface::IRULES)` brings
                // into the registry.  Production callers (the LSP
                // server) load the active dialect
                // before lowering; tests that lower iRule code call
                // `registry.load_irules()` explicitly (see
                // `irules_checks::tests::registry`).  Callers that
                // lower iRule code against a vanilla `build_default()`
                // registry now silently flow through to
                // `lower_default` — that path was always a
                // misconfiguration; the dialect needs to match the
                // source.
                LoweringHookId::When => {
                    return this.box_optional_lowered_statement(|this| {
                        this.try_lower_when_declaration(
                            seg,
                            namespace,
                            resolved.requirement.descriptor_key().label(),
                        )
                    });
                }
                // `foreachLine varName filename body` — Tcl 9.0
                // (TIP 670).  Always registered in `build_default()`
                // via `tcl::foreachline::spec`, so no dialect-load
                // dance is needed.  The `lower_foreach_line`
                // emitter handles its own shape errors and
                // dynamic-body fallback, but the `args.len() == 3`
                // guard keeps an under- or over-argued `foreachLine`
                // flowing to `lower_default` instead of triggering a
                // barrier inside the dedicated emitter.
                LoweringHookId::ForeachLine => {
                    return this.box_optional_lowered_statement(|this| {
                        this.try_lower_foreach_line_structured(seg, namespace, resolved.requirement)
                    });
                }

                // Non-structured hooks (`Expr` / `Return` / `Set` /
                // `Incr` / `AppendOrLappend` / `Unset` / `Global` /
                // `Variable` / `Upvar`) are handled by
                // [`try_lower_hook`] before [`lower_command`] reaches
                // this dispatcher.  They can still reach here when
                // their static dispatcher returned `None` for the
                // input shape (e.g. `try_lower_expr` rejects
                // multi-arg `expr`); fall through to `lower_default`.
                LoweringHookId::Expr
                | LoweringHookId::Return
                | LoweringHookId::Set
                | LoweringHookId::Incr
                | LoweringHookId::AppendOrLappend
                | LoweringHookId::Unset
                | LoweringHookId::Global
                | LoweringHookId::Variable
                | LoweringHookId::Upvar => return None,
            };
            Some(statement)
        })
    }

    /// Keep the genuine parent vector through all structured body callbacks,
    /// restoring it even when a hook declines before producing a statement.
    fn with_original_body_parent<T>(
        &mut self,
        parent: Box<CommandTokens>,
        lower: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let enclosing = self.structured_source_parent.replace(parent);
        let result = lower(self);
        self.structured_source_parent = enclosing;
        result
    }

    /// Materialise statement return storage outside the recursive dispatcher.
    /// Retaining one inline return slot per hook arm otherwise grows every
    /// nested structured lowering frame as the IR's statement variants grow.
    #[inline(never)]
    fn box_lowered_statement(
        &mut self,
        lower: impl FnOnce(&mut Self) -> Statement,
    ) -> Box<Statement> {
        Box::new(lower(self))
    }

    #[inline(never)]
    fn box_optional_lowered_statement(
        &mut self,
        lower: impl FnOnce(&mut Self) -> Option<Statement>,
    ) -> Option<Box<Statement>> {
        lower(self).map(Box::new)
    }

    /// Retain the resolved head of a command whose typed lowering consumes its
    /// live command semantics. This is recorded before lowering nested bodies,
    /// while the enclosing script's site scope is current.
    fn record_consumed_command_binding(
        &mut self,
        seg: &SegmentedCommand,
        resolution_namespace: &str,
        requirement: crate::registry_invocation::SelectedLoweringBindingIdentity<'_>,
    ) {
        let source_tokens = Some(Box::new(self.cmd_tokens(seg)));
        let name = crate::registry_invocation::static_command_word(
            source_tokens.as_deref().expect("retained invocation"),
            self.config.escapes,
            tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
        )
        .unwrap_or_else(|| seg.name().to_owned());
        let guard = crate::registry_invocation::command_binding_guard(
            source_tokens.as_deref().expect("retained invocation"),
        );
        let namespace_context = crate::registry_invocation::compiled_namespace_context(
            source_tokens.as_deref().expect("retained invocation"),
        );
        let shape = self.invocation_shape(seg);
        let Some(sites) = self.command_binding_site_stack.last_mut() else {
            return;
        };
        sites.push(CommandBindingSite {
            span: seg.span,
            source_tokens,
            known_namespaces: self
                .source_bindings
                .as_ref()
                .map(|bindings| bindings.known_namespaces_at(seg.span.start())),
            variable_frame: self.source_bindings.as_ref().map(|bindings| {
                bindings
                    .invocation_at_source(seg.name(), seg.span.start())
                    .variable_frame
            }),
            variable_context: self.source_bindings.as_ref().map(|bindings| {
                bindings
                    .invocation_at_source(seg.name(), seg.span.start())
                    .variable_context
            }),
            existing_namespace_cells: self.source_bindings.as_ref().map(|bindings| {
                bindings
                    .invocation_at_source(seg.name(), seg.span.start())
                    .existing_namespace_cells
            }),
            binding: tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
                resolution_namespace,
                name,
                requirement.binding_identity(),
            )
            .with_namespace_context(namespace_context)
            .with_guard(guard),
            shape,
        });
    }

    /// Whether the release's signature of `command` admits `count` arguments.
    fn release_admits_arity(&self, command: &str, count: usize) -> bool {
        self.registry.get(command).is_none_or(|spec| {
            self.registry
                .release_arity(spec)
                .accepts(u16::try_from(count).unwrap_or(u16::MAX))
        })
    }

    /// The words after `seg`'s command, as a release's compile reads them.
    fn invocation_shape(
        &self,
        seg: &SegmentedCommand,
    ) -> tcl_registry::release_compilation::InvocationShape {
        let release = self
            .dialect
            .map_or(tcl_dialect::TclVersion::V9_0, |profile| {
                profile.vm_runtime_version
            });
        let args = seg.texts.get(1..).unwrap_or_default();
        tcl_registry::release_compilation::InvocationShape::of(release, args, |index| {
            seg_word_is_static_literal(seg, index + 1)
        })
    }

    fn lower_catch_with_binding(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        requirement: crate::registry_invocation::SelectedLoweringBindingIdentity<'_>,
    ) -> Statement {
        let lowered = self.lower_catch(seg, namespace);
        if matches!(lowered, Statement::Catch { .. }) {
            self.record_consumed_command_binding(seg, namespace, requirement);
        }
        lowered
    }

    fn lower_eval_structured(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        inline_body_error_context: Option<tcl_registry::InlineBodyErrorContext>,
        requirement: crate::registry_invocation::SelectedLoweringBindingIdentity<'_>,
    ) -> Statement {
        let lowered = self.try_lower_eval_static(seg, namespace, inline_body_error_context);
        if lowered.is_some() {
            self.record_consumed_command_binding(seg, namespace, requirement);
        }
        lowered.unwrap_or_else(|| self.lower_default(seg, namespace))
    }

    fn try_lower_foreach_line_structured(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        requirement: crate::registry_invocation::SelectedLoweringBindingIdentity<'_>,
    ) -> Option<Statement> {
        if seg.args().len() != 3 {
            return None;
        }

        let lowered = self.lower_foreach_line(seg, namespace);
        if matches!(lowered, Statement::Foreach { .. }) {
            self.record_consumed_command_binding(seg, namespace, requirement);
        }
        Some(lowered)
    }

    /// Retain a conditional Logical procedure directly from its sealed source
    /// recipe. Native parameter/publication/entry receipts remain unavailable.
    fn try_lower_logical_procedure_declaration(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: &CommandTokens,
    ) -> Option<Statement> {
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource {
            return None;
        }
        let input = self
            .invocation_metadata_context()?
            .source_analysis_input()?;
        let declaration = self
            .source_bindings
            .as_ref()?
            .original_logical_procedure_declaration(tokens, self.registry, input)?;
        let qualified = declaration.qualified_name()?.to_owned();
        let name = declaration.name()?.to_owned();
        let params_raw = declaration.parameters_text()?.to_owned();
        let params = declaration
            .parameters()
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect();
        let source = declaration.source().clone();
        let body_offset = source.base();
        let body_source = source.try_text().ok()?.to_owned();
        let body = self
            .in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                lowerer.lower_original_body(source, Some(declaration.namespace()))
            });
        if !self.suppress_proc_register {
            let publication = original_procedure_publication(tokens);
            if publication.as_ref().is_some_and(|publication| {
                self.module
                    .procedures
                    .values()
                    .any(|previous| previous.publication.as_ref() == Some(publication))
            }) {
                self.module.redefined_procedures.insert(qualified.clone());
            }
            self.module.retain_procedure(Procedure {
                name,
                qualified_name: qualified,
                publication,
                declaration_site: tokens
                    .source_binding
                    .as_ref()
                    .and_then(|binding| binding.invocation_site())
                    .cloned(),
                body_kind: crate::ir::ProcedureBodyKind::Named,
                params,
                span: seg.span,
                body,
                params_raw,
                body_source: Some(body_source),
                body_offset,
                namespace_scoped: self.in_namespace_eval,
                base_priority: 500,
            });
        }
        Some(self.lower_default(seg, namespace))
    }

    fn try_lower_proc_declaration(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        canonical_command: &str,
    ) -> Option<Statement> {
        if seg.args().len() != 3 || seg.arg_tokens().len() < 3 {
            return None;
        }
        if self
            .registry
            .profile()
            .is_some_and(tcl_dialect::DialectProfile::is_irules)
        {
            if self
                .registry
                .irules_command_placement(canonical_command, self.irules_execution_context)
                != IrulesCommandPlacement::Allowed
            {
                return None;
            }
            let arg_refs: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            let closed = tcl_registry::events::closed_braced_argument_words(
                &self.source,
                seg.arg_tokens(),
                seg.arg_single_token(),
            )?;
            let declaration = tcl_registry::events::IrulesDeclarationArguments::new(
                &arg_refs,
                seg.arg_tokens(),
                seg.arg_single_token(),
                &closed,
            )
            .and_then(|arguments| {
                self.registry
                    .irules_top_level_declaration_shape(canonical_command, arguments)
            });
            if !matches!(
                declaration,
                Some(tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. })
            ) {
                return None;
            }
        }
        Some(self.lower_proc(seg, namespace))
    }

    fn try_lower_when_declaration(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        canonical_command: &str,
    ) -> Option<Statement> {
        if self
            .registry
            .irules_command_placement(canonical_command, self.irules_execution_context)
            != IrulesCommandPlacement::Allowed
        {
            return None;
        }
        let arg_refs: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let closed = tcl_registry::events::closed_braced_argument_words(
            &self.source,
            seg.arg_tokens(),
            seg.arg_single_token(),
        )?;
        let arguments = tcl_registry::events::IrulesDeclarationArguments::new(
            &arg_refs,
            seg.arg_tokens(),
            seg.arg_single_token(),
            &closed,
        )?;
        let tcl_registry::events::IrulesTopLevelDeclaration::Event {
            event,
            body_index,
            priority,
        } = self
            .registry
            .irules_top_level_declaration_shape(canonical_command, arguments)?
        else {
            return None;
        };
        Some(self.lower_when(seg, namespace, &event, body_index, priority))
    }

    /// Store a non-structured hook's exact dependency only when it replaced
    /// ordinary command dispatch with specialised IR.
    fn finish_resolved_lowering(
        &mut self,
        seg: &SegmentedCommand,
        lowered: ResolvedLowering,
    ) -> Statement {
        let source_tokens = lowered
            .binding
            .as_ref()
            .map(|_| Box::new(self.cmd_tokens(seg)));
        let shape = if lowered.binding.is_some() {
            self.invocation_shape(seg)
        } else {
            tcl_registry::release_compilation::InvocationShape::default()
        };
        if let Some(binding) = lowered.binding
            && let Some(sites) = self.command_binding_site_stack.last_mut()
        {
            sites.push(CommandBindingSite {
                span: seg.span,
                source_tokens,
                known_namespaces: self
                    .source_bindings
                    .as_ref()
                    .map(|bindings| bindings.known_namespaces_at(seg.span.start())),
                variable_frame: self.source_bindings.as_ref().map(|bindings| {
                    bindings
                        .invocation_at_source(seg.name(), seg.span.start())
                        .variable_frame
                }),
                variable_context: self.source_bindings.as_ref().map(|bindings| {
                    bindings
                        .invocation_at_source(seg.name(), seg.span.start())
                        .variable_context
                }),
                existing_namespace_cells: self.source_bindings.as_ref().map(|bindings| {
                    bindings
                        .invocation_at_source(seg.name(), seg.span.start())
                        .existing_namespace_cells
                }),
                binding,
                shape,
            });
        }
        lowered.statement
    }

    /// Retain vendor rule declarations independently of native compiler protocol.
    /// The host declaration grammar supplies a deferred inventory, not an opcode
    /// or proof that runtime command installation necessarily completes.
    fn try_lower_rule_loader_declaration(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: &CommandTokens,
    ) -> Option<Box<Statement>> {
        use tcl_dialect::model::InvocationRealm;
        use tcl_registry::events::IrulesTopLevelDeclaration;
        if !self
            .registry
            .profile()
            .is_some_and(tcl_dialect::DialectProfile::is_irules)
            || self.irules_execution_context != IrulesExecutionContext::TopLevel
        {
            return None;
        }
        let binding = tokens.source_binding.as_ref()?;
        if binding.invocation_realm() != Some(InvocationRealm::RuleLoader) {
            return None;
        }
        match self.retained_rule_loader_declaration(seg.span.start()) {
            Some(IrulesTopLevelDeclaration::Priority { value }) => {
                self.irules_priority = value;
                return Some(self.lower_default_boxed(seg, namespace));
            }
            Some(IrulesTopLevelDeclaration::Event {
                event,
                body_index,
                priority,
            }) => {
                return Some(Box::new(
                    self.lower_when(seg, namespace, &event, body_index, priority),
                ));
            }
            Some(IrulesTopLevelDeclaration::Procedure { .. }) => {
                // A validated alternative retains a deferred declaration only.
                // The original Call and its runtime/compilation residual remain.
                return Some(Box::new(self.lower_proc(seg, namespace)));
            }
            _ => {}
        }
        let target = binding.proved_handler_target()?;
        if !target.registry_backed || !target.prepended.is_empty() {
            return None;
        }
        let name = target.command.as_str();
        if let Some(value) =
            resolved_top_level_priority(self.registry, name, &binding.evaluated_argument_values)
        {
            self.irules_priority = value;
            return Some(self.lower_default_boxed(seg, namespace));
        }
        match self.registry.get(name)?.lowering_hook? {
            LoweringHookId::When => self
                .try_lower_when_declaration(seg, namespace, name)
                .map(Box::new),
            LoweringHookId::Proc => self
                .try_lower_proc_declaration(seg, namespace, name)
                .map(Box::new),
            _ => None,
        }
    }

    /// Body-bearing alternatives supply deferred inventory; file policy requires a
    /// closed declaration candidate. Neither projection licenses an opcode.
    fn retained_rule_loader_declaration(
        &self,
        offset: u32,
    ) -> Option<tcl_registry::events::IrulesTopLevelDeclaration> {
        use tcl_registry::events::IrulesTopLevelDeclaration;
        self.source_bindings
            .as_ref()?
            .deferred_rule_declaration_candidates(offset)?
            .iter()
            .find_map(|candidate| match &candidate.declaration {
                declaration @ IrulesTopLevelDeclaration::Event { .. } => Some(declaration.clone()),
                declaration @ IrulesTopLevelDeclaration::Procedure { .. }
                    if candidate.body.is_some() =>
                {
                    Some(declaration.clone())
                }
                declaration @ IrulesTopLevelDeclaration::Priority { .. } if !candidate.unknown => {
                    Some(declaration.clone())
                }
                _ => None,
            })
    }

    /// Lower a single command.
    fn lower_command(&mut self, seg: &SegmentedCommand, namespace: &str) -> Option<Box<Statement>> {
        if seg.texts.is_empty() {
            return None;
        }
        // Only the structured dispatch below recurses into nested bodies, so
        // every other selection runs in its own out-of-line frame: this frame
        // recurs once per nested body level under the depth guard's budget.
        match self.lower_command_selection(seg, namespace) {
            CommandLoweringSelection::Lowered(statement) => Some(statement),
            CommandLoweringSelection::Structured(resolved_name) => {
                if let Some(stmt) =
                    self.try_dispatch_structured_hook(&resolved_name, seg, namespace)
                {
                    return Some(stmt);
                }
                Some(self.lower_default_boxed(seg, namespace))
            }
        }
    }

    /// Every non-recursive selection of [`Self::lower_command`]: a finished
    /// statement, or the resolved label of a possible structured form.
    #[inline(never)]
    fn lower_command_selection(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> CommandLoweringSelection {
        match self.select_command_lowering(seg, namespace) {
            CommandLoweringStep::Selected(selection) => selection,
            CommandLoweringStep::Default => {
                CommandLoweringSelection::Lowered(self.lower_default_boxed(seg, namespace))
            }
        }
    }

    /// Decide how a command lowers. The default lowering recurses into
    /// evaluated bodies, so it is left to the caller: this frame, with every
    /// branch's temporaries, is then not held across that recursion.
    #[inline(never)]
    fn select_command_lowering(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> CommandLoweringStep {
        // Analysis uses the selected logical handler in its owned source scope.
        // Bytecode separately requires original native compiler admission.
        let tokens = self.cmd_tokens_boxed(seg);
        self.record_namespace_directives(&tokens, namespace);
        if let Some(declaration) = self.try_lower_rule_loader_declaration(seg, namespace, &tokens) {
            return CommandLoweringStep::lowered(declaration);
        }
        if self.target.is_bytecode()
            && crate::registry_invocation::proved_native_admitted_inline_operation(&tokens)
                .is_none()
        {
            self.retain_entered_namespace_body(seg, namespace, &tokens);
        }
        if !self.target.is_bytecode()
            && !self.target.is_trace_visible()
            && let Some(statement) = self
                .try_lower_authored_procedure_declaration(seg, namespace, &tokens)
                .or_else(|| self.try_lower_logical_procedure_declaration(seg, namespace, &tokens))
        {
            return CommandLoweringStep::lowered(Box::new(statement));
        }
        if !self.target.is_bytecode()
            && !self.target.is_trace_visible()
            && let Some(statement) =
                self.try_lower_original_operand_evaluation_update(seg, namespace, &tokens)
        {
            return CommandLoweringStep::lowered(statement);
        }
        let resolved_name = if self.target.is_bytecode() {
            let target = tokens
                .source_binding
                .as_ref()
                .and_then(|binding| {
                    binding
                        .admitted_inline_invocation()
                        .map(|proof| &proof.target)
                        .or_else(|| binding.proved_execution_target())
                })
                .filter(|target| target.registry_backed);
            if target
                .as_ref()
                .is_none_or(|target| !target.prepended.is_empty())
            {
                return CommandLoweringStep::Default;
            }
            if let Some(admitted) = crate::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                &tokens,
            ) {
                admitted.descriptor_key().label().to_owned()
            } else if let Some(label) = self.retained_metadata_descriptor_label(&tokens) {
                label
            } else {
                return CommandLoweringStep::Default;
            }
        } else {
            let Some(logical) = self.logical_structured_invocation(&tokens) else {
                return self
                    .expanded_structured_barrier(seg, &tokens)
                    .map_or(CommandLoweringStep::Default, CommandLoweringStep::lowered);
            };
            logical.descriptor_key().label().to_owned()
        };
        let resolved_name = resolved_name.as_str();

        if self.irules_execution_context == IrulesExecutionContext::TopLevel
            && let Some(value) = tokens.source_binding.as_ref().and_then(|binding| {
                resolved_top_level_priority(
                    self.registry,
                    resolved_name,
                    &binding.evaluated_argument_values,
                )
            })
        {
            self.irules_priority = value;
        }

        // Trace-visible compilation retains runtime dispatch at every site.
        if self.target.is_trace_visible() {
            return CommandLoweringStep::Default;
        }

        if self.target.is_bytecode()
            && crate::registry_invocation::proved_native_admitted_inline_operation(&tokens)
                .is_none()
        {
            // A declaration still supplies a deferred body inventory. Its
            // returned Call retains ordinary dispatch; no native opcode or
            // before-arguments command requirement is minted from that inventory.
            if self
                .registry
                .get(resolved_name)
                .is_some_and(|spec| spec.lowering_hook == Some(LoweringHookId::Proc))
                && let Some(statement) =
                    self.try_lower_proc_declaration_boxed(seg, namespace, resolved_name)
            {
                return CommandLoweringStep::lowered(statement);
            }
            return CommandLoweringStep::Default;
        }

        if let Some(statement) = self.try_registered_lowering_hook(resolved_name, seg, namespace) {
            return CommandLoweringStep::lowered(statement);
        }

        // Registry-driven hook-ID dispatch covers all 17
        // structured command forms — every typed
        // `LoweringHookId` (Proc, When, NamespaceEval, If,
        // Switch, For, While, Foreach, Lmap, ForeachLine, Catch,
        // Try, Dict, Eval, Uplevel, Apply, ArrayFor) flows through
        // [`try_dispatch_structured_hook`], which also raises the
        // `{*}`-expansion barrier for them.  Commands that
        // aren't in the registry, or whose hook is `None`, fall
        // through to [`lower_default`] in [`Self::lower_command`].
        CommandLoweringStep::Selected(CommandLoweringSelection::Structured(
            resolved_name.to_owned(),
        ))
    }

    /// Retain an attempted update's original read plan without admitting its
    /// failed argv, handler or any body. Ordinary handler lowering stays gated.
    fn try_lower_original_operand_evaluation_update(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: &CommandTokens,
    ) -> Option<Box<Statement>> {
        let selected = crate::registry_invocation::original_operand_evaluation_update(
            self.registry,
            self.invocation_metadata_context()?,
            tokens,
            self.source_bindings.as_deref()?,
        )?;
        let command = LoweringCommand {
            span: seg.span,
            name: selected.descriptor_key().label(),
            resolution_namespace: namespace,
            args: seg.args(),
            single_token_word: &seg.single_token_word,
            expand_word: seg.expand_word.as_deref(),
            tokens: Some(tokens.clone()),
            arg_kinds: &Self::arg_kinds(seg),
            dialect: self.dialect,
            lexer_config: self.config,
        };
        let lowered = crate::lowering_hooks::try_lower_operand_evaluation_update_with_binding(
            &command,
            &selected,
            &self.aliases,
            self.registry,
            self.dialect_context
                .as_deref()
                .map(tcl_registry::model::ContextRegistry::context),
            self.safe_on_uninit(command.name, command.args),
        )?;
        Some(Box::new(self.finish_resolved_lowering(seg, lowered)))
    }

    /// Keep declaration construction out of ordinary command descent frames.
    #[inline(never)]
    fn try_lower_proc_declaration_boxed(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        resolved_name: &str,
    ) -> Option<Box<Statement>> {
        self.try_lower_proc_declaration(seg, namespace, resolved_name)
            .map(Box::new)
    }

    #[inline(never)]
    fn try_registered_lowering_hook(
        &mut self,
        resolved_name: &str,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Box<Statement>> {
        let args = seg.args();
        let hook_cmd = LoweringCommand {
            span: seg.span,
            name: resolved_name,
            resolution_namespace: namespace,
            args,
            single_token_word: &seg.single_token_word,
            expand_word: seg.expand_word.as_deref(),
            tokens: Some(self.cmd_tokens(seg)),
            arg_kinds: &Self::arg_kinds(seg),
            dialect: self.dialect,
            lexer_config: self.config,
        };
        let context = self
            .dialect_context
            .as_deref()
            .map(tcl_registry::model::ContextRegistry::context);
        let admitted = self.target.is_bytecode().then(|| {
            crate::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                hook_cmd.tokens.as_ref()?,
            )
        });
        let logical = (!self.target.is_bytecode())
            .then(|| self.logical_structured_invocation(hook_cmd.tokens.as_ref()?));
        let requirement = if let Some(admitted) = &admitted {
            crate::registry_invocation::SelectedLoweringBindingIdentity::Native(admitted.as_ref()?)
        } else {
            logical.as_ref()?.as_ref()?.lowering_identity()
        };
        let mut lowered = if self.target.is_bytecode() {
            crate::lowering_hooks::try_lower_admitted_hook_with_binding(
                &hook_cmd,
                &self.aliases,
                self.registry,
                context,
                self.safe_on_uninit(resolved_name, args),
            )?
        } else {
            let logical = logical.as_ref()?.as_ref()?;
            crate::lowering_hooks::try_lower_logical_hook_with_binding(
                &hook_cmd,
                logical,
                &self.aliases,
                self.registry,
                context,
                self.safe_on_uninit(resolved_name, args),
            )?
        };
        let tokens = self.cmd_tokens(seg);
        let name = crate::registry_invocation::static_command_word(
            &tokens,
            self.config.escapes,
            tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
        )
        .unwrap_or_else(|| seg.name().to_owned());
        let binding = tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
            namespace,
            name,
            requirement.binding_identity(),
        )
        .with_namespace_context(crate::registry_invocation::compiled_namespace_context(
            &tokens,
        ))
        .with_guard(crate::registry_invocation::command_binding_guard(&tokens));
        if let Statement::ExprEval {
            command_binding, ..
        } = &mut lowered.statement
        {
            *command_binding = binding.clone();
        }
        if lowered.binding.is_some() {
            lowered.binding = Some(binding);
        }
        Some(Box::new(self.finish_resolved_lowering(seg, lowered)))
    }

    #[inline(never)]
    fn lower_default_boxed(&mut self, seg: &SegmentedCommand, namespace: &str) -> Box<Statement> {
        Box::new(self.lower_default(seg, namespace))
    }

    /// Lower `proc name params body`.
    ///
    /// In order:
    ///   1. Empty-simple-name barrier (`proc ::ns:: {…} {…}`).
    ///   2. Dynamic name resolution (`proc $x …` / `proc [cmd] …`).
    ///   3. Dynamic body / params check, with const-map body
    ///      materialisation when possible.
    ///   4. IR registration + emit the runtime `proc` call.
    fn lower_proc(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource {
            return self.lower_default(seg, namespace);
        }
        let args_borrow = seg.args();
        let proc_name_initial = &args_borrow[0];

        // Empty simple names need runtime publication; preserve the original
        // invocation instead of registering a lifted procedure.
        if tcl_syntax::naming::ends_with_separator(proc_name_initial.as_bytes()) {
            return Statement::Barrier {
                span: seg.span,
                reason: "empty-simple-name proc".into(),
                command: seg.name().into(),
                canonical_command: None,
                args: args_borrow.to_vec(),
                tokens: Some(self.cmd_tokens(seg)),
            };
        }
        // Resolve a dynamic proc name.
        let (proc_name_owned, args_owned, name_was_substituted) =
            match self.resolve_dynamic_proc_name(seg, args_borrow) {
                Ok(triple) => triple,
                Err(barrier) => return *barrier,
            };
        let args: &[String] = &args_owned;
        let proc_name = &proc_name_owned;
        // Check for a dynamic body / params, materialising the body via the
        // const-map where possible.
        let (materialised_body, body_is_dynamic, body_offset) =
            match self.check_proc_body_dynamic(seg, args, name_was_substituted) {
                Ok(triple) => triple,
                Err(barrier) => return *barrier,
            };

        // Lower the body (materialised, static, or empty for
        // dynamic-with-resolved-name), register the Procedure, and emit the
        // runtime `proc` Call.
        // A parameter list Tcl would reject (malformed, an overlong specifier,
        // an array-element or qualified name) creates no procedure: leave it to
        // the runtime `proc`, which raises the error, rather than registering a
        // Procedure whose parameters we guessed at.
        let Some(params) = parse_formal_param_names(
            &args[1],
            WordValueRules::from_config(&self.config),
            self.invocation_dialect
                .and_then(tcl_registry::InvocationDialect::parameter_grammar),
        ) else {
            return Statement::Barrier {
                span: seg.span,
                reason: "malformed proc params".into(),
                command: seg.name().into(),
                canonical_command: None,
                args: seg.args().to_vec(),
                tokens: Some(self.cmd_tokens(seg)),
            };
        };
        let tokens = self.cmd_tokens(seg);
        let Some(publication) = original_procedure_publication(&tokens) else {
            return self.lower_default(seg, namespace);
        };
        let Some(qualified) = publication.reported_full_name() else {
            return self.lower_default(seg, namespace);
        };
        let body_text = &args[2];
        let body = self.lower_declared_proc_body(
            seg,
            materialised_body,
            body_is_dynamic,
            body_text,
            body_offset,
            namespace,
        );

        // A `proc` defined inside a TclOO method body is created
        // at method-call time in the runtime receiver namespace, not at script
        // load — so it must not be lifted into `module.procedures`
        // (codegen would otherwise emit it unconditionally). The body
        // was still lowered above for analysis; only the premature static
        // registration is suppressed.
        if !self.suppress_proc_register {
            if self
                .module
                .procedures
                .values()
                .any(|previous| previous.publication.as_ref() == Some(&publication))
            {
                self.module.redefined_procedures.insert(qualified.clone());
            }
            self.module.retain_procedure(Procedure {
                name: proc_name.clone(),
                qualified_name: qualified,
                publication: Some(publication),
                declaration_site: tokens
                    .source_binding
                    .as_ref()
                    .and_then(|binding| binding.invocation_site())
                    .cloned(),
                body_kind: crate::ir::ProcedureBodyKind::Named,
                params,
                span: seg.span,
                body,
                params_raw: args[1].clone(),
                body_source: Some(args[2].clone()),
                body_offset,
                namespace_scoped: self.in_namespace_eval,
                base_priority: 500,
            });
        }

        Statement::Call {
            span: seg.span,
            command: seg.name().into(),
            canonical_command: None,
            args: seg.args().to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(self.cmd_tokens(seg)),
            foreach_groups: None,
        }
    }

    /// Prefer the declaration's exact retained script and execution frame.
    fn lower_declared_proc_body(
        &mut self,
        seg: &SegmentedCommand,
        materialised_body: Option<String>,
        body_is_dynamic: bool,
        body_text: &str,
        body_offset: u32,
        body_namespace: &str,
    ) -> Script {
        let retained = seg
            .arg_tokens()
            .get(2)
            .and_then(|token| {
                self.source_bindings
                    .as_ref()?
                    .executed_script_for_word(token.span)
                    .cloned()
            })
            .or_else(|| {
                self.source_bindings
                    .as_ref()?
                    .deferred_rule_declaration_candidates(seg.span.start())?
                    .iter()
                    .find_map(|candidate| {
                        matches!(
                            candidate.declaration,
                            tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. }
                        )
                        .then(|| candidate.body.clone())
                        .flatten()
                    })
            })
            .or_else(|| {
                // A declaration replaced before any call never enters its body,
                // yet `proc` created it from this exact script: C Tcl runs it in
                // the declaration's namespace if a caller reaches it first.
                let tokens = self.cmd_tokens(seg);
                let declaration = tokens.source_binding.as_ref()?.invocation_site()?;
                let mut bodies = self
                    .source_bindings
                    .as_ref()?
                    .procedure_implementation_bodies()
                    .filter(|body| &body.allocation.site == declaration);
                let first = bodies.next()?.source.clone();
                bodies.all(|body| body.source == &first).then_some(first)
            });
        if let Some(source) = retained {
            let declaration = self
                .cmd_tokens(seg)
                .source_binding
                .as_ref()
                .and_then(|binding| binding.invocation_site())
                .cloned();
            let body_namespace = self
                .source_bindings
                .as_ref()
                .and_then(|bindings| {
                    let declaration = declaration.as_ref()?;
                    let mut bodies = bindings.procedure_implementation_bodies().filter(|body| {
                        &body.allocation.site == declaration && body.source == &source
                    });
                    let first = bodies.next()?.namespace_key;
                    bodies
                        .all(|body| body.namespace_key == first)
                        .then(|| first.clone())
                })
                .or_else(|| {
                    self.source_bindings
                        .as_ref()?
                        .executed_script_namespace_context(&source)
                });
            self.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                lowerer.lower_original_body(source, body_namespace.as_ref())
            })
        } else if self.native_entry.is_some() {
            // A native declaration without retained body geometry cannot be
            // reanalysed under a namespace reconstructed from its label.
            over_depth_script(body_offset, body_text.len())
        } else {
            self.lower_proc_body(
                materialised_body,
                body_is_dynamic,
                body_text,
                body_offset,
                body_namespace,
            )
        }
    }

    /// Lower one procedure body in its fresh procedure frame.
    fn lower_proc_body(
        &mut self,
        materialised_body: Option<String>,
        body_is_dynamic: bool,
        body_text: &str,
        body_offset: u32,
        body_namespace: &str,
    ) -> Script {
        // ``lower_body`` otherwise inherits the enclosing scope's tracked
        // scalars, which is sound for control flow but not a proc's fresh
        // runtime frame.
        self.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
            if let Some(text) = materialised_body {
                lowerer.lower_script_in_irules_context(
                    &text,
                    body_namespace,
                    IrulesExecutionContext::ProcedureBody,
                )
            } else if body_is_dynamic {
                // The resolved body value is dynamic; retain the runtime call but
                // do not compile the literal `$body` spelling as Tcl source.
                Script::default()
            } else if (lowerer.body_cache.is_some() || lowerer.original_body_cache.is_some()) && {
                lowerer.proc_depth == 1
                    && !lowerer
                        .registry
                        .profile()
                        .is_some_and(tcl_dialect::DialectProfile::is_irules)
                    && lowerer.source_bindings.as_ref().is_some_and(|bindings| {
                        bindings.can_reuse_isolated_body(
                            body_text,
                            body_offset,
                            body_namespace,
                            lowerer.config,
                            lowerer.registry,
                        )
                    })
            } {
                // Only context-free Tcl bodies can use the offset-zero body cache.
                let (mut script, issued) = if let Some(cache) = lowerer.original_body_cache {
                    let template = cache(body_text, body_namespace);
                    let issued = template.matches(
                        body_text,
                        body_namespace,
                        lowerer.registry,
                        lowerer.config,
                        lowerer.dialect,
                    );
                    (template.body, issued)
                } else {
                    (
                        (lowerer.body_cache.expect("selected body cache"))(
                            body_text,
                            body_namespace,
                        ),
                        false,
                    )
                };
                crate::lattice_rebase::rebase_script(&mut script, i64::from(body_offset));
                if lowerer
                    .source_bindings
                    .as_ref()
                    .is_some_and(|bindings| bindings.restore_script_bindings(&mut script))
                {
                    if !issued && let Some(integrity) = lowerer.optimiser_lowering_integrity {
                        integrity.set(false);
                    }
                    script
                } else {
                    lowerer.lower_body_in_irules_context(
                        body_text,
                        body_offset,
                        body_namespace,
                        IrulesExecutionContext::ProcedureBody,
                    )
                }
            } else {
                lowerer.lower_body_in_irules_context(
                    body_text,
                    body_offset,
                    body_namespace,
                    IrulesExecutionContext::ProcedureBody,
                )
            }
        })
    }

    /// Resolve a single variable name from its original Native argument value
    /// or quiet Logical source binding. Unknown and compound names retain
    /// runtime dispatch; literal names keep their original argument vector.
    fn resolve_dynamic_proc_name(
        &mut self,
        seg: &SegmentedCommand,
        args_borrow: &[String],
    ) -> Result<(String, Vec<String>, bool), Box<Statement>> {
        let proc_name_initial = &args_borrow[0];
        if !proc_name_initial.contains('$') && !proc_name_initial.contains('[') {
            return Ok((proc_name_initial.clone(), args_borrow.to_vec(), false));
        }

        let arg_tokens = seg.arg_tokens();
        let single_token_proc_name = seg.single_token_word.get(1).copied().unwrap_or(false);
        let resolved = if !proc_name_initial.contains('[')
            && single_token_proc_name
            && arg_tokens
                .first()
                .is_some_and(|t| t.kind == tcl_lexer::TokenType::Var)
        {
            self.original_native_written_argument_value(seg, 0)
                .or_else(|| self.original_authored_written_argument_value(seg, 0))
                .or_else(|| self.const_map_lookup(proc_name_initial))
        } else {
            None
        };

        let Some(literal) = resolved else {
            return Err(Box::new(Statement::Barrier {
                span: seg.span,
                reason: "dynamic proc name".into(),
                command: seg.name().into(),
                canonical_command: None,
                args: args_borrow.to_vec(),
                tokens: Some(self.cmd_tokens(seg)),
            }));
        };

        let mut args_owned = args_borrow.to_vec();
        args_owned[0].clone_from(&literal);
        Ok((literal, args_owned, true))
    }

    /// Check whether the proc body and/or params are dynamic, and
    /// attempt to materialise the body from the const-map.
    ///
    /// Returns:
    ///   * `Ok((Some(text), _, body_offset))` — body materialised
    ///     successfully; caller should lower `text` as a fresh
    ///     script.
    ///   * `Ok((None, body_is_dynamic, body_offset))` — body is
    ///     static (lower from source) or dynamic-with-name-resolved
    ///     (lower as empty Script).
    ///   * `Err(barrier)` — params dynamic, or body dynamic without
    ///     name substitution.  Caller propagates the barrier.
    fn check_proc_body_dynamic(
        &mut self,
        seg: &SegmentedCommand,
        args: &[String],
        name_was_substituted: bool,
    ) -> Result<(Option<String>, bool, u32), Box<Statement>> {
        let arg_tokens = seg.arg_tokens();
        let params_tok = arg_tokens[1];
        let body_tok = arg_tokens[2];
        // `single_token_word[0]` covers the command itself; args[0..]
        // live at indices `[1..]`.  See `SegmentedCommand::arg_single_token`.
        let body_is_single = seg.single_token_word.get(3).copied().unwrap_or(false);
        let params_is_single = seg.single_token_word.get(2).copied().unwrap_or(false);
        let body_is_dynamic = matches!(
            body_tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) || !body_is_single;
        let params_is_dynamic = matches!(
            params_tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) || !params_is_single;
        let materialised_body: Option<String> =
            if body_is_single && body_tok.kind == tcl_lexer::TokenType::Cmd {
                // `args[2]` is the `word_piece`-reconstructed source
                // — for a Cmd token that's `[subst -nocommands {…}]`
                // (with the brackets re-added).
                // `eval_subst_nocommands_body` segments its input as
                // a top-level Tcl command stream and expects the
                // inner content, so strip the outer `[…]` before
                // handing it over.
                self.original_native_written_argument_value(seg, 2)
                    .or_else(|| self.original_authored_written_argument_value(seg, 2))
                    .or_else(|| {
                        args[2]
                            .strip_prefix('[')
                            .and_then(|s| s.strip_suffix(']'))
                            .and_then(|inner| self.eval_subst_nocommands_body(inner))
                    })
            } else if body_is_single && body_tok.kind == tcl_lexer::TokenType::Var {
                self.original_native_written_argument_value(seg, 2)
                    .or_else(|| self.original_authored_written_argument_value(seg, 2))
                    .or_else(|| self.const_map_lookup(&args[2]))
            } else {
                None
            };
        if params_is_dynamic
            || (body_is_dynamic && materialised_body.is_none() && !name_was_substituted)
        {
            return Err(Box::new(Statement::Barrier {
                span: seg.span,
                reason: if params_is_dynamic {
                    "dynamic proc params"
                } else {
                    "dynamic proc body"
                }
                .into(),
                command: seg.name().into(),
                canonical_command: None,
                args: seg.args().to_vec(),
                tokens: Some(self.cmd_tokens(seg)),
            }));
        }
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        Ok((materialised_body, body_is_dynamic, body_offset))
    }

    // Preserve each queried Option; this reports source ownership only.
    fn trace_report_event_option<T>(value: Option<T>, stage: &str) -> Option<T> {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_EVENT_OPERAND").is_some() {
            eprintln!(
                "REPORT_EVENT_SOURCE stage={stage} available={}",
                value.is_some()
            );
        }
        value
    }

    fn trace_report_event_refusal(value: bool, stage: &str) -> bool {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_EVENT_OPERAND").is_some() {
            eprintln!("REPORT_EVENT_SOURCE stage={stage} refused={value}");
        }
        value
    }

    fn original_hosted_when_event_word(
        &self,
        context: crate::registry_invocation::InvocationMetadataContext<'_>,
        bindings: &crate::command_binding::SourceCommandBindings,
        candidates: &[crate::command_binding::SourceRuleDeclarationCandidate],
        source: (
            &SegmentedCommand,
            &tcl_lexer::SourceImage,
            &CommandTokens,
            &[tcl_lexer::NativeWord],
        ),
        word: &tcl_lexer::NativeWord,
        event: &str,
    ) -> Option<tcl_lexer::NativeWord> {
        let (seg, image, tokens, words) = source;
        let input =
            Self::trace_report_event_option(context.source_analysis_input(), "hosted-input")?;
        let owner =
            Self::trace_report_event_option(bindings.original_analysis_owner(), "analysis-owner")?;
        let root = Self::trace_report_event_option(bindings.source_origin(), "root-origin")?;
        let binding =
            Self::trace_report_event_option(tokens.source_binding.as_ref(), "token-binding")?;
        let site = Self::trace_report_event_option(binding.invocation_site(), "invocation-site")?;
        if Self::trace_report_event_refusal(tokens.synthetic.is_some(), "synthetic")
            || Self::trace_report_event_refusal(
                !tokens.original_words_match_argv_text(),
                "original-argv",
            )
            || Self::trace_report_event_refusal(
                words.iter().any(|word| word.group().expand),
                "expanded",
            )
            || Self::trace_report_event_refusal(site.offset != seg.span.start(), "site-offset")
            || Self::trace_report_event_refusal(
                !std::sync::Arc::ptr_eq(&site.source, root),
                "site-root-owner",
            )
            || Self::trace_report_event_refusal(
                !owner.matches(bindings, input),
                "analysis-owner-current",
            )
            || Self::trace_report_event_refusal(
                bindings.source_origin()?.source_image() != image,
                "root-image",
            )
            || Self::trace_report_event_refusal(input.lexer_config() != self.config, "input-config")
            || Self::trace_report_event_refusal(
                !context.matches_registry(self.registry),
                "registry-current",
            )
        {
            return None;
        }
        // The retained declaration owns source geometry, not a current
        // dispatch cell or installation of this event handler.
        let policy = Self::trace_report_event_option(input.vendor_source_policy().or_else(|| {
            let hosted = tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
                &input.borrowed_context_registry().context().environment.id,
            )?;
            tcl_syntax::naming::VendorSourceNamePolicy::authored(hosted)
        }), "hosted-policy")?;
        let head = crate::signature_scan::vendor_name::VendorSourceNameInput::from_original_word(
            words.first()?,
            policy,
        );
        let written = std::str::from_utf8(Self::trace_report_event_option(
            head.literal_units(tcl_syntax::naming::VendorSourceNamePurpose::CommandHead),
            "original-head-units",
        )?)
        .ok()?;
        let command = tcl_syntax::naming::unroot_rooted_key(written).unwrap_or(written);
        if candidates.iter().any(|candidate| {
            tcl_syntax::naming::unroot_rooted_key(&candidate.target.command)
                .unwrap_or(&candidate.target.command)
                != command
        }) {
            return None;
        }
        let input = crate::signature_scan::vendor_name::VendorSourceNameInput::from_original_word(
            word, policy,
        );
        let literal = Self::trace_report_event_option(
            input.literal_units(tcl_syntax::naming::VendorSourceNamePurpose::EventName),
            "hosted-event-units",
        )?;
        Self::trace_report_event_option(
            literal
                .eq_ignore_ascii_case(event.as_bytes())
                .then(|| word.clone()),
            "hosted-event-value",
        )
    }

    /// Event geometry comes from the retained declaration and original words.
    /// A captured/derived argument cannot borrow a displayed event spelling.
    fn original_when_event_word(
        &self,
        seg: &SegmentedCommand,
        event: &str,
        image: &tcl_lexer::SourceImage,
    ) -> Option<tcl_lexer::NativeWord> {
        use tcl_registry::events::IrulesTopLevelDeclaration;
        let bindings =
            Self::trace_report_event_option(self.source_bindings.as_ref(), "source-bindings")?;
        let candidates = Self::trace_report_event_option(
            bindings.deferred_rule_declaration_candidates(seg.span.start()),
            "deferred-candidates",
        )?;
        let indices = Self::trace_report_event_option(
            candidates
                .iter()
                .map(|candidate| {
                    let IrulesTopLevelDeclaration::Event { event: name, .. } =
                        &candidate.declaration
                    else {
                        return None;
                    };
                    if name != event
                        || !candidate.target.registry_backed
                        || !candidate.target.prepended.is_empty()
                    {
                        return None;
                    }
                    let spec = Self::trace_report_event_option(
                        self.registry.get(&candidate.target.command),
                        "candidate-registry-spec",
                    )?;
                    let symbol =
                        Self::trace_report_event_option(spec.defines_symbol, "candidate-symbol")?;
                    (symbol.kind == tcl_registry::DefinedSymbolKind::Event)
                        .then_some(usize::from(symbol.name_arg) + 1)
                })
                .collect::<Option<Vec<_>>>(),
            "event-indices",
        )?;
        let &index = Self::trace_report_event_option(indices.first(), "event-index")?;
        if indices.iter().any(|candidate| *candidate != index) {
            return None;
        }
        let tokens = self.cmd_tokens(seg);
        let words = Self::trace_report_event_option(
            crate::registry_invocation::original_native_compiler_words(
                image,
                &tokens.word_exprs,
                seg.span.start(),
                self.config,
            ),
            "original-words",
        )?;
        let word = Self::trace_report_event_option(words.get(index), "original-event-word")?;
        if let Some(context) =
            Self::trace_report_event_option(self.invocation_metadata_context(), "metadata-context")
            && context
                .source_analysis_input()
                .is_some_and(crate::analyser::ResolvedAnalysisInput::has_hosted_source_name_context)
        {
            return self.original_hosted_when_event_word(
                context,
                bindings,
                candidates,
                (seg, image, &tokens, &words),
                word,
                event,
            );
        }
        let dialect =
            Self::trace_report_event_option(self.invocation_dialect, "standalone-dialect")?;
        let key = crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
            word,
            dialect.word_values,
            dialect.authored_name_policy()?,
        )?;
        key.bytes()
            .eq_ignore_ascii_case(event.as_bytes())
            .then(|| word.clone())
    }

    fn original_when_source_geometry(
        &self,
        seg: &SegmentedCommand,
        event: &str,
    ) -> (tcl_lexer::SourceImage, Option<tcl_lexer::NativeWord>) {
        let image = self
            .source_bindings
            .as_ref()
            .map(|bindings| bindings.invocation_at_source(seg.name(), seg.span.start()))
            .and_then(|binding| {
                binding
                    .invocation_site()
                    .map(|site| site.source.source_image().clone())
            })
            .unwrap_or_else(|| {
                tcl_lexer::SourceImage::from_bytes(self.source.as_bytes(), self.source_channel)
            });
        let word = self.original_when_event_word(seg, event, &image);
        (image, word)
    }

    /// Lower a registry-validated iRules event declaration.
    fn lower_when(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        event_name: &str,
        body_idx: usize,
        priority: Option<u16>,
    ) -> Statement {
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource {
            return self.lower_default(seg, namespace);
        }
        let args = seg.args();
        let body_tok = seg.arg_tokens()[body_idx];
        let retained = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.deferred_rule_declaration_candidates(seg.span.start()))
            .and_then(|candidates| {
                candidates
                    .iter()
                    .find_map(|candidate| candidate.body.clone())
            });
        let body = self.in_procedure_frame(Some(IrulesExecutionContext::EventBody), |lowerer| {
            if let Some(source) = retained {
                lowerer.lower_retained_script(source)
            } else {
                lowerer.lower_body_from_tok(&args[body_idx], Some(&body_tok), namespace)
            }
        });
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);

        let base_priority = u32::from(priority.unwrap_or(self.irules_priority));

        let n = self.when_counts.get(event_name).copied().unwrap_or(0);
        *self.when_counts.entry(event_name.to_owned()).or_insert(0) += 1;
        let qualified = if n == 0 {
            format!("::when::{event_name}")
        } else {
            format!("::when::{event_name}#{n}")
        };

        let (image, event_word) = self.original_when_source_geometry(seg, event_name);
        let event_body = std::sync::Arc::new(crate::ir::SourceIrulesEventBody::from_lowering(
            event_name,
            seg.span,
            &body,
            image,
            self.config,
            self.registry,
            event_word,
        ));
        let id = self.module.retain_procedure(Procedure {
            name: event_name.to_owned(),
            qualified_name: qualified,
            publication: None,
            declaration_site: self
                .cmd_tokens(seg)
                .source_binding
                .as_ref()
                .and_then(|binding| binding.invocation_site())
                .cloned(),
            body_kind: crate::ir::ProcedureBodyKind::Event,
            params: vec![],
            span: seg.span,
            body,
            params_raw: String::new(),
            body_source: None,
            body_offset,
            namespace_scoped: false,
            base_priority,
        });
        self.module.irules_event_bodies.insert(id, event_body);

        Statement::Call {
            span: seg.span,
            command: "when".into(),
            canonical_command: None,
            args: args.to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(self.cmd_tokens(seg)),
            foreach_groups: None,
        }
    }

    /// Lower `namespace eval ns body`.
    /// Try to lower `uplevel ?level? {body}` to a static-body
    /// [`Statement::UpFrame`] when:
    ///
    /// 1. The body argument is a brace-string token (`TokenType::Str`),
    ///    and
    /// 2. The level argument (if present) parses as a positive integer
    ///    or `#0` / `#N` global form.
    ///
    /// Returns `None` if the call doesn't match the static shape, in
    /// which case the caller falls back to [`Self::lower_default`]
    /// (producing a runtime [`Statement::Barrier`]).
    fn try_lower_uplevel_static(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Statement> {
        use tcl_lexer::TokenType;

        // Static-body `uplevel` lowers to `Statement::UpFrame` so the analysers
        // (interproc purity, code-sinking, var-escape, the inline_uplevel pass)
        // see a structured frame-shift with a lowered body instead of an opaque
        // runtime barrier. Runtime correctness is preserved at codegen: a
        // surviving `UpFrame` re-emits the original `uplevel <level> {body}`
        // invoke from its preserved command tokens (see
        // `codegen::statements`), producing byte-identical bytecode to the old
        // barrier path and dispatching through the `uplevel` builtin's correct
        // frame semantics. The whole-callee inline_uplevel splice (which would
        // need frame-shift opcodes to run a *bare* UpFrame against the right
        // activation) is an opt-in optimiser pass, not wired into codegen.
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let (level, body_tok_idx) = match args.len() {
            // A bare `uplevel {body}` means relative level 1.
            1 => (
                UplevelLevel {
                    shift: 1,
                    absolute: false,
                },
                0,
            ),
            2 => (parse_uplevel_level(&args[0])?, 1),
            _ => return None,
        };
        let body_tok = arg_tokens.get(body_tok_idx)?;
        // Barrier gate: see `try_lower_eval_static` for the
        // rationale.  A static-body `uplevel 1 {eval $x}` would
        // relax to inline IR with a still-dynamic `eval`; reject.
        if body_tok.kind == TokenType::Str {
            let body_text = &args[body_tok_idx];
            if body_has_dynamic_barrier(body_text, self.registry, self.config) {
                return None;
            }
        }
        let body_namespace = match (level.absolute, level.shift) {
            (true, 0) => "::",
            // A non-zero relative frame is selected only at runtime. Keep the
            // lexical fallback for lowering; ExecutionNamespace later marks
            // the body runtime-selected and invalidates relative typed proof.
            _ => namespace,
        };
        let body = if body_tok.kind == TokenType::Str {
            self.lower_body_from_tok(&args[body_tok_idx], Some(body_tok), body_namespace)
        } else if body_tok.kind == TokenType::Var {
            // Retain the original Native argument or quiet Logical source value.
            let literal = self
                .original_native_written_argument_value(seg, body_tok_idx)
                .or_else(|| self.const_map_lookup(&args[body_tok_idx]))?;
            self.lower_script(
                &literal,
                body_namespace,
                SourceSegmentationPurpose::NestedScript,
            )
        } else {
            return None;
        };
        Some(Statement::UpFrame {
            span: seg.span,
            frame_shift: level.shift,
            absolute: level.absolute,
            body,
            tokens: Some(self.cmd_tokens(seg)),
        })
    }

    /// If *`cmd_text`* is a call whose template-word plan
    /// (`docs/design/compiler/value-transfers.md` § *The template-word
    /// plan*) runs exactly [`SUBST_NOCOMMANDS_KINDS`] over a braced
    /// template AND every variable that template reads is in the current
    /// const-map, return the substituted string. Otherwise `None` so the
    /// caller falls back to runtime dispatch.
    ///
    /// Used to materialise the tcltest-style `Option` factory body
    /// at compile time when the surrounding proc has all the
    /// template vars const-tracked.
    ///
    /// The plan is the registry's answer over the call's literal words: any
    /// other set of kinds is a call this evaluator does not reproduce, and a
    /// computed switch word is no spelling a release accepts, so it folds
    /// nothing.
    fn eval_subst_nocommands_body(&self, cmd_text: &str) -> Option<String> {
        use tcl_lexer::TokenType;
        let inner = self.segment_source(cmd_text, 0, SourceSegmentationPurpose::NestedScript)?;
        let [inner_cmd] = inner.as_slice() else {
            return None;
        };
        let head = inner_cmd.texts.first()?;
        let texts = inner_cmd.args();
        let arg_refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        // A braced word is one `Str` token; only it is a template this can
        // substitute at compile time.
        let source = |index: usize| {
            let braced = inner_cmd
                .single_token_word
                .get(index + 1)
                .copied()
                .unwrap_or(false)
                && inner_cmd
                    .arg_tokens()
                    .get(index)
                    .is_some_and(|token| token.kind == TokenType::Str);
            crate::value_transfer::SourceWord::of(arg_refs.get(index).copied(), braced)
        };
        let plan =
            crate::value_transfer::literal_template_plan(self.registry, head, &arg_refs, source)?;
        if plan.kinds != SUBST_NOCOMMANDS_KINDS || self.proc_depth == 0 {
            return None;
        }
        let template = arg_refs.get(plan.operand.0)?;
        let scope = self.const_map_stack.last()?;
        crate::subst_nocommands::subst_nocommands(template, &plan, scope)
    }

    /// Try to lower `eval ?body?` to a static-body
    /// [`Statement::Block`] when the body is a brace-string literal,
    /// a const-mapped `$var`, or an `eval [list lit1 lit2 ...]`
    /// command-substitution shape. Returns `None` for dynamic
    /// bodies so the caller falls through to runtime barrier
    /// dispatch.
    fn try_lower_eval_static(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        error_context: Option<tcl_registry::InlineBodyErrorContext>,
    ) -> Option<Statement> {
        use tcl_lexer::TokenType;
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        // Single-body shape only: ``eval $body`` / ``eval {body}``.
        // The list form (``eval cmd arg1 arg2``) keeps runtime
        // semantics — joining the words with spaces is observable.
        if args.len() != 1 || arg_tokens.is_empty() {
            return None;
        }
        let body_tok = arg_tokens[0];
        // Barrier gate: a braced literal body might contain a
        // nested ``eval $x`` / ``uplevel $lvl {...}`` whose own body
        // is still dynamic.  Relaxing the outer barrier in that case
        // produces IR that runs a compiled inner barrier with a
        // still-dynamic shape — we'd lose the runtime barrier without
        // gaining static knowledge.  Reject and fall back to the
        // default Statement::Barrier dispatch.
        if body_tok.kind == TokenType::Str {
            let body_text = &args[0];
            if body_has_dynamic_barrier(body_text, self.registry, self.config) {
                return None;
            }
            // Parse gate: inlining lowers the body with the LSP-lenient
            // lowering, which gives malformed text a meaning C never gives it
            // — `eval {set y "a"b}` would quietly mean `set y ab` instead of
            // raising `extra characters after close-quote` (#1829).  C parses
            // an `eval` body when the command runs, so decline the relaxation
            // and fall back to the runtime barrier, which is the path a
            // non-literal `eval $body` already takes and which reports the
            // error correctly.
            if !matches!(
                tcl_lexer::first_parse_cut_image_checked(
                    &tcl_lexer::SourceImage::from_bytes(
                        self.guarded_body_text(body_tok, body_text).as_bytes(),
                        self.current_source_channel()
                    ),
                    self.config.nested()
                ),
                Ok(None)
            ) {
                return None;
            }
        }
        let body = if body_tok.kind == TokenType::Str {
            self.lower_body_from_tok(&args[0], Some(&body_tok), namespace)
        } else if body_tok.kind == TokenType::Var {
            // The script `eval` evaluates at run time is a unit of its own
            // (8.4 runs it directly), which creates no locals and reaches only
            // those the procedure compiled (`TclFindCompiledLocal`), and whose
            // errors name its own commands; folded into the body it would
            // create them and be logged as the body's text. Analysis keeps the
            // structure it can see.
            if self.target.is_bytecode() {
                return None;
            }
            let literal = self
                .original_native_written_argument_value(seg, 0)
                .or_else(|| self.const_map_lookup(&args[0]))?;
            if body_has_dynamic_barrier(&literal, self.registry, self.config) {
                return None;
            }
            self.lower_script(&literal, namespace, SourceSegmentationPurpose::NestedScript)
        } else if body_tok.kind == TokenType::Cmd {
            // ``eval [list lit1 lit2 ...]`` — synthesise the
            // body by joining the list's literal arguments and
            // re-lowering. The bracket-substitution text retains
            // the surrounding ``[...]``; strip them via
            // ``content_offset`` if present, otherwise the helper
            // strips them itself.
            let inner_text = if body_tok.content_offset > 0 {
                let start = u32::from(body_tok.content_offset) as usize;
                &args[0][start..args[0].len() - start]
            } else {
                args[0].trim_start_matches('[').trim_end_matches(']')
            };
            let synthesised = eval_list_literal_body(inner_text, self.config)?;
            // `eval` runs the list as one command without compiling it; a word
            // the release compiles apart stays with the runtime `eval`.
            let release = self
                .dialect
                .map_or(tcl_dialect::TclVersion::V9_0, |profile| {
                    profile.vm_runtime_version
                });
            if crate::const_subst::folded_literal_reads_apart(&synthesised, release) {
                return None;
            }
            self.lower_script(
                &synthesised,
                namespace,
                SourceSegmentationPurpose::NestedScript,
            )
        } else {
            return None;
        };
        Some(Statement::Block {
            span: seg.span,
            body,
            namespace: namespace.to_string(),
            tokens: Some(self.cmd_tokens(seg)),
            error_context,
        })
    }

    /// Retain a synthetic analysis body under its issued Module ID.
    /// Labels are displays; the body retains its own namespace/source receipt.
    /// Lambda parameters bind a fresh frame, while namespace bodies do not.
    fn register_body_unit(
        &mut self,
        label: &str,
        params: Vec<String>,
        span: tcl_lexer::Span,
        body: Script,
        is_lambda: bool,
    ) {
        let n = self.body_unit_count;
        self.body_unit_count += 1;
        let prefix = tcl_syntax::naming::unroot_rooted_key(label).unwrap_or(label);
        let qualified = tcl_syntax::naming::root_unrooted_key(&format!("{prefix}#{n}"));
        let short_name = tcl_syntax::naming::key_tail(label).to_owned();
        let id = self.module.retain_body_unit(Procedure {
            name: short_name,
            qualified_name: qualified,
            publication: None,
            declaration_site: body.executed_source.as_ref().map(|source| {
                crate::command_binding::CommandAllocationSite {
                    source: std::sync::Arc::clone(&source.origin),
                    offset: source.base(),
                }
            }),
            body_kind: if is_lambda {
                crate::ir::ProcedureBodyKind::Lambda
            } else {
                crate::ir::ProcedureBodyKind::NamespaceScript
            },
            params,
            span,
            body,
            params_raw: String::new(),
            body_source: None,
            body_offset: 0,
            namespace_scoped: false,
            base_priority: 500,
        });
        if is_lambda {
            self.module.lambda_body_units.insert(id);
        }
    }

    fn lower_apply(&mut self, seg: &SegmentedCommand) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();

        let tokens = self.cmd_tokens(seg);
        let selected = self
            .resolved_metadata_invocation(&tokens)
            .and_then(|invocation| {
                let dialect = invocation.dialect?;
                let values = (0..invocation.arguments.len())
                    .map(|index| invocation.argument_literal(index))
                    .collect::<Vec<_>>();
                let words = values
                    .iter()
                    .map(|value| {
                        value.as_deref().map_or(
                            tcl_registry::InvocationWord::Dynamic,
                            tcl_registry::InvocationWord::Literal,
                        )
                    })
                    .collect::<Vec<_>>();
                let arguments =
                    tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect);
                let tcl_registry::lambda_invocation::LambdaInvocationSelection::Selected(lambda) =
                    tcl_registry::lambda_invocation::select_lambda_invocation(arguments, 0)
                else {
                    return None;
                };
                let parameters = tcl_syntax::formal_params::parse_formal_parameters_in(
                    &lambda.parameters,
                    dialect.parameter_grammar()?,
                )
                .ok()?
                .into_iter()
                .map(|parameter| parameter.name)
                .collect();
                let source = self
                    .source_bindings
                    .as_ref()?
                    .executed_script_for_list_element(arg_tokens.first()?.span, 1)?
                    .clone();
                let namespace = self.original_body_namespace_context(&tokens, 0, &source);
                Some((lambda, parameters, source, namespace))
            });

        if let Some((lambda, params, source, namespace)) = selected {
            let body_offset = source.base();
            let body_length = source.text.len();
            let body_ns = join_namespace("::", &lambda.namespace);
            let body = self.in_procedure_frame(None, |lowerer| {
                lowerer.lower_original_body(source, namespace.as_ref())
            });

            // A body defined inside a TclOO method body is not registered
            // globally (`suppress_proc_register`), but a body unit is
            // analysis-only (never codegen-emitted), so it is always safe to
            // record for coverage.
            let span = tcl_lexer::Span::new(
                body_offset,
                body_offset + u32::try_from(body_length).unwrap_or(u32::MAX),
            );
            // The body retains its own lambda namespace; the label is a display.
            // tclsh8.6/9.0-confirmed: inside
            // `::foo::runIt`, `apply {{x} { helper $x } ::foo} b` calls
            // `::foo::helper`, not `::helper`.
            self.register_body_unit(&join_namespace(&body_ns, "apply"), params, span, body, true);
        }

        Statement::Barrier {
            span: seg.span,
            reason: "unsupported body command".into(),
            command: seg.name().into(),
            canonical_command: None,
            args: args.to_vec(),
            tokens: Some(self.cmd_tokens(seg)),
        }
    }

    /// `array for {keyVar valueVar} arrayName body` (Tcl 9.0).
    ///
    /// The body runs in the caller's frame with the two loop variables bound
    /// per entry. C Tcl compiles this to an `invokeStk` of `::tcl::array::for`
    /// with the body pushed as an unparsed literal — it does not compile the
    /// body — so, exactly like [`Self::lower_apply`], the call itself stays a
    /// runtime [`Statement::Barrier`] (codegen unchanged, byte-identical to C
    /// Tcl). But a braced literal body is walked in a fresh frame bound to the
    /// loop variables and recorded as a body unit so static analysis reaches
    /// inside it. A `$var` / `[cmd]` body stays fully opaque.
    fn lower_array_for(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        use tcl_lexer::TokenType;
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        // `array for {k v} arr body` → args = [for, {k v}, arr, body].
        // The body (index 3) must be a single braced-literal token to walk.
        let body_is_braced = arg_tokens.get(3).is_some_and(|t| t.kind == TokenType::Str)
            && arg_single.get(3).copied() == Some(true);
        // A varList Tcl would refuse to split binds nothing statically, so it
        // falls through to the opaque runtime barrier below. Decided before the
        // body is walked, so a rejected loop leaves no const-map traces behind.
        if args.len() == 4
            && body_is_braced
            && let Some(vars) = crate::registry_invocation::original_name_role_values(
                self.registry,
                &self.cmd_tokens(seg),
                1,
                tcl_registry::ArgRole::LoopVarList,
            )
        {
            let body_tok = &arg_tokens[3];
            // The body runs in the *caller's* frame (`vm.eval_source` in place),
            // so lower it inline — inheriting the caller's const-map — into a
            // loop-variable-bound `Foreach` rather than isolating it in a
            // fresh-frame body unit. The analysis CFG inlines this into the
            // caller unit (so a body `$re` resolves to a caller literal for
            // regex-source / taint), while codegen barriers it to the byte-
            // identical `::tcl::array::for` invoke — see `lower_foreach_dispatch`.
            let body = self.lower_body_from_tok(&args[3], Some(body_tok), namespace);
            return Statement::Foreach {
                span: seg.span,
                iterators: vec![ForeachIterator {
                    vars,
                    list_arg: args[2].clone(),
                    // `array for {k v} {arr} …` — a braced array-name word
                    // is a literal name, not a substitution.
                    list_braced: self.cmd_tokens(seg).arg_is_braced_literal(2),
                }],
                body,
                body_span: self.script_word_span(body_tok),
                is_lmap: false,
                raw_args: args.to_vec(),
                is_dict_iteration: false,
                is_array_iteration: true,
                raw_tokens: Some(self.cmd_tokens(seg)),
            };
        }

        // A dynamic (`$body` / `[cmd]`) body stays fully opaque: the runtime
        // barrier re-emits `array for …`.
        self.lower_default(seg, namespace)
    }

    fn lower_namespace_eval(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        // The body is walked from its *written* spelling, rebased by the body
        // word's own content offset, so the word must not substitute.
        // A `$body` / `[list …]` word's value is run-time data
        // that appears nowhere in this document: rebasing its unsubstituted
        // spelling fabricates statement spans that overrun the source —
        // `namespace eval ::ns [list $o fw]` rebases the whole `[list $o fw]`
        // text past the opening `[`, one byte off the end.
        // [`Self::guarded_body_text`] deliberately passes such words through
        // untouched because clamping run-time data proves nothing; refusing to
        // walk them is the caller-side half of that contract.  The namespace is
        // still entered at run time — the `Barrier` below re-emits the call —
        // the body just stays opaque to static analysis, exactly as
        // [`Self::lower_apply`] and [`Self::lower_array_for`] leave a dynamic
        // body opaque.
        if !seg_word_substitutes(seg, 3) {
            self.walk_namespace_eval_body(seg, namespace);
        }

        Statement::Barrier {
            span: seg.span,
            reason: "namespace eval".into(),
            command: "namespace".into(),
            canonical_command: None,
            args: seg.args().to_vec(),
            tokens: Some(self.cmd_tokens(seg)),
        }
    }

    /// A generic namespace handler can retain entered declarations while its
    /// original invocation remains generic. The exact source carrier supplies
    /// the body; no lexical fallback or executable hook is inferred here.
    fn retain_entered_namespace_body(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: &CommandTokens,
    ) {
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource {
            return;
        }
        let Some(invocation) = self.resolved_metadata_invocation(tokens) else {
            return;
        };
        if invocation.facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(LoweringHookId::NamespaceEval)
            || tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.proved_execution_target())
                .is_none_or(|target| !target.prepended.is_empty())
        {
            return;
        }
        let name_at = invocation.facts.argument_offset;
        let body_at = name_at + 1;
        if seg.args().len() != body_at + 1 {
            return;
        }
        let Some(child) = invocation.argument_literal(name_at) else {
            return;
        };
        let Some(token) = seg.arg_tokens().get(body_at) else {
            return;
        };
        let Some(source) = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.executed_script_for_word(token.span))
            .cloned()
        else {
            return;
        };
        let child_namespace = join_namespace(namespace, &child);
        let body_span = tcl_lexer::Span::new(
            source.base(),
            source.base() + u32::try_from(source.text.len()).unwrap_or(u32::MAX),
        );
        let previous = self.in_namespace_eval;
        self.in_namespace_eval = true;
        let namespace = self.original_body_namespace_context(tokens, body_at, &source);
        let body = self.lower_original_body(source, namespace.as_ref());
        self.in_namespace_eval = previous;
        self.register_body_unit(
            &join_namespace(&child_namespace, "namespace-eval"),
            vec![],
            body_span,
            body,
            false,
        );
    }

    /// Walk a `namespace eval` body inline and record it as a body unit.
    ///
    /// Only reached for a body word that does not substitute — see
    /// [`Self::lower_namespace_eval`] for why.
    fn walk_namespace_eval_body(&mut self, seg: &SegmentedCommand, namespace: &str) {
        let args = seg.args();
        let child_ns = join_namespace(namespace, &args[1]);
        let body_tok = seg.arg_tokens()[2];
        // `namespace eval` inlines whatever literal body word it is given,
        // including a compound `{…}x` one, so the rebase below needs the
        // clamp in [`Self::guarded_body_text`] — both for the body itself and
        // for the body unit's span derived from its length.
        let body_text = self.guarded_body_text(body_tok, &args[2]);
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let prev = self.in_namespace_eval;
        self.in_namespace_eval = true;
        let body = self.lower_body_from_tok(&args[2], Some(&body_tok), &child_ns);
        self.in_namespace_eval = prev;

        // A `namespace eval` body runs in the child namespace's own frame — its
        // straight-line statements are a real analysable script, so record them
        // as a body unit (no bound params: namespace vars, not locals).
        let span = tcl_lexer::Span::new(
            body_offset,
            body_offset + u32::try_from(body_text.len()).unwrap_or(u32::MAX),
        );
        // Original body context selects the child namespace; labels do not.
        // tclsh8.6-confirmed:
        // `namespace eval ::foo { helper }` calls `::foo::helper` even when
        // nested inside an unrelated proc, never a same-named proc in the
        // caller's own namespace.
        self.register_body_unit(
            &join_namespace(&child_ns, "namespace-eval"),
            vec![],
            span,
            body,
            false,
        );
    }

    /// Registry-resolved reads hidden inside the value word of a generic
    /// value-passthrough store.
    ///
    /// An alias/rename of `set` must remain a runtime [`Statement::Call`], but
    /// its one value word still has the same expression semantics as a direct
    /// `set`.  Capture those reads while the lowering alias map and resolved
    /// dialect context are live, rather than asking SSA to reconstruct command
    /// bindings from a flattened call later.
    fn nested_expr_reads_in_set_value(
        &self,
        seg: &SegmentedCommand,
        namespace: &str,
        resolved_command: &str,
        prepend_n: usize,
    ) -> Vec<String> {
        if prepend_n != 0
            || seg.args().len() != 2
            || !matches!(
                seg.argv.get(2).map(|token| token.kind),
                Some(TokenType::Cmd)
            )
            || !seg.single_token_word.get(2).copied().unwrap_or(false)
            || self
                .registry
                .get(resolved_command)
                .and_then(|spec| spec.lowering_hook)
                != Some(LoweringHookId::Set)
        {
            return Vec::new();
        }

        let value = &seg.args()[1];
        let inner = value
            .strip_prefix('[')
            .and_then(|text| text.strip_suffix(']'))
            .unwrap_or(value);
        let context = self
            .dialect_context
            .as_deref()
            .map(tcl_registry::model::ContextRegistry::context);
        let tokens = self.cmd_tokens(seg);
        let Some((_, _, expression, _)) = extract_proved_expr_arg(
            (Some(&tokens), 2),
            inner,
            &self.aliases,
            namespace,
            self.registry,
            context,
            self.config,
        ) else {
            return Vec::new();
        };
        parse_expr_for_profile_with_lexer_config(&expression, self.dialect, self.config)
            .vars_element_qualified_with_config(self.config)
            .into_iter()
            .collect()
    }

    fn retained_metadata_descriptor_label(&self, tokens: &CommandTokens) -> Option<String> {
        self.resolved_metadata_invocation(tokens)
            .map(|invocation| invocation.facts.descriptor_key().label().to_owned())
    }

    fn advisory_method_instance_names(&self, statement: &Statement) -> Vec<String> {
        let Statement::Call { args, tokens, .. } = statement else {
            return Vec::new();
        };
        let possible = tokens.as_ref().and_then(|tokens| {
            let context = self.invocation_metadata_context()?;
            let advice = crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                self.registry, Some(context), tokens,
            )?;
            if advice.unknown_residual || advice.may_be_absent || advice.candidates.is_empty() {
                return None;
            }
            let mut names = Vec::new();
            for candidate in &advice.candidates {
                names.extend(candidate.possible_method_instance_variable_names(
                    self.registry,
                    context,
                    tokens,
                )?);
            }
            Some(names)
        });
        // Unavailable source advice only widens the textual protection inventory.
        possible.unwrap_or_else(|| args.clone())
    }

    fn resolved_metadata_invocation(
        &self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::ResolvedStatementInvocation> {
        // Explicit standalone authoring has its own metadata context. An
        // unavailable supplied owner cannot enter the scalar compatibility API.
        match self.invocation_metadata_context() {
            Some(context) => {
                crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                    self.registry,
                    Some(context),
                    tokens,
                )
            }
            None if self.allows_standalone_registry_metadata() => {
                crate::registry_invocation::resolved_tokens_invocation(self.registry, None, tokens)
            }
            None => None,
        }
    }

    fn allows_standalone_registry_metadata(&self) -> bool {
        self.source_entry_origin == SourceEntryOrigin::Authoring
            && self.metadata_origin == LoweringMetadataOrigin::Standalone
            && self.metadata_input.is_none()
            && self.logical_source_input.is_none()
            && self.vendor_source_input.is_none()
            && self.dialect_context.is_none()
    }

    fn source_metadata_input(&self) -> crate::registry_invocation::InvocationMetadataInput<'_> {
        use crate::registry_invocation::InvocationMetadataInput;
        if let Some(input) = self
            .metadata_input
            .as_ref()
            .or(self.logical_source_input.as_ref())
            .or(self.vendor_source_input.as_ref())
        {
            return InvocationMetadataInput::SuppliedSource(Some(input));
        }
        if let Some(context) = self.dialect_context.as_ref() {
            return InvocationMetadataInput::Supplied(context);
        }
        if self.allows_standalone_registry_metadata() {
            InvocationMetadataInput::Standalone
        } else {
            InvocationMetadataInput::SuppliedSource(None)
        }
    }

    fn invocation_metadata_context(
        &self,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'_>> {
        if let Some(input) = self
            .metadata_input
            .as_ref()
            .or(self.logical_source_input.as_ref())
            .or(self.vendor_source_input.as_ref())
        {
            return crate::registry_invocation::InvocationMetadataContext::for_source_input(
                self.registry,
                input,
                self.config,
                self.dialect,
            );
        }
        match self.dialect_context.as_deref() {
            Some(context) => Some(context.into()),
            None if self.allows_standalone_registry_metadata() => self
                .dialect
                .or_else(|| self.registry.profile())
                .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                .map(crate::registry_invocation::InvocationMetadataContext::from),
            None => None,
        }
    }

    /// Default lowering: generic [`Statement::Call`] with registry-based
    /// arg roles.
    fn lower_default(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let tokens = self.attach_evaluated_body(seg, namespace, self.cmd_tokens_boxed(seg));
        // Only the evaluated-body attachment recurses; the rest of the
        // statement is built in a non-inlined leaf, so its locals are not
        // held in the frame of every enclosing body level.
        self.lower_default_with_tokens(seg, namespace, tokens)
    }

    // The generic path deliberately assembles every registry-owned call fact
    // in one place so consumers never need per-command recovery logic.
    #[allow(clippy::too_many_lines)]
    #[inline(never)]
    /// Whether the document declares `command` with a word this call runs as
    /// code: a body, a lambda or a command prefix.
    fn declares_executable_word(&self, command: &str, args: &[String]) -> bool {
        let Some(declared) = self
            .source_declared_commands
            .as_ref()
            .or(self.declared_commands)
        else {
            return false;
        };
        let Some(command) =
            tcl_registry::model::DocumentCommandSurface::new(self.registry, Some(declared))
                .declared_command(command)
        else {
            return false;
        };
        [
            ArgRole::Body,
            ArgRole::LambdaLiteral,
            ArgRole::CommandPrefix,
        ]
        .into_iter()
        .any(|role| !command.arg_indices_for_role(role, args.len()).is_empty())
    }

    fn tokens_realm(&self, tokens: &CommandTokens) -> tcl_dialect::model::InvocationRealm {
        tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
            .unwrap_or(self.invocation_realm)
    }

    fn lower_default_with_tokens(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: Box<CommandTokens>,
    ) -> Statement {
        let tokens = *tokens;
        let cmd_name = seg.name();
        let args = seg.args();
        let Some(target) = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.proved_execution_target())
            .filter(|target| target.registry_backed)
        else {
            // A document-declared command with a word it runs as code lowers
            // to the barrier a registry body command lowers to, whatever the
            // frame effect it states (#2533).
            if self.declares_executable_word(cmd_name, args) {
                return body_barrier(seg, None, tokens);
            }
            return source_body_syntax::generic_call(seg.span, cmd_name, args, Some(tokens));
        };
        let Some(role_cmd) = self.retained_metadata_descriptor_label(&tokens) else {
            return untyped_call(seg, None, tokens);
        };
        let prepend_n = target.prepended.len();
        let Some(prefix) = literal_words(&target.prepended) else {
            // A retained alias operand still contributes one argv slot when
            // its value is unknown. Presentation text must never license
            // specialised lowering or move the remaining operand positions.
            return untyped_call(seg, Some(role_cmd), tokens);
        };
        let role_args: Vec<String> = prefix.into_iter().chain(args.iter().cloned()).collect();
        let canonical = (role_cmd != cmd_name).then(|| role_cmd.clone());

        let context = self.invocation_metadata_context();
        let invocation = context.and_then(|context| {
            crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                self.registry,
                Some(context),
                &tokens,
            )
        });
        let facts = invocation.as_ref().map(|invocation| &*invocation.facts);
        let realm = self.tokens_realm(&tokens);
        let RoleIndices {
            executable: executable_indices,
            var_writes: var_indices,
            var_reads: var_read_indices,
            reads_before_write,
        } = RoleIndices::of(facts);

        // A stored callback is data for this invocation, not executable code
        // that can complete or mutate the caller before the command returns.
        // Every live Body, LambdaLiteral, or CommandPrefix whose exact timing
        // is SameInvocation needs the opaque runtime barrier. This is checked
        // per index because one invocation may mix immediate and deferred code.
        let has_same_invocation_executable = executable_indices.iter().any(|&index| {
            invocation
                .as_ref()
                .and_then(|invocation| {
                    invocation.with_metadata_schema(self.registry, context?, realm, |selected| {
                        selected.authored_source_script_timing_at(index)
                    })
                })
                .is_none_or(|timing| timing == tcl_registry::ScriptTiming::SameInvocation)
        });
        if has_same_invocation_executable && tokens.evaluated_body().is_none() {
            return body_barrier(seg, canonical, tokens);
        }

        if !var_indices.is_empty() || !var_read_indices.is_empty() {
            // A brace-quoted name word (`set {$n} 1`, `unset {$n}`) is Tcl's
            // literal spelling for a name that contains `$` / `[`: it
            // substitutes nothing, so the word's content **is** the variable
            // name. The de-braced `args` text cannot show that;
            // the word's own token kind can.
            let variable_at = |index: usize| -> Option<String> {
                let invocation = invocation.as_ref()?;
                let effective = &invocation.effective;
                let word = effective.words.get(index + 1)?;
                let value = invocation.argument_literal(index)?;
                let braced = matches!(word, crate::ir::WordExpr::BracedLiteral { .. })
                    || effective.written_argument(index).is_none();
                let name = crate::naming::element_var_name_braced(&value, braced);
                if invocation.with_metadata_schema(self.registry, context?, realm, |selected| {
                    selected.authored_source_option_variable_scope_at(index)
                }) == Some(tcl_registry::VariableScope::Global)
                    && !tcl_syntax::naming::is_absolute(name)
                {
                    Some(tcl_syntax::naming::root_unrooted_key(name))
                } else {
                    Some(name.to_owned())
                }
            };
            let var_defs: Vec<String> = var_indices
                .iter()
                .filter_map(|&index| variable_at(index))
                .collect();
            let mut var_reads: Vec<String> = var_read_indices
                .iter()
                .filter_map(|&index| variable_at(index))
                .collect();
            let nested_expr_reads =
                self.nested_expr_reads_in_set_value(seg, namespace, &role_cmd, prepend_n);
            var_reads.extend(nested_expr_reads);
            let reads_own_defs = reads_before_write
                || var_reads
                    .iter()
                    .any(|read| var_defs.iter().any(|definition| definition == read));
            return Statement::Call {
                span: seg.span,
                command: cmd_name.into(),
                canonical_command: canonical.clone(),
                args: args.to_vec(),
                defs: var_defs,
                reads: var_reads,
                reads_own_defs,
                safe_on_uninit: self.safe_on_uninit(&role_cmd, &role_args),
                tokens: Some(tokens.clone()),
                foreach_groups: None,
            };
        }

        untyped_call(seg, canonical, tokens)
    }

    // TclOO method-body lowering (method purity for O126)

    /// Populate `module.methods` from the fully-assembled IR.
    ///
    /// Runs as a cache-independent post-pass (mirroring
    /// [`populate_trace_facts`]): clone the top-level script and proc
    /// bodies, then walk them — plus any `namespace eval` blocks — for
    /// `oo::class create` / `oo::define` barriers and lift each
    /// `method` / `constructor` / `destructor` body to an
    /// [`MethodDef`]. Method bodies are lowered for analysis only —
    /// codegen never reads `module.methods`, and the barrier emitted
    /// for the class command is unchanged, so bytecode is
    /// byte-identical.
    pub fn extract_oo_methods_pass(&mut self) {
        let retained = self.module_source_bindings.take();
        let enclosing = self.exchange_boxed_source_bindings(retained);
        // The walk borrows each script out of the module rather than cloning
        // it: the derived `Clone` of cap-deep structured IR recurses several
        // frames per nesting level, which overflowed a 2 MiB thread at the
        // structured-lowering depth cap. No member extraction reads these
        // scripts back from the module.
        let top_level = std::mem::take(&mut self.module.top_level);
        let top_namespace = if self.module.top_level_namespace.is_empty() {
            "::".to_owned()
        } else {
            self.module.top_level_namespace.clone()
        };
        self.walk_for_oo_methods(&top_level.statements, &top_namespace);
        self.module.top_level = top_level;
        let procs: Vec<(crate::ir::ProcedureId, String)> = self
            .module
            .procedures
            .iter()
            .filter_map(|(id, p)| Some((*id, p.body.namespace_context.as_deref()?.advisory_key()?)))
            .collect();
        for (id, namespace) in &procs {
            let Some(body) = self
                .module
                .procedures
                .get_mut(id)
                .map(|procedure| std::mem::take(&mut procedure.body))
            else {
                continue;
            };
            self.walk_for_oo_methods(&body.statements, namespace);
            if let Some(procedure) = self.module.procedures.get_mut(id) {
                procedure.body = body;
            }
        }
        // Every definition block for every class has now been walked, so the
        // per-class instance-variable union is complete.  Merge it into each
        // method: a method extracted from an early block must still see state
        // a later `oo::define` block declared for its class, or a consumer
        // reading `MethodDef::instance_vars` (the optimiser's method-body
        // propagation gate, `elimination.rs`'s dead-store protection) would
        // treat object state as a private local.  Order-free by construction —
        // the union does not depend on which block was walked first.
        for method in self.module.methods.values_mut() {
            if method.body.executed_source.is_none()
                && let Some(class_vars) = self.class_instance_vars.get(&method.class_name)
            {
                method.instance_vars.extend(class_vars.iter().cloned());
            }
        }
        self.module_source_bindings = self.exchange_boxed_source_bindings(enclosing);
    }

    /// Recursive walk for `oo::class` / `oo::define` definition
    /// barriers. Descends `namespace eval` blocks (tracking the active
    /// namespace) and extracts methods from any class/define command
    /// carrying a static braced body. The lowerer evaluates a
    /// `namespace eval` body inline and discards it (emitting a
    /// `Barrier`), so rather than descending a preserved block this
    /// walk re-segments the barrier's body token instead (see
    /// [`Self::walk_segments_for_oo`]).
    fn walk_for_oo_methods(&mut self, statements: &[Statement], namespace: &str) {
        for stmt in statements {
            match stmt {
                // Folded `eval` / `uplevel` bodies survive as Blocks.
                Statement::Block {
                    body,
                    namespace: block_ns,
                    ..
                } => {
                    let ns = if block_ns.is_empty() {
                        namespace
                    } else {
                        block_ns.as_str()
                    };
                    self.walk_for_oo_methods(&body.statements, ns);
                }
                Statement::Call {
                    tokens: Some(ct), ..
                }
                | Statement::Barrier {
                    tokens: Some(ct), ..
                } => {
                    if let Some(call) = self.classify_definer_call(ct) {
                        self.extract_definer_members(
                            call,
                            ct,
                            ct.argv[call.body_idx].start() + 1,
                            namespace,
                        );
                    } else if self.definer_invocation_may_supply_body(ct) {
                        self.module.oo_evidence.unretained_executable_roots = true;
                    } else if self.is_namespace_eval_shape(ct) {
                        // The body was lowered inline and discarded;
                        // re-segment it to find classes defined directly
                        // inside the namespace.
                        let child_ns = join_namespace(namespace, &ct.argv_texts[2]);
                        let body_off = ct.argv[3].start() + 1;
                        let Some(segments) = self.segment_source(
                            &ct.argv_texts[3],
                            body_off,
                            SourceSegmentationPurpose::NestedScript,
                        ) else {
                            self.module.oo_evidence.unretained_executable_roots = true;
                            continue;
                        };
                        self.in_word_space(&ct.argv_texts[3], body_off, |this| {
                            this.walk_segments_for_oo(&segments, &child_ns);
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// Segment-level counterpart of [`Self::walk_for_oo_methods`] used
    /// for `namespace eval` bodies (which the lowerer discards). Walks
    /// re-segmented commands, recursing through nested `namespace eval`
    /// and extracting members from any registry-classified definer block.
    fn walk_segments_for_oo(&mut self, segments: &[SegmentedCommand], namespace: &str) {
        for seg in segments {
            if seg.texts.is_empty() {
                continue;
            }
            let tokens = self.cmd_tokens(seg);
            if seg.is_partial {
                if self.definer_invocation_may_supply_body(&tokens) {
                    self.module.oo_evidence.unretained_executable_roots = true;
                }
                continue;
            }
            if let Some(call) = self.classify_definer_call(&tokens) {
                let body = seg.argv[call.body_idx];
                let off = body.span.start() + u32::from(body.content_offset);
                self.extract_definer_members(call, &tokens, off, namespace);
            } else if self.definer_invocation_may_supply_body(&tokens) {
                self.module.oo_evidence.unretained_executable_roots = true;
            } else if self.is_namespace_eval_shape(&tokens) {
                let child_ns = join_namespace(namespace, &seg.texts[2]);
                let off = seg.argv[3].span.start() + u32::from(seg.argv[3].content_offset);
                let Some(sub) = self.segment_source(
                    &seg.texts[3],
                    off,
                    SourceSegmentationPurpose::NestedScript,
                ) else {
                    self.module.oo_evidence.unretained_executable_roots = true;
                    continue;
                };
                self.in_word_space(&seg.texts[3], off, |this| {
                    this.walk_segments_for_oo(&sub, &child_ns);
                });
            }
        }
    }

    /// Retain each selected class-relation member with its authentic outer
    /// definer. Value formation and target selection belong to the source owner;
    /// punctuation in the written operand never determines ancestry here.
    fn record_class_relation_member(
        &mut self,
        call: &DefinerCall,
        definer: &CommandTokens,
        seg: &SegmentedCommand,
        class_qname: &str,
    ) -> bool {
        let head = seg.texts[0].as_str();
        let Some(member) = self.source_definition_member(call.grammar, head) else {
            return false;
        };
        if member.all_args_ref != Some(tcl_registry::definer::MemberRefKind::Class) {
            return false;
        }
        self.module.class_relations.push(crate::ir::ClassRelation {
            declaring_class: class_qname.to_owned(),
            definer: std::sync::Arc::new(definer.clone()),
            member: std::sync::Arc::new(self.cmd_tokens(seg)),
            target_word: call.name_idx,
            body_word: call.body_idx,
        });
        true
    }

    /// Retain each member's actual body ID, typed kind and declaring source owner.
    /// The selected [`DefinerCall::grammar`] supplies recognition and operand layout.
    ///
    /// `texts` is the definer command's full per-word text array;
    /// `body_content_offset` is the absolute source offset of the first
    /// byte inside the body's braces.
    fn extract_definer_members(
        &mut self,
        call: DefinerCall,
        definer: &CommandTokens,
        body_content_offset: u32,
        namespace: &str,
    ) {
        let call = DefinerCall {
            definition_scope: Some(self.module.retain_definition_script_source(
                definer,
                &crate::ir::DefinitionScriptLayout {
                    body: call.body_idx,
                    name: call.name_idx,
                    per_object: call.per_object,
                    offset: body_content_offset,
                    grammar: call.grammar,
                    descriptor: call.descriptor,
                },
                self.registry,
            )),
            ..call
        };
        let texts = &definer.argv_texts;
        let body_text = texts[call.body_idx].as_str();
        let target = definer
            .words()
            .first()
            .and_then(|head| head.original_written_span(self.module.source.bytes()))
            .and_then(|span| {
                crate::registry_invocation::original_native_compiler_words(
                    &self.module.source,
                    definer.words(),
                    span.start(),
                    self.config,
                )
            })
            .and_then(|words| {
                words
                    .get(call.name_idx)
                    .and_then(crate::registry_invocation::original_source_receiver_name_operand)
            })
            .filter(|target| match target {
                crate::registry_invocation::SourceReceiverNameOperand::Literal(name)
                | crate::registry_invocation::SourceReceiverNameOperand::ScalarVariable(name) => {
                    !name.as_bytes().contains(&0)
                }
            });
        let class_qname = match (call.per_object, target) {
            // This label reports per-object source units; definition IDs own their scope.
            (
                true,
                Some(
                    crate::registry_invocation::SourceReceiverNameOperand::Literal(name)
                    | crate::registry_invocation::SourceReceiverNameOperand::ScalarVariable(name),
                ),
            ) => format!("::@objdefine@::{name}"),
            (false, Some(crate::registry_invocation::SourceReceiverNameOperand::Literal(name))) => {
                qualify_proc_name(namespace, &name)
            }
            _ => {
                self.module.oo_evidence.dynamic_target = true;
                self.module.oo_evidence.unretained_executable_roots = true;
                return;
            }
        };
        let Some(segments) = self.segment_source(
            body_text,
            body_content_offset,
            SourceSegmentationPurpose::NestedScript,
        ) else {
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        };

        let surface = self
            .invocation_metadata_context()
            .map(|context| context.context().authoring_query());
        let class_ivars = declared_member_vars(call.grammar, &segments, surface, self.config);
        // This block sees only its own declarations; a sibling `oo::define`
        // block may declare more state for the same class, and may be walked
        // *after* the methods that use it.  Accumulate the whole-class union
        // for `extract_oo_methods_pass`'s merge — see
        // [`Self::class_instance_vars`].  A per-object block accumulates
        // under its own synthetic key, so per-object `variable` declarations
        // stay per-object by construction.
        if !class_ivars.is_empty() {
            self.class_instance_vars
                .entry(class_qname.clone())
                .or_default()
                .extend(class_ivars.iter().cloned());
        }

        self.in_word_space(body_text, body_content_offset, |this| {
            this.extract_members_from_segments(
                &call,
                definer,
                &segments,
                &class_qname,
                &class_ivars,
                namespace,
            );
        });
    }

    /// The per-segment member walk behind [`Self::extract_definer_members`],
    /// factored out so a wrapper's block form (`self { … }` /
    /// `private { … }`) can recurse into its nested definition script with
    /// the same class context.
    fn extract_members_from_segments(
        &mut self,
        call: &DefinerCall,
        definer: &CommandTokens,
        segments: &[SegmentedCommand],
        class_qname: &str,
        class_ivars: &HashSet<String>,
        namespace: &str,
    ) {
        for seg in segments {
            if seg.texts.is_empty() {
                continue;
            }
            let head = seg.texts[0].as_str();
            let member_supplies_body = self.member_invocation_supplies_body(call.grammar, seg);
            if seg.is_partial {
                self.module.oo_evidence.unretained_executable_roots |= member_supplies_body;
                continue;
            }
            if self.record_class_relation_member(call, definer, seg, class_qname) {
                continue;
            }
            let Some(member) = self.source_definition_member(call.grammar, head) else {
                // Definition scripts can call ordinary absolute commands (or
                // dynamically installed definition helpers). Their execution
                // is not represented by a MethodDef root.
                self.module.oo_evidence.unretained_executable_roots = true;
                continue;
            };
            // A wrapper member (`self …`, `private …`, itcl's access
            // modifiers) either prefixes an inner member (shift one place
            // right) or — for `wrapper_block_body` wrappers — carries a
            // whole nested definition script to recurse into.
            let (member, kw, base) = match member.kind {
                tcl_registry::definer::MemberKind::Wrapper => match seg.texts.get(1) {
                    Some(inner) if self.source_definition_member(call.grammar, inner).is_some() => {
                        let inner_member = self
                            .source_definition_member(call.grammar, inner)
                            .expect("selected source member");
                        // No double wrapping (`self private method …` is not
                        // a real Tcl shape).
                        if inner_member.kind != tcl_registry::definer::MemberKind::Flat {
                            self.module.oo_evidence.unretained_executable_roots |=
                                member_supplies_body;
                            continue;
                        }
                        (inner_member, inner.as_str(), 2usize)
                    }
                    Some(_)
                        if member.wrapper_block_body
                            && seg.texts.len() >= 2
                            && seg_word_is_static_braced(seg, 1) =>
                    {
                        // `self { … }` / `private { … }` — a nested
                        // definition script with the same member grammar.
                        let block_tok = seg.argv[1];
                        let off = block_tok.span.start() + u32::from(block_tok.content_offset);
                        let Some(sub) = self.segment_source(
                            &seg.texts[1],
                            off,
                            SourceSegmentationPurpose::NestedScript,
                        ) else {
                            self.module.oo_evidence.unretained_executable_roots = true;
                            continue;
                        };
                        // A `self { variable v }` declares per-class-object
                        // state, not instance state; keep it out of the
                        // instance union — only the members are lifted.
                        let wrapped_call = DefinerCall { ..*call };
                        let empty = HashSet::new();
                        let moves_off_the_instances = member
                            .wrapper_shift
                            .and_then(|shift| shift.receiver)
                            .is_some_and(|receiver| receiver != MemberReceiver::Instance);
                        let ivars = if moves_off_the_instances {
                            &empty
                        } else {
                            class_ivars
                        };
                        self.in_word_space(&seg.texts[1], off, |this| {
                            this.extract_members_from_wrapper_block(
                                &wrapped_call,
                                &sub,
                                class_qname,
                                ivars,
                                namespace,
                                head,
                            );
                        });
                        continue;
                    }
                    _ => {
                        self.module.oo_evidence.unretained_executable_roots |= member_supplies_body;
                        continue;
                    }
                },
                tcl_registry::definer::MemberKind::Flat => (member, head, 1usize),
                // Flag-keyed bodies (`property … -get/-set …`) are accessor
                // scripts, not method frames — no unit today (documented
                // limit).
                tcl_registry::definer::MemberKind::FlagKeyed => {
                    self.module.oo_evidence.unretained_executable_roots |= member_supplies_body;
                    continue;
                }
            };
            let frame = self.member_frame_for_segment(call, seg);
            self.extract_one_member(
                MemberExtraction {
                    call,
                    seg,
                    member,
                    kw,
                    base,
                    frame,
                },
                class_qname,
                class_ivars,
                namespace,
            );
        }
    }

    fn member_frame_for_segment(&self, call: &DefinerCall, seg: &SegmentedCommand) -> MemberFrame {
        let effective = statement_words(&self.cmd_tokens(seg), self.config);
        let words: Vec<_> = effective
            .iter()
            .map(|word| word.as_registry_word())
            .collect();
        MemberFrame::of_row(
            call.grammar,
            call.grammar
                .member_row(0, InvocationArguments::structured(&words), None)
                .as_ref(),
        )
    }

    /// Recurse into a wrapper's block form with the wrapper prefixed onto
    /// each member — `self { method m … }` records `m` as a class-object
    /// method, and `private { method m … }` as an instance method, exactly
    /// like their prefix spellings, because the registry answers the prefixed
    /// statement's row with the wrapper's shift applied.
    fn extract_members_from_wrapper_block(
        &mut self,
        call: &DefinerCall,
        segments: &[SegmentedCommand],
        class_qname: &str,
        class_ivars: &HashSet<String>,
        namespace: &str,
        wrapper: &str,
    ) {
        for seg in segments {
            if seg.texts.is_empty() {
                continue;
            }
            let head = seg.texts[0].as_str();
            let member_supplies_body = self.member_invocation_supplies_body(call.grammar, seg);
            if seg.is_partial {
                self.module.oo_evidence.unretained_executable_roots |= member_supplies_body;
                continue;
            }
            let Some(member) = self.source_definition_member(call.grammar, head) else {
                self.module.oo_evidence.unretained_executable_roots = true;
                continue;
            };
            if member.kind != tcl_registry::definer::MemberKind::Flat {
                self.module.oo_evidence.unretained_executable_roots |= member_supplies_body;
                continue;
            }
            let effective = statement_words(&self.cmd_tokens(seg), self.config);
            let words: Vec<_> = std::iter::once(InvocationWord::Literal(wrapper))
                .chain(effective.iter().map(|word| word.as_registry_word()))
                .collect();
            let frame = MemberFrame::of_row(
                call.grammar,
                call.grammar
                    .member_row(0, InvocationArguments::structured(&words), None)
                    .as_ref(),
            );
            self.extract_one_member(
                MemberExtraction {
                    call,
                    seg,
                    member,
                    kw: head,
                    base: 1,
                    frame,
                },
                class_qname,
                class_ivars,
                namespace,
            );
        }
    }

    /// Lift one body-bearing member call to a [`MethodDef`] unit, when its
    /// grammar layout and this consumer's kind routing say it opens a
    /// method frame.
    // One grammar-driven member extraction with shared validation paths.
    #[allow(clippy::too_many_lines)]
    fn extract_one_member(
        &mut self,
        ex: MemberExtraction<'_, '_>,
        class_qname: &str,
        class_ivars: &HashSet<String>,
        namespace: &str,
    ) {
        let MemberExtraction {
            call,
            seg,
            member,
            kw,
            base,
            frame,
        } = ex;
        let args = &seg.texts[base..];
        let Some(context) = self
            .invocation_metadata_context()
            .filter(|context| context.matches_registry(self.registry))
        else {
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        };
        let surface = Some(context.context().authoring_query());
        // Argument layout comes from the grammar: which relative index (0-
        // based after the keyword) is the body / name / parameter list.
        let Some(body_rel) = self
            .source_member_body_indices(call.grammar, kw, seg, base)
            .and_then(|indices| indices.into_iter().find(|&index| index < args.len()))
        else {
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        };
        let kind = match frame {
            MemberFrame::Opens(kind) => kind,
            MemberFrame::NoFrame => {
                self.module.oo_evidence.unretained_executable_roots = true;
                return;
            }
            // The layout the body word sits in is unknowable (a computed word
            // where an optional one may stand): whichever method it defines,
            // no scan can read it, so the class abstains as a whole — the
            // same answer a computed name gives below.
            MemberFrame::Unreadable => {
                self.module
                    .oo_unanalysed_classes
                    .insert(class_qname.to_string());
                self.module.oo_evidence.unretained_executable_roots = true;
                return;
            }
        };
        // A member that also declares a variable (itcl `variable NAME ?init?
        // ?configbody?`, snit 1.x `onconfigure`) is a declaration whose
        // trailing script is not an ordinary method frame — skipped
        // (documented limit; `MemberFrame::of_row` already excludes them by
        // effect, this keeps the exclusion structural too).
        if member
            .indices_for_call_in(args, surface, ArgRole::VarWrite)
            .next()
            .is_some()
        {
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        }
        let b_idx = base + body_rel;
        let name_owned: String;
        let name: &str = if let Some(rel) = member
            .indices_for_call_in(args, surface, ArgRole::Name)
            .next()
        {
            let Some(n) = seg.texts.get(base + rel) else {
                self.module.oo_evidence.unretained_executable_roots = true;
                return;
            };
            n.as_str()
        } else {
            // Nameless members: the synthetic id is the keyword, matching
            // the established `<constructor>` / `<destructor>` scheme.
            name_owned = format!("<{kw}>");
            &name_owned
        };
        // Dynamic method names / non-static bodies are left un-lowered —
        // and recorded as an unanalysable member of the class: any method of
        // the class may have been (re)defined with a body no scan can read,
        // so per-class analysis must abstain for the whole class.
        if name.contains('$') || name.contains('[') || !seg_word_is_static_braced(seg, b_idx) {
            self.module
                .oo_unanalysed_classes
                .insert(class_qname.to_string());
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        }
        // A parameter list Tcl would reject defines no method, and the class as
        // a whole becomes unanalysable — the same abstention the dynamic name /
        // body check above makes.
        let Some(params) = member_param_names(
            member,
            args,
            seg,
            base,
            WordValueRules::from_config(&self.config),
            self.invocation_dialect
                .and_then(tcl_registry::InvocationDialect::parameter_grammar),
            surface,
        ) else {
            self.module
                .oo_unanalysed_classes
                .insert(class_qname.to_string());
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        };
        // Lower the method body in its own frame (fresh const-map +
        // proc-depth) so the enclosing scope's tracked scalars
        // don't leak into the body's barrier-relaxation gate —
        // exactly as the `proc` case does — and suppress
        // nested-proc registration while doing so.
        let body_tok = seg.argv[b_idx];
        let body_off = body_tok.span.start() + u32::from(body_tok.content_offset);
        let execution_namespace = match call.grammar.member_current_namespace() {
            tcl_registry::definer::MemberCurrentNamespace::DefinedEntity => {
                crate::ir::ExecutionNamespace::exact(class_qname)
            }
            tcl_registry::definer::MemberCurrentNamespace::RuntimeReceiver => {
                crate::ir::ExecutionNamespace::RuntimeSelected
            }
        };
        // A runtime receiver namespace cannot be named during lowering. Keep
        // the lexical namespace for source reconstruction; the typed
        // execution context carried by MethodDef makes every proof consumer
        // abstain from relative command-resolution assumptions.
        let lowering_namespace = execution_namespace.for_head("").unwrap_or(namespace);
        self.proc_depth += 1;
        self.const_map_stack.push(HashMap::new());
        let prev_suppress = self.suppress_proc_register;
        self.suppress_proc_register = true;
        let retained = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.executed_script_for_word(body_tok.span).cloned());
        let body_script = if let Some(source) = retained {
            let tokens = self.cmd_tokens(seg);
            let namespace = b_idx
                .checked_sub(1)
                .and_then(|argument| {
                    self.original_body_namespace_context(&tokens, argument, &source)
                })
                .or_else(|| {
                    self.source_bindings
                        .as_ref()?
                        .executed_script_namespace_context(&source)
                });
            match namespace {
                Some(namespace) => self.lower_original_body(source, Some(&namespace)),
                // No receiver namespace is retained for a runtime-selected
                // body: report it through the same lexical approximation as an
                // unretained one, under the MethodDef's runtime-selected context.
                // It is not an original receiver frame, so it keeps no
                // executed source.
                None if execution_namespace == crate::ir::ExecutionNamespace::RuntimeSelected => {
                    let mut script =
                        self.lower_body(seg.texts[b_idx].as_str(), body_off, lowering_namespace);
                    script.executed_source = None;
                    script
                }
                None => self.lower_original_body(source, None),
            }
        } else {
            self.lower_body(seg.texts[b_idx].as_str(), body_off, lowering_namespace)
        };
        self.suppress_proc_register = prev_suppress;
        self.const_map_stack.pop();
        self.proc_depth -= 1;
        if self.method_body_may_install_definition(&body_script) {
            self.module.oo_evidence.unretained_executable_roots = true;
        }

        // Instance vars in scope: class-level decls (plus the grammar's
        // implicit member-body variables — snit's `self`/`selfns`/`type`/
        // `options`, a widget's `win`/`hull`, itcl's `this`) plus this
        // method's own top-level `variable` declarations. A write to any of
        // these mutates object state (impure for O126).
        let mut method_ivars = class_ivars.clone();
        method_ivars.extend(call.grammar.implicit_vars.iter().map(|s| (*s).to_string()));
        for statement in &body_script.statements {
            method_ivars.extend(self.advisory_method_instance_names(statement));
        }

        let original_receiver_context = body_script.executed_source.as_deref().and_then(|source| {
            self.source_bindings
                .as_ref()?
                .original_receiver_body_context(source)
        });
        let original_parameters = body_script.executed_source.as_deref().and_then(|source| {
            self.source_bindings
                .as_ref()?
                .original_script_formals(source)
        });
        if body_script.executed_source.is_some() {
            method_ivars.clear();
        }
        let def = MethodDef {
            definition_scope: call.definition_scope,
            class_name: class_qname.to_string(),
            method_name: name.to_string(),
            params,
            body: body_script,
            execution_namespace,
            kind,
            span: Some(seg.span),
            instance_vars: method_ivars,
            original_receiver_context,
            original_parameters,
        };
        self.module.retain_method(def);
    }
}

/// Formal-parameter names of a body-bearing member, read from the word its
/// grammar marks as the parameter list (`base + rel` in `seg.texts`).
///
/// An absent or empty word is the nullary member `method m {} {…}`, so it
/// yields no names; `None` is a list Tcl itself would reject.
fn member_param_names(
    member: &tcl_registry::definer::MemberSpec,
    args: &[String],
    seg: &SegmentedCommand,
    base: usize,
    rules: WordValueRules,
    grammar: Option<tcl_dialect::ParameterGrammar>,
    surface: Option<tcl_dialect::model::SurfaceQuery<'_>>,
) -> Option<Vec<String>> {
    let param_text = member
        .indices_for_call_in(args, surface, ArgRole::ParamList)
        .next()
        .and_then(|rel| seg.texts.get(base + rel))
        .filter(|text| !text.is_empty());
    match param_text {
        Some(text) => parse_formal_param_names(text, rules, grammar),
        None => Some(Vec::new()),
    }
}

/// One body-bearing member call ready to lift — bundles the walk context so
/// [`Lowerer::extract_one_member`] stays under the argument limit.
#[derive(Clone, Copy)]
struct MemberExtraction<'a, 'b> {
    call: &'a DefinerCall,
    seg: &'a SegmentedCommand,
    member: &'b tcl_registry::definer::MemberSpec,
    /// The effective member keyword (the inner one for a wrapper prefix).
    kw: &'a str,
    /// Index of the member's first argument word in `seg.texts` (1, or 2
    /// past a wrapper prefix).
    base: usize,
    /// The frame the member's body opens, read off its row.
    frame: MemberFrame,
}

/// The frame a member statement's body opens, read off the statement's
/// [`MemberRow`] — its effect and the side it resolves to after every wrapper
/// shift — never its keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberFrame {
    /// A method frame of this shape ([`MethodKind::from_effect`]).
    Opens(MethodKind),
    /// A body that is not a method frame: an option accessor or mutator, a
    /// declaration's configuration script, a definition-time script whose
    /// namespace no name reaches.
    NoFrame,
    /// The registry cannot read the statement's layout (a computed word where
    /// an optional one may stand).
    Unreadable,
}

impl MemberFrame {
    /// The frame `row` opens under `grammar`.
    ///
    /// A callable opens the frame [`MethodKind::from_effect`] names. A script
    /// run once at definition opens a class-level frame when the family runs
    /// member bodies in the defined entity's own namespace
    /// ([`MemberCurrentNamespace::DefinedEntity`] — snit's `typeconstructor`
    /// is the proc `${type}::Snit_typeconstructor`, run with `type` bound);
    /// under [`MemberCurrentNamespace::RuntimeReceiver`] it runs in the class
    /// object's own namespace (`TclOO`'s `initialise`, in `::oo::ObjN`), which
    /// neither the class name nor a receiver names, so it opens none.
    fn of_row(grammar: &DefinitionBodyGrammar, row: Option<&MemberRow>) -> Self {
        let Some(row) = row else {
            return Self::Unreadable;
        };
        let kind = match row.effect {
            MemberEffect::Callable { role, .. } => MethodKind::from_effect(role, row.receiver),
            MemberEffect::InitScript {
                timing: InitTiming::AtDefinition,
                ..
            } if grammar.member_current_namespace() == MemberCurrentNamespace::DefinedEntity => {
                Some(MethodKind::ClassMethod)
            }
            _ => None,
        };
        kind.map_or(Self::NoFrame, Self::Opens)
    }
}

/// Value-conservative definition-member words from the original positioned
/// token owner, under the source's retained lexical axes. Wrapper prefixes
/// are composed by the caller after this projection.
fn statement_words(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
) -> Vec<crate::registry_invocation::EffectiveInvocationWord> {
    tokens
        .words()
        .iter()
        .map(|word| {
            crate::registry_invocation::effective_invocation_word(
                word,
                config.escapes,
                WordValueRules::from_config(&config),
            )
        })
        .collect()
}

/// The instance variables one definition body declares at class level, per
/// the definer's grammar: every `all_args_var` member's names (the `TclOO`
/// `variable` slot — skipping a leading slot-operation word, which names no
/// variable) plus every `VarWrite`-role name (snit's `variable` /
/// `typevariable` / `component` / `typecomponent`, itcl's `variable` /
/// `common`), including itcl's access-modifier-wrapped spellings.
///
/// The result is the **union of every name ever declared** across the
/// block: slot removal operations (`variable -remove a`, `-set`, `-clear`)
/// are deliberately not folded here — the union is the conservative
/// direction for every consumer of [`crate::ir::MethodDef::instance_vars`]
/// (existence-fold abstention, W-family known-bound, O126 impurity), where
/// a stale extra name only widens abstention while a missed name produces a
/// false claim.  A *sibling* definition block may declare more, which is
/// why the caller accumulates a per-class union rather than using this
/// result directly.
fn declared_member_vars(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    segments: &[crate::segmenter::SegmentedCommand],
    surface: Option<tcl_dialect::model::SurfaceQuery<'_>>,
    config: tcl_lexer::LexerConfig,
) -> HashSet<String> {
    let mut out = HashSet::new();
    for seg in segments {
        if seg.is_partial || seg.texts.is_empty() {
            continue;
        }
        let head = seg.texts[0].as_str();
        let Some(member) = grammar.source_member_in(head.as_bytes(), surface) else {
            continue;
        };
        // Unwrap an access-modifier prefix (itcl `public variable x`,
        // TclOO 9's `private variable x`) one level.
        let (member, args, base): (&tcl_registry::definer::MemberSpec, &[String], usize) =
            if member.kind == tcl_registry::definer::MemberKind::Wrapper {
                match seg
                    .texts
                    .get(1)
                    .and_then(|inner| grammar.source_member_in(inner.as_bytes(), surface))
                {
                    Some(inner_member)
                        if inner_member.kind == tcl_registry::definer::MemberKind::Flat
                            && seg.texts.len() >= 3 =>
                    {
                        (inner_member, &seg.texts[2..], 2)
                    }
                    _ => continue,
                }
            } else {
                (member, &seg.texts[1..], 1)
            };
        let decoded: Vec<Option<String>> = args
            .iter()
            .enumerate()
            .map(|(relative, text)| {
                let index = base.checked_add(relative)?;
                if seg.single_token_word.get(index) != Some(&true) {
                    return None;
                }
                let token = seg.argv.get(index)?;
                crate::value_transfer::literal_token_value(text, token.kind, &config)
                    .map(std::borrow::Cow::into_owned)
            })
            .collect();
        let Some(values) = decoded.iter().cloned().collect::<Option<Vec<_>>>() else {
            // Unknown member operands widen the union of known literal names.
            out.extend(decoded.into_iter().flatten());
            continue;
        };
        if member.all_args_var {
            let names = match member.slot {
                Some(slot) => match slot.split_call(&values) {
                    Some((_, names)) => names,
                    None => continue,
                },
                None => &values,
            };
            out.extend(names.iter().cloned());
        } else {
            out.extend(
                member
                    .indices_for_call_in(&values, surface, ArgRole::VarWrite)
                    .filter_map(|relative| values.get(relative).cloned()),
            );
        }
    }
    out
}

// Public API

/// Lower Tcl source to an IR module.
///
/// Lexes, parses expressions and resolves commands under `registry`'s own
/// profile; a profile-less registry abstains from resolution. Use
/// [`lower_to_ir_with_dialect`] to honour a document's dialect.
#[must_use]
pub fn lower_to_ir(source: &str, registry: &CommandRegistry) -> Module {
    // dialect-drift-ok: the test-only entry point — every production caller
    // goes through `lower_to_ir_with_config` / `_with_dialect` with the
    // document's own config. The command world fails closed on a lexer
    // config that differs from its profile's, so the config follows the
    // registry's profile.
    // Its IR is executed by hosts that register their own commands (a VM's
    // `enable_threads`), so the catalogue does not close its command table.
    let profile = registry.profile();
    lower_with(
        Lowerer::with_config(registry, tcl_lexer::LexerConfig::for_profile(profile))
            .with_dialect(profile)
            .with_host_extensible_command_table(),
        source,
    )
}

/// Lower a single `proc` body in isolation at `proc_depth == 1` (a fresh
/// [`Lowerer`] with an empty const-map frame), returning the **offset-0** body
/// [`Script`].  Replicates the body-lowering setup `lower_proc` performs for a
/// static literal body (`proc_depth += 1`; push an empty const-map;
/// `lower_body(text, 0, namespace)`).
///
/// For a body free of cross-item context (no nested `proc` / `namespace
/// import`/`export` / command alias / const-map materialisation), this is
/// byte-identical to the body the whole-file lowering produces for that
/// procedure, normalised to offset 0 — the seam the per-procedure
/// lowering memo keys on the offset-0 body text and feeds back through
/// [`Lowerer::with_body_cache`].
#[must_use]
pub fn lower_proc_body_isolated(
    body_text: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Script {
    let mut lowerer = Lowerer::for_standalone_source(registry, config, dialect);
    lowerer.start_module();
    lowerer.source.clone_from(&body_text.to_owned());
    let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
        namespace: namespace.to_owned(),
        identity: "::body-cache".to_owned(),
    };
    lowerer.source_bindings = Some(
        SourceCommandBindings::analyse_in_frame_with_options(
            body_text,
            &frame,
            config,
            registry,
            lowerer.module.source_entry.options(),
        )
        .into(),
    );
    lowerer.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
        lowerer.lower_body(body_text, 0, namespace)
    })
}

/// Lower one runtime procedure body as a complete IR module.
///
/// Unlike the ordinary module entry point, the outer source runs at procedure
/// depth in `namespace`. This preserves procedure-only lowering semantics and
/// retains any nested procedure definitions and whole-unit command-mutation
/// facts for bytecode emission.
#[must_use]
pub fn lower_proc_body_module_for_bytecode(
    body_text: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
) -> Module {
    lower_proc_body_module_for_bytecode_with_options(
        body_text,
        namespace,
        registry,
        config,
        dialect,
        plain_command_dispatch,
        None,
    )
}

/// Runtime lowering with an explicit live interpreter entry contract.
#[must_use]
pub fn lower_proc_body_module_for_bytecode_with_options(
    body_text: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Module {
    lower_procedure_target_module_for_bytecode_with_options(
        tcl_runtime_api::ProcedureCompileTarget {
            source: body_text,
            namespace,
            parameters: &[],
        },
        registry,
        config,
        dialect,
        plain_command_dispatch,
        options,
    )
}

/// Lower a runtime procedure with its actual formal input contract.
/// C formal names are ordinary slots. Name-only Jim headers cannot prove
/// reference/default binding kind, so they retain the live entry's uncertainty.
#[must_use]
pub fn lower_procedure_target_module_for_bytecode_with_options(
    target: tcl_runtime_api::ProcedureCompileTarget<'_>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Module {
    let mut lowerer = Lowerer::with_config(registry, config)
        .with_dialect(dialect)
        .for_bytecode_backend();
    if let Some(options) = options {
        lowerer.set_source_analysis_options(options);
    }
    if plain_command_dispatch {
        lowerer = lowerer.trace_visible();
    }
    if lowerer
        .invocation_dialect
        .and_then(tcl_registry::InvocationDialect::parameter_grammar)
        == Some(tcl_dialect::ParameterGrammar::Tcl)
    {
        lowerer.incoming_formals = target.parameters.to_vec();
    }
    lowerer.lower_procedure_target(target.source, target.namespace);
    lowerer.finish_module(target.source)
}

/// Lower one runtime script as a complete bytecode IR module in its exact
/// constructed command-resolution namespace.
#[must_use]
pub fn lower_script_module_for_bytecode(
    source: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
) -> Module {
    lower_script_module_for_bytecode_with_options(
        source,
        namespace,
        registry,
        config,
        dialect,
        plain_command_dispatch,
        None,
    )
}

/// Runtime lowering with an explicit live interpreter entry contract.
#[must_use]
pub fn lower_script_module_for_bytecode_with_options(
    source: &str,
    namespace: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Module {
    let mut lowerer = Lowerer::with_config(registry, config)
        .with_dialect(dialect)
        .for_bytecode_backend();
    if let Some(options) = options {
        lowerer.set_source_analysis_options(options);
    }
    if plain_command_dispatch {
        lowerer = lowerer.trace_visible();
    }
    lowerer.lower_script_target(source, namespace);
    lowerer.finish_module(source)
}

/// Lower original native script operands through the shared byte lexical owner.
/// No Unicode projection supplies execution or compiler-admission authority.
///
/// # Errors
/// Returns the exact lexical operand capability failure.
pub fn lower_script_bytes_module_for_bytecode_with_options(
    target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Result<Module, tcl_lexer::word_parts::NativeWordError> {
    lower_native_target(
        NativeLoweringTarget {
            source: target.source,
            namespace: target.namespace,
            kind: crate::ir::TopLevelKind::Script,
            ordinary_parameters: None,
            original_request: None,
        },
        registry,
        config,
        dialect,
        plain_command_dispatch,
        options,
    )
}

/// Lower a native procedure body while retaining its exact namespace and source.
/// Formal binding is owned by the caller's original-object activation contract.
///
/// # Errors
/// Returns the exact lexical operand capability failure.
pub fn lower_procedure_bytes_module_for_bytecode_with_options(
    target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Result<Module, tcl_lexer::word_parts::NativeWordError> {
    lower_native_target(
        NativeLoweringTarget {
            source: target.source,
            namespace: target.namespace,
            kind: crate::ir::TopLevelKind::ProcedureBody,
            ordinary_parameters: None,
            original_request: None,
        },
        registry,
        config,
        dialect,
        plain_command_dispatch,
        options,
    )
}

/// Lower the immutable body request with its independently captured binding plan.
/// Unsupported formal bindings keep the legacy conservative variable treatment.
///
/// # Errors
/// Returns the original lexical operand capability failure.
pub fn lower_original_procedure_bytes_module_for_bytecode_with_options(
    request: &tcl_runtime_api::OriginalProcedureCompileRequest,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    mut options: Option<SourceAnalysisOptions<'_>>,
) -> Result<Module, tcl_lexer::word_parts::NativeWordError> {
    let ordinary_parameters = options
        .as_ref()
        .and_then(|options| options.native_entry)
        .filter(|entry| request.matches_entry(entry))
        .and_then(|_| request.ordinary_formals());
    if let Some(options) = &mut options {
        options.incoming_formals = &[];
    }
    let target = request.target();
    lower_native_target(
        NativeLoweringTarget {
            source: target.source,
            namespace: target.namespace,
            kind: crate::ir::TopLevelKind::ProcedureBody,
            ordinary_parameters,
            original_request: Some(request),
        },
        registry,
        config,
        dialect,
        plain_command_dispatch,
        options,
    )
}

#[derive(Clone, Copy)]
struct NativeLoweringTarget<'a> {
    source: &'a tcl_lexer::SourceImage,
    namespace: &'a tcl_core_types::ByteNamespacePath,
    kind: crate::ir::TopLevelKind,
    ordinary_parameters: Option<&'a [tcl_core_types::NameBytes]>,
    original_request: Option<&'a tcl_runtime_api::OriginalProcedureCompileRequest>,
}

fn lower_native_target(
    target: NativeLoweringTarget<'_>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    plain_command_dispatch: bool,
    options: Option<SourceAnalysisOptions<'_>>,
) -> Result<Module, tcl_lexer::word_parts::NativeWordError> {
    let NativeLoweringTarget {
        source,
        namespace,
        kind,
        ordinary_parameters,
        original_request,
    } = target;
    let mut lowerer = Lowerer::with_config(registry, config)
        .with_dialect(dialect)
        .for_bytecode_backend();
    if let Some(options) = options {
        lowerer.set_source_analysis_options(options);
    }
    if let (Some(request), Some(entry)) = (original_request, lowerer.native_entry.as_ref()) {
        lowerer.original_native_compiler_formals =
            crate::command_binding::OriginalNativeCompilerFormals::capture(
                request,
                std::sync::Arc::clone(entry),
                lowerer.config,
            );
    }
    if plain_command_dispatch {
        lowerer = lowerer.trace_visible();
    }
    if kind == crate::ir::TopLevelKind::ProcedureBody
        && lowerer
            .invocation_dialect
            .and_then(tcl_registry::InvocationDialect::parameter_grammar)
            == Some(tcl_dialect::ParameterGrammar::Tcl)
        && let Some(parameters) = ordinary_parameters
        && let Ok(names) = parameters
            .iter()
            .map(|name| name.try_utf8().map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()
    {
        lowerer.incoming_formals = names;
    }
    lowerer.set_source_image_channel(source);
    lowerer.module.native_namespace = Some(namespace.clone());
    if let (Ok(text), Some(namespace_key)) = (
        source.try_text(),
        lowerer.module.advisory_top_level_namespace(),
    ) {
        if crate::segmenter::segment_commands_image_with_offset_and_config(
            source,
            0,
            lowerer.config,
        )
        .is_none()
        {
            return Err(tcl_lexer::word_parts::NativeWordError::LexicalStreamUnavailable);
        }
        let namespace_key = tcl_syntax::naming::unroot_rooted_key(&namespace_key)
            .expect("checked analytical namespace is rooted");
        match kind {
            crate::ir::TopLevelKind::Script => {
                lowerer.lower_script_target(text, namespace_key);
            }
            crate::ir::TopLevelKind::ProcedureBody => {
                lowerer.lower_procedure_target(text, namespace_key);
            }
        }
        let mut module = lowerer.finish_module(text);
        module.source = source.clone();
        module.native_namespace = Some(namespace.clone());
        return Ok(module);
    }
    lower_native_byte_calls(lowerer, source, namespace, registry, options, kind)
}

fn lower_native_byte_calls(
    mut lowerer: Lowerer<'_>,
    source: &tcl_lexer::SourceImage,
    namespace: &tcl_core_types::ByteNamespacePath,
    registry: &CommandRegistry,
    options: Option<SourceAnalysisOptions<'_>>,
    kind: crate::ir::TopLevelKind,
) -> Result<Module, tcl_lexer::word_parts::NativeWordError> {
    lowerer.start_module();
    let config = lowerer.config;
    let tokens = tcl_lexer::Lexer::with_source_image(source, config)
        .tokenise_all()
        .map_err(|_| tcl_lexer::word_parts::NativeWordError::LexicalStreamUnavailable)?;
    let groups = tcl_lexer::group_commands_bytes(&tokens, source.bytes(), config);
    let mut module = lowerer.module;
    module.source = source.clone();
    module.native_namespace = Some(namespace.clone());
    module.top_level_kind = kind;
    module.top_level_namespace = module.advisory_top_level_namespace().unwrap_or_default();
    for group in groups {
        let words = group
            .words
            .iter()
            .map(|word| {
                let word = tcl_lexer::word_parts::NativeWord::from_group(
                    source.clone(),
                    config,
                    &tokens,
                    word,
                )?;
                Ok(word)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if let (Some(first), Some(last)) = (words.first(), words.last()) {
            module.top_level.statements.push(Statement::NativeCall {
                span: tcl_lexer::Span::new(first.span().start(), last.span().end()),
                words,
            });
        }
    }
    let origin = std::sync::Arc::new(crate::command_binding::SourceOriginId::authored_image(
        source.clone(),
    ));
    let executed =
        crate::command_binding::ExecutedScriptSource::contiguous_image(origin, source.clone(), 0)
            .expect("the complete original image is contiguous");
    module.top_level.executed_source = Some(std::sync::Arc::new(executed.clone()));
    let compiler_dialect = options.and_then(|options| options.native_compiler_dialect());
    let native_inline_disabled = compiler_dialect
        .and_then(tcl_registry::InvocationDialect::native_string_protocol)
        .is_some_and(|protocol| protocol.tcl_version().is_some())
        && options
            .and_then(|options| options.native_entry)
            .is_some_and(|entry| entry.inline_compilation_disabled);
    let original_byte_compilation = options
        .and_then(|options| options.native_entry)
        .is_some_and(|entry| {
            let mut context = module.source_entry.native_compilation;
            if module.top_level_kind == crate::ir::TopLevelKind::ProcedureBody {
                context.mode =
                    tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject;
                context.frame =
                    tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
            }
            crate::native_byte_compilation::closed_original_byte_compilation(
                &module, entry, registry, context,
            )
        });
    if !native_inline_disabled
        && !original_byte_compilation
        && compiler_dialect
            .is_none_or(|dialect| dialect.family() != Some(tcl_dialect::model::Family::Jim))
    {
        module.top_level.native_compilation_admission = Some(std::sync::Arc::new(
            crate::native_compilation_admission::NativeCompilationAdmission {
                source: Some(executed),
                failure: None,
                provider_required: true,
            },
        ));
    }
    // Native calls retain unconstrained callbacks and variable effects.
    module.has_dynamic_trace = true;
    module.has_dynamic_variable_trace = true;
    module.retain_original_byte_source_namespace(namespace, registry);
    Ok(module)
}

/// Legacy text-shape predicate retained for callers that inspect source shape.
///
/// This does not prove cache eligibility. The lowering ingress uses
/// [`SourceCommandBindings::can_reuse_isolated_body`] and restores exact
/// positioned proof carriers through
/// [`SourceCommandBindings::restore_script_bindings`] after a cache hit.
/// A command's spelling cannot establish the implementation it executes.
#[must_use]
pub fn body_cache_eligible(body: &str) -> bool {
    // Command keywords whose body-level use carries a cross-item effect the
    // isolated lowering drops. `proc` covers a nested procedure definition.
    // Matched as a word followed by Tcl inter-word whitespace (space, tab, CR,
    // newline, or a `\`-continuation), so a tab-separated `interp\talias` trips
    // it too.
    const WORD_DISQUALIFIERS: &[&str] = &[
        "namespace",
        "interp",
        "rename",
        "method",
        "when",
        "apply",
        "alias",
        "proc",
    ];
    // Substring markers (the token itself is the marker — no trailing arg).
    const SUBSTR_DISQUALIFIERS: &[&str] = &["oo::", "::oo", "itcl"];
    !SUBSTR_DISQUALIFIERS.iter().any(|kw| body.contains(kw))
        && !WORD_DISQUALIFIERS
            .iter()
            .any(|kw| contains_word_followed_by_ws(body, kw))
}

/// Legacy lexical indication that source contains alias-related words.
///
/// This is a presentation/compatibility predicate, not command-table evidence
/// or a cache gate. Use the shared positioned binding owner for those questions.
#[must_use]
pub fn source_may_alias_commands(source: &str) -> bool {
    contains_word_followed_by_ws(source, "interp") || contains_word_followed_by_ws(source, "rename")
}

/// True when `word` occurs in `source` followed by Tcl inter-word whitespace —
/// i.e. used as a command with an argument. Ignores the character *before* the
/// word (a preceding non-boundary only over-disqualifies, which is safe).
fn contains_word_followed_by_ws(source: &str, word: &str) -> bool {
    source.match_indices(word).any(|(i, _)| {
        source
            .as_bytes()
            .get(i + word.len())
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'\\'))
    })
}

#[cfg(test)]
mod body_cache_eligible_tests;

/// Like [`lower_to_ir_with_dialect`] but with a memoised per-procedure body-lowering
/// callback: a top-level `proc`'s static body is lowered
/// through `body_cache` `(offset-0 body text, namespace) -> offset-0 Script` and
/// rebased, so an unchanged proc's body IR is reused across edits.  The caller must
/// only install a cache for **context-free** files (see [`Lowerer::body_cache`]);
/// byte-identity is guarded by the corpus differential gates.
///
/// The cached bodies must be produced by [`lower_proc_body_isolated`] under the
/// **same** `dialect`, since the dialect selects the expression grammar the
/// body's conditions are parsed with.
#[must_use]
pub fn lower_to_ir_with_body_cache(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    body_cache: &dyn Fn(&str, &str) -> Script,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config)
            .with_dialect(dialect)
            .with_body_cache(body_cache),
        source,
    )
}

/// Like [`lower_to_ir`] but with an explicit dialect
/// [`tcl_lexer::LexerConfig`], threaded into every body re-segmentation
/// so `{*}` expansion (off for Tcl 8.4 / iRules) and the iRules `}{`
/// ghost SEP are honoured.
///
/// Expressions are still parsed in the plain-Tcl grammar — a caller that
/// knows the dialect *name* should use [`lower_to_ir_with_dialect`], which
/// also gives `if` / `while` / `for` / `expr` the dialect's operator set.
#[must_use]
pub fn lower_to_ir_with_config(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Module {
    lower_with(Lowerer::with_config(registry, config), source)
}

/// Like [`lower_to_ir_with_config`] but also naming the document's analysis
/// `dialect` (`""` for plain Tcl), so every `if` / `while` / `for` condition
/// and every inlined `expr` is parsed with that dialect's operator set.
///
/// This is what makes an iRules word-operator condition — `if {$x contains
/// "cd"}` — reach the IR as a real
/// [`BinOp::Contains`](crate::expr_ast::BinOp::Contains) comparison instead of
/// an opaque [`ExprNode::Raw`](crate::expr_ast::ExprNode::Raw), which is the
/// only shape the constant folder (and the I230 constant-condition
/// diagnostic behind it) can evaluate.
#[must_use]
pub fn lower_to_ir_with_dialect(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config).with_dialect(dialect),
        source,
    )
}

/// Lower `source` with an already-configured [`Lowerer`].
///
/// The seam a host uses when it has more to install than the fixed-argument
/// entry points above accept — the document's own
/// [`with_declared_commands`](Lowerer::with_declared_commands) surface, above
/// all. Every other `lower_to_ir_*` function is this one with a `Lowerer`
/// built for it.
#[must_use]
pub fn lower_to_ir_with(lowerer: Lowerer<'_>, source: &str) -> Module {
    lower_with(lowerer, source)
}

/// Like [`lower_to_ir`] but for the bytecode/VM compile path: constructs the
/// backend can't compile correctly (`try`, and a `foreach`/`lmap` directly
/// nesting another) are lowered to runtime-command barriers (see
/// [`Lowerer::for_bytecode`]). Analysis callers keep the structured IR via
/// [`lower_to_ir`].
#[must_use]
pub fn lower_to_ir_for_bytecode(source: &str, registry: &CommandRegistry) -> Module {
    lower_to_ir_for_bytecode_with_dialect(source, registry, tcl_lexer::LexerConfig::default(), None)
}

/// Like [`lower_to_ir_for_bytecode`] but also naming the document's `dialect`
/// (`""` for plain Tcl) — the bytecode-path counterpart of
/// [`lower_to_ir_with_dialect`], so a dialect expression such as an iRules
/// word-operator condition compiles to its dedicated opcode rather than the
/// generic runtime-`expr` fallback.
#[must_use]
pub fn lower_to_ir_for_bytecode_with_dialect(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config)
            .with_dialect(dialect)
            .for_bytecode_backend(),
        source,
    )
}

/// Like [`lower_to_ir_for_bytecode`] but with every inline/structured
/// lowering hook suppressed (see [`Lowerer::trace_visible`]) — every command
/// in `source` compiles to a plain runtime dispatch, so an execution trace
/// observes it. For recompiling a proc/script body once a step-capable
/// execution trace targets it; see
/// [`tcl_runtime_api::CompileService::compile_traced`].
#[must_use]
pub fn lower_to_ir_traced(source: &str, registry: &CommandRegistry) -> Module {
    lower_to_ir_traced_with_config(source, registry, tcl_lexer::LexerConfig::default())
}

/// Like [`lower_to_ir_traced`] but with an explicit dialect
/// [`tcl_lexer::LexerConfig`], so a version-pinned host's traced recompiles
/// parse under the same grammar as its ordinary compiles.
#[must_use]
pub fn lower_to_ir_traced_with_config(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Module {
    lower_to_ir_traced_with_dialect(source, registry, config, None)
}

/// Like [`lower_to_ir_traced_with_config`] but also records the exact dialect
/// selected by the host.  Runtime compile services for a named profile must
/// use this entry point so the resulting bytecode artifact retains the
/// profile identity that its lexer, registry, and expression grammar used.
#[must_use]
pub fn lower_to_ir_traced_with_dialect(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config)
            .with_dialect(dialect)
            .for_bytecode_backend()
            .trace_visible(),
        source,
    )
}

/// Return the first hard parse-error message in `source`, in source order, or
/// `None` if `source` parses cleanly.
///
/// A VM compile front-end (`CompileService::compile`) calls this so that a
/// malformed script becomes a catchable runtime error carrying the **exact**
/// C Tcl message (e.g. `extra characters after close-quote`), matching C Tcl's
/// deferral of a compile-time parse error to a runtime instruction. The lexer
/// otherwise recovers from these for the benefit of editor diagnostics, so the
/// IR / bytecode never sees them.
///
/// Lexes with the default (Tcl-8.5+) dialect config — the same one
/// [`lower_to_ir_for_bytecode`] uses.
#[must_use]
pub fn first_fatal_parse_error(source: &str) -> Option<String> {
    first_fatal_parse_error_with_config(source, tcl_lexer::LexerConfig::default())
}

/// Like [`first_fatal_parse_error`] but lexing under an explicit dialect
/// [`tcl_lexer::LexerConfig`], so a version-pinned host rejects exactly what
/// its emulated release rejects — under the Tcl 8.4 grammar `{*}{a b}` is a
/// hard `extra characters after close-brace`, which the default (8.5+) config
/// lexes as an ordinary expansion.
#[must_use]
pub fn first_fatal_parse_error_with_config(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    first_fatal_parse_cut(source, config).map(|cut| cut.message.to_owned())
}

/// The full [`tcl_lexer::ParseCut`] for `source` — the message *and* the
/// index of the top-level command it was found under.
///
/// The command index is what separates this from
/// [`first_fatal_parse_error_with_config`]: C runs every command before the
/// cut and only then raises, so a front-end that wants C's behaviour has to
/// know where the clean prefix ends. The message alone cannot say.
///
/// This is a thin re-export of [`tcl_lexer::first_parse_cut`] — the shared
/// owner — and exists so `tcl-compiler`'s consumers have one import for the
/// whole parse-error question rather than reaching past it into the lexer.
/// The lexer's flat **warning stream** cannot answer it: filtering that
/// stream by message and taking the lowest-offset hit disagrees with C
/// whenever the failure is nested — `list [sfx one] [list "oops]` warns
/// `missing close-bracket` where C reports `missing "` — and a welded
/// close-brace (`set y {a}b`) does not appear in the warning stream at all.
#[must_use]
pub fn first_fatal_parse_cut(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<tcl_lexer::ParseCut> {
    tcl_lexer::first_parse_cut(source, config)
}

/// Segment `source` using `config`, retaining only complete commands before a
/// fatal command and the original structured cut, including its kind and term.
#[doc(hidden)]
pub fn command_at_time_script_with_config(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Result<crate::segmenter::SegmentedScript, tcl_lexer::ParseCutUnavailable> {
    command_at_time_script_image(&tcl_lexer::SourceImage::document(source), config)
}

/// Complete command prefix of an original source image. Unavailable lexical
/// ownership remains distinct from an authentic malformed command tail.
#[doc(hidden)]
pub fn command_at_time_script_image(
    source: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
) -> Result<crate::segmenter::SegmentedScript, tcl_lexer::ParseCutUnavailable> {
    crate::segmenter::segment_script_image_with_offset_and_config(source, 0, config)
}

/// Drive a configured [`Lowerer`] to a finished [`Module`] (the shared tail of
/// the `lower_to_ir*` entry points).
fn lower_with(mut lowerer: Lowerer<'_>, source: &str) -> Module {
    lowerer.lower(source);
    lowerer.finish_module(source)
}

impl Lowerer<'_> {
    /// Complete either module entry path after its top-level script has been
    /// lowered.  Namespace tables, OO extraction, source/profile stamps, and
    /// trace and callback-write facts deliberately have this one owner so
    /// compiling a runtime procedure target cannot drift from ordinary module
    /// lowering.
    #[doc(hidden)]
    #[must_use]
    pub fn finish_module(self, source: &str) -> Module {
        self.finish_module_with(source, true)
    }

    /// The module a command-mutation scan reads: everything
    /// [`Self::finish_module`] retains except trace and deferred-write
    /// observer facts, which no command-binding analysis consults. Each of
    /// those scans re-walks every retained body (one prepares a whole-source
    /// callback tape), so the analyser's constant-folding trust oracle skips
    /// them.
    pub(crate) fn finish_module_for_command_trust(self, source: &str) -> Module {
        self.finish_module_with(source, false)
    }

    fn finish_module_with(mut self, source: &str, observer_facts: bool) -> Module {
        source.clone_into(&mut self.source);
        self.module.source =
            tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_channel);
        // Surface namespace import / export directives onto the module for
        // downstream consumers (codegen import resolution and warning passes).
        self.module.namespace_imports = std::mem::take(&mut self.namespace_imports);
        self.module.namespace_exports = std::mem::take(&mut self.namespace_exports);
        // Extract TclOO method bodies from the fully-assembled module
        // (cache-independent — see `extract_oo_methods_pass`).
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::WholeModule {
            self.retain_installed_procedure_body_units();
            self.retain_hosted_procedure_body_units();
            self.extract_oo_methods_pass();
        }
        let registry = self.registry;
        let standalone_metadata = self.metadata_origin == LoweringMetadataOrigin::Standalone
            && self.source_entry_origin == SourceEntryOrigin::Authoring;
        let declared_frame_effects = crate::declared_frame_effects::collect_declared_frame_effects(
            registry,
            self.source_declared_commands
                .as_ref()
                .or(self.declared_commands),
        );
        let mut module = self.module;
        module.declared_frame_effects = declared_frame_effects;
        module.retained_source_bindings =
            self.module_source_bindings.as_deref().and_then(|bindings| {
                crate::command_binding::RetainedSourceModuleBindings::retain(
                    bindings,
                    &module,
                    registry,
                    self.supplied_source.as_ref(),
                )
            });
        if observer_facts {
            populate_trace_facts(&mut module, registry, standalone_metadata);
            module.deferred_writes = crate::deferred_writes::scan_module(&module, registry);
        }
        module
    }
}

/// Post-lower scan for `trace add ...` calls that populates
/// `Module::traced_commands` / `has_dynamic_trace` (execution traces)
/// and `Module::traced_variables` / `has_dynamic_variable_trace`
/// (variable traces).
///
/// Runs over the top-level script + every procedure body + every `TclOO`
/// method body (`module.methods`, already populated by
/// `extract_oo_methods_pass` before this runs). Original command operands
/// contribute source-renderable keys from their retained lookup geometry;
/// missing or opaque geometry sets `has_dynamic_trace` so GVN treats every
/// call as potentially traced. Authored compatibility has its separate written
/// grammar; a descriptor label cannot supply an original trace operand.
/// Literal variable names targeted by any `Traits::
/// ESTABLISHES_VARIABLE_TRACE` subcommand — the registry's
/// `ArgRole::VarWrite` resolution for `trace add|remove|variable|vdelete`,
/// covering both the modern and deprecated legacy spellings — land in
/// `traced_variables`; non-literal targets flip
/// `has_dynamic_variable_trace`.
fn populate_trace_facts(module: &mut Module, registry: &CommandRegistry, standalone: bool) {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let actual = crate::registry_invocation::retained_module_metadata_context(registry, module);
    let context = actual.as_deref().map(Into::into).or_else(|| {
        (standalone && module.source_metadata_input.is_none())
            .then(|| {
                registry
                    .profile()
                    .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                    .map(crate::registry_invocation::InvocationMetadataContext::from)
            })
            .flatten()
    });
    // Borrow every retained body instead of cloning it: a derived `Clone` of
    // cap-deep structured IR overflows a 2 MiB thread. The facts collect
    // separately and are applied once the walk has finished.
    let mut facts = TraceFacts::default();
    walk_for_trace(&module.top_level, &mut facts, registry, context);
    // Every statically-known frame, not just named procedures: a `trace`
    // call inside a `namespace eval` / `apply` body (`Module::body_units`)
    // or a `TclOO` method (`Module::methods`) is just as live as one inside
    // a `proc` — omitting either class here would silently under-populate
    // `traced_variables` the moment an optimiser pass starts reaching into
    // those bodies (`CompilationUnit::all_body_function_units`).
    for body in module
        .procedures
        .values()
        .map(|p| &p.body)
        .chain(module.body_lowerings().map(|(_, p)| &p.body))
        .chain(module.methods.values().map(|m| &m.body))
    {
        walk_for_trace(body, &mut facts, registry, context);
    }
    module.traced_variables.extend(facts.variables);
    module.traced_commands.extend(facts.commands);
    module.has_dynamic_variable_trace |= facts.dynamic_variable;
    module.has_dynamic_trace |= facts.dynamic;
}

/// Trace facts one retained-body walk collects for its [`Module`].
#[derive(Default)]
struct TraceFacts {
    variables: std::collections::BTreeSet<String>,
    commands: std::collections::BTreeSet<String>,
    dynamic_variable: bool,
    dynamic: bool,
}

/// Possible variable registrations are observer hazards even when receiver
/// lookup cannot select a unique normal handler. Execution traces retain their
/// stricter dispatch proof in the separate path below.
fn record_possible_variable_trace(
    transition: &tcl_registry::TraceTransition,
    module: &mut TraceFacts,
) {
    use tcl_registry::{TraceTarget, TraceTransition};
    let (TraceTransition::Add { target, .. } | TraceTransition::Remove { target, .. }) = transition;
    let TraceTarget::Variable(subject) = target else {
        return;
    };
    if let Some(target) = subject.literal() {
        let name = crate::variable_observation_names::module_observation_key(target);
        module.variables.insert(name.to_owned());
    } else {
        module.dynamic_variable = true;
    }
}

/// Collect typed trace transitions through the shared retained-body visitor.
fn walk_for_trace(
    script: &Script,
    module: &mut TraceFacts,
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) {
    crate::ir::for_each_statement(script, &mut |stmt| {
        if !matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. }) {
            return;
        }
        let Some(context) = context else {
            // Missing source metadata cannot close either observer-hazard inventory.
            module.dynamic_variable = true;
            module.dynamic = true;
            return;
        };
        if let Some(possible) = stmt.tokens().and_then(|tokens| {
            crate::registry_invocation::possible_variable_trace_transitions_with_metadata_context(
                registry,
                Some(context),
                tokens,
            )
        }) {
            for transition in possible.transitions() {
                record_possible_variable_trace(transition, module);
            }
            if possible.metadata_unavailable() {
                module.dynamic_variable = true;
                module.dynamic = true;
            }
        }
        let Some(invocation) =
            crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
                registry,
                Some(context),
                stmt,
            )
        else {
            return;
        };
        let Some(transitions) = invocation.facts.state_transitions.declared() else {
            return;
        };
        for fact in transitions.facts() {
            use tcl_registry::{StateTransition, TraceTarget, TraceTransition};
            let StateTransition::Trace(transition) = &fact.transition else {
                continue;
            };
            let (target, add) = match transition {
                TraceTransition::Add { target, .. } => (target, true),
                TraceTransition::Remove { target, .. } => (target, false),
            };
            match target {
                TraceTarget::Execution(subject) if add => {
                    let keys = stmt.tokens().and_then(|tokens| {
                        crate::registry_invocation::original_execution_trace_keys(
                            registry, context, tokens, subject,
                        )
                    });
                    if let Some(keys) = keys {
                        module.commands.extend(keys);
                    } else {
                        module.dynamic = true;
                    }
                }
                TraceTarget::Variable(_) | TraceTarget::Command(_) | TraceTarget::Execution(_) => {}
            }
        }
    });
}

fn untyped_call(
    seg: &SegmentedCommand,
    canonical_command: Option<String>,
    tokens: CommandTokens,
) -> Statement {
    Statement::Call {
        span: seg.span,
        command: seg.name().into(),
        canonical_command,
        args: seg.args().to_vec(),
        defs: vec![],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: Some(tokens),
        foreach_groups: None,
    }
}

fn body_barrier(
    seg: &SegmentedCommand,
    canonical_command: Option<String>,
    tokens: CommandTokens,
) -> Statement {
    Statement::Barrier {
        span: seg.span,
        reason: "unsupported body command".into(),
        command: seg.name().into(),
        canonical_command,
        args: seg.args().to_vec(),
        tokens: Some(tokens),
    }
}

fn literal_words(
    words: &[crate::registry_invocation::EffectiveInvocationWord],
) -> Option<Vec<String>> {
    words
        .iter()
        .map(|word| match word {
            crate::registry_invocation::EffectiveInvocationWord::Literal(value) => {
                Some(value.clone())
            }
            _ => None,
        })
        .collect()
}

struct RoleIndices {
    executable: Vec<usize>,
    var_writes: Vec<usize>,
    var_reads: Vec<usize>,
    reads_before_write: bool,
}

impl RoleIndices {
    fn of(facts: Option<&tcl_registry::resolved_invocation::InvocationFacts>) -> Self {
        let Some(facts) = facts else {
            return Self {
                executable: Vec::new(),
                var_writes: Vec::new(),
                var_reads: Vec::new(),
                reads_before_write: false,
            };
        };
        let indices = |role| -> Vec<usize> {
            facts
                .arg_roles
                .iter()
                .filter_map(|&(index, found)| {
                    (found == role).then_some(facts.argument_offset + usize::from(index))
                })
                .collect()
        };
        let mut executable: Vec<usize> = [
            ArgRole::Body,
            ArgRole::LambdaLiteral,
            ArgRole::CommandPrefix,
        ]
        .into_iter()
        .flat_map(indices)
        .collect();
        executable.sort_unstable();
        executable.dedup();
        // Incomplete dynamic option/argument resolution cannot establish a
        // strong definition. The shared place owner retains the uncertainty.
        let var_writes = if !facts.arg_roles_complete
            || facts.frame_effect.is_some_and(|effect| {
                effect.layout == tcl_registry::frame_effect::FrameArgLayout::AliasPairs
            }) {
            Vec::new()
        } else {
            indices(ArgRole::VarWrite)
        };
        Self {
            executable,
            var_writes,
            var_reads: indices(ArgRole::VarRead),
            reads_before_write: facts
                .traits
                .contains(tcl_registry::Traits::READS_BEFORE_WRITE),
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_script_purpose_tests;

/// [`Lowerer::select_command_lowering`]'s answer: a finished selection, or the
/// default lowering, which the caller performs.
enum CommandLoweringStep {
    Selected(CommandLoweringSelection),
    Default,
}

impl CommandLoweringStep {
    #[allow(clippy::unnecessary_box_returns)]
    fn lowered(statement: Box<Statement>) -> Self {
        Self::Selected(CommandLoweringSelection::Lowered(statement))
    }
}

/// What [`Lowerer::lower_command`] does after its out-of-line selection.
enum CommandLoweringSelection {
    /// A finished statement, built outside the recursive descent frame.
    Lowered(Box<Statement>),
    /// The resolved label of a possible structured form, whose hook may
    /// recurse into nested bodies.
    Structured(String),
}
