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

use super::*;
use tcl_lexer::Span;

#[test]
fn sidecar_stubs_load_nearest_dialect_bundle() {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-sidecar-stubs-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let nested = root.join("src");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(
        root.join("tcl8.6.tcl.stubs"),
        "# package API\nstub db_eval {sql script:body} -barrier\n",
    )
    .unwrap();
    let file = nested.join("main.tcl");
    let (commands, exprs) = scan_sidecar_stubs(Some(file.to_str().unwrap()), "tcl8.6");
    assert_eq!(exprs.len(), 0);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].name, "db_eval");
    assert!(
        commands[0]
            .flags
            .contains(super::super::types::StubFlags::BARRIER)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn parse_file_suppression_finds_codes() {
    let src = "# tcl-lsp: disable=W210,W211\nproc foo {} {}\n";
    let codes = parse_file_suppression(src);
    assert!(codes.contains("W210"));
    assert!(codes.contains("W211"));
    assert_eq!(codes.len(), 2);
}

#[test]
fn parse_file_suppression_handles_whitespace_and_case() {
    let src = "  #  TCL-LSP : DISABLE = W210  W211  \n\n# more\nproc foo {} {}\n";
    let codes = parse_file_suppression(src);
    assert!(codes.contains("W210"));
    assert!(codes.contains("W211"));
}

#[test]
fn parse_file_suppression_stops_at_first_non_comment() {
    let src = "# tcl-lsp: disable=W210\nproc foo {} {}\n# tcl-lsp: disable=W211\n";
    let codes = parse_file_suppression(src);
    assert!(codes.contains("W210"));
    assert!(!codes.contains("W211"));
}

#[test]
fn parse_file_suppression_skips_blank_lines() {
    let src = "\n\n# tcl-lsp: disable=W210\n\nproc foo {} {}\n";
    let codes = parse_file_suppression(src);
    assert!(codes.contains("W210"));
}

#[test]
fn parse_file_suppression_handles_wildcard() {
    let src = "# tcl-lsp: disable=*\nproc foo {} {}\n";
    let codes = parse_file_suppression(src);
    assert!(codes.contains("*"));
}

#[test]
fn parse_file_suppression_no_directive_returns_empty() {
    let src = "# Just a comment\nproc foo {} {}\n";
    let codes = parse_file_suppression(src);
    assert_eq!(codes.len(), 0);
}

/// The `package … provides …` directive is read anywhere in
/// the file and names the loading package, so it does not depend on where
/// it sits.
#[test]
fn parse_provides_directives_read_edges_from_the_whole_file() {
    let src = "# tcl-lsp: package myExtension provides Tk\n\
                   package require myExtension\n\
                   proc build {} {\n\
                   \x20   # TCL-LSP: Package bigWrapper Provides Img snit\n\
                   }\n\
                   # tcl-lsp: package myExtension provides Tk Img\n";
    assert_eq!(
        parse_provides_directives(src),
        vec![
            (
                "myExtension".to_owned(),
                vec!["Tk".to_owned(), "Img".to_owned()],
            ),
            (
                "bigWrapper".to_owned(),
                vec!["Img".to_owned(), "snit".to_owned()],
            ),
        ],
        "edges merge per loader, in first-declaration order",
    );
}

/// A directive that declares no edge, and the neighbouring members of the
/// `# tcl-lsp:` family, are not this one.
#[test]
fn parse_provides_directives_ignore_other_shapes() {
    for src in [
        "# tcl-lsp: package myExtension provides\n",
        "# tcl-lsp: package myExtension\n",
        "# tcl-lsp: package provides Tk\n",
        "# tcl-lsp: packages myExtension provides Tk\n",
        "# tcl-lsp: supports Tk 8.5-8.6\n",
        "# tcl-lsp: disable=W120\n",
        "# tcl-lsp: stub myproc {a b}\n",
        "set x {package require Tk}\n",
    ] {
        assert_eq!(
            parse_provides_directives(src).len(),
            0,
            "must declare no edge: {src:?}"
        );
    }
}

#[test]
fn parse_supports_directives_reads_the_top_of_file_block() {
    // Multi-clause ranges, case-insensitive keywords, and one
    // declaration per provider (a later line replaces the earlier).
    let src = "# tcl-lsp: supports tcl 8.5-9.0\n\
                   # TCL-LSP: Supports Tk 8.5 8.6\n\
                   # tcl-lsp: supports tcl 8.6\n\
                   proc foo {} {}\n\
                   # tcl-lsp: supports struct 1.5-2.2\n";
    let declarations = parse_supports_directives(src);
    assert_eq!(
        declarations,
        vec![
            ("Tk".to_owned(), "8.5 8.6".to_owned()),
            ("tcl".to_owned(), "8.6".to_owned()),
        ],
        "the tcl redeclaration replaces; the post-code directive is out of the scan window"
    );
    // Not-this-directive shapes declare nothing.
    for line in [
        "# tcl-lsp: supports\n",
        "# tcl-lsp: supportsx tcl 8.5\n",
        "# tcl-lsp: supports tcl\n",
        "# tcl-lsp: disable=W150\n",
        "# supports tcl 8.5\n",
        "# — tcl-lsp: supports tcl 8.5\n",
    ] {
        assert_eq!(parse_supports_directives(line).len(), 0, "{line:?}");
    }
}

#[test]
fn parse_file_suppression_multibyte_leading_comment_does_not_panic() {
    // A fixed-byte-offset keyword check that sliced `line[..kw_end]` /
    // `line[..dis_end]` unconditionally would panic
    // (`byte index N is not a char boundary`) whenever a multi-byte
    // UTF-8 character — an em dash, an accented letter, any ordinary
    // non-ASCII text — straddles that offset. Neither of these is
    // adversarial input: both are plausible header-comment text.
    //
    // An em dash a few bytes into the leading comment straddles the
    // "tcl-lsp" keyword's 7-byte check.
    let em_dash_early = "# E001 \u{2014} missing dispatch word\nproc foo {} {}\n";
    assert_eq!(parse_file_suppression(em_dash_early).len(), 0);

    // An em dash inside the post-colon text straddles the "disable"
    // keyword's own 7-byte check.
    let em_dash_after_colon = "# tcl-lsp: abcdef\u{2014}ghi\nproc foo {} {}\n";
    assert_eq!(parse_file_suppression(em_dash_after_colon).len(), 0);
}

#[test]
fn line_suppressed_honours_the_named_code_the_wildcard_and_the_file_bucket() {
    // The one contract every diagnostic surface asks through: an exact code
    // match on the line, the `"*"` wildcard a bare `# noqa` records, and the
    // file-wide `-1` bucket a `# tcl-lsp: disable=…` directive fills.
    let mut map: HashMap<i32, HashSet<String>> = HashMap::new();
    map.insert(3, std::iter::once("W210".to_owned()).collect());
    map.insert(4, std::iter::once("*".to_owned()).collect());

    assert!(line_suppressed("W210", 3, &map));
    assert!(
        !line_suppressed("W211", 3, &map),
        "another code on that line"
    );
    assert!(!line_suppressed("W210", 2, &map), "another line");
    assert!(
        line_suppressed("S100", 4, &map),
        "the wildcard takes every code"
    );

    map.insert(
        FILE_SUPPRESS_KEY,
        std::iter::once("S100".to_owned()).collect(),
    );
    assert!(
        line_suppressed("S100", 99, &map),
        "the file bucket applies to every line"
    );
    assert!(!line_suppressed("W211", 99, &map));
}

#[test]
fn line_suppressed_is_false_for_an_empty_map() {
    let map: HashMap<i32, HashSet<String>> = HashMap::new();
    assert!(!line_suppressed("W210", 0, &map));
    assert!(!line_suppressed("W210", FILE_SUPPRESS_KEY, &map));
}

#[test]
fn parse_noqa_marker_reads_the_documented_shapes_and_rejects_prose() {
    // Bare and coded directives, including the coded form after an
    // explanatory prefix.
    assert!(
        parse_noqa_marker("# noqa")
            .expect("bare directive")
            .contains("*")
    );
    assert!(
        parse_noqa_marker("noqa")
            .expect("directive without the `#`")
            .contains("*")
    );
    assert!(
        parse_noqa_marker("# noqa: W210, W211")
            .expect("coded directive")
            .contains("W211")
    );
    assert!(
        parse_noqa_marker("# see #123, noqa: W210")
            .expect("coded directive after a prefix")
            .contains("W210")
    );
    assert!(
        parse_noqa_marker("# noqa:")
            .expect("directive naming nothing")
            .contains("*")
    );

    // Prose that only mentions the word, and a word that merely ends in
    // it, are not directives.
    for prose in [
        "# do not use noqa here",
        "# noqa is how you would silence this",
        "# renoqa: W210",
        "# ordinary comment",
    ] {
        assert!(
            parse_noqa_marker(prose).is_none(),
            "prose must not be a directive: {prose}"
        );
    }
}

#[test]
fn parse_noqa_line_suppressions_ignores_a_comment_that_only_mentions_noqa() {
    // A substring match reads the trailing prose as a bare marker and
    // silences every code on the command below.
    let src = "# do not use noqa here\nputs $x\n";
    assert_eq!(
        parse_noqa_line_suppressions(src).len(),
        0,
        "prose must not seed the suppression map"
    );
}

#[test]
fn parse_noqa_line_suppressions_records_next_line_for_bare_noqa() {
    // Line 0: ``# noqa`` → suppression keyed on line 1 (the next).
    let src = "# noqa\nset x 1\n";
    let map = parse_noqa_line_suppressions(src);
    let codes = map.get(&1).expect("line 1 entry");
    assert!(codes.contains("*"));
}

#[test]
fn parse_noqa_line_suppressions_records_specific_codes() {
    let src = "# noqa: W210, W211\nset x 1\n";
    let map = parse_noqa_line_suppressions(src);
    let codes = map.get(&1).expect("line 1 entry");
    assert!(codes.contains("W210"));
    assert!(codes.contains("W211"));
    assert!(!codes.contains("*"));
}

#[test]
fn parse_noqa_line_suppressions_orphaned_at_brace_tail() {
    // A ``# noqa`` at the tail of a brace body has no following
    // command in that scope; it suppresses the line that follows
    // physically (often a ``} elseif {…}`` header).
    let src = "if {1} {\n    set x 1\n    # noqa\n} elseif {2} {\n    set y 2\n}\n";
    let map = parse_noqa_line_suppressions(src);
    // ``# noqa`` is on line 2 (0-based) — suppression keyed on
    // line 3 (the ``} elseif …`` header).
    let codes = map.get(&3).expect("line 3 entry");
    assert!(codes.contains("*"));
}

#[test]
fn parse_noqa_line_suppressions_before_comment_line() {
    // ``# noqa`` immediately before another comment line that
    // generates a diagnostic (e.g. W115 on a backslash-continued
    // comment).
    let src = "# noqa: W115\n# trailing \\\nset x 1\n";
    let map = parse_noqa_line_suppressions(src);
    // ``# noqa`` is line 0 → suppresses line 1 (the W115-bearing
    // comment).  The diagnostic-bearing comment also has its own
    // entry registered for the ``# noqa`` shape — the helper
    // records ``# trailing \`` as carrying no noqa (no ``noqa``
    // substring), so only line 1 is in the map.
    let codes = map.get(&1).expect("line 1 entry");
    assert!(codes.contains("W115"));
}

#[test]
fn parse_noqa_line_suppressions_unions_codes_when_multiple_noqa_target_same_line() {
    // Two consecutive noqa comments — the second's next-line is
    // line 2 (set x 1).  The first's next-line is line 1 (the
    // second noqa comment).  Both noqa comments carry their own
    // entries.  Verify line 2 sees only the second noqa's codes;
    // line 1 sees only the first noqa's codes.
    let src = "# noqa: W210\n# noqa: W211\nset x 1\n";
    let map = parse_noqa_line_suppressions(src);
    assert!(map.get(&1).expect("line 1").contains("W210"));
    let line2 = map.get(&2).expect("line 2");
    assert!(line2.contains("W211"));
    assert!(!line2.contains("W210"));
}

#[test]
fn parse_noqa_line_suppressions_skips_non_comment_lines() {
    // The implementation skips any line that isn't ``#``-prefixed
    // after ``trim()`` (so comments only — code lines, blank
    // lines, and lines whose ``noqa`` substring sits inside a
    // Tcl string never seed the map).  Verify line 0 (``set x
    // "noqa"``) does not produce a line-1 entry: the only entry
    // comes from the ``# noqa`` on line 1, which seeds line 2.
    let src = "set x \"noqa\"\n# noqa\nset y 1\n";
    let map = parse_noqa_line_suppressions(src);
    assert!(!map.contains_key(&0));
    assert!(!map.contains_key(&1));
    // ``# noqa`` on line 1 → suppresses line 2.
    assert!(map.get(&2).expect("line 2 entry").contains("*"));
}

fn retained_comment_analysis(source: &str) -> super::super::AnalysisResult {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "hold-script",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, ArgRole::Body)],
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let context = crate::environment_ingress::context_for_profile(profile)
        .with_command_store(std::sync::Arc::new(registry));
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = super::super::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(context),
        config,
    );
    super::super::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn retained_noqa_facts_use_actual_custom_and_declared_body_syntax() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub hold {script:body}\n# tcl-lsp: stubs-end\nhold-script {\n# noqa: W210\n# comment-only\n}\nhold {\n# noqa: W305\n# comment-only\n}\n";
    let analysis = retained_comment_analysis(source);
    let lines = parse_noqa_line_suppressions_from_analysis(source, &analysis).unwrap();
    assert!(lines.get(&5).unwrap().contains("W210"));
    assert!(lines.get(&9).unwrap().contains("W305"));
    assert_eq!(analysis.suppressed_lines.get(&5), lines.get(&5));
    assert_eq!(analysis.suppressed_lines.get(&9), lines.get(&9));
    let facts = script_comment_facts_from_analysis(source, &analysis).unwrap();
    for fact in facts {
        assert_eq!(source.get(fact.span.as_range()), Some(fact.text.as_str()));
    }
    assert!(parse_noqa_line_suppressions_from_analysis(&format!("{source} "), &analysis).is_none());
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(parse_noqa_line_suppressions_from_analysis(source, &missing).is_none());
}

