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

//! Best-practice / security diagnostic-check suite.
//!
//! Where `analyser.rs` exercises the
//! *semantic-model* surface plus the arity / read-before-set core, this suite
//! drives the **compiler-checks** families: the style / injection / taint /
//! IP-validation / iRules diagnostics (`W100`–`W126`, `W200`–`W313`,
//! `IRULE2002`/`IRULE2003`/`IRULE3102`/`IRULE1002`, …). Each assertion
//! that a diagnostic *fires* (or stays silent) checks a code firing /
//! not firing through the same merged pipeline.
//!
//! ## Diagnostic surface
//!
//! [`codes`]/[`fires`]/[`count`] are copied verbatim from the canonical
//! `analyser.rs` template (itself copied from the in-crate harness
//! `src/analyser/diagnostics/fp/mod.rs`): they merge the analyser pass with the
//! `run_all_checks` compiler-checks pass and drop optimisation codes, exactly
//! mirroring the user-facing `tcl diag` surface.
//! [`adiags`] returns `(code, message, severity, fixes)` from the analyser pass
//! only, for the tests that pin a message substring, a severity, or a fix's
//! `new_text` (every code those tests inspect is owned by the analyser pass).
//!
//! ## C-Tcl ground truth
//!
//! Diagnostics that mirror a real tclsh error are pinned to ground truth from
//! `scripts/dev/tclsh_check.sh` (tclsh8.6 + tclsh9.0 on PATH). Verified while
//! authoring this file:
//! - `W213`: `unset x` on a never-set var → `can't unset "x": no such variable`.
//! - `W212`: `set $x 1` sets the var *named by* `$x` — almost never intended.
//! - `W101`: `eval "puts $x"` re-parses the built string → injection vector.
//! - `W102`: `subst $template` executes `[cmd]`/`$var` in the value.
//! - `W124`: `192.168.1.256` is not parseable as an IP octet (`>255`).
//! - `W100`: `expr $x + 1` double-substitutes `$x` then re-parses — the
//!   canonical "always brace your expr" footgun (`expr {$x + 1}` is safe).
//! - `W210` (the catch-body case): `if {![catch {set x 1}]} {puts $x}` prints
//!   `1` in tclsh — the catch *body* defines `x`, so the read is safe (see the
//!   `catch_body_defs_no_false_w210` module — the former false positive is now
//!   fixed; the recovery scans catch bodies and branch conditions).
//!
//! Pure static heuristics with no direct runtime analogue (path-concatenation
//! style `W201`, string-vs-list `W104`, `ReDoS` `W303`, credential-scan `W310`,
//! …) are noted as such inline.
//!
//! ## Known divergences
//!
//! Several expectations target precision the analyser +
//! `run_all_checks` surface does not (yet) reproduce. Each
//! is adapted to the **actual verdict** with an explanatory comment (and,
//! where the analyser is provably wrong against tclsh, called out separately).
//! The headline divergences:
//! - `W104` fires on pure separators (`append msg ", "`) and usage notation
//!   (`" ?opt?..."`) — the heuristic is coarse and does not suppress these.
//! - `W307` fires on `$cmd` iterating a `foreach` over *known* commands
//!   (no "known-command-list" provenance on this surface to suppress it).
//! - `W210` no longer false-fires when a `catch {set x …}` body defines the
//!   variable read in the surrounding `if` body (fixed: catch-body + branch-
//!   condition out-var recovery).
//! - `W210` now fires when a proc `unset`s (and never sets) a global later read
//!   at top level, where tclsh errors `can't read` (fixed: `unset` excluded
//!   from the proc global-write set).
//! - `W308` is the *`TclOO` unknown-method* code here; it does not cover the
//!   `subst`-without-`-nocommands` hint (those cases are out-of-surface; the
//!   `TclOO` `W308` cases are covered).
//! - `W122` never fires on the single-document surface — the SSA-traced `W124`
//!   supersedes it everywhere — so the IP tests assert "W122 or W124".
//! - Per-event cross-`when` scoping, taint-pipeline-only `W201`/`W313`
//!   internals, and a few message-format / fix-shape assertions are pinned to
//!   the Rust shape (dead-store reported as `W220` "never read" rather than
//!   `W211` "set but never used" in structural-body cases, etc.).

use tcl_compiler::analyser::{Analyser, Severity};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_registry::registry_for_dialect;

/// Default dialect for reproducers that are not dialect-sensitive.
const D: &str = "tcl8.6";
/// iRules dialect for the F5-specific checks.
const IR: &str = "f5-irules";

/// Every diagnostic code the full pipeline surfaces for `src` under `dialect`,
/// mirroring the user-facing `tcl diag` path: the analyser pass plus the
/// `run_all_checks` compiler-checks pass (shimmer / taint / dead-store), with
/// optimisation codes excluded exactly as `diag` excludes them.
///
/// Copied verbatim from `analyser.rs` (← `src/analyser/diagnostics/fp/mod.rs::codes`).
fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

/// True if `code` appears anywhere in the full diagnostic set for `src`.
fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes(src, dialect).iter().any(|c| c == code)
}

/// Count of how many times `code` appears in the merged diagnostic set.
fn count(src: &str, dialect: &str, code: &str) -> usize {
    codes(src, dialect)
        .iter()
        .filter(|c| c.as_str() == code)
        .count()
}

/// Full `(code, message, severity, fix-new-texts)` tuples from the *analyser
/// pass only* — used when a test pins a message substring, a severity,
/// or a fix's `new_text`. Every code those tests inspect (W100–W126,
/// W200–W313, IRULE*) is owned by the analyser pass.
fn adiags(src: &str, dialect: &str) -> Vec<(String, String, Severity, Vec<String>)> {
    Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| {
            (
                d.code.to_string(),
                d.message.clone(),
                d.severity,
                d.fixes.iter().map(|f| f.new_text.clone()).collect(),
            )
        })
        .collect()
}

/// `(message, severity, fixes)` for every diagnostic of `code` under `dialect`.
fn of_code(src: &str, dialect: &str, code: &str) -> Vec<(String, Severity, Vec<String>)> {
    adiags(src, dialect)
        .into_iter()
        .filter(|(c, _, _, _)| c == code)
        .map(|(_, m, s, f)| (m, s, f))
        .collect()
}

/// `(message, severity)` for every diagnostic of `code` emitted by the
/// **`run_all_checks` taint / compiler-checks pass** (W201 path-concat, W313
/// destructive-file, and the other taint-pipeline codes do not flow through the
/// analyser pass, so [`of_code`] cannot see them).
fn taint_of_code(src: &str, dialect: &str, code: &str) -> Vec<(String, Severity)> {
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    run_all_checks(&cu, registry, dialect_opt)
        .into_iter()
        .filter(|d| !d.code.is_optimisation() && d.code.to_string() == code)
        .map(|d| (d.message, d.severity))
        .collect()
}

// ===========================================================================
// W100 — unbraced expr argument to expr/if/while/for.
//
// tclsh ground truth: `expr $x + 1` double-substitutes `$x` and re-parses the
// result as an expression — the canonical "always brace your expr" footgun
// (`expr {$x + 1}` is the safe form). A `$`/`[` substitution makes it dangerous
// → Error severity; a quoted literal with no substitution is style-only → Warning.
// ===========================================================================
mod unbraced_expr {
    use super::*;

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

// ===========================================================================
// W101 — eval with substituted/concatenated arguments (injection).
//
// tclsh ground truth: `eval "puts $x"` builds a string then re-parses it as a
// script, so a hostile `$x` (`x={; exec rm -rf /}`) executes. `eval [list …]`
// and other canonical list-returning idioms cannot inject (the list is a single
// already-quoted argument). Heuristic for the "safe idiom" recognition.
// ===========================================================================
mod eval_injection {
    use super::*;

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

// ===========================================================================
// W102 — subst on variable input (injection).
//
// tclsh: `subst $template` performs command/variable/backslash substitution on
// the template's value; a `[exec …]` inside it runs. `-nocommands -novariables`
// disables both substitution classes → safe.
// ===========================================================================
mod subst_injection {
    use super::*;

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

// ===========================================================================
// W103 — open with pipeline or variable argument.
//
// tclsh: `open "|cmd"` runs `cmd` as a subprocess. A literal pipeline is a
// known-shape hint; a pipeline with substitution or a bare variable (whose
// value might begin with `|`) is a warning.
// ===========================================================================
mod open_pipeline {
    use super::*;

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

// ===========================================================================
// W104 — string/list confusion (append with space-prefixed values).
//
// Pure style heuristic (no runtime analogue): `append items " item"` builds a
// space-separated string that looks like list construction; `lappend` is the
// list-safe form.
//
// The heuristic is coarse: it does not suppress pure separators
// (`", "` / `": "`) or usage notation (`" ?opt?..."`, `" <value>"`) — W104
// fires on those too. Each case below is pinned to the actual verdict.
// ===========================================================================
mod string_list_confusion {
    use super::*;

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

// ===========================================================================
// W110 — ==/!= used for string comparison in an expr.
//
// tclsh: `expr {$x == "foo"}` compares numerically-then-stringwise; `eq`/`ne`
// are the explicit string operators. Fires only when one operand is a string
// literal (two variables might both be ints → no warning).
// ===========================================================================
mod string_compare_in_expr {
    use super::*;

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
    fn numeric_and_boolean_string_literals_warn() {
        // The user wrote a quoted literal → still string-shaped.
        assert_eq!(count("if {$x == \"42\"} {puts yes}", D, "W110"), 1);
        assert_eq!(count("if {$x == \"true\"} {puts yes}", D, "W110"), 1);
    }

    #[test]
    fn negated_string_compare_warns() {
        // W110 fires through the unary negation wrapping a string ==.
        assert_eq!(count("if {!($x == \"foo\")} {puts no}", D, "W110"), 1);
    }

    #[test]
    fn mixed_ops_fire_without_blanket_fix() {
        // One `==` has no string literal → the blanket rewrite is unsafe, so no fix.
        let ds = of_code("if {$a == $b || $x == \"foo\"} {puts y}", D, "W110");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2.len(), 0);
    }
}

// ===========================================================================
// W201 — manual path concatenation (taint-pipeline style heuristic).
//
// `set path "$dir/file.txt"` builds a path by string concat; `[file join]` is
// the portable form. No runtime analogue (it is valid Tcl) — pure portability
// style. URLs (`scheme://…`) are explicitly excluded ([file join] would emit
// native separators).
// ===========================================================================
mod path_concatenation {
    use super::*;

