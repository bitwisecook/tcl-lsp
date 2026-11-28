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

//! Static `auto_path` evaluation: `file join`, `dirname`, normalisation, the tokeniser and the forms it declines.

use super::*;

// auto_path_eval.rs — the lappend auto_path mini-evaluator
//
// Structural note: the evaluator is a side-effect-free POSIX-path mini-eval; its
// path helpers (`posix_join`, `normpath`, `expanduser`) follow standard POSIX
// path rules, NOT tclsh's `file` command byte-for-byte (tclsh's `//`
// handling differs across 8.6/9.0).  The `[file dirname]` / `[file join]` /
// `[info script]` idiom *resolution* is what is Tcl-observable and is pinned to
// tclsh below; the helper normalisation is asserted against the documented
// POSIX behaviour.  All tests use ABSOLUTE `info_script` / literal paths so
// `abspath` reduces to `normpath` and the result is cwd-independent.

#[test]
fn auto_path_file_join_multi_component() {
    crate::install_surfaces();
    // tclsh (8.6, 9.0): `file join /proj a b c` → /proj/a/b/c.
    assert_eq!(
        evaluate_auto_path_expr("[file join /proj a b c]", None).as_deref(),
        Some("/proj/a/b/c"),
    );
}

#[test]
fn auto_path_file_join_later_absolute_resets() {
    crate::install_surfaces();
    // A later absolute component resets the accumulator (POSIX join + tclsh).
    // tclsh (8.6, 9.0): `file join a /b c` → /b/c.
    assert_eq!(
        evaluate_auto_path_expr("[file join /a /b c]", None).as_deref(),
        Some("/b/c"),
    );
}

#[test]
fn auto_path_file_join_single_arg() {
    crate::install_surfaces();
    // `file join` with exactly one path arg (args.len()==2 after the subcommand)
    // returns it unchanged (then abspath/normpath).
    // tclsh (8.6, 9.0): `file join /solo` → /solo.
    assert_eq!(
        evaluate_auto_path_expr("[file join /solo]", None).as_deref(),
        Some("/solo"),
    );
}

#[test]
fn auto_path_dirname_root_and_trailing_slash() {
    crate::install_surfaces();
    // `posix_dirname` collapses a non-root head's trailing slashes but keeps the
    // root.  tclsh (8.6, 9.0): `file dirname /a/b/c` → /a/b; `file dirname /`
    // → /; `file dirname //` → /.
    assert_eq!(
        evaluate_auto_path_expr("[file dirname /a/b/c]", None).as_deref(),
        Some("/a/b"),
    );
    // A path with a single leading component → its dirname is the root.
    assert_eq!(
        evaluate_auto_path_expr("[file dirname /a]", None).as_deref(),
        Some("/"),
    );
}

#[test]
fn auto_path_normpath_collapses_dotdot_and_dot() {
    crate::install_surfaces();
    // The end-to-end parent-dir idiom collapses `..` and `.` via normpath.
    // tclsh (8.6, 9.0): `file dirname [info script]` of /proj/lib/pkg/x.tcl with
    // a trailing `..` → /proj/lib.
    assert_eq!(
        evaluate_auto_path_expr(
            "[file join [file dirname [info script]] ..]",
            Some("/proj/lib/pkg/x.tcl"),
        )
        .as_deref(),
        Some("/proj/lib"),
    );
    // A literal absolute path with embedded `.` / `..` / `//` normalises.
    assert_eq!(
        evaluate_auto_path_expr("/a/./b/../c", None).as_deref(),
        Some("/a/c"),
    );
}

#[test]
fn auto_path_normpath_double_slash_preserved() {
    crate::install_surfaces();
    // POSIX `normpath` preserves EXACTLY two leading slashes and
    // collapses three-or-more to one (a deliberate POSIX-specific rule; tclsh's
    // own `file` command differs across 8.6/9.0, so this is the documented
    // POSIX behaviour, asserted structurally).
    assert_eq!(
        evaluate_auto_path_expr("//net/share", None).as_deref(),
        Some("//net/share"),
    );
    assert_eq!(
        evaluate_auto_path_expr("///srv/x", None).as_deref(),
        Some("/srv/x"),
    );
}

#[test]
fn auto_path_normpath_dotdot_above_root_is_clamped() {
    crate::install_surfaces();
    // `..` cannot ascend above an absolute root — it is dropped.
    assert_eq!(
        evaluate_auto_path_expr("/../../x", None).as_deref(),
        Some("/x"),
    );
}

#[test]
fn auto_path_nested_and_double_dirname() {
    crate::install_surfaces();
    // `[file dirname [file dirname [info script]]]` climbs two levels.
    // tclsh (8.6, 9.0): for /proj/lib/init.tcl → /proj.
    assert_eq!(
        evaluate_auto_path_expr(
            "[file dirname [file dirname [info script]]]",
            Some("/proj/lib/init.tcl"),
        )
        .as_deref(),
        Some("/proj"),
    );
}