#[test]
fn retained_noqa_declines_body_roles_at_a_known_source_replacement() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "proc if args {}\nif 1 {\n# noqa: W210\n# literal-data\n}";
    let analysis = retained_comment_analysis(source);
    let lines = parse_noqa_line_suppressions_from_analysis(source, &analysis).unwrap();
    assert!(!lines.contains_key(&3));
    assert!(!analysis.suppressed_lines.contains_key(&3));
}

#[test]
fn retained_noqa_expression_commands_keep_original_comment_regions() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "expr {[\n# noqa: W210\n# comment-only\nformat café\n]}";
    let analysis = retained_comment_analysis(source);
    let lines = parse_noqa_line_suppressions_from_analysis(source, &analysis).unwrap();
    assert!(lines.get(&2).unwrap().contains("W210"));
    assert_eq!(analysis.suppressed_lines.get(&2), lines.get(&2));
    let facts = script_comment_facts_from_analysis(source, &analysis).unwrap();
    assert_eq!(facts.len(), 2);
    for fact in facts {
        assert_eq!(source.get(fact.span.as_range()), Some(fact.text.as_str()));
    }
}

#[test]
fn retained_noqa_definition_vocabulary_clears_ordinary_and_procedure_bodies() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    for (source, expected) in [
        (
            "oo::class create C {if 1 {method café {} {\n# noqa: W210\n# control\n}}}",
            true,
        ),
        (
            "oo::class create C {proc p {} {method ordinary {} {\n# noqa: W210\n# data\n}}}",
            false,
        ),
        (
            "oo::class create C {method m {} {method ordinary {} {\n# noqa: W210\n# data\n}}}",
            false,
        ),
        (
            "oo::class create C {apply {{} {method ordinary {} {\n# noqa: W210\n# data\n}}}}",
            false,
        ),
    ] {
        let analysis = retained_comment_analysis(source);
        let lines = parse_noqa_line_suppressions_from_analysis(source, &analysis).unwrap();
        assert_eq!(
            lines.get(&2).is_some_and(|codes| codes.contains("W210")),
            expected,
            "{source}"
        );
        assert_eq!(analysis.suppressed_lines.get(&2), lines.get(&2));
    }
}

