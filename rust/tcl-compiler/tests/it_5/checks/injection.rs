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

//! Security checks: code injection, pipelines, paths, sourced variables, script concatenation, credentials and `ReDoS`.

// W101 — eval with substituted/concatenated arguments (injection).
//
// tclsh ground truth: `eval "puts $x"` builds a string then re-parses it as a
// script, so a hostile `$x` (`x={; exec rm -rf /}`) executes. `eval [list …]`
// and other canonical list-returning idioms cannot inject (the list is a single
// already-quoted argument). Heuristic for the "safe idiom" recognition.
mod eval_injection {
    use super::super::*;

    #[test]
    fn eval_with_variable_fires() {
        let ds = of_code("eval \"puts $x\"", D, "W101");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("injection"));
    }

    #[test]
    fn eval_braced_scripts_clean() {
        assert!(!fires("eval {puts hello}", D, "W101"));
        assert!(!fires("eval {set x 1} {puts $x}", D, "W101"));
    }

    #[test]
    fn eval_command_subst_fires() {
        assert_eq!(count("eval [build_cmd]", D, "W101"), 1);
    }

    #[test]
    fn eval_literal_no_substitution_clean() {
        assert!(!fires("eval puts hello", D, "W101"));
    }

    #[test]
    fn eval_list_returning_idioms_clean() {
        // list / linsert / split are canonical safe quoting.
        for src in [
            "eval [list set $varname $value]",
            "eval [linsert $cmdlist 0 extraarg]",
            "eval [split $line :]",
        ] {
            assert!(!fires(src, D, "W101"), "unexpected W101 for {src:?}");
        }
    }

    #[test]
    fn eval_concat_still_warns() {
        // concat strips quoting → still an injection vector.
        assert_eq!(count("eval [concat $script $args]", D, "W101"), 1);
    }

    #[test]
    fn eval_offers_list_quickfix() {
        // `eval "cmd $x"` → safe `eval [list cmd $x]` form. Verified vs tclsh:
        // the list form passes $x as one argument and is never re-parsed.
        let ds = of_code("eval \"process $x\"", D, "W101");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2, vec!["[list process $x]".to_string()]);
    }
}

// W102 — subst on variable input (injection).
//
// tclsh: `subst $template` performs command/variable/backslash substitution on
// the template's value; a `[exec …]` inside it runs. `-nocommands -novariables`
// disables both substitution classes → safe.
mod subst_injection {
    use super::super::*;

    #[test]
    fn subst_variable_fires_and_suggests_nocommands() {
        let ds = of_code("subst $template", D, "W102");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("injection"));
        assert!(ds[0].0.contains("-nocommands"));
    }

    #[test]
    fn subst_braced_or_literal_clean() {
        assert!(!fires("subst {hello $name}", D, "W102"));
        assert!(!fires("subst \"hello world\"", D, "W102"));
    }

    #[test]
    fn subst_with_flags_still_fires() {
        assert_eq!(count("subst -nobackslashes $template", D, "W102"), 1);
    }

    #[test]
    fn subst_nocommands_only_still_fires_and_suggests_novariables() {
        // `-nocommands` leaves `$var` expansion → still warn, suggest -novariables.
        let ds = of_code("subst -nocommands $template", D, "W102");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("-novariables"));
    }

    #[test]
    fn subst_nocommands_novariables_clean() {
        // Only backslash substitution left → safe.
        assert!(!fires(
            "subst -nocommands -novariables $template",
            D,
            "W102"
        ));
    }
}

// W103 — open with pipeline or variable argument.
//
// tclsh: `open "|cmd"` runs `cmd` as a subprocess. A literal pipeline is a
// known-shape hint; a pipeline with substitution or a bare variable (whose
// value might begin with `|`) is a warning.
mod open_pipeline {
    use super::super::*;

