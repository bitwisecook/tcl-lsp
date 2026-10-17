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

//! Which `package ifneeded` declarations in a `pkgIndex.tcl` a given Tcl
//! release actually runs.
//!
//! # Why a `pkgIndex.tcl` needs control flow at all
//!
//! `tclPkgUnknown` sources every `pkgIndex.tcl` on `$auto_path` in a frame
//! where `$dir` names the package directory.  Real index files are therefore
//! ordinary scripts, and the ones shipped with tcllib, Tk, and every TEA-built
//! C extension use two guard idioms to keep a package off the wrong Tcl
//! release:
//!
//! **Early return.**  Everything below the guard is skipped:
//!
//! ```tcl
//! if {![package vsatisfies [package provide Tcl] 8.5 9]} {return}
//! package ifneeded log 1.5 [list source [file join $dir log.tcl]]
//! ```
//!
//! That is the head of 95 of tcllib 2.0's own `pkgIndex.tcl` files.  Reading
//! the `package ifneeded` without the guard claims `log` is available on Tcl
//! 8.4, where `package require log` really fails.
//!
//! **Branch selection.**  The declaration lives *inside* a branch:
//!
//! ```tcl
//! if {[package vsatisfies [package provide Tcl] 9.0-]} {
//!     package ifneeded tclopt 0.4 [list source [file join $dir tclopt.tcl]]
//! } else {
//!     package ifneeded tclopt 0.4 [list source [file join $dir tclopt.tcl]]
//! }
//! ```
//!
//! That is the standard TEA shape for picking a Tcl-8 or Tcl-9 shared
//! library.  Reading only *top-level* commands misses it entirely and the
//! package looks nonexistent, so its commands draw a false "unknown command"
//! — the same mechanism, the opposite error.
//!
//! # What this module models
//!
//! [`scan`] walks a `pkgIndex.tcl` and reports each `package ifneeded` it
//! finds together with the [`Condition`]s that must hold for the interpreter
//! to reach it.  Conditions stay **symbolic**: the scan runs once, when the
//! `auto_path` is indexed, and each consumer evaluates the result against
//! whatever Tcl release *it* targets ([`Conditions::availability`]).
//!
//! Modelled:
//!
//! * any command the registry marks [`Traits::TERMINATES_BLOCK`] (`return`,
//!   `error`, `throw`, `exit`) ends the script — `tclPkgUnknown` sources the
//!   index inside a `catch`, so a raise stops registration just as a `return`
//!   does;
//! * `if` / `elseif` / `else` chains, **symbolically**: each clause's guard
//!   becomes a [`Condition`], the clause structure comes from the registry's
//!   own `if` grammar ([`ArgRole::Expr`] / [`ArgRole::Body`]), and a branch
//!   that terminates gates everything after the chain under the negated guard.
//!   That last step is what makes the `if {GUARD} {return}` index head work,
//!   and it is why only `if` may go through [`ScriptWalker::walk_if`]: `while` / `for` /
//!   `foreach` have `Expr` and `Body` arguments too, so reading one as a clause
//!   chain would treat `for {set i 0} {$i < $n} {incr i} {…}` as "if `$i < $n`
//!   then `incr i`, else `…`" and gate the loop body behind a *negated* guard —
//!   a false [`Condition::Impossible`] for anything declared in it.  The
//!   load-bearing distinction is not "loops are not modelled" but the clause
//!   grammar's [`ClauseSelection`]: `if`'s chain selects the first clause
//!   whose condition holds, while a loop's grammar runs every clause it has
//!   (a pack command whose roles come from a resolver is still offered to the
//!   chain walk, as before).
//! * conditions that are a constant boolean (`1`, `0`, `true`, `no`, …) or a
//!   `package vsatisfies [package provide Tcl] REQ …` test, optionally negated
//!   with `!`.
//! * every **other** body-taking command's script regions ([`ArgRole::Body`]
//!   and `apply`'s [`ArgRole::LambdaLiteral`]) — visited, but under
//!   [`Condition::Undecidable`], since this scan models neither whether nor how
//!   often such a body runs.  Visiting them is not optional: Tcl 9 declares its
//!   own core packages from inside `apply {{dir} { … foreach … }} $dir`.
//!
//! Deliberately **not** modelled — each yields [`Condition::Undecidable`], so
//! the registration is reported as merely *conditional* rather than guessed
//! either way:
//!
//! * any other expression (`$::tcl_platform(platform) eq "windows"`,
//!   `[file exists …]`, arithmetic, `&&` / `||` of two guards);
//! * which iterations of a loop run, or whether a `catch` / `eval` /
//!   `namespace eval` / `apply` body runs at all — the body is walked, but
//!   everything found in it is conditional, and a `return` inside one is
//!   treated as "may terminate", never "does terminate";
//! * a guard whose two branches disagree about terminating in a way that
//!   cannot be written as one negated condition;
//! * a declaration whose *name* or *version* word is a substitution — Tcl 9's
//!   own index writes `package ifneeded $package $version …` inside a
//!   `foreach` over a literal list, and this scan reports the declaration but
//!   does not evaluate the loop, so [`super::parse_pkg_index`] still cannot
//!   name it.
//!
//! Nothing here is a *decision* about diagnostics — it reports availability,
//! and the caller decides what to do with `Conditional`.

use tcl_dialect::{TclVersion, Ternary};
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span, Token, TokenType};
use tcl_registry::{ClauseSelection, CommandRegistry, Traits, arg_role::ArgRole};
use tcl_syntax::naming::{canonical_written_command, unroot_rooted_key};

use super::{walk_command_words_with_config, word_raw};

mod original;

/// Cap on `if`-body nesting this scan descends through.  A `pkgIndex.tcl` is
/// machine-generated or hand-written boilerplate; nothing real nests deeply,
/// and the limit keeps a pathological file from exhausting the native stack
/// while the workspace indexer walks it.  See
/// `docs/design/compiler/recursive-descent-depth-limits.md`.
const MAX_GUARD_NESTING_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(32);