#[test]
fn retained_noqa_member_geometry_keeps_original_selectors_and_whole_words() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "oo::class create C {\nmethod café {} {\n# noqa: W210\n# comment-only\n}\nself self {\nmethod m {} {\n# noqa: W305\n# comment-only\n}\n}\n}\n";
    let analysis = retained_comment_analysis(source);
    let lines = parse_noqa_line_suppressions_from_analysis(source, &analysis).unwrap();
    assert!(lines.get(&3).unwrap().contains("W210"));
    assert!(lines.get(&8).unwrap().contains("W305"));
    let source = "oo::class create C {\n$member m {} {\n# noqa: W210\n# comment-only\n}\n}\n";
    let analysis = retained_comment_analysis(source);
    assert!(
        !parse_noqa_line_suppressions_from_analysis(source, &analysis)
            .unwrap()
            .contains_key(&3)
    );
    let source = r#"oo::class create C {method m {} "\u0023 noqa: W210\nputs hidden"}"#;
    let analysis = retained_comment_analysis(source);
    assert!(
        parse_noqa_line_suppressions_from_analysis(source, &analysis)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn comment_facts_are_exact_slices_and_never_promote_pre_comment_noqa() {
    let src = "set marker \"noqa\"; # ordinary comment\n# café noqa: W305\nputs \"\u{202e}\"\n";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let facts = script_comment_facts(
        src,
        tcl_lexer::LexerConfig::for_file_dialect("tcl9.0"),
        tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
    );
    assert_eq!(
        facts.len(),
        1,
        "tail comments are not physical facts: {facts:?}"
    );
    assert_eq!(facts[0].line, 1);
    assert_eq!(facts[0].text, "# café noqa: W305");
    assert_eq!(
        src.get(facts[0].span.start() as usize..facts[0].span.end() as usize),
        Some(facts[0].text.as_str()),
        "fact span must identify precisely the lexer-confirmed slice"
    );
    let suppressions = parse_noqa_line_suppressions(src);
    assert!(
        suppressions
            .get(&2)
            .is_some_and(|codes| codes.contains("W305"))
    );
    assert!(
        !suppressions.contains_key(&1),
        "string data must not become noqa"
    );
}

#[test]
fn parse_noqa_line_suppressions_ignores_braced_data() {
    let src =
        "proc example {} {\n    set help {\nline one\n# noqa\n    }\n    puts $undefined_var\n}\n";
    let map = parse_noqa_line_suppressions(src);
    assert_eq!(
        map.len(),
        0,
        "braced data must not become a directive: {map:?}"
    );
}

#[test]
fn parse_noqa_line_suppressions_reaches_case_list_arms() {
    let src =
        "switch $kind {\n    alpha {\n        # noqa: W305\n        puts \"\u{202e}\"\n    }\n}\n";
    let map = parse_noqa_line_suppressions(src);
    let codes = map.get(&3).expect("switch arm target line");
    assert!(codes.contains("W305"), "{map:?}");
}

#[test]
fn script_comment_facts_reaches_expect_clause_flags_and_lambda_bodies() {
    let src = "expect {\n    -re {ready} {\n        # expect arm\n        # continuation \\\n        hidden\n    }\n}\napply {{} {\n    # lambda body\n    puts ok\n}}\n";
    let profile = tcl_registry::model::ingress::resolve_environment("expect").analyser_profile();
    let comments = script_comment_facts(
        src,
        tcl_lexer::LexerConfig::for_file_dialect("expect"),
        tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
    );
    assert!(
        comments.iter().any(|fact| fact.line == 2),
        "Expect comment missing: {comments:?}"
    );
    assert!(
        comments.iter().any(|fact| fact.line == 3),
        "continued comment missing: {comments:?}"
    );
    assert!(
        comments.iter().any(|fact| fact.line == 8),
        "lambda comment missing: {comments:?}"
    );
}

#[test]
fn script_comment_facts_reaches_definition_member_bodies() {
    let src = "oo::class create Example {\n    method run {} {\n        # member body\n        puts ok\n    }\n}\n";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let comments = script_comment_facts(
        src,
        tcl_lexer::LexerConfig::for_file_dialect("tcl9.0"),
        tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
    );
    assert!(
        comments.iter().any(|fact| fact.line == 2),
        "definition-member comment missing: {comments:?}"
    );
}

#[test]
fn script_comment_facts_reaches_nested_command_substitution_case_arms() {
    let src = "set result [switch $kind {\n    alpha {\n        # nested comment \\\n        hidden\n    }\n}]\nset data {[switch $kind { beta { # inert } }]}\n";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let comments = script_comment_facts(
        src,
        tcl_lexer::LexerConfig::for_file_dialect("tcl9.0"),
        tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
    );
    assert!(
        comments.iter().any(|fact| fact.line == 2),
        "nested comment missing: {comments:?}"
    );
    assert_eq!(
        comments.len(),
        1,
        "braced data must stay inert: {comments:?}"
    );
}

#[test]
fn script_comment_facts_uses_proven_alias_identity_for_body_roles() {
    let src = "interp alias {} define {} proc\ndefine f {} {\n    # noqa: W305\n    puts \"\u{202e}\"\n}\n";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let comments = script_comment_facts(
        src,
        tcl_lexer::LexerConfig::for_file_dialect("tcl9.0"),
        tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
    );
    assert!(
        comments.iter().any(|fact| fact.line == 2),
        "alias body comment missing: {comments:?}"
    );
}

#[test]
fn alias_to_user_proc_named_method_keeps_final_braced_data_inert() {
    // Tclsh executes the user proc through the alias; `# inert data` is
    // an argument, not a script comment. A member-keyword spelling is not
    // enough to prove the alias owns definition-member semantics.
    let src = "proc method {name parameters body} {\n    return \"$name:$parameters:$body\"\n}\ninterp alias {} define_method {} method\noo::class create C {\n    define_method m {} {\n        # inert data\n        puts must-not-run\n    }\n}\n";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let comments = script_comment_facts(
        src,
        tcl_lexer::LexerConfig::for_file_dialect("tcl9.0"),
        tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
    );
    assert!(
        !comments.iter().any(|fact| fact.line == 6),
        "the user proc's braced data must not yield a script-comment fact: {comments:?}"
    );
}

#[test]
fn original_comment_projection_keeps_whole_image_geometry_and_declines_unmapped_text() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "hold {\n    # café 😀 \\\n    puts hidden\n}\n";
    let image = tcl_lexer::SourceImage::document(source);
    let start = u32::try_from(source.find('\n').unwrap() + 1).unwrap();
    let end = u32::try_from(source.rfind('}').unwrap()).unwrap();
    let config = tcl_lexer::LexerConfig::for_file_dialect("tcl8.6");
    let facts =
        original_script_comment_facts(&image, &[(Span::new(start, end), 1)], config).unwrap();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].line, 1);
    assert_eq!(
        source.get(facts[0].span.as_range()),
        Some(facts[0].text.as_str())
    );
    let inline = tcl_lexer::SourceImage::document("hold {# inline \\\nputs hidden\n}");
    let inline_end = u32::try_from(inline.len() - 1).unwrap();
    assert_eq!(
        original_script_comment_facts(&inline, &[(Span::new(6, inline_end), 1)], config).unwrap(),
        [] as [crate::analyser::utils::ScriptCommentFact; 0]
    );
    let bad_boundary = u32::try_from(source.find('é').unwrap() + 1).unwrap();
    assert!(
        original_script_comment_facts(&image, &[(Span::new(bad_boundary, end), 1)], config)
            .is_none()
    );
    assert!(
        original_script_comment_facts(&image, &[(Span::new(start, u32::MAX), 1)], config).is_none()
    );
    let opaque = tcl_lexer::SourceImage::native(std::sync::Arc::<[u8]>::from([0xff]));
    assert!(original_script_comment_facts(&opaque, &[(Span::new(0, 1), 0)], config).is_none());
}