    #[test]
    fn literal_pipeline_is_hint() {
        let ds = of_code("open \"|ls -la\"", D, "W103");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Hint);
    }

    #[test]
    fn pipeline_with_variable_is_warning() {
        let ds = of_code("open \"|$cmd\"", D, "W103");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Warning);
    }

    #[test]
    fn variable_arg_is_warning() {
        let ds = of_code("open $filename", D, "W103");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Warning);
    }

    #[test]
    fn literal_and_braced_file_clean() {
        assert!(!fires("open \"data.txt\"", D, "W103"));
        assert!(!fires("open {data.txt}", D, "W103"));
    }
}

// W201 — manual path concatenation (taint-pipeline style heuristic).
//
// `set path "$dir/file.txt"` builds a path by string concat; `[file join]` is
// the portable form. No runtime analogue (it is valid Tcl) — pure portability
// style. URLs (`scheme://…`) are explicitly excluded ([file join] would emit
// native separators).
mod path_concatenation {
    use super::super::*;

    #[test]
    fn path_with_var_fires() {
        // W201 runs in the taint pass (run_all_checks), not the analyser pass.
        // `dir` is bound first: tclsh 8.6.18/9.0.4 raise `can't read "dir"`
        // otherwise and the assignment never runs.
        let ds = taint_of_code("set dir [pwd]\nset path \"$dir/file.txt\"", D, "W201");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("file join"));
    }

    #[test]
    fn literal_path_and_non_path_clean() {
        assert!(!fires("set path \"/etc/config\"", D, "W201"));
        assert!(!fires("set x 42", D, "W201"));
    }

    #[test]
    fn genuine_two_var_path_fires() {
        assert_eq!(
            count(
                "set dir [pwd]\nset file [info script]\nset p \"$dir/$file\"",
                D,
                "W201"
            ),
            1
        );
    }

    #[test]
    fn file_join_transform_does_not_masquerade_as_normalisation() {
        // End-to-end registry transform coverage: `file join`
        // stamps PATH_JOINED on a tainted result, but it does not collapse
        // traversal or otherwise prove the preceding manual concatenation
        // safe. W201 must therefore still report the original assignment.
        let source = "set fd [open /etc/hostname]\nset raw [read $fd]\nset p \"/base/$raw\"\nset p [file join /base $raw]\n";
        assert_eq!(
            taint_of_code(source, D, "W201").len(),
            1,
            "PATH_JOINED must not suppress the W201 portability hint"
        );
    }

    #[test]
    fn url_scheme_not_path_concat() {
        // A URL is not a filesystem path → no W201.
        assert!(!fires("set u http://example.com/$page", D, "W201"));
        assert!(!fires(
            "set url \"${proto}://$host:$port/$path\"",
            D,
            "W201"
        ));
    }
}

// W300 — source with a variable path (code-execution vector).
//
// tclsh: `source $p` executes the file named by `$p` as Tcl. A literal path is
// fixed and safe; a variable / option-prefixed variable is a warning.
mod source_variable {
    use super::super::*;

    #[test]
    fn source_variable_is_warning() {
        let ds = of_code("source $script_path", D, "W300");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Warning);
    }

    #[test]
    fn source_literal_and_braced_clean() {
        assert!(!fires("source \"lib/utils.tcl\"", D, "W300"));
        assert!(!fires("source {lib/utils.tcl}", D, "W300"));
    }

    #[test]
    fn source_encoding_flag_variable_fires() {
        assert_eq!(count("source -encoding utf-8 $path", D, "W300"), 1);
    }
}

// W301 — uplevel with unbraced / multi-arg / interpolated scripts.
//
// tclsh: `uplevel 1 "set x $y"` substitutes `$y` in the *current* frame then
// evaluates the result in the caller's → double substitution. `[list …]` and
// other list-returning commands are the canonical safe form; a *single pure
// variable* (`$body` / `${body}` / `$cmds(init)`) is also safe (the value is
// the script — verified vs tclsh) and cannot be braced.
mod uplevel_injection {
    use super::super::*;

    #[test]
    fn unbraced_with_variable_fires() {
        assert_eq!(count("uplevel 1 \"set x $y\"", D, "W301"), 1);
        assert_eq!(count("uplevel \"set x $y\"", D, "W301"), 1);
    }