/// A target-release relation issued by the genuine source-query owner.
/// The caller's target remains hypothetical; no physical Tcl provision is asserted.
#[derive(Debug, Clone)]
pub struct SourceTargetCondition {
    receipt: std::sync::Arc<
        tcl_compiler::registry_invocation::source_structure::OriginalPackageIndexTargetGuard,
    >,
    expect: bool,
}
impl PartialEq for SourceTargetCondition {
    fn eq(&self, other: &Self) -> bool {
        self.expect == other.expect && std::sync::Arc::ptr_eq(&self.receipt, &other.receipt)
    }
}
impl Eq for SourceTargetCondition {}
#[derive(Debug, Clone)]
struct SourceAdvertisement(
    std::sync::Arc<
        tcl_compiler::registry_invocation::source_structure::OriginalPackageIndexAdvertisement,
    >,
);
impl PartialEq for SourceAdvertisement {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SourceAdvertisement {}

/// One condition that must hold for a `package ifneeded` to be reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// Authentic selected source-query dependency for the explicit target survey.
    SourceTarget(SourceTargetCondition),
    /// `package vsatisfies [package provide Tcl] REQ …` must evaluate to
    /// `expect`.  `requirements` are the requirement words as written
    /// (`"8.5"`, `"9-"`, `"8.5-9.0"`).
    TclSatisfies {
        /// The requirement words; the test passes when the running release
        /// satisfies **any** of them, as `package vsatisfies` does.
        requirements: Vec<String>,
        /// The value the test must take for the registration to be reached.
        expect: bool,
    },
    /// The guard is real but this scan cannot evaluate it.
    Undecidable,
    /// The registration is unreachable on every Tcl release — it sits after
    /// an unconditional terminator, or inside a constant-false branch.
    Impossible,
}

impl Condition {
    /// Evaluate this condition for a target release, or [`Ternary::Inert`]
    /// when it cannot be decided (including when the target release itself is
    /// unknown — an unversioned `tcl` or a non-Tcl dialect).
    #[must_use]
    fn holds(&self, target: Option<TclVersion>) -> Ternary {
        match self {
            Self::SourceTarget(condition) => {
                let Some(target) = target else {
                    return Ternary::Inert;
                };
                if !condition.receipt.subject_is_interpreter_release_in(target) {
                    return Ternary::Inert;
                }
                match target.satisfies_any_ternary(condition.receipt.requirements()) {
                    Ternary::Yes => Ternary::from(condition.expect),
                    Ternary::No => Ternary::from(!condition.expect),
                    Ternary::Inert => Ternary::Inert,
                }
            }
            Self::TclSatisfies {
                requirements,
                expect,
            } => match target {
                // A requirement naming a patch level (`9.0.1`, `9.0.1-`)
                // cannot be evaluated against the two-component release model
                // and comes back `Inert` — the one direction that would
                // otherwise make a real 9.0.4 look like it fails its own
                // guard, and so fire a false W123 on the package's commands.
                Some(v) => match v.satisfies_any_ternary(requirements) {
                    Ternary::Yes => Ternary::from(*expect),
                    Ternary::No => Ternary::from(!*expect),
                    Ternary::Inert => Ternary::Inert,
                },
                None => Ternary::Inert,
            },
            Self::Undecidable => Ternary::Inert,
            Self::Impossible => Ternary::No,
        }
    }

    /// This condition with its sense flipped, or `None` when the negation is
    /// not expressible as a single condition.
    #[must_use]
    fn negated(&self) -> Option<Self> {
        match self {
            Self::SourceTarget(condition) => Some(Self::SourceTarget(SourceTargetCondition {
                receipt: std::sync::Arc::clone(&condition.receipt),
                expect: !condition.expect,
            })),
            Self::TclSatisfies {
                requirements,
                expect,
            } => Some(Self::TclSatisfies {
                requirements: requirements.clone(),
                expect: !*expect,
            }),
            // "Cannot decide" negates to "cannot decide".
            Self::Undecidable => Some(Self::Undecidable),
            // "Never" negates to "always" — no condition at all.
            Self::Impossible => None,
        }
    }
}

/// How certainly a `package ifneeded` runs under a given Tcl release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Every guard between the top of the index and this declaration was
    /// evaluated and lets it through.
    Available,
    /// At least one guard could not be decided.  The declaration may or may
    /// not run; nothing here says which.
    Conditional,
    /// A guard was evaluated and definitely skips this declaration.
    Unavailable,
}

/// The conjunction of conditions guarding one `package ifneeded`.
///
/// Empty means "reached unconditionally" — the common, unguarded index file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Conditions(Vec<Condition>, Vec<SourceAdvertisement>);

impl Conditions {
    /// Whether a Tcl release running this index reaches the declaration.
    ///
    /// One definitely-false condition makes it [`Availability::Unavailable`],
    /// whatever the others say; otherwise one undecidable condition makes it
    /// [`Availability::Conditional`].
    #[must_use]
    pub fn availability(&self, target: Option<TclVersion>) -> Availability {
        let mut undecided = false;
        for condition in &self.0 {
            match condition.holds(target) {
                Ternary::No => return Availability::Unavailable,
                Ternary::Inert => undecided = true,
                Ternary::Yes => {}
            }
        }
        if undecided {
            Availability::Conditional
        } else {
            Availability::Available
        }
    }

    /// Whether any guard at all stands in front of the declaration.
    #[must_use]
    pub fn is_unconditional(&self) -> bool {
        self.0.is_empty()
    }

