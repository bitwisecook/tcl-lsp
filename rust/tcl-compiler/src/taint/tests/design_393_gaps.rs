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

//! The analyser gaps listed in `docs/design/registry/model/393-taint.md`
//! § *Bugs found with today's analyser*, run end to end through
//! `run_all_checks` as `tcl diag` does.

use crate::compilation_unit::CompilationUnit;
use crate::compiler_checks::run_all_checks;
use tcl_core_types::DiagCode;

const TCL_RELEASES: [&str; 5] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"];
const TCL_AND_JIM: [&str; 6] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];

fn codes(dialect: &str, source: &str) -> Vec<DiagCode> {
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
    let unit = CompilationUnit::build_for_dialect(source, registry, false, dialect)
        .with_interprocedural(registry, Some(profile));
    let mut codes: Vec<_> = run_all_checks(&unit, registry, Some(profile))
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    codes.sort_by_key(|code| format!("{code:?}"));
    codes
}

fn has(dialect: &str, source: &str, code: DiagCode) -> bool {
    codes(dialect, source).contains(&code)
}

fn assert_has(dialects: &[&str], source: &str, code: DiagCode) {
    for &dialect in dialects {
        let found = codes(dialect, source);
        assert!(
            found.contains(&code),
            "{dialect}: {source:?} lacks {code:?}: {found:?}"
        );
    }
}

fn assert_lacks(dialects: &[&str], source: &str, code: DiagCode) {
    for &dialect in dialects {
        let found = codes(dialect, source);
        assert!(
            !found.contains(&code),
            "{dialect}: {source:?} has {code:?}: {found:?}"
        );
    }
}

/// Item 1: host I/O re-enters Tcl only through a callback-carrying channel,
/// and no such channel exists while the stock channel proof holds.
#[test]
fn command_sources_reach_sinks_under_every_release() {
    for source in [
        "set x [gets stdin]; eval $x",
        "set x [read stdin]; eval $x",
        "set x [exec cat]; eval $x",
        "set x [lindex $argv 0]; eval $x",
    ] {
        assert_has(&TCL_AND_JIM, source, DiagCode::T100);
    }
    // `chan` arrived in 8.5 and Jim has none.
    assert_has(
        &TCL_RELEASES[1..],
        "set x [chan gets stdin]; eval $x",
        DiagCode::T100,
    );
}

/// Item 2: an implicit-stdout `puts` withdraws nothing later sinks need.
#[test]
fn sinks_after_a_one_argument_puts_still_fire() {
    let same = "set x [gets stdin]\nputs $x\nexec sh -c $x";
    let other = "set x [gets stdin]\nputs $x\nset z [gets stdin]; exec sh -c $z";
    for source in [same, other] {
        assert_has(&TCL_AND_JIM, source, DiagCode::T100);
        assert_has(&TCL_AND_JIM, source, DiagCode::T101);
    }
}

/// Item 3: a sink in a command substitution anywhere in an argument word.
#[test]
fn nested_substitution_sinks_fire() {
    for source in [
        "set n [gets stdin]; list [exec sh -c $n]",
        "set n [gets stdin]; list a [list [eval $n]]",
        "proc p {} { set n [gets stdin]; list [exec sh -c $n] }",
    ] {
        assert_has(&TCL_AND_JIM, source, DiagCode::T100);
    }
    let found = codes("tcl8.6", "set n [gets stdin]; [eval $n] a");
    assert_eq!(
        found.iter().filter(|code| **code == DiagCode::T100).count(),
        1,
        "{found:?}"
    );
}

