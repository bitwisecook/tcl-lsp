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

//! Which argument words hold a pattern or format: quoting, shadowing, aliases, nested regions and dynamic options.

use super::*;

#[test]
fn quoted_format_words_use_the_production_hover_path() {
    for (source, needle, expected) in [
        (
            "clock format $time -format \"%Y\"\n",
            "%Y",
            "Clock format string",
        ),
        ("format \"%d\" 42\n", "%d", "Format string"),
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new()
            .analyse(source, "tcl8.6")
            .clone();
        let column = u32::try_from(source.find(needle).expect("format word")).unwrap();
        let hover = hover(source, 0, column, &analysis, None).expect("format hover");
        assert!(hover.value.contains(expected), "{source:?}: {hover:?}");
    }
}

#[test]
fn unrelated_quoted_words_have_no_format_hover() {
    for source in ["puts \"%Y\"\n", "puts \"%d\"\n"] {
        let analysis = tcl_compiler::analyser::Analyser::new()
            .analyse(source, "tcl8.6")
            .clone();
        let column = u32::try_from(source.find('%').expect("quoted word")).unwrap();
        let result = hover(source, 0, column, &analysis, None);
        assert!(
            result
                .as_ref()
                .is_none_or(|hover| !hover.value.contains("Format string")),
            "{source:?}: {result:?}"
        );
    }
}

#[test]
fn hover_abstains_for_local_proc_shadowing_format_command() {
    let src = "proc clock {args} { return 0 }\nclock format $time {%Y}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    assert!(hover(src, 1, 25, &analysis, None).is_none());
}

#[test]
fn hover_abstains_after_static_rename_moves_builtin_away() {
    let src = "rename clock saved\nclock format $time {%Y}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let cursor = u32::try_from(src.find("%Y").unwrap() - src.find('\n').unwrap() - 1).unwrap();
    assert!(hover(src, 1, cursor, &analysis, None).is_none());
}

#[test]
fn hover_uses_effective_registry_identity_after_rename_and_alias() {
    let renamed = "rename format local_format\nlocal_format {%Y}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(renamed, "tcl8.6").clone();
    let cursor =
        u32::try_from(renamed.find("%Y").unwrap() - renamed.find('\n').unwrap() - 1).unwrap();
    let h = hover(renamed, 1, cursor, &analysis, None).expect("renamed format hover");
    assert!(h.value.contains("**Format string**"), "{}", h.value);

    let aliased = "interp alias {} local_format {} format\nlocal_format {%Y}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(aliased, "tcl8.6").clone();
    let cursor =
        u32::try_from(aliased.find("%Y").unwrap() - aliased.find('\n').unwrap() - 1).unwrap();
    let h = hover(aliased, 1, cursor, &analysis, None).expect("aliased format hover");
    assert!(h.value.contains("**Format string**"), "{}", h.value);
}

#[test]
fn pattern_and_format_hover_descend_through_every_registry_executable_region() {
    // The cursor offsets deliberately sit in separately nested source
    // slices.  A line-local or re-based walker finds the command but
    // reports against the wrong source position; all must retain the
    // original document's absolute coordinates.
    let cases = [
        (
            "proc p {} { format {%04d} 1 }\n",
            "%04d",
            "**Format string**",
            "tcl8.6",
        ),
        (
            "oo::class create C { method m {} { string match {*needle*} x } }\n",
            "*needle*",
            "**Glob pattern**",
            "tcl8.6",
        ),
        (
            "switch $x { a { format {%03d} 1 } default { puts no } }\n",
            "%03d",
            "**Format string**",
            "tcl8.6",
        ),
        (
            "switch $x { a \"format {%08d} 1\" default { puts no } }\n",
            "%08d",
            "**Format string**",
            "tcl8.6",
        ),
        (
            "expect { -re {ready} \"format {%09d} 1\" }\n",
            "%09d",
            "**Format string**",
            "expect",
        ),
        (
            "apply {{} {format {%02d} 1}}\n",
            "%02d",
            "**Format string**",
            "tcl8.6",
        ),
        (
            "puts [format {%01d} 1]\n",
            "%01d",
            "**Format string**",
            "tcl8.6",
        ),
        (
            "when HTTP_REQUEST { format {%05d} 1 }\n",
            "%05d",
            "**Format string**",
            "f5-irules",
        ),
    ];
    for (source, needle, expected, dialect) in cases {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, dialect).clone();
        let (line, column) = position_of(source, needle);
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let hover = hover_with_profile(source, line, column, &analysis, None, profile)
            .unwrap_or_else(|| panic!("no hover for {dialect}: {source}"));
        assert!(hover.value.contains(expected), "{}", hover.value);
    }
}

