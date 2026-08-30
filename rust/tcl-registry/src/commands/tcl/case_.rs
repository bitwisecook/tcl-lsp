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

//! `case` — obsolete pattern-based branching, superseded by `switch` (Tcl 8.x).
//
// Release span, verified against the shipped source trees: `doc/case.n` is
// present in tcl8.4.20, tcl8.5.19, and tcl8.6.16 and absent from tcl9.0.4 and
// tcl9.1b0; `info commands ::case` finds it on tclsh 8.4.20 / 8.5.19 / 8.6.14
// and not on 9.0.4 / 9.1b0.  The three 8.x pages differ only in `man.macros`
// boilerplate placement, so the documented grammar and semantics are identical
// across the whole 8.x span.  The page carries `.TH case n 7.0` — the command
// predates 8.0 and its own DESCRIPTION opens with "the case command is
// obsolete and is supported only for backward compatibility … You should use
// the switch command instead", which is why `deprecated_replacement` points at
// `switch`.
//
// It is *not* a drop-in rename: `case` has no options at all (no `-exact` /
// `-glob` / `-regexp` / `--`), always matches with `string match` glob
// semantics, and accepts an optional literal `in` word between the subject and
// the pattern list that `switch` does not.  A mechanical rewrite therefore has
// to drop `in` and, when the subject can begin with `-`, insert `--`; hence
// `deprecated_replacement_drop_in: false`.
//
// The iRules TMM interpreter is an embedded Tcl 8.4.6 and provides `case`
// exactly as its base release does, so the group carries an iRules row
// alongside `TCL8X`.
use crate::prelude::*;
use tcl_dialect::model::Family;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::surface;

const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "case string ?in? patList body ?patList body ...?",
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "case string ?in? {patList body ?patList body ...?}",
        ..FormSpec::DEFAULT
    },
];

/// A `case` body is a script, so folding / semantic tokens / analysis treat it
/// as one.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// Dynamic arg role resolver for `case`.
///
/// The subject is always word 0.  An optional literal `in` may follow it (the
/// manpage's `?in?`), after which the remainder is either a single braced
/// `{patList body ...}` word or alternating `patList body` pairs — the same
/// two shapes `switch` has, minus every option.  Only the *body* words get a
/// role; the pattern words are ordinary data.
fn case_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let mut i = 1;
    // The optional `in` separator, taken literally: a substituted word cannot
    // be recognised, and treating an unknown word as `in` would shift every
    // body index by one.
    if args.get(i).is_some_and(|a| *a == "in") {
        i += 1;
    }
    let mut roles = Vec::new();
    if i >= args.len() {
        return roles;
    }
    // Braced-list form: one trailing word holding all the pairs.
    if i == args.len() - 1 {
        if let Ok(idx) = u8::try_from(i) {
            roles.push((idx, ArgRole::Body));
        }
        return roles;
    }
    // Separate-words form: alternating `patList body` pairs.
    while i + 1 < args.len() {
        if let Ok(idx) = u8::try_from(i + 1) {
            roles.push((idx, ArgRole::Body));
        }
        i += 2;
    }
    roles
}

