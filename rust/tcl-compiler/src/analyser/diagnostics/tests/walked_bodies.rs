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

//! Which script bodies the checks reach: lambda literals in command substitutions, `catch`, iRules nesting scripts, `when` and `tcltest` bodies.

use super::*;

/// A literal `apply {{params} {body}}` inside a `[…]` command substitution
/// must not report `Unknown command '<the parameter list>'`: re-segmenting
/// the whole `{params body}` list as if it were script source makes the
/// parameter-list word a command head.
///
/// The handling is registry-role-driven, not `apply`-aware: `descend_command`
/// resolves `ArgRole::LambdaLiteral` arguments through
/// `lambda_literal::split_lambda_literal` and descends only the body
/// element, so the parameter list is never walked as code and the body is.
///
/// Source is the `validateHelper` lambda body verbatim from
/// `georgtree/argparse`'s `argparse.tcl:19-34`, with the outer
/// `{*}$validateHelper` variable indirection removed so `apply` is called
/// literally. Oracle: tclsh9.0.4 and tclsh8.6.14 both run it and print
/// `green`, so every command in it is real.
#[test]
fn apply_lambda_in_command_substitution_does_not_report_its_parameter_list() {
    let src = r#"set result [apply {{name opt args} {
    if {[dict exists $opt enum]} {
        set command [list tcl::prefix match -message "$name value" \
                             {*}[if {[uplevel 1 {info exists exact}]} {list -exact}] [dict get $opt enum]]
        set args [lmap arg $args {{*}$command $arg}]
    }
    return $args
}} widget [dict create enum {red green blue}] gr]
puts $result
"#;
    let mut a = crate::analyser::Analyser::new();
    let unknown: Vec<String> = a
        .analyse(src, "tcl9.0")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W123)
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(
        unknown.len(),
        0,
        "the lambda's parameter list is not a command: {unknown:?}"
    );
}

/// FN half: descending the lambda's *body* element is what makes the
/// commands inside it visible at all.  A lambda body nested in a `[…]`
/// substitution that nothing walks lets a genuinely unknown command in it
/// escape W123 entirely.
#[test]
fn apply_lambda_body_in_command_substitution_is_walked() {
    for src in [
        "set r [apply {{a} {zz9unknowncmd}} 5]\nputs $r\n",
        "puts [apply {{a} {zz9unknowncmd}} 5]\n",
        "proc p {} { set r [apply {{a} {zz9unknowncmd}} 5]; return $r }\n",
        "set r [catch {apply {{a} {zz9unknowncmd}} 5}]\nputs $r\n",
        // Three-element lambda: element 2 is a namespace, not a script. It
        // must exist, or tclsh 8.6/9.0 fail `namespace "::ns" not found`
        // before the body runs.
        "namespace eval ::ns {}\nset r [apply {{a} {zz9unknowncmd} ::ns} 5]\nputs $r\n",
    ] {
        let codes = codes_for_dialect(src, "tcl9.0");
        assert!(
            codes.contains(&"W123".to_string()),
            "the lambda body's unknown command must be reported for {src:?}: {codes:?}"
        );
    }
    // TP control — the same command at the top level was already reported.
    assert!(
        codes_for_dialect("apply {{a} {zz9unknowncmd}} 5\n", "tcl9.0")
            .contains(&"W123".to_string())
    );
}

/// TN for the lambda carve-out: an ordinary registry `ArgRole::Body`
/// argument nested in the same substitution position is still a script and
/// is still walked — the carve-out narrows nothing but the lambda shape.
#[test]
fn plain_body_argument_in_command_substitution_still_walked() {
    for src in [
        "set r [if {1} {zz9unknowncmd}]\nputs $r\n",
        "set r [eval {zz9unknowncmd}]\nputs $r\n",
        // `uplevel 1` needs a caller: at the top level tclsh 8.6/9.0 fail
        // `bad level "1"` before the body runs.
        "proc p {} { set r [uplevel 1 {zz9unknowncmd}] }\np\n",
    ] {
        let codes = codes_for_dialect(src, "tcl9.0");
        assert!(
            codes.contains(&"W123".to_string()),
            "a plain body argument is still a script for {src:?}: {codes:?}"
        );
    }
}

