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

//! Folding an assigned value word: literals, variable references, `foreach` lists and pure command substitutions.

use super::super::{BuiltinFoldInputs, FoldTrust};
use super::RetainedExpressionFoldInputs;
use super::definition_values::{
    fold_original_substitution_value, resolve_simple_var_ref, source_read_key,
};
use super::math_expressions::{env_from_uses, env_from_uses_numeric, tcl_value_to_const};
use crate::analyses::{ConstValue, LatticeValue};
use crate::codegen_helpers::split_list_values;
use crate::ssa::{SsaSourceView, Symbol, ValueKey};
use crate::tcl_expr_eval::{FoldPolicy, TclValue, eval_tcl_expr_with_policy};
use std::collections::HashMap;
use tcl_dialect::StringCharacterModel;

/// Extract iteration-variable elements from a foreach list arg
/// that is a literal (no `$` / `[` substitution).
///
/// `list_text` must already be delimiter-stripped by the segmenter (the shape
/// `Statement::Foreach`'s `list_arg` is built in, and the shape every caller
/// of this function must pass) — a second strip here would wrongly peel a
/// single-element list like `{{a b} {c d}}` (segmented to `{a b} {c d}`) down
/// to the elements `a` and `b}`. See `cfg_builder::list_literal_nonempty` and
/// `analyser::bounds_checks` for the same contract.
///
/// Behaviour:
/// - Strip whitespace.
/// - Split on Tcl list semantics (`Tcl_SplitList`), not ASCII whitespace.
/// - Returns `None` for anything that starts with `$` or `[` so
///   callers fall through to
///   [`resolve_foreach_list_via_lattice`].
#[must_use]
pub fn extract_foreach_elements(
    list_text: &str,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<Vec<String>> {
    let stripped = list_text.trim();
    if stripped.is_empty() {
        return Some(Vec::new());
    }
    if stripped.starts_with('[') || stripped.starts_with('$') {
        return None;
    }
    // List-aware split (Tcl_SplitList semantics): a nested-brace list like
    // `a {b c} d` yields the three elements `a`, `b c`, `d` — not the four
    // whitespace runs `a`, `{b`, `c}`, `d` a naive split would produce.
    Some(split_list_values(stripped, rules))
}

/// Resolve `$var` / `${var}` to a `Vec<String>` of list elements
/// via the SCCP lattice. Returns `None` when the operand is not a
/// simple var reference or its lattice value is not a
/// Const(String).
#[must_use]
pub fn resolve_foreach_list_via_lattice<'a, S1, S2>(
    list_text: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    rules: tcl_syntax::word_rules::WordValueRules,
    reference_config: Option<tcl_lexer::LexerConfig>,
) -> Option<Vec<String>>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
{
    let ssa = ssa.into();
    let stripped = list_text.trim();
    let name =
        tcl_syntax::naming::whole_scalar_variable_reference_name(stripped, reference_config?)?;
    let sym = ssa.symbol(name)?;
    let ver = uses.get(&sym).copied()?;
    match values.get(&(sym, ver))? {
        // The lattice value is the variable's runtime string; splitting it as a
        // `foreach` list uses Tcl list semantics (nested-brace aware), not a
        // whitespace split.
        LatticeValue::Const(ConstValue::String(s)) => Some(split_list_values(s, rules)),
        _ => None,
    }
}

// AssignValue folding

/// Follow the original passthrough store's operand to its retained command
/// substitution. No spelling fallback can prove that physical store or read.
fn fold_stored_substitution<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    tokens: &crate::ir::CommandTokens,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: SsaSourceView<'_>,
    inputs: BuiltinFoldInputs<'_>,
    policy: FoldPolicy,
) -> Option<String> {
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        inputs.registry,
        inputs.source_metadata_input?,
    )?;
    let invocation = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        inputs.registry,
        Some(metadata),
        tokens,
    )?;
    let (_, word) = invocation.stored_value_operand(
        &tokens.source_binding.as_ref()?.variable_context,
        inputs.registry,
    )?;
    let (_, site) = word.sole_command_substitution()?;
    fold_original_substitution_value(
        tokens,
        site,
        uses,
        values,
        ssa,
        RetainedExpressionFoldInputs {
            source_metadata_input: inputs.source_metadata_input,
            registry: inputs.registry,
            policy,
            math: None,
        },
    )
}

