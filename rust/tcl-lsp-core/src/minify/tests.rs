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
fn original_minifier_body_parsing_keeps_bom_content_and_complete_parent_config() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Original source geometry only; this does not grant body execution.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..LexerConfig::for_file_grammar(profile.grammar)
    };
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    for source in [
        "proc p {} {\u{feff}puts    1}",
        "set x [\u{feff}puts    1]",
        "apply {{} {\u{feff}puts    1}}",
        "switch x {x {\u{feff}puts    1}}",
    ] {
        let analysis = original::analysis_with_input(source, &input);
        let result = lexical_default(source, &analysis).unwrap();
        assert_eq!(
            result.matches('\u{feff}').count(),
            1,
            "{source:?}: {result:?}"
        );
        assert_eq!(analysis.resolved_input.as_ref(), Some(&input));
        assert!(
            analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        );
    }
    let source = "\u{feff}puts    1";
    let analysis = original::analysis_with_input(source, &input);
    assert_eq!(lexical_default(source, &analysis).unwrap(), "puts 1");
}

fn min(src: &str) -> String {
    let registry = CommandRegistry::build_default();
    minify_tcl(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &registry,
    )
}

/// A `${…}` whose name is not a bare `$name` word may never shed its
/// braces, whatever follows it: `${a{b}c}` reads the variable `a{b}c`,
/// while `$a{b}c` reads `a` and appends the literal `{b}c`.
///
/// Found while pinning the plumb above — minify dropped the braces
/// because it only asked whether the *next* token would extend the
/// reference, never whether the name itself could be written bare.
#[test]
fn a_non_bare_braced_name_keeps_its_braces() {
    let registry = CommandRegistry::build_default();
    for release in ["tcl9.0", "tcl8.6"] {
        let out = minify_tcl(
            "set {a b} 1\nputs ${a b}\n",
            tcl_registry::model::ingress::resolve_environment(release).analyser_profile(),
            &registry,
        );
        assert!(
            out.contains("${a b}"),
            "a name with a space must stay braced on {release}: {out}"
        );
    }
    // A plain name still sheds them.
    let out = minify_tcl(
        "set plain 1\nputs ${plain}\n",
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
    );
    assert!(out.contains("$plain"), "{out}");
    assert!(!out.contains("${plain}"), "{out}");
}

#[test]
fn expression_commands_keep_configured_literal_closers_and_unavailable_extents() {
    for label in ["tcl8.6", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(label).analyser_profile();
        for expression in [
            "[list {literal ] value}] + 1",
            "[list \"literal ] value\"] + 1",
            "[list [list {literal ] value}]] + 1",
            "[\n# literal ]\nlist {literal [ value}\n] + 1",
        ] {
            let source = format!("expr {{{expression}}}");
            let output = min_dialect(&source, profile);
            let data = if expression.contains("literal [ value") {
                "literal [ value"
            } else {
                "literal ] value"
            };
            assert!(output.contains(data), "{label}: {output}");
            let opener = output.find('[').unwrap();
            let end = tcl_lexer::command_subst_close(
                output.as_bytes(),
                opener,
                tcl_lexer::SubstFlags::default(),
                LexerConfig::for_file_grammar(profile.grammar),
            )
            .unwrap();
            assert_eq!(&output[end..], "+1}", "{label}: {output}");
        }
        let source = "expr {[é + 1}";
        let registry = CommandRegistry::build_default();
        let analysis = original::analysis_for_profile(source, profile, &registry);
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let env = MinifyEnv {
            original_source: source,
            analysis: &analysis,
            input,
            dialect: input.unit_profile(),
            config: input.lexer_config(),
            registry: context.commands(),
            context: &context,
            identities: analysis.retained_command_realm().unwrap(),
        };
        let expression = "[é + 1";
        let tokens = tokenise_expr(expression, env, 0);
        assert!(matches!(tokens.as_slice(), [ExprTok::Other(value)] if value == expression));
        assert_eq!(strip_expr_whitespace(expression, env, 0), expression);
    }
}

#[test]
fn expression_minification_preserves_original_array_index_name_data() {
    for label in ["tcl8.6", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(label).analyser_profile();
        for reference in ["$arr(a + b)", "$arr($key + 1)", "${$literal}", "${}"] {
            let source = format!("expr {{{reference} + 1}}");
            let output = min_dialect(&source, profile);
            assert!(output.contains(reference), "{label}: {output}");
        }
    }
}

/// The **plumb**: minify must take the `${…}` rule from the document's
/// own dialect, not a constant. A quoted `"${a{b}c}"` may shed its
/// quotes on 9.x (the whole word is one variable reference) but not on
/// 8.x, where the reference stops at the first `}` and the residual `c}`
/// is a bare brace.
#[test]
fn quote_stripping_follows_the_documents_dialect() {
    let registry = CommandRegistry::build_default();
    let src = "puts \"${a{b}c}\"\n";
    let nine = minify_tcl(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
    );
    let eight = minify_tcl(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &registry,
    );
    assert!(
        !nine.contains('"'),
        "9.x reads one reference, so the quotes may go: {nine}"
    );
    assert!(
        eight.contains('"'),
        "8.x leaves a bare `c}}`, so the quotes must stay: {eight}"
    );
}

/// Minify decides whether a quoted word may drop its
/// quotes by checking for braces left over after `${…}` references are
/// removed, so the removal must use the document's own `${…}` close
/// rule.
///
/// Oracle: `puts ${a{b}c}` prints `NINE` on tclsh 9.0.4 (the whole thing
/// is one reference, so nothing brace-like remains) and `EIGHTc}` on
/// 8.6.16 (the reference ends at the first `}` and the residual `c}`
/// really is a bare brace).
#[test]
fn braced_var_stripping_follows_the_documents_release() {
    use tcl_dialect::BracedVarStyle;
    // 9.x: one reference, nothing left behind.
    assert_eq!(
        strip_braced_var_refs("${a{b}c}", BracedVarStyle::Tcl9Nesting),
        ""
    );
    // 8.x: the reference is `${a{b}`, so `c}` survives as a bare brace.
    assert_eq!(
        strip_braced_var_refs("${a{b}c}", BracedVarStyle::FirstClose),
        "c}"
    );
    // …which is exactly what decides whether the quotes may go.
    assert!(can_strip_quotes("${a{b}c}", BracedVarStyle::Tcl9Nesting));
    assert!(!can_strip_quotes("${a{b}c}", BracedVarStyle::FirstClose));
}

#[test]
fn quote_stripping_uses_complete_configured_components() {
    let registry = CommandRegistry::build_default();
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    for source in [
        "puts \"[list {a b}]\"",
        "puts \"[list \"a b\"]\"",
        "puts \"${x}tail\"",
        "puts \"${x}(key)\"",
        "puts \"$array($index)\"",
        "puts \"a\\ b\"",
    ] {
        let analysis = original::analysis_for_profile(source, profile, &registry);
        let config = analysis.body_lexer_config.unwrap();
        let output = lexical_default(source, &analysis).unwrap();
        let original = lexical_script_words(source, config).unwrap();
        let proposed = output.strip_prefix("puts ").unwrap();
        if source == "puts \"[list {a b}]\"" {
            assert_eq!(output, "puts [list {a b}]");
        }
        assert!(
            tcl_syntax::word_rules::lexical_source_word_preserves_components(
                &original.commands[0].words[1],
                proposed,
            ),
            "{source} -> {output}"
        );
    }
}