/// FP guard — a lambda whose parameters are *named* like commands must draw
/// no W123: the parameter list is never a script, whatever the words in it
/// happen to spell.  Oracle (tclsh9.0.4, tclsh8.6.14): `apply {{set list}
/// {…}} a b` binds locals named `set` and `list` and runs fine — the words
/// are formal-parameter names, not calls.
#[test]
fn apply_lambda_parameters_named_like_commands_draw_no_unknown_command() {
    for src in [
        "set r [apply {{set list} {return \"$set$list\"}} a b]\nputs $r\n",
        // A defaulted parameter whose default value is a bareword.
        "set r [apply {{a {puts x}} {return $a}} 1]\nputs $r\n",
        // A parameter list whose words are not commands at all — a shape that
        // must not be reported as `Unknown command 'name opt args'`.
        "set r [apply {{name opt args} {return $name}} a b c]\nputs $r\n",
    ] {
        let codes = codes_for_dialect(src, "tcl9.0");
        assert!(
            !codes.contains(&"W123".to_string()),
            "parameter names are not command calls for {src:?}: {codes:?}"
        );
    }
}

/// Scope isolation for the substitution-position lambda.
///
/// `apply`'s lambda body runs in a **fresh call frame**, so a variable it sets
/// is a local of the lambda, never of the enclosing proc. Walking the body as
/// an ordinary registry `Body` argument would bind it in whatever scope the
/// `[…]` substitution sits in; routing the call through `apply`'s own analyser
/// hook is what keeps the frame separate.
///
/// Oracle (tclsh9.0.4) for
/// `proc p {} { set r [apply {{} {gets stdin leaked}}]; puts $leaked }`:
/// `can't read "leaked": no such variable` from `puts $leaked` — the lambda's
/// `leaked` is invisible in `p`.
#[test]
fn apply_lambda_in_command_substitution_keeps_its_own_frame() {
    let scoped_vars = |src: &str| -> Vec<String> {
        let mut a = crate::analyser::Analyser::new();
        let mut keys: Vec<String> = a.analyse(src, "tcl9.0").all_variables.into_keys().collect();
        keys.sort();
        keys
    };
    // TP — `leaked` belongs to the lambda, not to `p`.
    let vars = scoped_vars(
        "proc p {} {\n    set r [apply {{} {gets stdin leaked}}]\n    puts $leaked\n}\n",
    );
    assert!(
        !vars.iter().any(|v| v == "p::leaked"),
        "the lambda's local must not leak into the enclosing proc: {vars:?}"
    );
    assert!(
        vars.iter()
            .any(|v| v.starts_with("apply@") && v.ends_with("::leaked")),
        "the lambda's local belongs to the lambda's own scope: {vars:?}"
    );
    // …and the read after the `try`-less lambda still warns, matching the
    // oracle's `can't read "leaked"`.
    assert!(
        codes_for_dialect(
            "proc p {} {\n    set r [apply {{} {gets stdin leaked}}]\n    puts $leaked\n}\n",
            "tcl9.0",
        )
        .contains(&"W210".to_string()),
        "an undefined read after the lambda must still warn"
    );
    // TP — a plain `set` inside the lambda body, same isolation.
    let vars = scoped_vars("proc p {} {\n    set r [apply {{} {set inner 1}}]\n    return $r\n}\n");
    assert!(
        !vars.iter().any(|v| v == "p::inner"),
        "a lambda-body `set` must not bind in the caller: {vars:?}"
    );
    // TP — the lambda's *parameters* are bound too, in the lambda's scope.
    let vars = scoped_vars("set r [apply {{name opt args} {return $name}} a b c]\n");
    for want in ["name", "opt", "args"] {
        assert!(
            vars.iter()
                .any(|v| v.starts_with("apply@") && v.ends_with(&format!("::{want}"))),
            "parameter {want} must bind in the lambda scope: {vars:?}"
        );
    }
    // TN — a genuine enclosing-scope binding from the same statement is
    // unaffected: `set r […]` still binds `r` in `p`.
    let vars = scoped_vars("proc p {} {\n    set r [apply {{} {set inner 1}}]\n    return $r\n}\n");
    assert!(
        vars.iter().any(|v| v == "p::r"),
        "the enclosing `set` still binds in the caller: {vars:?}"
    );
}

