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

//! `[cmd …]` folds in call words and interpolated strings: `expr` arguments, O115 nested `expr` and O129 pure builtins.

use super::*;

#[test]
fn braced_expr_cmd_sub_arg_folds_under_constants() {
    // `puts [expr {$a + $b}]` with a, b proven constant folds the braced
    // expr in the call-argument position to its value.
    let opts = run_pass("set a 3\nset b 4\nputs [expr {$a + $b}]");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O101 && o.replacement == "7"),
        "expected O101 folding [expr {{$a + $b}}] to 7, got {opts:?}",
    );
}

#[test]
fn shadowed_mathfunc_cmd_sub_arg_not_folded() {
    // FP guard: `proc ::tcl::mathfunc::abs` shadows the builtin
    // everywhere in the module — the cmd-sub-argument O101 fold path
    // (try_o101_expr_arg_fold) must not fold `abs(-5)` using builtin
    // semantics.
    let opts = run_pass("proc ::tcl::mathfunc::abs {x} { return 999 }\nputs [expr {abs(-5)}]");
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O101),
        "must not fold a shadowed math function in a cmd-sub argument: {opts:?}",
    );
}

#[test]
fn quoted_expr_cmd_sub_arg_not_folded_under_constants() {
    // `puts [expr "$a + $b"]` is the quoted form — it uses textual
    // substitution, so we conservatively never fold it.
    let opts = run_pass("set a 3\nset b 4\nputs [expr \"$a + $b\"]");
    assert!(
        opts.iter()
            .all(|o| !(o.code == DiagCode::O101 && o.replacement == "7")),
        "quoted expr must not fold under constants, got {opts:?}",
    );
}

/// Like [`run_pass`] but populates `ctx.command_mutations` from the
/// whole module, the way the real pipeline does — needed to exercise
/// the `expr`-redefinition trust gate.
fn run_pass_with_mutations(source: &str) -> Vec<Optimisation> {
    let reg = registry();
    let cu = CompilationUnit::build_for(source, &reg, false);
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    // Same production wiring `run_pass` above documents: the top-level
    // body's `scan_module_global_names` set is registry-resolved, so a
    // context without a registry is deliberately not propagated at all.
    ctx.registry = Some(&reg);
    ctx.command_mutations =
        crate::command_binding::scan_module_command_mutations(&cu.ir_module, &reg);
    ctx.ir_module = Some(&cu.ir_module);
    run(&mut ctx, &cu);
    ctx.optimisations
}

#[test]
fn renamed_expr_cmd_sub_arg_not_folded() {
    // FP guard: once `expr` is renamed anywhere in the module,
    // `puts [expr {$a + $b}]` no longer calls the builtin evaluator.
    let opts =
        run_pass_with_mutations("rename expr real_expr\nset a 3\nset b 4\nputs [expr {$a + $b}]");
    assert!(
        opts.iter().all(|o| o.code != DiagCode::O101),
        "must not fold a cmd-sub through a renamed expr: {opts:?}",
    );
}

#[test]
fn renamed_expr_assign_expr_substitution_not_folded() {
    // FP guard: `set x [expr {$a + $b}]` (the AssignExpr form,
    // try_substitute_assign_expr) must also respect the trust gate.
    let opts = run_pass_with_mutations("rename expr real_expr\nset a 3\nset x [expr {$a + 1}]");
    assert!(
        opts.iter()
            .all(|o| !(o.code == DiagCode::O100 || o.code == DiagCode::O101)),
        "must not propagate/fold set x [expr {{…}}] through a renamed expr: {opts:?}",
    );
}

#[test]
fn ordinary_expr_cmd_sub_arg_still_folds_under_mutation_scan() {
    // TN/control: an unrelated module-wide mutation scan (no rename of
    // `expr` itself) must not block the fold.
    let opts = run_pass_with_mutations("set a 3\nset b 4\nputs [expr {$a + $b}]");
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O101 && o.replacement == "7"),
        "expected O101 fold under an unrelated mutation scan, got {opts:?}",
    );
}