/// Fold the RHS of an `AssignValue` statement to a lattice value.
///
/// Covers three tiers:
/// 1. **Plain literal** — no `$` / `[` → `Const(parse_literal_value)`.
/// 2. **Simple var reference** `$x` / `${x}` → lattice lookup.
/// 3. **Command substitution** `[cmd args…]` → delegate to
///    [`try_fold_cmd_subst`], then (when `folds` is supplied) to the
///    registry const-fold engine ([`BuiltinFoldInputs`]).
///
/// Anything else widens to `Overdefined`.
pub(super) fn fold_assign_value<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    value: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
    reference_config: Option<tcl_lexer::LexerConfig>,
) -> LatticeValue {
    let ssa = ssa.into();
    let stripped = value.trim();
    // Plain literal.
    if !stripped.contains('$') && !stripped.contains('[') {
        return LatticeValue::Const(parse_literal_value(stripped));
    }
    // A retained set value word identifies its exact read. The statement's
    // later spelling map cannot represent alias reselection within argv.
    if let Some(tokens) = ssa.source_tokens()
        && let Some(effective) = crate::registry_invocation::effective_command_words(tokens)
        && let Some(word) = effective.words.get(2)
        && word.sole_variable_substitution().is_some()
    {
        return ssa
            .read_word(word)
            .and_then(|read| read.version.map(|version| (read.symbol, version)))
            .map_or(LatticeValue::Overdefined, |key| {
                values.get(&key).cloned().unwrap_or(LatticeValue::Unknown)
            });
    }
    if let Some(tokens) = ssa.source_tokens() {
        return folds
            .and_then(|inputs| fold_stored_substitution(tokens, uses, values, ssa, inputs, policy))
            .map_or(LatticeValue::Overdefined, |value| {
                LatticeValue::Const(parse_literal_value(&value))
            });
    }
    // Legacy tokenless callers retain the unique-binding compatibility view.
    if let Some(resolved) = resolve_simple_var_ref(stripped, uses, values, ssa, reference_config) {
        return resolved;
    }
    // Command substitution.
    if stripped.starts_with('[') && stripped.ends_with(']') {
        if let Some(lv) =
            try_fold_cmd_subst(stripped, uses, values, ssa, policy, folds, reference_config)
        {
            return lv;
        }
        // Literal lists and their selected native serialization use the same
        // checked registry dispatch as the optional multi-hop engine.
        if let Some(f) = folds {
            let trusts = |name: &str| {
                let constructs_list = f
                    .registry
                    .get_for_surface(name, f.registry.own_surface_query())
                    .is_some_and(|spec| {
                        spec.native_result
                            == Some(
                                tcl_registry::native_result::NativeResultContract::ListArguments {
                                    from: 0,
                                },
                            )
                    });
                if constructs_list && matches!(f.trust, FoldTrust::ObservedBindings) {
                    f.mutations.observed_binding_is_the_builtin(name)
                } else {
                    (f.registry_engine || constructs_list) && f.mutations.trusts(name)
                }
            };
            let lookup = |name: &str| {
                f.registry_engine
                    .then(|| lattice_const_text(name, uses, values, ssa, policy))
                    .flatten()
            };
            let context = crate::const_subst::ConstSubstCtx {
                registry: f.registry,
                resolution_namespace: "::",
                namespace_context: None,
                version: f
                    .dialect
                    .and_then(tcl_dialect::DialectProfile::const_fold_version),
                defining_class: f.defining_class,
                trusts: &trusts,
                lookup_var: &lookup,
                script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
            };
            if let Some(dialect) = policy.invocation_dialect
                && let Some(folded) =
                    context.fold_cmd_subst_in(&stripped[1..stripped.len() - 1], dialect)
            {
                return LatticeValue::Const(parse_literal_value(&folded));
            }
        }
    }
    LatticeValue::Overdefined
}