    #[test]
    fn braced_script_clean() {
        assert!(!fires("uplevel 1 {set x 1}", D, "W301"));
    }

    #[test]
    fn multiple_args_concatenate_and_fire() {
        let ds = of_code("uplevel 1 set x $y", D, "W301");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("concatenate"));
    }

    #[test]
    fn list_returning_idioms_clean() {
        for src in [
            "uplevel 1 [list set $varname $value]",
            "set ns [uplevel 1 [list ::namespace current]]",
            "uplevel 1 [lsort $cmdlist]",
        ] {
            assert!(!fires(src, D, "W301"), "unexpected W301 for {src:?}");
        }
    }

    #[test]
    fn concat_still_warns() {
        assert_eq!(count("uplevel 1 [concat $script $args]", D, "W301"), 1);
    }

    #[test]
    fn bare_single_variable_is_safe_idiom() {
        // `uplevel 1 $body` / `${body}` / `$cmds(init)` — single pure reference,
        // value IS the script (verified vs tclsh); cannot be braced → no W301.
        for src in [
            "proc f {body} { uplevel 1 $body }",
            "proc f {body} { uplevel 1 ${body} }",
            "proc f {} { uplevel 1 $cmds(init) }",
            "proc f {body} { uplevel $body }",
        ] {
            assert!(!fires(src, D, "W301"), "unexpected W301 for {src:?}");
        }
    }

    #[test]
    fn quoted_interpolation_and_concatenation_still_warn() {
        // `"set y $x"` (quoted interp), `$x$y` / `$x.foo` (double substitution),
        // and multi-arg concat are NOT the safe single-var idiom → warn.
        for src in [
            "proc f {x} { uplevel 1 \"set y $x\" }",
            "proc f {a b} { uplevel 1 set $a $b }",
            "proc f {x y} { uplevel 1 $x$y }",
            "proc f {x} { uplevel 1 $x.foo }",
        ] {
            assert_eq!(count(src, D, "W301"), 1, "expected W301 for {src:?}");
        }
    }
}

// W303 over backreferences combined with nested or alternative repetition.
mod redos {
    use super::super::*;

    #[test]
    fn nested_quantifier_fires() {
        let ds = of_code("regexp {^((a+)+)\\1$} $input", D, "W303");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("backtracking"));
    }

    #[test]
    fn nested_star_and_overlapping_alternation_fire() {
        assert_eq!(count("regexp {^((a*)*)\\1$} $input", D, "W303"), 1);
        assert_eq!(count("regexp {^((a|aa)+)\\1$} $input", D, "W303"), 1);
    }

    #[test]
    fn safe_patterns_clean() {
        assert!(!fires("regexp {^\\d+$} $input", D, "W303"));
        assert!(!fires("regexp {a+b+} $input", D, "W303"));
    }

    #[test]
    fn options_do_not_hide_the_pattern() {
        assert_eq!(count("regexp -nocase {^((a+)+)\\1$} $input", D, "W303"), 1);
        assert_eq!(
            count("regsub {^((a+)+)\\1$} $input replacement result", D, "W303"),
            1
        );
        assert_eq!(
            count(
                "regsub -all -nocase {^((a*)*)\\1$} $input X result",
                D,
                "W303"
            ),
            1
        );
    }

    #[test]
    fn safe_regsub_clean() {
        assert!(!fires("regsub {\\d+} $input X result", D, "W303"));
    }

    #[test]
    fn switch_regexp_arms_checked_glob_not() {
        assert_eq!(
            count(
                "switch -regexp $x { {^((a+)+)\\1$} {puts bad} {^ok} {puts ok} }",
                D,
                "W303"
            ),
            1
        );
        assert!(!fires(
            "switch -regexp $x { {^hello} {puts hi} {^world} {puts world} }",
            D,
            "W303"
        ));
        // -glob patterns are not regexes → never ReDoS-checked.
        assert!(!fires(
            "switch -glob $x { {^((a+)+)\\1$} {puts bad} }",
            D,
            "W303"
        ));
    }
}

