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

//! Shared constant command-substitution fold engine (issues #1132 / #1134).
//!
//! Answers one question for three different consumers: *does this
//! `[cmd args…]` command substitution evaluate to a compile-time constant?*
//! Everything command-specific comes from registry data — the
//! [`tcl_registry::CommandSpec::const_fold`] /
//! [`tcl_registry::SubCommand::const_fold`] callbacks (pure functions of the
//! constant argument words) and the
//! [`tcl_registry::CommandSpec::oo_context_facts`] table (a keyword whose
//! value the enclosing `TclOO` method frame fixes, like `[self class]`).
//! No command name is matched here.
//!
//! The three consumers, each with its own view of "what is a constant
//! variable" and "which commands still have their original semantics":
//!
//! * the **optimiser propagation pass** (`optimiser/propagation.rs`, the
//!   O129 rewrite) — constants from the projected SCCP lattice, trust from
//!   the whole-module [`crate::command_binding::ModuleCommandMutations`]
//!   scan;
//! * **SCCP lattice evaluation itself** (`crate::sccp`, issue #1134) —
//!   constants resolved per SSA use version, so a folded value re-enters the
//!   lattice and multi-statement chains
//!   (`set base [self class]; set ns [namespace qualifiers $base]`) fold to
//!   fixpoint under SCCP's ordinary monotone iteration;
//! * the **analyser** (`analyser/handlers.rs`, issue #1132) — constants
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
use tcl_registry::{CommandRegistry, CommandSpec, TclType};
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
    /// Resolved Tcl release forwarded to versioned folds
    /// (`const_fold_versioned`); `None` when the consumer has no release fact.
    pub version: Option<TclVersion>,
    /// The fully-qualified class defining the enclosing `TclOO` method
    /// implementation, when the consumer *proved* the frame (instance-side
    /// method of a statically-named, never-renamed class). Enables the
    /// registry [`tcl_registry::OoContextFact`] folds (`[self class]`).
    /// `None` abstains from every frame-fact fold.
    pub defining_class: Option<&'a str>,
    /// Trust oracle: `true` when `name` still denotes its original command
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
/// setup such as `VERIFY_DICT` without recognizing a command name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConstSubst {
    /// Raw result value, before any caller-specific word quoting.
    pub value: String,
    /// Exact source spelling → registry implementation dependencies.
    pub command_bindings: Vec<CommandBindingIdentity>,
    /// Registry-declared return type of the outer invocation.
    pub return_type: Option<TclType>,
}

impl ConstSubstCtx<'_> {
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
        self.fold_at_depth(inner, 0)
    }

    fn fold_at_depth(&self, inner: &str, depth: u32) -> Option<ResolvedConstSubst> {
        if depth > MAX_CONST_SUBST_DEPTH {
            return None;
        }
        let (words, mut command_bindings) = self.literal_words_at_depth(inner, depth)?;
        let (head, rest) = words.split_first()?;
        if !(self.trusts)(head) {
            return None;
        }
        let arg_refs: Vec<&str> = rest.iter().map(String::as_str).collect();
        let resolved =
            self.registry
                .resolve_call(head, &arg_refs, self.registry.own_surface_query())?;
        let spec = resolved.spec;
        // A keyword whose value the enclosing `TclOO` method frame fixes
        // (`[self class]`) answers from the frame rather than from its
        // arguments — it has no `const_fold`, because the value is not a
        // function of the args. Only reachable when the consumer proved a
        // frame; `None` everywhere else.
        if let Some(class) = self.defining_class
            && let Some(folded) = oo_context_fact_fold(spec, rest, class)
        {
            command_bindings.push(CommandBindingIdentity::in_rooted_namespace(
                self.resolution_namespace,
                head,
                spec.name,
            ));
            return Some(ResolvedConstSubst {
                value: folded,
                command_bindings,
                return_type: spec.return_type_for_call(&arg_refs),
            });
        }
        let folded = if spec.subcommands.is_empty() {
            spec.run_const_fold(&arg_refs, self.version)?
        } else {
            // Subcommand-dispatched builtin (`string`, `namespace`, …): the
            // fold lives on the matching subcommand and sees the args after
            // it.
            let (_, sub_rest) = rest.split_first()?;
            let arg_refs: Vec<&str> = sub_rest.iter().map(String::as_str).collect();
            resolved.sub?.run_const_fold(&arg_refs, self.version)?
        };
        command_bindings.push(CommandBindingIdentity::in_rooted_namespace(
            self.resolution_namespace,
            head,
            spec.name,
        ));
        Some(ResolvedConstSubst {
            value: folded,
            command_bindings,
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
        self.literal_words_at_depth(inner, 0)
            .map(|(words, _)| words)
    }

    fn literal_words_at_depth(
        &self,
        inner: &str,
        depth: u32,
    ) -> Option<(Vec<String>, Vec<CommandBindingIdentity>)> {
        use tcl_lexer::{Lexer, LexerConfig, SourceMap, TokenType};

        // Re-split the substitution under the selected registry profile, so
        // release and dialect word grammar cannot drift from command lookup.
        let config = LexerConfig::for_profile(self.registry.profile());
        if !crate::segmenter::has_exactly_one_command_with_config(inner, config) {
            return None;
        }
        let sm = SourceMap::new(inner);
        let tokens = Lexer::with_config(inner, config).tokenise_all().ok()?;
        let mut words: Vec<String> = Vec::new();
        let mut command_bindings = Vec::new();
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
                    words.push(tcl_lexer::backslash_subst_in(text, config.escapes).into_owned());
                    prev_is_sep = false;
                }
                TokenType::Str => {
                    if !prev_is_sep {
                        return None; // multi-token word — not a clean literal
                    }
                    let text = sm.token_text(*tok);
                    words.push(
                        tcl_syntax::word_rules::WordValueRules::from_config(&config)
                            .collapse_braced_word(text)
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
                    let folded = self.fold_at_depth(nested, depth + 1)?;
                    words.push(folded.value);
                    command_bindings.extend(folded.command_bindings);
                    prev_is_sep = false;
                }
                // `{*}$x`-style expansion is substitution-bearing → bail.
                TokenType::Expand => return None,
            }
        }
        Some((words, command_bindings))
    }
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
/// resolves to a spec that carries a `const_fold` / versioned fold, a
/// subcommand with one, or an [`tcl_registry::OoContextFact`] table.
/// Consumers whose trust oracle is expensive to build (the analyser's lazy
/// whole-module mutation scan, issue #1132) call this first so the oracle is
/// only materialised for a substitution that could actually fold.
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
        || spec
            .subcommands
            .iter()
            .any(|sc| sc.const_fold.is_some() || sc.const_fold_versioned.is_some())
}

/// Whether `body` textually contains any `[cmd …]` opener whose head could
/// fold ([`head_may_fold`]) — the cheap pre-filter the analyser's per-item
/// path uses to decide which deferred proc/method bodies need the
/// whole-file command-trust snapshot attached to their memo key (issue
/// #1132). Deliberately the same predicate the fold itself gates on, so a
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
            version: None,
            defining_class: None,
            trusts,
            lookup_var: lookup,
        }
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
