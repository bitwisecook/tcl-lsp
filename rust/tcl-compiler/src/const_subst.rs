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

//! Shared constant command-substitution fold engine.
//!
//! Answers one question for three different consumers: *does this
//! `[cmd args…]` command substitution evaluate to a compile-time constant?*
//! Everything command-specific comes from registry data — the registry-owned
//! direct route the call resolves to, run over its literal words
//! ([`tcl_registry::value_transfer::evaluate_literal`]), which is the
//! implementation the lattice runs for the same call; for a command that
//! declares no route, the [`tcl_registry::CommandSpec::const_fold`] /
//! [`tcl_registry::SubCommand::const_fold`] callbacks (pure functions of the
//! constant argument words); and the
//! [`tcl_registry::CommandSpec::oo_context_facts`] table (a keyword whose
//! value the enclosing `TclOO` method frame fixes, like `[self class]`).
//! No command name is matched here.
//!
//! The three consumers, each with its own view of "what is a constant
//! variable" and "which commands still have their original semantics":
//!
//! * the **optimiser propagation pass** (`optimiser/propagation.rs`, the
//!   O129 rewrite) — original retained execution targets and effective argv
//!   through [`ConstSubstCtx::fold_retained_call`];
//! * **SCCP lattice evaluation itself** (`crate::sccp`) —
//!   constants resolved per SSA use version, so a folded value re-enters the
//!   lattice and multi-statement chains
//!   (`set base [self class]; set ns [namespace qualifiers $base]`) fold to
//!   fixpoint under SCCP's ordinary monotone iteration;
//! * the **analyser** (`analyser/handlers.rs`) — constants
//!   from the scope-chain *dominating* constant-string lattice, trust from a
//!   lazily-built whole-module mutation scan, so `set ns [namespace
//!   qualifiers ::tc::X]; ${ns}::setdef …` resolves for navigation.
//!
//! Soundness stance: **abstain-toward-no-fold**. Every gate that cannot be
//! answered (a renamed / aliased / shadowed head, a non-literal word, an
//! expansion, an unresolvable variable, a class-side method frame) declines
//! the fold; a wrong constant is a miscompile, a missed one only a lost
//! optimisation. Literal escape decoding is delegated to `tcl-lexer` under
//! the selected profile rather than reimplemented here.

use tcl_dialect::TclVersion;
use tcl_registry::value_transfer::{
    Budget, EvalAnswer, EvalRoute, EvaluatorOwner, ExactValueOrUnavailable, LiteralInputs,
    RepresentationEvidence, ResolvedSemantics, resolve_semantics,
};
use tcl_registry::{CommandRegistry, CommandSpec, SubCommand, TclType};
use tcl_runtime_api::CommandBindingIdentity;

use crate::naming::normalise_var_name;

/// Nesting bound for recursive folds of nested command substitutions
/// (`[llength [list a [list b c]]]`). Each level consumes one bracketed
/// interior of strictly smaller text, so recursion is structurally bounded
/// anyway; the explicit cap is the belt-and-braces termination bound for
/// adversarial inputs.
const MAX_CONST_SUBST_DEPTH: u32 = 16;

/// Everything the fold engine needs to answer a fold soundly, supplied by
/// the consumer.
pub struct ConstSubstCtx<'a> {
    /// Command / subcommand specs — the fold callbacks live here.
    pub registry: &'a CommandRegistry,
    /// Rooted constructed namespace in which every command head in the
    /// substitution resolves.
    pub resolution_namespace: &'a str,
    /// Exact original lookup context for executable fold dependencies.
    /// Source-only assistance supplies `None` and grants no native identity.
    pub namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
    /// Resolved Tcl release forwarded to versioned folds
    /// (`const_fold_versioned`); `None` when the consumer has no release fact.
    pub version: Option<TclVersion>,
    /// The fully-qualified class defining the enclosing `TclOO` method
    /// implementation, when the consumer *proved* the frame (instance-side
    /// method of a statically-named, never-renamed class). Enables the
    /// registry [`tcl_registry::OoContextFact`] folds (`[self class]`).
    /// `None` abstains from every frame-fact fold.
    pub defining_class: Option<&'a str>,
    /// Trust check: `true` when `name` still denotes its original command
    /// at every point this fold's result could be observed — i.e. the name
    /// was never `rename`d, `interp alias`ed, or shadowed by a user proc
    /// anywhere in the module. Consumers back this with a whole-module,
    /// flow-insensitive scan ([`crate::command_binding::ModuleCommandMutations`]);
    /// a flow-sensitive "no rename seen so far" answer is NOT sound here
    /// (a rename buried in a proc body can fire before a later call runs).
    pub trusts: &'a dyn Fn(&str) -> bool,
    /// Constant lookup for a `$var` word inside the substitution: the
    /// variable's proven compile-time value, or `None` to abstain. The
    /// engine queries both the raw written name and its normalised form.
    pub lookup_var: &'a dyn Fn(&str) -> Option<String>,
}

