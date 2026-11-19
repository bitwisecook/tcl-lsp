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

//! Regular-expression words and their component sub-tokens, and `regsub` substitutions.

use super::*;

#[test]
fn lsearch_regexp_pattern_uses_the_registry_selected_language() {
    let ks = kinds("lsearch -regexp {a b} {a+}\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
    // The default glob form must not acquire regex subtokens merely
    // because lsearch also supports -regexp.
    let glob = kinds("lsearch {a b} {a+}\n", tcl(), &reg());
    assert!(!glob.contains(&(TokenKind::Regexp as u32)), "{glob:?}");
}

#[test]
fn lsearch_pattern_tokens_abstain_for_invalid_profiled_option_prefixes() {
    // A dash-prefixed word inside lsearch's outer option scan is not a
    // positional list operand when it cannot resolve in that release.
    // In each invalid call the old fallback would put the regex override
    // on `{a+}`; the registry must instead abstain entirely.
    let registry = reg();
    let kinds_at_claimed_pattern = |source: &str, dialect: &str| {
        let claimed_pattern = u32::try_from(source.find("a+").expect("claimed pattern")).unwrap();
        let tokens = full(
            source,
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            &registry,
        );
        let mut line = 0u32;
        let mut column = 0u32;
        tokens
            .data
            .chunks(5)
            .filter_map(|chunk| {
                if chunk[0] > 0 {
                    line += chunk[0];
                    column = chunk[1];
                } else {
                    column += chunk[1];
                }
                (line == 0 && column <= claimed_pattern && claimed_pattern < column + chunk[2])
                    .then_some(chunk[3])
            })
            .collect::<Vec<_>>()
    };

    for (dialect, option) in [("tcl8.6", "-str"), ("tcl9.0", "-st"), ("tcl9.0", "--")] {
        let source = format!("lsearch -regexp {option} {{a+}} {{a b}} x\n");
        let kinds = kinds_at_claimed_pattern(&source, dialect);
        assert!(
            !kinds.contains(&(TokenKind::Regexp as u32)),
            "{dialect} {option} is invalid inside the option scan: {kinds:?}"
        );
    }

    // Tcl 9's unique -str abbreviation does consume its stride value,
    // so the real final pattern remains regex-classified.
    let source = "lsearch -regexp -str 2 {a b} {a+}\n";
    let kinds = kinds_at_claimed_pattern(source, "tcl9.0");
    assert!(
        kinds.contains(&(TokenKind::Regexp as u32)),
        "tcl9.0 must parse -str as -stride: {kinds:?}"
    );
}

#[test]
fn regex_pattern_with_substitution_splits_regex_and_tcl() {
    // `regexp "abc$var.*" $s` — literal `abc` / `.*` sub-tokenise as
    // regex, but `$var` stays a Tcl variable (Tcl resolves it before
    // regexp runs), with no overlap.
    let toks = decode_full("regexp \"abc$var.*\" $s\n", tcl(), &reg());
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
        "expected $var as a variable; got {toks:?}"
    );
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::RegexpQuantifier as u32),
        "expected the `*` quantifier from the literal part; got {toks:?}"
    );
    // No overlaps.
    let simple = decode("regexp \"abc$var.*\" $s\n", tcl(), &reg());
    for w in simple.windows(2) {
        let (l0, c0, len0) = w[0];
        let (l1, c1, _) = w[1];
        if l0 == l1 {
            assert!(c1 >= c0 + len0, "overlap; toks={simple:?}");
        }
    }
    // `regexp "$only" $s` — a bare-substitution pattern is just a
    // variable, not a regex anchor.
    let toks = decode_full("regexp \"$only\" $s\n", tcl(), &reg());
    assert!(
        !toks
            .iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::RegexpAnchor as u32),
        "the `$` must not be a regex anchor; got {toks:?}"
    );
}

#[test]
fn regex_source_variable_highlights_def_site_literal() {
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    // The subject is bound: reading an unset `$s` raises before `regexp`
    // runs, so the pattern would never be used.
    let src = "set my_re \".*abc\"\nset s subject\nregexp $my_re $s\n";
    // Without a CompilationUnit the `set` value is a plain string.
    let plain = decode_full(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
    );
    assert!(
        !plain
            .iter()
            .any(|&(l, _, _, k, _)| l == 0 && k == TokenKind::RegexpQuantifier as u32),
        "no regex tokens without a CU; got {plain:?}"
    );
    // With a CU, the `.*abc` literal at the `set` reads as a regex.
    let cu = CompilationUnit::build_for(src, &registry, false);
    let st = full_with_cu(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
    );
    let toks = decode_semantic(&st);
    assert!(
        toks.iter()
            .any(|&(l, _, _, k, _)| l == 0 && k == TokenKind::RegexpQuantifier as u32),
        "expected the `*` from the def-site literal as a regex quantifier; got {toks:?}"
    );
    // No overlaps introduced.
    for w in toks.windows(2) {
        let (l0, c0, len0, _, _) = w[0];
        let (l1, c1, _, _, _) = w[1];
        if l0 == l1 {
            assert!(c1 >= c0 + len0, "overlap; toks={toks:?}");
        }
    }
}