#[test]
fn o115_collapses_redundant_nested_expr_in_value_positions() {
    // A redundant double-`expr` cmd-sub
    // `[expr {[expr {E}]}]` collapses to `[expr {E}]` in command-arg
    // and return value positions (O115 otherwise fires only on a
    // standalone `expr` statement).
    let collapsed = |src: &str| -> Vec<String> {
        crate::optimiser::optimise_raw(src, &registry(), None)
            .into_iter()
            .filter(|o| o.code == DiagCode::O115)
            .map(|o| o.replacement)
            .collect()
    };
    assert_eq!(
        collapsed("proc ::f {x} { puts [expr {[expr {$x * 2}]}] }"),
        vec!["[expr {$x * 2}]".to_string()],
        "command-arg position should collapse",
    );
    assert_eq!(
        collapsed("proc ::f {x} { return [expr {[expr {$x * 2}]}] }"),
        vec!["return [expr {$x * 2}]".to_string()],
        "return position should collapse",
    );
}

#[test]
fn o115_value_position_is_sound() {
    // A single `[expr {$x + 1}]` must NOT collapse (its unwrap
    // `$x + 1` would be invalid as a bare value), and an
    // `[expr {[other]}]` whose inner is not itself `expr` is not
    // value-equivalent for non-numeric results.
    let has_o115 = |src: &str| -> bool {
        crate::optimiser::optimise_raw(src, &registry(), None)
            .iter()
            .any(|o| o.code == DiagCode::O115)
    };
    assert!(!has_o115("proc ::f {x} { puts [expr {$x + 1}] }"));
    assert!(!has_o115("proc ::f {x} { return [expr {$x + 1}] }"));
    assert!(!has_o115("proc ::f {x} { puts [expr {[someproc]}] }"));
}

fn profiled_registry() -> &'static CommandRegistry {
    tcl_registry::model::ingress::static_context_for("tcl9.0").commands()
}

fn fold_code(src: &str, code: &str) -> Vec<String> {
    crate::optimiser::optimise_raw(src, &registry(), None)
        .into_iter()
        .filter(|o| o.code.as_str() == code)
        .map(|o| o.replacement)
        .collect()
}

/// #2430: `string cat` arrives in 8.6 and `string reverse` in 8.5;
/// tclsh 8.4.20 / 8.5.19 raise `bad option` for the missing one, so the
/// fold declines under those releases and takes the value from 8.6 on.
#[test]
fn o129_declines_a_string_subcommand_the_release_lacks() {
    let folds = |release: &str, src: &str| -> Vec<String> {
        let context = tcl_registry::model::ingress::static_context_for(release);
        let profile = tcl_registry::model::ingress::resolve_environment(release).analyser_profile();
        crate::optimiser::manager::optimise_with_dialect(src, context.commands(), Some(profile))
            .into_iter()
            .filter(|o| o.code == DiagCode::O129 && !o.hint_only)
            .map(|o| o.replacement)
            .collect()
    };
    let cat = "puts [string cat a b c]";
    let reverse = "puts [string reverse abc]";
    assert_eq!(folds("tcl8.4", cat), Vec::<String>::new());
    assert_eq!(folds("tcl8.5", cat), Vec::<String>::new());
    assert_eq!(folds("tcl8.4", reverse), Vec::<String>::new());
    assert_eq!(folds("tcl8.5", reverse), vec!["cba".to_string()]);
    assert_eq!(folds("tcl8.6", cat), vec!["abc".to_string()]);
}

