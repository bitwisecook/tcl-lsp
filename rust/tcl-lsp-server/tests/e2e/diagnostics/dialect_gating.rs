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

//! What the active dialect admits: options (W004), `expr` operators (W003), math functions and each vendor profile's command set.

use super::*;

// W004 fires when an option is unavailable in the active dialect. It must
// also recognise abbreviated subcommands, and stay silent when a same-file
// proc redefines the builtin it would otherwise apply to.

#[test]
fn lsearch_stride_on_tcl86_is_w004() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.6\nlsearch -stride 2 {a b} x\n");
    assert!(has_code(&diags, "W004"));
}

#[test]
fn chan_configure_inputmode_on_tcl86_is_w004() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.6\nchan configure $chan -inputmode raw\n",
    );
    assert!(has_code(&diags, "W004"));
}

#[test]
fn abbreviated_chan_conf_inputmode_on_tcl86_is_still_w004() {
    // `conf` uniquely abbreviates `configure` — real Tcl ensemble dispatch
    // accepts it, so the option check must resolve it too rather than
    // silently skipping the whole option scan.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.6\nchan conf $chan -inputmode raw\n",
    );
    assert!(has_code(&diags, "W004"));
}

#[test]
fn lsearch_stride_on_tcl90_has_no_w004() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl9.0\nlsearch -stride 2 {a b} x\n");
    assert!(!has_code(&diags, "W004"));
}

#[test]
fn tcl9_lowercase_core_package_alias_selects_source_nopkg_grammar() {
    // C Tcl 9's bundled init.tcl uses `package require -exact tcl 9.0.3`.
    // The lower-case alias is a Tcl 9-only package spelling, so its version
    // guard must select Tcl 9 before source's registry-defined options are
    // checked. Without that detection, `-nopkg` was counted as a second file
    // name and produced a false E003.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "package require -exact tcl 9.0.3\nsource -nopkg library.tcl\n",
    );
    assert!(!has_code(&diags, "E003"), "got {diags:?}");
}

#[test]
fn source_tcl9_rejects_combined_encoding_and_nopkg_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl9.0\nsource -encoding utf-8 -nopkg library.tcl\n",
    );
    let conflicts = with_code(&diags, "W147");
    assert_eq!(conflicts.len(), 1, "expected one W147: {diags:?}");
    assert!(message(&conflicts[0]).contains("cannot be used together"));
}

#[test]
fn repeated_option_is_not_a_mutual_exclusion_conflict() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.6\nglob -directory root -directory other *.tcl\n",
    );
    assert!(
        !has_code(&diags, "W147"),
        "one canonical option repeated is not two mutually exclusive options: {diags:?}"
    );
}

#[test]
fn user_proc_shadowing_lsearch_suppresses_w004() {
    // `lsearch` really dispatches to the user's own proc here, so the
    // builtin's dialect-restricted `-stride` no longer applies.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src =
        "# tcl-dialect: tcl8.6\nproc lsearch {l args} { return $l }\nlsearch -stride 2 {a b}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W004"),
        "a user proc shadowing lsearch should suppress W004: {diags:?}"
    );
}

// W003 (dialect-gated expr operators).

#[test]
fn w003_tight_span_covers_only_the_operator() {
    // The diagnostic must highlight just the 2-byte `lt`, not the whole
    // `{$a lt $b}` condition or the enclosing `if`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.4\nif {$a lt $b} { puts hi }\n");
    let w003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W003"))
        .collect();
    assert_eq!(w003.len(), 1, "{diags:?}");
    let range = &w003[0]["range"];
    assert_eq!(range["start"]["line"], 1);
    assert_eq!(range["start"]["character"], 7);
    assert_eq!(range["end"]["line"], 1);
    assert_eq!(range["end"]["character"], 9, "span must cover only 'lt'");
}

#[test]
fn w003_repeated_operators_get_distinct_ranges_over_the_wire() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.4\nif {$a in $b && $c in $d} { puts hi }\n",
    );
    let w003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W003"))
        .collect();
    assert_eq!(w003.len(), 2, "{diags:?}");
    assert_ne!(w003[0]["range"], w003[1]["range"]);
}

#[test]
fn w003_message_cites_the_relevant_tip() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.4\nexpr {2 in {1 2 3}}\n");
    let w003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W003"))
        .collect();
    assert_eq!(w003.len(), 1, "{diags:?}");
    assert!(message(w003[0]).contains("TIP 201"), "{}", message(w003[0]));
}

#[test]
fn w003_eda_vendor_dialect_does_not_over_fire_on_in() {
    // `xilinx-eda-tcl` is documented as running on top of a real Tcl 8.5
    // core, so TIP 201's `in` must not be flagged there.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: xilinx-eda-tcl\nexpr {2 in {1 2 3}}\n");
    assert!(!has_code(&diags, "W003"), "{diags:?}");
}