/// The lambda's namespace argument decides where its body's bareword calls
/// resolve — element 2, or the **global** namespace when absent, never the
/// caller's (`doc/apply.n`; `TclNRApplyObjCmd` in `generic/tclProc.c`
/// `::`-prefixes the word before the lookup).
///
/// Oracle (tclsh9.0.4 and tclsh8.6.14): with `helper` defined only in
/// `::myns`, `apply {{} {helper} ::myns}` returns its result, while
/// `apply {{} {helper}}` fails `invalid command name "helper"`.
///
/// The analyser records that namespace as a span-keyed override the LSP's
/// command resolution consults. A substitution-position lambda recorded none
/// at all without the hook.
#[test]
fn apply_lambda_in_command_substitution_records_its_namespace() {
    let overrides = |src: &str| -> Vec<String> {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl9.0")
            .namespace_overrides
            .into_iter()
            .map(|(_, ns)| ns)
            .collect()
    };
    // TP — explicit third element wins, in substitution position.
    assert_eq!(
        overrides("proc caller {} {\n    return [apply {{} {helper} ::myns}]\n}\n"),
        vec!["::myns".to_string()],
    );
    // TP — an unqualified namespace word is still resolved against the global
    // namespace, not the caller's.
    assert_eq!(
        overrides(
            "namespace eval ::outer {\n    proc caller {} {\n        return [apply {{} {helper} sub}]\n    }\n}\n"
        ),
        vec!["::sub".to_string()],
    );
    // TN — no third element means the global namespace, not `caller`'s.
    assert_eq!(
        overrides("proc caller {} {\n    return [apply {{} {helper}}]\n}\n"),
        vec!["::".to_string()],
    );
    // TN — the statement-position form was always right; it must stay
    // byte-identical to the substitution-position answer.
    assert_eq!(
        overrides("proc caller {} {\n    apply {{} {helper} ::myns}\n}\n"),
        overrides("proc caller {} {\n    return [apply {{} {helper} ::myns}]\n}\n"),
    );
}

// BODY role on iRules nesting scripts

#[test]
fn analyser_recurses_into_irules_nesting_script_bodies() {
    // clientside / serverside / peer / after now carry an
    // `ArgRole::Body`, so the analyser descends into the nesting
    // script and flags problems inside it.  A nested `set` with no
    // arguments trips E002 only when the body is actually analysed —
    // i.e. the generic body-walk picks the role up automatically.
    for src in [
        "when CLIENT_DATA { clientside { set } }",
        "when CLIENT_ACCEPTED { serverside { set } }",
        "when CLIENT_ACCEPTED { peer { set } }",
        "when RULE_INIT { after 1000 { set } }",
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "f5-irules");
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == DiagCode::E002 && d.message.contains("'set'")),
            "expected E002 from the nested `set` (body must be analysed) in {src:?}, got {:?}",
            r.diagnostics
        );
    }
}

#[test]
fn when_body_not_analysed_under_plain_tcl() {
    // `when` is an iRules-only builtin; under plain Tcl it is a disabled
    // foreign-dialect command whose braced argument is opaque *data*, not a
    // handler script.  The body must not be scanned: only W002 (disabled) +
    // W123 (unknown-in-dialect) on `when` itself — no W210 on the body's
    // `$undefvar`, no W123 naming the body command.
    let mut a = Analyser::new();
    let r = a.analyse("when HTTP_REQUEST {\n    boguscmd $undefvar\n}\n", "tcl");
    let codes: Vec<&str> = r.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert!(
        !codes.contains(&"W210"),
        "disabled `when` body must not be analysed (no W210); got {:?}",
        r.diagnostics
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123 && d.message.contains("boguscmd")),
        "disabled `when` body command must not draw W123; got {:?}",
        r.diagnostics
    );
    // The `when` head itself is still flagged disabled + unknown-in-dialect.
    assert!(codes.contains(&"W002"), "expected W002; got {codes:?}");
}