    #[test]
    fn path_with_var_fires() {
        // W201 runs in the taint pass (run_all_checks), not the analyser pass.
        let ds = taint_of_code("set path \"$dir/file.txt\"", D, "W201");
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
        assert_eq!(count("set p \"$dir/$file\"", D, "W201"), 1);
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

// ===========================================================================
// W300 — source with a variable path (code-execution vector).
//
// tclsh: `source $p` executes the file named by `$p` as Tcl. A literal path is
// fixed and safe; a variable / option-prefixed variable is a warning.
// ===========================================================================
mod source_variable {
    use super::*;

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

// ===========================================================================
// W301 — uplevel with unbraced / multi-arg / interpolated scripts.
//
// tclsh: `uplevel 1 "set x $y"` substitutes `$y` in the *current* frame then
// evaluates the result in the caller's → double substitution. `[list …]` and
// other list-returning commands are the canonical safe form; a *single pure
// variable* (`$body` / `${body}` / `$cmds(init)`) is also safe (the value is
// the script — verified vs tclsh) and cannot be braced.
// ===========================================================================
mod uplevel_injection {
    use super::*;

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

// ===========================================================================
// W302 — catch without a result variable (silently swallows errors).
//
// tclsh ground truth: the destructive "fire-and-forget" commands below all
// error on a missing target (`after cancel`, `file delete`, `close`, `unset`,
// `dict unset`, `array unset`, `namespace delete`, `rename foo ""`), so
// `catch {…}` with no result var is the documented "do if possible" idiom for
// them — capturing the rc would just yield a discarded value. Constructive
// siblings (`file copy`, `interp create`, `dict set`, `chan configure`,
// `after <ms> <script>`) genuinely shouldn't swallow errors → still fire.
// ===========================================================================
mod catch_ignore {
    use super::*;

    #[test]
    fn catch_no_result_is_hint() {
        let ds = of_code("catch {error oops}", D, "W302");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Hint);
    }

    #[test]
    fn catch_with_result_clean() {
        assert!(!fires("catch {error oops} result", D, "W302"));
        assert!(!fires("catch {error oops} result opts", D, "W302"));
    }

    #[test]
    fn fire_and_forget_destructive_commands_suppressed() {
        // Each errors on a missing target in tclsh 9.0 → catch-ignore is the idiom.
        for src in [
            "catch {after cancel $h}",
            "catch {file delete $f}",
            "catch {close $fh}",
            "catch {unset var}",
            "catch {chan close $h}",
            "catch {interp delete slave}",
            "catch {::close $h}", // bare-name normalised
            "catch {dict unset d k}",
            "catch {array unset a *}",
            "catch {namespace delete ::ns}",
            "catch {namespace forget ::pkg::*}",
            "catch {rename foo \"\"}",
        ] {
            assert!(!fires(src, D, "W302"), "unexpected W302 for {src:?}");
        }
    }

    #[test]
    fn genuine_and_constructive_bodies_still_fire() {
        // User proc body / multi-statement / loadable / constructive ensemble
        // subcommands genuinely shouldn't swallow errors.
        for src in [
            "catch {parse_user_input $x}",
            "catch {parse_x; parse_y}",
            "catch {package require Tk}",
            "catch {chan configure $h -buffering none}",
            "catch {file copy a b}",
            "catch {after 1000 puts hi}",
            "catch {interp create slave}",
            "catch {namespace eval ::ns {}}",
            "catch {dict set d k v}",
        ] {
            assert_eq!(count(src, D, "W302"), 1, "expected W302 for {src:?}");
        }
    }
}

// ===========================================================================
// W303 — ReDoS (catastrophic-backtracking regex shapes).
//
// Pure static heuristic over the pattern literal: nested quantifiers
// `(a+)+`/`(a*)*` and overlapping alternation `(a|a)+`. `switch -glob` patterns
// are NOT regexes → never checked.
// ===========================================================================
mod redos {
    use super::*;

    #[test]
    fn nested_quantifier_fires() {
        let ds = of_code("regexp {(a+)+} $input", D, "W303");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("backtracking"));
    }

    #[test]
    fn nested_star_and_overlapping_alternation_fire() {
        assert_eq!(count("regexp {(a*)*} $input", D, "W303"), 1);
        assert_eq!(count("regexp {(a|a)+} $input", D, "W303"), 1);
    }

    #[test]
    fn safe_patterns_clean() {
        assert!(!fires("regexp {^\\d+$} $input", D, "W303"));
        assert!(!fires("regexp {a+b+} $input", D, "W303"));
    }

    #[test]
    fn options_do_not_hide_the_pattern() {
        assert_eq!(count("regexp -nocase {(a+)+} $input", D, "W303"), 1);
        assert_eq!(
            count("regsub {(a+)+} $input replacement result", D, "W303"),
            1
        );
        assert_eq!(
            count("regsub -all -nocase {(a*)*} $input X result", D, "W303"),
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
                "switch -regexp $x { {(a+)+} {puts bad} {^ok} {puts ok} }",
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
        assert!(!fires("switch -glob $x { (a+)+ {puts bad} }", D, "W303"));
    }
}

// ===========================================================================
// W304 — missing option terminator (--) before a substituted value.
//
// tclsh: option-bearing commands (`regexp`, `regsub`, `exec`, `file delete`,
// `unset`, `load`, …) parse a leading `-` as an option, so `regexp $pat …` with
// `pat="-nocase"` is mis-parsed. Commands that don't support `--`
// (`subst`, `string match`, `lsearch`) are not checked.
// ===========================================================================
mod missing_option_terminator {
    use super::*;

    #[test]
    fn regexp_pattern_variable_fires_with_fix() {
        let ds = of_code("regexp $pattern $text", D, "W304");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("--"));
        assert_eq!(ds[0].2.len(), 1);
        assert!(ds[0].2[0].starts_with("-- "));
    }

    #[test]
    fn structurally_safe_pattern_suppressed() {
        // Pattern starts with `(` — cannot be confused with an option.
        assert!(!fires("regexp {(a+)+$} $text", D, "W304"));
    }

    #[test]
    fn literal_dash_pattern_warns() {
        // Pattern starts with `-` → could be read as `-nocase` etc.
        assert_eq!(count("regexp {-[0-9]+} $text", D, "W304"), 1);
    }

    #[test]
    fn with_terminator_clean() {
        assert!(!fires("regexp -- $pattern $text", D, "W304"));
        assert!(!fires("exec -- $cmd", D, "W304"));
        assert!(!fires("file delete -- $path", D, "W304"));
        assert!(!fires("unset -- $varname", D, "W304"));
    }

    #[test]
    fn option_bearing_commands_fire() {
        for src in [
            "regsub $pattern $text X out",
            "exec $cmd",
            "file delete $path",
            "load $fileName",
            "unset $varname",
        ] {
            assert_eq!(count(src, D, "W304"), 1, "expected W304 for {src:?}");
        }
    }

    #[test]
    fn commands_without_dashdash_support_not_checked() {
        // subst / string match / lsearch / glob-literal do not support `--`.
        assert!(!fires("subst $template", D, "W304"));
        assert!(!fires("string match $pattern $value", D, "W304"));
        assert!(!fires("lsearch -exact $domain c", D, "W304"));
        assert!(!fires("glob *.tcl", D, "W304"));
    }

    #[test]
    fn table_and_class_under_irules() {
        assert_eq!(count("table set $key $value", IR, "W304"), 1);
        assert_eq!(
            count("class match $item starts_with my_class", IR, "W304"),
            1
        );
    }
}

// ===========================================================================
// W217 — `unset` whose options consume every argument, so nothing is unset.
//
// `unset` recognises only `-nocomplain` / `--`; any other word (even `-foo`) is
// a variable name. When leading options eat all the arguments, the call is a
// silent no-op — usually a variable named `-nocomplain` that needs a `--`.
// ===========================================================================
mod unset_option_only {
    use super::*;

    #[test]
    fn nocomplain_only_fires_with_terminator_fix() {
        let ds = of_code("unset -nocomplain", D, "W217");
        assert_eq!(ds.len(), 1, "expected one W217");
        assert!(ds[0].0.contains("--"), "message mentions --: {:?}", ds[0].0);
        assert_eq!(ds[0].2.len(), 1, "one fix offered");
        assert!(
            ds[0].2[0].starts_with("-- "),
            "fix inserts the terminator: {:?}",
            ds[0].2[0]
        );
    }

    #[test]
    fn bare_terminator_and_repeated_flags_fire() {
        assert_eq!(count("unset --", D, "W217"), 1);
        assert_eq!(count("unset -nocomplain --", D, "W217"), 1);
        assert_eq!(count("unset -nocomplain -nocomplain", D, "W217"), 1);
    }

    #[test]
    fn option_only_unset_is_not_an_arity_error() {
        // tclsh 8.5+: `unset`, `unset -nocomplain`, `unset --` are all valid
        // no-ops, so the misleading E002 "too few arguments" must not fire —
        // the option-only cases are covered by W217 instead.
        assert!(!fires("unset -nocomplain", D, "E002"));
        assert!(!fires("unset --", D, "E002"));
        assert!(!fires("unset", D, "E002"));
    }

    #[test]
    fn call_that_unsets_a_variable_is_clean() {
        // A real variable name follows the options.
        assert!(!fires("unset -nocomplain $x", D, "W217"));
        assert!(!fires("unset -nocomplain foo", D, "W217"));
        assert!(!fires("unset x", D, "W217"));
        assert!(!fires("unset x y z", D, "W217"));
        // `--` terminator already present, then a real name.
        assert!(!fires("unset -- -nocomplain", D, "W217"));
        // A dash-name in first position is a variable, not an option.
        assert!(!fires("unset -foo", D, "W217"));
        assert!(!fires("unset foo -nocomplain", D, "W217"));
    }
}

// ===========================================================================
// W306 — literal-expected position contains a substitution.
//
// A bare single `$var`/`${ns::v}` regex pattern is the canonical parameterised-
// regex idiom (no braced equivalent) → silent. But `[cmd]` patterns (`[a-z]`
// parses as a *command substitution*, a classic footgun) and quoted patterns
// with a *live* `$var` are flagged.
// ===========================================================================
mod literal_expected {
    use super::*;

    #[test]
    fn bare_var_pattern_clean() {
        assert!(!fires("regexp -- $pattern $text", D, "W306"));
        assert!(!fires("regexp -- ${ns::pattern} $text", D, "W306"));
        assert!(!fires("regexp -- {^foo} $text", D, "W306"));
    }

    #[test]
    fn bare_var_pattern_clean_deep_in_proc_body() {
        // Suppression must apply inside nested proc/if/foreach bodies.
        let src = "proc f {} {\n    if {1} {\n        foreach x [list a b] {\n            regexp $pattern $x\n        }\n    }\n}\n";
        assert!(!fires(src, D, "W306"));
    }

    #[test]
    fn command_substitution_pattern_warns() {
        // `[build_re]` / `[a-z]` parse as command substitution → flagged.
        assert_eq!(count("regsub -- [build_re] $text X out", D, "W306"), 1);
        assert_eq!(count("regexp -- [a-z] $text", D, "W306"), 1);
    }

    #[test]
    fn concatenated_var_pattern_no_w306_rust_behaviour() {
        // The analyser only flags a *bracketed* command-substitution /
        // quoted-live-`$var` pattern, not a bare var-concatenation like
        // `$pattern$suffix`, so W306 does NOT fire here. (The reads still
        // surface W210 — `$pattern`/`$suffix`/`$text` are all unset.) Pinned to
        // the actual verdict.
        assert!(!fires("regexp -- $pattern$suffix $text", D, "W306"));
    }

    #[test]
    fn quoted_pure_var_pattern_no_w306() {
        // `"$pattern"` is byte-for-byte identical at runtime to the bare
        // `$pattern` parameterised-pattern idiom (the quotes group nothing), so
        // it is the same canonical form, not a foot-gun — no W306.
        assert!(!fires("regexp -- \"$pattern\" $text", D, "W306"));
        assert!(!fires("regexp -- \"${pattern}\" $text", D, "W306"));
    }

    #[test]
    fn quoted_concatenated_var_pattern_warns() {
        // A live `$suffix` concatenated with literal text (`"pre$suffix"`) is no
        // longer a pure reference — a literal WAS expected — so W306 fires and
        // advises braces.
        let ds = of_code("regexp -- \"pre$suffix\" $text", D, "W306");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("quotes"));
    }

    #[test]
    fn quoted_dollar_anchor_clean_live_var_warns() {
        // `"^foo$"` — the `$` before the closing quote is the regex end-anchor,
        // a *literal* `$` (verified vs tclsh: `regexp -- "^foo$" foo` → 1).
        assert!(!fires("regexp -- \"^foo$\" $text", D, "W306"));
        // `"^foo$bar"` — `$bar` IS a live substitution → still the footgun.
        assert_eq!(count("regexp -- \"^foo$bar\" $text", D, "W306"), 1);
    }

    #[test]
    fn class_match_literal_name_clean_under_irules() {
        assert!(!fires(
            "class match -- [HTTP::uri] ends_with {my_class}",
            IR,
            "W306"
        ));
    }
}

// ===========================================================================
// W307 — non-literal command name.
//
// tclsh: `$cmd arg` dispatches on the *value* of `$cmd` as a command word — a
// command-injection surface when `$cmd` is user-controlled. TclOO/snit object
// dispatch (`$obj method`) and param-dispatcher idioms are recognised and
// suppressed.
//
// `$cmd` in a `foreach` over a list of *known* command names is not suppressed
// here — that known-command-list provenance is absent on this surface, so W307
// fires there (pinned below).
// ===========================================================================
mod non_literal_command {
    use super::*;

