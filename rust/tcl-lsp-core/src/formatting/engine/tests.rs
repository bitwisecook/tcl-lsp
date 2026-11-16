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

#[test]
fn original_formatter_body_parsing_keeps_bom_content_and_complete_parent_config() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Original source presentation only; no callable or Native entry grant.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let lexer_config = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        lexer_config,
    );
    let config = FormatterConfig::for_profile(profile).for_resolved_input(&input);
    for source in [
        "\u{feff}proc p {} {\u{feff}puts    1}",
        "apply {{} {\u{feff}puts    1}}",
        "switch x {x {\u{feff}puts    1}}",
    ] {
        let result = format_tcl_with_input(source, &config, &input);
        assert_eq!(
            result.matches('\u{feff}').count(),
            1,
            "{source:?}: {result:?}"
        );
        assert_eq!(config.lexer_config(), input.lexer_config());
    }
    let body = "\u{feff}puts    1";
    let identities = FormattingSourceLayout::with_input(body, &input);
    let context = input.context_registry();
    let result = format_body(
        body,
        &config,
        context.commands(),
        &identities,
        FormattingRegion::body(&config, 0, 0),
    );
    assert_eq!(result, "\u{feff}puts 1");
    let body = "puts ${a{b}; set x 1";
    let identities = FormattingSourceLayout::with_input(body, &input);
    let result = format_body(
        body,
        &config,
        context.commands(),
        &identities,
        FormattingRegion::body(&config, 0, 0),
    );
    assert_eq!(result, "puts ${a{b}\nset x 1");
}

#[test]
fn original_formatter_range_uses_the_selected_script_prologue_purpose() {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = FormatterConfig::for_profile(profile);
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let source = "set x 0\n\u{feff}puts    1\n";
    let edits = crate::formatting::range_formatting(
        source,
        crate::definition::LspRange {
            start_line: 1,
            start_character: 0,
            end_line: 1,
            end_character: 20,
        },
        &config,
        context.commands(),
    );
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "\u{feff}puts 1\n");
}

// Implementation contract: naming.editor.original-source-formatting
// docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
#[test]
fn original_source_formatting_uses_effective_roles_without_widening_captured_operands() {
    let profile = profile_of("tcl8.6");
    let registry = tcl_registry::model::ingress::context_for_profile(profile);
    let config = FormatterConfig::for_profile(profile);
    let source = "interp alias {} branch {} if 1\nbranch {puts    selected}\n";
    let result = format_tcl(source, &config, registry.commands());
    assert!(
        result.contains("branch {\n    puts selected\n}"),
        "{result}"
    );
    let source = "proc branch {args} {}\nbranch {puts    data}\n";
    let result = format_tcl(source, &config, registry.commands());
    assert!(result.contains("branch {puts    data}"), "{result}");
    let source = "for {set i 0} {$i < 2} {incr i} {puts    selected}\n";
    let result = format_tcl(source, &config, registry.commands());
    assert!(
        result.starts_with("for {set i 0} {$i < 2} {incr i} {\n"),
        "{result}"
    );
    assert!(result.contains("    puts selected"), "{result}");
}

// Implementation contract: naming.editor.original-source-formatting
// docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
#[test]
fn original_source_formatting_braces_only_a_retained_literal_equivalence() {
    let profile = profile_of("tcl8.6");
    let registry = tcl_registry::model::ingress::context_for_profile(profile);
    let config = FormatterConfig {
        enforce_braced_expr: true,
        ..FormatterConfig::for_profile(profile)
    };
    let literal = format_tcl("expr \"1 + 2\"\n", &config, registry.commands());
    assert_eq!(literal, "expr {1 + 2}\n");
    let dynamic = format_tcl("expr $value\n", &config, registry.commands());
    assert_eq!(dynamic, "expr $value\n");
    let shadowed = format_tcl(
        "proc expr {args} {}\nexpr \"1 + 2\"\n",
        &config,
        registry.commands(),
    );
    assert!(shadowed.ends_with("expr \"1 + 2\"\n"), "{shadowed}");
}

/// The seam's analyser ingress — the profile every test names a
/// dialect through (the exact twin of the retired name resolver).
fn profile_of(name: &str) -> &'static tcl_dialect::DialectProfile {
    tcl_registry::model::ingress::resolve_environment(name).analyser_profile()
}

fn fmt(src: &str) -> String {
    let registry = CommandRegistry::build_default();
    format_tcl(src, &FormatterConfig::default(), &registry)
}

fn fmt_with(src: &str, config: &FormatterConfig) -> String {
    let registry = CommandRegistry::build_default();
    format_tcl(src, config, &registry)
}

fn fmt_dialect(src: &str, dialect: &'static tcl_dialect::DialectProfile) -> String {
    let registry = crate::registry_for_dialect_profile(dialect);
    format_tcl(src, &FormatterConfig::for_profile(dialect), registry)
}