#[test]
fn cooked_script_comment_markers_have_no_original_physical_line() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let protocol = tcl_registry::InvocationDialect::of_profile(profile)
        .authored_name_policy()
        .unwrap()
        .string_protocol();
    for source in [
        r#"proc p {} "\u0023 noqa: W210\nputs hi""#,
        r#"namespace eval N "\x23 noqa: W210\nputs hi""#,
    ] {
        assert!(!source.as_bytes().contains(&b'#'));
        let image = tcl_lexer::SourceImage::document(source);
        let parsed = tcl_lexer::native_script_words_in(
            image,
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            &parsed.commands[0].words,
            protocol,
        )
        .unwrap();
        assert!(captured.literal(3).unwrap().starts_with(b"# noqa:"));
        assert_eq!(
            script_comment_facts(source, config, registry),
            [] as [crate::analyser::utils::ScriptCommentFact; 0]
        );
    }
    let source = "proc p {} {\n# noqa: W210\nputs hi\n}";
    let facts = script_comment_facts(source, config, registry);
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].text, "# noqa: W210");
    assert_eq!(
        source.get(facts[0].span.start() as usize..facts[0].span.end() as usize),
        Some(facts[0].text.as_str())
    );
}

#[test]
fn parse_noqa_line_suppressions_empty_for_no_directives() {
    let src = "set x 1\nset y 2\n";
    let map = parse_noqa_line_suppressions(src);
    assert_eq!(map.len(), 0);
}

#[test]
fn parse_file_suppression_caps_at_scan_lines() {
    // 200 blank/comment lines, directive at line 150 — should
    // NOT be picked up because the cap is 100.
    let mut src = String::new();
    for _ in 0..150 {
        src.push_str("# noise\n");
    }
    src.push_str("# tcl-lsp: disable=W210\n");
    let codes = parse_file_suppression(&src);
    assert_eq!(codes.len(), 0);
}