/// The `$var` reader behind constant folding and the live-name sweep
/// splits the same bytes at the same release-dependent point.
#[test]
fn parse_var_ref_reads_braced_names_by_the_documents_release() {
    use tcl_dialect::BracedVarStyle;
    let text = "${a{b}c}";
    assert_eq!(
        parse_var_ref(text, 0, BracedVarStyle::Tcl9Nesting),
        (8, Some("a{b}c"))
    );
    assert_eq!(
        parse_var_ref(text, 0, BracedVarStyle::FirstClose),
        (6, Some("a{b"))
    );
    // An unterminated `${` yields no name under either rule.
    assert_eq!(
        parse_var_ref("${abc", 0, BracedVarStyle::Tcl9Nesting),
        (1, None)
    );
}

/// `minify_body`'s recursive
/// descent (shared with `render_command`, `reconstruct_arg`/
/// `reconstruct_raw`, `minify_switch_case_list`, `minify_lambda_literal`,
/// `compress_expr`/`tokenise_expr`) is capped at `MAX_MINIFY_DEPTH`
/// (128), mirroring `formatting::engine`'s `MAX_FORMAT_DEPTH` and the
/// same empirical crash-range reasoning documented there (this
/// recursion's per-level shape is close enough not to warrant a
/// separate calibration run). 2000 is comfortably past the cap; the
/// assertion is that minifying returns at all, not what it returns.
#[test]
fn deeply_nested_if_survives_minify() {
    const DEPTH: usize = 2000;
    let mut src = String::new();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("set done 1\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    let _ = min(&src);
}

/// A moderately nested body (well under `MAX_MINIFY_DEPTH`) still
/// minifies normally — the safety net must not fire on realistic
/// nesting depths.
#[test]
fn moderately_nested_if_still_minifies() {
    let out = min("if {1} {\nif {2} {\nset x 1\n}\n}\n");
    assert!(out.contains("set x 1"), "{out:?}");
}

#[test]
fn bare_dollar_in_quoted_string_not_brace_wrapped() {
    // Minifying a lone `$` inside a double-quoted string
    // must keep it literal, not emit `{$}`.
    let out = min("puts \"cost: $\"\n");
    assert!(out.contains("\"cost: $\""), "{out:?}");
    assert!(!out.contains("{$}"), "bare $ was brace-wrapped: {out:?}");
}

fn check(input: &str, expected: &str) {
    let got = min(input);
    assert_eq!(
        got, expected,
        "\ninput:    {input:?}\ngot:      {got:?}\nexpected: {expected:?}"
    );
}

#[test]
fn strips_comments() {
    check("# a comment\nputs hi\n", "puts hi");
}

#[test]
fn keeps_braces_when_next_token_extends_bare_word() {
    // A bare word `${a}jumps` must keep its braces — dropping
    // them yields `$ajumps`, reading a different variable.
    let out = min("set a 1\nputs ${a}jumps\n");
    assert!(
        out.contains("${a}jumps"),
        "bare-word ${{a}}jumps was unbraced unsafely:\n{out}"
    );
    assert!(
        !out.contains("$ajumps"),
        "unsafe unbrace produced $ajumps:\n{out}"
    );
}

#[test]
fn keeps_braces_before_paren_in_quoted_word() {
    // `"${x}(k)"` must not become `"$x(k)"` (array element).
    let out = min("set x 1\nputs \"${x}(k)\"\n");
    assert!(
        out.contains("${x}(k)"),
        "`(` after ${{x}} must keep braces (array-element hazard):\n{out}"
    );
}

#[test]
fn keeps_braces_before_namespace_separator() {
    // `${x}::y` unbraced reads variable `x::y`, not `$x` + literal `::y`.
    let out = min("set x 1\nputs \"${x}::y\"\n");
    assert!(
        out.contains("${x}::y"),
        "`::` after ${{x}} must keep braces:\n{out}"
    );
}

#[test]
fn still_unbraces_when_safe() {
    // A following non-extending char (`.`) is safe to unbrace.
    let out = min("set a 1\nputs ${a}.txt\n");
    assert!(
        out.contains("$a.txt"),
        "safe unbrace did not happen:\n{out}"
    );
}

fn min_dialect(src: &str, dialect: &'static tcl_dialect::DialectProfile) -> String {
    let registry = CommandRegistry::build_default();
    minify_tcl(src, dialect, &registry)
}

fn min_compact(src: &str) -> String {
    let registry = CommandRegistry::build_default();
    minify_tcl_compact(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        false,
        &registry,
    )
    .0
}

fn min_compact_isolated(src: &str) -> String {
    let registry = CommandRegistry::build_default();
    minify_tcl_compact(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        true,
        &registry,
    )
    .0
}

#[test]
fn compact_isolated_keeps_procs_when_reflection_present() {
    // `info procs` reflects proc names (registry
    // REFLECTS_COMMAND_NAMES), so even `isolated` must keep them.
    // tclsh 9.0.4: renaming only the definition+call leaves
    // `info procs longprocedure` returning an empty list — an
    // observable change.
    let out = min_compact_isolated(
        "proc longprocedure {} {return ok}\nputs [info procs longprocedure]\nputs [longprocedure]\n",
    );
    assert!(out.contains("proc longprocedure {}"), "{out}");
    assert!(out.contains("info procs longprocedure"), "{out}");
}

#[test]
fn compact_keeps_locals_when_info_locals_present() {
    // `info locals` reflects local variable names (registry
    // INTROSPECTS_BY_NAME), so the scope must not be renamed.
    // tclsh 9.0.4: the original prints `longvariable`; a compacted
    // `set a 1` would print `a`.
    let out = min_compact("proc f {} {\n    set longvariable 1\n    return [info locals]\n}\n");
    assert_eq!(out, "proc f {} {set longvariable 1;return [info locals]}");
}

#[test]
fn compact_keeps_locals_when_info_exists_present() {
    // `info exists NAME` reads a variable by bare name; renaming the
    // `set` site but not the introspection flips the result 1 → 0.
    let out = min_compact("proc f {} {\n    set myvar 1\n    return [info exists myvar]\n}\n");
    assert_eq!(out, "proc f {} {set myvar 1;return [info exists myvar]}");
}

#[test]
fn compact_never_splits_unset_from_its_variable() {
    // `unset myvar` names the variable as a bare argument the
    // analyser does not link as a reference, so `myvar` must be
    // left unrenamed everywhere (renaming only the `set` site
    // would unset a different variable).  Unrelated locals still
    // compact.
    let out = min_compact(
        "proc f {} {\n    set myvar 1\n    unset myvar\n    set other 2\n    return $other\n}\n",
    );
    // Naming passes are refused until a shared transactional rename
    // planner exists, so every name stays as written.
    assert_eq!(
        out,
        "proc f {} {set myvar 1;unset myvar;set other 2;return $other}"
    );
}

#[test]
fn compact_upvar_blocks_all_scopes() {
    // `upvar` aliases a caller frame chosen at runtime (registry
    // ALIASES_CALLER_FRAME): any proc could be the caller, so no
    // scope may rename its locals while one exists.  tclsh 9.0.4:
    // renaming `callervar` in `use` breaks `get`'s upvar target.
    let out = min_compact(
        "proc get {} {\n    upvar 1 callervar m\n    return $m\n}\nproc use {} {\n    set callervar 5\n    return [get]\n}\n",
    );
    assert!(out.contains("set callervar 5"), "{out}");
}

#[test]
fn compact_renames_param_name_not_other_defaults() {
    // Renaming param `x` must touch only its NAME, not a
    // literal `x` sitting in another param's default value. Here `{y x}`
    // gives `y` the default string `x`; that `x` must survive the `x`→…
    // rename, otherwise the proc's default silently changes.
    let out = min_compact("proc f {{alpha 1} {beta alpha}} {\n    return [list $alpha $beta]\n}\n");
    // The name `alpha` is renamed; `beta`'s default literal `alpha` is
    // left intact (it is the string value `alpha`, not the parameter).
    // Naming passes are refused until a shared transactional rename
    // planner exists, so every name stays as written.
    assert!(out.contains("{alpha 1}"), "alpha name kept: {out:?}");
    assert!(out.contains("{beta alpha}"), "beta default intact: {out:?}");
}

#[test]
fn compact_variable_reads_keep_original_braced_names_and_current_source() {
    let source =
        "proc helper {} {set longVariable VALUE; puts ${longVariable}; puts $longVariable}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    // Naming passes are refused until a shared transactional rename
    // planner exists, so every name stays as written.
    assert_eq!(
        min_compact(source),
        "proc helper {} {set longVariable VALUE;puts ${longVariable};puts $longVariable}"
    );
    let scope = analysis
        .global_scope
        .children
        .iter()
        .find(|scope| scope.kind == ScopeKind::Proc)
        .unwrap();
    let variable = &scope.variables["longVariable"];
    let geometry = crate::source_structure::OriginalVariableReferences::capture(source, &analysis)
        .expect("current source geometry");
    let spans =
        variable_reference_name_spans(source, &geometry, "longVariable", &variable.references)
            .expect("actual configured original reference names");
    assert_eq!(spans.len(), 2);
    assert!(
        spans
            .iter()
            .all(|span| source.get(span.as_range()) == Some("longVariable"))
    );
    assert!(
        variable_reference_name_spans(source, &geometry, "donated", &variable.references).is_none()
    );
    let mut stale = analysis.clone();
    stale.resolved_input = None;
    assert!(crate::source_structure::OriginalVariableReferences::capture(source, &stale).is_none());
}

#[test]
fn compact_renames_refs_inside_expr_and_command_subst() {
    // The `$value` ref lives inside `[expr {...}]`; it must be
    // renamed in lock-step with the param declaration (relies on
    // the analyser tracking expr/command-subst references).  Proc
    // names stay — public identities in the non-isolated tier.
    // Naming passes are refused until a shared transactional rename
    // planner exists, so every name stays as written.
    assert_eq!(
        min_compact(
            "proc helper {value} {\n    return [expr {$value * 2}]\n}\nproc main {} {\n    set result [helper 21]\n    puts $result\n}\n"
        ),
        "proc helper {value} {return [expr {$value*2}]};proc main {} {set result [helper 21];puts $result}",
    );
}

#[test]
fn unminify_error_round_trips_via_symbol_map() {
    // The compact tier renames nothing until a shared rename planner exists,
    // so the map an earlier minification would have reported is built here.
    let mut sym = SymbolMap::default();
    sym.procs.insert("greet".to_owned(), "a".to_owned());
    sym.variables.insert(
        "::greet".to_owned(),
        BTreeMap::from([("name".to_owned(), "a".to_owned())]),
    );
    // Procs win the bare-name reverse entry, so `a` -> `greet`.
    assert_eq!(
        unminify_error("can't read \"a\": no such variable", &sym),
        "can't read \"greet\": no such variable",
    );
    assert_eq!(
        unminify_error("invalid command name \"a\"", &sym),
        "invalid command name \"greet\"",
    );
}

#[test]
fn remap_line_references_maps_proportionally() {
    let orig = "proc f {} {\n    set x 1\n    set y 2\n    set z 3\n}\n";
    let mini = "proc f {} {set x 1;set y 2;set z 3}";
    // min_commands = 3; line 2 -> proportional original line 3.
    assert_eq!(
        remap_line_references("error at line 2", mini, orig),
        "error at line 3 (minified line 2)",
    );
}

#[test]
fn remap_line_references_procline_single_pass() {
    let orig = "proc f {} {\n    set x 1\n    set y 2\n    set z 3\n}\n";
    let mini = "proc f {} {set x 1;set y 2;set z 3}";
    // Single pass: line 3 -> 5.
    assert_eq!(
        remap_line_references("(procedure \"f\" line 3)", mini, orig),
        "(procedure \"f\" line 5, minified line 3)",
    );
}

#[test]
fn unminify_error_parse_round_trip() {
    let mut sym = SymbolMap::default();
    sym.procs.insert("greet".to_owned(), "a".to_owned());
    let text = sym.format();
    let parsed = SymbolMap::parse(&text);
    assert_eq!(parsed.procs.get("greet").map(String::as_str), Some("a"));
    assert_eq!(
        unminify_error("calling $a now", &parsed),
        "calling $greet now"
    );
}

fn agg(src: &str) -> String {
    let registry = CommandRegistry::build_default();
    minify_tcl_aggressive(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        false,
        &registry,
    )
    .source
}

#[test]
fn original_static_folds_do_not_borrow_erasure_from_logical_source_values() {
    let source = "set value 42\nputs \"n=$value\"\n";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let registry = CommandRegistry::build_default();
    let analysis = original::analysis_for_profile(source, profile, &registry);
    let input = analysis.resolved_input.as_ref().unwrap();
    assert!(analysis.allows_retained_logical_declaration_advice());
    let (out, count, map) = fold_original_static_substrings(source, input);
    assert_eq!(out, source);
    assert_eq!(count, 0);
    assert!(map.is_empty());
}

#[test]
fn live_name_inventory_uses_the_actual_complete_reference_grammar() {
    let source = "# ${a{b}c} $a::value $a(index) $Ω\n";
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = CommandRegistry::build_default();
    let analysis = original::analysis_for_profile(source, profile, &registry);
    let input = analysis.resolved_input.as_ref().unwrap();
    let names = collect_live_names(source, input);
    for name in ["a{b}c", "a::value", "a", "Ω"] {
        assert!(names.names.contains(name), "{name}: {names:?}");
    }
}

#[test]
fn guarded_body_keeps_collision_candidates_but_not_absence_authority() {
    let body = format!(
        "# $retained_candidate {}\nmystery {{set a 1}}\n",
        "x".repeat(270_000)
    );
    let command = "repeatedlyLongCommandName";
    let source = format!("proc p {{}} {{{body}}}\n{command} x;{command} x;{command} x\n");
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = CommandRegistry::build_default();
    let analysis = original::analysis_for_profile(&source, profile, &registry);
    let input = analysis.resolved_input.as_ref().unwrap();
    let names = collect_live_names(&source, input);
    assert!(names.names.contains("retained_candidate"));
    assert!(
        names
            .gaps
            .contains(&LiveNameInventoryGap::ComplexityGuardedBody)
    );
    assert!(
        names
            .gaps
            .contains(&LiveNameInventoryGap::SourceSyntaxCoverage)
    );
    // These debts remain explicit, while this patch leaves the existing
    // aggressive authoring policy and its alias output unchanged.
    let result = lexical_aggressive(&source, input, false, false).unwrap();
    assert!(result.source.contains(command));
}

#[test]
fn conservative_collision_candidates_keep_configured_reads_and_insertion_separate() {
    let source = "# $Ω $café ${braced name}\nfoo --long-argument;foo --long-argument\n";
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = CommandRegistry::build_default();
    let analysis = original::analysis_for_profile(source, profile, &registry);
    let input = analysis.resolved_input.as_ref().unwrap();
    let names = collect_live_names(source, input);
    for name in ["Ω", "caf", "café", "braced name"] {
        assert!(names.names.contains(name), "{name}: {names:?}");
    }
    // Native Tcl keeps the non-ASCII suffix outside its bare reference.
    let unicode = source.find("$Ω").unwrap();
    assert!(
        tcl_lexer::scan_var_ref(source.as_bytes(), unicode, input.lexer_config())
            .unwrap()
            .is_none()
    );
    let mixed = source.find("$café").unwrap();
    assert_eq!(
        tcl_lexer::scan_var_ref(source.as_bytes(), mixed, input.lexer_config())
            .unwrap()
            .unwrap()
            .name,
        b"caf"
    );
    let result = minify_with_analysis(
        source,
        &analysis,
        MinifyOptions {
            tier: MinifyTier::Aggressive,
            isolated: true,
            abbreviations: true,
        },
    );
    assert!(
        result
            .refusals
            .contains(&MinifyRefusal::MissingAliasInsertionContract)
    );
    assert!(result.symbol_map.command_aliases.is_empty());
    assert!(result.symbol_map.argument_aliases.is_empty());
    assert_eq!(result.source, "foo --long-argument;foo --long-argument");
}

#[test]
fn static_fold_folds_sccp_constant_interpolation() {
    let registry = CommandRegistry::build_default();
    // `$x` is a proven integer constant; fold `"n=$x"` -> `n=5`.
    let (out, count, map) = fold_static_substrings(
        "set x 5\nputs \"n=$x\"\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &registry,
    );
    assert_eq!(out, "set x 5\nputs n=5\n");
    assert_eq!(count, 1);
    assert!(map.values().any(|v| v == "n=5"), "{map:?}");
}

#[test]
fn static_fold_quotes_carriage_return_value() {
    let registry = CommandRegistry::build_default();
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let (out, count, _) =
        fold_static_substrings("set x \"a\\rb\"\nputs \"v=$x\"\n", profile, &registry);
    assert_eq!(count, 1, "{out}");
    let word = out
        .split_once("\nputs ")
        .unwrap()
        .1
        .strip_suffix('\n')
        .unwrap();
    assert!(
        tcl_syntax::word_rules::lexical_unicode_source_word_matches(
            word,
            "v=a\rb",
            LexerConfig::for_profile(Some(profile)),
        ),
        "{out}"
    );
}

/// The quoted word embeds a command substitution whose own
/// argument is quoted. The shared close-quote scanner skips the whole
/// `[…]`, so no edit is ever anchored on the inner `"b"` — a scanner
/// stopping there would splice a replacement over `"a[string toupper "`
/// and leave `b"] $x"` dangling.
#[test]
fn static_fold_keeps_a_quoted_command_substitution_well_formed() {
    let registry = CommandRegistry::build_default();
    let src = "set x 5\nputs \"a[string toupper \"b\"] $x\"\n";
    let (out, count, _) = fold_static_substrings(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &registry,
    );
    assert_eq!(count, 0, "a command substitution is not SCCP-foldable");
    assert_eq!(out, src, "source must come back untouched, not truncated");
}

/// The multiline shape: the `]` sitting in a command-position comment
/// inside the substitution is inert in C Tcl (verified against tclsh
/// 8.6/9.0), so the quoted word runs to the final `"`.  A scanner that
/// stopped at that `]` would report the quote opening the inner `"b"` as
/// the closer, and any fold edge anchored there would splice over the
/// middle of valid source.
#[test]
fn static_fold_keeps_a_commented_command_substitution_well_formed() {
    let registry = CommandRegistry::build_default();
    let src = "set x 5\nputs \"a[\n# ] comment\nset y \"b\"\n]c $x\"\n";
    let (out, count, _) = fold_static_substrings(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &registry,
    );
    assert_eq!(count, 0, "a command substitution is not SCCP-foldable");
    assert_eq!(out, src, "source must come back untouched, not truncated");
}

#[test]
fn static_fold_skips_non_constant() {
    let registry = CommandRegistry::build_default();
    // `[HTTP::uri]` is dynamic — nothing folds.
    let (out, count, _) = fold_static_substrings(
        "set u [HTTP::uri]\nputs \"got $u\"\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &registry,
    );
    assert_eq!(count, 0);
    assert!(out.contains("$u"));
}

#[test]
fn string_alias_reference_preserves_original_six_provider_suffix_values() {
    // Controlled original providers: experiments/name-handling/results/minifier-alias-reference/oracles/controlled,
    // manifest 756df4fa5b54ea09a6c2310da741cfc215a94a4481b4a8e972c5f23179e0461b.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
        let config = LexerConfig::for_profile(Some(profile));
        for (suffix, braced) in [
            ("::tail", true),
            (":tail", false),
            ("x", true),
            ("(k)", true),
            ("é", dialect == "jim"),
        ] {
            let source = format!("ALIAS{suffix}");
            let fragment = StringLiteralFragment {
                span: Span::new(0, u32::try_from(source.len()).unwrap()),
                text: source.clone(),
                config,
                following_end: source.len(),
            };
            let expected = if braced { "${a}" } else { "$a" };
            assert_eq!(
                alias_string_reference(&source, &fragment, 0, 5, "a", "ALIAS").as_deref(),
                Some(expected),
                "{dialect}: {suffix}",
            );
        }
    }
}