#[test]
fn w003_f5_tmsh_flags_string_relational_operators() {
    // `f5-tmsh` needs its own `SpecSurface` bit for W003 to fire — without
    // it, string relational operators go unflagged.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: f5-tmsh\nif {$a lt $b} { puts hi }\n");
    assert!(has_code(&diags, "W003"), "{diags:?}");
}

// -- Dialect-profile availability (dialect-profile-model.md): vendor
// dialects resolve their embedded Tcl core end-to-end — server →
// JSON-RPC → publishDiagnostics — via the composed (version|vendor) profile
// masks; the version ladder and the restricted iRules view still hold.

#[test]
fn iapps_embedded_84_trunk_gates_the_85_core_end_to_end() {
    // iApps ride the `f5-tcl` trunk — a fork of Tcl at 8.4.6 (measured,
    // bigip-irule-parser-measurements.md §4a: `dict`, `lassign` and
    // `apply` all fail in `IAppImplementation`) — so the 8.5 core draws
    // the availability diagnostics, while the 8.4 core stays clean.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("iapp");
    let diags = lsp.open_ready_lang(
        &uri,
        "dict set cfg pool p1\nlassign {a b} x y\napply {{v} {return $v}} 1\n",
        "tcl-iapp",
    );
    assert!(
        has_code(&diags, "W123") || has_code(&diags, "W002"),
        "iApps (8.4.6 trunk, measured) must gate the 8.5 core: {:?}",
        codes(&diags)
    );
    let uri2 = unique_uri("iapp");
    let diags2 = lsp.open_ready_lang(&uri2, "lappend cfg p1\nexec ls\n", "tcl-iapp");
    assert!(
        !has_code(&diags2, "W123") && !has_code(&diags2, "W002"),
        "the 8.4 core and the host's `exec` stay clean: {:?}",
        codes(&diags2)
    );
}

#[test]
fn iapps_86_core_still_draws_w123_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("iapp");
    let diags = lsp.open_ready_lang(&uri, "lmap x {1 2} {set x}\n", "tcl-iapp");
    assert!(
        has_code(&diags, "W123"),
        "lmap is 8.6 — unavailable on the iApps 8.5 base: {:?}",
        codes(&diags)
    );
}

#[test]
fn expect_embedded_86_core_is_clean_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("exp");
    let diags = lsp.open_ready_lang(
        &uri,
        "spawn ssh host\ndict set cfg k v\ncoroutine pump ::apply {{} {}}\n",
        "tcl-expect",
    );
    assert!(
        !has_code(&diags, "W123") && !has_code(&diags, "W002"),
        "expect (Tcl 8.6 base) must resolve 8.6 core + expect surface cleanly: {:?}",
        codes(&diags)
    );
}

#[test]
fn irules_banned_commands_still_flag_end_to_end() {
    // The iRules profile is unchanged by the composed-mask fix: 8.4 core that
    // carries no iRules row still draws W002 in a `when` handler.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n  exec /bin/true\n}\n",
        "tcl-irule",
    );
    assert!(
        has_code(&diags, "W002"),
        "exec is banned in iRules: {:?}",
        codes(&diags)
    );
}

#[test]
fn argument_dsl_rung_gates_format_and_string_is_end_to_end() {
    // §6: the argument mini-languages validate against the
    // dialect's effective Tcl version. iRules embed Tcl 8.4.6, so the
    // 8.6-introduced `format %b` and the 9.0-introduced `string is dict`
    // both flag; the same source is clean on a 9.0 host.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n  set x [format %b 5]\n  set y [string is dict {a 1}]\n}\n",
        "tcl-irule",
    );
    assert!(
        has_code(&diags, "W138") && has_code(&diags, "W137"),
        "8.4-embedded iRules must flag %b (8.6+) and the dict class (9.0+): {:?}",
        codes(&diags)
    );
    let uri2 = unique_uri("tcl90");
    let diags2 = lsp.open_ready_lang(
        &uri2,
        "set x [format %b 5]\nset y [string is dict {a 1}]\n",
        "tcl9.0",
    );
    assert!(
        !has_code(&diags2, "W138") && !has_code(&diags2, "W137"),
        "a 9.0 host accepts both: {:?}",
        codes(&diags2)
    );
}