#[test]
fn when_body_analysed_under_irules() {
    // Under the iRules dialect `when` IS enabled, so its body is a real
    // handler script and the read-before-set inside it fires (the inverse
    // of `when_body_not_analysed_under_plain_tcl`).
    let mut a = Analyser::new();
    let r = a.analyse("when HTTP_REQUEST {\n    set x $undefvar\n}\n", "f5-irules");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("undefvar")),
        "iRules `when` body must be analysed (W210 on $undefvar); got {:?}",
        r.diagnostics
    );
}

#[test]
fn catch_body_is_walked_for_syntactic_checks() {
    // `catch { … }` evaluates its script body, so the per-command syntactic
    // checks (here W100, unbraced `expr`) must reach inside it:
    // `handle_catch_command` recurses into `args[0]` as well as defining
    // the result/options vars.
    assert_eq!(count_code("catch { expr $x+1 }\n", "W100"), 1);
    assert_eq!(count_code("catch { set y [expr $x+1] } res\n", "W100"), 1);
    // A dynamic body (`catch $cmd`) stays opaque — nothing to walk.
    assert_eq!(count_code("catch $cmd\n", "W100"), 0);
}

#[test]
fn tcltest_test_body_is_walked_when_imported() {
    let count_loaded = |source: &str| {
        crate::provider_fixtures::analyse(
            source,
            "tcl8.6",
            &[crate::provider_fixtures::Provider::Tcltest],
        )
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.as_str() == "W100")
        .count()
    };
    // `tcltest::test` carries `Body` roles on its `-setup`/`-body`/`-cleanup`
    // option values and on the legacy positional body (penultimate arg), so
    // the analyser descends into the test script — both when the command is
    // called fully-qualified and when it is reached through a
    // `namespace import ::tcltest::*` by its bare name.
    let qualified = "package require tcltest\n\
                     tcltest::test t1 {d} { expr $x+1 } {}\n";
    assert_eq!(count_loaded(qualified), 1);
    assert_eq!(
        crate::provider_fixtures::analyse(qualified, "tcl8.6", &[])
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W100)
            .count(),
        0,
        "unattested require does not enter a test body"
    );

    let imported = "package require tcltest\n\
                    namespace import -force ::tcltest::*\n\
                    test t1 {d} -body { expr $x+1 } -result {}\n";
    assert_eq!(count_loaded(imported), 1);

    // The expected-result field is data, not a script, and must not be walked.
    let result_field = "package require tcltest\n\
                        namespace import -force ::tcltest::*\n\
                        test t1 {d} -body { puts ok } -result {[expr $x]}\n";
    assert_eq!(count_loaded(result_field), 0);

    // Without the import, a bare `test` is an unknown command whose braced
    // argument is an opaque string — do not recurse (matches Tcl: `test` is
    // undefined until tcltest is loaded).
    assert_eq!(count_code("test t1 {d} { expr $x+1 } {}\n", "W100"), 0);
}

#[test]
fn tcltest_shared_frame_preserves_the_reported_setup_body_cleanup_reads() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for source in [
            include_str!("../../../../tests/data/diagnostics/tcltest-shared-frame/1.tcl"),
            include_str!("../../../../tests/data/diagnostics/tcltest-shared-frame/2.tcl"),
            include_str!("../../../../tests/data/diagnostics/tcltest-shared-frame/3.tcl"),
            include_str!("../../../../tests/data/diagnostics/tcltest-shared-frame/4.tcl"),
        ] {
            let result = crate::provider_fixtures::analyse(
                source,
                profile,
                &[crate::provider_fixtures::Provider::Tcltest],
            );
            let undefined = result
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::W210)
                .collect::<Vec<_>>();
            assert!(undefined.is_empty(), "{profile}/{source}\n{undefined:?}");
        }
    }
}