#[test]
fn string_alias_reference_rejects_escaped_insertion_and_split_unicode() {
    let config = LexerConfig::for_profile(Some(tcl_dialect::DialectProfile::plain_tcl()));
    for (source, start, end, value) in [(r"\ALIAS!", 1, 6, "ALIAS"), ("éALIAS!", 1, 7, "ALIAS")] {
        let fragment = StringLiteralFragment {
            span: Span::new(0, u32::try_from(source.len()).unwrap()),
            text: source.to_owned(),
            config,
            following_end: source.len(),
        };
        assert!(alias_string_reference(source, &fragment, start, end, "a", value).is_none());
    }
}

#[test]
fn string_alias_inventory_keeps_data_braces_inert_and_real_body_geometry() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let registry = CommandRegistry::build_default();
    let source =
        "set data {puts \"inert repeated text\"}\nproc p {} {puts \"live repeated text\"}\n";
    let analysis = original::analysis_for_profile(source, profile, &registry);
    let fragments = collect_string_literals(source, analysis.resolved_input.as_ref().unwrap());
    assert!(!fragments.is_empty());
    assert!(
        fragments
            .iter()
            .all(|fragment| !fragment.text.contains("inert"))
    );
    assert!(
        fragments
            .iter()
            .any(|fragment| fragment.text == "live repeated text")
    );
    for fragment in fragments {
        assert_eq!(
            source.get(fragment.span.as_range()),
            Some(fragment.text.as_str())
        );
    }
}