    #[test]
    fn variable_and_command_subst_command_name_fire() {
        let dv = of_code("$cmd arg1 arg2", D, "W307");
        assert_eq!(dv.len(), 1);
        assert!(dv[0].0.contains("Non-literal"));
        assert_eq!(dv[0].1, Severity::Warning);
        assert_eq!(count("[get_cmd] arg1", D, "W307"), 1);
    }

    #[test]
    fn literal_command_clean() {
        assert!(!fires("puts hello", D, "W307"));
    }

    #[test]
    fn tcloo_instance_dispatch_clean() {
        // `$obj greet world` where obj is a TclOO instance is method dispatch.
        let src = "oo::class create MyClass {\n    method greet {name} {\n        puts \"hello $name\"\n    }\n}\nset obj [MyClass new]\n$obj greet world\n";
        assert!(!fires(src, D, "W307"));
    }

    #[test]
    fn param_used_as_dispatcher_clean() {
        // A proc dispatching on its own parameter documents an object-handle API.
        assert!(!fires("proc once {tree} { $tree visit }", D, "W307"));
        let walk = "proc walk {tree} {\n    foreach n [$tree leaves] {\n        $tree visit $n\n    }\n}\n";
        assert!(!fires(walk, D, "W307"));
    }

    #[test]
    fn local_dispatcher_not_param_still_fires() {
        // A single dispatch on a non-param local is still flagged.
        assert_eq!(
            count(
                "proc f {} {\n    set foo [some_factory]\n    $foo method\n}",
                D,
                "W307"
            ),
            1
        );
    }

    #[test]
    fn foreach_over_known_commands_is_suppressed() {
        // `foreach cmd [list …]` now folds the literal list into a CONSTSET, so
        // `$cmd` dispatch resolves to each element.  When every element is a
        // *known* command (`puts`/`set`/`expr`) the dispatch is provably safe and
        // W307 is suppressed.
        let known = "foreach cmd [list puts set expr] {\n    $cmd hello\n}\n";
        assert!(
            !fires(known, D, "W307"),
            "known-command-list dispatch must not fire W307 (folded provenance)"
        );
        // A list whose elements are *unresolved* command names is still flagged —
        // the folded names don't resolve to real commands, so the dispatch may
        // execute anything.
        let unknown = "foreach cmd [list unknown_cmd_a unknown_cmd_b] {\n    $cmd value\n}\n";
        assert!(fires(unknown, D, "W307"));
    }
}

// ===========================================================================
// W308 — TclOO unknown-method validation.
//
// `$obj nonexistent` where the class is statically known and has no such
// method → W308. Built-in methods (`destroy`), classes with a `method unknown`,
// `oo::objdefine`-added methods, and external/unknown classes suppress it.
//
// NOTE — `W308` here is solely the TclOO method check; the analyser does not
// emit a `subst`-without-`-nocommands` HINT on this surface, so those cases are
// out-of-surface.
// ===========================================================================
mod tcloo_unknown_method {
    use super::*;

    #[test]
    fn unknown_method_fires() {
        let src = "oo::class create MyClass {\n    method greet {name} {\n        puts \"hello $name\"\n    }\n}\nset obj [MyClass new]\n$obj nonexistent_method\n";
        let ds = of_code(src, D, "W308");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("nonexistent_method"));
    }

    #[test]
    fn builtin_method_clean() {
        let src = "oo::class create MyClass {\n    method greet {} { puts hello }\n}\nset obj [MyClass new]\n$obj destroy\n";
        assert!(!fires(src, D, "W308"));
    }

    #[test]
    fn chained_constructor_validates_method() {
        let src =
            "oo::class create Dog {\n    method bark {} { puts woof }\n}\n[Dog new] nonexistent\n";
        let ds = of_code(src, D, "W308");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("nonexistent"));
    }

    #[test]
    fn method_unknown_and_objdefine_suppress() {
        // A `method unknown` catch-all means any method may exist.
        let mu = "oo::class create Proxy {\n    method unknown {name args} { puts $name }\n}\nset p [Proxy new]\n$p anyMethod\n";
        assert!(!fires(mu, D, "W308"));
        // oo::objdefine adds methods at runtime → conservative, suppress.
        let od = "oo::class create Base {\n    method existing {} { puts hello }\n}\nset obj [Base new]\noo::objdefine $obj method custom {} { puts custom }\n$obj custom\n";
        assert!(!fires(od, D, "W308"));
    }

    #[test]
    fn external_class_no_w308() {
        // Unknown `[Circuit new …]` is not in all_classes → no method validation.
        let src = "set circuit [Circuit new {Monte-Carlo}]\n$circuit runAndRead\n";
        assert!(!fires(src, D, "W308"));
    }
}

// ===========================================================================
// W309 — eval/uplevel with [subst] (double substitution).
//
// `eval [subst $template]` substitutes the template, then evaluates the result
// as a script → two rounds of substitution, an injection vector → Error.
// ===========================================================================
mod eval_subst_double_substitution {
    use super::*;

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

// ===========================================================================
// W105 / W106 — unbraced code-block / switch-arm bodies.
//
// tclsh: `if {1} "puts $x"` substitutes `$x` before the body runs as code → a
// double-substitution / injection risk (Error). A single bare variable body
// (`eval $script`) is a script-valued reference, not an inline block → silent.
// ===========================================================================
mod unbraced_body {
    use super::*;

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
    fn switch_regexp_unbraced_body_is_error() {
        let ds = of_code("switch -regexp -- $x {^a} \"puts matched\"", D, "W106");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("regexp"));
        assert_eq!(ds[0].1, Severity::Error);
    }
}

// ===========================================================================
// W108 — non-ASCII token content (confusables mode).
//
// Pure style/security heuristic: in the default "confusables" mode, smart
// quotes and homoglyphs (Cyrillic А) are flagged with an ASCII auto-fix;
// non-confusable symbols (© °) are allowed.
// ===========================================================================
mod non_ascii {
    use super::*;

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

// ===========================================================================
// W212 — variable substitution where a variable *name* is expected.
//
// tclsh: `set $x 1` sets the variable *named by the value of* `$x` — almost
// never intended (the writer meant `set x 1`). Same for incr/append/lappend/
// unset/info-exists/upvar's local slot. `upvar`'s *remote* slot legitimately
// takes a `$var`; `info commands` is not name-taking.
// ===========================================================================
mod name_vs_value {
    use super::*;

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

// ===========================================================================
// E100 — unmatched close bracket.
//
// `set x foo]` has a stray `]` (the lexer flags it ESC, not a structural
// closer). A properly matched `[cmd]` or a `]` inside braces is fine.
// ===========================================================================
mod unmatched_close_bracket {
    use super::*;

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
        assert!(!ds.is_empty());
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

// ===========================================================================
// E102 — unmatched close brace.
//
// A bare `}` has no special meaning to Tcl outside a `{…}` word, so — like
// E100 — this is a "probably a typo" heuristic, not a hard parse error. It
// must fire wherever the `}` is stray, whether it is the whole token
// (`puts foo\n}`) or embedded in a larger word (`set x foo}bar`); a matched
// `{…}` or a `}` inside a quoted/braced word is fine.
// ===========================================================================
mod unmatched_close_brace {
    use super::*;

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

// ===========================================================================
// W213 — unset without -nocomplain on a possibly-undefined variable.
//
// tclsh ground truth: `unset x` on a never-set var → `can't unset "x": no such
// variable`. `unset -nocomplain` opts out (tclsh suppresses the error too);
// unsetting a previously-set var is fine.
// ===========================================================================
mod unset_nocomplain {
    use super::*;

