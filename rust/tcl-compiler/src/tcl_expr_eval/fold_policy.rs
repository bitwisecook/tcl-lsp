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

//! `FoldPolicy`: the dialect-derived facts a constant fold needs, and the dialect questions about octal numerals and math functions.

use tcl_dialect::{NumberSpelling, NumberSyntax, StringCharacterModel};

/// The dialect-derived facts a constant fold needs: the leading-zero octal
/// rule, and whether the dialect's `expr` grammar carries the iRules word
/// operators (`contains`, `starts_with`, `equals`, …).
///
/// Bundled into one `Copy` value rather than threaded as two parallel
/// parameters, because the passes that need it — SCCP, the static-loop
/// simulator — already carry `octal` through a long chain of helpers, several
/// of which sit on the `clippy::too_many_arguments` ceiling.  Adding a
/// further dialect fact then means extending this struct in one place instead
/// of every signature on the chain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FoldPolicy {
    /// Exact invocation snapshot for engine-owned native function tables.
    pub invocation_dialect: Option<tcl_registry::InvocationDialect>,
    /// Integer representation and overflow policy of the selected engine.
    /// Legacy callers without an engine retain the modern Tcl tower.
    pub arithmetic: Option<tcl_dialect::NativeArithmetic>,
    /// Leading-zero octal policy: `Some(true)` = the 8.x octal rule,
    /// `Some(false)` = the 9.0 decimal rule, `None` = decline to fold a
    /// dialect-ambiguous leading-zero operand.
    pub octal: Option<bool>,
    /// The active dialect's profile, for the passes that re-parse a nested
    /// `[expr …]` while folding: the parse must use the same grammar the
    /// document was lexed under, never the ambient one.
    pub dialect: Option<&'static tcl_dialect::DialectProfile>,
    /// Explicit authored F5 predicate recipe for this fold. It grants no
    /// native C/Jim execution or measured appliance-context capability.
    pub f5_predicates: Option<tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider>,
    /// Explicitly installed logical expression parser, matching the one the
    /// source walk prepared its expression witnesses under. Never inferred
    /// from the dialect.
    pub expression_parser: Option<tcl_registry::invocation_words::LogicalExpressionParseProvider>,
    /// What the active dialect counts as a string character: `Some(model)`
    /// folds character counts under that model, `None` declines a fold whose
    /// answer the Tcl 8 and Tcl 9 models disagree on — the same
    /// dialect-ambiguity rule as [`Self::octal`].
    pub characters: Option<StringCharacterModel>,
    /// Which numeric literal forms the active dialect accepts: `Some(syntax)`
    /// parses operands under it, `None` falls back to the 9.0 grammar for a
    /// caller that knows no dialect.
    ///
    /// Distinct from [`Self::octal`], which is only the leading-zero rule:
    /// this also decides whether `0b`/`0o` exist at all (8.5+), whether `0d`
    /// does (9.0+), and whether `_` separators do (9.0+) — so folding `0o17`
    /// for an 8.4 target correctly yields nothing.
    pub numbers: Option<NumberSyntax>,
    /// The active dialect's word-value rules — how a word-shaped list of this
    /// document divides, and whether a braced `\<newline>` folds.  Carried
    /// beside the numeral axis for the same reason: a fold that re-splits a
    /// literal list must split it the way the document's own list parser
    /// does.  Defaults to C Tcl for a caller with no dialect.
    pub word_rules: tcl_syntax::word_rules::WordValueRules,
    /// Explicit engine identity, including registry-less native snapshots.
    pub native_family: Option<tcl_dialect::model::Family>,
    /// Pure mathematical evaluation explicitly requested by a caller. This is
    /// not a Tcl execution proof; execution consumers leave it false and supply
    /// the reached native function-binding query instead.
    pub intrinsic_math: bool,
}

