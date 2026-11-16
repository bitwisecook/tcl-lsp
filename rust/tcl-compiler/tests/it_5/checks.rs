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
//! - `W210` does not false-fire when a `catch {set x …}` body defines the
//!   variable read in the surrounding `if` body (catch-body + branch-
//!   condition out-var recovery).
//! - `W210` fires when a proc `unset`s (and never sets) a global later read
//!   at top level, where tclsh errors `can't read` (`unset` is excluded
//!   from the proc global-write set).
//! - `W308` is the *`TclOO` unknown-method* code here; it does not cover the
//!   `subst`-without-`-nocommands` hint (those cases are out-of-surface; the
//!   `TclOO` `W308` cases are covered).
//! - `W122` never fires on the single-document surface — the SSA-traced `W124`
//!   supersedes it everywhere — so the IP tests assert "W122 or W124".
//! - Per-event cross-`when` scoping, taint-pipeline-only `W201`/`W313`
//!   internals, and a few message-format / fix-shape assertions are pinned to
//!   this implementation's shape (dead-store reported as `W220` "never read"
//!   rather than `W211` "set but never used" in structural-body cases, etc.).

use tcl_compiler::analyser::{Analyser, Severity};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_registry::model::ingress::static_context_for;

mod argument_checks;
mod commands;
mod injection;
mod irules;
mod quoting;
mod variable_flow;

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
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
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
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    run_all_checks(&cu, registry, dialect_opt)
        .into_iter()
        .filter(|d| !d.code.is_optimisation() && d.code.to_string() == code)
        .map(|d| (d.message, d.severity))
        .collect()
}

// Integration — multiple checks on realistic code.
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
        // The inputs are bound first: tclsh errors on reading an unset variable.
        let src = "set userArg [gets stdin]\nset untrusted_path [gets stdin]\nset user_template [gets stdin]\nset user_cmd [gets stdin]\n\nset script \"doThing $userArg\"\neval $script\nsource $untrusted_path\nsubst $user_template\nopen \"|$user_cmd\"\n";
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

// Edge cases.
mod edge_cases {
    use super::*;

    #[test]
    fn empty_and_comment_only_sources_are_silent() {
        assert_eq!(codes("", D), [] as [std::string::String; 0]);
        assert_eq!(
            codes("# just a comment\n# another comment", D),
            [] as [std::string::String; 0]
        );
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