/// A registry-resolved constant command substitution.
///
/// Besides the raw folded value, code generators need the exact live command
/// identities whose semantics the fold consumed. Nested folds contribute
/// their identities too: folding `[llength [list a b]]` depends on both
/// commands, not only the outer one. The return type is the registry's answer
/// for the outer invocation and lets bytecode consumers preserve typed literal
/// setup such as `VERIFY_DICT` without recognising a command name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConstSubst {
    /// Raw result value, before any caller-specific word quoting.
    pub value: String,
    /// Exact source spelling → registry implementation dependencies.
    pub command_bindings: Vec<CommandBindingIdentity>,
    /// The spec-pack facts the fold rests on: one rung-1 claim for each
    /// folding spec a pack supplied ([`crate::site_claims::pack_facts_claim`]),
    /// nested folds included.
    pub site_claims: Vec<tcl_runtime_api::SiteClaim>,
    /// Registry-declared return type of the outer invocation.
    pub return_type: Option<TclType>,
}

impl ConstSubstCtx<'_> {
    /// Fold one original substitution using its retained execution identity and
    /// effective argv. Every nested result comes from the same complete source
    /// inventory; textual command trust is not used by this entry point.
    pub(crate) fn fold_retained_call_with_metadata_context(
        &self,
        call: &crate::word_subst::LiftedCall,
        calls: &[crate::word_subst::LiftedCall],
        context: crate::registry_invocation::InvocationMetadataContext<'_>,
    ) -> Option<String> {
        self.fold_retained_at_depth(call, calls, 0, context)
    }

    fn fold_retained_at_depth(
        &self,
        call: &crate::word_subst::LiftedCall,
        calls: &[crate::word_subst::LiftedCall],
        depth: u32,
        context: crate::registry_invocation::InvocationMetadataContext<'_>,
    ) -> Option<String> {
        if depth > MAX_CONST_SUBST_DEPTH {
            return None;
        }
        let tokens = call.tokens.as_ref()?;
        let binding = tokens.source_binding.as_ref()?;
        binding
            .proved_execution_target()
            .filter(|target| target.registry_backed)?;
        let invocation =
            crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                self.registry,
                Some(context),
                tokens,
            )?;
        let dialect = invocation.dialect?;
        if invocation.facts.effects.requires_world_barrier() {
            return None;
        }
        if invocation.facts.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Expr,
            )
        {
            return None;
        }
        let args = self.retained_arguments_at_depth(call, calls, depth, context)?;
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let spec = self.registry.get_for_surface(
            &invocation.facts.canonical_command,
            Some(context.context().authoring_query()),
        )?;
        if let Some(class) = self.defining_class
            && let Some(value) = oo_context_fact_fold(spec, &args, class)
        {
            return Some(value);
        }
        let arguments = arg_refs.get(invocation.facts.argument_offset..)?;
        if spec.subcommands.is_empty() {
            spec.run_const_fold_in(arguments, dialect)
        } else {
            spec.subcommand(invocation.facts.subcommand.canonical_name()?)?
                .run_const_fold_in(arguments, dialect)
        }
    }

    /// Original argument values whose evaluation has independently closed
    /// effects. Known child contents do not erase its actual invocation.
    pub(crate) fn retained_arguments_with_metadata_context(
        &self,
        call: &crate::word_subst::LiftedCall,
        calls: &[crate::word_subst::LiftedCall],
        context: crate::registry_invocation::InvocationMetadataContext<'_>,
    ) -> Option<Vec<String>> {
        self.retained_arguments_at_depth(call, calls, 0, context)
    }

    /// Closed values from genuine whole original argv, without a fabricated
    /// command-substitution occurrence or a projected word range.
    pub(crate) fn retained_token_arguments_with_metadata_context(
        &self,
        tokens: &crate::ir::CommandTokens,
        calls: &[crate::word_subst::LiftedCall],
        context: crate::registry_invocation::InvocationMetadataContext<'_>,
    ) -> Option<Vec<String>> {
        self.retained_token_arguments_at_depth(tokens, calls, 0, context)
    }

    fn retained_arguments_at_depth(
        &self,
        call: &crate::word_subst::LiftedCall,
        calls: &[crate::word_subst::LiftedCall],
        depth: u32,
        context: crate::registry_invocation::InvocationMetadataContext<'_>,
    ) -> Option<Vec<String>> {
        self.retained_token_arguments_at_depth(call.tokens.as_ref()?, calls, depth, context)
    }

    fn retained_token_arguments_at_depth(
        &self,
        tokens: &crate::ir::CommandTokens,
        calls: &[crate::word_subst::LiftedCall],
        depth: u32,
        context: crate::registry_invocation::InvocationMetadataContext<'_>,
    ) -> Option<Vec<String>> {
        if !context.matches_registry(self.registry) {
            return None;
        }
        let binding = tokens.source_binding.as_ref()?;
        let target = binding.proved_execution_target().or_else(|| {
            binding
                .scoped_procedure_evaluation(tokens)
                .map(|receipt| receipt.target)
        })?;
        let dialect = binding.variable_context.invocation_dialect?;
        let effective = crate::registry_invocation::effective_words_for_target(tokens, target)?;
        effective
            .words
            .iter()
            .skip(1)
            .enumerate()
            .map(|(index, word)| {
                if matches!(
                    effective.origins.get(index + 1),
                    Some(crate::registry_invocation::InvocationWordOrigin::BindingPrefix(_))
                ) && !binding.object_callback_effects_closed()
                {
                    return None;
                }
                let value = || {
                    if let Some(crate::registry_invocation::InvocationWordOrigin::Written(
                        written,
                    )) = effective.origins.get(index + 1)
                        && let Some(value) = written
                            .checked_sub(1)
                            .and_then(|argument| binding.evaluated_written_argument_value(argument))
                        {
                            return Some(value.to_owned());
                        }
                    crate::registry_invocation::effective_invocation_word(
                        word,
                        dialect.lexer_grammar.escapes,
                        dialect.word_values,
                    )
                    .literal_bytes()
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    .map(str::to_owned)
                };
                match word {
                    crate::ir::WordExpr::Literal { .. }
                    | crate::ir::WordExpr::BracedLiteral { .. } => value(),
                    crate::ir::WordExpr::Template { parts, .. }
                        if parts
                            .iter()
                            .all(|part| matches!(part, crate::ir::WordPart::Text { .. })) =>
                    {
                        value()
                    }
                    crate::ir::WordExpr::Variable { spelling, source } => {
                        let access = tokens.variable_access_at(source, spelling)?;
                        if access.context_alternatives().is_empty()
                            || access.context_residual()
                                != crate::command_binding::SourceVariableReadResidual::Closed
                            || !access.context_alternatives().iter().all(|context| {
                                let place = access.place_in_context(context, self.registry);
                                context.read_produces_value(&place, self.registry)
                                    && context.contents_native_string_access_closed_at(
                                        &place,
                                        self.registry,
                                    )
                            })
                        {
                            return None;
                        }
                        value()
                    }
                    crate::ir::WordExpr::CommandSubstitution { source, .. } => {
                        let mut selected = calls
                            .iter()
                            .filter(|child| child.span.start() == source.span.start());
                        let child = selected.next()?;
                        if selected.next().is_some() {
                            return None;
                        }
                        self.fold_retained_at_depth(child, calls, depth + 1, context)
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Fold the command-substitution interior `inner` (text between `[` and
    /// `]`) to its constant result, or `None` to abstain. The result is the
    /// **raw** value (no re-quoting) — callers that splice it into a word
    /// position re-render it themselves.
    #[must_use]
    pub fn fold_cmd_subst(&self, inner: &str) -> Option<String> {
        self.fold_cmd_subst_resolved(inner).map(|fold| fold.value)
    }

    /// Fold `inner` and retain every registry command identity the result
    /// assumes. This is the code-generation face of the shared fold engine;
    /// analysis-only consumers can continue using [`Self::fold_cmd_subst`].
    #[must_use]
    pub fn fold_cmd_subst_resolved(&self, inner: &str) -> Option<ResolvedConstSubst> {
        self.fold_at_depth(inner, 0, None)
    }

    /// Fold under the actual retained invocation axes, independently of the
    /// catalogue profile. This does not prove that the reached command is stock;
    /// the caller's binding trust contract remains required.
    #[must_use]
    pub fn fold_cmd_subst_in(
        &self,
        inner: &str,
        dialect: tcl_registry::InvocationDialect,
    ) -> Option<String> {
        self.fold_at_depth(inner, 0, Some(dialect))
            .map(|fold| fold.value)
    }

    fn fold_at_depth(
        &self,
        inner: &str,
        depth: u32,
        dialect: Option<tcl_registry::InvocationDialect>,
    ) -> Option<ResolvedConstSubst> {
        if depth > MAX_CONST_SUBST_DEPTH {
            return None;
        }
        let (words, mut command_bindings, mut site_claims) =
            self.literal_words_at_depth(inner, depth, dialect)?;
        let (head, rest) = words.split_first()?;
        if !(self.trusts)(head) {
            return None;
        }
        let arg_refs: Vec<&str> = rest.iter().map(String::as_str).collect();
        let query = match dialect {
            Some(dialect) => Some(dialect.authoring_query()?),
            None => self.registry.own_surface_query(),
        };
        let resolved = self.registry.resolve_call(head, &arg_refs, query)?;
        let spec = resolved.spec;
        // A keyword whose value the enclosing `TclOO` method frame fixes
        // (`[self class]`) answers from the frame rather than from its
        // arguments — it has no `const_fold`, because the value is not a
        // function of the args. Only reachable when the consumer proved a
        // frame; `None` everywhere else.
        if let Some(class) = self.defining_class
            && let Some(folded) = oo_context_fact_fold(spec, rest, class)
        {
            command_bindings.push(
                CommandBindingIdentity::in_rooted_namespace(
                    self.resolution_namespace,
                    head,
                    spec.name,
                )
                .with_namespace_context(self.namespace_context.clone()),
            );
            site_claims.extend(crate::site_claims::pack_facts_claim(self.registry, spec));
            return Some(ResolvedConstSubst {
                value: folded,
                command_bindings,
                site_claims,
                return_type: spec.return_type_for_call(&arg_refs),
            });
        }
        let version = dialect.map_or(self.version, |dialect| dialect.tcl_version);
        let folded = if spec.subcommands.is_empty() {
            match registry_route(spec, None) {
                Some(route) => route_literal(route, spec.name, None, &arg_refs, version)?,
                None => match dialect {
                    Some(dialect) => spec.run_const_fold_in(&arg_refs, dialect),
                    None => spec.run_const_fold(&arg_refs, version),
                }?,
            }
        } else {
            // Subcommand-dispatched builtin (`string`, `namespace`, …): the
            // fold lives on the matching subcommand and sees the args after
            // it.
            let sub = resolved.sub?;
            let (_, sub_rest) = rest.split_first()?;
            let arg_refs: Vec<&str> = sub_rest.iter().map(String::as_str).collect();
            let subcommand = resolved.sub?;
            match registry_route(spec, Some(subcommand)) {
                Some(route) => {
                    route_literal(route, spec.name, Some(subcommand.name), &arg_refs, version)?
                }
                None => match dialect {
                    Some(dialect) => subcommand.run_const_fold_in(&arg_refs, dialect),
                    None => subcommand.run_const_fold(&arg_refs, version),
                }?,
            }
        };
        command_bindings.push(
            CommandBindingIdentity::in_rooted_namespace(self.resolution_namespace, head, spec.name)
                .with_namespace_context(self.namespace_context.clone()),
        );
        site_claims.extend(crate::site_claims::pack_facts_claim(self.registry, spec));
        Some(ResolvedConstSubst {
            value: folded,
            command_bindings,
            site_claims,
            return_type: spec.return_type_for_call(&arg_refs),
        })
    }

    /// Re-lex a command-substitution interior into its literal words.
    /// Returns `None` (bail — do not fold) if any word is not a single clean
    /// literal token: a multi-token word (`foo$bar`), a `{*}` expansion, or a
    /// `$var` that [`Self::lookup_var`](ConstSubstCtx::lookup_var) cannot
    /// resolve. Bare and quoted literal escapes are decoded through the
    /// release-aware lexer owner; a braced literal (`{a b}`, `{a$b}`) yields
    /// its interior text with only Tcl's permitted backslash-newline collapse.
    /// A nested `[cmd …]` substitution is folded recursively:
    /// `[llength [list a b c]]` folds its inner `[list a b c]` to `a b c`
    /// first, so `llength` then sees a constant argument and folds to `3`.
    /// A nested sub that doesn't fold to a constant bails the whole fold.
    #[must_use]
    pub fn literal_words(&self, inner: &str) -> Option<Vec<String>> {
        self.literal_words_at_depth(inner, 0, None)
            .map(|(words, _, _)| words)
    }

    fn literal_words_at_depth(
        &self,
        inner: &str,
        depth: u32,
        dialect: Option<tcl_registry::InvocationDialect>,
    ) -> Option<(
        Vec<String>,
        Vec<CommandBindingIdentity>,
        Vec<tcl_runtime_api::SiteClaim>,
    )> {
        use tcl_lexer::{Lexer, LexerConfig, SourceMap, TokenType};

        // Re-split the substitution under the selected registry profile, so
        // release and dialect word grammar cannot drift from command lookup.
        let config = dialect.map_or_else(
            || LexerConfig::for_profile(self.registry.profile()),
            |dialect| LexerConfig::from_grammar(dialect.lexer_grammar),
        );
        if !crate::segmenter::has_exactly_one_command_with_config(inner, config) {
            return None;
        }
        let sm = SourceMap::new(inner);
        let tokens = Lexer::with_config(inner, config).tokenise_all().ok()?;
        let mut words: Vec<String> = Vec::new();
        let mut command_bindings = Vec::new();
        let mut site_claims = Vec::new();
        let mut prev_is_sep = true;
        for tok in &tokens {
            match tok.kind {
                TokenType::Sep | TokenType::Eol | TokenType::Eof | TokenType::Comment => {
                    prev_is_sep = true;
                }
                TokenType::Esc => {
                    if !prev_is_sep {
                        return None; // multi-token word — not a clean literal
                    }
                    let text = sm.token_text(*tok);
                    let bytes =
                        tcl_lexer::backslash_subst_bytes_in(text.as_bytes(), config.escapes);
                    words.push(std::str::from_utf8(&bytes).ok()?.to_owned());
                    prev_is_sep = false;
                }
                TokenType::Str => {
                    if !prev_is_sep {
                        return None; // multi-token word — not a clean literal
                    }
                    let text = sm.token_text(*tok);
                    words.push(
                        crate::value_transfer::literal_token_value(text, tok.kind, &config)?
                            .into_owned(),
                    );
                    prev_is_sep = false;
                }
                TokenType::Var => {
                    // Resolve a single-token `$var` word to its constant
                    // value (kept as ONE argument so a multi-word value
                    // isn't re-split). A composite word (`foo$bar`), an
                    // array element (`$a(1)` — never a scalar constant), or
                    // a non-constant var bails.
                    if !prev_is_sep {
                        return None;
                    }
                    let name = sm.token_text(*tok);
                    let normalised = normalise_var_name(&format!("${name}")).to_owned();
                    let value =
                        (self.lookup_var)(&normalised).or_else(|| (self.lookup_var)(name))?;
                    words.push(value);
                    prev_is_sep = false;
                }
                TokenType::ExprSugar => {
                    // `JimTcl` `$(…)` expression substitution: the value is
                    // whatever the expression evaluates to at run time, so
                    // there is no literal to fold. Bail conservatively, as
                    // for any other non-constant substitution.
                    return None;
                }
                TokenType::Cmd => {
                    // Nested command substitution: fold it recursively.
                    // Only a const-foldable nested builtin (`[list a b c]`
                    // → `a b c`) yields a literal word the outer fold can
                    // use; anything else bails.
                    if !prev_is_sep {
                        return None;
                    }
                    // A `Cmd` token's text is already the bracket
                    // *interior* (`list a b c`, not `[list a b c]`), so
                    // fold it directly.
                    let nested = sm.token_text(*tok);
                    let folded = self.fold_at_depth(nested, depth + 1, dialect)?;
                    words.push(folded.value);
                    command_bindings.extend(folded.command_bindings);
                    site_claims.extend(folded.site_claims);
                    prev_is_sep = false;
                }
                // `{*}$x`-style expansion is substitution-bearing → bail.
                TokenType::Expand => return None,
            }
        }
        Some((words, command_bindings, site_claims))
    }
}

/// The registry-owned direct route `spec` (or its subcommand `sub`) declares,
/// when it declares one: the evaluator the lattice runs for the same call,
/// whose answer — a decline included — stands for the fold, so no second
/// implementation answers beside it.
fn registry_route(spec: &CommandSpec, sub: Option<&SubCommand>) -> Option<ResolvedSemantics> {
    let resolved = resolve_semantics(spec, sub, None);
    match resolved.route()? {
        EvalRoute::Direct { id } if id.owner() == EvaluatorOwner::Registry => Some(resolved),
        _ => None,
    }
}

/// What `route` answers for `command ?sub? args…` over literal words under
/// `version` — with none, the answer every release gives — as a literal the
/// engine may write back into a script: an exact result with no store, never
/// a byte array, which has no lossless spelling in a script, and never text
/// beyond ASCII, which 8.x reads in the system encoding.
fn route_literal(
    route: ResolvedSemantics,
    command: &str,
    sub: Option<&str>,
    args: &[&str],
    version: Option<TclVersion>,
) -> Option<String> {
    let profile = version.and_then(|v| tcl_dialect::DialectProfile::find(v.dialect_profile_name()));
    let inputs = LiteralInputs::new(command, sub, args, profile);
    let EvalAnswer::Evaluated(outcome) = route
        .semantics()?
        .evaluate(&inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    let ExactValueOrUnavailable::Exact(value) = &outcome.result else {
        return None;
    };
    let byte_array = outcome.types.result == Some(TclType::ByteArray)
        || value.representation == RepresentationEvidence::Constructed(TclType::ByteArray);
    if outcome.has_stores() || byte_array || !value.bytes.is_ascii() {
        return None;
    }
    String::from_utf8(value.bytes.clone()).ok()
}

/// Answer a command substitution from the enclosing method frame, when the
/// registry declares that this command's invoked keyword *is* a frame fact.
///
/// Entirely registry-driven: the word is looked up in the spec's
/// [`CommandSpec::oo_context_facts`] table, so no command or subcommand name
/// appears here. A call carrying anything other than exactly the one keyword
/// word declines — a bare `[self]` (equivalent to `self object`, the
/// receiving instance) has no entry, and neither does any word the table
/// omits.
#[must_use]
pub fn oo_context_fact_fold(spec: &CommandSpec, args: &[String], class: &str) -> Option<String> {
    if spec.oo_context_facts.is_empty() {
        return None;
    }
    let [word] = args else {
        return None;
    };
    let fact = spec
        .oo_context_facts
        .iter()
        .find(|(w, _)| *w == word.as_str())
        .map(|(_, f)| *f)?;
    match fact {
        tcl_registry::OoContextFact::DefiningClass => Some(class.to_owned()),
    }
}

/// Cheap pre-gate: could a fold of the substitution interior `inner` even
/// consult a registry fold? True when the (static, literal) head word
/// resolves to a spec that carries a `const_fold` / versioned fold or a
/// registry-owned direct route, a subcommand with either, or an
/// [`tcl_registry::OoContextFact`] table.
/// Consumers whose trust check is expensive to build (the analyser's lazy
/// whole-module mutation scan) call this first, so that check is only
/// materialised for a substitution that could actually fold.
#[must_use]
pub fn head_may_fold(registry: &CommandRegistry, inner: &str) -> bool {
    let trimmed = inner.trim_start();
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let head = &trimmed[..end];
    if head.is_empty() || head.contains(['$', '[', '\\', '"', '{', '(', '}', ']']) {
        return false;
    }
    let Some(spec) = registry.get(head) else {
        return false;
    };
    spec.const_fold.is_some()
        || spec.const_fold_versioned.is_some()
        || !spec.oo_context_facts.is_empty()
        || registry_route(spec, None).is_some()
        || spec.subcommands.iter().any(|sc| {
            sc.const_fold.is_some()
                || sc.const_fold_versioned.is_some()
                || registry_route(spec, Some(sc)).is_some()
        })
}

/// Whether `body` textually contains any `[cmd …]` opener whose head could
/// fold ([`head_may_fold`]) — the cheap pre-filter the analyser's per-item
/// path uses to decide which deferred proc/method bodies need the
/// whole-file command-trust snapshot attached to their memo key.
/// Deliberately the same predicate the fold itself gates on, so a
/// body this scan clears can never attempt a fold.
#[must_use]
pub fn body_has_fold_candidate(body: &str, registry: &CommandRegistry) -> bool {
    let mut rest = body;
    while let Some(i) = rest.find('[') {
        let tail = &rest[i + 1..];
        // The head word ends at whitespace OR the closing bracket (`[list]`
        // has no interior whitespace at all).
        let end = tail
            .find(|c: char| c.is_whitespace() || c == ']')
            .unwrap_or(tail.len());
        if head_may_fold(registry, &tail[..end]) {
            return true;
        }
        rest = tail;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> &'static CommandRegistry {
        tcl_registry::default_registry()
    }

    fn ctx<'a>(
        registry: &'a CommandRegistry,
        trusts: &'a dyn Fn(&str) -> bool,
        lookup: &'a dyn Fn(&str) -> Option<String>,
    ) -> ConstSubstCtx<'a> {
        ConstSubstCtx {
            registry,
            resolution_namespace: "::",
            namespace_context: None,
            version: None,
            defining_class: None,
            trusts,
            lookup_var: lookup,
        }
    }

    #[test]
    fn range_folding_retains_the_actual_native_result_bytes() {
        let trust = |_: &str| true;
        let lookup = |_: &str| None;
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let supplied = tcl_registry::model::ingress::static_context_for(profile);
            let registry = supplied.commands();
            let dialect = tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap());
            let fold = ctx(registry, &trust, &lookup);
            assert_eq!(
                fold.fold_cmd_subst_in("lrange {x #value end} 1 1", dialect)
                    .as_deref(),
                Some(if profile == "tcl8.4" {
                    "#value"
                } else {
                    "{#value}"
                }),
                "{profile}"
            );
            assert_eq!(
                fold.fold_cmd_subst_in("lrange {x a b} 8 end", dialect)
                    .as_deref(),
                Some("")
            );
            assert!(
                fold.fold_cmd_subst_in("lrange {x a b} nope end", dialect)
                    .is_none()
            );
        }
        assert!(
            ctx(registry(), &trust, &lookup)
                .fold_cmd_subst("lrange {x #value end} 1 1")
                .is_none()
        );
    }

    #[test]
    fn folds_subcommand_dispatch() {
        let trust = |_: &str| true;
        let lookup = |_: &str| None;
        let c = ctx(registry(), &trust, &lookup);
        assert_eq!(
            c.fold_cmd_subst("namespace qualifiers ::tc::X").as_deref(),
            Some("::tc"),
        );
        assert_eq!(c.fold_cmd_subst("string length abc").as_deref(), Some("3"));
    }

    #[test]
    fn actual_native_axes_select_the_fold_independently_of_catalogue_order() {
        let catalogue = CommandRegistry::build_default();
        let trust = |_: &str| true;
        let lookup = |_: &str| None;
        let fold = ctx(&catalogue, &trust, &lookup);
        let tcl = tcl_registry::InvocationDialect::for_version(TclVersion::V9_0);
        let jim = tcl_registry::InvocationDialect::of_profile(
            tcl_registry::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        assert_eq!(
            fold.fold_cmd_subst_in("concat a b c", tcl).as_deref(),
            Some("a b c")
        );
        assert!(fold.fold_cmd_subst_in("concat a b c", jim).is_none());
        let reject = |_: &str| false;
        assert!(
            ctx(&catalogue, &reject, &lookup)
                .fold_cmd_subst_in("concat a b c", tcl)
                .is_none()
        );
        let mut unknown = tcl;
        unknown.native_family = None;
        unknown.core_point = None;
        unknown.tcl_version = None;
        assert!(fold.fold_cmd_subst_in("concat a b c", unknown).is_none());
    }

    #[test]
    fn multiple_commands_decline_const_fold() {
        let trust = |_: &str| true;
        let lookup = |_: &str| None;
        let c = ctx(registry(), &trust, &lookup);
        assert_eq!(c.fold_cmd_subst("string cat a; string cat b"), None);
        // A separator after the sole command does not manufacture another
        // command and keeps the existing fold behaviour.
        assert_eq!(c.fold_cmd_subst("string length abc;").as_deref(), Some("3"));
    }

    #[test]
    fn resolves_constant_vars_through_the_lookup() {
        let trust = |_: &str| true;
        let lookup = |name: &str| (name == "base").then(|| "::a::b".to_owned());
        let c = ctx(registry(), &trust, &lookup);
        assert_eq!(
            c.fold_cmd_subst("namespace qualifiers $base").as_deref(),
            Some("::a"),
        );
        // Unknown var → abstain.
        assert_eq!(c.fold_cmd_subst("namespace qualifiers $other"), None);
    }

    #[test]
    fn untrusted_head_declines() {
        let trust = |name: &str| name != "namespace";
        let lookup = |_: &str| None;
        let c = ctx(registry(), &trust, &lookup);
        assert_eq!(c.fold_cmd_subst("namespace qualifiers ::tc::X"), None);
    }

    #[test]
    fn nested_untrusted_head_declines_the_whole_fold() {
        // The nested `[list …]` is untrusted, so the outer `llength` must
        // not see a manufactured constant.
        let trust = |name: &str| name != "list";
        let lookup = |_: &str| None;
        let c = ctx(registry(), &trust, &lookup);
        assert_eq!(c.fold_cmd_subst("llength [list a b c]"), None);
        let trust_all = |_: &str| true;
        let c = ctx(registry(), &trust_all, &lookup);
        assert_eq!(
            c.fold_cmd_subst("llength [list a b c]").as_deref(),
            Some("3")
        );
    }

    #[test]
    fn frame_fact_folds_only_with_a_proven_class() {
        let trust = |_: &str| true;
        let lookup = |_: &str| None;
        let mut c = ctx(registry(), &trust, &lookup);
        // No proven frame → abstain (a class-side method would raise).
        assert_eq!(c.fold_cmd_subst("self class"), None);
        c.defining_class = Some("::C");
        assert_eq!(c.fold_cmd_subst("self class").as_deref(), Some("::C"));
        // Chained through a nested sub in one step.
        assert_eq!(
            c.fold_cmd_subst("namespace qualifiers [self class]")
                .as_deref(),
            Some(""),
        );
    }

    #[test]
    fn literal_escapes_decode_but_expansions_and_composites_bail() {
        let trust = |_: &str| true;
        let lookup = |_: &str| None;
        let c = ctx(registry(), &trust, &lookup);
        assert_eq!(
            c.fold_cmd_subst(r"string length a\tb").as_deref(),
            Some("3")
        );
        assert_eq!(c.fold_cmd_subst(r"format %s a\ b").as_deref(), Some("a b"));
        assert_eq!(c.fold_cmd_subst(r"format %s \{\}").as_deref(), Some("{}"));
        assert_eq!(
            c.fold_cmd_subst(r"format %s {a\tb}").as_deref(),
            Some(r"a\tb")
        );
        assert_eq!(
            c.fold_cmd_subst("format %s {a\\\n  b}").as_deref(),
            Some("a b")
        );
        assert_eq!(c.fold_cmd_subst("list {*}$xs"), None);
        assert_eq!(c.fold_cmd_subst("string length a$b"), None);
    }

    #[test]
    fn literal_escape_decoding_uses_the_registry_profile() {
        let trust = |_: &str| true;
        let lookup = |_: &str| None;
        let profile_85 =
            tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
        let profile_86 =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let c85 = ctx(
            tcl_registry::model::ingress::static_context_for_profile(profile_85).commands(),
            &trust,
            &lookup,
        );
        let c86 = ctx(
            tcl_registry::model::ingress::static_context_for_profile(profile_86).commands(),
            &trust,
            &lookup,
        );

        assert_eq!(c85.fold_cmd_subst(r"format %s \x123").as_deref(), Some("#"));
        assert_eq!(
            c86.fold_cmd_subst(r"format %s \x123").as_deref(),
            Some("\u{12}3")
        );
    }

    /// A routed command folds through its route — the empty `split` is the
    /// empty list (#2418) — and a byte array or an answer beyond ASCII stays
    /// a call: the engine writes its answer back into a script, where a byte
    /// array has no lossless spelling and 8.x reads text in the system
    /// encoding.
    #[test]
    fn a_routed_command_folds_through_its_route_within_ascii() {
        let trust = |_: &str| true;
        let lookup = |_: &str| None;
        let c = ctx(registry(), &trust, &lookup);
        assert_eq!(c.fold_cmd_subst("split {}").as_deref(), Some(""));
        assert_eq!(c.fold_cmd_subst("split {a b}").as_deref(), Some("a b"));
        assert_eq!(
            c.fold_cmd_subst("file dirname a/b/c").as_deref(),
            Some("a/b")
        );
        assert_eq!(c.fold_cmd_subst("file dirname C:/a"), None);
        assert_eq!(c.fold_cmd_subst("binary format a3 abc"), None);
        assert_eq!(c.fold_cmd_subst("binary format c* {128 195 255}"), None);
        assert_eq!(c.fold_cmd_subst("format %c 233"), None);
        assert_eq!(c.fold_cmd_subst("format %c 65").as_deref(), Some("A"));
    }

    #[test]
    fn head_may_fold_gates_cheaply() {
        let reg = registry();
        assert!(head_may_fold(reg, "namespace qualifiers ::a::b"));
        assert!(head_may_fold(reg, "string length abc"));
        assert!(head_may_fold(reg, "self class"));
        // `puts` has no fold surface; a dynamic head never folds.
        assert!(!head_may_fold(reg, "puts hi"));
        assert!(!head_may_fold(reg, "$cmd x"));
        assert!(!head_may_fold(reg, ""));
    }
}