/// `format_body` recurses once per
/// nested control-flow body. Empirically, unguarded nested `if` bodies
/// overflowed the native stack (SIGABRT) between depth 800 and 1200 on
/// a 2 MiB thread (`cargo test`'s per-test default). 2000 is
/// comfortably past both that crash range and `MAX_FORMAT_DEPTH` (128);
/// the assertion is that formatting returns at all, not what it
/// returns — past the cap, `format_body` leaves the excess nesting
/// unformatted rather than crashing.
#[test]
fn deeply_nested_if_survives_formatting() {
    // Implementation contract: naming.editor.original-source-formatting-budget
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting-budget.md
    const DEPTH: usize = 2000;
    let mut src = String::new();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("set done 1\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    let _ = fmt(&src);
}

#[test]
fn formatting_budget_preserves_complete_source_before_analysis() {
    // Implementation contract: naming.editor.original-source-formatting-budget
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting-budget.md
    let source = format!(
        "set value {{{}}}\r\n",
        "{".repeat(128) + "opaque  " + &"}".repeat(128)
    );
    let config = FormatterConfig {
        ensure_final_newline: true,
        trim_trailing_whitespace: true,
        ..FormatterConfig::default()
    };
    assert!(!source_within_formatting_budget(&source, &config));
    assert_eq!(fmt_with(&source, &config), source);
    assert!(source_within_formatting_budget("puts \"{{{{\"", &config));
}

/// A moderately nested body (well under `MAX_FORMAT_DEPTH`) still
/// formats normally — the safety net must not fire on realistic
/// nesting depths.
#[test]
fn moderately_nested_if_still_formats() {
    let src = "if {1} {\nif {2} {\nset x 1\n}\n}\n";
    let out = fmt(src);
    assert!(out.contains("set x 1"), "{out:?}");
}

#[test]
fn count_body_commands_counts_top_level_statements() {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    let config = FormatterConfig::default().lexer_config();
    for (source, expected) in [
        ("", 0),
        ("  \n ", 0),
        ("puts hi", 1),
        ("puts a\nputs b", 2),
        ("puts a; puts b", 2),
        ("set x {a; b}", 1),
        ("set x [expr {1; 2}]", 1),
        ("puts \"a; b\"", 1),
        ("# ignored; comment\nputs hi", 1),
    ] {
        assert_eq!(
            count_body_commands(source, config),
            Some(expected),
            "{source:?}"
        );
    }
    assert!(count_body_commands("puts \"missing", config).is_none());
}

#[test]
fn command_count_retains_complete_body_grammar() {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    let source = "puts x\n{y}";
    let config = FormatterConfig::default().lexer_config();
    assert_eq!(count_body_commands(source, config), Some(2));
    let continued = tcl_lexer::LexerConfig {
        brace_line_continuation: tcl_dialect::BraceLineContinuation::Continues,
        ..config
    };
    assert_eq!(count_body_commands(source, continued), Some(1));
    let strict = tcl_lexer::LexerConfig {
        strict_quoting: true,
        ..config
    };
    assert!(count_body_commands("puts {missing", strict).is_none());
}

#[test]
fn min_body_commands_keeps_single_command_body_inline() {
    // With expansion on but the threshold at 2, a
    // one-command body stays inline while a two-command body expands.
    // `catch` has an inline-able body (not NEVER_INLINE_BODY), so it
    // exercises the `expand_single_line_bodies` path.
    let config = FormatterConfig {
        expand_single_line_bodies: true,
        min_body_commands_for_expansion: 2,
        ..FormatterConfig::default()
    };
    let one = fmt_with("catch { puts hi }\n", &config);
    assert!(
        one.contains("{ puts hi }") && !one.contains("puts hi\n"),
        "single-command body should stay inline: {one:?}"
    );
    let two = fmt_with("catch { puts a; puts b }\n", &config);
    assert!(
        two.contains("puts a\n") && two.contains("puts b\n"),
        "two-command body should expand: {two:?}"
    );
}

#[test]
fn enforce_braced_expr_braces_expr_and_conditions() {
    // `enforceBracedExpr` braces bare expressions.
    let config = FormatterConfig {
        enforce_braced_expr: true,
        ..FormatterConfig::default()
    };
    // `expr` concatenates its whole argument tail into one expression, so
    // the entire tail is braced (not just arg 0, which would corrupt it).
    assert_eq!(
        fmt_with("expr $a + $b\n", &config),
        "expr {$a + $b}\n",
        "expr tail must be braced as one group"
    );
    // A single-argument condition (bounded expr) is braced in place.
    assert_eq!(
        fmt_with("while $x {\n    incr y\n}\n", &config),
        "while {$x} {\n    incr y\n}\n"
    );
    // Already-braced expressions are left unchanged (idempotent).
    assert_eq!(fmt_with("expr {$a + $b}\n", &config), "expr {$a + $b}\n");
    assert_eq!(
        fmt_with("if {$x} {\n    puts hi\n}\n", &config),
        "if {$x} {\n    puts hi\n}\n"
    );
}

#[test]
fn align_comments_to_code_toggles_reindentation() {
    // Default (true): a mis-indented standalone comment is re-indented to
    // the code column.
    let src = "proc f {} {\n        # over-indented\n    set x 1\n}\n";
    let aligned = fmt(src);
    assert!(
        aligned.contains("\n    # over-indented\n"),
        "default should re-indent comment to code column: {aligned:?}"
    );
    // Disabled: the comment keeps its original column.
    let config = FormatterConfig {
        align_comments_to_code: false,
        ..FormatterConfig::default()
    };
    let preserved = fmt_with(src, &config);
    assert!(
        preserved.contains("\n        # over-indented\n"),
        "disabled should preserve the original comment column: {preserved:?}"
    );
}

#[test]
fn enforce_braced_expr_off_by_default() {
    // FP-guard: with the default config (off) bare exprs are untouched.
    assert_eq!(fmt("expr $a + $b\n"), "expr $a + $b\n");
}

#[test]
fn enforce_braced_expr_preserves_expansion_tail() {
    // `expr {*}$pieces` expands the list before evaluating;
    // bracing it (`expr {{*}$pieces}`) would demote `{*}` to literal text
    // and break the expression. The rewrite must leave it alone.
    let config = FormatterConfig {
        enforce_braced_expr: true,
        ..FormatterConfig::default()
    };
    assert_eq!(
        fmt_with("expr {*}$pieces\n", &config),
        "expr {*}$pieces\n",
        "an expansion tail must not be braced"
    );
}

#[test]
fn replace_semicolons_disabled_keeps_commands_inline() {
    // Default splits `a; b` onto two lines; disabling the
    // setting keeps them on one line joined by `; `.
    let split = fmt("puts a; puts b\n");
    assert!(
        split.contains("puts a\nputs b"),
        "default should split on `;`: {split:?}"
    );
    let config = FormatterConfig {
        replace_semicolons_with_newlines: false,
        ..FormatterConfig::default()
    };
    let joined = fmt_with("puts a; puts b\n", &config);
    assert!(
        joined.contains("puts a; puts b"),
        "disabled should keep `;`-joined commands inline: {joined:?}"
    );
}

#[test]
fn bare_dollar_in_quoted_string_is_not_brace_wrapped() {
    // A lone `$` inside a double-quoted string is literal
    // data; formatting must not turn `"cost: $"` into `"cost: {$}"`.
    let out = fmt("puts \"cost: $\"\n");
    assert!(out.contains("\"cost: $\""), "{out:?}");
    assert!(!out.contains("{$}"), "bare $ was brace-wrapped: {out:?}");
}

#[test]
fn backslash_newline_keeps_preceding_spaces_in_quoted_string() {
    // Tcl replaces `\<nl><following-ws>` with a single
    // space but keeps whitespace *before* the backslash. `"a \<nl> b"` is
    // the value `a  b` (two spaces) — the pre-backslash space must survive.
    let out = fmt("puts \"a \\\n b\"\n");
    assert!(
        out.contains("\"a  b\""),
        "expected two spaces preserved: {out:?}"
    );
}

#[test]
fn backslash_newline_collapse_handles_crlf() {
    // A `\<CR><LF>` continuation collapses exactly like `\<LF>` (the old
    // LF-only normalisation left the `\` and CRLF in place). In a quoted
    // string the pre-backslash space is data and survives (`a  b`)…
    let out = fmt("puts \"a \\\r\n b\"\n");
    assert!(
        out.contains("\"a  b\""),
        "expected CRLF continuation collapsed with data space kept: {out:?}"
    );
    // …while in a command substitution the whole run is one separator.
    let out = fmt("set y [string cat a \\\r\n  b]\n");
    assert!(
        out.contains("[string cat a b]"),
        "expected CRLF continuation collapsed to one separator: {out:?}"
    );
}

#[test]
fn trim_preserves_multiline_braced_string_interior() {
    // Trailing spaces *inside* a multi-line braced word are
    // string data and must survive the trailing-whitespace pass.
    let input = "set x {line1   \nline2}\n";
    let out = fmt(input);
    assert!(
        out.contains("line1   \n"),
        "trailing spaces inside the braced string were stripped:\n{out:?}"
    );
}

#[test]
fn trailing_trim_preserves_spaces_inside_multiline_braces() {
    // The spaces before the newline live inside the braced word, so they
    // are part of the Tcl string value — trimming them would change the
    // runtime string, not just presentation (default trim is enabled).
    let out = fmt("set x {foo   \n   bar}\n");
    assert!(
        out.contains("foo   \n"),
        "significant whitespace inside braces preserved: {out:?}",
    );
}

#[test]
fn trim_preserves_multiline_quoted_string_interior() {
    let input = "set x \"line1   \nline2\"\n";
    let out = fmt(input);
    assert!(
        out.contains("line1   \n"),
        "trailing spaces inside the quoted string were stripped:\n{out:?}"
    );
}

#[test]
fn trim_still_strips_structural_trailing_whitespace() {
    // Ordinary code lines (outside any literal) are still trimmed.
    let out =
        trim_trailing_ws_preserving_literals("set a 1   \nset b 2   ", &FormatterConfig::default());
    assert_eq!(out, "set a 1\nset b 2");
}

#[test]
fn trailing_trim_still_trims_ordinary_code_lines() {
    // A trailing run of spaces on a real code line (not inside a literal)
    // is still trimmed.
    let out = fmt("set a 1   \nset b 2\n");
    assert_eq!(out, "set a 1\nset b 2\n");
}

#[test]
fn irule_brace_chain_gets_a_space() {
    // TMM accepts `}{` (e.g. `if {expr}{body}`); with the f5-irules lexer
    // preset the formatter parses it as two words and re-emits `} {`.
    let registry = CommandRegistry::build_default();
    let config = FormatterConfig::for_profile(tcl_dialect::DialectProfile::irules());
    let out = format_tcl("if { 1 }{\n    pool p\n}\n", &config, &registry);
    assert!(!out.contains("}{"), "left `}}{{` unfixed:\n{out}");
    assert!(out.contains("} {"), "no `}} {{`:\n{out}");

    // Plain-Tcl default preset does not synthesise the separator, so it
    // leaves the (invalid-in-stock-Tcl) input alone rather than inventing a
    // parse — no accidental change to non-iRule formatting.
    let plain = format_tcl(
        "if { 1 }{\n    pool p\n}\n",
        &FormatterConfig::default(),
        &registry,
    );
    assert!(
        plain.contains("}{"),
        "default preset should not rewrite `}}{{`"
    );
}

#[test]
fn the_irules_profile_alone_decides_the_ghost_separator() {
    // The dialect reaches the formatter as one resolved
    // profile, so a caller that names iRules — under either spelling —
    // gets the iRules lexer, and one that names a Tcl release does not.
    // `}{` is the discriminator: TMM parses it as two words, stock Tcl
    // does not.
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let source = "when HTTP_REQUEST {\n    if { 1 }{\n        pool p\n    }\n}\n";
    for spelling in ["f5-irules", "irules", "tcl-irule"] {
        let out = format_tcl(
            source,
            &FormatterConfig::for_profile(profile_of(spelling)),
            registry,
        );
        assert!(out.contains("} {"), "{spelling} emitted no `}} {{`:\n{out}");
        assert!(
            !out.contains("}{"),
            "{spelling} left `}}{{` unfixed:\n{out}"
        );
    }
    // The mismatched modern-Tcl profile — what a caller that forgets the
    // dialect gets — leaves the same bytes alone.
    let tcl9 = format_tcl(
        source,
        &FormatterConfig::for_profile(profile_of("tcl9.0")),
        registry,
    );
    assert!(
        tcl9.contains("}{"),
        "the Tcl 9 profile must not synthesise the separator:\n{tcl9}"
    );
}

/// Each `(input, expected)` pair is the expected formatted output
/// for the same input.
fn check(input: &str, expected: &str) {
    let got = fmt(input);
    assert_eq!(
        got, expected,
        "\ninput:    {input:?}\ngot:      {got:?}\nexpected: {expected:?}"
    );
}

#[test]
fn keyword_edits_recheck_the_actual_subset_after_braced_word_filtering() {
    use tcl_registry::forms::{CommandForm, LiteralArgumentPrefix};
    static ROLES: &[(u8, ArgRole)] = &[
        (0, ArgRole::Boolean),
        (1, ArgRole::Boolean),
        (2, ArgRole::Boolean),
    ];
    static FORMS: &[CommandForm] = &[
        CommandForm {
            name: "same-label",
            arity: tcl_registry::Arity::exact(3),
            literal_argument_prefix: Some(LiteralArgumentPrefix::exact(&["yes", "true", "true"])),
            arg_roles: ROLES,
            ..CommandForm::DEFAULT
        },
        CommandForm {
            name: "same-label",
            arity: tcl_registry::Arity::exact(3),
            arg_roles: ROLES,
            ..CommandForm::DEFAULT
        },
    ];
    let mut registry = CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "format_source_filtered_subset",
        command_forms: FORMS,
        arg_roles: ROLES,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let config = FormatterConfig {
        boolean_form: super::super::config::BooleanForm::TrueFalse,
        ..FormatterConfig::default()
    };
    let capture = |source: &str| {
        let identities = FormattingSourceLayout::new(source, &config, &registry);
        let map = SourceMap::new(source);
        let tokens = Lexer::with_config(source, config.lexer_config())
            .tokenise_all()
            .unwrap();
        let mut command = parse_commands(source, &map, &tokens, config.lexer_config())
            .unwrap()
            .0
            .remove(0);
        identify_body_args(&mut command, &registry, &identities, 0, &config, source);
        command
    };
    let plain = capture("format_source_filtered_subset yes yes yes");
    assert_eq!(keyword_rewrites_for(&plain, &registry, &config).len(), 3);
    let braced = capture("format_source_filtered_subset {yes} yes yes");
    assert!(braced.args[1].is_braced);
    let (Some(source), Some(context)) = (&braced.source_words, &braced.logical_context) else {
        panic!("genuine source schema required");
    };
    assert_eq!(
        source.with_source_schema(context, |selected| {
            super::super::keywords::rewrites_for_selected_invocation(selected, &config).len()
        }),
        Some(3)
    );
    assert!(keyword_rewrites_for(&braced, &registry, &config).is_empty());
    let quoted = capture("format_source_filtered_subset \"yes\" yes yes");
    assert!(quoted.args[1].is_quoted);
    assert!(keyword_rewrites_for(&quoted, &registry, &config).is_empty());
}

#[test]
fn keyword_edits_require_same_original_selected_words_context_and_config() {
    let source = "lsearch -noc $values $pattern";
    let registry = CommandRegistry::build_default();
    let config = FormatterConfig {
        expand_abbreviations: true,
        ..FormatterConfig::default()
    };
    let identities = FormattingSourceLayout::new(source, &config, &registry);
    let map = SourceMap::new(source);
    let tokens = Lexer::with_config(source, config.lexer_config())
        .tokenise_all()
        .unwrap();
    let capture = || {
        let mut command = parse_commands(source, &map, &tokens, config.lexer_config())
            .unwrap()
            .0
            .remove(0);
        identify_body_args(&mut command, &registry, &identities, 0, &config, source);
        command
    };
    let command = capture();
    assert_eq!(
        keyword_rewrites_for(&command, &registry, &config)
            .get(&1)
            .map(String::as_str),
        Some("-nocase")
    );
    let mut command = capture();
    command.args[1].original_word = None;
    assert!(keyword_rewrites_for(&command, &registry, &config).is_empty());
    let mut command = capture();
    command.logical_context = None;
    assert!(keyword_rewrites_for(&command, &registry, &config).is_empty());
    let mut command = capture();
    command.source_roles = Some(Vec::new());
    assert!(keyword_rewrites_for(&command, &registry, &config).is_empty());
    let mut foreign = config.clone();
    let mut lexer = config.lexer_config();
    lexer.strict_quoting = !lexer.strict_quoting;
    foreign.lexer_config_override = Some(lexer);
    assert!(keyword_rewrites_for(&capture(), &registry, &foreign).is_empty());
}

#[test]
fn logical_default_formatting_retains_its_whole_context_over_mixed_specs() {
    // naming.editor.logical-formatting-context
    // docs/design/analysis/name-resolution-proofs/logical-formatting-context.md
    let registry = CommandRegistry::build_default();
    assert!(
        !registry
            .get("expr")
            .unwrap()
            .traits
            .contains(Traits::EXPR_CONCATENATES_ARGS)
    );
    let config = FormatterConfig::default();
    let identities = FormattingSourceLayout::new("expr $a + $b", &config, &registry);
    assert!(identities.analysis.is_some());
    let context = identities.input.context_registry();
    assert_eq!(context.context().environment.id.as_str(), "tcl");
    assert!(
        tcl_registry::InvocationDialect::of_profile(identities.input.analyser_profile(),)
            .native_string_protocol()
            .is_none()
    );
    let source = "expr $a + $b";
    let map = SourceMap::new(source);
    let tokens = Lexer::with_config(source, config.lexer_config())
        .tokenise_all()
        .unwrap();
    let mut command = parse_commands(source, &map, &tokens, config.lexer_config())
        .unwrap()
        .0
        .remove(0);
    identify_body_args(&mut command, &registry, &identities, 0, &config, source);
    assert!(command.source_roles.is_none());
    assert!(command.logical_roles.is_some());
    assert!(command.logical_context.is_some());
    assert!(
        command
            .source_traits
            .unwrap()
            .contains(Traits::EXPR_CONCATENATES_ARGS)
    );
    assert!(command.args.iter().all(|arg| arg.original_input.is_none()));
    assert!(
        command
            .args
            .iter()
            .skip(1)
            .all(|arg| arg.original_word.is_some())
    );
    assert!(
        matches!(command.source_words.as_ref().map(|words| &words.source),
            Some(crate::original_invocation::OriginalRegistrySource::SourceTransitions(advice))
                if advice.logical_source_input() == Some(&identities.input))
    );
    let foreign = FormatterConfig {
        lexer_config_override: Some(tcl_lexer::LexerConfig {
            strict_quoting: true,
            ..config.lexer_config()
        }),
        ..FormatterConfig::default()
    };
    let mut command = parse_commands(source, &map, &tokens, config.lexer_config())
        .unwrap()
        .0
        .remove(0);
    identify_body_args(&mut command, &registry, &identities, 0, &foreign, source);
    assert_eq!(command.source_traits, Some(Traits::empty()));
    assert_eq!(command.source_roles, Some(Vec::new()));
    assert!(command.logical_roles.is_none());
    assert!(command.logical_context.is_none());
    assert!(keyword_rewrites_for(&command, &registry, &foreign).is_empty());
}

#[test]
fn analysed_layout_retains_supplied_entry_and_provider_world_without_recapture() {
    // naming.editor.original-source-formatting
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
    // Actual source entry/provider coverage remains independent of source roles.
    use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
    use tcl_compiler::command_binding::SourceAnalysisEntry;
    let source = "if 1 {puts α}\r\n";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let mut entry = SourceAnalysisEntry::for_logical_source(&input).unwrap();
    entry.unknown_entry = true;
    let mut analysis = Analyser::new()
        .with_resolved_input(input.clone())
        .with_source_analysis_entry(std::sync::Arc::new(entry))
        .analyse(source, profile.name);
    analysis.has_dynamic_providers = true;
    let layout = FormattingSourceLayout::with_analysis(source, &analysis).unwrap();
    assert_eq!(layout.realm, *analysis.retained_command_realm().unwrap());
    assert!(layout.analysis.as_ref().unwrap().has_dynamic_providers);
    assert_eq!(layout.input, input);
    let fresh =
        FormattingSourceLayout::with_input(layout.original.as_ref().unwrap().text(), &input);
    assert_ne!(
        layout.realm, fresh.realm,
        "unknown supplied history cannot become a fresh source world"
    );
}

#[test]
fn simple_command() {
    check("puts hi\n", "puts hi\n");
}

#[test]
fn proc_body_indents() {
    check("proc f {} {\nset x 1\n}\n", "proc f {} {\n    set x 1\n}\n");
}

#[test]
fn if_else() {
    check(
        "if {$x} {\nputs a\n} else {\nputs b\n}\n",
        "if {$x} {\n    puts a\n} else {\n    puts b\n}\n",
    );
}

#[test]
fn nested_indentation() {
    check(
        "proc f {} {\nif {1} {\nset x 1\n}\n}\n",
        "proc f {} {\n    if {1} {\n        set x 1\n    }\n}\n",
    );
}

#[test]
fn comments_preserved() {
    check("#hello\n# spaced\nputs hi\n", "#hello\n# spaced\nputs hi\n");
}

#[test]
fn collapses_blank_lines_to_max() {
    check("set x 1\n\n\n\nset y 2\n", "set x 1\n\n\nset y 2\n");
}

#[test]
fn switch_body() {
    check(
        "switch $x {\na {\nputs 1\n}\nb {\nputs 2\n}\n}\n",
        "switch $x {\n    a {\n        puts 1\n    }\n    b {\n        puts 2\n    }\n}\n",
    );
}

#[test]
fn switch_body_preserves_comments() {
    // A comment line inside a case-list body must survive
    // formatting rather than being silently deleted.
    let out = fmt("switch $x {\n# note\na { puts 1 }\n}\n");
    assert!(
        out.contains("# note"),
        "switch-body comment was deleted:\n{out}"
    );
    assert!(out.contains("puts 1"), "arm body lost:\n{out}");
}

#[test]
fn switch_case_list_formatter_follows_release_matrix() {
    let valid = "switch subject {\ndefault {\nputs hit\n}\n}\n";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"] {
        let out = fmt_dialect(valid, profile_of(dialect));
        assert!(
            out.contains("    default {\n        puts hit\n    }"),
            "{dialect} must format a normal case-list body:\n{out}"
        );
    }
    for subject in ["-regexp", "--"] {
        let ambiguous = format!("switch {subject} {{\ndefault {{\nputs hit\n}}\n}}\n");
        let old = fmt_dialect(&ambiguous, profile_of("tcl8.4"));
        assert!(
            !old.contains("        puts hit"),
            "Tcl 8.4 must not descend the invalid option-like two-word form {subject:?}:\n{old}"
        );
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
            let out = fmt_dialect(&ambiguous, profile_of(dialect));
            assert!(
                out.contains("    default {\n        puts hit\n    }"),
                "{dialect} must format the optionless two-word case list with subject {subject:?}:\n{out}"
            );
        }
    }
}

