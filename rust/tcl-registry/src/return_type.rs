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
//! **Known limitation — `{*}` expansion.** The words reaching these
//! algorithms have already had any `{*}` prefix stripped by the lexer, so
//! `regexp {*}$opts $s` arrives as `["$opts", "$s"]` and is indistinguishable
//! from a call with one literal positional. An expansion that supplies
//! `-inline` at run time is therefore still typed from the switches visible
//! statically. This predates per-call typing — `regexp` was unconditionally
//! `Int` before — and closing it needs the expansion marker preserved through
//! `tcl_compiler::value_shapes::parse_command_substitution`, which every
//! command-substitution consumer shares. Guarding here alone cannot work: the
//! marker is gone before the call is made.
//!
//! Every algorithm may answer `None`, meaning the result's intrep is unknown
//! for that call. That is the honest answer whenever a command hands back a
//! value the caller supplied (`lsearch -inline` returns an element whose
//! intrep is whatever was put in the list), and every consumer already handles
//! it. A confidently wrong type is what issue #1720 was.

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
        ReturnTypeHookId::Regexp => Some(regexp(spec, args)),
        ReturnTypeHookId::Lsearch => lsearch(spec, args),
        ReturnTypeHookId::Regsub => regsub(spec, args),
        ReturnTypeHookId::Scan => scan(spec, args),
        ReturnTypeHookId::Pid => pid(spec, args),
    }
}

/// `regexp ?switches? exp string ?matchVar ...?`.
///
/// `-inline` replaces the 0/1 (or `-all` count) result with the list of
/// matched substrings — a list even when nothing matched, since that is the
/// empty list. `-about` skips matching and returns a two-element
/// `{subexpressionCount propertyList}`. Verified on tclsh 9.0.4:
/// `llength [regexp -all -inline {(\d+)\.(\d+)} "tcl 8.6 tk 8.6"]` is 6.
fn regexp(spec: &CommandSpec, args: &[&str]) -> TclType {
    let switches = spec.leading_switch_names(args);
    if switches.iter().any(|s| matches!(*s, "-inline" | "-about")) {
        return TclType::List;
    }
    TclType::Int
}