#[test]
fn aggressive_aliases_avoid_live_variable_names() {
    // The alias generator must not claim a name the
    // script reads through a name-taking command (`[set a]` has no
    // `$a` spelling anywhere).  tclsh 9.0.4: with `a` pre-set to
    // SENTINEL, an alias preamble `set a mylongcmd` changes what
    // `[set a]` returns.
    let out = agg("mylongcmd x\nmylongcmd y\nmylongcmd z\nputs [set a]\n");
    assert!(
        !out.contains("set a mylongcmd"),
        "alias clobbered live variable `a`: {out}"
    );
    assert!(out.contains("puts [set a]"), "{out}");
}

#[test]
fn aggressive_runs_optimise_compact_minify() {
    let registry = CommandRegistry::build_default();
    let src = "proc greet {name} {\n    set message \"hi $name\"\n    return $message\n}\n";
    let res = minify_tcl_aggressive(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        false,
        &registry,
    );
    // With no applicable optimisations this equals the compact tier
    // (proc name preserved — public identity).
    // Naming passes are refused until a shared transactional rename
    // planner exists, so every name stays as written.
    assert_eq!(
        res.source,
        "proc greet {name} {set message \"hi $name\";return $message}"
    );
    assert_eq!(res.original_length, src.len());
    assert_eq!(res.minified_length(), res.source.len());
    assert!(res.savings_pct() > 0.0);
}