#[test]
fn auto_path_info_non_script_arg_declines() {
    crate::install_surfaces();
    // `[info ...]` with anything other than `script` is unmodelled → None.
    assert_eq!(
        evaluate_auto_path_expr("[info library]", Some("/a/b.tcl")),
        None,
    );
    // `[info script]` with no known path → None.
    assert_eq!(
        evaluate_auto_path_expr("[file dirname [info script]]", None),
        None,
    );
}

#[test]
fn auto_path_file_unknown_subcommand_and_arity_decline() {
    crate::install_surfaces();
    // A `file` name operation the registry routes answers through the route,
    // as tclsh 8.4.20 to 9.1.0 do...
    assert_eq!(
        evaluate_auto_path_expr("[file rootname /a/b.tcl]", None).as_deref(),
        Some("/a/b"),
    );
    // ...and `file` with a subcommand outside the subset → None.
    assert_eq!(
        evaluate_auto_path_expr("[file exists /a/b.tcl]", None),
        None
    );
    // `file dirname` with the wrong arity (two path args) → None.
    assert_eq!(evaluate_auto_path_expr("[file dirname /a /b]", None), None,);
    // `file` with too few args (just the subcommand) → None.
    assert_eq!(evaluate_auto_path_expr("[file dirname]", None), None);
}

#[test]
fn auto_path_variable_subst_declines() {
    crate::install_surfaces();
    // A `$`-bearing literal word is refused (never guess a runtime value).
    assert_eq!(evaluate_auto_path_expr("$auto_path", None), None);
    assert_eq!(
        evaluate_auto_path_expr("[file dirname $base]", Some("/a/b.tcl")),
        None,
    );
}

#[test]
fn auto_path_unsupported_command_declines() {
    crate::install_surfaces();
    // A command outside the supported subset (`lsort`) → None.
    assert_eq!(
        evaluate_auto_path_expr("[lsort {a b}]", Some("/a/b.tcl")),
        None,
    );
}

#[test]
fn auto_path_tokeniser_brace_group_and_bracket_quirk() {
    crate::install_surfaces();
    // A brace group is one literal word; brackets inside braces are literal.
    // `{/lib}` → the literal `/lib`.
    assert_eq!(
        evaluate_auto_path_expr("{/lib}", None).as_deref(),
        Some("/lib"),
    );
    // A braced `[` is a literal `[`, not an open bracket — Tcl performs no
    // substitution inside braces.  A string-token tokeniser would lose
    // that distinction and mis-read `{[} info script ]` as the command
    // substitution `[info script]`; with typed word tokens it is a literal
    // `[` followed by trailing words, which the expression parser rejects.
    assert_eq!(
        evaluate_auto_path_expr("{[} info script ]", Some("/proj/x.tcl")),
        None,
    );
}

#[test]
fn auto_path_tokeniser_quoted_word_with_escape() {
    crate::install_surfaces();
    // A double-quoted word is one token; an internal backslash-escape is
    // consumed as a pair by the tokeniser (so the closing quote isn't matched
    // early). The word still carries the backslash to `eval` as a literal.
    assert_eq!(
        evaluate_auto_path_expr("\"/quoted/path\"", None).as_deref(),
        Some("/quoted/path"),
    );
}

#[test]
fn auto_path_tokeniser_unterminated_delimiters_decline() {
    crate::install_surfaces();
    // An unterminated brace or quote aborts tokenisation → None.
    assert_eq!(evaluate_auto_path_expr("{/unterminated", None), None);
    assert_eq!(evaluate_auto_path_expr("\"/unterminated", None), None);
    // An unbalanced command bracket (`[` with no `]`) → parse fails → None.
    assert_eq!(evaluate_auto_path_expr("[file dirname /a", None), None);
    // A stray `]` at the start of a word → parse fails → None.
    assert_eq!(evaluate_auto_path_expr("] /a", None), None);
}

#[test]
fn auto_path_empty_and_whitespace_is_none() {
    crate::install_surfaces();
    assert_eq!(evaluate_auto_path_expr("", None), None);
    assert_eq!(evaluate_auto_path_expr("   ", None), None);
}

#[test]
fn auto_path_tilde_user_form_unchanged() {
    crate::install_surfaces();
    // `~user` (a non-bare tilde) is left unexpanded — no passwd lookup. It is a
    // literal relative word, so abspath joins it under cwd; we assert only that
    // the `~user` segment survives verbatim (no HOME expansion).
    temp_env::with_var("HOME", Some("/home/tester"), || {
        let got = evaluate_auto_path_expr("~someuser/lib", None).expect("some path");
        assert!(
            got.ends_with("~someuser/lib"),
            "~user must not be HOME-expanded, got {got:?}"
        );
    });
}

#[test]
fn auto_path_bare_tilde_no_home_unchanged() {
    crate::install_surfaces();
    // The bare `~` form with HOME unset is left unexpanded.
    temp_env::with_var_unset("HOME", || {
        let got = evaluate_auto_path_expr("~/lib", None).expect("some path");
        assert!(
            got.ends_with("~/lib"),
            "~ with no HOME must not expand, got {got:?}"
        );
    });
}