#[test]
fn incomplete_case_list_recovery_is_presentation_only_and_idempotent() {
    fn assert_fixed_point(
        source: &str,
        dialect: &'static tcl_dialect::DialectProfile,
        nested: &str,
    ) -> String {
        let once = fmt_dialect(source, dialect);
        assert!(
            once.contains(nested),
            "{} did not present the incomplete case list:\n{once}",
            dialect.name
        );
        assert_eq!(
            fmt_dialect(&once, dialect),
            once,
            "{} formatter recovery is not idempotent",
            dialect.name
        );
        once
    }

    let ordinary = "switch subject {\na {\nputs hit\n}\norphan\n}\n";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"] {
        assert_fixed_point(
            ordinary,
            profile_of(dialect),
            "a {\n        puts hit\n    }",
        );
    }

    // An empty list is not a semantic case invocation, but remains a
    // formatter-local recoverable body position. Rendering it must stay
    // safe and must not change Tcl's erroneous source into a made-up
    // clause.
    let empty = "switch subject {}\n";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"] {
        assert_eq!(fmt_dialect(empty, profile_of(dialect)), empty, "{dialect}");
    }

    let option_like = "switch -regexp {\na {\nputs hit\n}\norphan\n}\n";
    let old = fmt_dialect(option_like, profile_of("tcl8.4"));
    assert!(
        !old.contains("        puts hit"),
        "Tcl 8.4 must not recover the invalid two-word option form:\n{old}"
    );
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        assert_fixed_point(
            option_like,
            profile_of(dialect),
            "a {\n        puts hit\n    }",
        );
    }

    let aliased =
        fmt("interp alias {} pick {} switch\npick subject {\na {\nputs hit\n}\norphan\n}\n");
    assert!(
        aliased.contains("a {\n        puts hit\n    }"),
        "resolved alias did not inherit formatter recovery:\n{aliased}"
    );
    assert_eq!(fmt(&aliased), aliased);
}