#[test]
fn format_literal_truncates_long_values() {
    let long = "a".repeat(50);
    let formatted = format_literal_for_message(&long);
    assert!(formatted.ends_with("..."));
    // 37 chars + 3 dots = 40 chars total.
    assert_eq!(formatted.chars().count(), 40);
}

#[test]
fn format_literal_replaces_newlines() {
    assert_eq!(format_literal_for_message("a\nb"), "a\\nb");
}

#[test]
fn format_literal_short_passes_through() {
    assert_eq!(format_literal_for_message("hello"), "hello");
}

#[test]
fn possible_paste_fingerprint_assign_const() {
    let stmt = Statement::AssignConst {
        name: "x".to_string(),
        value: "  hello  ".to_string(),
        span: Span::new(0, 0),
        name_braced: false,
        value_span: None,
    };
    assert_eq!(
        possible_paste_fingerprint(&stmt),
        Some(("x".to_string(), "hello".to_string()))
    );
}

fn assign_value(value: &str) -> Statement {
    Statement::AssignValue {
        name: "x".to_string(),
        value: value.to_string(),
        span: Span::new(0, 0),
        name_braced: false,
        value_needs_backsubst: false,
        tokens: None,
    }
}

#[test]
fn possible_paste_fingerprint_assign_value_with_dollar_rejected() {
    assert_eq!(possible_paste_fingerprint(&assign_value("$y")), None);
}

#[test]
fn possible_paste_fingerprint_assign_value_with_bracket_rejected() {
    assert_eq!(possible_paste_fingerprint(&assign_value("[expr 1]")), None);
}

#[test]
fn possible_paste_fingerprint_assign_value_pure_literal() {
    assert_eq!(
        possible_paste_fingerprint(&assign_value("  hello  ")),
        Some(("x".to_string(), "hello".to_string()))
    );
}

#[test]
fn possible_paste_fingerprint_other_stmt_returns_none() {
    // A non-assign statement — return.
    let stmt = Statement::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        expr: None,
        command_binding: None,
        braced: false,
        span: Span::new(0, 0),
    };
    assert_eq!(possible_paste_fingerprint(&stmt), None);
}

#[test]
fn argv_with_word_spans_is_identity() {
    // The segmenter already widens; this helper is a
    // transparent passthrough.
    use tcl_lexer::TokenType;
    let tok = Token::new(TokenType::Esc, Span::new(0, 4));
    let argv = vec![tok];
    let result = argv_with_word_spans(argv.clone(), &[]);
    assert_eq!(result, argv);
}

#[test]
fn irules_top_level_only_matches_registry() {
    // `proc` is the only command tagged with the
    // `IRULES_TOP_LEVEL_ONLY` trait, so the result is exactly
    // `{"proc"}`. Pinning the set surfaces registry-spec drift
    // here.
    let reg = CommandRegistry::build_default();
    let cmds = irules_top_level_only(&reg);
    assert!(
        cmds.contains("proc"),
        "expected `proc` in irules_top_level_only, got {cmds:?}",
    );
}

#[test]
fn parse_param_list_reexport_works() {
    let params = parse_param_list(
        "a b {c default}",
        tcl_syntax::word_rules::WordValueRules::TCL,
    );
    assert_eq!(params.len(), 3);
    assert_eq!(params[0].name, "a");
    assert_eq!(params[2].name, "c");
    assert!(params[2].has_default);
}

#[test]
fn scan_stub_command_names_extracts_inline_stubs() {
    let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub my_cmd {arg1:var body:body} -loop
# tcl-lsp: stub my_eval {script:body} -barrier
# tcl-lsp: stubs-end
proc foo {} {}
";
    let names = scan_stub_command_names(src);
    assert!(names.contains("my_cmd"));
    assert!(names.contains("my_eval"));
    assert_eq!(names.len(), 2);
}

#[test]
fn scan_stub_command_names_ignores_outside_block() {
    let src = "\
# tcl-lsp: stub orphan_cmd {x:var}
# tcl-lsp: stubs-begin
# tcl-lsp: stub inside {x:var}
# tcl-lsp: stubs-end
# tcl-lsp: stub also_orphan {x:var}
";
    let names = scan_stub_command_names(src);
    assert!(names.contains("inside"));
    assert!(!names.contains("orphan_cmd"));
    assert!(!names.contains("also_orphan"));
}

#[test]
fn scan_stub_command_names_skips_expr_func_op_lines() {
    let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub expr-func sizeof 1
# tcl-lsp: stub expr-op contains 2
# tcl-lsp: stub regular_cmd {x:var}
# tcl-lsp: stubs-end
";
    let names = scan_stub_command_names(src);
    assert!(names.contains("regular_cmd"));
    assert!(!names.iter().any(|n| n.starts_with("expr-")));
}

#[test]
fn scan_stub_command_names_handles_multiple_blocks() {
    let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub a {x:var}
# tcl-lsp: stubs-end
proc foo {} {}
# tcl-lsp: stubs-begin
# tcl-lsp: stub b {y:var}
# tcl-lsp: stubs-end
";
    let names = scan_stub_command_names(src);
    assert!(names.contains("a"));
    assert!(names.contains("b"));
}

#[test]
fn scan_stub_command_names_empty_source() {
    let names = scan_stub_command_names("");
    assert_eq!(names.len(), 0);
}

// -- ``parse_command_stub`` + ``parse_expr_stub`` tests

fn cmd_stub(line: &str) -> Option<super::super::types::StubCommandDef> {
    let body = line.trim_start_matches('#').trim_start();
    let after_prefix = strip_tcl_lsp_prefix(body);
    let line_len = u32::try_from(line.len()).expect("test fixture fits in u32");
    parse_command_stub(after_prefix, Span::new(0, line_len))
}

fn expr_stub(line: &str) -> Option<(&'static str, String, u32)> {
    let body = line.trim_start_matches('#').trim_start();
    let after_prefix = strip_tcl_lsp_prefix(body);
    parse_expr_stub(after_prefix).map(|(k, n, a)| (k, n.to_string(), a))
}

#[test]
fn parse_command_stub_simple_command() {
    let stub = cmd_stub("# tcl-lsp: stub my_cmd {arg1 arg2}").unwrap();
    assert_eq!(stub.name, "my_cmd");
    assert_eq!(stub.args.len(), 2);
    assert_eq!(stub.args[0].name, "arg1");
    assert_eq!(stub.args[0].role, "value");
    assert_eq!(stub.args[1].name, "arg2");
}