/// Item 4: what `subst` and `eval` return keeps the template's taint.
#[test]
fn subst_and_eval_results_stay_tainted() {
    for source in [
        "set x [gets stdin]; set y [subst $x]; puts $y",
        "set x [gets stdin]; set y [eval $x]; puts $y",
    ] {
        assert_has(&TCL_AND_JIM, source, DiagCode::T100);
        assert_has(&TCL_AND_JIM, source, DiagCode::T101);
    }
    // The backslash pass can add CR or LF, so no colour survives.
    let crlf = "set x [gets stdin]; set m [string map {\"\\r\" \"\" \"\\n\" \"\"} $x]; \
                puts [subst -nocommands -novariables $m]";
    assert_has(&TCL_AND_JIM, crlf, DiagCode::T101);
    assert_has(
        &TCL_AND_JIM,
        "set x [gets stdin]; set y [subst -nocommands -novariables $x]; puts $y",
        DiagCode::T101,
    );
    // A later write after the withdrawn world is still read for what it holds.
    assert_lacks(
        &TCL_AND_JIM,
        "source lib.tcl\nset x abc\nputs $x",
        DiagCode::T101,
    );
    assert_lacks(&TCL_AND_JIM, "source lib.tcl\nputs $cfg", DiagCode::T101);
}

/// Item 5: `$a([cmd])` runs `cmd` whenever variables are substituted.
#[test]
fn subst_nocommands_is_still_a_code_sink() {
    assert_has(
        &TCL_AND_JIM,
        "set x [gets stdin]\nsubst -nocommands $x",
        DiagCode::T100,
    );
    assert_has(
        &["tcl9.1"],
        "set x [gets stdin]\nsubst -variables $x",
        DiagCode::T100,
    );
    assert_lacks(
        &TCL_AND_JIM,
        "set x [gets stdin]\nsubst -nocommands -novariables $x",
        DiagCode::T100,
    );
    assert_lacks(
        &["tcl9.1"],
        "set x [gets stdin]\nsubst -backslashes $x",
        DiagCode::T100,
    );
}

/// Item 6: an unresolved package leaves later taint in force.
#[test]
fn unresolved_package_require_keeps_taint() {
    assert_has(
        &TCL_AND_JIM,
        "package require foo\nset x [gets stdin]\nputs $x",
        DiagCode::T101,
    );
}

/// Item 7: the inline CRLF sanitiser clears iRules header and log sinks.
#[test]
fn irules_inline_crlf_sanitiser_is_recognised() {
    let map = "[string map {\"\\r\" \"\" \"\\n\" \"\"} $l]";
    let header =
        format!("when HTTP_REQUEST {{ set l [HTTP::host]; HTTP::header insert X-L {map} }}");
    let log = format!("when HTTP_REQUEST {{ set l [HTTP::host]; log local0. {map} }}");
    assert!(!has("f5-irules", &header, DiagCode::Irule3002));
    assert!(!has("f5-irules", &log, DiagCode::Irule3003));
    assert!(has(
        "f5-irules",
        "when HTTP_REQUEST { set l [HTTP::host]; log local0. [string map {\"\\n\" \"\"} $l] }",
        DiagCode::Irule3003
    ));
}

/// Item 8: a sanitising procedure's summary carries what its body adds.
#[test]
fn procedure_sanitiser_is_carried_through_its_summary() {
    let oneline = "proc oneline {s} {\n    return [string map {\"\\r\" \"\" \"\\n\" \"\"} $s]\n}\n\
                   set x [gets stdin]\n";
    // Under Jim and Tcl 8.4 this body's summary carries no transform yet, so
    // the call keeps its taint there.
    for source in [
        format!("{oneline}puts [oneline $x]"),
        format!("{oneline}set y [oneline $x]\nputs $y"),
    ] {
        assert_lacks(&TCL_RELEASES[1..], &source, DiagCode::T101);
    }
    assert_has(
        &TCL_AND_JIM,
        &format!("{oneline}exec sh -c [oneline $x]"),
        DiagCode::T100,
    );
    let half = "proc half {s} {\n    return [string map {\"\\r\" \"\"} $s]\n}\n\
                set x [gets stdin]\nputs [half $x]";
    assert_has(&TCL_AND_JIM, half, DiagCode::T101);
}

