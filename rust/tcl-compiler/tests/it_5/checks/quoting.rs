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

//! Quoting and shape checks: unbraced expressions and bodies, string and list confusion, unmatched closers and orphaned keywords.

// W100 — unbraced expr argument to expr/if/while/for.
//
// tclsh ground truth: `expr $x + 1` double-substitutes `$x` and re-parses the
// result as an expression — the canonical "always brace your expr" footgun
// (`expr {$x + 1}` is the safe form). A `$`/`[` substitution makes it dangerous
// → Error severity; a quoted literal with no substitution is style-only → Warning.
mod unbraced_expr {
    use super::super::*;

    #[test]
    fn unbraced_expr_variable_is_error() {
        let ds = of_code("expr $x + 1", D, "W100");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("not braced"));
        // $x substitution → dangerous → Error.
        assert_eq!(ds[0].1, Severity::Error);
    }

    #[test]
    fn unbraced_quoted_literal_is_warning() {
        // `if "1 > 0" {…}` — no substitution → style-only → Warning.
        let ds = of_code("if \"1 > 0\" {puts yes}", D, "W100");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Warning);
    }

    #[test]
    fn braced_expr_clean() {
        assert!(!fires("expr {$x + 1}", D, "W100"));
        assert!(!fires("if {$x > 0} {puts yes}", D, "W100"));
        assert!(!fires("while {$i < 10} {incr i}", D, "W100"));
        assert!(!fires(
            "for {set i 0} {$i < 10} {incr i} {puts $i}",
            D,
            "W100"
        ));
    }

    #[test]
    fn unbraced_if_while_for_conditions_fire() {
        assert_eq!(count("if $x {puts yes}", D, "W100"), 1);
        assert_eq!(count("while $running {update}", D, "W100"), 1);
        assert_eq!(
            count("for {set i 0} $cond {incr i} {puts $i}", D, "W100"),
            1
        );
    }

    #[test]
    fn numeric_and_boolean_literals_clean() {
        // Pure numeric / boolean literals are safe unbraced.
        assert!(!fires("expr 42", D, "W100"));
        assert!(!fires("if true {puts yes}", D, "W100"));
    }

    #[test]
    fn expr_with_command_subst_fires() {
        // `expr [clock seconds]` re-parses the command's result → warn.
        assert_eq!(count("expr [clock seconds]", D, "W100"), 1);
    }

    #[test]
    fn has_brace_wrapping_fix() {
        let ds = of_code("expr $x + 1", D, "W100");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2.len(), 1);
        let fix = &ds[0].2[0];
        assert!(fix.starts_with('{') && fix.ends_with('}'));
        assert_eq!(fix, "{$x + 1}");
    }

    #[test]
    fn if_fix_preserves_variable_marker() {
        // `if $x {…}` → fix `{$x}` (the substitution survives the wrap).
        let ds = of_code("if $x {puts yes}", D, "W100");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2, vec!["{$x}".to_string()]);
    }

    #[test]
    fn quoted_expr_fix_drops_outer_quotes() {
        let ds = of_code("set ok [expr \"$a == $b\"]", D, "W100");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2, vec!["{$a == $b}".to_string()]);
    }

    #[test]
    fn unbraced_expr_inside_when_switch_body_under_irules() {
        // Nested `when … { switch … { … { set n [expr $n << 5] } } }` still warns.
        let src = "\nwhen ACCESS_POLICY_AGENT_EVENT {\n    switch x {\n        \"a\" {\n            set n [expr $n << 5]\n        }\n    }\n}\n";
        assert_eq!(count(src, IR, "W100"), 1);
    }
}

// W104 — string/list confusion (append with space-prefixed values).
//
// Pure style heuristic (no runtime analogue): `append items " item"` builds a
// space-separated string that looks like list construction; `lappend` is the
// list-safe form.
//
// The heuristic is coarse: it does not suppress pure separators
// (`", "` / `": "`) or usage notation (`" ?opt?..."`, `" <value>"`) — W104
// fires on those too. Each case below is pinned to the actual verdict.
mod string_list_confusion {
    use super::super::*;