    fn with(&self, extra: Condition) -> Self {
        let mut out = self.0.clone();
        out.push(extra);
        Self(out, self.1.clone())
    }
    fn with_advertisement(
        &self,
        receipt: std::sync::Arc<
            tcl_compiler::registry_invocation::source_structure::OriginalPackageIndexAdvertisement,
        >,
    ) -> Self {
        let mut out = self.clone();
        out.1.push(SourceAdvertisement(receipt));
        out
    }
    /// Whether this condition set retains a selected source-advertisement premise.
    /// This grants no physical registration or complete inventory.
    #[must_use]
    pub fn has_source_advertisement_premise(&self) -> bool {
        !self.1.is_empty()
    }
}

/// One `package ifneeded` declaration the scan reached, in the script text it
/// was written in.
pub struct Reached<'t> {
    /// The script the declaration's tokens index into — the whole index file
    /// for a top-level declaration, or an `if` body's inner text.
    pub text: &'t str,
    /// The declaration's words, `package` first.
    pub words: &'t [Vec<Token>],
    /// What must hold for the interpreter to run it.
    pub conditions: Conditions,
    /// Actual complete original words in their retained source image. Entered
    /// decoded bodies keep their NativeValue channel; this field alone supplies
    /// no Document coordinate map, current-input or package-identity grant.
    pub original_words: Option<Vec<NativeWord>>,
}

/// A genuine standalone selection retained beside its unchanged callback data.
/// Original-analysis traversal has an independent invocation/schema purpose.
pub(super) struct SelectedSurveyRegistration<'t> {
    pub(super) reached: Reached<'t>,
    pub(super) registration: super::registration::Registration,
}