#[test]
fn compact_two_namespace_same_proc_name_is_deterministic_and_intact() {
    // Two namespaces define a `dup` proc.  `::a::dup` has a local
    // (`collidevar`) whose name equals `::b::dup`'s *parameter*.  Keying the
    // proc by simple name matched whichever `dup` came first in `HashMap`
    // order, so `rename_params_in_list` was handed the wrong declaration's
    // parameter region — renaming one proc's `$use` sites while leaving its
    // definition (or the colliding local) under the other proc's name.  The
    // corruption surfaced non-deterministically, so drive it many times and
    // require every run to produce the same intact output.
    let registry = CommandRegistry::build_default();
    let src = "namespace eval a {\n    proc dup {arg} { set collidevar [expr {$arg + 1}]; return $collidevar }\n}\nnamespace eval b {\n    proc dup {collidevar} { return $collidevar }\n}\n";
    // Naming passes are refused until a shared transactional rename
    // planner exists, so every name and keyword stays as written.
    let expected = "namespace eval a {proc dup {arg} {set collidevar [expr {$arg+1}];return $collidevar}};namespace eval b {proc dup {collidevar} {return $collidevar}}";
    for _ in 0..32 {
        let res = minify_tcl_aggressive(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            false,
            &registry,
        );
        assert_eq!(res.source, expected, "collision corrupted the output");
    }
}

#[test]
fn compact_never_renames_array_members() {
    // Array member names are Tcl DATA, not private symbols: `array
    // get` / `array names` / traces / serialisation observe them, so
    // no tier may rename them.  tclsh 9.0.4: the original prints
    // `longmember 1`, so a compaction that renames the key prints `a 1`
    // instead.
    let out = min_compact(
        "proc f {} {\n    set arr(longmember) 1\n    return [array get arr]\n}\nputs [f]\n",
    );
    assert!(out.contains("arr(longmember)"), "{out}");
    // And even in isolated mode — observability does not depend on
    // the closed-world assertion.
    let out = min_compact_isolated(
        "proc f {} {\n    set arr(longmember) 1\n    return [array get arr]\n}\nputs [f]\n",
    );
    assert!(out.contains("arr(longmember)"), "{out}");
}

#[test]
fn compact_symbol_map_has_no_array_member_section() {
    let registry = CommandRegistry::build_default();
    let (_, sym) = minify_tcl_compact(
        "proc f {} {\n    set config(database) 1\n    puts $config(database)\n}\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        false,
        &registry,
    );
    assert!(!sym.format().contains("Array members"), "{}", sym.format());
}

#[test]
fn compact_non_isolated_keeps_global_vars() {
    assert_eq!(
        min_compact("set globalvar 1\nputs $globalvar\n"),
        "set globalvar 1;puts $globalvar"
    );
}

#[test]
fn default_tier_never_introduces_variables() {
    // Template deduplication would emit a `set a {…}` preamble +
    // `[subst $a]` — a real variable write that clobbers any live `a`
    // (tclsh 9.0.4: `puts [set a]` then stops printing the pre-existing
    // value), fires traces, and changes `info vars`.  The default tier
    // must stay frame-transparent: no `set`, no `subst`, strings
    // verbatim.
    check(
        "puts \"value is $longvariablename here\"\nputs \"value is $longvariablename here\"\nputs [set a]\n",
        "puts \"value is $longvariablename here\";puts \"value is $longvariablename here\";puts [set a]",
    );
}

#[test]
fn abbreviates_ensemble_subcommand_in_irules() {
    assert_eq!(
        min_dialect("string length $x\n", tcl_dialect::DialectProfile::irules()),
        "string le $x"
    );
    // `info e` would also prefix `info errorstack` (Tcl 8.6+) — reading
    // the abbreviation from `info`'s own registry table (rather than a
    // hand-kept copy that only knew of `exists`) surfaces that and picks
    // the shortest spelling actually safe against the whole table:
    // `ex`, not the old table's `e`.
    assert_eq!(
        min_dialect("info exists $x\n", tcl_dialect::DialectProfile::irules()),
        "info ex $x"
    );
}

#[test]
fn no_subcommand_abbreviation_in_plain_tcl() {
    assert_eq!(
        min_dialect(
            "string length $x\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
        ),
        "string length $x"
    );
}