#[test]
fn tmsh_first_class_gates_both_directions_end_to_end() {
    // D8, corrected by measurement (bigip-irule-parser-measurements.md
    // §4a): a tmsh document analyses under TCL84|TMSH — the tmsh::
    // surface and the 8.4 core are clean, while the 8.5+ core draws the
    // availability diagnostics (a tmsh `cli script` fails every 8.5
    // discriminator, `dict` included).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tmsh");
    let diags = lsp.open_ready_lang(
        &uri,
        "tmsh::create ltm pool p1\nlappend cfg v\n",
        "tcl-tmsh",
    );
    assert!(
        !has_code(&diags, "W123") && !has_code(&diags, "W002"),
        "tmsh surface + 8.4 core must be clean: {:?}",
        codes(&diags)
    );
    let uri2 = unique_uri("tmsh");
    let diags2 = lsp.open_ready_lang(&uri2, "dict set cfg k v\n", "tcl-tmsh");
    assert!(
        has_code(&diags2, "W123") || has_code(&diags2, "W002"),
        "dict is 8.5 — unavailable on the tmsh 8.4.6 trunk: {:?}",
        codes(&diags2)
    );
}

#[test]
fn bpf_first_class_admits_90_core_end_to_end() {
    // bpf = TCL90|BPF — 9.0-era core (lmap, dict) is
    // real, and the bpf surface resolves.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("bpf");
    let diags = lsp.open_ready_lang(&uri, "dict set cfg k v\nlmap x {1 2} {set x}\n", "tcl-bpf");
    assert!(
        !has_code(&diags, "W123") && !has_code(&diags, "W002"),
        "9.0 core is real under bpf: {:?}",
        codes(&diags)
    );
}

#[test]
fn irules_subcommands_named_like_banned_commands_are_clean_end_to_end() {
    // `DNS::header cd` (the DNS Checking-Disabled flag) and `IP::stats in`
    // (inbound stats) are real iRules subcommands whose names collide with
    // the banned `cd` command and the `in` operator spelling. The old bulk
    // name-keyed tagging hid them and drew
    // spurious availability/subcommand diagnostics; exclusion is keyed on
    // the resolved spec now, never on a bare name.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "when DNS_REQUEST {\n  DNS::header cd\n}\nwhen CLIENT_ACCEPTED {\n  IP::stats in\n}\n",
        "tcl-irule",
    );
    assert!(
        !has_code(&diags, "W001") && !has_code(&diags, "W002") && !has_code(&diags, "W123"),
        "real iRules subcommands must not be hidden by name collisions: {:?}",
        codes(&diags)
    );
}

// Every built-in `expr` math function (`sin(...)`, `max(...)`, ...) must
// not fire W123. `expr_function_call_records_a_mathfunc_invocation`
// (commands.rs) resolves the call to `::tcl::mathfunc::<name>`; the W123
// pass must recognise that qualified name even when no same-named user
// proc shadows it.

#[test]
fn issue_968_builtin_expr_math_functions_are_clean_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [expr {sin(1.0) + max(1, 2, 3)}]\nputs $x\n");
    assert!(
        !has_code(&diags, "W123"),
        "issue #968: built-in expr math functions must not draw W123: {:?}",
        codes(&diags)
    );
}

#[test]
fn issue_968_nested_expr_math_functions_are_clean_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [expr {sqrt(abs($y))}]\n");
    assert!(
        !has_code(&diags, "W123"),
        "nested built-in expr math functions must not draw W123: {:?}",
        codes(&diags)
    );
}

#[test]
fn issue_968_unknown_expr_function_still_w123_end_to_end() {
    // Control: the fix must not swallow a genuinely unknown function name.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [expr {frobnicate(1)}]\n");
    assert!(
        has_code(&diags, "W123"),
        "an unknown expr function must still draw W123: {:?}",
        codes(&diags)
    );
    assert!(
        diags.iter().any(|d| message(d).contains("frobnicate")),
        "message names the unknown function: {diags:?}",
    );
}

#[test]
fn issue_968_version_gated_math_function_dual_fires_end_to_end() {
    // `min`/`max` are TIP 232 (Tcl 8.5+) — under an 8.4 document the name
    // is a real function elsewhere but disabled here, so both the
    // dialect-availability diagnostic (W002) and the unresolved-command
    // diagnostic (W123) fire, matching the established dual-fire precedent
    // for a dialect-disabled registry builtin (e.g. `lmap` under an 8.5
    // host, exercised by `iapps_86_core_still_draws_w123_end_to_end` above).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl84");
    let diags = lsp.open_ready_lang(&uri, "set x [expr {min(1, 2)}]\n", "tcl8.4");
    assert!(
        has_code(&diags, "W002") && has_code(&diags, "W123"),
        "min() under tcl8.4 must draw both W002 and W123: {:?}",
        codes(&diags)
    );

    let uri2 = unique_uri("tcl86");
    let diags2 = lsp.open_ready_lang(&uri2, "set x [expr {min(1, 2)}]\n", "tcl8.6");
    assert!(
        !has_code(&diags2, "W002") && !has_code(&diags2, "W123"),
        "min() under tcl8.6 must be clean: {:?}",
        codes(&diags2)
    );
}