/// A regex literal bound to a variable is retagged at its definition in
/// every kind of body the unit lowers on its own: a method, a
/// `namespace eval` script and an `apply` lambda. Runs under Tcl 9.0; the
/// rule does not differ between releases.
#[test]
fn regex_source_literal_is_retagged_in_every_body_kind() {
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    for (body, src) in [
        // Rooted calls: TclOO selects the receiver's private namespace at
        // run time, where a relative command may be shadowed.
        (
            "method",
            "oo::class create C {\n  method m {s} {\n    ::set re \".*x\"\n    ::regexp $re $s\n  }\n}\n",
        ),
        // The subject is bound, so the `regexp` call is reached.
        (
            "namespace eval",
            "namespace eval ::ns {\n  set s subject\n  set re \".*x\"\n  regexp $re $s\n}\n",
        ),
        (
            "apply lambda",
            "apply {{s} {\n  set re \".*x\"\n  regexp $re $s\n}} foo\n",
        ),
    ] {
        let cu = CompilationUnit::build_for(src, &registry, false);
        let toks = decode_semantic(&full_with_cu(src, profile, &registry, Some(&cu)));
        assert!(
            toks.iter()
                .any(|&(_, _, _, k, _)| k == TokenKind::RegexpQuantifier as u32),
            "{body}: expected a regex quantifier from the definition literal; got {toks:?}"
        );
        for w in toks.windows(2) {
            let (l0, c0, len0, _, _) = w[0];
            let (l1, c1, _, _, _) = w[1];
            if l0 == l1 {
                assert!(c1 >= c0 + len0, "{body}: overlap; toks={toks:?}");
            }
        }
    }
}

#[test]
fn regexp_pattern_classified_as_regexp() {
    // `regexp {abc} $s` — the `{abc}` pattern argument is `regexp`,
    // not `string`.
    let ks = kinds("regexp {abc} $s\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Regexp as u32)),
        "expected a regexp token; got {ks:?}"
    );
    // `regsub -all {x+} $s y out` — option-skip finds the pattern.
    let ks = kinds("regsub -all {x+} $s y out\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::Regexp as u32)),
        "expected a regexp token after -all; got {ks:?}"
    );
}

/// Every resolver-owned pattern layout needs the same
/// source proof as lsearch. A dynamic leading word can be a value-taking
/// switch at runtime, so it cannot make the later literal a regexp; the
/// analogous regsub query must not steal the replacement as a template.
#[test]
fn dynamic_leading_options_do_not_claim_pattern_or_regsub_replacement_tokens() {
    let registry = reg();
    for src in [
        "regexp $mode {a+} $value\n",
        "regsub $mode {a+} $value {\\1}\n",
    ] {
        let tokens = decode_full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
        );
        assert!(
            !tokens.iter().any(|&(_, _, _, kind, _)| {
                matches!(
                    kind,
                    value
                        if value == TokenKind::Regexp as u32
                            || value == TokenKind::RegexpQuantifier as u32
                )
            }),
            "{src:?}: unresolved leading option must not claim a regex word: {tokens:?}",
        );
    }

    let tokens = decode_full(
        "regsub $mode {a} $value {\\1}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
    );
    assert!(
        !tokens.iter().any(|&(_, _, _, kind, _)| {
            kind == TokenKind::Number as u32 || kind == TokenKind::Operator as u32
        }),
        "an unresolved regsub replacement is ordinary Tcl text: {tokens:?}",
    );

    for src in [
        "regsub -c {a} $value {\\1}\n",
        "regsub -command {a} $value callback\n",
    ] {
        let tokens = decode_full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
        );
        assert!(
            !tokens.iter().any(|&(_, _, _, kind, _)| {
                kind == TokenKind::Number as u32 || kind == TokenKind::Operator as u32
            }),
            "{src:?}: only a valid template layout may receive regsub sub-tokens: {tokens:?}",
        );
    }

    let tokens = decode_full(
        "regsub -start $start {a} $value {\\1}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
    );
    assert!(
        tokens
            .iter()
            .any(|&(_, _, _, kind, _)| kind == TokenKind::Number as u32),
        "a known fixed-width -start option preserves the regsub replacement: {tokens:?}",
    );
}