/// Walk `content`, calling `visit` once per `package ifneeded` declaration
/// reachable on *some* Tcl release, with the conditions guarding it.
///
/// A declaration under a constant-false guard is still visited, carrying
/// [`Condition::Impossible`]: reporting it lets a consumer distinguish "this
/// index does not mention the package" from "this index mentions it but can
/// never register it".
pub fn scan(content: &str, registry: &CommandRegistry, visit: &mut dyn FnMut(Reached<'_>)) {
    scan_with_config(content, registry, LexerConfig::default(), visit);
}

/// Walk index content with the loader's complete grammar and original image.
/// Decoded or computed nested scripts retain advisory conditions, but cannot
/// manufacture original word provenance. This authored index convention maps
/// written heads through shared colon geometry before selecting Registry data;
/// it supplies no C/Jim command lookup or loader authority.
pub fn scan_with_config(
    content: &str,
    registry: &CommandRegistry,
    config: LexerConfig,
    visit: &mut dyn FnMut(Reached<'_>),
) {
    scan_selected_registrations(content, registry, config, &mut |selected| {
        visit(selected.reached);
    });
}

/// Retain the actual selected original source advertisement and callback geometry.
pub(super) fn scan_selected_registrations(
    content: &str,
    registry: &CommandRegistry,
    config: LexerConfig,
    visit: &mut dyn FnMut(SelectedSurveyRegistration<'_>),
) {
    let _ = original::scan_survey(content, registry, config, &mut |reached| {
        let Some(premise) = reached
            .conditions
            .1
            .last()
            .map(|receipt| std::sync::Arc::clone(&receipt.0))
        else {
            return;
        };
        let Some(original) = reached.original_words.as_ref() else {
            return;
        };
        let Some(first) = original.first() else {
            return;
        };
        let Some(actual) = first.tokens().first() else {
            return;
        };
        let Some(local) = reached.words.first().and_then(|word| word.first()) else {
            return;
        };
        let Some(base) = actual.span.start().checked_sub(local.span.start()) else {
            return;
        };
        let Some(end) = u32::try_from(reached.text.len())
            .ok()
            .and_then(|len| base.checked_add(len))
        else {
            return;
        };
        let Some(registration) = super::registration::select_original_advertisement(
            reached.text,
            reached.words,
            first.image(),
            Span::new(base, end),
            registry,
            config,
            premise,
        ) else {
            return;
        };
        visit(SelectedSurveyRegistration {
            reached,
            registration,
        });
    });
}

/// Coverage of the existing original-source package survey, independently of
/// the conditions attached to positive registration discoveries. No result
/// proves runtime execution, physical provision or command-world absence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalSourceSurveyDisposition {
    /// All modelled source phases were covered and control may continue.
    Complete(Conditions),
    /// Covered control unconditionally stops before the end of this script.
    Stopped,
    /// Positive discoveries remain useful, but omitted phases prevent closure.
    Incomplete(Conditions),
}

/// Source advice under a current retained analysis and its exact loader axes.
/// Unknown source applicability never establishes an unconditional path.
pub(super) fn scan_with_analysis(
    content: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    registry: &CommandRegistry,
    config: LexerConfig,
    visit: &mut dyn FnMut(Reached<'_>),
) -> Option<()> {
    original::scan(content, analysis, registry, config, visit)
}

#[derive(Clone)]
struct OriginalScript {
    image: SourceImage,
    span: Span,
    config: LexerConfig,
}

impl OriginalScript {
    fn words(
        &self,
        words: &[Vec<Token>],
        plan: &tcl_lexer::NativeScriptWordsPlan,
    ) -> Option<Vec<NativeWord>> {
        let first = words.first()?.first()?;
        let start = self.span.start().checked_add(first.span.start())?;
        let command = plan.commands.iter().find(|command| {
            command
                .words
                .first()
                .and_then(|word| word.tokens().first())
                .is_some_and(|token| token.span.start() == start)
        })?;
        if command.words.len() != words.len()
            || command.words.iter().zip(words).any(|(original, local)| {
                original.tokens().len() != local.len()
                    || original.tokens().iter().zip(local).any(|(a, b)| {
                        a.kind != b.kind
                            || a.content_offset != b.content_offset
                            || a.span.start() != self.span.start().saturating_add(b.span.start())
                            || a.span.end() != self.span.start().saturating_add(b.span.end())
                    })
            })
        {
            return None;
        }
        Some(command.words.clone())
    }

    fn child(&self, word: &NativeWord, body: &str, registry: &CommandRegistry) -> Option<Self> {
        let policy = registry.profile().and_then(|profile| {
            tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy()
        })?;
        let key =
            tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                word,
                tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
                policy,
            )?;
        let span = word.content_span().ok()?;
        if key.bytes() != body.as_bytes()
            || self.image.bytes().get(span.as_range())? != body.as_bytes()
        {
            return None;
        }
        Some(Self {
            image: self.image.clone(),
            span,
            config: self.config,
        })
    }
}

struct ScriptWalker<'r, 'v> {
    registry: &'r CommandRegistry,
    config: LexerConfig,
    visit: &'v mut dyn FnMut(SelectedSurveyRegistration<'_>),
}

impl ScriptWalker<'_, '_> {
    /// Walk one script level in order.
    ///
    /// Returns the conditions under which control reaches the *end* of the script,
    /// or `None` when it never does (an unconditional terminator).
    fn walk_script(
        &mut self,
        text: &str,
        reached: &Conditions,
        depth: u32,
        original: Option<&OriginalScript>,
    ) -> Option<Conditions> {
        let registry = self.registry;
        let config = self.config;
        if MAX_GUARD_NESTING_DEPTH.exceeded(depth) {
            return Some(reached.with(Condition::Undecidable));
        }
        let commands = walk_command_words_with_config(text, config);
        let registration_plan = u32::try_from(text.len()).ok().and_then(|end| {
            tcl_lexer::native_script_words_in(
                SourceImage::document(text),
                Span::new(0, end),
                config,
            )
            .ok()
        });
        let plan = original.and_then(|original| {
            tcl_lexer::native_script_words_in(original.image.clone(), original.span, config).ok()
        });
        let mut reached = reached.clone();
        for words in &commands {
            if words.is_empty() {
                continue;
            }
            let original_words = original
                .zip(plan.as_ref())
                .and_then(|(original, plan)| original.words(words, plan));
            if let Some(registration) = registration_plan.as_ref().and_then(|plan| {
                super::registration::select_in_plan(text, words, registry, config, plan)
            }) {
                (self.visit)(SelectedSurveyRegistration {
                    reached: Reached {
                        text,
                        words,
                        conditions: reached.clone(),
                        original_words,
                    },
                    registration,
                });
                continue;
            }
            let written_head = canonical_written_command(word_raw(text, &words[0]));
            let head = unroot_rooted_key(&written_head).unwrap_or(&written_head);
            let Some(spec) = registry.get(head) else {
                continue;
            };
            if spec.traits.contains(Traits::TERMINATES_BLOCK) {
                return None;
            }
            let first_match_chain = spec
                .clause_grammar
                .is_some_and(|grammar| grammar.selection == ClauseSelection::FirstMatch);
            if spec.traits.contains(Traits::HAS_BOOLEAN_COND)
                && (first_match_chain || spec.arg_role_resolver.is_some())
            {
                match self.walk_if(text, words, head, &reached, depth, original) {
                    AfterIf::Stops => return None,
                    AfterIf::Continues(after) => {
                        reached = after;
                        continue;
                    }
                    AfterIf::NotAChain => {}
                }
            }
            // Any *other* control-flow command (a loop, `switch`, `catch`, `try`)
            // may or may not run its body, and this scan does not model how many
            // times or under what condition.  If a body could terminate the
            // script, everything below becomes undecidable rather than being
            // claimed unconditional.  A body that cannot terminate — and the
            // bodies of non-control-flow commands such as `proc`, whose `return`
            // belongs to the procedure, not to this script — changes nothing.
            if spec.traits.contains(Traits::CONTROL_FLOW)
                && command_body_may_terminate(text, words, head, registry, depth, config)
            {
                reached = reached.with(Condition::Undecidable);
            }
            // Declarations *inside* that body still have to be found.  Every
            // body-taking command's script regions are visited under
            // [`Condition::Undecidable`]: this scan cannot say whether — or how
            // many times — the body runs, and "conditional" is the safe direction
            // (a conditional registration counts as loadable downstream, so a
            // package whose commands really are there draws no false W123).
            //
            // Not looking at all was the defect: Tcl 9 ships its own core-package
            // index as `apply {{dir} { … foreach … { package ifneeded … } }} $dir`
            // (`library/pkgIndex.tcl` in the zipfs `tcl_library`), so a tcl9.0
            // workspace saw http / msgcat / tcltest / platform / cookiejar declare
            // nothing whatsoever.
            for (body, body_index) in body_scripts(text, words, head, registry, config) {
                let child = original
                    .zip(original_words.as_ref())
                    .and_then(|(original, words)| {
                        original.child(words.get(body_index + 1)?, &body, registry)
                    });
                self.walk_script(
                    &body,
                    &reached.with(Condition::Undecidable),
                    depth + 1,
                    child.as_ref(),
                );
            }
        }
        Some(reached)
    }

    /// Walk an `if` chain: visit the declarations in every branch, and report what
    /// happens to control flow after it.
    fn walk_if(
        &mut self,
        text: &str,
        words: &[Vec<Token>],
        head: &str,
        reached: &Conditions,
        depth: u32,
        original: Option<&OriginalScript>,
    ) -> AfterIf {
        let registry = self.registry;
        let config = self.config;
        let args: Vec<&str> = words[1..].iter().map(|w| word_raw(text, w)).collect();
        let Some(clauses) = clause_chain(head, &args, registry) else {
            return AfterIf::NotAChain;
        };

        // Conditions accumulated from every earlier clause having tested false —
        // what must hold for this clause to be *considered* at all.
        let mut earlier_false = Conditions::default();
        // One entry per branch that stops control continuing past the chain,
        // carrying the single condition that selects it where there is one.
        let mut terminating: Vec<Option<Condition>> = Vec::new();
        let mut has_else = false;
        let mut definitely_taken_terminates = false;
        let mut unavailable_body = false;

        for clause in &clauses {
            let branch_conditions = match clause.expr {
                // The `else` branch runs exactly when every earlier test failed.
                None => {
                    has_else = true;
                    earlier_false.clone()
                }
                Some(expr_index) => {
                    let guard = guard_expr(text, &words[1..][expr_index], config);
                    let mut conditions = earlier_false.clone();
                    if let Some(condition) = guard.condition(true) {
                        conditions = conditions.with(condition);
                    }
                    if let Some(condition) = guard.condition(false) {
                        earlier_false = earlier_false.with(condition);
                    }
                    conditions
                }
            };
            let Some(body_text) = script_of(text, &words[1..][clause.body], config) else {
                unavailable_body = true;
                continue;
            };
            let mut branch_reached = reached.clone();
            for condition in &branch_conditions.0 {
                branch_reached = branch_reached.with(condition.clone());
            }
            let child = original.and_then(|original| {
                let plan = tcl_lexer::native_script_words_in(
                    original.image.clone(),
                    original.span,
                    config,
                )
                .ok()?;
                let native = original.words(words, &plan)?;
                original.child(native.get(clause.body + 1)?, &body_text, registry)
            });
            if self
                .walk_script(&body_text, &branch_reached, depth + 1, child.as_ref())
                .is_none()
            {
                if branch_conditions.is_unconditional() {
                    definitely_taken_terminates = true;
                }
                terminating.push(match branch_conditions.0.as_slice() {
                    [only] => Some(only.clone()),
                    _ => None,
                });
            } else if body_may_terminate(&body_text, registry, depth + 1, config) {
                terminating.push(Some(Condition::Undecidable));
            }
        }

        if definitely_taken_terminates
            || (!unavailable_body && has_else && terminating.len() == clauses.len())
        {
            return AfterIf::Stops;
        }
        if unavailable_body {
            return AfterIf::Continues(reached.with(Condition::Undecidable));
        }
        AfterIf::Continues(match terminating.as_slice() {
            // Nothing in the chain stops control continuing.
            [] => reached.clone(),
            // Exactly one branch terminates, under one negatable condition: what
            // follows is guarded by that condition's negation.  This is the shape
            // every real `if {GUARD} {return}` index-file head takes.
            [Some(condition)] => match condition.negated() {
                Some(negated) => reached.with(negated),
                None => reached.clone(),
            },
            _ => reached.with(Condition::Undecidable),
        })
    }
}

/// The script text of every body-role region of one command.
///
/// Both script-bearing argument roles, taken from the registry so no command
/// name appears here:
///
/// * [`ArgRole::Body`] — the argument *is* the script (`foreach`'s body,
///   `namespace eval`'s block, `catch`'s script, `eval`'s argument).
/// * [`ArgRole::LambdaLiteral`] — the argument is `apply`'s
///   `{params} {body} ?ns?` **list**, one word containing the script one level
///   down.  Split with the shared
///   [`tcl_compiler::lambda_literal`] splitter rather than re-deriving the
///   shape; the decoded variant is the one documented for consumers that need
///   the script text and report no spans back into it, so a bare or quoted
///   body element's escapes collapse exactly as `apply` collapses them.
///
/// An `if` chain never reaches here — [`ScriptWalker::walk_if`] models its clauses
/// symbolically and the caller continues past it.
fn body_scripts(
    text: &str,
    words: &[Vec<Token>],
    head: &str,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Vec<(String, usize)> {
    let args: Vec<&str> = words[1..].iter().map(|w| word_raw(text, w)).collect();
    let mut out: Vec<(String, usize)> = registry
        .arg_indices_for_role(head, &args, ArgRole::Body)
        .into_iter()
        .filter_map(|i| Some((script_of(text, words[1..].get(i)?, config)?, i)))
        .collect();
    for index in registry.arg_indices_for_role(head, &args, ArgRole::LambdaLiteral) {
        // A statically-splittable lambda is a single braced word; a `$var` /
        // `[cmd]` lambda has nothing to walk and is skipped.
        let Some([token]) = words[1..].get(index).map(Vec::as_slice) else {
            continue;
        };
        if let Some(body) = tcl_compiler::lambda_literal::split_lambda_literal_decoded(text, *token)
            .and_then(|elements| elements.body)
        {
            out.push((body.into_owned(), index));
        }
    }
    out
}

/// Whether any [`ArgRole::Body`] argument of this command contains a
/// terminator.
fn command_body_may_terminate(
    text: &str,
    words: &[Vec<Token>],
    head: &str,
    registry: &CommandRegistry,
    depth: u32,
    config: LexerConfig,
) -> bool {
    let args: Vec<&str> = words[1..].iter().map(|w| word_raw(text, w)).collect();
    registry
        .arg_indices_for_role(head, &args, ArgRole::Body)
        .into_iter()
        .filter_map(|i| words[1..].get(i))
        .any(|word| {
            script_of(text, word, config)
                .is_none_or(|body| body_may_terminate(&body, registry, depth + 1, config))
        })
}

/// Authenticate a lexical word against the complete configured source plan.
fn original_data_word(text: &str, word: &[Token], config: LexerConfig) -> Option<NativeWord> {
    let length = u32::try_from(text.len()).ok()?;
    let plan = tcl_lexer::native_script_words_in_with_purpose(
        SourceImage::document(text),
        Span::new(0, length),
        config,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    if plan.fatal_tail.is_some() {
        return None;
    }
    let mut matching = plan
        .commands
        .iter()
        .flat_map(|command| &command.words)
        .filter(|original| original.tokens() == word);
    let original = matching.next()?;
    matching.next().is_none().then(|| original.clone())
}

/// Decode a static Body or Expr operand once, before reading its script data.
fn script_of(text: &str, word: &[Token], config: LexerConfig) -> Option<String> {
    let original = original_data_word(text, word, config)?;
    String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
        &original,
    )?)
    .ok()
}

/// A whole command substitution contributes its original script, not its result.
fn command_source(text: &str, word: &[Token], config: LexerConfig) -> Option<String> {
    let original = original_data_word(text, word, config)?;
    if original.group().expand || original.group().kind != tcl_lexer::WordKind::Bare {
        return None;
    }
    let arena = original.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    let tcl_lexer::ExecutablePart::Command { body } = part.part else {
        return None;
    };
    if part.span != original.word_span() {
        return None;
    }
    std::str::from_utf8(original.image().bytes().get(body.as_range())?)
        .ok()
        .map(str::to_owned)
}

/// What an `if` chain does to control flow.
enum AfterIf {
    /// The command's words are not a clause chain this can read (a malformed
    /// `if` the interpreter would reject), so the caller treats it as inert.
    NotAChain,
    /// No path out of the chain continues — the whole script ends here.
    Stops,
    /// Control continues past the chain, under these conditions.
    Continues(Conditions),
}

/// Whether `script` contains a terminator anywhere, however deeply guarded.
///
/// Used only to decide that a branch *might* stop the script, so an
/// undecidable guard is recorded rather than none at all.
fn body_may_terminate(
    script: &str,
    registry: &CommandRegistry,
    depth: u32,
    config: LexerConfig,
) -> bool {
    if MAX_GUARD_NESTING_DEPTH.exceeded(depth) {
        return false;
    }
    walk_command_words_with_config(script, config)
        .iter()
        .any(|words| {
            words.first().is_some_and(|first| {
                let written_head = canonical_written_command(word_raw(script, first));
                let head = unroot_rooted_key(&written_head).unwrap_or(&written_head);
                registry
                    .get(head)
                    .is_some_and(|spec| spec.traits.contains(Traits::TERMINATES_BLOCK))
            }) || words.iter().any(|word| {
                let Some(original) = original_data_word(script, word, config) else {
                    return true;
                };
                original.executable_parts().all_parts().any(|part| {
                    let tcl_lexer::ExecutablePart::Command { body } = part.part else {
                        return false;
                    };
                    std::str::from_utf8(&original.image().bytes()[body.as_range()])
                        .ok()
                        .is_none_or(|body| body_may_terminate(body, registry, depth + 1, config))
                }) || (matches!(
                    original.group().kind,
                    tcl_lexer::WordKind::Braced | tcl_lexer::WordKind::Quoted
                ) && script_of(script, word, config)
                    .is_some_and(|inner| body_may_terminate(&inner, registry, depth + 1, config)))
            })
        })
}

/// One clause of an `if` chain: an optional condition word and its body word,
/// as argument indices (0 = the word after the command name).
struct Clause {
    expr: Option<usize>,
    body: usize,
}

/// Pair an `if` invocation's argument words into clauses, using the registry's
/// own grammar for the command rather than re-deriving it here.
///
/// Returns `None` when the shape is not a clause chain (no bodies, or a
/// trailing condition with no body — a malformed `if` the interpreter would
/// reject anyway).
fn clause_chain(head: &str, args: &[&str], registry: &CommandRegistry) -> Option<Vec<Clause>> {
    let exprs = registry.arg_indices_for_role(head, args, ArgRole::Expr);
    let bodies = registry.arg_indices_for_role(head, args, ArgRole::Body);
    if bodies.is_empty() {
        return None;
    }
    let mut clauses = Vec::with_capacity(bodies.len());
    for &body in &bodies {
        // The clause's condition is the last `Expr` word before the body that
        // no earlier clause has claimed.
        let expr = exprs
            .iter()
            .copied()
            .rfind(|&e| e < body && !clauses.iter().any(|c: &Clause| c.expr == Some(e)));
        clauses.push(Clause { expr, body });
    }
    Some(clauses)
}

/// A guard expression this scan understands.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Guard {
    SourceTarget(SourceTargetCondition),
    /// A literal boolean (`1`, `0`, `true`, `off`, …).
    Constant(bool),
    /// `package vsatisfies [package provide Tcl] REQ …`, `negated` when
    /// written under a leading `!`.
    TclSatisfies {
        requirements: Vec<String>,
        negated: bool,
    },
    /// Anything else.
    Unknown,
}