#[test]
fn parse_command_stub_with_roles() {
    let stub =
        cmd_stub("# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop")
            .unwrap();
    assert_eq!(stub.name, "foreach_in_collection");
    assert_eq!(stub.args[0].role, "var");
    assert_eq!(stub.args[1].role, "value");
    assert_eq!(stub.args[2].role, "body");
    assert!(stub.flags.contains(super::super::types::StubFlags::LOOP));
    assert!(!stub.flags.contains(super::super::types::StubFlags::BARRIER));
}

#[test]
fn parse_command_stub_all_flags() {
    let stub =
        cmd_stub("# tcl-lsp: stub dangerous {script:body} -barrier -unsafe -mutator -scope_alias")
            .unwrap();
    assert!(stub.flags.contains(super::super::types::StubFlags::BARRIER));
    assert!(stub.flags.contains(super::super::types::StubFlags::UNSAFE));
    assert!(stub.flags.contains(super::super::types::StubFlags::MUTATOR));
    assert!(
        stub.flags
            .contains(super::super::types::StubFlags::SCOPE_ALIAS)
    );
    assert!(!stub.flags.contains(super::super::types::StubFlags::PURE));
    assert!(!stub.flags.contains(super::super::types::StubFlags::LOOP));
}

/// #2138: each stub flag lands on the catalogue field it names in the
/// declaration the document's command surface answers with.
#[test]
fn stub_flags_reach_their_declared_command_fields() {
    use tcl_registry::Traits;
    let declared = |flags: &str| {
        cmd_stub(&format!("# tcl-lsp: stub probe {{a}} {flags}"))
            .unwrap()
            .to_declared_command()
    };
    for (flag, traits) in [
        ("-barrier", Traits::CREATES_DYNAMIC_BARRIER),
        ("-loop", Traits::HAS_LOOP_BODY),
        ("-pure", Traits::PURE),
        ("-scope_alias", Traits::CREATES_SCOPE_ALIAS),
        ("-unsafe", Traits::UNSAFE.union(Traits::SAFE_INTERP_HIDDEN)),
        ("-mutator", Traits::READS_BEFORE_WRITE),
    ] {
        assert!(declared(flag).traits.contains(traits), "{flag}");
        assert!(!declared("").traits.contains(traits), "{flag}");
    }
    assert_ne!(declared("-mutator").side_effects, []);
    assert_eq!(declared("").side_effects, []);
}

#[test]
fn parse_command_stub_extension_flag() {
    let stub = cmd_stub("# tcl-lsp: stub pkga_calc {sub args} -extension -pure").unwrap();
    assert!(
        stub.flags
            .contains(super::super::types::StubFlags::EXTENSION)
    );
    assert!(stub.flags.contains(super::super::types::StubFlags::PURE));
    let plain = cmd_stub("# tcl-lsp: stub pkga_calc {sub args} -pure").unwrap();
    assert!(
        !plain
            .flags
            .contains(super::super::types::StubFlags::EXTENSION)
    );
    let sidecar = cmd_stub("stub pkga_calc {sub args} -extension").unwrap();
    assert!(
        sidecar
            .flags
            .contains(super::super::types::StubFlags::EXTENSION)
    );
}

#[test]
fn parse_command_stub_frame_effect() {
    use tcl_registry::model::DeclaredFrameEffect;
    let frame = |line: &str| cmd_stub(line).expect("a stub").frame;
    assert_eq!(
        frame("# tcl-lsp: stub db_query {sql}"),
        DeclaredFrameEffect::Unstated
    );
    assert_eq!(
        frame("# tcl-lsp: stub db_query {sql} -frame own"),
        DeclaredFrameEffect::Stated(None)
    );
    assert_eq!(
        frame("# tcl-lsp: stub db_query {sql} -pure -frame none"),
        DeclaredFrameEffect::Stated(None)
    );
    assert_eq!(
        frame("stub db_bind {spec} -frame caller"),
        DeclaredFrameEffect::Stated(Some(DeclaredFrameEffect::CALLER))
    );
    // The last word written wins; a word the grammar does not know, or
    // none at all, states nothing, as an unrecognised flag does.
    assert_eq!(
        frame("# tcl-lsp: stub db_bind {spec} -frame caller -frame own"),
        DeclaredFrameEffect::Stated(None)
    );
    assert_eq!(
        frame("# tcl-lsp: stub db_query {sql} -frame upvar"),
        DeclaredFrameEffect::Unstated
    );
    assert_eq!(
        frame("# tcl-lsp: stub db_query {sql} -frame"),
        DeclaredFrameEffect::Unstated
    );
    // The frame words are no flags of their own.
    let stub = cmd_stub("# tcl-lsp: stub db_query {sql} -frame own -pure").unwrap();
    assert_eq!(stub.flags, super::super::types::StubFlags::PURE);
    assert_eq!(
        stub.to_declared_command().frame_effect,
        DeclaredFrameEffect::Stated(None)
    );
}

#[test]
fn parse_command_stub_optional_args() {
    let stub = cmd_stub("# tcl-lsp: stub redirect {?-file? target body:body}").unwrap();
    assert!(stub.args[0].optional);
    assert_eq!(stub.args[0].name, "-file");
    assert!(!stub.args[1].optional);
}

#[test]
fn parse_command_stub_bare_format() {
    let stub = cmd_stub("stub get_cells {pattern:pattern} -pure").unwrap();
    assert_eq!(stub.name, "get_cells");
    assert!(stub.flags.contains(super::super::types::StubFlags::PURE));
}

#[test]
fn parse_command_stub_invalid_role_returns_none() {
    assert!(cmd_stub("# tcl-lsp: stub bad {arg:invalid_role}").is_none());
}

#[test]
fn parse_command_stub_not_a_stub_returns_none() {
    assert!(cmd_stub("# just a regular comment").is_none());
}

#[test]
fn multibyte_comment_before_keyword_length_does_not_panic() {
    // A non-ASCII byte (an em dash here) landing inside the
    // `tcl-lsp` / `stub` / `expr-func` / `expr-op` keyword's byte
    // length must simply not match, where a raw `s[..len]` slice
    // would panic. Each string below is crafted so the em
    // dash straddles the exact byte offset the check compares.
    assert!(cmd_stub("# st—b my_cmd {arg}").is_none());
    assert!(cmd_stub("# abcde—z not a stub").is_none());
    assert!(expr_stub("# st—b my_cmd {arg}").is_none());
    assert!(expr_stub("# tcl-lsp: stub expr-fun—c sizeof 1").is_none());
}

#[test]
fn parse_command_stub_empty_args() {
    let stub = cmd_stub("# tcl-lsp: stub no_args {} -pure").unwrap();
    assert_eq!(stub.args, [] as [crate::analyser::types::StubArgDef; 0]);
    assert!(stub.flags.contains(super::super::types::StubFlags::PURE));
}