#[test]
fn expr_comparison_inversion() {
    check("if {!($a == $b)} {puts x}\n", "if {$a!=$b} {puts x}");
    check("if {!($a != $b)} {puts x}\n", "if {$a==$b} {puts x}");
}

/// `!($a < $b)` is not `$a >= $b` when an operand may be NaN
/// (`expr {!(NaN < 1)}` is 1, `expr {NaN >= 1}` is 0). The minifier rewrites
/// source text with no type information, so it can never prove an operand
/// NaN-free and must leave the ordered four alone — the parenthesised form
/// survives verbatim even though inverting would be shorter.
#[test]
fn expr_ordered_comparison_inversion_is_declined() {
    check("if {!($a < $b)} {puts x}\n", "if {!($a<$b)} {puts x}");
    check("if {!($a <= $b)} {puts x}\n", "if {!($a<=$b)} {puts x}");
    check("if {!($a > $b)} {puts x}\n", "if {!($a>$b)} {puts x}");
    check("if {!($a >= $b)} {puts x}\n", "if {!($a>=$b)} {puts x}");
}

/// The string and membership operators never compare numerically, so no NaN
/// rule reaches them and their inversions remain available.
#[test]
fn expr_string_and_membership_inversion_still_applies() {
    // 13:17 whitespace decision; pinned C/Jim wordop controls. The
    // inversion expectations remain independent and unchanged.
    check("if {!($a eq \"x\")} {puts x}\n", "if {$a ne\"x\"} {puts x}");
    check("if {!($a ne \"x\")} {puts x}\n", "if {$a eq\"x\"} {puts x}");
    check("if {!($a in $b)} {puts x}\n", "if {$a ni$b} {puts x}");
    check("if {!($a ni $b)} {puts x}\n", "if {$a in$b} {puts x}");
}

#[test]
fn expr_de_morgan_forward() {
    check("if {!($a && $b)} {puts x}\n", "if {!$a||!$b} {puts x}");
}

#[test]
fn expr_de_morgan_reverse() {
    check("if {!$a || !$b} {puts x}\n", "if {!$a||!$b} {puts x}");
}

#[test]
fn expr_double_negation_not_folded() {
    // `!!x → x` is unsound outside a top-level boolean condition (it drops
    // the 0/1 coercion), and the minifier can't tell the context apart, so
    // `!!$x` is left intact rather than risk changing a value-context result.
    check("if {!!$x} {puts x}\n", "if {!!$x} {puts x}");
}

#[test]
fn expr_no_change_when_already_minimal() {
    check("if {$a < $b} {puts x}\n", "if {$a<$b} {puts x}");
}

#[test]
fn expr_shrink_nested_in_command_subst() {
    check(
        "set y [expr {!($a==1 && $b==2)}]\n",
        "set y [expr {$a!=1||$b!=2}]",
    );
}

#[test]
fn collapses_commands_to_semicolons() {
    check("set x 1\nset y 2\n", "set x 1;set y 2");
}

#[test]
fn collapses_intra_command_whitespace() {
    check("set    x     1\n", "set x 1");
}

#[test]
fn recurses_into_proc_body() {
    check(
        "proc f {} {\n    # c\n    set x 1\n}\n",
        "proc f {} {set x 1}",
    );
}

#[test]
fn recurses_into_command_substitution() {
    check("set y [ expr {1 + 2} ]\n", "set y [expr {1+2}]");
}

#[test]
fn strips_redundant_quotes() {
    check("puts \"hello\"\n", "puts hello");
}

#[test]
fn keeps_quotes_when_needed() {
    check("puts \"hello world\"\n", "puts \"hello world\"");
}

#[test]
fn compresses_expr_whitespace() {
    check("if {$a == 1} {\n    puts hi\n}\n", "if {$a==1} {puts hi}");
}

#[test]
fn keeps_word_operator_spacing() {
    // User decision 2026-10-22T13:17Z: remove every safe gap. Pinned
    // C8.4-9.1/Jim controls (wordop-manual-oracle-20261022.json) prove
    // the space before eq is required here while eq->$b is removable.
    check(
        "if {$a eq $b} {\n    puts hi\n}\n",
        "if {$a eq$b} {puts hi}",
    );
}

/// Shared TIP 461 word-operator spellings retain canonical spacing.
#[test]
fn keeps_tip461_word_operator_spacing() {
    check(
        "if {$a lt $b} {\n    puts hi\n}\n",
        "if {$a lt $b} {puts hi}",
    );
}

#[test]
fn minifies_switch_case_bodies() {
    check(
        "switch $x {\n    a {\n        puts 1\n    }\n    b {\n        puts 2\n    }\n}\n",
        "switch $x {a {puts 1} b {puts 2}}",
    );
}

#[test]
fn switch_fallthrough_preserved() {
    check(
        "switch $x {\n    a -\n    b {\n        puts 2\n    }\n}\n",
        "switch $x {a - b {puts 2}}",
    );
}

#[test]
fn switch_hash_pattern_is_not_a_comment() {
    // A braced case list is a Tcl LIST, so `#` is an ordinary pattern
    // there, never a comment.  tclsh 9.0.4: `switch # { # {puts matched}
    // default {puts default} }` prints `matched`, so a minifier that
    // drops the `#` arm changes the output to `default`.
    check(
        "switch # {\n    # {puts matched}\n    default {puts default}\n}\n",
        "switch # {# {puts matched} default {puts default}}",
    );
}

#[test]
fn switch_hash_prefixed_pattern_and_semicolon_pattern_survive() {
    // `#foo` and `;` are ordinary list elements too.
    check(
        "switch $x {\n    #foo {puts a}\n    {;} {puts b}\n    default {puts c}\n}\n",
        "switch $x {#foo {puts a} {;} {puts b} default {puts c}}",
    );
}

#[test]
fn switch_backslash_escaped_pattern_survives_verbatim() {
    // A bare element with backslash escapes (`a\ b` — one pattern
    // word containing a space) must be re-emitted exactly as
    // written, not decoded or re-quoted.
    check(
        "switch $x {\n    a\\ b {puts one}\n    default {puts two}\n}\n",
        "switch $x {a\\ b {puts one} default {puts two}}",
    );
}

#[test]
fn switch_odd_length_case_list_preserved_verbatim() {
    // An odd-length list is a runtime error ("extra switch pattern
    // with no body") whose message quotes the original text — the
    // minifier must not restructure it.
    check(
        "switch $x {\n    a {puts 1}\n    b\n}\n",
        "switch $x {\n    a {puts 1}\n    b\n}",
    );
}

#[test]
fn switch_dynamic_case_list_not_treated_as_case_list() {
    // A non-braced final arg (`$cases`) is not the braced clause-list
    // form; nothing to recurse into.
    check("switch $x $cases\n", "switch $x $cases");
}

#[test]
fn renamed_switch_lookalike_user_command_untouched() {
    // A same-named USER command in a namespace (`::my::switch`) is not
    // the registry `switch`; its braced arg is not a case list.  The
    // registry lookup is by resolved head, so the qualified spelling
    // does not match and the argument body is left as an opaque word.
    let out = min("my::switch x {\n    # {puts matched}\n}\n");
    assert!(out.contains("# {puts matched}"), "{out}");
}