#[test]
fn expect_case_list_formatter_preserves_descriptor_fields_and_formats_only_actions() {
    let source = "expect {\n-regexp {a; b} {puts canonical}\n-re {c; d} {puts abbreviated}\n-glob \"hello world\" {puts quoted}\n-exact \"escaped\\ pattern\" {puts escaped}\n-timeout 5 timeout {puts timed}\n-i $spawn_id eof {puts eof}\n-- {-literal} {puts literal}\nfull_buffer\n}\n";
    let once = fmt_dialect(source, profile_of("expect"));
    assert_eq!(fmt_dialect(&once, profile_of("expect")), once, "{once}");

    for literal in [
        "-regexp {a; b} {",
        "-re {c; d} {",
        "-glob \"hello world\" {",
        "-exact \"escaped\\ pattern\" {",
        "-timeout 5 timeout {",
        "-i $spawn_id eof {",
        "-- {-literal} {",
    ] {
        assert!(
            once.contains(literal),
            "descriptor field changed: {literal:?}\n{once}"
        );
    }
    for action in [
        "puts canonical",
        "puts abbreviated",
        "puts quoted",
        "puts escaped",
        "puts timed",
        "puts eof",
        "puts literal",
    ] {
        assert!(
            once.contains(&format!("        {action}")),
            "action was not formatted: {action:?}\n{once}"
        );
    }
    assert!(
        once.contains("    full_buffer\n"),
        "omitted final action was invented or lost:\n{once}"
    );
    assert!(
        !once.contains("        a\n"),
        "regex data was formatted as a script:\n{once}"
    );
    assert!(
        !once.contains("        c\n"),
        "abbreviated regex data was formatted as a script:\n{once}"
    );

    for source in [
        "expect {\n-bogus {a; b} {puts opaque}\n}\n",
        "expect {\n-timeout\n}\n",
    ] {
        let formatted = fmt_dialect(source, profile_of("expect"));
        assert_eq!(
            fmt_dialect(&formatted, profile_of("expect")),
            formatted,
            "{formatted}"
        );
        assert!(
            !formatted.contains("        a\n"),
            "malformed data was treated as an action:\n{formatted}"
        );
    }

    // Once a descriptor flag cannot be resolved, no later apparent
    // pattern/action pairing is proven. The formatter must keep the
    // entire case-list value opaque rather than resynchronising at `q`
    // or `x` and interpreting data as Tcl scripts.
    for malformed in [
        "expect {\n-bogus p q {puts falsely_formatted}\nx {puts also_formatted}\n}\n",
        "expect {\n-n p {puts ambiguous}\nx {puts also_ambiguous}\n}\n",
        "expect {\n-timeout\n}\n",
    ] {
        let formatted = fmt_dialect(malformed, profile_of("expect"));
        assert_eq!(formatted, malformed, "malformed list was rewritten");
        assert_eq!(fmt_dialect(&formatted, profile_of("expect")), formatted);
    }
    let dynamic = fmt_dialect(
        "expect {\n-re $pattern {puts dynamic}\n}\n",
        profile_of("expect"),
    );
    assert!(
        dynamic.contains("-re $pattern {"),
        "dynamic pattern changed:\n{dynamic}"
    );
    assert!(
        dynamic.contains("        puts dynamic"),
        "proven action was not formatted:\n{dynamic}"
    );
}

