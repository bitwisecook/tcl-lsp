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

//! W307: a variable or command substitution as the command word, and the constants, parameters and aliases that excuse it.

use super::*;

#[test]
fn w307_selected_alias_slot_does_not_require_a_live_terminal_command() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let declaration = if dialect == "jimtcl" {
            "alias short missing"
        } else {
            "interp alias {} short {} missing"
        };
        let source = format!("{declaration}; set c short; $c arg");
        let result = Analyser::new().analyse(&source, dialect);
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != DiagCode::W307),
            "{dialect}: an actual alias slot is independently known: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn w307_expanded_prefix_uses_the_original_dispatch_point() {
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert!(
            !has_code(
                "set command {list first}\nputs [{*}$command second]",
                dialect,
                "W307",
            ),
            "{dialect}: expansion selects a present command slot"
        );
        assert!(
            has_code(
                "set command {phase1_missing_prefix call}\nputs [{*}$command second]",
                dialect,
                "W307",
            ),
            "{dialect}: expansion of a missing head still warns"
        );
    }
}

#[test]
fn w307_stock_namespace_capture_keeps_the_proved_expanded_command() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/work/name-handling/handover/oo-dispatch-probe/dispatch/c13-namespace-code.tcl",
    ));
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert!(!has_code(source, dialect, "W307"), "{dialect}");
    }
}

#[test]
fn w307_absence_advice_does_not_override_custom_fallback_uncertainty() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let dispatches = "set cmd definitely_missing; $cmd one; $cmd two";
        assert!(
            has_code(dispatches, dialect, "W307"),
            "{dialect}: repeated use does not establish a present slot"
        );
        assert!(
            !has_code(
                &format!("proc unknown args {{return handled}}; {dispatches}"),
                dialect,
                "W307"
            ),
            "{dialect}: a custom fallback remains independent of slot absence"
        );
    }
}

#[test]
fn tn_w307_expanded_runtime_command_list_stays_abstaining() {
    // The SpiceGenTcl shape that must stay abstaining: `{*}$element` expands a *runtime-built*
    // list into the callee inside a method body (SpiceGenTcl's
    // `buildTopNetlist`).  No W307/W308/E001, and no false resolution.
    let src = "oo::class create Netlist { method add {e} { return $e } }\n\
               oo::class create Builder {\n\
                 method build {cls args} {\n\
                   set topNetlist [Netlist new]\n\
                   set element [list $cls new {*}$args]\n\
                   $topNetlist add [{*}$element]\n\
                   return $topNetlist\n\
                 }\n\
               }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::W307 | DiagCode::W308 | DiagCode::E001)),
        "the runtime-built `{{*}}$element` dispatch must stay a silent \
         abstention; got {:?}",
        r.diagnostics
    );
}

#[test]
fn w307_suppressed_for_braced_namespace_var_proc_param() {
    // `${namespace}::define::…` — the dispatched variable is the *braced*
    // name `namespace` (a proc parameter), not the whole composite head.
    // The var-name extraction must isolate `namespace` so the proc-param
    // dispatch suppression fires (a mangled name defeats it → false W307).
    let src = "proc Define {namespace class args} {\n  \
                   ${namespace}::dynamic_methods $class\n  \
                   ${namespace}::define::[lindex $args 0] {*}[lrange $args 1 end]\n}\n";
    assert_eq!(
        w30x_codes(src).len(),
        0,
        "braced-namespace-var proc-param dispatch must suppress W307; got {:?}",
        w30x_codes(src)
    );
}

#[test]
fn w307_suppressed_for_snit_instance_var_in_helper_proc() {
    // `mytree` is a snit instance variable dispatched inside a non-method
    // helper `proc` in the type body (`upvar`'d component object).  The
    // dispatch falls in the class body range and names an instance var, so
    // `is_snit_member` suppresses W307.
    let src = "snit::type T {\n  variable mytree\n  \
                   proc Check {id} { upvar 1 mytree mytree; if {![$mytree exists $id]} { return } }\n}\n";
    assert_eq!(
        w30x_codes(src).len(),
        0,
        "snit instance-var dispatch in a helper proc must suppress W307; got {:?}",
        w30x_codes(src)
    );
}