// W309 — eval/uplevel with [subst] (double substitution).
//
// `eval [subst $template]` substitutes the template, then evaluates the result
// as a script → two rounds of substitution, an injection vector → Error.
mod eval_subst_double_substitution {
    use super::super::*;

    #[test]
    fn eval_subst_variable_is_error() {
        let ds = of_code("eval [subst $template]", D, "W309");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Error);
        assert!(ds[0].0.to_lowercase().contains("double substitution"));
    }

    #[test]
    fn eval_subst_braced_is_error() {
        let ds = of_code("eval [subst {set x [expr {$a + $b}]}]", D, "W309");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Error);
    }

    #[test]
    fn uplevel_subst_is_error() {
        let ds = of_code("uplevel 1 [subst $template]", D, "W309");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Error);
    }

    #[test]
    fn eval_without_subst_clean() {
        assert!(!fires("eval {puts hello}", D, "W309"));
        assert!(!fires("eval [build_cmd]", D, "W309"));
    }
}

// W310 — hardcoded credentials.
//
// Heuristic: a literal value after a credential-bearing flag (`-password`,
// `-token`) or sensitive header (`Authorization`) is a likely secret. A
// variable value is not hardcoded → clean.
mod hardcoded_credentials {
    use super::super::*;

    #[test]
    fn password_literal_warns_variable_clean() {
        let ds = of_code("http::geturl $url -password \"s3cret\"", D, "W310");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Warning);
        assert!(ds[0].0.to_lowercase().contains("credential"));
        assert!(!fires("http::geturl $url -password $pw", D, "W310"));
    }

    #[test]
    fn token_literal_warns() {
        assert_eq!(count("some_command -token \"abc123def456\"", D, "W310"), 1);
    }

    #[test]
    fn sensitive_header_literal_warns_under_irules() {
        let ds = of_code(
            "HTTP::header insert Authorization \"Bearer hardcoded\"",
            IR,
            "W310",
        );
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("authorization"));
        assert!(!fires(
            "HTTP::header insert Authorization $token",
            IR,
            "W310"
        ));
        // Non-sensitive header is fine.
        assert!(!fires(
            "HTTP::header insert Content-Type \"text/html\"",
            IR,
            "W310"
        ));
    }
}

// W312 — interp eval / interp invokehidden injection.
//
// Same shape as W301 for child interpreters: an unbraced / multi-arg script
// risks injection; `[list …]` is the safe canonical form.
mod interp_eval_injection {
    use super::super::*;

    #[test]
    fn unbraced_with_variable_warns() {
        let ds = of_code("interp eval $child \"set x $y\"", D, "W312");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Warning);
    }

    #[test]
    fn braced_and_list_clean() {
        // The words are bound first: tclsh errors on reading an unset variable.
        assert!(!fires(
            "set child [gets stdin]\ninterp eval $child {set x 1}",
            D,
            "W312"
        ));
        assert!(!fires(
            "set child [gets stdin]\nset varname [gets stdin]\nset value [gets stdin]\n\
             interp eval $child [list set $varname $value]",
            D,
            "W312"
        ));
        assert!(!fires(
            "set child [gets stdin]\ninterp aliases $child",
            D,
            "W312"
        ));
    }

    #[test]
    fn multiple_args_concatenate_and_warn() {
        let ds = of_code("interp eval $child set x $y", D, "W312");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("concatenate"));
    }

    #[test]
    fn invokehidden_unbraced_warns() {
        assert_eq!(
            count("interp invokehidden $child \"source $path\"", D, "W312"),
            1
        );
    }
}

// W313 — destructive file operations with a variable path (taint-aware).
//
// `file delete/rename/mkdir $path` risks path traversal. A literal path / a
// non-destructive op (`file exists`/`file join`) is clean. (Bounds-check
// suppression lives in the taint pipeline; the basic firing surface is here.)
mod destructive_file_ops {
    use super::super::*;