#[test]
fn control_flow_always_expands() {
    check("if {$x} { return }\n", "if {$x} {\n    return\n}\n");
}

#[test]
fn while_body() {
    check("while {$x} {\nincr x\n}\n", "while {$x} {\n    incr x\n}\n");
}

#[test]
fn foreach_body() {
    check(
        "foreach i {1 2 3} {\nputs $i\n}\n",
        "foreach i {1 2 3} {\n    puts $i\n}\n",
    );
}

// The formatting engine holds no `if` / `try` / `for` name checks; every
// layout decision comes from the registry.

/// TP / FN — C Tcl resolves the absolute global spelling to the same
/// command (`namespace which -command ::if` → `::if`), so `::if`,
/// `::for`, and `::try` must format exactly like their bare forms. The
/// old `name == "if"` comparisons simply did not fire for them.
#[test]
fn qualified_control_flow_formats_like_the_bare_form() {
    for (bare, qualified) in [
        ("if {$x} then {\nputs yes\n} else {\nputs no\n}\n", "::if"),
        ("for {set i 0} {$i < 3} {incr i} {\nputs $i\n}\n", "::for"),
        (
            "try {\nrisky\n} on error {msg opts} {\nputs $msg\n}\n",
            "::try",
        ),
        ("while {$x} {\nincr x\n}\n", "::while"),
    ] {
        let head = bare.split_whitespace().next().expect("a head word");
        let qualified_src = bare.replacen(head, qualified, 1);
        let want = fmt(bare).replacen(head, qualified, 1);
        assert_eq!(
            fmt(&qualified_src),
            want,
            "{qualified} formatted differently from {head}"
        );
    }
}

/// TP — `for`'s `start` and `next` scripts stay inline while only `body`
/// expands. That preference is now registry data
/// (`arg_presentation: InlineScript`), not a formatter branch.
#[test]
fn for_keeps_start_and_next_inline() {
    check(
        "for {set i 0} {$i < 3} {incr i} {\nputs $i\n}\n",
        "for {set i 0} {$i < 3} {incr i} {\n    puts $i\n}\n",
    );
}

/// TP — every `if` clause shape: implicit and explicit `then`, repeated
/// `elseif`, and the bare implicit-else body.
#[test]
fn every_if_clause_shape_formats() {
    check(
        "if {$a} {\nx\n} elseif {$b} then {\ny\n} elseif {$c} {\nz\n} else {\nw\n}\n",
        "if {$a} {\n    x\n} elseif {$b} then {\n    y\n} elseif {$c} {\n    z\n} else {\n    w\n}\n",
    );
    // Implicit else — the trailing bare body is still a body.
    check(
        "if {$a} {\nx\n} {\ny\n}\n",
        "if {$a} {\n    x\n} {\n    y\n}\n",
    );
}

/// TP — every `try` handler shape.
#[test]
fn every_try_handler_shape_formats() {
    let out =
        fmt("try {\na\n} on error {m o} {\nb\n} trap {POSIX} {m o} {\nc\n} finally {\nd\n}\n");
    assert!(out.contains("} on error {m o} {"), "{out}");
    assert!(out.contains("} trap {POSIX} {m o} {"), "{out}");
    assert!(out.contains("} finally {"), "{out}");
}

/// FP guard — a *data* word that merely spells a keyword is not one. The
/// old scan matched by word value across every argument, so `puts else`
/// and `lappend l on trap` had their words retyped as keywords. The
/// registry answers by grammar position instead, so nothing fires for a
/// command whose spec declares no keyword there.
#[test]
fn keyword_lookalike_data_words_are_not_keywords() {
    check("puts else\n", "puts else\n");
    check("lappend l on trap finally\n", "lappend l on trap finally\n");
    // A user proc named `if` in a *namespace* is a different command
    // entirely — `ns::if` does not resolve to the built-in (only the
    // absolute global `::if` does), so its braced argument is left as a
    // plain word rather than reformatted as a control-flow body.
    check("ns::if {$a} {\nx\n}\n", "ns::if {$a} {\nx\n}\n");
}

/// FP guard — `if {1} {a} else then`: the trailing `then` sits in the
/// else-branch **body** slot, so the grammar walk gives it the body role,
/// not `Keyword` (verified against tclsh 8.6 / 9.0.4, which run `a` and
/// treat `then` as the else-branch script). The old value-matching scan
/// retyped it as a keyword wherever it appeared.
#[test]
fn then_in_the_else_body_slot_is_not_a_keyword() {
    let registry = CommandRegistry::build_default();
    let keywords =
        registry.arg_indices_for_role("if", &["1", "a", "else", "then"], ArgRole::Keyword);
    assert_eq!(
        keywords,
        vec![2],
        "only the real `else` at index 2 is a keyword"
    );
    // And formatting stays a fixed point over the shape.
    let once = fmt("if {1} {a} else then\n");
    assert_eq!(fmt(&once), once, "{once}");
}