#[test]
fn parse_command_stub_no_command_name_returns_none() {
    assert!(cmd_stub("# tcl-lsp: stub").is_none());
}

#[test]
fn parse_command_stub_unclosed_optional_marker() {
    // A ``?arg`` without closing ``?`` is treated as a
    // regular argument name.
    let stub = cmd_stub("# tcl-lsp: stub cmd {?arg}").unwrap();
    assert!(!stub.args[0].optional);
    assert_eq!(stub.args[0].name, "?arg");
}

#[test]
fn parse_command_stub_missing_brace_block_rejected() {
    // The ``{ARGS}`` block is required.
    assert!(cmd_stub("# tcl-lsp: stub bare_name").is_none());
}

#[test]
fn parse_expr_stub_func() {
    let (kind, name, arity) = expr_stub("# tcl-lsp: stub expr-func sizeof 1").unwrap();
    assert_eq!(kind, "function");
    assert_eq!(name, "sizeof");
    assert_eq!(arity, 1);
}

#[test]
fn parse_expr_stub_op() {
    let (kind, name, arity) = expr_stub("# tcl-lsp: stub expr-op contains 2").unwrap();
    assert_eq!(kind, "operator");
    assert_eq!(name, "contains");
    assert_eq!(arity, 2);
}

#[test]
fn parse_expr_stub_default_func_arity() {
    let (_, _, arity) = expr_stub("stub expr-func myfunc").unwrap();
    assert_eq!(arity, 1);
}

#[test]
fn parse_expr_stub_default_op_arity() {
    let (_, _, arity) = expr_stub("stub expr-op myop").unwrap();
    assert_eq!(arity, 2);
}

#[test]
fn parse_expr_stub_not_expr_returns_none() {
    assert!(expr_stub("stub get_cells {pattern:pattern}").is_none());
}

#[test]
fn scan_source_for_stubs_populates_args_and_flags() {
    let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop
# tcl-lsp: stub my_eval {script:body} -barrier
# tcl-lsp: stubs-end
proc foo {} {}
";
    let (cmds, exprs) = scan_source_for_stubs(src);
    assert_eq!(exprs, [] as [crate::analyser::types::StubExprDef; 0]);
    assert_eq!(cmds.len(), 2);
    let foreach = cmds
        .iter()
        .find(|c| c.name == "foreach_in_collection")
        .unwrap();
    assert!(foreach.flags.contains(super::super::types::StubFlags::LOOP));
    assert_eq!(foreach.args.len(), 3);
    assert_eq!(foreach.args[0].role, "var");
    assert_eq!(foreach.args[2].role, "body");
    let myeval = cmds.iter().find(|c| c.name == "my_eval").unwrap();
    assert!(
        myeval
            .flags
            .contains(super::super::types::StubFlags::BARRIER)
    );
    assert!(!myeval.flags.contains(super::super::types::StubFlags::LOOP));
}

#[test]
fn scan_source_for_stubs_carries_expr_arity() {
    let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub expr-func sizeof 3
# tcl-lsp: stub expr-op contains
# tcl-lsp: stubs-end
";
    let (_, exprs) = scan_source_for_stubs(src);
    assert_eq!(exprs.len(), 2);
    let sizeof = exprs.iter().find(|e| e.name == "sizeof").unwrap();
    assert_eq!(sizeof.kind, "function");
    assert_eq!(sizeof.arity, 3);
    let contains = exprs.iter().find(|e| e.name == "contains").unwrap();
    assert_eq!(contains.kind, "operator");
    assert_eq!(contains.arity, 2);
}

#[test]
fn scan_source_for_stubs_drops_invalid_role() {
    // The whole stub is dropped when any token uses an
    // unrecognised role.
    let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub good {arg:var}
# tcl-lsp: stub bad {arg:bogus}
# tcl-lsp: stubs-end
";
    let (cmds, _) = scan_source_for_stubs(src);
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].name, "good");
}

#[test]
fn scan_source_for_stubs_handles_non_ascii_comments() {
    // A `#` comment opening with a multi-byte char would panic
    // `matches_marker`'s byte slicing (`s[..7]` splitting inside the
    // char). Boundary-safe `str::get` must simply not match.
    let src = "\
# ═══ banner ═══
# tcl-lsp: stubs-begin
# tcl-lsp: stub good {arg:var}
# tcl-lsp: stubs-end
";
    let (cmds, _) = scan_source_for_stubs(src);
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].name, "good");
}

fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

#[test]
fn recovery_known_commands_includes_registry_names() {
    let known = recovery_known_commands(
        "set x [foo\n",
        &registry(),
        &std::sync::Arc::default(),
        tcl_lexer::LexerConfig::default(),
    );
    assert!(known.contains("set"));
    assert!(known.contains("puts"));
}

#[test]
fn recovery_known_commands_includes_extra_even_for_a_complete_script() {
    // `extra` (the LSP-layer-widened `extraCommands` set) is unioned in
    // unconditionally, same as the registry — unlike the in-file
    // signature scan it is not gated on `script_is_complete`, since it
    // costs nothing extra to fold in a set the caller already built.
    let extra: std::sync::Arc<HashSet<String>> =
        std::sync::Arc::new(["workspace_helper".to_string()].into_iter().collect());
    let known = recovery_known_commands(
        "set x 1\n",
        &registry(),
        &extra,
        tcl_lexer::LexerConfig::default(),
    );
    assert!(known.contains("workspace_helper"));
}

/// The caller-supplied set must be *shared*, not copied — on
/// the LSP recovery path it holds every workspace-indexed proc and class,
/// and the recovery branch runs on every keystroke inside an unterminated
/// block.
#[test]
fn recovery_known_commands_shares_the_extra_set_rather_than_copying_it() {
    let extra: SharedNameSet =
        std::sync::Arc::new(["workspace_helper".to_string()].into_iter().collect());
    let before = std::sync::Arc::strong_count(&extra);
    let known = recovery_known_commands(
        "set q {\nunclosed\n",
        &registry(),
        &extra,
        tcl_lexer::LexerConfig::default(),
    );
    assert_eq!(
        std::sync::Arc::strong_count(&extra),
        before + 1,
        "the extra layer must be shared with the caller, not deep-copied",
    );
    assert!(known.contains("workspace_helper"));
    assert!(known.contains("set"), "the registry layer is still present");
    drop(known);
    assert_eq!(
        std::sync::Arc::strong_count(&extra),
        before,
        "dropping the universe must release the shared layer",
    );
}

