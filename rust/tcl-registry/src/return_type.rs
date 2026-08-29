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

//! Per-call return typing — the algorithms behind [`ReturnTypeHookId`].
//!
//! [`CommandSpec::return_type`] is one fact per command. A handful of core
//! commands hand back a different *kind* of value depending on how they were
//! called, and the spec names the algorithm for those with a
//! [`ReturnTypeHookId`]; this module keeps the algorithms themselves, the way
//! the compiler keeps the lowering and analyser hook implementations.
//!
//! They live in the registry rather than the compiler because a consumer
//! inside the registry needs them: [`crate::taint::is_sanitiser`] asks whether
//! a call's result is a fixed numeric type, which is the same question.
//! Keeping one implementation is the point —
//! [`CommandSpec::return_type_for_call`] is the single entry point for "what
//! does this call return", so SSA type propagation, the taint sanitiser test
//! and the shimmer byte-array check cannot disagree about a per-form result.
//!
//! Every algorithm may answer `None`, meaning the result's intrep is unknown
//! for that call, and every consumer already handles that. A confidently wrong
//! type is what issue #1720 was, so an algorithm names a type only where the
//! intrep is *guaranteed*. Three things make it not guaranteed:
//!
//! * **The value is the caller's.** `lsearch -inline` hands back an element
//!   lifted out of the source list, whose intrep is whatever was put there.
//! * **The empty result is a pure string.** Several list-returning forms only
//!   build a list object when they find something: verified with
//!   `tcl::unsupported::representation` on tclsh 9.0.4,
//!   `regexp -inline z a`, `lsearch -all {a b} z`, `scan "" {%d %d}` and
//!   `pid` on a non-pipeline channel are each a *pure string*, while their
//!   matching counterparts are lists. Typing those `List` would both hide a
//!   real string→list conversion and invent list→string ones. (`regexp
//!   -about` and `lsearch -subindices` *are* guaranteed — `-about` never
//!   matches at all, and `-subindices` answers `-1 0` rather than a bare
//!   `-1`.)
//! * **A switch could be hiding in a substitution.** See
//!   [`switches_are_certain`].
//!
//! One thing this layer cannot see is `{*}` expansion: the lexer strips the
//! prefix, so `regexp {*}$opts $s` arrives as `["$opts", "$s"]`. The dynamic
//! word still trips [`switches_are_certain`], so such a call answers
//! "unknown" — but by that route rather than by recognising the expansion.

use crate::hooks::ReturnTypeHookId;
use crate::spec::CommandSpec;
use crate::types::TclType;

/// The type `[<spec's command> args…]` produces, for a spec carrying `hook`.
///
/// `args` excludes the command name. The `match` is exhaustive, so adding a
/// [`ReturnTypeHookId`] variant is a compile error until its arm lands here.
#[must_use]
pub(crate) fn resolve(
    hook: ReturnTypeHookId,
    spec: &CommandSpec,
    args: &[&str],
) -> Option<TclType> {
    match hook {
        ReturnTypeHookId::Regexp => regexp(spec, args),
        ReturnTypeHookId::Lsearch => lsearch(spec, args),
        ReturnTypeHookId::Regsub => regsub(spec, args),
        ReturnTypeHookId::Scan => scan(spec, args),
        ReturnTypeHookId::Pid => pid(spec, args),
    }
}

/// `regexp ?switches? exp string ?matchVar ...?`.
///
/// `-about` skips matching entirely and always returns the two-element
/// `{subexpressionCount propertyList}` — a guaranteed list.
///
/// `-inline` returns the matched substrings, which is a list *when something
/// matched*: `regexp -inline z a` is a pure string on tclsh 9.0.4. So the
/// answer is "not the int a bare `regexp` returns", which is what #1720
/// needed, without claiming a list intrep that only a match produces.
///
/// Everything else is the 0/1 flag, or the count under `-all`.
fn regexp(spec: &CommandSpec, args: &[&str]) -> Option<TclType> {
    let switches = spec.leading_switch_names(args);
    if switches.contains(&"-about") {
        return Some(TclType::List);
    }
    if switches.contains(&"-inline") {
        return None;
    }
    if !switches_are_certain(spec, args) {
        return None;
    }
    Some(TclType::Int)
}