/// TN — a dynamic command head (`{*}$cmd`) has no resolvable identity, so
/// the engine leaves it alone entirely.
#[test]
fn dynamic_head_is_left_alone() {
    check(
        "{*}$cmd {$x} {\nputs hi\n}\n",
        "{*}$cmd {$x} {\nputs hi\n}\n",
    );
}

/// TN — an incomplete/malformed clause must stay semantics-preserving and
/// idempotent rather than being reshaped into a guess.
#[test]
fn malformed_clauses_stay_stable_and_idempotent() {
    for src in [
        "if {$a}\n",
        "if {$a} {\nx\n} elseif\n",
        "try\n",
        "for {set i 0} {$i < 3}\n",
    ] {
        let once = fmt(src);
        let twice = fmt(&once);
        assert_eq!(once, twice, "not idempotent for {src:?}:\n{once}\n{twice}");
    }
}

fn original_list_arg(
    source: &str,
    profile: &'static tcl_dialect::DialectProfile,
) -> (FormatterConfig, CommandArg) {
    let config = FormatterConfig::for_profile(profile);
    let lexer_config = config.lexer_config();
    let image = tcl_lexer::SourceImage::document(source);
    let plan = tcl_lexer::native_script_words_in(
        image,
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        lexer_config,
    )
    .unwrap();
    let word = &plan.commands[0].words[0];
    let policy = tcl_registry::InvocationDialect::of_profile(profile)
        .authored_name_policy()
        .unwrap();
    let key =
        tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
            word,
            WordValueRules::from_config(&lexer_config),
            policy,
        )
        .unwrap();
    let map = SourceMap::new(source);
    let tokens = Lexer::with_config(source, lexer_config)
        .tokenise_all()
        .unwrap();
    let mut command = parse_commands(source, &map, &tokens, lexer_config)
        .unwrap()
        .0
        .remove(0);
    let mut arg = command.args.remove(0);
    arg.kind = ArgKind::ParamList;
    arg.original_word = Some(word.clone());
    arg.original_input =
        Some(tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key));
    (config, arg)
}

#[test]
fn original_param_spacing_retains_native_children_and_opaque_units() {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = profile_of(name);
        for (source, expected) in [
            ("{a    b}", "{a b}"),
            (r"{a\ b    c}", "{{a b} c}"),
            (r"{{a\uD800 default}    b}", r"{{a\uD800 default} b}"),
        ] {
            let (config, arg) = original_list_arg(source, profile);
            let map = SourceMap::new(source);
            assert_eq!(
                render_param_list_arg(&map, &arg, &config, false),
                expected,
                "{name}"
            );
        }
        let opaque = r"{a\uD800    b}";
        let (config, arg) = original_list_arg(opaque, profile);
        let map = SourceMap::new(opaque);
        assert!(
            normalise_original_param_list(
                arg.original_word.as_ref().unwrap(),
                arg.original_input.as_ref().unwrap(),
                &config,
            )
            .is_none(),
            "{name}"
        );
        assert_eq!(
            render_param_list_arg(&map, &arg, &config, false),
            opaque,
            "{name}"
        );
        let source = "{😀    b}";
        let (config, arg) = original_list_arg(source, profile);
        let expected = if name.starts_with("tcl8.") {
            source
        } else {
            "{😀 b}"
        };
        assert_eq!(
            render_param_list_arg(&SourceMap::new(source), &arg, &config, false),
            expected,
            "{name}"
        );
    }
}

#[test]
fn original_param_spacing_declines_foreign_word_and_full_config() {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    let source = "{a    b}";
    let (config, mut arg) = original_list_arg(source, profile_of("tcl8.6"));
    let word = arg.original_word.as_ref().unwrap();
    let input = arg.original_input.as_ref().unwrap();
    let mut changed = config.clone();
    let mut lexer = config.lexer_config();
    lexer.strict_quoting = !lexer.strict_quoting;
    changed.lexer_config_override = Some(lexer);
    assert!(normalise_original_param_list(word, input, &changed).is_none());
    let (_, foreign) = original_list_arg(" {a    b}", profile_of("tcl8.6"));
    assert!(
        normalise_original_param_list(word, foreign.original_input.as_ref().unwrap(), &config)
            .is_none()
    );
    arg.original_input = None;
    assert_eq!(
        render_param_list_arg(&SourceMap::new(source), &arg, &config, false),
        source
    );
}

#[test]
fn original_lambda_spacing_checks_nested_formals_before_emission() {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = profile_of(name);
        for source in [r"{{a\uD800    b} {puts    ok}}", r"{a\\uD800 {puts    ok}}"] {
            let (config, mut arg) = original_list_arg(source, profile);
            arg.kind = ArgKind::LambdaLiteral;
            arg.formatted_body = Some("puts ok".to_owned());
            assert_eq!(
                render_lambda_literal_arg(
                    &SourceMap::new(source),
                    &arg,
                    &config,
                    "",
                    0,
                    false,
                    false,
                ),
                source,
                "{name}"
            );
        }
        let source = "{{a    b} {puts    ok} ::N}";
        let (config, mut arg) = original_list_arg(source, profile);
        arg.kind = ArgKind::LambdaLiteral;
        arg.formatted_body = Some("puts ok".to_owned());
        assert_eq!(
            render_lambda_literal_arg(&SourceMap::new(source), &arg, &config, "", 0, false, false,),
            "{{a b} { puts ok } ::N}",
            "{name}"
        );
    }
}

#[test]
fn original_source_formatting_keeps_opaque_formal_word_verbatim() {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let output = fmt_dialect(r"proc p {a\uD800    b} {return}", profile_of(name));
        assert!(output.contains(r"{a\uD800    b}"), "{name}: {output}");
        let output = fmt_dialect("proc p {a    b} {return}", profile_of(name));
        assert!(output.starts_with("proc p {a b} "), "{name}: {output}");
    }
}

#[test]
fn param_list_normalised() {
    check(
        "proc f {a    b   c} {\nreturn\n}\n",
        "proc f {a b c} {\n    return\n}\n",
    );
}

/// C Tcl 9 collapses the
/// backslash-newline in a pre-pass *before* the parameter word is
/// list-parsed (even inside braces), so `proc f {a\<newline> b}` has the
/// two required parameters `a` and `b`:
///
/// ```text
/// % proc f {a\
///  b} {return}
/// % list [llength [info args f]] [info args f]
/// 2 {a b}
/// ```
///
/// The old hand-rolled scanner emitted `{a\ b}` — one *optional*
/// parameter `a` defaulting to `b`, a silent arity change.
#[test]
fn param_list_backslash_newline_preserves_arity() {
    let out = fmt("proc f {a\\\n b} {return}\n");
    assert!(
        out.starts_with("proc f {a b} {"),
        "backslash-newline changed the parameter list:\n{out}"
    );
    assert!(
        !out.contains(r"a\ b"),
        "two parameters were fused into one defaulted parameter:\n{out}"
    );
    // The parsed signature agrees: two params, neither defaulted.
    let params = tcl_compiler::signature_scan::params::parse_param_list("a b", WordValueRules::TCL);
    assert_eq!(params.len(), 2);
    assert!(params.iter().all(|p| !p.has_default));
}

/// FP guard — an *escaped space* really is one element (parameter `a`
/// defaulting to `b`), and must stay one element. The renderer re-quotes
/// it canonically as `{a b}`, which C Tcl reads identically.
#[test]
fn param_list_escaped_space_stays_one_parameter() {
    let out = fmt("proc f {a\\ b c} {return}\n");
    assert!(
        out.starts_with("proc f {{a b} c} {"),
        "escaped-space spec lost its element identity:\n{out}"
    );
    let params =
        tcl_compiler::signature_scan::params::parse_param_list("{a b} c", WordValueRules::TCL);
    assert_eq!(params.len(), 2);
    assert_eq!(params[0].name, "a");
    assert_eq!(params[0].default_value.as_deref(), Some("b"));
}