    #[test]
    fn append_with_leading_space_fires() {
        let ds = of_code("append items \" item\"", D, "W104");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("list"));
    }

    #[test]
    fn append_normal_value_clean() {
        // No leading space → ordinary string append, not list-shaped.
        assert!(!fires("append result \"value\"", D, "W104"));
    }

    #[test]
    fn lappend_never_flagged() {
        assert!(!fires("lappend items item", D, "W104"));
    }

    #[test]
    fn genuine_list_element_fires() {
        assert_eq!(count("append items \" $item\"", D, "W104"), 1);
    }

    #[test]
    fn pure_separator_and_usage_notation_fire_rust_behaviour() {
        // These ideally would not flag (a separator joins a string; usage
        // notation is display formatting), but the space-prefixed-value heuristic
        // does not special-case them, so W104 fires. Pinned to the actual verdict.
        for src in [
            "append msg \", \"",
            "append msg \": \"",
            "append result \" ?option value?...\"",
            "append name \" <value>\"",
        ] {
            assert_eq!(
                count(src, D, "W104"),
                1,
                "expected W104 (Rust verdict) for {src:?}"
            );
        }
    }
}

// W110 — ==/!= used for string comparison in an expr.
//
// tclsh: `expr {$x == "foo"}` compares numerically-then-stringwise; `eq`/`ne`
// are the explicit string operators. Fires only when one operand is a string
// literal (two variables might both be ints → no warning).
mod string_compare_in_expr {
    use super::super::*;

    #[test]
    fn eq_suggested_for_string_literal_double_equals() {
        let ds = of_code("if {$x == \"foo\"} {puts yes}", D, "W110");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("eq"));
        assert_eq!(ds[0].2.len(), 1);
        assert!(ds[0].2[0].contains("eq"));
    }

    #[test]
    fn ne_suggested_for_string_literal_bang_equals() {
        let ds = of_code("if {$x != \"bar\"} {puts no}", D, "W110");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("ne"));
    }

    #[test]
    fn numeric_and_variable_only_comparisons_clean() {
        assert!(!fires("if {$x == 42} {puts yes}", D, "W110"));
        assert!(!fires("if {$x eq \"foo\"} {puts yes}", D, "W110"));
        // Two variables — could both be ints → no warning.
        assert!(!fires("if {$a == $b} {puts yes}", D, "W110"));
        // Unbraced expr with variables only — outer quotes do not trigger.
        assert!(!fires("set r [expr \"$a == $b\"]", D, "W110"));
    }

    #[test]
    fn fires_only_where_eq_keeps_the_result() {
        // `==` compares numerically when both operands are numbers, so `eq`
        // gives the same answer only when one operand is a fixed string that
        // is not a number in any release. tclsh 8.4.20 through 9.1b0: `x` =
        // `42.0` makes `$x == "42"` true and `$x eq "42"` false; `"$x" == 1`
        // likewise at `1.0`; ` 1`, `nan`, `0x10`, `08` (9.0) and `1_0` (9.0)
        // are numbers. A boolean word is not: `"true" == 1` is false.
        for (src, fires) in [
            ("if {$x == \"42\"} {puts yes}", false),
            ("if {\"$x\" == 1} {puts yes}", false),
            ("if {\"$x\" == \"$y\"} {puts yes}", false),
            ("if {$x == \" 1\"} {puts yes}", false),
            ("if {$x == \"nan\"} {puts yes}", false),
            ("if {$x == \"0x10\"} {puts yes}", false),
            ("if {$x == \"08\"} {puts yes}", false),
            ("if {$x == \"1_0\"} {puts yes}", false),
            ("if {$x == \"true\"} {puts yes}", true),
            ("if {$x == \"\"} {puts yes}", true),
            ("if {$x == {foo}} {puts yes}", true),
            ("if {\"$x\" == \"foo\"} {puts yes}", true),
            // Unbraced, the words are substituted before `expr` parses them,
            // so nothing is proven about the operands it will see.
            ("if \"\\$x == {foo}\" {puts yes}", false),
        ] {
            assert_eq!(count(src, D, "W110") == 1, fires, "{src}");
        }
    }

    #[test]
    fn the_fix_rewrites_only_the_operators() {
        // The literal keeps its `==` and its spacing, the braces stay, and an
        // unproven compare keeps its operator.
        for (src, applied) in [
            (
                "if {$x == \"a  ==b\"} {puts yes}",
                "if {$x eq \"a  ==b\"} {puts yes}",
            ),
            (
                "while {$x==\"foo\"} {break}",
                "while {$x eq \"foo\"} {break}",
            ),
            (
                "if {$a == $b && $c != {x} || $d == \"\"} {puts yes}",
                "if {$a == $b && $c ne {x} || $d eq \"\"} {puts yes}",
            ),
        ] {
            let mut a = Analyser::new();
            let r = a.analyse(src, D);
            let fixes: Vec<_> = r
                .diagnostics
                .iter()
                .filter(|d| d.code.to_string() == "W110")
                .flat_map(|d| d.fixes.iter())
                .collect();
            assert_eq!(fixes.len(), 1, "{src}: {fixes:?}");
            let (s, e) = (fixes[0].span.start() as usize, fixes[0].span.end() as usize);
            assert_eq!(
                format!("{}{}{}", &src[..s], fixes[0].new_text, &src[e..]),
                applied
            );
        }
    }

    #[test]
    fn negated_string_compare_warns() {
        // W110 fires through the unary negation wrapping a string ==.
        assert_eq!(count("if {!($x == \"foo\")} {puts no}", D, "W110"), 1);
    }

    #[test]
    fn mixed_ops_fix_only_the_proven_compare() {
        let ds = of_code("if {$a == $b || $x == \"foo\"} {puts y}", D, "W110");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2.len(), 1);
    }
}