impl Guard {
    /// The condition that must hold for this guard to take the value
    /// `expect`, or `None` when the guard already settles it.
    fn condition(&self, expect: bool) -> Option<Condition> {
        match self {
            Self::SourceTarget(condition) => Some(Condition::SourceTarget(SourceTargetCondition {
                receipt: std::sync::Arc::clone(&condition.receipt),
                expect: expect != condition.receipt.negated(),
            })),
            Self::Constant(value) => (*value != expect).then_some(Condition::Impossible),
            Self::TclSatisfies {
                requirements,
                negated,
            } => Some(Condition::TclSatisfies {
                requirements: requirements.clone(),
                expect: expect != *negated,
            }),
            Self::Unknown => Some(Condition::Undecidable),
        }
    }
}

/// Read a guard word (`if`'s condition) into a [`Guard`].
///
/// A braced or quoted condition — the universal spelling — is read from its
/// inner text; a bare condition is read as written.
fn guard_expr(text: &str, word: &[Token], config: LexerConfig) -> Guard {
    if let Some(inner) = command_source(text, word, config) {
        return parse_vsatisfies(&inner, config);
    }
    script_of(text, word, config).map_or(Guard::Unknown, |value| parse_guard(value.trim(), config))
}

/// Parse a condition's source text.
fn parse_guard(expr: &str, config: LexerConfig) -> Guard {
    parse_guard_mode(expr, config, true)
}
fn constant_guard(expr: &str, config: LexerConfig) -> Guard {
    parse_guard_mode(expr, config, false)
}
fn parse_guard_mode(expr: &str, config: LexerConfig, source_query_compatibility: bool) -> Guard {
    let expr = expr.trim();
    if let Some(rest) = expr.strip_prefix('!') {
        return match parse_guard_mode(rest, config, source_query_compatibility) {
            Guard::Constant(value) => Guard::Constant(!value),
            Guard::TclSatisfies {
                requirements,
                negated,
            } => Guard::TclSatisfies {
                requirements,
                negated: !negated,
            },
            Guard::SourceTarget(_) | Guard::Unknown => Guard::Unknown,
        };
    }
    // A whole-expression `[...]` substitution: unwrap and read the command.
    if source_query_compatibility && let Some(inner) = bracketed_body(expr, config) {
        return parse_vsatisfies(&inner, config);
    }
    // The same acceptor `if` itself uses (`Tcl_GetBooleanFromObj`'s string
    // path), so `1`, `true`, `yes`, `on`, `2`, and `0x0` read exactly as the
    // interpreter reads them.
    // Unanimous across releases: this guard decides *reachability*, so claiming
    // a constant that only holds on some releases would prune a branch that is
    // live on the others. A spelling the releases read differently falls to
    // `Unknown`, which is the abstaining direction here.
    match tcl_syntax::boolean::truthiness_in_every_release(expr) {
        Some(value) => Guard::Constant(value),
        None => Guard::Unknown,
    }
}