#[test]
fn tcltest_shared_frame_keeps_genuine_undefined_reads_and_data_opaque() {
    let source = "package require tcltest\n\
                  namespace import -force ::tcltest::*\n\
                  proc exercise {} {\n\
                  test t shared -setup {set prepared 1} \
                  -body {puts $prepared; puts $absent} \
                  -cleanup {puts $prepared} -result {[puts $data_only]}\n\
                  }\nexercise\n";
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = crate::provider_fixtures::analyse(
            source,
            profile,
            &[crate::provider_fixtures::Provider::Tcltest],
        );
        let undefined = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W210)
            .collect::<Vec<_>>();
        assert_eq!(undefined.len(), 1, "{profile}/{undefined:?}");
        assert!(
            undefined[0].message.contains("absent"),
            "{profile}/{undefined:?}"
        );
        // An explicit driver with no selected loader keeps the callback data
        // opaque. The default C authoring driver selects its audited provider.
        let unattested = crate::provider_fixtures::analyse(source, profile, &[]);
        let unattested_undefined = unattested
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W210)
            .collect::<Vec<_>>();
        assert!(
            unattested_undefined.is_empty(),
            "{profile}: a package requirement alone cannot authenticate callback entry: {unattested_undefined:?}"
        );
    }
}

#[test]
fn tcltest_oracle_program_keeps_missing_reads_and_setup_completion() {
    let source =
        include_str!("../../../../../../experiments/name-handling/tcltest-shared-frame/probe.tcl");
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = crate::provider_fixtures::analyse(
            source,
            profile,
            &[crate::provider_fixtures::Provider::Tcltest],
        );
        let undefined = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W210)
            .collect::<Vec<_>>();
        assert_eq!(undefined.len(), 2, "{profile}/{undefined:?}");
        for diagnostic in undefined {
            assert_eq!(&source[diagnostic.span.as_range()], "$absent", "{profile}");
        }
    }
}

/// Change only the original test head/name/description fixture words. The
/// remaining original option words and every setup/body/cleanup byte stay
/// unchanged. Captured name/description words belong to the alias declaration.
fn tcltest_aliased_fixture(source: &str, profile: &str, captured: bool) -> (String, u32) {
    let context = tcl_registry::model::ingress::static_context_for(profile);
    let config = tcl_lexer::LexerConfig::for_profile(context.commands().profile());
    let command = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
        .into_iter()
        .find(|command| command.name() == "test")
        .expect("one original reported test command");
    assert!(command.word_views_aligned());
    let head = tcl_lexer::word_span_at(source, command.argv[0].span);
    let last_header_word = tcl_lexer::word_span_at(source, command.argv[2].span);
    let capture = if captured {
        &source[head.end() as usize..last_header_word.end() as usize]
    } else {
        ""
    };
    let tail_start = if captured {
        last_header_word.end()
    } else {
        head.end()
    };
    let prefix = format!(
        "{}interp alias {{}} report_test {{}} ::tcltest::test{capture}\n",
        &source[..head.start() as usize]
    );
    let offset = u32::try_from(prefix.len()).expect("finite fixture offset");
    let aliased = format!("{prefix}report_test{}", &source[tail_start as usize..]);
    assert_eq!(
        &aliased[offset as usize + "report_test".len()..],
        &source[tail_start as usize..],
        "original option/body bytes are retained"
    );
    (aliased, offset)
}