impl FoldPolicy {
    /// Select mathematical intrinsic semantics without claiming Tcl dispatch.
    /// Use only for an explicitly mathematical evaluation API or after another
    /// owner has proved the exact native implementation for every reached call.
    #[must_use]
    pub const fn with_intrinsic_math(mut self) -> Self {
        self.intrinsic_math = true;
        self
    }

    /// Render a floating result only when native precision is immutable.
    /// Numeric folding itself can proceed under mutable precision; string
    /// materialisation requires this independent interpreter-world proof.
    #[must_use]
    pub fn double_format(self) -> Option<tcl_dialect::DoubleFormat> {
        self.invocation_dialect
            .and_then(tcl_registry::InvocationDialect::double_string_policy)
            .and_then(tcl_dialect::DoubleStringPolicy::constant_format)
    }

    /// The numeral grammar this policy reads operands under: the dialect's,
    /// or for a caller that names none, the one its leading-zero rule
    /// implies (the 8.x octal rule, 8.5's grammar; anything else, 9.0's).
    #[must_use]
    pub fn numbers_or_inferred(self) -> NumberSyntax {
        self.numbers.unwrap_or(if self.octal == Some(true) {
            NumberSyntax::Tcl85
        } else {
            NumberSyntax::default()
        })
    }

    /// How the target spells a number it computes: under
    /// [`Self::numbers_or_inferred`], for the release the dialect runs.
    #[must_use]
    pub fn spelling(self) -> NumberSpelling {
        NumberSpelling::of(
            self.numbers_or_inferred(),
            self.dialect
                .and_then(tcl_dialect::DialectProfile::runtime_version),
        )
    }

    /// The policy for an explicit octal rule with no known dialect — the
    /// iRules word operators are declined.
    #[must_use]
    pub const fn from_octal(octal: Option<bool>) -> Self {
        Self {
            invocation_dialect: None,
            arithmetic: None,
            dialect: None,
            native_family: None,
            octal,
            f5_predicates: None,
            expression_parser: None,
            characters: None,
            numbers: None,
            word_rules: tcl_syntax::word_rules::WordValueRules::TCL,
            intrinsic_math: false,
        }
    }