    #[test]
    fn variable_path_destructive_ops_fire() {
        // W313 runs in the taint pass (run_all_checks), not the analyser pass.
        let ds = taint_of_code("set path /tmp/x\nfile delete $path", D, "W313");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("path-traversal"));
        assert_eq!(
            count("set src /tmp/x\nfile rename $src /tmp/dest", D, "W313"),
            1
        );
        assert_eq!(count("set dir /tmp/x\nfile mkdir $dir", D, "W313"), 1);
        assert_eq!(
            count("set path /tmp/x\nfile delete -force $path", D, "W313"),
            1
        );
    }

    #[test]
    fn literal_path_and_non_destructive_clean() {
        assert!(!fires("file delete /tmp/myfile.txt", D, "W313"));
        assert!(!fires("set path /tmp/x\nfile exists $path", D, "W313"));
        assert!(!fires("set base /tmp\nfile join $base subdir", D, "W313"));
    }
}

// The `Tcl_ConcatObj` eval family.
//
// `eval`, `uplevel`, `namespace eval`, `namespace inscope`, and `interp eval`
// evaluate the *concatenation* of every trailing script word, so analysing
// only the first word invents diagnostics about a script that never runs.
//
// Every expectation here was pinned against tclsh8.6.14 and tclsh9.0.4 (both
// agree on every shape); the transcript is quoted per test where the answer
// is not obvious.
mod script_concatenation {
    use super::super::*;

    /// FP — `eval set l2 hello` really runs `set l2 hello`, so neither the
    /// wrong-#-args error nor the read-before-set warning is real.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `eval set l2 hello; puts $l2` → `hello`.
    #[test]
    fn eval_fp_multiword_script_is_analysed_as_the_concatenation_1051() {
        let src = "eval set l2 hello\nputs $l2\n";
        assert!(!fires(src, D, "E002"), "{:?}", codes(src, D));
        assert!(!fires(src, D, "W210"), "{:?}", codes(src, D));
    }

    /// FP — a braced word contributes its *contents* to the join, so
    /// `eval {set l2} hello` is the same script as `eval set l2 hello`.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `concat {set x} {5}` → `set x 5`; the outer
    /// braces are gone by the time `Tcl_ConcatObj` sees the word.
    #[test]
    fn eval_fp_braced_word_contributes_its_contents_to_the_join_1051() {
        let src = "eval {set l2} hello\nputs $l2\n";
        assert!(!fires(src, D, "E002"), "{:?}", codes(src, D));
        assert!(!fires(src, D, "W210"), "{:?}", codes(src, D));
    }

    /// TP — a genuinely short script still draws the arity error.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `eval {set}` →
    /// `wrong # args: should be "set varName ?newValue?"`.
    #[test]
    fn eval_tp_short_script_still_draws_e002_1051() {
        assert!(fires("eval {set}\n", D, "E002"));
        assert!(fires("eval set\n", D, "E002"));
    }

    /// TN — a dynamic word makes the real script unknowable (substitution
    /// runs before concatenation), so the call is consumed without walking:
    /// no arity or definedness claim. W101 still fires — the injection risk
    /// is exactly what makes the script unknowable.
    #[test]
    fn eval_tn_dynamic_word_declines_without_claiming_anything_1051() {
        let src = "eval $cmd arg\n";
        assert!(!fires(src, D, "E002"), "{:?}", codes(src, D));
        assert!(!fires(src, D, "E003"), "{:?}", codes(src, D));
        assert!(fires(src, D, "W101"), "{:?}", codes(src, D));
    }

    /// TP — W101 keeps firing on the plain injection shape. Its gate now
    /// reads `SCRIPT_CONCATENATES_ARGS + TAINT_SINK` rather than inferring
    /// the concat shape from how `eval`'s arg roles happen to be spelled.
    #[test]
    fn eval_tp_w101_still_fires_on_a_user_input_script_1051() {
        assert!(fires("eval $userinput\n", D, "W101"));
        // `uplevel` stays W301's — the script runs in another frame, which is
        // a frame-escalation finding with its own message, not a second W101.
        assert!(!fires("uplevel 1 \"set x $y\"\n", D, "W101"));
    }