    #[test]
    fn unset_undefined_var_fires() {
        let ds = of_code("unset x", D, "W213");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains('x'));
        assert!(ds[0].0.contains("nocomplain"));
    }

    #[test]
    fn nocomplain_and_defined_var_clean() {
        assert!(!fires("unset -nocomplain x", D, "W213"));
        assert!(!fires("set x 1\nunset x", D, "W213"));
        assert!(!fires("unset -nocomplain -- x", D, "W213"));
    }
}

// ===========================================================================
// W121 — invalid subnet mask (non-contiguous bits).
//
// Heuristic over dotted-quad literals: `255.255.255.1` looks like a mask but has
// non-contiguous one-bits. Valid CIDR masks (/0…/32) are clean.
// ===========================================================================
mod invalid_subnet_mask {
    use super::*;

    #[test]
    fn non_contiguous_masks_fire() {
        let ds = of_code("set mask 255.255.255.1", D, "W121");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("non-contiguous"));
        for m in ["255.0.255.0", "255.255.0.128", "255.255.253.0"] {
            assert_eq!(
                count(&format!("set mask {m}"), D, "W121"),
                1,
                "expected W121 for {m}"
            );
        }
    }

    #[test]
    fn valid_masks_clean() {
        for m in [
            "255.255.255.0",
            "255.255.254.0",
            "255.255.255.192",
            "255.255.255.255",
            "255.0.0.0",
            "0.0.0.0",
            "128.0.0.0",
        ] {
            assert!(
                !fires(&format!("set mask {m}"), D, "W121"),
                "unexpected W121 for {m}"
            );
        }
    }

    #[test]
    fn regular_ips_not_masks() {
        assert!(!fires("set addr 10.0.0.1", D, "W121"));
        assert!(!fires("set addr 192.168.1.1", D, "W121"));
    }

    #[test]
    fn invalid_mask_in_string_still_fires_with_suggestion() {
        // The invalid mask inside a string literal still fires; the suggested
        // mask is in the message. The diagnostic carries the suggestion in the
        // message text but no structured fix object here, so assert on the message.
        let ds = of_code("puts \"mask is 255.255.255.1\"", D, "W121");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("255.255.255.0"));
    }
}

// ===========================================================================
// W122 / W124 — mistyped IPv4 address.
//
// tclsh: `192.168.1.256` is not a valid IP (octet > 255). On this surface the
// SSA-traced **W124** supersedes the lexical **W122** everywhere (W122 never
// fires on its own), so these
// tests assert "W122 OR W124" via [`ip_count`]. Octet > 255 → Error;
// octal-ambiguous leading zero → Warning. OIDs / version numbers are excluded.
// ===========================================================================
mod mistyped_ipv4 {
    use super::*;

    /// Count of W122 + W124.
    fn ip_count(src: &str) -> usize {
        count(src, D, "W122") + count(src, D, "W124")
    }

    #[test]
    fn octet_over_255_is_error() {
        assert_eq!(ip_count("set addr 192.168.1.256"), 1);
        let ds = of_code("set addr 192.168.1.256", D, "W124");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("exceeds 255"));
        assert_eq!(ds[0].1, Severity::Error);
        assert_eq!(ip_count("set addr 10.0.0.999"), 1);
    }

    #[test]
    fn octal_ambiguous_leading_zero_is_warning() {
        let ds = of_code("set addr 192.168.01.1", D, "W124");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("leading zero"));
        assert_eq!(ds[0].1, Severity::Warning);
        assert_eq!(ip_count("set addr 192.168.077.1"), 1);
        assert_eq!(ip_count("set addr 10.00.0.1"), 1);
    }

    #[test]
    fn valid_ips_clean() {
        for a in ["192.168.1.1", "10.0.0.1", "0.0.0.0", "255.255.255.0"] {
            assert_eq!(
                ip_count(&format!("set addr {a}")),
                0,
                "unexpected IP diag for {a}"
            );
        }
    }

    #[test]
    fn non_octal_leading_zero_clean() {
        // A leading zero followed by a non-octal digit is not octal-ambiguous.
        for a in ["192.168.09.1", "10.0.08.1", "192.168.099.1"] {
            assert_eq!(
                ip_count(&format!("set addr {a}")),
                0,
                "unexpected IP diag for {a}"
            );
        }
    }

    #[test]
    fn oids_and_version_numbers_excluded() {
        // LDAP / SNMP OIDs and User-Agent version numbers are not IPv4 quads.
        assert_eq!(ip_count("set oid 1.3.6.1.4.1.4203.1.11.3"), 0);
        assert_eq!(ip_count("set oid 1.3.6.1.2.1.1.5.0"), 0);
        assert_eq!(
            ip_count("set ua \"Mozilla/5.0 Chrome/28.0.1550.0 Safari/537.36\""),
            0
        );
    }

    #[test]
    fn ssa_traced_in_proc_and_top_level() {
        // W124 covers proc bodies and top-level alike.
        let ds = of_code("proc f {} { set a 192.168.1.256 }", D, "W124");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("exceeds 255"));
        assert_eq!(count("set a 192.168.1.256", D, "W124"), 1);
        // A valid IP / version string inside a proc stays clean.
        assert!(!fires("proc f {} { set a 10.0.0.1 }", D, "W124"));
        assert!(!fires(
            "proc f {} { set ua \"Chrome/28.0.1550.0\" }",
            D,
            "W124"
        ));
    }
}

// ===========================================================================
// W126 — channel-type validation.
//
// SCCP-typed: a value whose inferred type cannot be a channel handle
// (INT/LIST) passed to a channel slot (`close`/`gets`/…), or a non-standard
// string literal, is flagged. Channels from open/socket, stdout/stderr, opaque
// STRING-typed handles, and conservatively-typed params are clean.
// ===========================================================================
mod channel_validation {
    use super::*;

    #[test]
    fn typed_channels_and_std_channels_clean() {
        assert!(!fires(
            "proc f {} { set fd [open /tmp/t r]; gets $fd line; close $fd }",
            D,
            "W126"
        ));
        assert!(!fires(
            "proc f {} { set s [socket localhost 80]; close $s }",
            D,
            "W126"
        ));
        assert!(!fires("proc f {} { puts stdout hello }", D, "W126"));
        assert!(!fires("proc f {} { puts stderr hello }", D, "W126"));
    }

    #[test]
    fn int_and_list_typed_values_fire() {
        let di = of_code("proc f {} { set x 42; close $x }", D, "W126");
        assert_eq!(di.len(), 1);
        assert!(di[0].0.contains("INT"));
        let dl = of_code("proc f {} { set l [list a b]; close $l }", D, "W126");
        assert_eq!(dl.len(), 1);
        assert!(dl[0].0.contains("LIST"));
    }

    #[test]
    fn non_channel_string_literal_fires() {
        let ds = of_code("proc f {} { gets notachannel line }", D, "W126");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("notachannel"));
    }

    #[test]
    fn opaque_param_and_string_typed_clean() {
        // Conservative: a bare param / round-tripped STRING handle could be a channel.
        assert!(!fires("proc f {fd} { close $fd }", D, "W126"));
        assert!(!fires(
            "proc f {token} { upvar 0 $token state; set sock $state(sock); close $sock }",
            D,
            "W126"
        ));
    }
}

// ===========================================================================
// W125 — orphaned control-flow keyword used as a standalone command.
//
// `else`/`elseif`/`then`/`finally`/`on`/`trap` on their own line (not `} else
// {` on the same line) parse as separate commands. A user proc named after the
// keyword suppresses it.
// ===========================================================================
mod orphaned_keyword {
    use super::*;

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

// ===========================================================================
// W113 — proc shadows a core built-in.
//
// Redefining an unqualified core builtin (`set`, `exit`) is a real shadow. A
// namespace-qualified library command (`::snit::type`, `::base64::encode`) is
// not a builtin → not flagged.
// ===========================================================================
mod builtin_shadow {
    use super::*;

    #[test]
    fn core_builtin_shadow_fires() {
        let ds = of_code("proc set {a b} {}", D, "W113");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("set"));
        assert_eq!(count("proc exit {} {}", D, "W113"), 1);
    }

    #[test]
    fn namespaced_library_command_not_flagged() {
        assert!(!fires(
            "namespace eval base64 { proc encode {s} {return $s} }",
            D,
            "W113"
        ));
        assert!(!fires("proc ::snit::type {name def} {}", D, "W113"));
    }

    // issue #923 idx 11: a bare-name `proc` whose only registry match is a
    // `required_package`-gated third-party command (argparse, a tcllib
    // package, …) must not fire W113 — that command does not exist in a
    // stock interpreter until its package is loaded, so defining a proc of
    // the same name is not shadowing a built-in; it is (often) that
    // package's own implementation. Ground truth (tclsh 9.0.4 / 8.6.14):
    // `info commands argparse` is empty and calling it errors
    // `invalid command name "argparse"` before the package is sourced.
    #[test]
    fn package_gated_command_name_not_flagged() {
        // TP (repro, argparse.tcl:17's exact shape): defining the
        // `argparse` package's own entry point must not warn that it
        // shadows a built-in — `argparse` is required_package-gated
        // (rust/tcl-registry/src/commands/argparse/command.rs), not a core
        // command.
        assert!(!fires("proc ::argparse {args} {}", D, "W113"));
        // A tcllib package command name hits the same gate.
        assert!(!fires("proc ::textutil::adjust {s} {}", D, "W113"));
    }

    // TN: a genuine core built-in (no `required_package`) still fires,
    // across dialects — the package-gating filter must not blunt the
    // original check.
    #[test]
    fn ungated_builtin_shadow_still_fires_across_dialects() {
        assert_eq!(count("proc set {a b} {}", "tcl8.4", "W113"), 1);
        assert_eq!(count("proc set {a b} {}", "tcl9.0", "W113"), 1);
    }

    // TN: iRules' `pool` is ambient (part of the profile's real, always-
    // present command surface) and carries no `required_package` at all —
    // it must keep firing W113 exactly as before this fix, proving the new
    // filter does not touch commands that were never package-gated.
    #[test]
    fn ambient_irules_command_still_flagged() {
        assert_eq!(count("proc pool {a} {}", IR, "W113"), 1);
    }
}