/// The **plain-command** spelling of a math function,
/// `tcl::mathfunc::isinf`, is a real, directly-callable, version-gated
/// command and not only an `expr` production. The registry carries it, so
/// the generic per-dispatch-site availability check fires for free; this pins
/// that it really reaches the wire, which the registry-level unit test
/// (`mathfunc_command_spellings_are_registered_and_gated`) cannot show.
///
/// Oracle:
///
/// ```text
/// $ tclsh8.6 -c 'set r [tcl::mathfunc::isinf 1.0]'
/// invalid command name "tcl::mathfunc::isinf"     (exit 1)
/// $ tclsh9.0 -c 'puts [tcl::mathfunc::isinf 1.0]'
/// 0                                                (exit 0)
/// ```
#[test]
fn issue_923_idx103_plain_command_mathfunc_is_dialect_gated_end_to_end() {
    let mut lsp = Lsp::tcl();
    let src = "set r [tcl::mathfunc::isinf 1.0]\n";

    let uri86 = unique_uri("idx103-86");
    let diags86 = lsp.open_ready_lang(&uri86, src, "tcl8.6");
    assert!(
        has_code(&diags86, "W002"),
        "`isinf` arrived in 9.0; calling `tcl::mathfunc::isinf` under an 8.6 \
         document is a real `invalid command name` and must draw W002: {:?}",
        codes(&diags86)
    );
    assert!(
        diags86
            .iter()
            .any(|d| code_str(d).as_deref() == Some("W002")
                && message(d).contains("tcl::mathfunc::isinf")),
        "the W002 must name the qualified command it gated: {diags86:?}",
    );

    let uri90 = unique_uri("idx103-90");
    let diags90 = lsp.open_ready_lang(&uri90, src, "tcl9.0");
    assert!(
        !has_code(&diags90, "W002") && !has_code(&diags90, "W123"),
        "under tcl9.0 the same call is valid and must be clean: {:?}",
        codes(&diags90)
    );

    // TN control: the `expr` spelling of the same function is gated
    // identically, so the two spellings cannot drift apart.
    let uri_expr = unique_uri("idx103-expr86");
    let diags_expr = lsp.open_ready_lang(&uri_expr, "set r [expr {isinf(1.0)}]\n", "tcl8.6");
    assert!(
        has_code(&diags_expr, "W002"),
        "the expr spelling must stay gated too: {:?}",
        codes(&diags_expr)
    );
}

/// The VS Code extension contributes *undotted* version-pinned language ids
/// (`tcl84` … `tcl91`) — a dotted id cannot carry a `configurationDefaults`
/// override. The server must resolve them to the same dialects
/// the dotted forms resolve to, while still accepting the dotted ids the other
/// editor integrations send.
#[test]
fn undotted_vscode_language_ids_resolve_to_the_same_dialect_end_to_end() {
    let mut lsp = Lsp::tcl();
    for language_id in ["tcl84", "tcl8.4"] {
        let uri = unique_uri("undotted84");
        let diags = lsp.open_ready_lang(&uri, "set x [expr {min(1, 2)}]\n", language_id);
        assert!(
            has_code(&diags, "W002") && has_code(&diags, "W123"),
            "min() under languageId {language_id} must resolve to the 8.4 dialect \
             and draw both W002 and W123: {:?}",
            codes(&diags)
        );
    }
    for language_id in ["tcl86", "tcl8.6"] {
        let uri = unique_uri("undotted86");
        let diags = lsp.open_ready_lang(&uri, "set x [expr {min(1, 2)}]\n", language_id);
        assert!(
            !has_code(&diags, "W002") && !has_code(&diags, "W123"),
            "min() under languageId {language_id} must resolve to the 8.6 dialect \
             and stay clean: {:?}",
            codes(&diags)
        );
    }
    for language_id in ["tcl90", "tcl9.0"] {
        let uri = unique_uri("undotted90");
        let diags = lsp.open_ready_lang(&uri, "set x [expr {min(1, 2)}]\n", language_id);
        assert!(
            !has_code(&diags, "W002") && !has_code(&diags, "W123"),
            "min() under languageId {language_id} must resolve to the 9.0 dialect \
             and stay clean: {:?}",
            codes(&diags)
        );
    }
}

#[test]
fn issue_968_expr_function_after_rename_away_still_w123_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "rename ::tcl::mathfunc::sin {}\nset x [expr {sin(1.0)}]\n",
    );
    assert!(
        has_code(&diags, "W123"),
        "a call through a renamed-away mathfunc must be unresolved: {:?}",
        codes(&diags)
    );
}

#[test]
fn issue_968_user_defined_mathfunc_override_still_resolves_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc ::tcl::mathfunc::myfunc {x} { return $x }\nset r [expr {myfunc(1)}]\n",
    );
    assert!(
        !has_code(&diags, "W123"),
        "a user-defined mathfunc override must resolve end-to-end: {:?}",
        codes(&diags)
    );
}