    /// FP — `uplevel` concatenates its post-level words the same way.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `proc p {} {uplevel 1 set x 5}; p; puts $x`
    /// → `5`. Written inside a proc because `uplevel 1` at top level errors
    /// `bad level "1"` — there is no caller frame to reach.
    #[test]
    fn uplevel_fp_multiword_script_draws_no_arity_error_1051() {
        let src = "proc p {} {\n    uplevel 1 set x 5\n}\np\nputs $x\n";
        assert!(!fires(src, D, "E002"), "{:?}", codes(src, D));
    }

    /// TN — `uplevel`'s script runs in the **caller's** frame, so a write it
    /// performs must not silence a read-before-set of the proc's own local.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `proc p {} {uplevel 1 set x 5; puts $x}; p`
    /// → `can't read "x": no such variable`.
    #[test]
    fn uplevel_tn_shifted_frame_write_does_not_define_a_local_1051() {
        let src = "proc p {} {\n    uplevel 1 set x 5\n    puts $x\n}\np\n";
        assert!(fires(src, D, "W210"), "{:?}", codes(src, D));
    }

    /// FP — `namespace eval` concatenates too: dropping everything past the
    /// first script word would be wrong.
    ///
    /// tclsh8.6.14 / tclsh9.0.4:
    /// `namespace eval ::n set l2 hello; puts $::n::l2` → `hello`.
    #[test]
    fn namespace_eval_fp_multiword_script_is_walked_as_one_1051() {
        let src = "namespace eval ::n set l2 hello\n";
        assert!(codes(src, D).is_empty(), "{:?}", codes(src, D));
        // …and the joined script is really walked: an over-long one still
        // reports. tclsh8.6.14 / tclsh9.0.4: `namespace eval ::n set a b c` →
        // `wrong # args: should be "set varName ?newValue?"`.
        assert!(fires("namespace eval ::n set a b c\n", D, "E003"));
    }

    /// FP — `namespace inscope`'s tail is appended as *list elements*, not
    /// space-joined, so the trailing words are arguments to the script's
    /// command. Analysing `{handler}` alone claimed a zero-argument call.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: the repro prints `ready`; and
    /// `namespace inscope :: {puts} {a b}` prints `a b` (one argument) where
    /// `namespace eval :: {puts} {a b}` errors
    /// `can not find channel named "a"`.
    #[test]
    fn namespace_inscope_fp_list_appended_tail_draws_nothing_1051() {
        let src = "namespace eval ::app { proc handler {tag} { puts $tag } }\n\
                   namespace inscope ::app {handler} ready\n";
        assert!(codes(src, D).is_empty(), "{:?}", codes(src, D));
    }

    /// TN — `interp eval`'s existing consume-without-walk behaviour is
    /// unchanged: the script runs in a child interpreter, so nothing it
    /// defines may leak into this one.
    ///
    /// tclsh8.6.14 / tclsh9.0.4:
    /// `interp create i; interp eval i {set y} 7; interp eval i {set y}` → `7`.
    #[test]
    fn interp_eval_tn_multiword_stays_isolated_1051() {
        let src = "interp create i\ninterp eval i {set y} 7\n";
        assert!(!fires(src, D, "E002"), "{:?}", codes(src, D));
        assert!(!fires(src, D, "W210"), "{:?}", codes(src, D));
    }

    /// FP — a dynamic tail declines the *join*, not the braced first word:
    /// `eval {script} $extra` still runs `script` as written (concatenation
    /// only appends after it), so the write it performs must stay recorded.
    #[test]
    fn eval_fp_braced_script_before_a_dynamic_tail_is_still_walked_1051() {
        let src = "set extra x\neval {set l2 5} $extra\nputs $l2\n";
        assert!(!fires(src, D, "W210"), "{:?}", codes(src, D));
    }