/// The inner text of `expr` when it is exactly one `[...]` substitution.
fn bracketed_body(expr: &str, config: LexerConfig) -> Option<String> {
    let commands = walk_command_words_with_config(expr, config);
    let [words] = commands.as_slice() else {
        return None;
    };
    let [word] = words.as_slice() else {
        return None;
    };
    let original = original_data_word(expr, word, config)?;
    if original.word_span() != Span::new(0, u32::try_from(expr.len()).ok()?) {
        return None;
    }
    command_source(expr, word, config)
}

/// Read `package vsatisfies [package provide Tcl] REQ …` — the only
/// non-constant test this scan evaluates.
///
/// The subject must be a `[package provide Tcl]` or `[package require Tcl]`
/// substitution: a `vsatisfies` over some *other* package says nothing about
/// the Tcl release, and guessing there would be worse than abstaining.
fn parse_vsatisfies(script: &str, config: LexerConfig) -> Guard {
    let commands = walk_command_words_with_config(script, config);
    let [words] = commands.as_slice() else {
        return Guard::Unknown;
    };
    if words.len() < 4 {
        return Guard::Unknown;
    }
    let written_head = canonical_written_command(word_raw(script, &words[0]));
    if unroot_rooted_key(&written_head).unwrap_or(&written_head) != "package"
        || word_raw(script, &words[1]) != "vsatisfies"
    {
        return Guard::Unknown;
    }
    if !is_tcl_version_subject(script, &words[2], config) {
        return Guard::Unknown;
    }
    let Ok(length) = u32::try_from(script.len()) else {
        return Guard::Unknown;
    };
    let Ok(plan) = tcl_lexer::native_script_words_in_with_purpose(
        SourceImage::document(script),
        Span::new(0, length),
        config,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    ) else {
        return Guard::Unknown;
    };
    let [command] = plan.commands.as_slice() else {
        return Guard::Unknown;
    };
    if plan.fatal_tail.is_some()
        || command.words.len() != words.len()
        || command
            .words
            .iter()
            .zip(words)
            .any(|(original, tokens)| original.tokens() != tokens.as_slice())
    {
        return Guard::Unknown;
    }
    let Some(requirements) = command.words[3..]
        .iter()
        .map(|word| {
            String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
                word,
            )?)
            .ok()
        })
        .collect::<Option<Vec<_>>>()
    else {
        return Guard::Unknown;
    };
    if requirements.iter().any(|r| !is_requirement_word(r)) {
        return Guard::Unknown;
    }
    Guard::TclSatisfies {
        requirements,
        negated: false,
    }
}