/// Item 9: `foreach` and `lmap` carry the iterated list's taint, and a sink
/// in a loop body survives the loop's back edge.
#[test]
fn foreach_keeps_taint() {
    for source in [
        "set c [lindex $argv 0]; foreach s [list $c] { eval $s }",
        "set c [lindex $argv 0]; foreach s [list $c] { exec sh -c $s }",
        "set c [lindex $argv 0]; foreach {a b} [list x $c] { exec sh -c $a }",
        "set c [lindex $argv 0]; while 1 { eval $c }",
    ] {
        assert_has(&TCL_AND_JIM, source, DiagCode::T100);
    }
    assert_has(
        &TCL_RELEASES[2..],
        "set c [lindex $argv 0]; set r [lmap s [list $c] { string toupper $s }]; eval $r",
        DiagCode::T100,
    );
    assert_lacks(
        &TCL_AND_JIM,
        "foreach s [list a b] { eval $s }",
        DiagCode::T100,
    );
}

/// Item 10: `string is digit|alnum -strict` and an anchored class `regexp`
/// validate; `integer`, a missing `-strict`, `||`, `-line` and `!` do not.
#[test]
fn validators_refine_the_guarded_branch() {
    let guarded = |condition: &str, body: &str| {
        format!("set x [gets stdin]; set force 1\nif {{{condition}}} {{ {body} }}")
    };
    for condition in [
        "[string is digit -strict $x]",
        "[string is alnum -strict $x]",
        "[regexp {^[0-9]+$} $x]",
        "[regexp -nocase {^[a-z0-9_]+$} $x]",
        "[string is digit -strict $x] && $force",
    ] {
        assert_lacks(
            &TCL_AND_JIM,
            &guarded(condition, "exec sleep $x"),
            DiagCode::T100,
        );
        assert_lacks(&TCL_AND_JIM, &guarded(condition, "puts $x"), DiagCode::T101);
    }
    for condition in [
        "[string is integer -strict $x]",
        "[string is digit $x]",
        "[string is digit -strict $x] || $force",
        "[regexp -line {^[a-z]+$} $x]",
        "[regexp {^[a-z-]+$} $x]",
        "![string is digit -strict $x]",
    ] {
        assert_has(
            &TCL_AND_JIM,
            &guarded(condition, "exec sleep $x"),
            DiagCode::T100,
        );
    }
    assert_has(
        &TCL_AND_JIM,
        &guarded(
            "[string is digit -strict $x]",
            "set x [gets stdin]; exec sleep $x",
        ),
        DiagCode::T100,
    );
    // A digit string is still a script to `eval`.
    assert_has(
        &TCL_AND_JIM,
        &guarded("[string is digit -strict $x]", "eval $x"),
        DiagCode::T100,
    );
}

/// Item 11: an iRules getter is a source only where its spec is visible.
#[test]
fn irules_getters_are_sources_only_in_irules() {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.load_irules();
    let irules = tcl_dialect::DialectProfile::irules();
    let f5 = registry.project_for_profile(irules);
    assert!(tcl_registry::taint::is_taint_source(
        &f5,
        "HTTP::host",
        &[],
        super::super::dialect_to_point(Some(irules)),
    ));
    for release in TCL_RELEASES {
        let profile = tcl_dialect::DialectProfile::find(release).unwrap();
        let tcl = registry.project_for_profile(profile);
        let point = super::super::dialect_to_point(Some(profile));
        assert!(
            !tcl_registry::taint::is_taint_source(&tcl, "HTTP::host", &[], point),
            "{release}"
        );
        assert!(tcl_registry::taint::is_taint_source(
            &tcl,
            "gets",
            &["stdin"],
            point
        ));
    }
    assert_lacks(&TCL_RELEASES, "set h [HTTP::host]; eval $h", DiagCode::T100);
    assert!(has(
        "f5-irules",
        "when HTTP_REQUEST { set h [HTTP::host]; eval $h }",
        DiagCode::T100
    ));
}