#[test]
fn switch_braced_and_quoted_multiword_patterns_keep_delimiters() {
    // A braced `{a b}` / quoted `"c d"` pattern must keep its
    // delimiters so the case list stays balanced and re-parses as one word
    // per pattern.  Dropping the quotes turned `"c d"` into two bare words.
    check(
        "switch $x {\n  {a b} { puts one }\n  \"c d\" { puts two }\n  default { puts def }\n}\n",
        "switch $x {{a b} {puts one} \"c d\" {puts two} default {puts def}}",
    );
}

#[test]
fn nested_body_recursion() {
    check(
        "proc f {} {\n    if {$x} {\n        set y 1\n    }\n}\n",
        "proc f {} {if {$x} {set y 1}}",
    );
}

#[test]
fn empty_source_minifies_to_empty() {
    check("\n\n# only a comment\n", "");
}

/// `apply`'s lambda-literal argument (`ArgRole::LambdaLiteral`,
/// not `Body`) must have its real body element minified — not the whole
/// `{argList} {body}` blob re-segmented as one script (which misreads
/// the parameter word as a command name and never reaches the real
/// body, leaving comments / extra whitespace inside it untouched). The
/// parameter-list element itself stays verbatim, matching how a plain
/// `proc` parameter list is never specially minified either.
#[test]
fn apply_lambda_body_minifies() {
    check(
        "apply {dir {\n    # a comment\n    puts    $dir\n}} /tmp\n",
        "apply {dir {puts $dir}} /tmp",
    );
}

/// A bare body element's backslash
/// escape must be decoded before minification, not pasted through raw —
/// `puts\ hi`'s real runtime body is the two-word command `puts hi`, and
/// the minified output must preserve that (not keep the backslash, which
/// would change what `apply` actually runs).
#[test]
fn apply_lambda_body_with_backslash_escape_decodes() {
    check(r"apply {{} puts\ hi}", "apply {{} {puts hi}}");
}

/// A multi-word parameter list is itself a *nested* list element
/// (`{x y}`) — reassembling it without its own braces would collapse it
/// into separate top-level elements and corrupt the lambda's arity.
#[test]
fn apply_lambda_multi_word_param_list_keeps_its_braces() {
    check(
        "apply {{x y} {\n    return [expr {$x + $y}]\n}} 1 2",
        "apply {{x y} {return [expr {$x+$y}]}} 1 2",
    );
}

// Keyword abbreviations.
//
// tclsh ground truth (8.6.16): `string le abc` → `3`, `string eq a a` → `1`,
// `lsearch -noc {A b} a` → `0`; `string l abc` → `unknown or ambiguous
// subcommand "l"`, so an ambiguous prefix must never be emitted.
mod abbreviations {
    use super::*;

    fn aggressive(src: &str) -> String {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        minify_tcl_aggressive(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            false,
            registry,
        )
        .source
    }

    fn aggressive_no_abbrev(src: &str) -> String {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        minify_tcl_aggressive_with(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            false,
            registry,
            false,
        )
        .source
    }

    #[test]
    fn subcommand_words_shorten_to_their_minimal_unique_prefix() {
        let out = aggressive("puts [string length $x]\n");
        assert!(out.contains("string le "), "{out}");
        assert!(!out.contains("string length"), "{out}");
    }

    // tclsh-proof (8.6.16): `string equal -n ABC abc` -> 1 -- `-n` is the
    // minimal unique prefix in `string equal`'s two-option table
    // (`-nocase`, `-length`).
    #[test]
    fn option_words_shorten_too() {
        let out = aggressive("puts [string equal -nocase $a $b]\n");
        assert!(out.contains("-n "), "{out}");
        assert!(!out.contains("-nocase"), "{out}");
    }

    #[test]
    fn the_emitter_can_be_turned_off() {
        let out = aggressive_no_abbrev("puts [string length $x]\n");
        assert!(out.contains("string length"), "{out}");
    }

    #[test]
    fn a_keyword_with_no_shorter_unique_spelling_is_left_alone() {
        // `trim` is a proper prefix of `trimleft`/`trimright`, so it can only
        // be written in full.
        let out = aggressive("puts [string trim $x]\n");
        assert!(out.contains("string trim"), "{out}");
    }

    #[test]
    fn dynamic_and_expanded_words_are_never_abbreviated() {
        for src in ["puts [string $sub $x]\n", "puts [string {*}$words $x]\n"] {
            let out = aggressive(src);
            assert!(!out.contains("string le"), "{src} -> {out}");
        }
    }

    #[test]
    fn command_names_are_never_abbreviated() {
        // Tcl does not prefix-match command names.
        let out = aggressive("puts hello\n");
        assert!(out.contains("puts"), "{out}");
        assert!(!out.contains("put "), "{out}");
    }

    #[test]
    fn a_boolean_value_definition_keeps_its_bytes() {
        // `set flag true` is a value-definition site: `$flag` may later meet
        // `eq "true"`, so `t` and `true` are not interchangeable.
        let out = aggressive("set flag true\nputs $flag\n");
        assert!(out.contains("true"), "{out}");
    }

    #[test]
    fn a_prefix_a_later_release_makes_ambiguous_is_not_emitted_for_an_older_target() {
        // `string compare` is `co` in 8.5 but `string cat` (8.6.2) collides
        // at `c`; the emitter checks every later core-Tcl table, so whatever
        // it emits must still resolve to `compare` under 9.0.
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.5").commands();
        let out = minify_tcl_aggressive(
            "puts [string compare $a $b]\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
            false,
            registry,
        )
        .source;
        let emitted = out.split_whitespace().find(|w| "compare".starts_with(*w));
        if let Some(word) = emitted {
            let t90 = tcl_registry::model::ingress::static_context_for("tcl9.0")
                .commands()
                .get("string")
                .expect("string in 9.0")
                .subcommand_table(None, None, None);
            assert_eq!(
                t90.resolve(word).unique(),
                Some("compare"),
                "emitted {word:?} is not `compare` under Tcl 9.0: {out}"
            );
        }
    }
}

/// Minification must resolve a command head's *effective
/// identity*, not its written spelling.
///
/// A body-role argument is re-minified as the script it is (comments
/// stripped, whitespace collapsed); an unrecognised command's braced word
/// is emitted verbatim.  That difference is the witness.
///
/// tclsh oracle (8.6.16 and 9.0.4, byte-identical): `interp alias {} maybe
/// {} if` makes `maybe` run `if`; `rename if maybe` moves it and leaves
/// `if` gone; a top-level `proc if …` takes the name over.
fn body_was_minified(src: &str) -> bool {
    !min(src).contains("# c")
}

const BODY_CALL: &str = " {$x} {\n    # c\n    puts a\n}\n";