#[test]
fn analyse_w307_emitted_for_cmd_substitution_with_unknown_return_type() {
    // ``[bogus_cmd] foo`` — the inner command isn't in the
    // registry, so the return type is unknown.  W307 should
    // fire for the cmd-as-command site.
    let src = "[bogus_cmd] foo";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 expected for [unknown] method pattern; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w307_suppressed_for_my_self_dispatch() {
    // ``[my method]`` is OO self-dispatch — never trips W307. It is one only
    // inside a method: at file level `my` is an undefined command (tclsh
    // 8.6.18 / 9.0.4).
    let src = "oo::class create C {\n    method m {} {}\n    method n {} {[my m] arg}\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 must not fire for OO self-dispatch; got {:?}",
        r.diagnostics,
    );
}

/// W307 reads the element and body bindings from outcomes, the lattice and
/// the plan: a `dict with` key path binds the nested dictionary's
/// keys (the spelling harvest read the outer dictionary's); an `array set`
/// over a lattice-constant list binds its elements; and in a function with
/// a barrier, which widens every value it holds, a literal `array set` or
/// `set arr(k)` still states its element write. A dispatch on a known
/// command is silent and one on a non-command still fires.
#[test]
fn w307_reads_element_and_body_bindings() {
    let w307 = |src: &str| {
        let mut a = Analyser::new();
        a.analyse(src, "tcl")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W307)
            .count()
    };
    let path = "proc f {d} { dict with d a { $cmd hi } }\n";
    assert_eq!(w307(&format!("{path}f {{a {{cmd puts}}}}\n")), 0);
    assert_eq!(w307(&format!("{path}f {{a {{cmd notACommand}}}}\n")), 1);
    let pairs = |value: &str| {
        format!("proc f {{}} {{ set pairs {{run {value}}}\n array set h $pairs\n $h(run) hello }}")
    };
    assert_eq!(w307(&pairs("puts")), 0);
    assert_eq!(w307(&pairs("notACommand")), 1);
    let barrier = |write: &str| format!("proc f {{x}} {{ {write}\n eval $x\n $h(run) hello }}");
    assert_eq!(w307(&barrier("array set h {run puts}")), 0);
    assert_eq!(w307(&barrier("array set h {run notACommand}")), 1);
    assert_eq!(w307(&barrier("set h(run) puts")), 0);
    assert_eq!(w307(&barrier("set h(run) notACommand")), 1);
}

/// **TN/TP for the array-element constant harvest** — the W307 input whose
/// element predicate is [`tcl_syntax::naming::split_element_ref`].
/// A dispatch through `$cmd(k)` is resolved from the literal the
/// element was set to: a real command suppresses W307, an unknown one does
/// not. If the harvest stops recognising `cmd(k)` as an element it collects
/// no constant and the first case warns.
#[test]
fn w307_reads_the_constant_an_array_element_was_set_to() {
    // `var_command_sites` is populated by the analyser's walk dispatch, so
    // this must go through `analyse` rather than the emitter pipeline.
    let codes = |src: &str| {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect::<Vec<_>>()
    };
    let known = codes("proc f {} { set cmd(k) puts\n $cmd(k) hi }");
    assert!(
        !known.contains(&"W307".to_string()),
        "the element's literal proves the head is `puts`; got {known:?}",
    );
    let unknown = codes("proc f {} { set cmd(k) nosuchcommand\n $cmd(k) hi }");
    assert!(
        unknown.contains(&"W307".to_string()),
        "an element set to a non-command must still warn; got {unknown:?}",
    );
}

#[test]
fn analyse_w307_var_as_command() {
    // ``proc foo {} { $cmd arg1 }`` — ``$cmd`` (a non-parameter local
    // dispatched once) is used as command head with no static knowledge
    // of what it holds, so W307 fires.  Must go through ``analyse`` (not
    // raw ``emit_cfg_ssa_diagnostics``) because ``var_command_sites`` is
    // populated by the analyser's walk dispatch, not the emitter pipeline.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { $cmd arg1 }", "tcl");
    let w307s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W307)
        .collect();
    assert!(
        !w307s.is_empty(),
        "W307 expected for ``$cmd arg1``; got {:?}",
        r.diagnostics,
    );
    assert_eq!(w307s[0].severity, Severity::Warning);
    assert!(w307s[0].message.contains("Non-literal command name"));
}