    /// The policy for an octal rule plus a resolved profile. Name parsing and
    /// alias handling happen before this point, so every fact comes from one
    /// canonical profile.
    #[must_use]
    pub fn for_profile(
        octal: Option<bool>,
        profile: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Self {
        Self {
            invocation_dialect: profile.map(tcl_registry::InvocationDialect::of_profile),
            arithmetic: profile.and_then(|profile| {
                tcl_registry::InvocationDialect::of_profile(profile).arithmetic()
            }),
            octal,
            dialect: profile,
            native_family: profile
                .and_then(|profile| tcl_registry::InvocationDialect::of_profile(profile).family()),
            f5_predicates: profile
                .is_some_and(tcl_dialect::DialectProfile::is_irules)
                .then_some(tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider::F5Trunk),
            expression_parser: None,
            characters: profile.and_then(tcl_dialect::DialectProfile::character_model),
            numbers: profile.map(|p| NumberSyntax::of_profile(Some(p))),
            word_rules: tcl_syntax::word_rules::WordValueRules::of_profile(profile),
            intrinsic_math: false,
        }
    }

    /// Complete parser axes expected by a native expression preparation proof.
    /// Missing engine and source profile remain unknown; display names and
    /// mathematical intrinsic mode do not establish executable preparation.
    #[must_use]
    pub fn preparation_context(self) -> Option<tcl_syntax::expr::parser::ExprParseContext> {
        self.invocation_dialect
            .map(|dialect| {
                dialect.expression_parse_context_with(self.expression_parser, self.dialect)
            })
            .or_else(|| {
                self.dialect
                    .map(tcl_syntax::expr::parser::ExprParseContext::for_profile)
            })
    }

    /// Select the logical expression parser the source entry installed.
    #[must_use]
    pub const fn with_expression_parser(
        mut self,
        provider: Option<tcl_registry::invocation_words::LogicalExpressionParseProvider>,
    ) -> Self {
        self.expression_parser = provider;
        self
    }

    /// Preserve native execution policy independently of the catalogue's
    /// authoring expression profile, applying only the retained lexer overlay.
    #[must_use]
    pub fn for_retained_entry(
        registry: &tcl_registry::CommandRegistry,
        invocation: Option<tcl_registry::InvocationDialect>,
        config: &tcl_lexer::LexerConfig,
    ) -> Self {
        let policy = Self::from_registry(registry);
        let Some(mut actual) = invocation else {
            return policy;
        };
        actual.lexer_grammar = config.grammar_over(actual.lexer_grammar);
        actual.word_values =
            tcl_syntax::word_rules::WordValueRules::from_grammar(&actual.lexer_grammar);
        policy.with_invocation_dialect(actual)
    }

    /// Retain the exact invocation's engine and grammar instead of inferring
    /// them from a catalogue profile which may be absent or unrelated.
    #[must_use]
    pub fn with_invocation_dialect(mut self, dialect: tcl_registry::InvocationDialect) -> Self {
        self.invocation_dialect = Some(dialect);
        self.arithmetic = dialect.arithmetic();
        self.numbers = Some(dialect.numbers);
        self.octal = Some(dialect.numbers.leading_zero_is_octal());
        self.word_rules = dialect.word_values;
        self.native_family = dialect.family();
        self.characters = dialect.characters;
        if self.native_family == Some(tcl_dialect::model::Family::Jim) {
            self.characters = None;
            self.f5_predicates = None;
        }
        self
    }

    /// The policy a registry's own dialect profile implies — both facts from
    /// the one source of truth, for the pipeline entry points that hold a
    /// registry rather than a dialect string.
    #[must_use]
    pub fn from_registry(registry: &tcl_registry::CommandRegistry) -> Self {
        Self {
            invocation_dialect: registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile),
            arithmetic: registry.profile().and_then(|profile| {
                tcl_registry::InvocationDialect::of_profile(profile).arithmetic()
            }),
            octal: registry.octal_fold_policy(),
            dialect: registry.profile(),
            native_family: registry
                .profile()
                .and_then(|profile| tcl_registry::InvocationDialect::of_profile(profile).family()),
            f5_predicates: registry
                .profile()
                .is_some_and(tcl_dialect::DialectProfile::is_irules)
                .then_some(tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider::F5Trunk),
            expression_parser: None,
            characters: registry.character_model(),
            numbers: Some(registry.numbers()),
            word_rules: tcl_syntax::word_rules::WordValueRules::of_profile(registry.profile()),
            intrinsic_math: false,
        }
    }
}

/// Whether `profile`'s target widens an integer past a wide and reads an
/// infinity as a value: from 8.5, and for a caller that names no dialect
/// (read as 9.0). An 8.4 runtime computes something else — `1 << 70` wraps
/// to 0, `1e308 * 10` raises `floating-point value too large to represent`
/// (tclsh 8.4) — and a profile whose runtime names no release cannot say
/// which, so under either no such operand or result folds.
pub(super) fn widens_past_a_wide(profile: Option<&tcl_dialect::DialectProfile>) -> bool {
    profile.is_none_or(|profile| {
        profile
            .runtime_base
            .is_some_and(|release| release >= tcl_dialect::TclVersion::V8_5)
    })
}

/// Whether the dialect's *runtime* reads a bare leading-zero integer as
/// octal, from the dialect profile's runtime base: `Some(true)` for the 8.x
/// runtimes (the F5 and EDA shells included), `Some(false)` for 9.x
/// runtimes (TIP 114/472 dropped the rule — `bpf` embeds Tcl 9.0, D7), and
/// `None` — abstain rather than guess — for a profile with no Tcl runtime
/// (`f5-bigip`) or an unknown dialect string (the permissive fallback,
/// design doc §11.1).
#[must_use]
pub fn leading_zero_is_octal(profile: &tcl_dialect::DialectProfile) -> Option<bool> {
    profile.leading_zero_is_octal.as_bool()
}