/// `lsearch ?options? list pattern`.
///
/// Order matters here in a way no switch/type table captures, which is why
/// this is a program:
///
/// * `-all` and `-inline` both leave the result untypeable, for different
///   reasons — `-all` builds a list only when it matches (`lsearch -all {a b}
///   z` is a pure string), and `-inline` hands back an element whose intrep is
///   the caller's. Both dominate, so they are tested first.
/// * `-inline` in particular beats `-subindices`, which only reshapes an
///   *index* result: `lsearch -inline -index 0 -subindices {{a} {b}} b` is
///   `b` on tclsh 9.0.4, the element, not a path.
/// * `-subindices` (legal only beside `-index`) is therefore reached only for
///   a plain index result, and turns it into the full index path. Unlike the
///   others it *is* guaranteed a list: the no-match answer is `-1 0`, not a
///   bare `-1`.
fn lsearch(spec: &CommandSpec, args: &[&str]) -> Option<TclType> {
    let switches = spec.leading_switch_names(args);
    let given = |name: &'static str| switches.contains(&name);
    if given("-all") || given("-inline") {
        return None;
    }
    if given("-subindices") {
        return Some(TclType::List);
    }
    if !switches_are_certain(spec, args) {
        return None;
    }
    Some(TclType::Int)
}

/// `regsub ?switches? exp string subSpec ?varName?`.
///
/// Three positionals is the `varName`-omitted form, which returns the
/// substituted string; four is the counting form. True in every release 8.4
/// through 9.1 — `Tcl_RegsubObjCmd`: "no varname supplied, so just return the
/// modified string".
fn regsub(spec: &CommandSpec, args: &[&str]) -> Option<TclType> {
    if !switches_are_certain(spec, args) {
        return None;
    }
    match spec.positional_word_count(args) {
        3 => Some(TclType::String),
        4 => Some(TclType::Int),
        // Any other count is an arity error, which is a diagnostic's job to
        // report and not a shape this can name a type for.
        _ => None,
    }
}

/// `scan string format ?varName ...?`.
///
/// Two positionals is the inline form: no variables to write, so the
/// converted values come back as a list — but only once something converts.
/// `scan "" {%d %d}` is a pure string on tclsh 9.0.4, so the inline form is
/// "not the conversion count", not a guaranteed list. With variables the
/// result really is the count.
fn scan(spec: &CommandSpec, args: &[&str]) -> Option<TclType> {
    match spec.positional_word_count(args) {
        n if n > 2 => Some(TclType::Int),
        _ => None,
    }
}

/// `pid ?fileId?`.
///
/// Bare `pid` is this process's id, an int. The `fileId` form yields the
/// pipeline's process ids as a list — except on a channel that is not a
/// pipeline, where tclsh 9.0.4 answers a pure string, so that form is not a
/// guaranteed list.
fn pid(spec: &CommandSpec, args: &[&str]) -> Option<TclType> {
    match spec.positional_word_count(args) {
        0 => Some(TclType::Int),
        _ => None,
    }
}

/// Whether the switch run this call shows us is the switch run Tcl will see.
///
/// Tcl parses options after substitution, so a word that arrives as `$mode`
/// can be `-inline` at run time: `set mode -inline; regexp $mode {.+} $x`
/// really does return the matched substrings. Reading the visible words alone
/// would type that call `Int` and let `is_sanitiser` launder attacker-derived
/// text — the #1720 mistake, reached through substitution.
///
/// A `--` in the consumed run settles it: everything after is positional
/// whatever it looks like, which is exactly why W304 tells authors to write
/// one. Otherwise the word the scan stopped on is the risk, and only when it
/// is dynamic; a literal there is a positional and always was.
///
/// A switch we *did* resolve stays authoritative regardless, since
/// substitution can add words but never remove them, so each algorithm
/// consults this only after failing to find its own.
fn switches_are_certain(spec: &CommandSpec, args: &[&str]) -> bool {
    let scanned = spec.switch_word_count(args);
    if args[..scanned].contains(&"--") {
        return true;
    }
    args.get(scanned).is_none_or(|word| !is_dynamic(word))
}