/// Resolve `name` to the textual form of its lattice constant at this
/// statement's use version, or `None` when it is not a single `Const` —
/// the variable-lookup the registry const-fold engine runs under (see
/// [`fold_assign_value`]).
pub(super) fn lattice_const_text<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    name: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
) -> Option<String> {
    let ssa = ssa.into();
    let key = source_read_key(name, uses, ssa)?;
    match values.get(&key)? {
        LatticeValue::Const(ConstValue::String(s)) => Some(s.clone()),
        LatticeValue::Const(ConstValue::Int(i)) => Some(i.to_string()),
        LatticeValue::Const(ConstValue::Bool(b)) => Some(if *b { "1" } else { "0" }.to_owned()),
        LatticeValue::Const(ConstValue::Float(f)) => {
            crate::tcl_expr_eval::format_tcl_value_with_policy(&TclValue::Float(*f), policy)
        }
        _ => None,
    }
}

/// Try to constant-fold a `[cmd args…]` command substitution.
///
/// Recognised forms:
/// - `[list arg1 arg2 …]` with all-literal args → folded list text.
/// - `[llength {a b c}]` / `[llength "a b c"]` → integer element count.
/// - `[string length "text"]` → integer character count.
/// - `[expr {EXPR}]` — parses the inner expression and folds it
///   under the current lattice (bridges to the expression evaluator).
///
/// Returns `None` for anything else so callers widen to
/// Overdefined.
/// Resolve a single command operand to its constant string value: a literal
/// word (optionally brace/quote wrapped), or a pure `$var` / `${var}` whose
/// SCCP lattice value is a constant. Returns `None` for anything that isn't a
/// compile-time constant (array refs, command substitutions, unknown vars),
/// so the caller skips folding.
pub(super) fn resolve_const_string<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    arg: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
    reference_config: Option<tcl_lexer::LexerConfig>,
) -> Option<String> {
    let ssa = ssa.into();
    let arg = arg.trim();
    if arg.starts_with('$') {
        let name =
            tcl_syntax::naming::whole_scalar_variable_reference_name(arg, reference_config?)?;
        let key = source_read_key(name, uses, ssa)?;
        return match values.get(&key)? {
            LatticeValue::Const(ConstValue::String(s)) => Some(s.clone()),
            LatticeValue::Const(ConstValue::Int(i)) => Some(i.to_string()),
            LatticeValue::Const(ConstValue::Bool(b)) => Some(if *b { "1" } else { "0" }.to_owned()),
            LatticeValue::Const(ConstValue::Float(f)) => {
                crate::tcl_expr_eval::format_tcl_value_with_policy(&TclValue::Float(*f), policy)
            }
            _ => None,
        };
    }
    // A literal word with no interpolation or command substitution.
    if !arg.contains('$') && !arg.contains('[') {
        return Some(strip_one_level(arg).to_owned());
    }
    None
}