/// The newest `expr` math-function release available in `dialect`, or `None`
/// when the dialect has no expr-grammar base (don't restrict).
///
/// The dialect-name-keyed form of
/// [`tcl_registry::mathfunc::expr_grammar_ceiling`], which owns the mapping —
/// the registry is where mathfunc facts live, so the const-folder, the
/// availability diagnostic, and the LSP's hover/completion all read one
/// table.
#[must_use]
pub fn math_func_ceiling_for_dialect(
    dialect: &'static tcl_dialect::DialectProfile,
) -> Option<tcl_syntax::expr::mathfunc::MathFuncSince> {
    tcl_registry::mathfunc::expr_grammar_ceiling(dialect)
}

/// The newest math function a fold under `profile` may call: the profile's
/// own ceiling, or 8.4's set for a profile that names no release, because
/// only a function every modelled release has answers alike on all of them
/// (tclsh 8.4 raises `unknown math function "min"`). Unlike
/// [`math_func_ceiling_for_dialect`], which leaves such a profile unbounded
/// so the availability diagnostic never flags a function one of its
/// releases has.
#[must_use]
pub(crate) fn fold_math_ceiling(
    profile: &'static tcl_dialect::DialectProfile,
) -> tcl_syntax::expr::mathfunc::MathFuncSince {
    math_func_ceiling_for_dialect(profile)
        .unwrap_or(tcl_syntax::expr::mathfunc::MathFuncSince::V8_4)
}

/// Whether `name` is a genuine built-in `expr` math function (`sin`, `max`,
/// …) available under `dialect` — a real name in
/// [`tcl_syntax::expr::mathfunc`] whose introducing release is at or before
/// [`math_func_ceiling_for_dialect`]'s ceiling for this dialect.  Free
/// function (rather than an `Analyser` method) so both the W123
/// unresolved-command check and the cross-namespace invocation resettlement
/// (`finalise_invocation_resolutions`, which runs after `self.result` is
/// borrowed mutably and so cannot call back through `&self`) share one
/// answer without either duplicating the other's logic.  Delegates to
/// [`tcl_registry::mathfunc::available_in_expr`].
#[must_use]
pub fn is_known_mathfunc_in_dialect(
    name: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> bool {
    tcl_registry::mathfunc::available_in_expr(name, dialect)
}

/// Whether `dialect` exposes math functions as literal `::tcl::mathfunc::*`
/// **commands** — TIP 232's own wrapper mechanism, landed in 8.5 alongside
/// `bool`/`entier`/`isqrt`/`min`/`max` (see
/// [`tcl_syntax::expr::mathfunc::MathFuncSince::V8_5`]'s doc comment). This
/// is a coarser, single fact than [`is_known_mathfunc_in_dialect`]: it does
/// not vary per function name, because every wrapper command — even one
/// backing an 8.4-vintage function like `sin` — only exists from 8.5
/// onward. An 8.4-based dialect supports `expr {sin(1)}` (the internal
/// expr-grammar dispatch predates TIP 232) but not a bareword
/// `::tcl::mathfunc::sin 1` call (the command itself does not exist there) —
/// [`crate::analyser::diagnostics::unresolved`]'s W123 check uses this to
/// keep an *ordinary* call that happens to resolve to a `tcl::mathfunc`-
/// shaped qualified name from being waved through by the `expr`
/// function-call shortcut, which only reflects the first, narrower fact.
/// Delegates to [`tcl_registry::mathfunc::command_wrappers_available`].
#[must_use]
pub fn mathfunc_command_wrappers_available_in_dialect(
    dialect: &'static tcl_dialect::DialectProfile,
) -> bool {
    tcl_registry::mathfunc::command_wrappers_available(dialect)
}