// W105 / W106 — unbraced code-block / switch-arm bodies.
//
// tclsh: `if {1} "puts $x"` substitutes `$x` before the body runs as code → a
// double-substitution / injection risk (Error). A single bare variable body
// (`eval $script`) is a script-valued reference, not an inline block → silent.
mod unbraced_body {
    use super::super::*;

    #[test]
    fn if_unbraced_quoted_body_with_var_is_error() {
        let ds = of_code("if {1} \"puts $x\"", D, "W105");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Error);
    }

    #[test]
    fn if_braced_body_clean() {
        assert!(!fires("if {1} {puts hello}", D, "W105"));
    }

    #[test]
    fn eval_single_var_or_list_body_not_flagged() {
        // `eval $script` / `eval [list …]` are script-valued; cannot be braced.
        assert!(!fires("eval $script", D, "W105"));
        assert!(!fires("proc f {a} { eval [list puts $a] }", D, "W105"));
        assert!(!fires("proc f {a} { eval [linsert $a 0 puts] }", D, "W105"));
    }

    #[test]
    fn eval_quoted_substitution_still_flagged() {
        assert_eq!(count("proc f {a} { eval \"puts $a\" }", D, "W105"), 1);
    }

    #[test]
    fn switch_unbraced_arm_body_with_var_is_error() {
        let ds = of_code("switch -- $x a \"puts $y\" b {puts ok}", D, "W106");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Error);
    }

    #[test]
    fn switch_braced_body_clean() {
        assert!(!fires(
            "switch -- $x { a {puts hi} b {puts ok} }",
            D,
            "W106"
        ));
    }

    #[test]
    fn switch_value_options_do_not_turn_patterns_into_commands() {
        let src = "switch -matchvar m -- $s {a {puts A} b {puts B}}";
        assert!(!fires(src, D, "W123"), "{:?}", codes(src, D));
    }

    #[test]
    fn switch_regexp_unbraced_body_is_error() {
        let ds = of_code("switch -regexp -- $x {^a} \"puts matched\"", D, "W106");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("regexp"));
        assert_eq!(ds[0].1, Severity::Error);
    }
}