// Exact command values use registry declarations; expression substitution
// policy below remains tied to the legacy tokenless source view.
pub(super) fn try_fold_cmd_subst<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    value: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
    reference_config: Option<tcl_lexer::LexerConfig>,
) -> Option<LatticeValue> {
    let ssa = ssa.into();
    // Each arm below *is* a builtin's semantics, so it may only run while
    // that name still denotes the builtin: after `rename list mylist` or a
    // shadowing `proc format …` anywhere in the unit, `[list a 1 a 2]` is a
    // call to something else entirely. This is the same trust fact the
    // registry-driven engine below consults; these arms run ahead of it, so
    // they must ask for themselves.
    //
    // Absent `folds` the caller holds **no** whole-module command view, so
    // there is no evidence the head still denotes its builtin and the arms
    // decline. An earlier revision inverted that — a missing fact trusted
    // everything — on the premise that no rewrite lands from a lattice built
    // without the fact. That premise was false: the optimiser's projection
    // takes the shared per-unit lattice as its baseline and only *adds* to it
    // from the trusted re-run, so `proc llength {l} {return 99}` had the
    // shared lattice answer `3` for `set v [llength {a b c}]` while the
    // trusted O103 fold answered `99` for the same call (#2164).
    let trusted = |name: &str| {
        folds.is_some_and(|f| match f.trust {
            FoldTrust::WholeModule => f.mutations.trusts(name),
            FoldTrust::ObservedBindings => f.mutations.observed_binding_is_the_builtin(name),
        })
    };

    if let Some(folds) = folds {
        let direct_trust = |name: &str| {
            use tcl_registry::value_transfer::{EvalRoute, EvaluatorOwner, resolve_semantics};
            trusted(name)
                && (folds.registry_engine
                    || folds.registry.get_for_surface(name, folds.registry.own_surface_query()).is_some_and(|spec| {
                        matches!(resolve_semantics(spec, None, None).route(), Some(EvalRoute::Direct { id }) if id.owner() == EvaluatorOwner::Registry)
                    }))
        };
        let lookup = |name: &str| lattice_const_text(name, uses, values, ssa, policy);
        let context = crate::const_subst::ConstSubstCtx {
            registry: folds.registry,
            resolution_namespace: "::",
            namespace_context: None,
            version: folds
                .dialect
                .and_then(tcl_dialect::DialectProfile::const_fold_version),
            defining_class: folds.defining_class,
            trusts: &direct_trust,
            lookup_var: &lookup,
            script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
        };
        let inner = value.strip_prefix('[')?.strip_suffix(']')?;
        let folded = policy.invocation_dialect.map_or_else(
            || context.fold_cmd_subst(inner),
            |dialect| context.fold_cmd_subst_in(inner, dialect),
        );
        if let Some(value) = folded {
            return Some(LatticeValue::Const(parse_literal_value(&value)));
        }
    }

    // Registry dispatch below owns a literal list's native result spelling.
    // `[format "..." args…]` with literal args.
    // The document's escape grammar, from the same resolved profile the rest
    // of the policy's axes come from. A caller with no dialect keeps the 9.0
    // default, which is what `FoldPolicy::numbers` documents for its own axis
    // — the fold stays available, it just stops guessing once a dialect is
    // known.
    let escapes = policy
        .dialect
        .map_or_else(tcl_dialect::EscapeSyntax::default, |profile| {
            profile.grammar.escapes
        });
    if trusted("format")
        && let Some(folded) = crate::codegen_helpers::try_format_fold(value, escapes)
    {
        return Some(LatticeValue::Const(ConstValue::String(folded)));
    }

    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let (cmd, rest) = split_head(inner);
    if !trusted(cmd) {
        return None;
    }

    // `[string length OPERAND]` where OPERAND resolves to a constant
    // string — a literal word, or a `$var` whose lattice value is known.
    // (Counting the chars of the *unresolved* operand text would mis-fold
    // `string length $s` to the length of "$s".)
    // value-transfer-ok: irreducible — tokenless legacy expression policy.
    // registry-axis-ok: command — tokenless legacy fold; the tokened path asks the registry; until the tokenless SCCP caller is retired
    if cmd == "string" {
        if let Some(after_cmd) = rest {
            let (sub, sub_rest) = split_head(after_cmd.trim());
            // value-transfer-ok: irreducible — tokenless legacy fold, as above.
            // registry-axis-ok: command — tokenless legacy fold; the tokened path asks the registry; until the tokenless SCCP caller is retired
            if sub == "length"
                && let Some(raw) = sub_rest
                && let Some(s) =
                    resolve_const_string(raw.trim(), uses, values, ssa, policy, reference_config)
            {
                // The selected release's character model, over the text the
                // release reads. A word spelt in ASCII (escapes included)
                // reads alike under every encoding; otherwise a non-ASCII
                // value declines before 9.0, whose script reader follows
                // `encoding system`.
                let raw = raw.trim();
                let spelt_in_ascii = raw.is_ascii() && !raw.contains(['$', '[']);
                let count = if spelt_in_ascii {
                    StringCharacterModel::count_for(policy.characters, &s)?
                } else {
                    StringCharacterModel::count_authored(policy.characters, &s)?
                };
                let len = i64::try_from(count).unwrap_or(i64::MAX);
                return Some(LatticeValue::Const(ConstValue::Int(len)));
            }
        }
        return None;
    }

    // `[expr {EXPR}]` — parse + fold under the current lattice.
    // value-transfer-ok: irreducible — tokenless legacy expression policy.
    // registry-axis-ok: command — tokenless legacy fold; the tokened path asks the registry; until the tokenless SCCP caller is retired
    if cmd == "expr" {
        let arg = rest?.trim();
        // Braced (`expr {…}`) vs quoted / bare (`expr "…"`, `expr …`) changes
        // the substitution model. In a braced expr the `$var` references are
        // resolved by *expr* itself, so a string-valued var is a valid string
        // operand (`expr {$a == $b}` with a="alpha" → string compare → 0). In
        // a quoted / bare expr Tcl substitutes the variable *values* textually
        // *before* parsing, so a non-numeric value becomes an invalid bareword
        // and the whole expr errors at runtime (`expr "$a == $b"` →
        // `expr "alpha == beta"` → `invalid bareword "alpha"`). Folding that
        // to `0` would turn an erroring program into a silent value.
        //
        // Numeric values survive textual substitution as valid expr tokens,
        // so for the non-braced form restrict the env to numeric constants:
        // a string-valued var is then left unbound and the fold bails,
        // matching Tcl.
        let braced = arg.starts_with('{');
        let expr_text = strip_one_level(arg);
        let expr = crate::expr_parser::parse_expr_for_profile(expr_text, policy.dialect);
        let env = if braced {
            env_from_uses(uses, values, ssa)
        } else {
            env_from_uses_numeric(uses, values, ssa)
        };
        return eval_tcl_expr_with_policy(&expr, &env, policy)
            .map(|v| LatticeValue::Const(tcl_value_to_const(v)));
    }

    None
}