#[test]
fn minify_follows_an_aliased_body_command() {
    assert!(body_was_minified(&format!(
        "interp alias {{}} maybe {{}} if\nmaybe{BODY_CALL}"
    )));
    // The `::`-qualified spelling of the alias classifies alike.
    assert!(body_was_minified(&format!(
        "interp alias {{}} maybe {{}} if\n::maybe{BODY_CALL}"
    )));
    // Guard: an unbound `maybe` has no body argument to descend into.
    assert!(!body_was_minified(&format!("set y 1\nmaybe{BODY_CALL}")));
}

#[test]
fn minify_follows_a_renamed_body_command() {
    assert!(body_was_minified(&format!(
        "rename if maybe\nmaybe{BODY_CALL}"
    )));
    assert!(
        !body_was_minified(&format!("rename if maybe\nif{BODY_CALL}")),
        "a renamed-away `if` must not keep the built-in's body grammar"
    );
}

#[test]
fn minify_abstains_for_a_builtin_shadowed_by_a_user_proc() {
    assert!(
        !body_was_minified(&format!("proc if {{c b}} {{return 1}}\nif{BODY_CALL}")),
        "a user `proc if` takes the name over; its braced word is opaque data"
    );
    // Guard: the unshadowed built-in still descends.
    assert!(body_was_minified(&format!("set y 1\nif{BODY_CALL}")));
}

#[test]
fn minify_abstains_for_a_dynamic_binding() {
    assert!(
        !body_was_minified(&format!("rename $old maybe\nmaybe{BODY_CALL}")),
        "a dynamic rename must not give `maybe` a body grammar"
    );
    assert!(
        body_was_minified(&format!("rename $old maybe\nif{BODY_CALL}")),
        "a dynamic rename must not take `if`'s body grammar away either"
    );
}

/// The aggressive tier's keyword-abbreviation phase reads the resolved
/// head too: the subcommand table it shortens against belongs to the
/// command the head *is*.
#[test]
fn keyword_abbreviation_follows_an_aliased_head() {
    let registry = CommandRegistry::build_default();
    let aliased = minify_tcl_aggressive(
        "interp alias {} str {} string\nstr toupper $::env(HOME)\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        true,
        &registry,
    );
    assert!(
        aliased.source.contains("str tou "),
        "an alias of `string` must abbreviate against `string`'s subcommand \
             table; got {:?}",
        aliased.source
    );
    // Guard: an unbound `str` has no subcommand table, so `toupper` is
    // ordinary data and must survive untouched.
    let unbound = minify_tcl_aggressive(
        "set y 1\nstr toupper $::env(HOME)\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        true,
        &registry,
    );
    assert!(
        unbound.source.contains("str toupper"),
        "{:?}",
        unbound.source
    );
}

/// The aggressive tier's rename-*barrier* scan reads the resolved head:
/// `upvar`'s "aliases the caller's frame" trait bars local-name compaction
/// in every scope, and that trait belongs to the command a head *is*.
///
/// The witness is whether `set local 1` survives: barred, the local keeps
/// its name; unbarred, it compacts to a single letter.
#[test]
fn rename_barriers_retain_distinct_actual_scopes_with_identical_labels() {
    let source = "proc repeated {} {set firstlong 1; puts $firstlong}\nproc repeated {} {set secondlong 2; puts $secondlong}\n";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let scopes = analysis
        .global_scope
        .children
        .iter()
        .filter(|scope| scope.kind == ScopeKind::Proc)
        .collect::<Vec<_>>();
    assert_eq!(scopes.len(), 2);
    assert_eq!(scopes[0].name, scopes[1].name);
    assert!(!std::ptr::eq(scopes[0], scopes[1]));
    let first = source_scope_at_offset(
        &analysis.global_scope,
        scopes[0].body_span.unwrap().start(),
        "::",
        false,
    )
    .unwrap();
    let second = source_scope_at_offset(
        &analysis.global_scope,
        scopes[1].body_span.unwrap().start(),
        "::",
        false,
    )
    .unwrap();
    assert_eq!(first.1, second.1);
    assert!(std::ptr::eq(first.0, scopes[0]));
    assert!(std::ptr::eq(second.0, scopes[1]));
    let mut barriers = RenameBarriers::default();
    apply_rename_traits(
        &mut barriers,
        Traits::CREATES_DYNAMIC_BARRIER,
        Some(first.0),
    );
    apply_rename_traits(
        &mut barriers,
        Traits::CREATES_DYNAMIC_BARRIER,
        Some(first.0),
    );
    assert_eq!(barriers.scopes.len(), 1);
    assert!(!barriers.allows_scope(first.0));
    assert!(barriers.allows_scope(second.0));
    barriers.global_variables = true;
    assert!(!barriers.allows_scope(&analysis.global_scope));
    assert!(barriers.allows_scope(second.0));
}

#[test]
fn report_scope_collisions_preserve_every_selected_mapping() {
    let mut map = SymbolMap::default();
    let first = BTreeMap::from([("longfirst".to_owned(), "a".to_owned())]);
    let second = BTreeMap::from([("longsecond".to_owned(), "b".to_owned())]);
    record_variable_report(&mut map, "::same", first.clone());
    record_variable_report(&mut map, "::same [source 2]", second.clone());
    record_variable_report(&mut map, "::same", second.clone());
    assert_eq!(map.variables.len(), 3);
    assert_eq!(map.variables["::same"], first);
    assert_eq!(map.variables["::same [source 2]"], second);
    assert_eq!(map.variables["::same [source 3]"], second);
}

#[test]
fn logical_formal_edit_deduplication_uses_actual_scope_not_report_keys() {
    let source = "proc first {longname} {return $longname}";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        LexerConfig::for_file_grammar(profile.grammar),
    );
    let analysis = original::analysis_with_input(source, &input);
    let body = analysis
        .global_scope
        .children
        .iter()
        .find(|scope| scope.kind == ScopeKind::Proc)
        .unwrap();
    let (_, label) = source_scope_at_offset(
        &analysis.global_scope,
        body.body_span.unwrap().start(),
        "::",
        false,
    )
    .unwrap();
    let unrelated_report = BTreeMap::from([("unrelated".to_owned(), "x".to_owned())]);
    let mut report = SymbolMap::default();
    report
        .variables
        .insert(label.clone(), unrelated_report.clone());
    let mut edits = Vec::new();
    compact_logical_formals(
        source,
        &input,
        &analysis,
        true,
        &[],
        &mut report,
        &mut edits,
    );
    assert_eq!(edits.len(), 2);
    assert_eq!(report.variables[&label], unrelated_report);
    assert_eq!(report.variables.len(), 2);
    let mut report = SymbolMap::default();
    let mut edits = Vec::new();
    compact_logical_formals(
        source,
        &input,
        &analysis,
        true,
        &[body],
        &mut report,
        &mut edits,
    );
    assert_eq!(edits, [] as [(usize, usize, std::string::String); 0]);
    assert!(report.variables.is_empty());
    compact_logical_formals(
        source,
        &input,
        &analysis,
        false,
        &[],
        &mut report,
        &mut edits,
    );
    assert_eq!(edits, [] as [(usize, usize, std::string::String); 0]);
}