#[test]
fn tcltest_aliases_preserve_the_reported_setup_body_cleanup_reads() {
    // naming.diagnostic.original-package-source-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
    // Original #2286 programs, source-only alias/ordinal and W210 regression.
    // Selecting a package fixture grants no new Native handler/frame permission.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for source in [
            include_str!("../../../../tests/data/diagnostics/tcltest-shared-frame/1.tcl"),
            include_str!("../../../../tests/data/diagnostics/tcltest-shared-frame/2.tcl"),
            include_str!("../../../../tests/data/diagnostics/tcltest-shared-frame/3.tcl"),
            include_str!("../../../../tests/data/diagnostics/tcltest-shared-frame/4.tcl"),
        ] {
            for captured in [false, true] {
                let (aliased, offset) = tcltest_aliased_fixture(source, profile, captured);
                let analysis = crate::provider_fixtures::analyse(
                    &aliased,
                    profile,
                    &[crate::provider_fixtures::Provider::Tcltest],
                );
                let undefined = analysis
                    .diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.code == DiagCode::W210)
                    .collect::<Vec<_>>();
                assert!(
                    undefined.is_empty(),
                    "{profile}/captured={captured}/{aliased}\n{undefined:?}"
                );
                let words = crate::registry_invocation::source_structure::source_registry_words_at(
                    &aliased, &analysis, offset,
                )
                .expect("actual package-selected original alias source");
                let captured_count = usize::from(captured) * 2;
                for argument in 0..captured_count {
                    assert_eq!(
                        words.origins()[argument + 1],
                        crate::registry_invocation::InvocationWordOrigin::BindingPrefix(argument)
                    );
                    assert!(words.operands()[argument].is_none());
                    let original = words.original_argument_word(argument).unwrap();
                    assert!(original.span().end() < offset);
                }
                let roles = words.roles().expect("selected original lifecycle roles");
                let mut body_count = 0;
                for &(argument, role) in roles {
                    if role != tcl_registry::ArgRole::Body {
                        continue;
                    }
                    body_count += 1;
                    assert_eq!(
                        words.origins()[argument + 1],
                        crate::registry_invocation::InvocationWordOrigin::Written(
                            argument + 1 - captured_count
                        )
                    );
                    let original = words.original_argument_word(argument).unwrap();
                    assert!(original.span().start() > offset);
                    assert_eq!(original.image().bytes(), aliased.as_bytes());
                }
                assert_eq!(body_count, 3, "setup/body/cleanup remain script operands");
            }
        }
    }
}

#[test]
fn tcltest_aliases_keep_genuine_undefined_reads_and_result_data_opaque() {
    // naming.diagnostic.original-package-source-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
    // Captured effective ordinals do not move the source read to result data.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for captured in [false, true] {
            let alias = if captured {
                "interp alias {} report_test {} ::tcltest::test t shared"
            } else {
                "interp alias {} report_test {} ::tcltest::test"
            };
            let header = if captured { "" } else { "t shared " };
            let source = format!(
                "package require tcltest\nnamespace import -force ::tcltest::*\n{alias}\n\
                 proc exercise {{}} {{report_test {header}-setup {{set prepared 1}} \
                 -body {{puts $prepared; puts $absent}} \
                 -cleanup {{puts $prepared}} -result {{[puts $data_only]}}}}\nexercise\n"
            );
            let selected = crate::provider_fixtures::analyse(
                &source,
                profile,
                &[crate::provider_fixtures::Provider::Tcltest],
            );
            let undefined = selected
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::W210)
                .collect::<Vec<_>>();
            assert_eq!(undefined.len(), 1, "{profile}/{source}\n{undefined:?}");
            assert_eq!(
                &source[undefined[0].span.as_range()],
                "$absent",
                "the original body read owns the warning, not setup/cleanup/result data"
            );
            let call_offset = u32::try_from(source.rfind("report_test ").unwrap()).unwrap();
            assert!(
                crate::registry_invocation::source_structure::source_registry_words_at(
                    &source,
                    &selected,
                    call_offset,
                )
                .is_some()
            );
            let mut missing = selected.clone();
            missing.resolved_input = None;
            assert!(
                crate::registry_invocation::source_structure::source_registry_words_at(
                    &source,
                    &missing,
                    call_offset,
                )
                .is_none()
            );
            assert!(
                crate::registry_invocation::source_structure::source_registry_words_at(
                    &(source.clone() + "# changed image\n"),
                    &selected,
                    call_offset,
                )
                .is_none()
            );
            for opaque_source in [
                source.clone(),
                source.replacen(
                    alias,
                    &format!("proc ::tcltest::test args {{return ordinary}}\n{alias}"),
                    1,
                ),
            ] {
                let selected_providers = [crate::provider_fixtures::Provider::Tcltest];
                let providers = if opaque_source == source {
                    &[][..]
                } else {
                    &selected_providers[..]
                };
                let opaque = crate::provider_fixtures::analyse(&opaque_source, profile, providers);
                assert!(
                    opaque
                        .diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.code != DiagCode::W210),
                    "an unselected or replaced package handler keeps body data opaque: {opaque_source}"
                );
            }
        }
    }
}