// ===========================================================================
// W200 — binary format signed/unsigned modifier requires Tcl 8.5+ (TIP 275).
//
// Version-gated on the dialect's *effective Tcl version* (the §6
// argument-DSL rung): it warns under 8.4-based runtimes (tcl8.4, iRules'
// embedded 8.4.6) and is clean from 8.5 up — including f5-iapps, whose
// host embeds a real Tcl 8.5.13 (the old hardcoded dialect list wrongly
// flagged it).
// ===========================================================================
mod binary_format_modifiers {
    use super::*;

    #[test]
    fn unsigned_modifier_warns_under_old_dialects() {
        let ds = of_code("binary format su $val", "tcl8.4", "W200");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("modifier"));
        assert_eq!(ds[0].1, Severity::Warning);
        assert_eq!(count("binary scan $data su x", "tcl8.4", "W200"), 1);
        assert_eq!(count("binary format su $val", IR, "W200"), 1);
        // iApps embed Tcl 8.5.13 — the u/s modifiers are real there, and
        // flagging them was a false positive the profile model fixed.
        assert_eq!(count("binary format su $val", "f5-iapps", "W200"), 0);
    }

    #[test]
    fn unsigned_modifier_clean_under_tcl86() {
        assert!(!fires("binary format su $val", "tcl8.6", "W200"));
    }

    #[test]
    fn no_modifier_clean() {
        assert!(!fires("binary format s $val", "tcl8.4", "W200"));
    }
}

// ===========================================================================
// W310 — hardcoded credentials.
//
// Heuristic: a literal value after a credential-bearing flag (`-password`,
// `-token`) or sensitive header (`Authorization`) is a likely secret. A
// variable value is not hardcoded → clean.
// ===========================================================================
mod hardcoded_credentials {
    use super::*;

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

// ===========================================================================
// W311 — unsafe channel encoding mismatch.
//
// `-encoding binary` with a non-binary `-translation` (auto/crlf) corrupts
// binary data. Matching `-translation binary` (or no translation) is clean.
// ===========================================================================
mod encoding_mismatch {
    use super::*;

    #[test]
    fn binary_encoding_text_translation_warns() {
        let ds = of_code(
            "fconfigure $fd -encoding binary -translation auto",
            D,
            "W311",
        );
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Warning);
        assert!(ds[0].0.to_lowercase().contains("binary"));
        assert_eq!(
            count(
                "chan configure $fd -encoding binary -translation crlf",
                D,
                "W311"
            ),
            1
        );
    }

    #[test]
    fn matching_or_absent_translation_clean() {
        assert!(!fires(
            "fconfigure $fd -encoding binary -translation binary",
            D,
            "W311"
        ));
        assert!(!fires(
            "fconfigure $fd -encoding utf-8 -translation auto",
            D,
            "W311"
        ));
        assert!(!fires("fconfigure $fd -encoding binary", D, "W311"));
    }
}

// ===========================================================================
// W312 — interp eval / interp invokehidden injection.
//
// Same shape as W301 for child interpreters: an unbraced / multi-arg script
// risks injection; `[list …]` is the safe canonical form.
// ===========================================================================
mod interp_eval_injection {
    use super::*;

    #[test]
    fn unbraced_with_variable_warns() {
        let ds = of_code("interp eval $child \"set x $y\"", D, "W312");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].1, Severity::Warning);
    }

    #[test]
    fn braced_and_list_clean() {
        assert!(!fires("interp eval $child {set x 1}", D, "W312"));
        assert!(!fires(
            "interp eval $child [list set $varname $value]",
            D,
            "W312"
        ));
        assert!(!fires("interp aliases $child", D, "W312"));
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

// ===========================================================================
// W313 — destructive file operations with a variable path (taint-aware).
//
// `file delete/rename/mkdir $path` risks path traversal. A literal path / a
// non-destructive op (`file exists`/`file join`) is clean. (Bounds-check
// suppression lives in the taint pipeline; the basic firing surface is here.)
// ===========================================================================
mod destructive_file_ops {
    use super::*;

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

// ===========================================================================
// IRULE2002 / IRULE2003 / IRULE3102 / IRULE1002 — F5 iRules checks.
//
// Dialect-gated to f5-irules (silent under plain Tcl). IRULE2002: deprecated
// command → modern replacement. IRULE2003: unsafe command (Error). IRULE3102:
// HTTP getters should use -normalized. IRULE1002: unknown event name.
// ===========================================================================
mod irules_checks {
    use super::*;

    #[test]
    fn deprecated_command_suggests_replacement() {
        let ds = of_code("client_addr", IR, "IRULE2002");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("deprecated"));
        assert!(ds[0].0.contains("IP::client_addr"));
        assert_eq!(ds[0].1, Severity::Warning);
        assert!(
            of_code("redirect \"http://example.com\"", IR, "IRULE2002")[0]
                .0
                .contains("HTTP::redirect")
        );
        assert!(
            of_code("set uri [http_uri]", IR, "IRULE2002")[0]
                .0
                .contains("HTTP::uri")
        );
    }

    #[test]
    fn deprecated_not_flagged_under_plain_tcl() {
        assert!(!fires("set x 1", D, "IRULE2002"));
        assert!(!fires("client_addr", D, "IRULE2002"));
    }

    #[test]
    fn unsafe_command_is_error() {
        let ds = of_code("uplevel 1 {set x 1}", IR, "IRULE2003");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.to_lowercase().contains("unsafe"));
        assert_eq!(ds[0].1, Severity::Error);
        assert_eq!(count("history", IR, "IRULE2003"), 1);
        // Not checked under plain Tcl.
        assert!(!fires("uplevel 1 {set x 1}", D, "IRULE2003"));
    }

    #[test]
    fn http_getter_should_be_normalized() {
        let ds = of_code("set p [HTTP::path]", IR, "IRULE3102");
        assert!(!ds.is_empty());
        assert!(ds[0].0.contains("HTTP::path -normalized"));
        assert_eq!(ds[0].1, Severity::Warning);
        // Already normalized / setter form / plain Tcl → clean.
        assert!(!fires("set p [HTTP::path -normalized]", IR, "IRULE3102"));
        assert!(!fires("HTTP::path /lowercase/path", IR, "IRULE3102"));
        assert!(!fires("set p [HTTP::path]", D, "IRULE3102"));
    }

    #[test]
    fn unknown_event_warns() {
        let ds = of_code("when FAKE_EVENT {puts hi}", IR, "IRULE1002");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("Unknown iRules event"));
        assert_eq!(ds[0].1, Severity::Warning);
        assert!(!fires("when HTTP_REQUEST {puts hi}", IR, "IRULE1002"));
        assert!(!fires("when FAKE_EVENT {puts hi}", D, "IRULE1002"));
    }
}

// ===========================================================================
// W214 — unused proc parameter (pure static heuristic), checks-specific angles.
//
// Empty-body signature stubs and dispatch-protocol params (≥3 peers sharing a
// signature *plus* a dispatcher) are recognised and suppressed; a `switch`
// subject counts as a use; genuinely-unused params still fire.
// ===========================================================================
mod unused_proc_parameters {
    use super::*;

    fn w214_names(src: &str) -> std::collections::HashSet<String> {
        of_code(src, D, "W214")
            .into_iter()
            .filter_map(|(m, _, _)| m.split('\'').nth(1).map(str::to_string))
            .collect()
    }

    #[test]
    fn empty_body_stub_silences_all_params() {
        // tclsh: an empty-body proc is a valid placeholder returning "". Every
        // param is necessarily "unused" — flagging them is noise.
        assert!(w214_names("proc reverse {fa} {}\n").is_empty());
        assert!(w214_names("proc complete {fa {sink {}}} {}\n").is_empty());
    }

    #[test]
    fn real_body_unused_params_still_fire() {
        assert_eq!(
            w214_names("proc foo {x y} { puts hello }\n"),
            ["x", "y"].map(String::from).into()
        );
    }

    #[test]
    fn switch_subject_counts_as_use() {
        // A param read only as a `switch -- $col {…}` subject is used.
        let src = "proc editStartCmd {tbl row col text} {\n    switch -- $col {\n        1 { set combList columns }\n        default { return $text }\n    }\n    return $combList\n}\n";
        let names = w214_names(src);
        assert!(!names.contains("col"), "col is used as switch subject");
        // `row` (and `tbl`) are genuinely unused.
        assert!(names.contains("row"));
    }

    #[test]
    fn dispatch_protocol_with_dispatcher_suppresses() {
        // ≥3 peers sharing `{s e}` + a variable-command dispatcher (`walk`) →
        // protocol contract, params suppressed (tcllib pt::peg parser-rule shape).
        let src = "namespace eval ::g {\n    proc ALNUM {s e} { return alnum }\n    proc ALPHA {s e} { return alpha }\n    proc DIGIT {s e} { return digit }\n    proc walk {rule s e} { $rule $s $e }\n}\n";
        assert!(w214_names(src).is_empty());
    }

    #[test]
    fn two_peers_without_protocol_still_fire() {
        // Only 2 peers — not enough to infer a protocol → each unused param fires.
        let src = "namespace eval ::g {\n    proc rule_a {s e} { return alnum }\n    proc rule_b {s e} { return alpha }\n}\n";
        assert_eq!(w214_names(src), ["s", "e"].map(String::from).into());
    }

    #[test]
    fn three_peers_without_dispatcher_still_fire() {
        // Peer-count alone is insufficient: no dispatcher anywhere → just helpers.
        let src = "namespace eval ::n {\n    proc a {ctx token} { puts $ctx }\n    proc b {ctx token} { puts $ctx }\n    proc c {ctx token} { puts $ctx }\n}\n";
        assert_eq!(w214_names(src), ["token"].map(String::from).into());
    }

    /// The proc FQNs W214 names, for `src`.
    fn w214_procs(src: &str) -> std::collections::HashSet<String> {
        of_code(src, D, "W214")
            .into_iter()
            .filter_map(|(m, _, _)| m.split('\'').nth(3).map(str::to_string))
            .collect()
    }

    #[test]
    fn nested_proc_is_named_by_its_defining_namespace() {
        // Issue #1077. Oracle (tclsh 9.0.4 / 8.6.16, identical): a proc body
        // runs in the namespace the proc is *defined* in, so `proc a::outer`'s
        // body creates `::a::helper`, not the lexical `::helper`.
        assert_eq!(
            w214_procs(
                "namespace eval ::a {}\nproc a::outer {} { proc helper {unusedp} { return 1 } }\n"
            ),
            ["::a::helper"].map(String::from).into(),
        );
        // The name word's own qualifier beats the enclosing `namespace eval`.
        assert_eq!(
            w214_procs(
                "namespace eval ::e {}\nnamespace eval ::d {\n    proc ::e::o3 {} { proc h3 {unusedz} { return 1 } }\n}\n"
            ),
            ["::e::h3"].map(String::from).into(),
        );
    }