// W108 — non-ASCII token content (confusables mode).
//
// Pure style/security heuristic: in the default "confusables" mode, smart
// quotes and homoglyphs (Cyrillic А) are flagged with an ASCII auto-fix;
// non-confusable symbols (© °) are allowed.
mod non_ascii {
    use super::super::*;

    #[test]
    fn smart_quotes_flagged_twice() {
        assert_eq!(count("set x \u{201c}hello\u{201d}", D, "W108"), 2);
    }

    #[test]
    fn cyrillic_homoglyph_flagged_with_fix() {
        let ds = of_code("set x \u{0410}bc", D, "W108");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2, vec!["A".to_string()]); // ASCII auto-fix
    }

    #[test]
    fn benign_symbols_allowed_in_default_mode() {
        assert!(!fires("set x \u{00a9}value", D, "W108")); // ©
        assert!(!fires("set x {90\u{00b0}}", D, "W108")); // °
    }

    #[test]
    fn pure_ascii_and_whitespace_clean() {
        assert!(!fires("set x hello", D, "W108"));
        assert!(!fires("set x {hello\tworld}", D, "W108"));
    }
}

// W212 — variable substitution where a variable *name* is expected.
//
// tclsh: `set $x 1` sets the variable *named by the value of* `$x` — almost
// never intended (the writer meant `set x 1`). Same for incr/append/lappend/
// unset/info-exists/upvar's local slot. `upvar`'s *remote* slot legitimately
// takes a `$var`; `info commands` is not name-taking.
mod name_vs_value {
    use super::super::*;

    #[test]
    fn set_incr_append_lappend_dollar_var_fire() {
        let ds = of_code("set $x 1", D, "W212");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("$x"));
        assert_eq!(count("incr $count", D, "W212"), 1);
        assert_eq!(count("append $buf \"text\"", D, "W212"), 1);
        assert_eq!(count("lappend $mylist item", D, "W212"), 1);
    }

    #[test]
    fn plain_names_clean() {
        for src in [
            "set x 1",
            "incr count",
            "append buf \"text\"",
            "lappend mylist item",
            "unset x",
        ] {
            assert!(!fires(src, D, "W212"), "unexpected W212 for {src:?}");
        }
    }

    #[test]
    fn unset_dollar_vars_fire_through_options() {
        assert_eq!(count("unset $x", D, "W212"), 1);
        assert_eq!(count("unset -nocomplain $x", D, "W212"), 1);
        assert_eq!(count("unset -nocomplain -- $x", D, "W212"), 1);
        // Two substituted names → two diagnostics.
        assert_eq!(count("unset $a $b", D, "W212"), 2);
    }

    #[test]
    fn info_exists_dollar_var_fires_commands_does_not() {
        let ds = of_code("info exists $x", D, "W212");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("info exists"));
        // Only `exists` is name-taking; `info commands $pattern` is fine.
        assert!(!fires("info commands $pattern", D, "W212"));
    }

    #[test]
    fn upvar_local_dollar_fires_remote_ok() {
        // The local-alias slot is name-taking; the remote slot legitimately
        // takes a `$var`.
        assert_eq!(count("upvar 1 other $local", D, "W212"), 1);
        assert_eq!(count("upvar other $local", D, "W212"), 1);
        assert!(!fires("upvar 1 $remote local", D, "W212"));
        assert!(!fires("upvar 1 other local", D, "W212"));
        // Two pairs, two bad locals → two diagnostics.
        assert_eq!(count("upvar 1 a $b c $d", D, "W212"), 2);
    }
}

// E100 — unmatched close bracket.
//
// `set x foo]` has a stray `]` (the lexer flags it ESC, not a structural
// closer). A properly matched `[cmd]` or a `]` inside braces is fine.
mod unmatched_close_bracket {
    use super::super::*;

