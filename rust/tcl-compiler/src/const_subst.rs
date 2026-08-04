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
//! escape, an expansion, an unresolvable variable, a class-side method
//! frame) declines the fold; a wrong constant is a miscompile, a missed one
//! only a lost optimisation.

use tcl_registry::{CommandRegistry, CommandSpec};

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
    /// Dialect name forwarded to versioned folds (`const_fold_versioned`);
    /// `None` when the consumer has no dialect context.
    pub dialect: Option<&'a str>,
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

impl ConstSubstCtx<'_> {
    /// Fold the command-substitution interior `inner` (text between `[` and
    /// `]`) to its constant result, or `None` to abstain. The result is the
    /// **raw** value (no re-quoting) — callers that splice it into a word
    /// position re-render it themselves.
    #[must_use]
    pub fn fold_cmd_subst(&self, inner: &str) -> Option<String> {
        self.fold_at_depth(inner, 0)
    }

    fn fold_at_depth(&self, inner: &str, depth: u32) -> Option<String> {
        if depth > MAX_CONST_SUBST_DEPTH {
            return None;
        }
        let words = self.literal_words_at_depth(inner, depth)?;
        let (head, rest) = words.split_first()?;
        if !(self.trusts)(head) {
            return None;
        }
        let spec = self.registry.get(head)?;
        // A keyword whose value the enclosing `TclOO` method frame fixes
        // (`[self class]`) answers from the frame rather than from its
        // arguments — it has no `const_fold`, because the value is not a
        // function of the args. Only reachable when the consumer proved a
        // frame; `None` everywhere else.
        if let Some(class) = self.defining_class
            && let Some(folded) = oo_context_fact_fold(spec, rest, class)
        {
            return Some(folded);
        }
        if spec.subcommands.is_empty() {
            let arg_refs: Vec<&str> = rest.iter().map(String::as_str).collect();
            spec.run_const_fold(&arg_refs, self.dialect)
        } else {
            // Subcommand-dispatched builtin (`string`, `namespace`, …): the
            // fold lives on the matching subcommand and sees the args after
            // it.
            let (sub, sub_rest) = rest.split_first()?;
            let arg_refs: Vec<&str> = sub_rest.iter().map(String::as_str).collect();
            spec.resolve_subcommand(sub)?
                .run_const_fold(&arg_refs, self.dialect)
        }
    }

    /// Re-lex a command-substitution interior into its literal words.
    /// Returns `None` (bail — do not fold) if any word is not a single clean
    /// literal token: a multi-token word (`foo$bar`), a word carrying a
    /// backslash escape (decoding is out of scope here), a `{*}` expansion,
    /// or a `$var` that [`Self::lookup_var`](ConstSubstCtx::lookup_var)
    /// cannot resolve. A braced literal (`{a b}`, `{a$b}`) yields its
    /// interior text — the contents are literal, so they fold soundly. A
    /// nested `[cmd …]` substitution is folded recursively:
    /// `[llength [list a b c]]` folds its inner `[list a b c]` to `a b c`
    /// first, so `llength` then sees a constant argument and folds to `3`.
    /// A nested sub that doesn't fold to a constant bails the whole fold.
    #[must_use]
    pub fn literal_words(&self, inner: &str) -> Option<Vec<String>> {
        self.literal_words_at_depth(inner, 0)
    }

    fn literal_words_at_depth(&self, inner: &str, depth: u32) -> Option<Vec<String>> {
        use tcl_lexer::{Lexer, SourceMap, TokenType};

        let sm = SourceMap::new(inner);
        let tokens = Lexer::new(inner).tokenise_all().ok()?;
        let mut words: Vec<String> = Vec::new();
        let mut prev_is_sep = true;
        for tok in &tokens {
            match tok.kind {
                TokenType::Sep | TokenType::Eol | TokenType::Eof | TokenType::Comment => {
                    prev_is_sep = true;
                }
                TokenType::Esc | TokenType::Str => {
                    if !prev_is_sep {
                        return None; // multi-token word — not a clean literal
                    }
                    let text = sm.token_text(*tok);
                    if text.contains('\\') {
                        return None; // unhandled escape — bail conservatively
                    }
                    words.push(text.to_owned());
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
                    words.push(folded);
                    prev_is_sep = false;
                }
                // `{*}$x`-style expansion is substitution-bearing → bail.
                TokenType::Expand => return None,
            }
        }
        Some(words)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> &'static CommandRegistry {
        tcl_registry::cache::default_registry()
    }

    fn ctx<'a>(
        registry: &'a CommandRegistry,
        trusts: &'a dyn Fn(&str) -> bool,
        lookup: &'a dyn Fn(&str) -> Option<String>,
    ) -> ConstSubstCtx<'a> {
        ConstSubstCtx {
            registry,
            dialect: None,
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
    fn escapes_expansions_and_composites_bail() {
        let trust = |_: &str| true;
        let lookup = |_: &str| None;
        let c = ctx(registry(), &trust, &lookup);
        assert_eq!(c.fold_cmd_subst(r"string length a\tb"), None);
        assert_eq!(c.fold_cmd_subst("list {*}$xs"), None);
        assert_eq!(c.fold_cmd_subst("string length a$b"), None);
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