/// TN — a parameter list that is not a well-formed Tcl list (mid-edit
/// unmatched brace) has no canonical rendering, so the source is
/// preserved verbatim rather than rewritten into something else.
#[test]
fn param_list_malformed_preserved_verbatim() {
    // `a "b` is a brace-balanced word, so the formatter sees it as the
    // parameter list — but it is not a well-formed *list* (unmatched
    // quote), so there is nothing safe to re-render.
    let out = fmt("proc f {a \"b} {return}\n");
    assert!(
        out.contains("{a \"b}"),
        "malformed parameter list was rewritten:\n{out}"
    );
}

/// Structure-preserving cases: defaults, nested braces, `args`, empty
/// defaults, and Unicode all survive a format round-trip, and formatting
/// is idempotent.
#[test]
fn param_list_shapes_round_trip_and_are_idempotent() {
    for src in [
        "proc f {a {b 1} c} {return}\n",
        "proc f {args} {return}\n",
        "proc f {{a {}} args} {return}\n",
        "proc f {{opts {-x 1 -y 2}}} {return}\n",
        "proc f {naïve {héllo wörld}} {return}\n",
        "proc f {a\\\r\n b} {return}\n",
    ] {
        let once = fmt(src);
        let twice = fmt(&once);
        assert_eq!(once, twice, "not idempotent for {src:?}:\n{once}\n{twice}");
    }
    // Defaults keep their element boundaries.
    assert!(fmt("proc f {a   {b 1}   c} {return}\n").starts_with("proc f {a {b 1} c} {"));
    // A nested-brace default is one element, braces intact.
    assert!(fmt("proc f {{opts {-x 1}}} {return}\n").starts_with("proc f {{opts {-x 1}}} {"));
    // CRLF continuations behave exactly like LF ones.
    assert!(fmt("proc f {a\\\r\n b} {return}\n").starts_with("proc f {a b} {"));
}

#[test]
fn quoted_string_preserved() {
    check("puts \"hello $name\"\n", "puts \"hello $name\"\n");
}

/// `apply`'s lambda-literal argument (`ArgRole::LambdaLiteral`,
/// not `Body`) must have its real body element reformatted — not the
/// whole `{argList} {body}` blob re-segmented as one script (which
/// misreads the parameter word as a command name and never reaches the
/// real body). The parameter list is normalised the same way a `proc`'s
/// is (bare `dir` → braced `{dir}`), matching `param_list_normalised`.
#[test]
fn apply_lambda_body_indents_and_params_normalise() {
    check(
        "apply {dir {\nputs    $dir\nset x 1\n}} /tmp\n",
        "apply {{dir} {\n    puts $dir\n    set x 1\n}} /tmp\n",
    );
}

/// The optional namespace element (lambda literal position 2) survives
/// reassembly untouched. A short single-command body inlines (matching
/// ordinary body formatting), so this also confirms the inline path
/// reassembles the namespace element correctly.
#[test]
fn apply_lambda_namespace_element_preserved() {
    check(
        "apply {dir {\nputs    $dir\n} ::foo} /tmp\n",
        "apply {{dir} { puts $dir } ::foo} /tmp\n",
    );
}

/// A bare body element's backslash
/// escape must be decoded before reformatting, not reformatted from its
/// raw source spelling — `puts\ hi`'s real runtime body is the two-word
/// command `puts hi`, not one word containing a literal backslash.
#[test]
fn apply_lambda_body_with_backslash_escape_decodes() {
    check(r"apply {{} puts\ hi}", "apply {{} { puts hi }}\n");
}

#[test]
fn force_split_long_line() {
    check(
        "mycommand argumentone argumenttwo argumentthree argumentfour argumentfive argumentsix argumentseven argumenteight argumentnine\n",
        "mycommand argumentone argumenttwo argumentthree argumentfour argumentfive argumentsix argumentseven argumenteight \\\n    argumentnine\n",
    );
}

#[test]
fn expression_wrapping_preserves_string_terms_and_declines_comments() {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    let config = FormatterConfig::for_profile(profile_of("tcl9.0"));
    let expression = "\"a  b\" eq \"a  b\" && [string equal {c   d} {c   d}]";
    let wrapped = wrap_braced_expr(expression, &config, 0).unwrap();
    assert!(wrapped.contains("\"a  b\" eq \"a  b\""), "{wrapped}");
    assert!(
        wrapped.contains("[string equal {c   d} {c   d}]"),
        "{wrapped}"
    );
    assert!(wrap_braced_expr("1 # && comment\n&& 2", &config, 0).is_none());
    assert!(wrap_braced_expr("1 && \"missing quote", &config, 0).is_none());
}

#[test]
fn bracket_continuations_preserve_nested_word_data() {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    let source = "set x [list \\\n \"a \\\n b\" {c \\\n d}]\n";
    let result = fmt(source);
    assert!(
        result.contains("[list \"a \\\n b\" {c \\\n d}]"),
        "{result:?}"
    );
}

#[test]
fn multiline_braced_data_keeps_its_original_continuation() {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    let source = "set x {a \\\n b}\n";
    for name in ["tcl8.4", "tcl8.6", "tcl9.0", "jim"] {
        let profile = profile_of(name);
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        assert_eq!(
            format_tcl(
                source,
                &FormatterConfig::for_profile(profile),
                context.commands()
            ),
            source,
            "{name}"
        );
    }
}

#[test]
fn force_expr_wrap() {
    check(
        "if {$variableaaaa == 1 && $variablebbbb == 2 && $variablecccc == 3 && $variabledddd == 4 && $variableeeee == 5 && $variableffff == 6} {\nputs hi\n}\n",
        "if {\n    $variableaaaa == 1\n    && $variablebbbb == 2\n    && $variablecccc == 3\n    && $variabledddd == 4\n    && $variableeeee == 5\n    && $variableffff == 6\n} {\n    puts hi\n}\n",
    );
}

#[test]
fn semicolon_splits_commands() {
    check("set x 1; set y 2\n", "set x 1\nset y 2\n");
}

#[test]
fn empty_proc_body() {
    check("proc f {} {}\n", "proc f {} {}\n");
}

#[test]
fn try_finally() {
    check(
        "try {\nfoo\n} finally {\nbar\n}\n",
        "try {\n    foo\n} finally {\n    bar\n}\n",
    );
}

#[test]
fn command_substitution_preserved() {
    check("set y [expr {1+2}]\n", "set y [expr {1+2}]\n");
}

#[test]
fn expand_prefix_preserved() {
    check("puts {*}$args\n", "puts {*}$args\n");
}

#[test]
fn multi_hash_comment() {
    check("## section\nputs hi\n", "## section\nputs hi\n");
}

#[test]
fn trailing_comment() {
    check("puts hi\n# trailing\n", "puts hi\n# trailing\n");
}

#[test]
fn deeply_nested_bodies() {
    check(
        "proc f {} {\nforeach x $list {\nif {$x} {\nputs $x\n}\n}\n}\n",
        "proc f {} {\n    foreach x $list {\n        if {$x} {\n            puts $x\n        }\n    }\n}\n",
    );
}

#[test]
fn enforce_braced_variables_rewrites_dollar_refs() {
    let registry = CommandRegistry::build_default();
    let cfg = FormatterConfig {
        enforce_braced_variables: true,
        ..FormatterConfig::default()
    };
    assert_eq!(format_tcl("puts $x\n", &cfg, &registry), "puts ${x}\n");
}

