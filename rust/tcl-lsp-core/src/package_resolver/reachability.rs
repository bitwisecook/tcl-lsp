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
//! 8.4, where `package require log` really fails (issue #1017).
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
//! (issue #923 idx 42) — the same mechanism, the opposite error.
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
//! * `if` / `elseif` / `else` chains, with the clause structure taken from the
//!   registry's own `if` grammar ([`ArgRole::Expr`] / [`ArgRole::Body`]), so
//!   `then` noise words and implicit else-bodies are handled without this
//!   module re-deriving the grammar;
//! * conditions that are a constant boolean (`1`, `0`, `true`, `no`, …) or a
//!   `package vsatisfies [package provide Tcl] REQ …` test, optionally negated
//!   with `!`.
//!
//! Deliberately **not** modelled — each yields [`Condition::Undecidable`], so
//! the registration is reported as merely *conditional* rather than guessed
//! either way:
//!
//! * any other expression (`$::tcl_platform(platform) eq "windows"`,
//!   `[file exists …]`, arithmetic, `&&` / `||` of two guards);
//! * loops (`while`, `for`, `foreach`) — a `return` inside one is treated as
//!   "may terminate", never "does terminate";
//! * `catch`, `uplevel`, `eval`, and any other command that runs a script
//!   argument;
//! * a guard whose two branches disagree about terminating in a way that
//!   cannot be written as one negated condition.
//!
//! Nothing here is a *decision* about diagnostics — it reports availability,
//! and the caller decides what to do with `Conditional`.

use tcl_dialect::{TclVersion, Ternary};
use tcl_lexer::{Token, TokenType};
use tcl_registry::{CommandRegistry, Traits, arg_role::ArgRole};

use super::{walk_command_words, word_raw, word_unwrap};

/// Cap on `if`-body nesting this scan descends through.  A `pkgIndex.tcl` is
/// machine-generated or hand-written boilerplate; nothing real nests deeply,
/// and the limit keeps a pathological file from exhausting the native stack
/// while the workspace indexer walks it.  See
/// `docs/design/compiler/recursive-descent-depth-limits.md`.
const MAX_GUARD_NESTING_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(32);

/// One condition that must hold for a `package ifneeded` to be reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
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
            Self::TclSatisfies {
                requirements,
                expect,
            } => match target {
                Some(v) => {
                    if v.satisfies_any(requirements) == *expect {
                        Ternary::Yes
                    } else {
                        Ternary::No
                    }
                }
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
pub struct Conditions(Vec<Condition>);

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
        Self(out)
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
}