#[test]
fn regex_pattern_subtokenised_into_components() {
    // `(a+)+` → group `(`, literal `a`, quantifier `+`, group `)`,
    // quantifier `+`.
    let ks = kinds("regexp {(a+)+} $s\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::RegexpGroup as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::RegexpQuantifier as u32)), "{ks:?}");
    // The whole-pattern `regexp` kind is replaced by sub-tokens, but
    // the literal `a` run is still `regexp`.
    assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
}

#[test]
fn regex_char_class_and_anchor_subtokens() {
    let ks = kinds("regexp {^[0-9]+$} $s\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::RegexpCharClass as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::RegexpAnchor as u32)), "{ks:?}");
    assert!(ks.contains(&(TokenKind::RegexpQuantifier as u32)), "{ks:?}");
}

#[test]
fn regex_alternation_and_escape_subtokens() {
    let ks = kinds("regexp {a\\d|b} $s\n", tcl(), &reg());
    assert!(
        ks.contains(&(TokenKind::RegexpAlternation as u32)),
        "{ks:?}"
    );
    // `\d` is an ARE class shortcut → char class.
    assert!(ks.contains(&(TokenKind::RegexpCharClass as u32)), "{ks:?}");
}

#[test]
fn scan_are_class_spans_posix_collating_equivalence_subbrackets() {
    // `[[:alpha:]]` is one bracket expression (the inner `[:alpha:]` is a
    // POSIX class whose `]` does not close the outer bracket) — the scanner
    // must span the whole thing, not stop at the first `]`.
    assert_eq!(scan_are_class(b"[[:alpha:]]", 0), Some(11));
    assert_eq!(scan_are_class(b"[[:digit:]xyz]", 0), Some(14));
    assert_eq!(scan_are_class(b"[[.ch.]]", 0), Some(8));
    assert_eq!(scan_are_class(b"[[=a=]]", 0), Some(7));
    // A plain class is unaffected; a leading literal `]` still works.
    assert_eq!(scan_are_class(b"[a-z]", 0), Some(5));
    assert_eq!(scan_are_class(b"[]a]", 0), Some(4));
    // An unterminated sub-bracket is not a token.
    assert_eq!(scan_are_class(b"[[:alpha", 0), None);
}

#[test]
fn regex_posix_class_has_no_dangling_bracket_token() {
    // Without the sub-bracket rule `[[:alpha:]]+` scans as `[[:alpha:]`
    // (char class) + a stray literal `]` + `+`. The whole `[[:alpha:]]` is
    // one char class and `+` its quantifier — and, per the token-overlap
    // invariant, no token may start inside another.
    let src = "regexp {[[:alpha:]]+} $s\n";
    let toks = decode_full(src, tcl(), &reg());
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::RegexpCharClass as u32),
        "expected a char-class token; got {toks:?}"
    );
    assert!(
        toks.iter()
            .any(|(_, _, _, k, _)| *k == TokenKind::RegexpQuantifier as u32),
        "expected the trailing `+` as a quantifier; got {toks:?}"
    );
    for w in toks.windows(2) {
        let (l0, c0, len0, _, _) = w[0];
        let (l1, c1, _, _, _) = w[1];
        if l0 == l1 {
            assert!(
                c1 >= c0 + len0,
                "token overlap in POSIX class; toks={toks:?}"
            );
        }
    }
}

#[test]
fn regex_without_metachars_stays_single_regexp() {
    // `abc` has no metacharacters → one `regexp` token, no sub-tokens.
    let ks = kinds("regexp {abc} $s\n", tcl(), &reg());
    assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
    assert!(!ks.contains(&(TokenKind::RegexpGroup as u32)), "{ks:?}");
    assert!(
        !ks.contains(&(TokenKind::RegexpQuantifier as u32)),
        "{ks:?}"
    );
}

#[test]
fn classify_regex_component_maps_each_kind() {
    assert_eq!(classify_regex_component("("), TokenKind::RegexpGroup);
    assert_eq!(classify_regex_component("(?:"), TokenKind::RegexpGroup);
    assert_eq!(
        classify_regex_component("[a-z]"),
        TokenKind::RegexpCharClass
    );
    assert_eq!(classify_regex_component("\\d"), TokenKind::RegexpCharClass);
    assert_eq!(classify_regex_component("."), TokenKind::RegexpCharClass);
    assert_eq!(classify_regex_component("+"), TokenKind::RegexpQuantifier);
    assert_eq!(
        classify_regex_component("{2,3}"),
        TokenKind::RegexpQuantifier
    );
    assert_eq!(classify_regex_component("^"), TokenKind::RegexpAnchor);
    assert_eq!(classify_regex_component("\\b"), TokenKind::RegexpAnchor);
    assert_eq!(classify_regex_component("\\n"), TokenKind::RegexpEscape);
    assert_eq!(classify_regex_component("\\3"), TokenKind::RegexpBackref);
    assert_eq!(classify_regex_component("|"), TokenKind::RegexpAlternation);
}

#[test]
fn regsub_replacement_backref_subtokens() {
    // Each Tcl oracle substitutes the first two references in {\1-&-\&-\\1}.
    let ks = kinds(r"regsub {(a)} a {\1-&-\&-\\1} out", tcl(), &reg());
    assert_eq!(
        ks.iter()
            .filter(|&&kind| kind == TokenKind::Number as u32)
            .count(),
        1,
        "{ks:?}"
    );
    assert_eq!(
        ks.iter()
            .filter(|&&kind| kind == TokenKind::Operator as u32)
            .count(),
        1,
        "{ks:?}"
    );
}

#[test]
fn regsub_replacement_without_backrefs_stays_string() {
    let ks = kinds("regsub {a} $s {plain} out\n", tcl(), &reg());
    assert!(!ks.contains(&(TokenKind::Operator as u32)), "{ks:?}");
}