    #[test]
    fn bare_close_bracket_detected() {
        let ds = of_code("set x foo]", D, "E100");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains(']'));
    }

    #[test]
    fn matched_bracket_and_braced_clean() {
        assert!(!fires("set x [string length foo]", D, "E100"));
        assert!(!fires("set x {contains ]}", D, "E100"));
    }

    #[test]
    fn no_fix_without_known_command_or_overflow() {
        // `set x blah]` — no recognised command / arity overflow → diagnostic, no fix.
        let ds = of_code("set x blah]", D, "E100");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2.len(), 0);
    }

    #[test]
    fn known_command_inserts_bracket_under_irules() {
        // ACCESS::policy is a known iRules command preceding `]`.
        let ds = of_code("switch -- ACCESS::policy result]", IR, "E100");
        assert_ne!(
            ds,
            [] as [(
                std::string::String,
                tcl_compiler::analyser::Severity,
                std::vec::Vec<std::string::String>
            ); 0]
        );
        assert_eq!(ds[0].2, vec!["[".to_string()]);
    }

    // TP: a call to an already-declared *user* proc missing its `[` is
    // just as recoverable as a call to a registry builtin — the E100
    // quick-fix heuristic must consult the analyser's own proc/class/
    // alias/ensemble tracking, not just `CommandRegistry`.
    #[test]
    fn known_user_proc_inserts_bracket() {
        let src = "proc myHelper {a b} {return $a}\nset y myHelper arg1 arg2]\n";
        let ds = of_code(src, D, "E100");
        assert_eq!(ds.len(), 1, "{ds:?}");
        assert_eq!(ds[0].2, vec!["[".to_string()]);
        // The repair must not corrupt the recorded command name into a
        // spurious "unknown command" elsewhere in the pipeline.
        assert!(!fires(src, D, "W123"), "{:?}", codes(src, D));
    }

    // TP: same for a tclOO class created earlier in the file.
    #[test]
    fn known_tcloo_class_inserts_bracket() {
        let src = "oo::class create Widget {}\nset y Widget create]\n";
        let ds = of_code(src, D, "E100");
        assert_eq!(ds.len(), 1, "{ds:?}");
        assert_eq!(ds[0].2, vec!["[".to_string()]);
    }

    // FN (was silently missed): `\\]` is a literal backslash (even run)
    // followed by a *bare* `]` — Tcl's `\\` fully consumes as a pair, so
    // the bracket is not escaped and the stray-bracket heuristic must
    // still fire.
    #[test]
    fn double_backslash_bracket_still_fires() {
        assert!(fires(r"puts foo\\]", D, "E100"));
    }

    // TN: a single backslash *does* escape the bracket (odd run) — a
    // deliberate literal `]`, not a typo.
    #[test]
    fn single_backslash_bracket_is_clean() {
        assert!(!fires(r"puts foo\]", D, "E100"));
    }

    // TN (was a real bug): a genuinely escaped trailing `]` combined
    // with an enclosing arity overflow must not be "repaired" by the
    // arity-fallback heuristic — the old, independent recovery detector
    // ignored escaping entirely and could merge tokens the diagnostic
    // itself correctly treated as inert, corrupting downstream analysis
    // with no governing E100 to explain it.
    #[test]
    fn escaped_bracket_under_arity_overflow_is_clean() {
        let src = r"set y bar baz\]";
        assert!(!fires(src, D, "E100"), "{:?}", codes(src, D));
        assert!(!fires(src, D, "W123"), "{:?}", codes(src, D));
    }
}

// E102 — unmatched close brace.
//
// A bare `}` has no special meaning to Tcl outside a `{…}` word, so — like
// E100 — this is a "probably a typo" heuristic, not a hard parse error. It
// must fire wherever the `}` is stray, whether it is the whole token
// (`puts foo\n}`) or embedded in a larger word (`set x foo}bar`); a matched
// `{…}` or a `}` inside a quoted/braced word is fine.
mod unmatched_close_brace {
    use super::super::*;