/// Whether *word* gets its value at run time, so its spelling is not known
/// here. Covers `$var`, `[cmd]`, and a `{*}`-expanded word (whose `{*}` prefix
/// the lexer has already stripped, leaving the substitution behind).
fn is_dynamic(word: &str) -> bool {
    word.contains('$') || word.contains('[')
}

#[cfg(test)]
mod tests {
    use crate::CommandRegistry;
    use crate::types::TclType;

    fn returns(command: &str, args: &[&str]) -> Option<TclType> {
        CommandRegistry::build_default()
            .get(command)
            .expect("command is registered")
            .return_type_for_call(args)
    }

    /// Issue #1720. The headline: `regexp -inline` is not the int a bare
    /// `regexp` returns, so iterating its result draws no shimmer warning.
    /// It is not typed `List` either — `regexp -inline z a` is a *pure
    /// string* on tclsh 9.0.4, only a match builds a list.
    #[test]
    fn regexp_inline_is_not_an_int_and_not_a_guaranteed_list() {
        assert_eq!(returns("regexp", &["-all", "-inline", "{p}", "s"]), None);
        assert_eq!(returns("regexp", &["-inline", "{p}", "s"]), None);
        assert_eq!(returns("regexp", &["{p}", "s"]), Some(TclType::Int));
        assert_eq!(
            returns("regexp", &["-all", "{p}", "s"]),
            Some(TclType::Int),
            "-all without -inline is a match count"
        );
    }

    /// `-about` never matches at all, so its two-element
    /// `{subexpressionCount propertyList}` is a guaranteed list.
    #[test]
    fn regexp_about_is_a_guaranteed_list() {
        assert_eq!(returns("regexp", &["-about", "{p}"]), Some(TclType::List));
    }

    /// `--` ends switch parsing, so a following `-inline` is the pattern.
    #[test]
    fn a_switch_past_the_terminator_is_an_operand() {
        assert_eq!(
            returns("regexp", &["--", "-inline", "s"]),
            Some(TclType::Int)
        );
    }

    /// `Tcl_RegexpObjCmd` reads its table with `TCL_EXACT`, so `-inl` is not
    /// `-inline` — tclsh 8.6 and 9.0.4 both answer `bad option "-inl"`.
    /// `Tcl_LsearchObjCmd` uses the abbreviating default, so `lsearch -al` is
    /// `-all` and really is the untypeable form.
    #[test]
    fn an_abbreviation_resolves_exactly_where_tcl_resolves_one() {
        assert_eq!(
            returns("regexp", &["-inl", "{p}", "s"]),
            Some(TclType::Int),
            "regexp matches switches exactly, so -inl is the pattern"
        );
        assert_eq!(
            returns("lsearch", &["-al", "l", "a"]),
            None,
            "lsearch resolves unique prefixes, so -al is -all"
        );
    }

    /// Tcl parses options *after* substitution, so a word arriving as `$mode`
    /// can be `-inline` at run time (`set mode -inline; regexp $mode {.+} $x`
    /// returns the matched text on tclsh 9.0.4). Typing that from the visible
    /// switches would let `is_sanitiser` launder attacker-derived text.
    #[test]
    fn a_dynamic_word_where_a_switch_could_go_is_unknown() {
        assert_eq!(returns("regexp", &["$mode", "{p}", "s"]), None);
        assert_eq!(returns("lsearch", &["$mode", "l", "a"]), None);
        assert_eq!(returns("regsub", &["$mode", "p", "s", "b"]), None);
        assert_eq!(
            returns("regexp", &["[getMode]", "{p}", "s"]),
            None,
            "a command substitution is just as unresolved"
        );
    }

    /// `--` settles it — which is exactly what W304 tells authors to write.
    /// A literal in the same position never was a switch.
    #[test]
    fn a_terminator_or_a_literal_makes_the_switch_run_certain() {
        assert_eq!(returns("regexp", &["--", "$pat", "$s"]), Some(TclType::Int));
        assert_eq!(returns("regexp", &["{p}", "$s"]), Some(TclType::Int));
        assert_eq!(
            returns("regexp", &["-nocase", "--", "$pat", "$s"]),
            Some(TclType::Int)
        );
    }

    /// A switch we did resolve stays authoritative — substitution can add
    /// words but never remove them.
    #[test]
    fn a_resolved_switch_survives_a_later_dynamic_word() {
        assert_eq!(returns("regexp", &["-inline", "$pat", "$s"]), None);
        assert_eq!(returns("regexp", &["-about", "$pat"]), Some(TclType::List));
    }