    #[test]
    fn unqualified_and_absolute_nested_names_keep_their_fqn() {
        // TN pair for #1077 — the shapes where lexical and defining namespace
        // already agreed must not move.
        assert_eq!(
            w214_procs("proc outer {} { proc inner {unusedq} { return 1 } }\n"),
            ["::inner"].map(String::from).into(),
        );
        assert_eq!(
            w214_procs(
                "namespace eval ::a {}\nproc a::outer4 {} { proc ::abs {unusedq} { return 1 } }\n"
            ),
            ["::abs"].map(String::from).into(),
        );
    }

    #[test]
    fn unique_signature_still_fires() {
        assert_eq!(
            w214_names("proc foo {x y unused} { puts $x; puts $y }\n"),
            ["unused"].map(String::from).into()
        );
    }
}

// ===========================================================================
// W210 — read-before-set, checks-specific shapes that are CORRECT in Rust.
//
// These mirror tclsh ground truth and pass cleanly. (The catch-body false
// positive is isolated in the `catch_body_defs_rust_bug` module below.)
// ===========================================================================
mod read_before_set_correct {
    use super::*;

    fn w210_named(src: &str, var: &str) -> usize {
        of_code(src, D, "W210")
            .iter()
            .filter(|(m, _, _)| m.contains(var))
            .count()
    }

    #[test]
    fn info_exists_guard_is_not_read_before_set() {
        // tclsh: `info exists` on an unset var returns 0 (legal); it is the
        // canonical test-before-use idiom.
        assert!(!fires(
            "proc f {} { if {![info exists ns]} { set ns 1 }\n return $ns }",
            D,
            "W210"
        ));
        assert!(!fires(
            "proc f {} { set y [info exists x]\n return $y }",
            D,
            "W210"
        ));
        assert!(!fires(
            "proc f {} { if {[array exists arr]} { return [array size arr] }\n return 0 }",
            D,
            "W210"
        ));
    }

    #[test]
    fn return_reads_its_variables() {
        // tclsh: `return $x` with x unset errors `can't read "x"`.
        assert_eq!(count("proc f {} { return $x }", D, "W210"), 1);
        assert_eq!(count("proc f {} { return [expr {$x + 1}] }", D, "W210"), 1);
        // …but a set var / literal is fine.
        assert!(!fires("proc f {} { set x 1\n return $x }", D, "W210"));
        assert!(!fires("proc f {} { return 42 }", D, "W210"));
    }

    #[test]
    fn try_handler_versions_are_correct() {
        // on-error after a throw that set x sees it; a never-set var still fires.
        assert!(!fires(
            "proc f {} {\n    try {\n        set x 1\n        error boom\n    } on error {} {\n        puts $x\n    }\n}",
            D,
            "W210"
        ));
        assert_eq!(
            count(
                "proc f {} {\n    try {\n        error boom\n    } on error {} {\n        puts $y\n    }\n}",
                D,
                "W210"
            ),
            1
        );
        // An earlier conditional throw leaves x maybe-unset → still warns.
        assert_eq!(
            count(
                "proc f {c} {\n    try {\n        if {$c} { error a }\n        set x 1\n        error b\n    } on error {} {\n        puts $x\n    }\n}",
                D,
                "W210"
            ),
            1
        );
    }

    #[test]
    fn proc_written_global_suppresses_top_level_read() {
        // A proc declaring+setting a global makes a later top-level read safe
        // (the proc may have run via an earlier `source`).
        assert_eq!(
            w210_named(
                "proc set_name {v} {global package_name; set package_name $v}\nsource foo.tcl\nset out $package_name\n",
                "package_name"
            ),
            0
        );
        assert_eq!(
            w210_named(
                "proc set_name {v} {set ::package_name $v}\nset out $package_name\n",
                "package_name"
            ),
            0
        );
        assert_eq!(
            w210_named(
                "proc bump {} {global counter; incr counter}\nputs $counter\n",
                "counter"
            ),
            0
        );
    }

    #[test]
    fn global_alias_without_write_and_unrelated_still_warn() {
        // `global X` with no set does not populate it.
        assert_eq!(
            w210_named(
                "proc reader {} {global package_name; return $package_name}\nset out $package_name\n",
                "package_name"
            ),
            1
        );
        // A proc writing a *different* global doesn't mask a real read.
        assert_eq!(
            w210_named(
                "proc set_name {v} {global package_name; set package_name $v}\nputs $other_var\n",
                "other_var"
            ),
            1
        );
    }

    #[test]
    fn genuine_local_read_before_set_warns() {
        assert_eq!(
            count(
                "proc f {} {\n    set kids $children\n    return $kids\n}",
                D,
                "W210"
            ),
            1
        );
        assert_eq!(count("puts $never_set", D, "W210"), 1);
    }

    #[test]
    fn qualified_variable_alias_not_read_before_set() {
        // `variable ::tcl::WordBreakRE` links the local alias by tail.
        assert!(!fires(
            "proc f {} {\n    variable ::tcl::WordBreakRE\n    regexp -indices -- $WordBreakRE(after) abc result\n    return $result\n}",
            D,
            "W210"
        ));
    }
}

// ===========================================================================
// W210 — catch-body variable definitions suppress the false positive (FIXED).
//
// tclsh ground truth (8.6 + 9.0): `if {![catch {set x 1}]} { puts $x }` prints
// `1` — the catch *body* defines `x`, so the read in the if-body is safe and
// W210 must NOT fire. The Rust analyser previously recognised the
// `[catch {…} err]` *result-message* var (FP-RBS-02) but not a body-`set` of
// an *other* variable read after the catch, so it false-fired W210. The
// command-substitution out-var recovery now also scans the catch body, and
// the read-before-set suppression scans branch conditions (not just
// `set x [expr …]` assignments), so these guard-safe reads are no longer
// flagged.
// ===========================================================================
mod catch_body_defs_no_false_w210 {
    use super::*;

    #[test]
    fn catch_body_set_does_not_fire_w210() {
        // tclsh: the no-error branch ran the body to completion, so `$x` /
        // `$line` is set; W210 must NOT fire.
        assert_eq!(count("if {![catch {set x 1}]} { puts $x }", D, "W210"), 0);
        assert_eq!(
            count("if {![catch {gets stdin line}]} { puts $line }", D, "W210"),
            0
        );
    }

    #[test]
    fn genuine_read_before_set_still_warns() {
        // The control that holds either way: an unrelated unset var still fires.
        assert_eq!(count("puts $never_set", D, "W210"), 1);
    }
}

// ===========================================================================
// W210 — `unset` of a global does NOT suppress a later read (FIXED).
//
// tclsh ground truth: `proc clear {} {unset ::x}` then top-level `puts $x`
// errors `can't read "x"` — `unset` destroys, it does not initialise — so a
// real W210 is warranted. `globals_written_by_procs` now excludes `unset`
// (it removes a variable, never assigns one, unlike `set`/`global`-then-`set`),
// so the warranted top-level W210 fires instead of being suppressed.
// ===========================================================================
mod unset_global_does_not_suppress_w210 {
    use super::*;

    fn w210_named(src: &str, var: &str) -> usize {
        of_code(src, D, "W210")
            .iter()
            .filter(|(m, _, _)| m.contains(var))
            .count()
    }

    #[test]
    fn unset_only_global_still_fires_w210() {
        // tclsh: top-level `puts $x` errors `can't read "x": no such variable`
        // (proc isn't called; `unset` never initialises). W210 is warranted.
        assert_eq!(w210_named("proc clear {} {unset ::x}\nputs $x\n", "'x'"), 1);
        assert_eq!(
            w210_named("proc clear {} {global x; unset x}\nputs $x\n", "'x'"),
            1
        );
    }
}

// ===========================================================================
// W220 / W211 — dead-store & unused, checks-specific read-recovery cases.
//
// Read-modify-write reads (incr/append/lappend), reads inside command-
// substituted exprs, and reads inside braced `eval {…}` bodies must be
// recovered so the feeding assignment is not falsely dead/unused. Array
// elements are distinguished by overlap. Genuine dead stores still fire.
// ===========================================================================
mod dead_store_and_unused {
    use super::*;

    #[test]
    fn distinct_array_element_write_not_dead() {
        // `set a(k)` read by `puts $a(k)`; the intervening `a(j)` write is not it.
        assert!(!fires(
            "proc f {} { set a(k) 1\n set a(j) 2\n puts $a(k) }",
            D,
            "W220"
        ));
        assert!(!fires(
            "proc f {} { set o(-mode) cbc\n set o(-key) k\n return [array get o] }",
            D,
            "W220"
        ));
    }

    #[test]
    fn genuinely_unused_element_and_scalar_dead_store_fire() {
        // `set a(k) 1` on an array that is never read is flagged W211 ("set but
        // never used"); the co-located dead-store W220 is deduped in favour of
        // that single, clearer hint.
        assert_eq!(count("proc f {} { set a(k) 1\n return 0 }", D, "W211"), 1);
        assert_eq!(count("proc f {} { set a(k) 1\n return 0 }", D, "W220"), 0);
        // A genuine dead store of a *used* scalar still fires a standalone W220
        // (no W211 to dedup against).
        assert_eq!(
            count("proc f {} { set x 1\n set x 2\n return $x }", D, "W220"),
            1
        );
    }

    #[test]
    fn read_modify_write_in_cmdsub_keeps_init_live() {
        let src = "proc f {} { set i 0\n foreach j {1 2 3} { lappend r [incr i $j] }\n return $r }";
        assert!(!fires(src, D, "W220"));
        assert!(!fires(src, D, "W211"));
    }

    #[test]
    fn cmdsub_expr_read_keeps_init_live() {
        assert!(!fires(
            "proc f {} { set w 5; set i 0; incr i [expr {$w}]; return $i }",
            D,
            "W220"
        ));
        assert!(!fires(
            "proc f {n} { set scale 3; return [expr {$n * $scale}] }",
            D,
            "W211"
        ));
        assert!(!fires(
            "proc f {n} { set scale 3; return [expr {$n * $scale}] }",
            D,
            "W220"
        ));
    }