#[test]
fn append_and_lappend_define_their_target_variable() {
    // `append`/`lappend` create their first argument if absent, so the target
    // is a variable definition (it must surface in `symbols`/completion/hover).
    // An `append` / `lappend` target defines a variable too — a symbol table
    // fed only by `set` / `variable` / `global` / `incr` drops it.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("lappend safe 1\nappend out hi\n", "tcl8.6");
    assert!(
        r.global_scope.variables.contains_key("safe"),
        "lappend target"
    );
    assert!(
        r.global_scope.variables.contains_key("out"),
        "append target"
    );
    // A read-modify-write target is not "set but never used": no W211.
    assert_eq!(count_code("lappend safe 1\n", "W211"), 0);
}

#[test]
fn nested_catch_result_var_is_defined() {
    // `catch SCRIPT ?resultVar? ?optionsVar?` binds its result/options vars even
    // when the `catch` is nested in a `[...]` substitution, so they must reach
    // the symbol table (symbols/completion/hover).
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("set out [catch {error x} msg opts]\n", "tcl8.6");
    assert!(
        r.global_scope.variables.contains_key("msg"),
        "catch result var"
    );
    assert!(
        r.global_scope.variables.contains_key("opts"),
        "catch options var"
    );
    // The binding is a side effect of catch, not a "set but unused" target.
    assert_eq!(count_code("if {[catch {foo} e]} {puts $e}\n", "W211"), 0);
}

#[test]
fn catch_body_package_require_is_conditional() {
    // `catch { package require Foo }` is a guarded optional-dependency probe, so
    // the package requirement recorded from inside the catch body must be marked
    // conditional (not promoted to an unconditional fact).
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("catch { package require Foo 1.2 }\n", "tcl8.6");
    let foo = r
        .package_requires
        .iter()
        .find(|p| p.name == "Foo")
        .expect("Foo package require recorded");
    assert!(
        foo.conditional,
        "catch-body package require must be conditional"
    );

    // A top-level `package require` stays unconditional.
    let mut b = crate::analyser::Analyser::new();
    let r2 = b.analyse("package require Bar\n", "tcl8.6");
    let bar = r2
        .package_requires
        .iter()
        .find(|p| p.name == "Bar")
        .unwrap();
    assert!(
        !bar.conditional,
        "top-level package require must be unconditional"
    );
}

#[test]
fn tcltest_import_is_namespace_scoped() {
    let count_loaded = |source: &str| {
        crate::provider_fixtures::analyse(
            source,
            "tcl8.6",
            &[crate::provider_fixtures::Provider::Tcltest],
        )
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.as_str() == "W100")
        .count()
    };
    // A `namespace import ::tcltest::*` made *inside* a namespace must not
    // resolve a bare `test` call in a sibling/parent namespace.
    let inside_ns_top_level_call = "package require tcltest\n\
        namespace eval ns { namespace import -force ::tcltest::* }\n\
        test t {d} { expr $x+1 } {}\n";
    assert_eq!(count_loaded(inside_ns_top_level_call), 0);

    // Same-namespace call still resolves and recurses.
    let same_ns = "package require tcltest\n\
        namespace eval ns { namespace import -force ::tcltest::* ; test t {d} { expr $x+1 } {} }\n";
    assert_eq!(count_loaded(same_ns), 1);

    // A global-scope import still applies at top level (Tcl's `::` fallback).
    let top_level = "package require tcltest\n\
        namespace import -force ::tcltest::*\n\
        test t {d} { expr $x+1 } {}\n";
    assert_eq!(count_loaded(top_level), 1);
}