    /// `-all` builds a list only when it matches (`lsearch -all {a b} z` is a
    /// pure string), and `-inline` yields an element whose intrep is the
    /// caller's — neither is typeable. A bare `lsearch` really is an index.
    #[test]
    fn lsearch_all_and_inline_are_untypeable_but_a_bare_search_is_an_int() {
        assert_eq!(returns("lsearch", &["l", "a"]), Some(TclType::Int));
        assert_eq!(returns("lsearch", &["-all", "l", "a"]), None);
        assert_eq!(returns("lsearch", &["-all", "-inline", "l", "a"]), None);
        assert_eq!(returns("lsearch", &["-inline", "l", "a"]), None);
    }

    /// `-subindices` is the one lsearch form that *is* guaranteed a list: its
    /// no-match answer is `-1 0`, not a bare `-1`. `-inline` still dominates
    /// it, since that yields the element.
    #[test]
    fn subindices_is_a_guaranteed_list_but_inline_still_dominates() {
        assert_eq!(
            returns("lsearch", &["-index", "0", "-subindices", "l", "b"]),
            Some(TclType::List)
        );
        assert_eq!(
            returns("lsearch", &["-index", "0", "l", "b"]),
            Some(TclType::Int),
            "-index alone is a single index"
        );
        assert_eq!(
            returns(
                "lsearch",
                &["-inline", "-index", "0", "-subindices", "l", "b"]
            ),
            None
        );
    }

    /// `Tcl_LsearchObjCmd` scans `i < objc - 2`, so the list and pattern
    /// operands are never option candidates however they are spelled:
    /// `lsearch -all foo` searches the one-element list `-all` for `foo` and
    /// returns -1 on tclsh 9.0.4.
    #[test]
    fn a_reserved_operand_spelled_like_a_switch_is_not_one() {
        assert_eq!(returns("lsearch", &["-all", "foo"]), Some(TclType::Int));
        assert_eq!(
            returns("lsearch", &["-inline", "-exact"]),
            Some(TclType::Int)
        );
    }

    /// A switch's value word is consumed by it, so it is neither read as a
    /// switch nor counted as a positional.
    #[test]
    fn a_switch_value_word_does_not_shift_the_result() {
        assert_eq!(
            returns("lsearch", &["-start", "2", "l", "c"]),
            Some(TclType::Int)
        );
    }

    /// tclsh 9.0.4: `regsub -all {8} "tcl 8.6" 9` is `tcl 9.6`; the same call
    /// with a `varName` is `1`. Both forms are guaranteed.
    #[test]
    fn regsub_returns_a_string_until_a_varname_makes_it_a_count() {
        assert_eq!(
            returns("regsub", &["-all", "a", "s", "b"]),
            Some(TclType::String)
        );
        assert_eq!(returns("regsub", &["a", "s", "b"]), Some(TclType::String));
        assert_eq!(
            returns("regsub", &["-all", "a", "s", "b", "out"]),
            Some(TclType::Int)
        );
    }

    /// The variable-writing `scan` really is a conversion count; the inline
    /// form is a list only once something converts (`scan "" {%d %d}` is a
    /// pure string).
    #[test]
    fn scan_counts_conversions_but_its_inline_form_is_untypeable() {
        assert_eq!(returns("scan", &["s", "{%d %d}"]), None);
        assert_eq!(
            returns("scan", &["s", "{%d %d}", "a", "b"]),
            Some(TclType::Int)
        );
    }

    /// Bare `pid` is this process's id; the `fileId` form is a list only for a
    /// real pipeline, and a pure string otherwise.
    #[test]
    fn pid_is_an_int_alone_and_untypeable_for_a_channel() {
        assert_eq!(returns("pid", &[]), Some(TclType::Int));
        assert_eq!(returns("pid", &["chan"]), None);
    }

    /// A command with no hook still answers from its static `return_type`.
    #[test]
    fn a_hookless_command_keeps_its_static_return_type() {
        assert_eq!(returns("llength", &["l"]), Some(TclType::Int));
        assert_eq!(returns("list", &["a", "b"]), Some(TclType::List));
    }
}