    #[test]
    fn eval_braced_body_read_keeps_init_live() {
        assert!(!fires("proc f {} { set x 1; eval {puts $x} }", D, "W220"));
        assert!(!fires("proc f {} { set x 1; eval {puts $x} }", D, "W211"));
        // …but an eval body that does NOT read x leaves it unused.
        assert_eq!(count("proc f {} { set x 1; eval {puts hi} }", D, "W211"), 1);
    }

    #[test]
    fn genuine_dead_store_still_flagged() {
        assert!(fires(
            "proc f {} { set i 0\n set i 5\n return $i }",
            D,
            "W220"
        ));
        assert!(fires(
            "proc f {} { set y 1; set y 2; return $y }",
            D,
            "W220"
        ));
    }
}

// ===========================================================================
// Structural-body scope isolation (issue #250).
//
// An OO / snit body is STRUCTURAL — it must not contribute reads/writes to the
// enclosing proc's data flow, otherwise it would silence the proc's own W210 /
// dead-store / W214 findings.
//
// Message shape: the dead `set x 1` surfaces as **W220 ("never read")** rather
// than W211 ("set but never used") — the structural-isolation intent (all three
// findings survive) holds, so the assertion is pinned to the code set
// {W210, W214, W220}.
// ===========================================================================
mod structural_body_isolation {
    use super::*;

    fn outer_codes(structural_call: &str) -> std::collections::HashSet<String> {
        let src =
            format!("proc outer {{p}} {{\n    set x 1\n    puts $y\n    {structural_call}\n}}\n");
        codes(&src, D)
            .into_iter()
            .filter(|c| ["W210", "W211", "W214", "W220"].contains(&c.as_str()))
            .collect()
    }

    #[test]
    fn structural_bodies_do_not_leak_into_outer_scope() {
        // For each structural call, the outer proc's own findings all survive:
        // W210 (read $y before set), W214 (unused param p), and the dead `set x`
        // (W220 on this surface).
        for structural_call in [
            "oo::class create C { method m {} { puts $x; set y 1; set p 2 } }",
            "oo::define C { method m {} { puts $x; set y 1; set p 2 } }",
            "oo::objdefine C method m {} { puts $x; set y 1; set p 2 }",
            "snit::type T { method m {} { puts $x; set y 1; set p 2 } }",
            "snit::method T m {} { puts $x; set y 1; set p 2 }",
            "uri::register demo { puts $x; set y 1; set p 2 }",
        ] {
            let cs = outer_codes(structural_call);
            assert!(
                cs.contains("W210"),
                "missing W210 for {structural_call:?}: {cs:?}"
            );
            assert!(
                cs.contains("W214"),
                "missing W214 for {structural_call:?}: {cs:?}"
            );
            // Dead `set x` surfaces as W220 on this surface.
            assert!(
                cs.contains("W220"),
                "missing dead-store for {structural_call:?}: {cs:?}"
            );
        }
    }
}

// ===========================================================================
// namespace eval body scope (caller-frame vs child-namespace).
//
// `namespace eval ns {…}` runs in ns, NOT the caller frame, so an unqualified
// `$x` inside it is not a use of the caller's local `x` (param stays unused).
// Plain `eval {…}` DOES run in the current frame → recovers the read.
// ===========================================================================
mod namespace_eval_body_scope {
    use super::*;

    fn w214_names(src: &str) -> std::collections::HashSet<String> {
        of_code(src, D, "W214")
            .into_iter()
            .filter_map(|(m, _, _)| m.split('\'').nth(1).map(str::to_string))
            .collect()
    }

    #[test]
    fn param_used_only_in_ns_eval_body_is_unused() {
        assert!(
            w214_names("proc g {x} { namespace eval ::ns { puts \"hello $x\" } }").contains("x")
        );
        assert!(
            w214_names("proc g {x ns} { namespace eval $ns { puts \"hello $x\" } }").contains("x")
        );
    }

    #[test]
    fn param_used_in_plain_eval_body_is_used() {
        assert!(!w214_names("proc g {x} { eval { puts \"hello $x\" } }").contains("x"));
    }

    #[test]
    fn ns_eval_dead_store_still_fires_name_arg_recovered() {
        // `set y 1` is dead in the caller (the body's `$y` runs in ::ns).
        assert!(fires(
            "proc g {} { set y 1; namespace eval ::ns { puts \"$y\" } }",
            D,
            "W220"
        ));
        // …but the namespace *name* expression IS evaluated in the caller scope.
        assert!(!fires(
            "proc f {conn} { set name [format ns%s $conn]\n namespace eval $name { variable s } }",
            D,
            "W220"
        ));
    }
}

// ===========================================================================
// Integration — multiple checks on realistic code.
// ===========================================================================
mod integration {
    use super::*;

    #[test]
    fn multiple_issues_in_one_proc() {
        let src = "\nproc dangerous {input} {\n    set result [eval \"process $input\"]\n    expr $result + 1\n    catch {risky_op}\n}\n";
        let cs = codes(src, D);
        assert!(cs.iter().any(|c| c == "W101"), "eval injection");
        assert!(cs.iter().any(|c| c == "W100"), "unbraced expr");
        assert!(cs.iter().any(|c| c == "W302"), "catch no result");
    }

    #[test]
    fn clean_proc_no_style_or_security_warnings() {
        // No W1xx/W3xx style or security warning fires on idiomatic code.
        let src = "\nproc safe {x} {\n    set result [expr {$x * 2}]\n    if {$result > 10} {\n        puts \"big\"\n    }\n    catch {risky_op} err\n    return $result\n}\n";
        let offenders: Vec<_> = codes(src, D)
            .into_iter()
            .filter(|c| (c.starts_with("W1") || c.starts_with("W3")) && c != "W123")
            .collect();
        assert!(
            offenders.is_empty(),
            "unexpected style/security warnings: {offenders:?}"
        );
    }

    #[test]
    fn security_critical_patterns_all_fire() {
        let src = "\nset script \"doThing $userArg\"\neval $script\nsource $untrusted_path\nsubst $user_template\nopen \"|$user_cmd\"\n";
        let cs: std::collections::HashSet<_> = codes(src, D).into_iter().collect();
        for code in ["W101", "W300", "W102", "W103"] {
            assert!(cs.contains(code), "missing {code} in {cs:?}");
        }
    }

    #[test]
    fn no_style_or_security_false_positives_on_standard_patterns() {
        // The dead-store on `a` (from `eval {set a 1}`) and an I230 fire here —
        // those are NOT the style/security families this test guards. Assert no
        // W1xx/W3xx (except W123) appears.
        let src = "\nset x 42\nset y [expr {$x + 1}]\nif {$y > 40} {\n    puts \"yes\"\n} else {\n    puts \"no\"\n}\nwhile {$x > 0} {\n    incr x -1\n}\nfor {set i 0} {$i < 10} {incr i} {\n    lappend result $i\n}\nforeach item {a b c} {\n    puts $item\n}\ncatch {risky} result opts\nopen {data.txt} r\nsource {lib/utils.tcl}\neval {set a 1}\n";
        let offenders: Vec<_> = codes(src, D)
            .into_iter()
            .filter(|c| (c.starts_with("W1") || c.starts_with("W3")) && c != "W123")
            .collect();
        assert!(
            offenders.is_empty(),
            "unexpected style/security warnings: {offenders:?}"
        );
    }

    #[test]
    fn nested_and_switch_bodies_checked() {
        // Checks run inside proc / if / switch / command-substitution bodies.
        assert!(fires(
            "\nproc test {} {\n    if {1} {\n        expr $x + 1\n    }\n}\n",
            D,
            "W100"
        ));
        assert!(fires(
            "\nswitch $x {\n    a {\n        expr $y + 1\n    }\n}\n",
            D,
            "W100"
        ));
        assert!(fires("set x [expr $y + 1]", D, "W100"));
        assert!(fires("expr $a + 1; eval $script", D, "W100"));
        assert!(fires("expr $a + 1; eval $script", D, "W101"));
    }
}

// ===========================================================================
// Edge cases.
// ===========================================================================
mod edge_cases {
    use super::*;

    #[test]
    fn empty_and_comment_only_sources_are_silent() {
        assert!(codes("", D).is_empty());
        assert!(codes("# just a comment\n# another comment", D).is_empty());
    }

    #[test]
    fn expr_and_eval_no_args_yield_arity_not_check_code() {
        // `expr` / `eval` with no args → E002 (too few), and the W100/W101 check
        // does not crash or fire.
        assert!(fires("expr", D, "E002"));
        assert!(!fires("expr", D, "W100"));
        assert!(fires("eval", D, "E002"));
        assert!(!fires("eval", D, "W101"));
    }

    #[test]
    fn deeply_nested_and_backslash_continuation_checked() {
        let src = "\nproc a {} {\n    proc b {} {\n        if {1} {\n            while {1} {\n                expr $deep\n            }\n        }\n    }\n}\n";
        assert!(fires(src, D, "W100"));
        // Backslash line-continuation before the expr argument.
        assert!(fires("expr \\\n$x + 1", D, "W100"));
    }
}

// ===========================================================================
// Call-by-name suppression precision — the `DynamicNameLocal` refinement.
//
// A callee whose parameter's *value* names a CALLEE-LOCAL variable
// (`set $p 1`, `scan … $p`, `lassign … $p`, `regsub … $p`) does NOT
// consume the caller's variable when a literal name is passed (`f x`),
// so the caller's otherwise-unused `set x 1` stays a dead store
// (W211 / W220).  Only a genuine `upvar`-aliased write-back suppresses
// it.  These pin the `DYNAMIC_NAME_LOCAL` refinement (PR #498 / #499
// findings 10 / 6); the absence of that refinement would re-open the
// caller-side false negatives (gap #6) it guards against.
// ===========================================================================
mod call_by_name_dynamic_name_local {
    use super::*;