#[test]
fn o129_folds_pure_builtin_cmd_subst() {
    // A pure-builtin cmd-sub with constant
    // (literal) args folds via the registry `const_fold` callback
    // (O129). `optimise_raw` sets `ctx.registry`, which the O129
    // path requires.
    assert_eq!(fold("puts [string toupper foo]"), vec!["FOO".to_string()]);
    assert_eq!(fold("puts [string tolower BAR]"), vec!["bar".to_string()]);
    assert_eq!(fold("puts [string reverse abc]"), vec!["cba".to_string()]);
    // The headline O129 example.
    assert_eq!(fold("puts [string length abcde]"), vec!["5".to_string()]);
    // The cat / repeat / trim folds.
    assert_eq!(
        fold("puts [string cat foo bar]"),
        vec!["foobar".to_string()]
    );
    assert_eq!(
        fold("puts [string repeat ab 3]"),
        vec!["ababab".to_string()]
    );
    assert_eq!(fold("puts [string trim {  hi  }]"), vec!["hi".to_string()]);
    assert_eq!(
        fold("puts [string range abcde 1 3]"),
        vec!["bcd".to_string()]
    );
    assert_eq!(fold("puts [string index abc end]"), vec!["c".to_string()]);
    // List + dict folds.
    assert_eq!(fold("puts [llength {a b c}]"), vec!["3".to_string()]);
    // The cmd-sub replacement is one word, so a spaced result is
    // brace-quoted by `render_propagation_word`.
    assert_eq!(fold("puts [concat a b c]"), vec!["{a b c}".to_string()]);
    assert_eq!(fold("puts [join {a b c} -]"), vec!["a-b-c".to_string()]);
    // `lindex` / `list` report their own codes (O118 / O116) for
    // editor granularity, so filter by those rather than O129.
    assert_eq!(
        fold_code("puts [lindex {a b c} 1]", "O118"),
        vec!["b".to_string()]
    );
    assert_eq!(fold_code("set x [list]", "O116"), vec!["{}".to_string()]);
    // A list 8.6 would read apart as a script, once a literal, stays the
    // list (`if 1 $x` ranges nothing; the braced literal ranges `abc`).
    let hazard = "set x [list string range abc 0 end-2147483649]\nif 1 $x";
    let o116 = |dialect| {
        crate::optimiser::optimise_raw(hazard, &registry(), Some(dialect))
            .into_iter()
            .filter(|o| o.code.as_str() == "O116")
            .count()
    };
    assert_eq!(o116("tcl8.6"), 0);
    assert_eq!(o116("tcl9.0"), 1);
    assert_eq!(fold("puts [dict get {a 1 b 2} b]"), vec!["2".to_string()]);
    assert_eq!(
        fold("puts [lreverse {a b c}]"),
        vec!["{c b a}".to_string()],
        "list result is brace-quoted as one word",
    );
    assert_eq!(fold("puts [string map {a b} aaa]"), vec!["bbb".to_string()]);
    assert_eq!(fold("puts [subst hello]"), vec!["hello".to_string()]);
    // `subst` with a substitution must NOT fold (no upstream resolution).
    assert_eq!(fold("puts [subst {$x}]"), [] as [std::string::String; 0]);
    // `string is` Tcl-faithful classes.
    assert_eq!(fold("puts [string is alpha abc]"), vec!["1".to_string()]);
    assert_eq!(fold("puts [string is lower abc1]"), vec!["0".to_string()]);
    assert_eq!(fold("puts [string is boolean yes]"), vec!["1".to_string()]);
    // The convenience driver retains its documented C9 native target.
    // Actual C8.4 and Jim reject these classes; the selected target matters.
    assert_eq!(fold("puts [string is list {a b c}]"), vec!["1".to_owned()]);
    let legacy = tcl_registry::model::ingress::static_context_for("tcl8.4");
    for source in [
        "puts [string is list {a b c}]",
        "puts [string is wideinteger 42]",
    ] {
        assert!(
            crate::optimiser::optimise_raw_for_profile(
                source,
                legacy.commands(),
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile()
                ),
            )
            .iter()
            .all(|finding| finding.code != DiagCode::O129)
        );
    }
    // `format` %s / %d / %% subset.
    assert_eq!(fold("puts [format %d 42]"), vec!["42".to_string()]);
    assert_eq!(fold("puts [format {v=%s} hi]"), vec!["v=hi".to_string()]);
    // `format` flag / width / precision folds for
    // the decimal-integer + string conversions (dialect-invariant).
    assert_eq!(fold("puts [format %05d 7]"), vec!["00007".to_string()]);
    assert_eq!(fold("puts [format %.3d 5]"), vec!["005".to_string()]);
    // `%#d` stays unfolded (`0d5` on Tcl 9, `5` on 8.6 — divergent).
    assert_eq!(fold("puts [format %#d 5]"), [] as [std::string::String; 0]);
    // `string is integer` / `double` fold over their
    // dialect-invariant subsets.
    assert_eq!(fold("puts [string is integer 42]"), vec!["1".to_string()]);
    assert_eq!(fold("puts [string is double 1.5]"), vec!["1".to_string()]);
    assert_eq!(fold("puts [string is double abc]"), vec!["0".to_string()]);
    assert_eq!(
        fold("puts [string is wideinteger 42]"),
        vec!["1".to_owned()]
    );
    // A braced literal with a space → result rendered as one word.
    assert_eq!(
        fold("puts [string toupper {a b}]"),
        vec!["{A B}".to_string()],
    );
    // A `$var` arg is not a constant literal → no fold.
    assert_eq!(
        fold("puts [string toupper $x]"),
        [] as [std::string::String; 0]
    );
    // The range form (`string toupper s first last`) does not fold.
    assert_eq!(
        fold("puts [string toupper foo 0 0]"),
        [] as [std::string::String; 0]
    );
    // A non-builtin head (a user proc) is not an O129 candidate.
    assert_eq!(
        fold("proc ::p {} { return 1 }\nputs [::p]"),
        [] as [std::string::String; 0]
    );
}