#[test]
fn quoted_case_action_hover_obeys_head_mutations_and_static_list_validation() {
    let aliased = "interp alias {} fmt {} format\nswitch $x { a \"fmt {%10d} 1\" }\n";
    let analysis = analyse(aliased);
    let (line, column) = position_of(aliased, "%10d");
    let result = hover(aliased, line, column, &analysis, None).expect("aliased quoted action");
    assert!(
        result.value.contains("**Format string**"),
        "{}",
        result.value
    );

    for source in [
        "rename format saved\nswitch $x { a \"format {%11d} 1\" }\n",
        "set actions { a \"format {%12d} 1\" }\nswitch $x $actions\n",
        "switch $x { a \"format {%13d} 1\" orphan }\n",
    ] {
        let analysis = analyse(source);
        let (line, column) = position_of(source, "%");
        assert!(
            hover(source, line, column, &analysis, None).is_none(),
            "mutated, dynamic, or malformed case list leaked a nested hover: {source}"
        );
    }
}

#[test]
fn nested_format_hover_honours_aliases_and_shadowing() {
    let aliased = "interp alias {} fmt {} format\nproc p {} { fmt {%06d} 1 }\n";
    let analysis = analyse(aliased);
    let (line, column) = position_of(aliased, "%06d");
    let result = hover(aliased, line, column, &analysis, None).expect("aliased nested format");
    assert!(
        result.value.contains("**Format string**"),
        "{}",
        result.value
    );

    // A same-named proc owns the nested call, so the builtin format
    // grammar must not leak through merely because its spelling matches.
    let shadowed = "proc format {args} {}\nproc p {} { format {%07d} 1 }\n";
    let analysis = analyse(shadowed);
    let (line, column) = position_of(shadowed, "%07d");
    assert!(hover(shadowed, line, column, &analysis, None).is_none());
}

#[test]
fn nested_substitution_hover_beats_its_outer_pattern_word() {
    let source = "regexp \"[format {%d} 1]\" $value\n";
    let analysis = analyse(source);
    let (line, column) = position_of(source, "%d");
    let result = hover(source, line, column, &analysis, None)
        .expect("format hover inside regexp substitution");
    assert!(
        result.value.contains("**Format string**"),
        "{}",
        result.value
    );
    assert!(
        !result.value.contains("**Regular expression**"),
        "outer regexp incorrectly claimed nested format: {}",
        result.value
    );
}

#[test]
fn hover_uses_lsearchs_registry_selected_pattern_language() {
    // `-regexp` changes lsearch's embedded language, while an exact or
    // abbreviated option still shifts list/pattern positions through the
    // registry's OptionSpec grammar.
    let src = "lsearch -regexp {a b} {a+}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let h = hover(src, 0, 24, &analysis, None).expect("hover");
    assert!(h.value.contains("Regex pattern"), "{}", h.value);
}

#[test]
fn dynamic_lsearch_option_prefix_does_not_claim_the_following_list_as_glob() {
    let src = "lsearch $mode {a b} {a*}\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    // `$mode` can evaluate to a value-taking lsearch option. The source
    // shape therefore cannot prove that `{a b}` is the pattern word.
    assert!(hover(src, 0, 16, &analysis, None).is_none());
}

/// A substituted leading word is not a positional operand until Tcl has
/// evaluated it.  The source-aware registry query must therefore keep
/// every resolver-owned pattern and regsub replacement unclaimed, not
/// merely the lsearch-specific descriptor.
#[test]
fn dynamic_leading_options_abstain_for_pattern_and_regsub_format_hovers() {
    let registry = tcl_registry::CommandRegistry::build_default();
    for (src, needle) in [
        ("regexp $mode {a+} $value\n", "a+"),
        ("glob $mode /tmp {*.tcl}\n", "*.tcl"),
        ("regsub $mode {a} $value {\\1}\n", "\\1"),
        ("regsub -c {a} $value {\\1}\n", "\\1"),
        ("regsub -command {a} $value callback\n", "callback"),
    ] {
        let analysis = analyse(src);
        let (line, character) = position_of(src, needle);
        assert!(
            !hover(src, line, character, &analysis, Some(&registry)).is_some_and(|found| {
                found.value.contains("Regex pattern")
                    || found.value.contains("Glob pattern")
                    || found.value.contains("Substitution spec")
            }),
            "{src:?}: an unresolved, invalid, or callback-leading layout must not claim {needle:?} as an embedded language",
        );
    }

    let src = "regsub -start $start {a} $value {\\1}\n";
    let analysis = analyse(src);
    let (line, character) = position_of(src, "\\1");
    let found = hover(src, line, character, &analysis, Some(&registry)).expect("hover");
    assert!(
        found.value.contains("Substitution spec"),
        "a declared -start value has a fixed width: {}",
        found.value
    );
}