    /// Each callee uses `$p` as a callee-local dynamic name; the caller's
    /// `set x 1; f x` leaves `x` unread → the dead store must still be flagged.
    /// `x` is set-once-never-read, so it surfaces as W211 ("set but never
    /// used"); the co-located dead-store W220 is deduped in favour of that
    /// single hint. The point of the test — that the callee's dynamic name does
    /// not *suppress* the caller's detection — holds via the surviving W211.
    #[test]
    fn callee_local_dynamic_name_does_not_suppress_caller_dead_store() {
        for (label, src) in [
            ("set", "proc f {p} { set $p 1 }\nproc g {} { set x 1; f x }"),
            (
                "scan",
                "proc f {p} { scan $s {%d} $p }\nproc g {} { set x 1; f x }",
            ),
            (
                "lassign",
                "proc f {p} { lassign $l $p }\nproc g {} { set x 1; f x }",
            ),
            (
                "regsub",
                "proc f {p} { regsub a $s b $p }\nproc g {} { set x 1; f x }",
            ),
        ] {
            assert!(
                fires(src, D, "W211"),
                "[{label}] callee-local dynamic name must not suppress caller W211; got {:?}",
                codes(src, D),
            );
            assert!(
                !fires(src, D, "W220"),
                "[{label}] the caller dead store is covered by W211, so the co-located \
                 W220 is deduped; got {:?}",
                codes(src, D),
            );
        }
    }

    /// A genuine caller-frame write-back through `upvar` IS call-by-name —
    /// the caller's `x` is consumed, so neither W211 nor W220 fires.
    #[test]
    fn genuine_upvar_call_by_name_suppresses_caller_dead_store() {
        let src = "proc f {p} { upvar 1 $p l; set l 5 }\nproc g {} { set x 1; f x }";
        assert!(
            !fires(src, D, "W211"),
            "genuine upvar call-by-name should suppress caller W211; got {:?}",
            codes(src, D),
        );
        assert!(
            !fires(src, D, "W220"),
            "genuine upvar call-by-name should suppress caller W220; got {:?}",
            codes(src, D),
        );
    }
}

// W143 — direct call into a private `::tcl::` implementation namespace.
//
// tclsh ground truth: `::tcl::dict::create a 1` runs and returns `a 1` under
// both tclsh8.6 and tclsh9.0 (confirmed live) — the call works, so this is
// advisory (Warning), not an error. Prefix-level only: no per-subcommand or
// per-version modelling (issue #988).
mod private_tcl_namespace {
    use super::*;

    #[test]
    fn direct_call_is_flagged_with_concrete_suggestion() {
        let ds = of_code("::tcl::dict::create a 1", D, "W143");
        assert_eq!(ds.len(), 1);
        assert!(
            ds[0].0.contains("dict create"),
            "expected the concrete 'dict create' suggestion in the message; got {:?}",
            ds[0].0
        );
        // The bare (non-`::`-rooted) spelling is equally flagged.
        assert!(fires("tcl::dict::create a 1", D, "W143"));
    }

    #[test]
    fn quick_fix_replaces_head_with_public_ensemble_call() {
        let ds = of_code("::tcl::string::totitle abc", D, "W143");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2, vec!["string totitle".to_string()]);
    }

    // The fix's description must name the *whole* replacement, not just the
    // ensemble command — "Replace with 'dict'" alongside a `dict create`
    // edit told the user one thing and did another.
    #[test]
    fn quick_fix_description_matches_its_replacement_text() {
        let src = "::tcl::dict::create a 1";
        let ds = Analyser::new()
            .analyse(src, D)
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W143")
            .flat_map(|d| d.fixes.clone())
            .collect::<Vec<_>>();
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].new_text, "dict create");
        assert_eq!(ds[0].description, "Replace with 'dict create'");
    }

    #[test]
    fn every_whole_private_namespace_fires() {
        use tcl_registry::private_tcl_namespaces::{PRIVATE_TCL_NAMESPACES, TailRule};
        for ns in PRIVATE_TCL_NAMESPACES
            .iter()
            .filter(|n| n.tail_rule == TailRule::Whole)
        {
            let src = format!("::tcl::{}::sub arg", ns.namespace);
            assert!(fires(&src, D, "W143"), "expected W143 for {src:?}");
        }
    }

    // (a) `::tcl::chan` is shared with tcllib's public `virtchannel_base`
    // packages, so only a genuine `chan` ensemble subcommand is private
    // there.  `tcl::chan::memchan` and friends are real, documented,
    // registry-known commands.
    #[test]
    fn public_tcllib_channel_packages_not_flagged() {
        for src in [
            "package require tcl::chan::memchan\nset c [tcl::chan::memchan]",
            "set c [::tcl::chan::memchan]",
            "set c [tcl::chan::fifo]",
            "set c [tcl::chan::cat $a $b]",
            "set c [tcl::transform::base64 $chan]",
        ] {
            assert!(!fires(src, D, "W143"), "unexpected W143 for {src:?}");
        }
    }

    // …and W120 and W143 can no longer contradict each other: the memchan
    // repro asks for a `package require` (W120) without also claiming the
    // command is Tcl-private (W143).
    #[test]
    fn missing_package_require_does_not_also_draw_w143() {
        let src = "set c [tcl::chan::memchan]";
        assert!(
            fires(src, D, "W120"),
            "expected W120; got {:?}",
            codes(src, D)
        );
        assert!(!fires(src, D, "W143"));
    }

    // A genuine `chan` ensemble-backing call under the shared namespace is
    // still flagged, with the fix.
    #[test]
    fn genuine_chan_ensemble_backing_still_fires() {
        let ds = of_code("::tcl::chan::truncate $c 0", D, "W143");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2, vec!["chan truncate".to_string()]);
    }

    // (b) A file that defines the command itself owns that name — a user's
    // own `proc ::tcl::dict::mine` is not a call into Tcl's internals.
    #[test]
    fn file_defining_the_command_suppresses_the_warning() {
        for src in [
            "proc ::tcl::dict::mine {d} { return $d }\n::tcl::dict::mine {a 1}",
            "namespace eval ::tcl::dict { proc mine {d} { return $d } }\n::tcl::dict::mine {a 1}",
            "proc ::tcl::dict::mine {d} { return $d }\nproc caller {} { ::tcl::dict::mine {a 1} }",
        ] {
            assert!(!fires(src, D, "W143"), "unexpected W143 for {src:?}");
        }
    }

    // …but only for the name it actually defines.
    #[test]
    fn defining_one_private_name_does_not_licence_another() {
        let src = "proc ::tcl::dict::mine {d} { return $d }\n::tcl::dict::create a 1";
        assert!(fires(src, D, "W143"));
    }

    // (c) A `package require` covering the qualified name (or any namespace
    // prefix of it) means the name belongs to that package.
    #[test]
    fn package_require_in_scope_suppresses_the_warning() {
        for src in [
            // The package *is* the command.
            "package require tcl::dict::mycustom\n::tcl::dict::mycustom {a 1}",
            // …or owns the namespace the command sits in.
            "package require tcl::dict\n::tcl::dict::mycustom {a 1}",
            // Order-independent: the analyser's whole-file view sees the
            // requirement wherever it sits.
            "::tcl::dict::mycustom {a 1}\npackage require tcl::dict::mycustom",
        ] {
            assert!(!fires(src, D, "W143"), "unexpected W143 for {src:?}");
        }
    }

    // (d) A tail that is not a subcommand of the public ensemble still
    // warns, but must NOT offer a code-corrupting rewrite.
    #[test]
    fn non_subcommand_tail_warns_without_a_fix() {
        for (src, needle) in [
            ("::tcl::clock::GetSystemTimeZone", "GetSystemTimeZone"),
            ("::tcl::dict::mycustom {a 1}", "mycustom"),
            ("::tcl::dict::a::b {a 1}", "a::b"),
        ] {
            let ds = of_code(src, D, "W143");
            assert_eq!(ds.len(), 1, "expected exactly one W143 for {src:?}");
            assert!(
                ds[0].2.is_empty(),
                "{src:?} must offer no quick fix; got {:?}",
                ds[0].2
            );
            assert!(
                ds[0].0.contains(needle),
                "message should name the tail {needle:?}; got {:?}",
                ds[0].0
            );
        }
    }

    // (e) A delimited command head — `"::tcl::dict::create"` or
    // `{::tcl::dict::create}` — still warns, but the head token's span does
    // not cover its delimiters, so no fix is offered rather than one that
    // leaves an orphan `"`/`}` behind.
    #[test]
    fn delimited_head_warns_without_a_fix() {
        for src in ["\"::tcl::dict::create\" a 1", "{::tcl::dict::create} a 1"] {
            let ds = of_code(src, D, "W143");
            assert_eq!(ds.len(), 1, "expected exactly one W143 for {src:?}");
            assert!(
                ds[0].2.is_empty(),
                "{src:?} must offer no quick fix; got {:?}",
                ds[0].2
            );
        }
    }

    #[test]
    fn public_ensemble_call_not_flagged() {
        assert!(!fires("dict create a 1", D, "W143"));
        assert!(!fires("string totitle abc", D, "W143"));
    }

    #[test]
    fn users_own_tcl_namespace_not_flagged() {
        assert!(!fires(
            "namespace eval ::tcl::mycustom { proc foo {} {} }\n::tcl::mycustom::foo",
            D,
            "W143"
        ));
    }

    #[test]
    fn public_documented_tcl_rooted_namespaces_not_flagged() {
        // `tcl::mathop`/`tcl::mathfunc` are public, documented namespaces —
        // must never be confused with the private implementation ones.
        // Both the `::`-rooted and the bare spellings, at the integration
        // layer (the registry classifier pins them too).
        for src in [
            "::tcl::mathop::+ 1 2",
            "tcl::mathop::+ 1 2",
            "::tcl::mathfunc::sin 0",
            "tcl::mathfunc::sin 0",
            "set x [tcl::mathop::* 2 3]",
            "set y [tcl::mathfunc::max 1 2]",
        ] {
            assert!(!fires(src, D, "W143"), "unexpected W143 for {src:?}");
        }
        // `tcl::prefix` is likewise a real, public, documented command
        // (Tcl 8.6+) — not a private namespace.
        assert!(!fires("tcl::prefix match {apple banana} app", D, "W143"));
    }

    #[test]
    fn near_miss_namespace_not_flagged() {
        // Shares the `dict` prefix textually but is not the exact segment.
        assert!(!fires("::tcl::dictionary::foo", D, "W143"));
    }
}

// ===========================================================================
// The `Tcl_ConcatObj` eval family — issue #1051.
//
// `eval`, `uplevel`, `namespace eval`, `namespace inscope`, and `interp eval`
// evaluate the *concatenation* of every trailing script word, so analysing
// only the first word invents diagnostics about a script that never runs.
//
// Every expectation here was pinned against tclsh8.6.14 and tclsh9.0.4 (both
// agree on every shape); the transcript is quoted per test where the answer
// is not obvious.
// ===========================================================================
mod script_concatenation {
    use super::*;

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

    /// FP — `namespace eval` concatenates too, and used to drop everything
    /// past the first script word.
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