#[test]
fn procedure_spacing_uses_selected_source_traits() {
    // naming.editor.original-source-formatting
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
    let profile = profile_of("tcl8.6");
    let registry = tcl_registry::model::ingress::context_for_profile(profile);
    let config = FormatterConfig {
        blank_lines_between_procs: 2,
        blank_lines_between_blocks: 1,
        ..FormatterConfig::for_profile(profile)
    };
    let qualified = format_tcl(
        "::proc first {} {}\n::proc second {} {}\n",
        &config,
        registry.commands(),
    );
    assert_eq!(qualified, "::proc first {} {}\n\n\n::proc second {} {}\n");
    let shadowed = format_tcl(
        "proc proc {args} {}\nproc first\nproc second\n",
        &config,
        registry.commands(),
    );
    assert!(
        shadowed.ends_with("proc first\nproc second\n"),
        "{shadowed}"
    );
    let source = "proc first {} {}\nproc second {} {}\n";
    let identities = FormattingSourceLayout::new(source, &config, registry.commands());
    let mut foreign = config.clone();
    foreign.lexer_config_override = Some(tcl_lexer::LexerConfig {
        strict_quoting: !config.lexer_config().strict_quoting,
        ..config.lexer_config()
    });
    let unchanged = format_body(
        source,
        &foreign,
        registry.commands(),
        &identities,
        FormattingRegion::document(&foreign),
    );
    assert_eq!(unchanged, source.trim_end_matches('\n'));
}

#[test]
fn blank_lines_between_procs() {
    check(
        "proc a {} {\nreturn\n}\nproc b {} {\nreturn\n}\n",
        "proc a {} {\n    return\n}\n\nproc b {} {\n    return\n}\n",
    );
}
/// The formatter must lay a command out under the grammar
/// of the command it *is*, not the one it is spelled as.
///
/// An `ArgRole::Body` argument is expanded onto its own lines, so a
/// rebound body-bearing command resolved by spelling would be formatted
/// under a grammar it no longer has.
///
/// tclsh oracle (8.6.16 and 9.0.4, byte-identical): `interp alias {} maybe
/// {} if` makes `maybe` run `if`; `rename if maybe` moves it and leaves
/// `if` gone; a top-level `proc if …` takes the name over.
fn body_was_expanded(src: &str) -> bool {
    fmt(src).contains("{\n    puts a\n}")
}

#[test]
fn formatting_follows_an_aliased_body_command() {
    assert!(body_was_expanded(
        "interp alias {} maybe {} if\nmaybe {$x} {puts a}\n"
    ));
    // The `::`-qualified spelling of the alias classifies alike.
    assert!(body_was_expanded(
        "interp alias {} maybe {} if\n::maybe {$x} {puts a}\n"
    ));
    // Guard: an unbound `maybe` has no body argument, so its braced word
    // is left exactly as written.
    assert!(!body_was_expanded("set y 1\nmaybe {$x} {puts a}\n"));
}

#[test]
fn formatting_follows_a_renamed_body_command() {
    assert!(body_was_expanded("rename if maybe\nmaybe {$x} {puts a}\n"));
    // The old spelling is gone from the rename onwards.
    assert!(
        !body_was_expanded("rename if maybe\nif {$x} {puts a}\n"),
        "a renamed-away `if` must not keep the built-in's layout"
    );
}

#[test]
fn formatting_abstains_for_a_builtin_shadowed_by_a_user_proc() {
    assert!(
        !body_was_expanded("proc if {c b} { return 1 }\nif {$x} {puts a}\n"),
        "a user `proc if` takes the name over; no registry layout applies"
    );
    // Guard: the unshadowed built-in still expands.
    assert!(body_was_expanded("set y 1\nif {$x} {puts a}\n"));
}

#[test]
fn formatting_abstains_for_a_dynamic_binding() {
    assert!(
        !body_was_expanded("rename $old maybe\nmaybe {$x} {puts a}\n"),
        "a dynamic rename must not give `maybe` a body layout"
    );
    assert!(
        body_was_expanded("rename $old maybe\nif {$x} {puts a}\n"),
        "a dynamic rename must not take `if`'s layout away either"
    );
}

/// The clause-list layout is head-driven too: `switch`'s pattern/body
/// pairs are laid out by [`format_case_list_body`], which the reconstruction
/// picks by the *resolved* head.
#[test]
fn formatting_follows_an_aliased_clause_list_command() {
    let formatted = fmt("interp alias {} pick {} switch\npick $v {a {puts 1} b {puts 2}}\n");
    assert!(
        formatted.contains("a {\n        puts 1\n    }"),
        "an alias of `switch` must get the clause-list layout; got:\n{formatted}"
    );
    // Guard: an unbound `pick` keeps its braced word verbatim.
    let formatted = fmt("set y 1\npick $v {a {puts 1} b {puts 2}}\n");
    assert!(!formatted.contains("a {\n        puts 1\n    }"));
}

fn case_list_action_was_expanded(formatted: &str, marker: &str) -> bool {
    formatted.contains(&format!("{{\n        puts {marker}\n    }}"))
}

#[test]
fn case_list_alias_formatting_uses_the_binding_at_each_call_position() {
    let source = concat!(
        "pick subject {default {puts before_alias}}\n",
        "interp alias {} pick {} switch\n",
        "pick subject {default {puts through_alias}}\n",
        "interp alias {} pick {}\n",
        "pick subject {default {puts deleted_alias}}\n",
        "interp alias {} pick {} puts\n",
        "pick subject {default {puts rebound_other}}\n",
        "interp alias {} pick {} switch\n",
        "pick subject {default {puts rebound_switch}}\n",
    );
    let formatted = fmt(source);

    for opaque in ["before_alias", "deleted_alias", "rebound_other"] {
        assert!(
            formatted.contains(&format!("default {{puts {opaque}}}")),
            "binding state at {opaque} was applied retroactively:\n{formatted}"
        );
        assert!(!case_list_action_was_expanded(&formatted, opaque));
    }
    for active in ["through_alias", "rebound_switch"] {
        assert!(
            case_list_action_was_expanded(&formatted, active),
            "live switch alias did not format {active}:\n{formatted}"
        );
    }
    assert_eq!(fmt(&formatted), formatted);
}

#[test]
fn case_list_rename_formatting_uses_the_binding_at_each_call_position() {
    let source = concat!(
        "pick subject {default {puts before_rename}}\n",
        "rename switch pick\n",
        "pick subject {default {puts after_rename}}\n",
        "rename pick {}\n",
        "pick subject {default {puts after_delete}}\n",
    );
    let formatted = fmt(source);
    for opaque in ["before_rename", "after_delete"] {
        assert!(formatted.contains(&format!("default {{puts {opaque}}}")));
        assert!(!case_list_action_was_expanded(&formatted, opaque));
    }
    assert!(case_list_action_was_expanded(&formatted, "after_rename"));
    assert_eq!(fmt(&formatted), formatted);
}

#[test]
fn rooted_and_namespaced_case_list_aliases_format_at_their_positions() {
    let source = concat!(
        "::root_pick subject {default {puts before_root}}\n",
        "::case_alias::pick subject {default {puts before_namespace}}\n",
        "namespace eval ::case_alias {}\n",
        "interp alias {} ::root_pick {} ::switch\n",
        "interp alias {} ::case_alias::pick {} switch\n",
        "::root_pick subject {default {puts rooted_alias}}\n",
        "::case_alias::pick subject {default {puts namespaced_alias}}\n",
    );
    let formatted = fmt(source);
    for opaque in ["before_root", "before_namespace"] {
        assert!(formatted.contains(&format!("default {{puts {opaque}}}")));
        assert!(!case_list_action_was_expanded(&formatted, opaque));
    }
    for active in ["rooted_alias", "namespaced_alias"] {
        assert!(
            case_list_action_was_expanded(&formatted, active),
            "qualified switch alias did not format {active}:\n{formatted}"
        );
    }
    assert_eq!(fmt(&formatted), formatted);
}