#[test]
fn hover_explains_a_computed_format_string() {
    // A literal `format "%-20s %d" a 1` already explains its
    // specifiers; the lattice proves the same text when it reaches the
    // word through a variable, so the computed spelling must not go
    // blind.
    let registry = tcl_registry::CommandRegistry::build_default();
    let src = "set fmt \"%-20s %d\"\nformat $fmt a 1\n";
    let analysis = analyse(src);
    let (line, character) = position_of(src, "$fmt");
    let found = hover(src, line, character, &analysis, Some(&registry)).expect("hover");
    assert!(
        found.value.contains("**Format string** (sprintf-style)"),
        "{}",
        found.value
    );
    assert!(
        found.value.contains("| `%d` | Signed decimal integer |"),
        "{}",
        found.value
    );

    // Negative: an unknown `$fmt` (a bare parameter, never assigned a
    // literal) proves nothing, so the plain variable card shows with no
    // format-string section appended.
    let src = "proc p {fmt} {\n    format $fmt a 1\n}\n";
    let analysis = analyse(src);
    let (line, character) = position_of(src, "$fmt");
    let found = hover(src, line, character, &analysis, Some(&registry)).expect("hover");
    assert!(
        found.value.contains("**Variable** `fmt`"),
        "{}",
        found.value
    );
    assert!(
        !found.value.contains("Format string"),
        "an unproven $fmt must not draw a format-string hover: {}",
        found.value
    );
}

#[test]
fn lsearch_pattern_hover_uses_the_documents_profiled_option_set() {
    // Tcl 9 makes -str the unique -stride abbreviation. Before 9 it is
    // not an lsearch option, so the preceding -regexp cannot claim the
    // final word as a regex pattern. This reaches the profile-aware
    // PatternArgResolver path, not a command-name hover branch.
    let src = "lsearch -regexp -str 2 {a b} {a+}\n";
    let cursor = u32::try_from(src.rfind("a+").expect("final pattern")).unwrap();
    for (dialect, expected) in [("tcl8.6", false), ("tcl9.0", true)] {
        let mut analyser = tcl_compiler::analyser::Analyser::new();
        let analysis = analyser.analyse(src, dialect).clone();
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let found = hover_with_profile(
            src,
            0,
            cursor,
            &analysis,
            Some(&tcl_registry::CommandRegistry::build_default()),
            profile,
        );
        assert_eq!(
            found.is_some_and(|hover| hover.value.contains("Regex pattern")),
            expected,
            "{dialect}: profile-filtered lsearch options decide the pattern position"
        );
    }
}

#[test]
fn conditional_pattern_hover_keeps_source_and_method_owners() {
    for subject in ["$line", "line"] {
        let source = format!("regexp {{^foo.*$}} {subject}\n");
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl8.6");
        let selected = hover(&source, 0, 10, &analysis, None).expect("original regex pattern");
        assert!(
            selected.value.contains("Regex pattern"),
            "{}",
            selected.value
        );
        let changed = source.replace("^foo", "^bar");
        assert!(
            hover(&changed, 0, 10, &analysis, None).is_none(),
            "foreign source"
        );
    }
    for source in [
        "proc regexp {args} { return 0 }\nregexp {^foo.*$} $line\n",
        "rename regexp {}\noo::class create regexp {}\nregexp {^foo.*$} $line\n",
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let line = u32::try_from(source.lines().count() - 1).unwrap();
        assert!(
            hover(source, line, 10, &analysis, None)
                .is_none_or(|h| !h.value.contains("Regex pattern")),
            "shadowed command: {source}"
        );
    }
    let source = "oo::class create C {}\nC {^foo.*$} $line\n";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(
        hover(source, 1, 5, &analysis, None).is_none_or(|h| !h.value.contains("Regex pattern")),
        "OO selector is not a regexp source pattern"
    );
    let unresolved = "oo::class create C {method m {} {}}\nset obj unknown\n$obj m\n";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(unresolved, "tcl8.6");
    assert!(
        matches!(
            crate::method_symbol::local_candidate(unresolved, &analysis, 2, 5),
            std::ops::ControlFlow::Break(None)
        ),
        "real unknown object selector retains its method refusal"
    );
    assert!(
        hover(unresolved, 2, 5, &analysis, None).is_none(),
        "an unresolved OO selector has no source pattern fallback"
    );
}

#[test]
fn hover_abstains_for_local_proc_shadowing_regexp_command() {
    let src = "proc regexp {args} { return 0 }\nregexp {^foo$} $line\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    assert!(hover(src, 1, 10, &analysis, None).is_none());
}