/// Whether `word` is `[package provide Tcl]` / `[package require Tcl]` — the
/// running interpreter's own version.
fn is_tcl_version_subject(script: &str, word: &[Token], config: LexerConfig) -> bool {
    if word.len() != 1 || word[0].kind != TokenType::Cmd {
        return false;
    }
    let Some(inner) = command_source(script, word, config) else {
        return false;
    };
    let commands = walk_command_words_with_config(&inner, config);
    let [words] = commands.as_slice() else {
        return false;
    };
    if words.len() != 3 {
        return false;
    }
    let written_head = canonical_written_command(word_raw(&inner, &words[0]));
    unroot_rooted_key(&written_head).unwrap_or(&written_head) == "package"
        && matches!(word_raw(&inner, &words[1]), "provide" | "require")
        // `Tcl` is matched case-sensitively — `tk` and the lowercase `tcl`
        // alias are other packages.
        && word_raw(&inner, &words[2]) == "Tcl"
}

/// Whether `word` has the shape of a version requirement (`8.5`, `9-`,
/// `8.5-9.0`) rather than a substitution or an option flag.
fn is_requirement_word(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with('$')
        && !word.starts_with('[')
        && !word.starts_with('-')
        && word
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'.' || b == b'-' || b == b'a' || b == b'b')
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod source_survey_owner_tests {
    use super::*;
    #[test]
    fn fixed_source_guard_retains_real_query_and_advertisement_dependencies() {
        let source = "if {[package vsatisfies [package provide Tcl] 9-]} {return}\npackage ifneeded p 1.0 [list source $dir/p.tcl]";
        let registry = CommandRegistry::build_default();
        let mut found = Vec::new();
        scan(source, &registry, &mut |reached| {
            found.push(reached.conditions)
        });
        assert_eq!(found.len(), 1);
        assert!(found[0].has_source_advertisement_premise());
        assert!(
            found[0]
                .0
                .iter()
                .any(|condition| matches!(condition, Condition::SourceTarget(_)))
        );
        assert_eq!(
            found[0].availability(Some(TclVersion::V8_6)),
            Availability::Available
        );
        assert_eq!(
            found[0].availability(Some(TclVersion::V9_0)),
            Availability::Unavailable
        );
    }
    #[test]
    fn known_false_source_body_keeps_unavailable_advertisement_without_entered_claim() {
        let registry = CommandRegistry::build_default();
        let mut found = Vec::new();
        scan(
            "if {0} {package ifneeded hidden 1.0 {source hidden.tcl}}",
            &registry,
            &mut |reached| found.push(reached.conditions),
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].has_source_advertisement_premise());
        assert_eq!(
            found[0].availability(Some(TclVersion::V9_0)),
            Availability::Unavailable
        );
    }
    #[test]
    fn decoded_registration_retains_native_value_original_words() {
        let registry = CommandRegistry::build_default();
        let mut found = Vec::new();
        scan(
            "if 1 \"package ifneeded caf\\u00e9 1.0 {}\"",
            &registry,
            &mut |reached| {
                assert_eq!(
                    reached.original_words.as_ref().unwrap()[0]
                        .image()
                        .channel(),
                    tcl_lexer::SourceChannel::NativeValue
                );
                assert!(reached.conditions.has_source_advertisement_premise());
                found.push(word_raw(reached.text, &reached.words[2]).to_owned());
            },
        );
        assert_eq!(found, vec!["café"]);
    }
    #[test]
    fn opaque_or_package_history_does_not_restore_target_release_model() {
        let registry = CommandRegistry::build_default();
        for prefix in [
            "package provide Tcl 7.0;",
            "package forget Tcl;",
            "mystery;",
        ] {
            let source = format!(
                "{prefix} if {{[package vsatisfies [package provide Tcl] 9-]}} {{return}}; package ifneeded p 1.0 {{}} "
            );
            let mut found = Vec::new();
            scan(&source, &registry, &mut |reached| {
                found.push(reached.conditions)
            });
            assert!(found.iter().all(|conditions| {
                !conditions
                    .0
                    .iter()
                    .any(|condition| matches!(condition, Condition::SourceTarget(_)))
            }));
        }
    }
}