/// The folded command substitution carries its own quoted argument, so
/// the rewrite span has to cross that inner `"b"`. A scanner that stops
/// at the first unescaped `"` produces a span covering only
/// `"a[string toupper "`, and applying the fix leaves the
/// tail `b"]c"` behind as a stray fragment. Applying the rewrite must
/// yield well-formed source.
#[test]
fn o129_interpolation_fold_replaces_the_whole_quoted_word() {
    let src = "puts \"a[string toupper \"b\"]c\"\n";
    let opts = crate::optimiser::optimise_raw(src, &registry(), None);
    let fold = opts
        .iter()
        .find(|o| o.code == DiagCode::O129 && !o.hint_only)
        .unwrap_or_else(|| panic!("expected an applicable O129, got {opts:?}"));
    let mut rewritten = src.to_owned();
    rewritten.replace_range(
        fold.span.start() as usize..fold.span.end() as usize,
        &fold.replacement,
    );
    assert_eq!(rewritten, "puts \"aBc\"\n");
}

#[test]
fn retained_const_subst_accepts_only_closed_literal_templates() {
    let literal = "puts [string toupper \"a\\tb\"]";
    let findings = crate::optimiser::optimise_raw(literal, &registry(), None);
    let fold = findings
        .iter()
        .find(|finding| finding.code == DiagCode::O129 && !finding.hint_only)
        .expect("the native word owner resolves a quoted literal");
    assert_eq!(
        tcl_syntax::list::split_list(&fold.replacement).unwrap(),
        vec!["A\tB"]
    );
    for source in [
        "proc p {value} {puts [string toupper \"$value\"]}",
        "puts [string toupper \"[operation]\"]",
    ] {
        assert!(
            crate::optimiser::optimise_raw(source, &registry(), None)
                .iter()
                .all(|finding| finding.code != DiagCode::O129 || finding.hint_only),
            "{source}"
        );
    }
}

/// A `"…"` word holding a command-position comment reaches across a
/// backslash-newline continuation. `tclsh` prints `x3abc`; a rewrite that
/// mis-spans the word prints `x3ab""b"c`.
///
/// Both the canonical lexer and `close_quote_offset` keep command
/// position across the continuation and reach the final `"`, so the safe
/// rewrite folds only the first command substitution and preserves the
/// commented-out closer verbatim.
#[test]
fn o129_fold_preserves_comment_hidden_closer() {
    let src = "puts \"x[string length abc]a[\n\\\n# ] comment\nset y \"b\"\n]c\"\n";
    let opts = crate::optimiser::optimise_raw(src, &registry(), None);
    let rewrites = opts
        .iter()
        .filter(|o| o.code == DiagCode::O129 && !o.hint_only)
        .collect::<Vec<_>>();
    assert_eq!(rewrites.len(), 1, "expected one safe O129: {opts:?}");
    let opt = rewrites[0];
    let mut rewritten = src.to_owned();
    rewritten.replace_range(
        opt.span.start() as usize..opt.span.end() as usize,
        &opt.replacement,
    );
    assert_eq!(
        rewritten,
        "puts \"x3a[\n\\\n# ] comment\nset y \"b\"\n]c\"\n"
    );
}