#[test]
fn analyse_w307_suppressed_for_proc_param_dispatch() {
    // A dispatch on a *parameter* of the enclosing proc is object dispatch
    // the user documented as the proc's API contract — W307 must stay
    // silent.  `$self configure` is the canonical method-dispatch
    // idiom on an opaque handle.
    let mut a = Analyser::new();
    let r = a.analyse("proc p {self} { $self configure -x 1 }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 must be suppressed for a dispatch on proc parameter; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w307_suppressed_for_multi_dispatch_local() {
    // A non-parameter local dispatched ≥2 times demonstrates intent
    // (object usage), so W307 is suppressed even without a known value.
    let mut a = Analyser::new();
    let r = a.analyse("proc p {} { $tree visit\n$tree leaves }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 must be suppressed for a local dispatched ≥2 times; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w307_fires_for_tainted_dispatch_despite_multi_use() {
    // Taint carve-out: a user-controlled command name dispatched multiple
    // times is still a command-injection risk — the dispatcher-suppression
    // must NOT apply, so W307 fires.
    let mut a = Analyser::new();
    let r = a.analyse("proc p {} { set c [gets stdin]\n$c one\n$c two }", "tcl8.6");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 must fire for a tainted dispatched command name; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_no_w307_for_static_known_command() {
    // ``proc foo {} { set cmd puts; $cmd hello }`` — ``cmd``
    // has constant value "puts" which IS a known command, so
    // W307 must be suppressed.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { set cmd puts\n$cmd hello }", "tcl");
    let w307s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W307)
        .collect();
    assert_eq!(
        w307s.len(),
        0,
        "W307 must be suppressed when var holds known command name; got {:?}",
        r.diagnostics,
    );
}

// `is_known_command` (used by `w307_site_suppressed`) must not suppress
// W307 merely because SCCP proved a dynamic-dispatch value equals a
// proc/class name: a variable holding a renamed-or-deleted-away command (no
// later re-establishment) would silence the real "invalid command name"
// hazard. The dispatch site's own offset is threaded through and gated by
// `fact_live_for_call`. Confirmed against tclsh 8.6.14.

#[test]
fn w307_tp_issue_1010_dispatch_value_is_deleted_proc_stays_flagged() {
    let src = "proc target {} {}\nrename target {}\nproc foo {} { set cmd target\n$cmd hello }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "a dispatch value equalling a deleted proc must still draw W307; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w307_fp_issue_1010_dispatch_value_is_reestablished_proc_resolves() {
    let src = "proc target {} {}\nrename target {}\nproc target {} {}\nproc foo {} { set cmd target\n$cmd hello }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "a proc re-established after deletion must still suppress W307; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w307_suppressed_per_ssa_version_after_reassignment() {
    // Per-SSA-version refinement: `cmd` is
    // reassigned from a non-command to a known command before the
    // dispatch.  The merged const-set {notacommand, puts} would
    // wrongly keep W307 alive; reading the value at the dispatch's
    // exact SSA use-version ("puts") suppresses it.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc foo {} { set cmd notacommand\nset cmd puts\n$cmd hello }",
        "tcl",
    );
    let w307s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W307)
        .collect();
    assert_eq!(
        w307s.len(),
        0,
        "W307 must read the precise reaching version (\"puts\"); got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w307_still_fires_when_precise_version_not_a_command() {
    // The mirror case: the reaching version is a non-command, so
    // W307 must still fire (the refinement only suppresses when the
    // exact value is provably a known command).
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc foo {} { set cmd puts\nset cmd notacommand\n$cmd hello }",
        "tcl",
    );
    let w307s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W307)
        .collect();
    assert!(
        !w307s.is_empty(),
        "W307 should fire when the reaching version isn't a command; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_var_command_sites_recorded_during_walk() {
    // Smoke: confirm the recording infrastructure populates
    // ``var_command_sites`` for ``$var`` heads.  Run analyse
    // (not just emit) so the apply_disabled_diagnostics +
    // dedupe don't matter — we inspect post-analyse state.
    let mut a = Analyser::new();
    let _ = a.analyse("proc foo {x} { $x arg }", "tcl");
    // After analyse, var_command_sites is consumed by the
    // post-pass but restored at the end (snapshot/restore
    // contract).
    assert!(
        a.var_command_sites.iter().any(|s| s.var_name == "x"),
        "var_command_sites should record ``$x`` head; got {:?}",
        a.var_command_sites,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_cmd_command_sites_recorded_during_walk() {
    // ``[cmd] arg`` records to ``cmd_command_sites`` even
    // though no W307 emitter consumes it yet.
    let mut a = Analyser::new();
    let _ = a.analyse("proc foo {} { [puts hi] arg }", "tcl");
    assert!(
        !a.cmd_command_sites.is_empty(),
        "cmd_command_sites should be populated for ``[cmd] arg``; got {:?}",
        a.cmd_command_sites,
    );
}