    #[test]
    fn bare_close_brace_detected() {
        let ds = of_code("set x 1\n}\n", D, "E102");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains('}'));
    }

    #[test]
    fn matched_brace_and_quoted_clean() {
        assert!(!fires("if {1} {puts hi}\n", D, "E102"));
        assert!(!fires("puts \"a }\"\n", D, "E102"));
    }

    // TP (was a confirmed false negative): a `}` embedded in a larger
    // bareword is just as stray as a lone `}` — the old detector only
    // matched a token whose *entire* text was `}`, so `foo}bar` produced
    // no diagnostic at all, asymmetric with E100's compound-token scan
    // of `foo]bar`.
    #[test]
    fn embedded_close_brace_detected() {
        assert!(fires("set x foo}bar\n", D, "E102"));
    }

    #[test]
    fn embedded_close_brace_escaped_is_clean() {
        assert!(!fires("set x foo\\}bar\n", D, "E102"));
    }

    #[test]
    fn embedded_close_brace_has_no_fix() {
        // The whole-line-deletion fix only applies when `}` is the
        // entire line; an embedded brace gets a diagnostic but no fix.
        let ds = of_code("set x foo}bar\n", D, "E102");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2.len(), 0);
    }

    #[test]
    fn isolated_close_brace_offers_line_deletion_fix() {
        let ds = of_code("set x 1\n}\n", D, "E102");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2, vec![String::new()]);
    }
}

// W125 — orphaned control-flow keyword used as a standalone command.
//
// `else`/`elseif`/`then`/`finally`/`on`/`trap` on their own line (not `} else
// {` on the same line) parse as separate commands. A user proc named after the
// keyword suppresses it.
mod orphaned_keyword {
    use super::super::*;

    #[test]
    fn orphaned_else_names_if() {
        let src = "if {1} {\n    puts yes\n}\nelse {\n    puts no\n}\n";
        let ds = of_code(src, D, "W125");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("else"));
        assert!(ds[0].0.contains("if"));
    }

    #[test]
    fn orphaned_keywords_each_fire() {
        for (src, kw) in [
            (
                "if {1} {\n    puts a\n}\nelseif {0} {\n    puts b\n}\n",
                "elseif",
            ),
            ("if {1}\nthen {\n    puts yes\n}\n", "then"),
            (
                "try {\n    error oops\n}\nfinally {\n    puts cleanup\n}\n",
                "finally",
            ),
            (
                "try {\n    error oops\n}\non error {msg opts} {\n    puts $msg\n}\n",
                "on",
            ),
            (
                "try {\n    error oops\n}\ntrap {} e {\n    puts caught\n}\n",
                "trap",
            ),
        ] {
            let ds = of_code(src, D, "W125");
            assert_eq!(ds.len(), 1, "for {kw}");
            assert!(ds[0].0.contains(kw), "message should name {kw}");
        }
    }

    #[test]
    fn same_line_forms_clean() {
        assert!(!fires(
            "if {1} {\n    puts yes\n} else {\n    puts no\n}\n",
            D,
            "W125"
        ));
        assert!(!fires(
            "if {1} {\n    puts a\n} elseif {0} {\n    puts b\n}\n",
            D,
            "W125"
        ));
        assert!(!fires(
            "try {\n    error oops\n} finally {\n    puts cleanup\n}\n",
            D,
            "W125"
        ));
    }

    #[test]
    fn user_proc_named_keyword_suppresses() {
        assert!(!fires("proc else {x} { puts $x }\nelse hello\n", D, "W125"));
        assert!(!fires(
            "proc finally {body} { uplevel 1 $body }\nfinally { puts done }\n",
            D,
            "W125"
        ));
    }

    #[test]
    fn multiple_orphaned_keywords() {
        let src = "if {1} {\n    puts a\n}\nelseif {0} {\n    puts b\n}\nelse {\n    puts c\n}\n";
        let ds = of_code(src, D, "W125");
        assert_eq!(ds.len(), 2);
        let kws: std::collections::HashSet<_> = ds
            .iter()
            .map(|(m, _, _)| m.split('"').nth(1).unwrap_or("?").to_string())
            .collect();
        assert_eq!(
            kws,
            ["elseif".to_string(), "else".to_string()]
                .into_iter()
                .collect()
        );
    }
}