#[test]
fn o129_concat_preserves_native_trailing_backslash_space_bytes() {
    // Profiled registry: a dialect-less standalone unit stays fail-closed.
    // All six native engines re-expose exactly one trailing byte after a
    // backslash. The shared concat owner now models that operation.
    for (source, expected) in [
        ("puts [concat a b c]", "a b c"),
        ("puts [concat a {b\\ } c]", "a b\\  c"),
        ("puts [concat {b\\ }]", "b\\ "),
        ("puts [concat a {b\\\\   } c]", "a b\\\\  c"),
    ] {
        let replacements = crate::optimiser::optimise_raw(source, profiled_registry(), None)
            .into_iter()
            .filter(|finding| finding.code == DiagCode::O129 && !finding.hint_only)
            .map(|finding| finding.replacement)
            .collect::<Vec<_>>();
        assert_eq!(replacements.len(), 1, "{source}");
        assert_eq!(
            tcl_syntax::list::split_list(&replacements[0]).unwrap(),
            vec![expected],
            "{source}"
        );
    }
}

#[test]
fn o129_resolves_constant_var_args_b2() {
    // Profiled registry: a dialect-less standalone unit stays fail-closed.
    // A constant `$var` arg in a builtin
    // cmd-sub is resolved (whole-function-constant → sound) before
    // folding; a multi-word value stays a single argument.
    let fold = |src: &str| -> Vec<String> {
        crate::optimiser::optimise_raw(src, profiled_registry(), None)
            .into_iter()
            .filter(|o| o.code == DiagCode::O129)
            .map(|o| o.replacement)
            .collect()
    };
    assert_eq!(
        fold("set s abcde\nputs [string length $s]"),
        vec!["5".to_string()],
    );
    // A multi-word constant value is kept as one arg.
    assert_eq!(
        fold("set s {a b}\nputs [llength $s]"),
        vec!["2".to_string()],
    );
    // Combined: a resolved var inside an interpolation cmd-sub.
    assert_eq!(
        fold("set s abc\nputs \"len=[string length $s]\""),
        vec!["\"len=3\"".to_string()],
    );
    // A non-constant var does not resolve → no fold.
    assert_eq!(
        fold("puts [string length $undefined]"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn o129_folds_embedded_cmd_subst_in_interpolation() {
    // A pure-builtin cmd-sub embedded inside
    // an interpolation string folds, splicing the raw result.
    let fold = |src: &str| -> Vec<String> {
        crate::optimiser::optimise_raw(src, &registry(), None)
            .into_iter()
            .filter(|o| o.code == DiagCode::O129)
            .map(|o| o.replacement)
            .collect()
    };
    assert_eq!(
        fold("puts \"v=[string length abc]\""),
        vec!["\"v=3\"".to_string()],
    );
    assert_eq!(
        fold("puts \"n=[llength {a b c}] items\""),
        vec!["\"n=3 items\"".to_string()],
    );
    // A non-foldable embedded sub leaves the string unfolded.
    assert_eq!(
        fold("puts \"x=[someproc]\""),
        [] as [std::string::String; 0]
    );
    // Soundness guard: a result that would re-introduce a `$`
    // substitution into the `\"…\"` is not spliced.
    assert_eq!(
        fold("puts \"v=[string cat {$} x]\""),
        [] as [std::string::String; 0]
    );
}

#[test]
fn o129_uses_the_actual_binding_before_and_after_a_rebinder() {
    // Profiled registry: a dialect-less standalone unit stays fail-closed.
    let fold = |src: &str| -> Vec<String> {
        crate::optimiser::optimise_raw(src, profiled_registry(), None)
            .into_iter()
            .filter(|o| o.code == DiagCode::O129)
            .map(|o| o.replacement)
            .collect()
    };
    // Baseline: untouched `string` folds.
    assert_eq!(fold("puts [string toupper foo]"), vec!["FOO".to_string()]);
    // An uncalled declaration leaves the stock command selected.
    assert_eq!(
        fold("proc clobber {} { rename string {} }\nputs [string toupper foo]"),
        vec!["FOO".to_string()],
    );
    assert_eq!(
        fold("proc clobber {} { rename string {} }\nclobber\nputs [string toupper foo]"),
        [] as [String; 0],
        "a completed rebinder removes the reached builtin",
    );
    assert_eq!(
        fold("rename string {}\nputs [string toupper foo]").len(),
        0,
        "renamed-away builtin must not fold",
    );
}