/// Walk `content`, calling `visit` once per `package ifneeded` declaration
/// reachable on *some* Tcl release, with the conditions guarding it.
///
/// A declaration under a constant-false guard is still visited, carrying
/// [`Condition::Impossible`]: reporting it lets a consumer distinguish "this
/// index does not mention the package" from "this index mentions it but can
/// never register it".
pub fn scan(content: &str, registry: &CommandRegistry, visit: &mut dyn FnMut(Reached<'_>)) {
    walk_script(content, registry, &Conditions::default(), 0, visit);
}

/// Whether `words` registers a deferred package-load script — `package
/// ifneeded NAME VERSION SCRIPT`.
///
/// Identified from registry data, not by name: the subcommand is resolved
/// through the registry (so an unambiguous abbreviation — `package ifn` — is
/// recognised exactly as the interpreter recognises it), and it qualifies when
/// it both declares [`Traits::PROVIDES_PACKAGE`] ("running this makes a
/// package available") and takes a [`ArgRole::Body`] script argument ("…via a
/// script evaluated later").  `package provide` carries the trait but takes no
/// script, so the pair picks out `ifneeded` alone.
fn is_package_ifneeded(text: &str, words: &[Vec<Token>], registry: &CommandRegistry) -> bool {
    if words.len() < 2 {
        return false;
    }
    let head = word_raw(text, &words[0]).trim_start_matches("::");
    let Some(spec) = registry.get(head) else {
        return false;
    };
    spec.resolve_subcommand(word_raw(text, &words[1]))
        .is_some_and(|sub| {
            sub.traits.contains(Traits::PROVIDES_PACKAGE)
                && sub.arg_roles.iter().any(|(_, role)| *role == ArgRole::Body)
        })
}

/// Walk one script level in order.
///
/// Returns the conditions under which control reaches the *end* of the script,
/// or `None` when it never does (an unconditional terminator).
fn walk_script<'t>(
    text: &'t str,
    registry: &CommandRegistry,
    reached: &Conditions,
    depth: u32,
    visit: &mut dyn FnMut(Reached<'_>),
) -> Option<Conditions> {
    if MAX_GUARD_NESTING_DEPTH.exceeded(depth) {
        return Some(reached.with(Condition::Undecidable));
    }
    let commands = walk_command_words(text);
    let mut reached = reached.clone();
    for words in &commands {
        if words.is_empty() {
            continue;
        }
        if is_package_ifneeded(text, words, registry) {
            visit(Reached {
                text,
                words,
                conditions: reached.clone(),
            });
            continue;
        }
        let head = word_raw(text, &words[0]).trim_start_matches("::");
        let Some(spec) = registry.get(head) else {
            continue;
        };
        if spec.traits.contains(Traits::TERMINATES_BLOCK) {
            return None;
        }
        if spec.traits.contains(Traits::HAS_BOOLEAN_COND) && spec.arg_role_resolver.is_some() {
            match walk_if(text, words, head, registry, &reached, depth, visit) {
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
            && command_body_may_terminate(text, words, head, registry, depth)
        {
            reached = reached.with(Condition::Undecidable);
        }
    }
    Some(reached)
}

/// Whether any [`ArgRole::Body`] argument of this command contains a
/// terminator.
fn command_body_may_terminate(
    text: &str,
    words: &[Vec<Token>],
    head: &str,
    registry: &CommandRegistry,
    depth: u32,
) -> bool {
    let args: Vec<&str> = words[1..].iter().map(|w| word_raw(text, w)).collect();
    registry
        .arg_indices_for_role(head, &args, ArgRole::Body)
        .into_iter()
        .filter_map(|i| words[1..].get(i))
        .any(|word| body_may_terminate(&script_of(text, word), registry, depth + 1))
}

/// The script text of a body word: the inside of a braced / quoted / bracketed
/// word, or the word itself when it is bare (`if {$c} return`).
fn script_of(text: &str, word: &[Token]) -> String {
    word_unwrap(text, word).unwrap_or_else(|| word_raw(text, word).to_owned())
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

/// Walk an `if` chain: visit the declarations in every branch, and report what
/// happens to control flow after it.
fn walk_if(
    text: &str,
    words: &[Vec<Token>],
    head: &str,
    registry: &CommandRegistry,
    reached: &Conditions,
    depth: u32,
    visit: &mut dyn FnMut(Reached<'_>),
) -> AfterIf {
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

    for clause in &clauses {
        let branch_conditions = match clause.expr {
            // The `else` branch runs exactly when every earlier test failed.
            None => {
                has_else = true;
                earlier_false.clone()
            }
            Some(expr_index) => {
                let guard = guard_expr(text, &words[1..][expr_index]);
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
        let body_text = script_of(text, &words[1..][clause.body]);
        let mut branch_reached = reached.clone();
        for condition in &branch_conditions.0 {
            branch_reached = branch_reached.with(condition.clone());
        }
        if walk_script(&body_text, registry, &branch_reached, depth + 1, visit).is_none() {
            if branch_conditions.is_unconditional() {
                definitely_taken_terminates = true;
            }
            terminating.push(match branch_conditions.0.as_slice() {
                [only] => Some(only.clone()),
                _ => None,
            });
        } else if body_may_terminate(&body_text, registry, depth + 1) {
            terminating.push(Some(Condition::Undecidable));
        }
    }

    if definitely_taken_terminates || (has_else && terminating.len() == clauses.len()) {
        return AfterIf::Stops;
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

/// Whether `script` contains a terminator anywhere, however deeply guarded.
///
/// Used only to decide that a branch *might* stop the script, so an
/// undecidable guard is recorded rather than none at all.
fn body_may_terminate(script: &str, registry: &CommandRegistry, depth: u32) -> bool {
    if MAX_GUARD_NESTING_DEPTH.exceeded(depth) {
        return false;
    }
    walk_command_words(script).iter().any(|words| {
        words.first().is_some_and(|first| {
            let head = word_raw(script, first).trim_start_matches("::");
            registry
                .get(head)
                .is_some_and(|spec| spec.traits.contains(Traits::TERMINATES_BLOCK))
        }) || words.iter().any(|word| {
            word_unwrap(script, word)
                .is_some_and(|inner| body_may_terminate(&inner, registry, depth + 1))
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
fn guard_expr(text: &str, word: &[Token]) -> Guard {
    parse_guard(script_of(text, word).trim())
}

/// Parse a condition's source text.
fn parse_guard(expr: &str) -> Guard {
    let expr = expr.trim();
    if let Some(rest) = expr.strip_prefix('!') {
        return match parse_guard(rest) {
            Guard::Constant(value) => Guard::Constant(!value),
            Guard::TclSatisfies {
                requirements,
                negated,
            } => Guard::TclSatisfies {
                requirements,
                negated: !negated,
            },
            Guard::Unknown => Guard::Unknown,
        };
    }
    // A whole-expression `[...]` substitution: unwrap and read the command.
    if let Some(inner) = bracketed_body(expr) {
        return parse_vsatisfies(inner);
    }
    // The same acceptor `if` itself uses (`Tcl_GetBooleanFromObj`'s string
    // path), so `1`, `true`, `yes`, `on`, `2`, and `0x0` read exactly as the
    // interpreter reads them.
    match tcl_syntax::boolean::truthiness(expr) {
        Some(value) => Guard::Constant(value),
        None => Guard::Unknown,
    }
}

/// The inner text of `expr` when it is exactly one `[...]` substitution.
fn bracketed_body(expr: &str) -> Option<&str> {
    let inner = expr.strip_prefix('[')?.strip_suffix(']')?;
    // Reject `[a] + [b]`, where the outer brackets are not a matched pair.
    let mut depth = 0i32;
    for ch in inner.chars() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            _ => {}
        }
    }
    (depth == 0).then_some(inner)
}

/// Read `package vsatisfies [package provide Tcl] REQ …` — the only
/// non-constant test this scan evaluates.
///
/// The subject must be a `[package provide Tcl]` or `[package require Tcl]`
/// substitution: a `vsatisfies` over some *other* package says nothing about
/// the Tcl release, and guessing there would be worse than abstaining.
fn parse_vsatisfies(script: &str) -> Guard {
    let commands = walk_command_words(script);
    let [words] = commands.as_slice() else {
        return Guard::Unknown;
    };
    if words.len() < 4 {
        return Guard::Unknown;
    }
    if word_raw(script, &words[0]).trim_start_matches("::") != "package"
        || word_raw(script, &words[1]) != "vsatisfies"
    {
        return Guard::Unknown;
    }
    if !is_tcl_version_subject(script, &words[2]) {
        return Guard::Unknown;
    }
    let requirements: Vec<String> = words[3..]
        .iter()
        .map(|w| word_raw(script, w).trim_matches(['{', '}', '"']).to_owned())
        .collect();
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
fn is_tcl_version_subject(script: &str, word: &[Token]) -> bool {
    if word.len() != 1 || word[0].kind != TokenType::Cmd {
        return false;
    }
    let Some(inner) = word_unwrap(script, word) else {
        return false;
    };
    let commands = walk_command_words(&inner);
    let [words] = commands.as_slice() else {
        return false;
    };
    if words.len() != 3 {
        return false;
    }
    word_raw(&inner, &words[0]).trim_start_matches("::") == "package"
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