/// `lsearch ?options? list pattern`.
///
/// Order matters here in a way no switch/type table captures, which is why
/// this is a program:
///
/// * `-all` wins outright — a list of indices, or of the matching elements
///   when combined with `-inline`.
/// * `-inline` then beats `-subindices`, because it yields the matching
///   *element* while `-subindices` only reshapes an *index* result:
///   `lsearch -inline -index 0 -subindices {{a} {b}} b` is `b` on tclsh 9.0.4.
/// * `-subindices` (legal only beside `-index`) is therefore reached only for
///   a plain index result, which it turns into the full index path —
///   `lsearch -index 0 -subindices {{a} {b}} b` is `1 0`.
fn lsearch(spec: &CommandSpec, args: &[&str]) -> Option<TclType> {
    let switches = spec.leading_switch_names(args);
    let given = |name: &'static str| switches.contains(&name);
    if given("-all") {
        return Some(TclType::List);
    }
    if given("-inline") {
        return None;
    }
    if given("-subindices") {
        return Some(TclType::List);
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
/// converted values come back as a list (`llength [scan "12 34" {%d %d}]` is
/// 2 on tclsh 9.0.4). With variables the result is the conversion count.
fn scan(spec: &CommandSpec, args: &[&str]) -> Option<TclType> {
    match spec.positional_word_count(args) {
        2 => Some(TclType::List),
        n if n > 2 => Some(TclType::Int),
        _ => None,
    }
}

/// `pid ?fileId?`.
///
/// The pipeline form yields the list of decimal-string process ids — empty
/// when the channel is not a pipeline, which is still a list. Bare `pid` is
/// this process's id.
fn pid(spec: &CommandSpec, args: &[&str]) -> Option<TclType> {
    match spec.positional_word_count(args) {
        0 => Some(TclType::Int),
        1 => Some(TclType::List),
        _ => None,
    }
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

    /// Issue #1720. Ground truth tclsh 9.0.4:
    /// `llength [regexp -all -inline {(\d+)\.(\d+)} "tcl 8.6 tk 8.6"]` is 6,
    /// a bare `regexp` of the same pattern is 1, and `regexp -about {(a)(b)*c}`
    /// is `1 {REG_UNONPOSIX REG_ULOCALE}`.
    #[test]
    fn regexp_inline_and_about_are_lists_and_a_bare_match_is_an_int() {
        assert_eq!(
            returns("regexp", &["-all", "-inline", "{p}", "$s"]),
            Some(TclType::List)
        );
        assert_eq!(
            returns("regexp", &["-inline", "{p}", "$s"]),
            Some(TclType::List)
        );
        assert_eq!(returns("regexp", &["-about", "{p}"]), Some(TclType::List));
        assert_eq!(returns("regexp", &["{p}", "$s"]), Some(TclType::Int));
        assert_eq!(
            returns("regexp", &["-all", "{p}", "$s"]),
            Some(TclType::Int),
            "-all without -inline is a match count"
        );
    }

    /// `--` ends switch parsing, so a following `-inline` is the pattern.
    #[test]
    fn a_switch_past_the_terminator_is_an_operand() {
        assert_eq!(
            returns("regexp", &["--", "-inline", "$s"]),
            Some(TclType::Int)
        );
    }

    /// `Tcl_RegexpObjCmd` reads its table with `TCL_EXACT`, so `-inl` is not
    /// `-inline` — tclsh 8.6 and 9.0.4 both answer `bad option "-inl"`.
    /// `Tcl_LsearchObjCmd` uses the abbreviating default, so `lsearch -al` is
    /// `-all` and really does return a list (`lsearch -al {a b a} a` is `0 2`).
    #[test]
    fn an_abbreviation_resolves_exactly_where_tcl_resolves_one() {
        assert_eq!(
            returns("regexp", &["-inl", "{p}", "$s"]),
            Some(TclType::Int),
            "regexp matches switches exactly"
        );
        assert_eq!(
            returns("lsearch", &["-al", "$l", "a"]),
            Some(TclType::List),
            "lsearch resolves unique prefixes"
        );
    }

    #[test]
    fn lsearch_all_is_a_list_and_a_bare_inline_is_untypeable() {
        assert_eq!(returns("lsearch", &["$l", "a"]), Some(TclType::Int));
        assert_eq!(
            returns("lsearch", &["-all", "$l", "a"]),
            Some(TclType::List)
        );
        assert_eq!(
            returns("lsearch", &["-all", "-inline", "$l", "a"]),
            Some(TclType::List)
        );
        assert_eq!(returns("lsearch", &["-inline", "$l", "a"]), None);
    }

    /// `-subindices` turns a plain index into the full index path
    /// (`lsearch -index 0 -subindices {{a} {b}} b` is `1 0`), but `-inline`
    /// dominates it — the same call with `-inline` is `b`.
    #[test]
    fn inline_dominates_subindices() {
        assert_eq!(
            returns("lsearch", &["-index", "0", "-subindices", "$l", "b"]),
            Some(TclType::List)
        );
        assert_eq!(
            returns("lsearch", &["-index", "0", "$l", "b"]),
            Some(TclType::Int),
            "-index alone is a single index"
        );
        assert_eq!(
            returns(
                "lsearch",
                &["-inline", "-index", "0", "-subindices", "$l", "b"]
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
            returns("lsearch", &["-start", "2", "$l", "c"]),
            Some(TclType::Int)
        );
    }

    /// tclsh 9.0.4: `regsub -all {8} "tcl 8.6" 9` is `tcl 9.6`; the same call
    /// with a `varName` is `1`.
    #[test]
    fn regsub_returns_a_string_until_a_varname_makes_it_a_count() {
        assert_eq!(
            returns("regsub", &["-all", "a", "$s", "b"]),
            Some(TclType::String)
        );
        assert_eq!(returns("regsub", &["a", "$s", "b"]), Some(TclType::String));
        assert_eq!(
            returns("regsub", &["-all", "a", "$s", "b", "out"]),
            Some(TclType::Int)
        );
    }

    #[test]
    fn scan_inline_is_a_list_and_the_writing_form_a_count() {
        assert_eq!(returns("scan", &["$s", "{%d %d}"]), Some(TclType::List));
        assert_eq!(
            returns("scan", &["$s", "{%d %d}", "a", "b"]),
            Some(TclType::Int)
        );
    }

    #[test]
    fn pid_is_an_int_alone_and_a_list_for_a_pipeline() {
        assert_eq!(returns("pid", &[]), Some(TclType::Int));
        assert_eq!(returns("pid", &["$chan"]), Some(TclType::List));
    }

    /// A command with no hook still answers from its static `return_type`.
    #[test]
    fn a_hookless_command_keeps_its_static_return_type() {
        assert_eq!(returns("llength", &["$l"]), Some(TclType::Int));
        assert_eq!(returns("list", &["a", "b"]), Some(TclType::List));
    }
}