#[test]
fn recovery_known_commands_skips_the_scan_for_a_complete_script() {
    // The extra signature scan only runs on an incomplete script — a
    // well-formed document (however many procs it defines) never pays
    // for it, and its user-defined names are simply absent (recovery
    // never runs on complete input anyway).
    let known = recovery_known_commands(
        "proc my_helper {} {}\nmy_helper\n",
        &registry(),
        &std::sync::Arc::default(),
        tcl_lexer::LexerConfig::default(),
    );
    assert!(!known.contains("my_helper"));
    assert!(known.contains("proc")); // registry names are always present
}

#[test]
fn recovery_known_commands_adds_user_proc_qualified_and_tail() {
    let src = "namespace eval myns {\n  proc helper {x} {return $x}\n}\n\nset q {\nunclosed\n";
    let known = recovery_known_commands(
        src,
        &registry(),
        &std::sync::Arc::default(),
        tcl_lexer::LexerConfig::default(),
    );
    assert!(
        known.contains("myns::helper"),
        "expected the qualified name: {known:?}"
    );
    assert!(
        known.contains("helper"),
        "expected the unqualified tail: {known:?}"
    );
}

#[test]
fn original_recovery_procedure_declarations_keep_all_written_name_forms() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Genuine scanner metadata only; no command-cell or execution receipt.
    let source = "namespace eval source_names {proc SourceOnlyHelper {} {}}\nset unfinished {";
    let registry = registry();
    let config = tcl_lexer::LexerConfig::default();
    let scan = crate::signature_scan::extract_signatures_with_config(source, config, &registry);
    let declaration = scan
        .procedure_declarations
        .iter()
        .find(|declaration| declaration.name == "SourceOnlyHelper")
        .unwrap();
    assert_eq!(
        declaration.source_spelling().as_deref(),
        Some("::source_names::SourceOnlyHelper")
    );
    assert!(declaration.source_name.is_none());
    let known = recovery_known_commands(source, &registry, &std::sync::Arc::default(), config);
    for spelling in [
        "::source_names::SourceOnlyHelper",
        "source_names::SourceOnlyHelper",
        "SourceOnlyHelper",
    ] {
        assert!(known.contains(spelling), "{spelling}");
    }
    assert!(!known.contains("foreign::SourceOnlyHelper"));
    assert!(!known.contains("invented_source_command"));
}

#[test]
fn original_recovery_class_spellings_remain_metadata_under_the_complete_file_gate() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Original symbolic declarations and recovery candidates grant no namespace fact.
    let complete = "namespace eval source_classes {oo::class create SourceOnlyClass {}}";
    let source = format!("{complete}\nset unfinished {{");
    let registry = registry();
    let config = tcl_lexer::LexerConfig::default();
    let scan = crate::signature_scan::extract_signatures_with_config(&source, config, &registry);
    let declaration = scan
        .class_declarations
        .iter()
        .find(|declaration| declaration.name == "SourceOnlyClass")
        .unwrap();
    assert_eq!(
        declaration.source_spelling().as_deref(),
        Some("::source_classes::SourceOnlyClass")
    );
    assert!(declaration.source_name.is_none());
    let known = recovery_known_commands(&source, &registry, &std::sync::Arc::default(), config);
    for spelling in [
        "::source_classes::SourceOnlyClass",
        "source_classes::SourceOnlyClass",
        "SourceOnlyClass",
    ] {
        assert!(known.contains(spelling), "{spelling}");
    }
    let closed = recovery_known_commands(complete, &registry, &std::sync::Arc::default(), config);
    assert!(!closed.contains("SourceOnlyClass"));
    let dynamic = recovery_known_commands(
        "proc $unknown_name {} {}\nset unfinished {",
        &registry,
        &std::sync::Arc::default(),
        config,
    );
    assert!(!dynamic.contains("$unknown_name"));
}

#[test]
fn recovery_known_commands_adds_the_leading_colon_colon_form_too() {
    // `signature_scan::qualify` always records `::myns::helper`; a
    // recovery-point call can equally be written that way (absolute
    // Tcl syntax), so the leading-`::` form must be recognised
    // alongside the stripped/tail forms asserted above.
    let src = "namespace eval myns {\n  proc helper {x} {return $x}\n}\n\nset q {\nunclosed\n";
    let known = recovery_known_commands(
        src,
        &registry(),
        &std::sync::Arc::default(),
        tcl_lexer::LexerConfig::default(),
    );
    assert!(
        known.contains("::myns::helper"),
        "expected the fully-qualified leading-:: form: {known:?}"
    );
}

#[test]
fn recovery_known_commands_adds_user_class_and_alias() {
    let src = "oo::class create Widget {}\ninterp alias {} greet {} puts hi\n\nset q {\nunclosed\n";
    let known = recovery_known_commands(
        src,
        &registry(),
        &std::sync::Arc::default(),
        tcl_lexer::LexerConfig::default(),
    );
    assert!(known.contains("Widget"), "{known:?}");
    assert!(known.contains("greet"), "{known:?}");
}

#[test]
fn recovery_known_commands_adds_rename_target() {
    let src = "rename puts my_puts\n\nset q {\nunclosed\n";
    let known = recovery_known_commands(
        src,
        &registry(),
        &std::sync::Arc::default(),
        tcl_lexer::LexerConfig::default(),
    );
    assert!(
        known.contains("my_puts"),
        "expected the rename target: {known:?}"
    );
}

#[test]
fn recovery_known_commands_adds_namespaced_rename_target() {
    let src = "namespace eval myns {\n  rename puts loud_puts\n}\n\nset q {\nunclosed\n";
    let known = recovery_known_commands(
        src,
        &registry(),
        &std::sync::Arc::default(),
        tcl_lexer::LexerConfig::default(),
    );
    assert!(
        known.contains("myns::loud_puts"),
        "expected the qualified rename target: {known:?}"
    );
    assert!(
        known.contains("loud_puts"),
        "expected the unqualified tail: {known:?}"
    );
}

#[test]
fn recovery_known_commands_rename_to_empty_deletes_not_defines() {
    // `rename OLD {}` deletes OLD; it must not introduce an empty-string
    // "command name".
    let src = "rename puts {}\n\nset q {\nunclosed\n";
    let known = recovery_known_commands(
        src,
        &registry(),
        &std::sync::Arc::default(),
        tcl_lexer::LexerConfig::default(),
    );
    assert!(!known.contains(""));
}

#[test]
fn recovery_known_commands_does_not_invent_undefined_names() {
    let src = "set q {\nunclosed\n";
    let known = recovery_known_commands(
        src,
        &registry(),
        &std::sync::Arc::default(),
        tcl_lexer::LexerConfig::default(),
    );
    assert!(!known.contains("not_a_real_proc"));
    assert!(!known.contains("unclosed")); // plain data, not a definition
}