    /// FP — a `namespace eval` whose body word is dragged open by an
    /// unbalanced brace *inside* it (here a `${gre` fragment in a comment)
    /// turns into a multi-word call with an un-joinable tail. Declining the
    /// join must not discard the braced body word — it is the namespace's
    /// whole visible content, and dropping it removed every proc scope from
    /// the tree (the editors' variableContexts fixture regression).
    #[test]
    fn namespace_eval_fp_mangled_body_word_keeps_its_proc_scopes_1051() {
        let src = "namespace eval ::ns1 {\n    proc p1 {} {\n        scan \"1 2\" \"%d %d\" sx sy\n    }\n    proc t2 {greeting} {\n        # probe (``$gre``, ``${gre``, midword)\n    }\n}\nset \"weird}name\" 1\n";
        let mut analyser = tcl_compiler::analyser::Analyser::new();
        let result = analyser.analyse(src, D);
        let ns = result
            .global_scope
            .children
            .iter()
            .find(|c| c.name.contains("ns1"))
            .expect("namespace scope registered");
        assert_eq!(
            ns.children.len(),
            2,
            "proc scopes lost from the mangled namespace body"
        );
    }

    /// TN — `catch` is not in the family: its remaining words are result and
    /// options variable names, so nothing is joined into the script.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `catch {set z} m; puts $m` →
    /// `can't read "z": no such variable` — `m` is a variable name, not
    /// script text.
    #[test]
    fn catch_tn_is_not_in_the_concat_family_1051() {
        assert!(fires("catch {set} m\n", D, "E002"));
    }

    /// FP — a brace-quoted word is static *script text* even when it carries
    /// `$`: the braces blocked the outer substitution, so the concatenated
    /// script is knowable and `eval` resolves `$value` when it runs. Scanning
    /// the text for `$` alone wrongly declined the walk and kept the false
    /// read-before-set this family fix exists to remove.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `set value 5; eval {set l2} {$value};
    /// puts $l2` → `5`.
    #[test]
    fn eval_fp_braced_word_with_substitution_text_is_static_1051() {
        let src = "set value 5\neval {set l2} {$value}\nputs $l2\n";
        assert!(!fires(src, D, "E002"), "{:?}", codes(src, D));
        assert!(!fires(src, D, "W210"), "{:?}", codes(src, D));
    }

    /// FP — a quoted word without substitutions joins like any other; a
    /// space inside it separates words when the script re-parses, exactly as
    /// `Tcl_ConcatObj`'s join does.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `eval "set q" 7; puts $q` → `7`.
    #[test]
    fn eval_fp_quoted_word_joins_like_concat_1051() {
        let src = "eval \"set q\" 7\nputs $q\n";
        assert!(!fires(src, D, "E002"), "{:?}", codes(src, D));
        assert!(!fires(src, D, "W210"), "{:?}", codes(src, D));
    }

    /// The walked script is anchored at its *real source bytes*: dropped
    /// empty words and stripped delimiters shift a naive join left, so its
    /// spans would edit delimiters on rename. `eval {} foo` is the smallest
    /// such shape — the reconstructed `foo` must span exactly the written
    /// `foo`, not start one byte early on the empty word's closing brace.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `proc foo {} {}; eval {} foo` runs `foo`.
    #[test]
    fn eval_offsets_of_the_walked_script_land_on_real_source_bytes_1051() {
        let src = "proc foo {} {}\neval {} foo\n";
        let mut analyser = tcl_compiler::analyser::Analyser::new();
        let result = analyser.analyse(src, D);
        let expected = u32::try_from(src.rfind("foo").expect("fixture")).unwrap();
        let inv = result
            .command_invocations
            .iter()
            .find(|i| i.name == "foo")
            .expect("the eval'd call is recorded");
        assert_eq!(
            (inv.range.start(), inv.range.end()),
            (expected, expected + 3),
            "the walked call must span the written `foo`"
        );
    }

    /// TN — a backslash in a bare word decodes *before* the join, so the
    /// written bytes are not the script text and the walk declines without
    /// claiming anything (no invented arity error on what really runs).
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `eval set\ x 5; puts $x` → `5` (the decoded
    /// `set x` re-splits when the joined script is parsed).
    #[test]
    fn eval_tn_a_backslash_word_declines_without_claiming_anything_1051() {
        let src = "eval set\\ x 5\n";
        assert!(!fires(src, D, "E002"), "{:?}", codes(src, D));
        assert!(!fires(src, D, "E003"), "{:?}", codes(src, D));
    }
}