/// Command spec for `case`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "case",
        // Tcl 8.x only; removed in Tcl 9.0 (no `doc/case.n`, no command) —
        // see the module comment. iRules embeds Tcl 8.4.6 and keeps it.
        surface: Some(surface![
            SpecSurface::core_in(Family::Tcl, &[("8.4", Some("8.7"))]),
            SpecSurface::core(Family::F5Irules)
        ]),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::NEVER_INLINE_BODY
            | Traits::HAS_SWITCH_BODY,
        // Floor of 2: `case string {patList body ...}`, the shortest legal
        // call (tclsh 8.6.14 rejects a bare `case x` with "wrong # args").
        // No upper bound and no parity rule, because the optional `in` word
        // shifts the pair alignment by one and the braced form collapses
        // every pair into a single trailing word.
        arity: Arity::new(2, Arity::UNLIMITED),
        arg_role_resolver: Some(case_arg_roles),
        // The braced `{patList body …}` form is a clause list, not a script:
        // without the descriptor + lowering hook the whole braced word would
        // be lowered as one body and every pattern read as a command name.
        // `case` reuses `switch`'s lowering because the two share a
        // shape — one subject, then either separate `patList body` pairs or a
        // single braced clause list; the descriptor carries the differences
        // (no options, glob-only matching, the optional `in` separator).
        case_list: Some(&CaseListSpec::CASE),
        lowering_hook: Some(crate::hooks::LoweringHookId::Switch),
        analyser_hook: Some(crate::hooks::AnalyserHookId::Switch),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        deprecated_replacement: Some("switch"),
        // Not a drop-in rename — `case` has no options and an extra optional
        // `in` word; see the module comment.
        deprecated_replacement_drop_in: false,
        hover: Some(HoverSnippet {
            summary: "Obsolete glob-pattern branching — use `switch` instead.",
            synopsis: &[
                "case string ?in? patList body ?patList body ...?",
                "case string ?in? {patList body ?patList body ...?}",
            ],
            snippet: "Matches string against each patList in turn and evaluates the body of the first one that matches, returning that body's result. Each patList is a single pattern or a list of patterns, and each pattern may use the wild-cards `string match` accepts. A patList of `default` matches unconditionally; with no match and no default, `case` returns the empty string. The patterns and bodies may be given as separate words, where substitutions apply normally, or grouped into one braced argument, which is convenient for multi-line commands but performs no command or variable substitution on the patterns — so the two forms can behave differently. An optional literal `in` may follow string. `case` is obsolete: it has been supported only for backward compatibility since Tcl 7.0 and was removed in Tcl 9.0. `switch` replaces it, but not as a plain rename — `switch` has no `in` word and takes `-exact`/`-glob`/`-regexp`/`--` options, so a subject that can begin with `-` needs `--` and glob matching needs `-glob`.",
            source: "Tcl case(n)",
            examples: "case $x in {a  {puts one}  b  {puts two}}\n\ncase $x {\n    a       { puts one }\n    b       { puts two }\n    default { puts other }\n}\n\n# The Tcl 8.5+/9.x replacement\nswitch -glob -- $x {\n    a       { puts one }\n    b       { puts two }\n    default { puts other }\n}",
            return_value: "The result of the matched body, or an empty string when no patList matches and there is no default clause.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Body positions for a call written as separate words, with and without
    /// the optional `in` separator.  Verified against tclsh 8.6.14:
    /// `set x b; case $x a {puts one} b {puts two}` prints `two`, and the same
    /// call with `in` after the subject prints `two` as well.
    #[test]
    fn separate_word_form_marks_every_body() {
        assert_eq!(
            case_arg_roles(&["$x", "a", "{puts one}", "b", "{puts two}"]),
            vec![(2, ArgRole::Body), (4, ArgRole::Body)]
        );
        assert_eq!(
            case_arg_roles(&["$x", "in", "a", "{puts one}", "b", "{puts two}"]),
            vec![(3, ArgRole::Body), (5, ArgRole::Body)]
        );
    }

    /// The braced clause list is one word, in both spellings.
    #[test]
    fn braced_form_marks_the_single_clause_list() {
        assert_eq!(
            case_arg_roles(&["$x", "a {puts one} b {puts two}"]),
            vec![(1, ArgRole::Body)]
        );
        assert_eq!(
            case_arg_roles(&["$x", "in", "a {puts one} b {puts two}"]),
            vec![(2, ArgRole::Body)]
        );
    }

    /// `in` is recognised only as that exact literal — `Tcl_CaseObjCmd`'s own
    /// `strcmp(arg, "in")`.  A substituted word at that position is the first
    /// pattern, so its *following* word is the body.
    #[test]
    fn separator_is_literal_only() {
        assert_eq!(
            case_arg_roles(&["$x", "$maybe_in", "{puts one}"]),
            vec![(2, ArgRole::Body)]
        );
    }

    /// The descriptor carries the differences from `switch`: no options at
    /// all, so no match-mode / `--` vocabulary, and a `default` honoured
    /// wherever it appears rather than only last.
    #[test]
    fn descriptor_has_no_options_and_a_positionless_default() {
        let case = CaseListSpec::CASE;
        assert_eq!(case.subject_args, 1);
        assert_eq!(case.optional_subject_separator, Some("in"));
        assert!(case.exact_option.is_none());
        assert!(case.glob_option.is_none());
        assert!(case.regex_option.is_none());
        assert!(case.nocase_option.is_none());
        assert!(case.end_options_option.is_none());
        assert!(case.fallthrough_body.is_none());
        assert_eq!(case.keyword_patterns, ["default"]);
        assert!(!case.keyword_patterns_require_final);
    }
}