#[cfg(test)]
mod source_advertisement_value_boundary_tests {
    use super::*;
    #[test]
    fn original_projection_preserves_legacy_nul_value_refusal() {
        let registry = CommandRegistry::build_default();
        let mut found = Vec::new();
        scan_selected_registrations(
            "package ifneeded \"bad\\u0000name\" 1.0 {}",
            &registry,
            LexerConfig::default(),
            &mut |selected| {
                assert!(
                    selected
                        .reached
                        .conditions
                        .has_source_advertisement_premise()
                );
                found.push(
                    selected
                        .registration
                        .advertisement(
                            selected.reached.text,
                            selected.reached.words,
                            &registry,
                            LexerConfig::default(),
                        )
                        .is_none(),
                );
            },
        );
        assert_eq!(found, vec![true]);
    }
}

#[cfg(test)]
mod source_clause_selection_tests {
    use super::*;
    fn conditions(source: &str) -> Vec<Conditions> {
        let registry = CommandRegistry::build_default();
        let mut out = Vec::new();
        scan(source, &registry, &mut |reached| {
            out.push(reached.conditions)
        });
        out
    }
    #[test]
    fn loops_do_not_become_first_match_source_guards() {
        let found = conditions("for {} {0} {} {package ifneeded p 1.0 {}}");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].availability(Some(TclVersion::V9_0)),
            Availability::Conditional
        );
        let found = conditions("foreach x {1} {return}; package ifneeded p 1.0 {}");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].availability(Some(TclVersion::V9_0)),
            Availability::Conditional
        );
    }
    #[test]
    fn unreachable_source_body_uncertainty_does_not_retire_outer_guard_path() {
        let found = conditions("if {0} {return}; package ifneeded p 1.0 {}");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].availability(Some(TclVersion::V9_0)),
            Availability::Available
        );
    }
}

#[cfg(test)]
mod source_lambda_namespace_tests {
    use super::*;
    #[test]
    fn logical_lambda_namespace_is_its_own_original_source_owner() {
        let registry = CommandRegistry::build_default();
        let mut found = Vec::new();
        scan(
            "namespace eval ::Caller {proc package args {}; apply {{} {package ifneeded p 1.0 {}}}}",
            &registry,
            &mut |reached| found.push(reached.conditions),
        );
        assert_eq!(found.len(), 1, "default lambda scope is global, not Caller");
        assert_eq!(
            found[0].availability(Some(TclVersion::V9_0)),
            Availability::Conditional
        );
        let mut shadowed = 0;
        scan(
            "namespace eval ::ns {proc package args {}}; apply {{} {package ifneeded hidden 1.0 {}} ::ns}",
            &registry,
            &mut |_| shadowed += 1,
        );
        assert_eq!(
            shadowed, 0,
            "explicit lambda scope retains its known package shadow"
        );
    }
}

#[cfg(test)]
mod source_uncertain_continuation_tests {
    use super::*;
    #[test]
    fn unknown_branch_continuations_do_not_become_definite_all_branch_stop() {
        let registry = CommandRegistry::build_default();
        let mut found = Vec::new();
        scan(
            "if {$condition} {mystery} else {mystery}; package ifneeded after 1.0 {}",
            &registry,
            &mut |reached| found.push(reached.conditions),
        );
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].availability(Some(TclVersion::V9_0)),
            Availability::Conditional
        );
        assert!(found[0].has_source_advertisement_premise());
    }
}