/// Split a command-substitution body into `(head_word, rest)`.
/// `rest` is `None` if the body is a single word, otherwise the
/// remaining text with the leading whitespace stripped.
pub(super) fn split_head(text: &str) -> (&str, Option<&str>) {
    let trimmed = text.trim_start();
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let head = &trimmed[..end];
    if end >= trimmed.len() {
        return (head, None);
    }
    let rest = trimmed[end..].trim_start();
    if rest.is_empty() {
        (head, None)
    } else {
        (head, Some(rest))
    }
}

/// Strip one level of `{…}` or `"…"` wrapping, returning the
/// inside trimmed.
pub(super) fn strip_one_level(text: &str) -> &str {
    if text.len() >= 2 {
        let bytes = text.as_bytes();
        if (bytes[0] == b'{' && bytes[text.len() - 1] == b'}')
            || (bytes[0] == b'"' && bytes[text.len() - 1] == b'"')
        {
            return text[1..text.len() - 1].trim();
        }
    }
    text
}

/// Parse a literal text as a [`ConstValue`]: prefers integer, then string fallback.
///
/// Only collapses to [`ConstValue::Int`] when the canonical integer text
/// round-trips (`str(int(s)) == s`).  A leading-zero literal such as `"08"` or
/// `"010"` parses as 8 / 10 but does *not* round-trip, so it is kept as a
/// string — preserving the identity SCCP needs to compare it correctly under
/// each dialect's leading-zero rule (octal in tcl8.x, decimal in tcl9.0).
/// Likewise `"+5"` / `"-0"` are kept as strings (they don't round-trip).
#[must_use]
pub fn parse_literal_value(text: &str) -> ConstValue {
    let stripped = text.trim();
    // Decimal integer grammar `[+-]?[0-9]+`.
    let digits = stripped.strip_prefix(['+', '-']).unwrap_or(stripped);
    let is_decimal_int = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
    if is_decimal_int
        && let Ok(i) = stripped.parse::<i64>()
        && i.to_string() == stripped
        // Only when the literal *is* the integer, with nothing around it.
        // `" 5"` renders back as `5`, losing the space the program keeps.
        && stripped == text
    {
        return ConstValue::Int(i);
    }
    // The exact spelling, not the trimmed one. Trimming here corrupted the
    // value for every consumer of the lattice: `set p { again}` reached the
    // lattice as `again`, so `append s $p` was rewritten to `append s again`
    // and the program printed `helloagain` where tclsh prints `hello again`
    // (#2052). Whitespace inside a Tcl word is data.
    ConstValue::String(text.to_owned())
}
