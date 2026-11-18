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

//! Dialect profiles: the commands, subcommands, options and operators each vendor profile admits, and Expect clause roles.

use super::*;

// Dialect-profile availability (dialect-profile-model.md): the
// composed (version|vendor) masks admit each vendor dialect's embedded Tcl
// core, the version ladder still gates later-version core, iRules stays
// subtractive, and unknown dialects stay permissive.

#[test]
fn w123_vendor_profiles_admit_their_embedded_tcl_core() {
    // FP-fix (the confirmed bare-bit defect): real embedded-core commands
    // must resolve cleanly — no W123 (unknown) and no W002 (disabled).
    let clean: &[(&str, &str)] = &[
        // F5 reclassification (measurements §4/§4a,
        // `docs/design/f5/bigip-irule-parser-measurements.md`): the iApps
        // host is the 8.4.6 fork, not an 8.5.13 one — its real core is the
        // 8.4 line…
        ("string tolower ABC", "f5-iapps"),
        ("array exists a", "f5-iapps"),
        // …and the scriptd host is NOT the TMM sandbox: exec is measured
        // real there (measurements §4a).
        ("exec /bin/true", "f5-iapps"),
        // Expect embeds Tcl 8.6: 8.5 and 8.6 core are real.
        ("dict get {a 1} a", "expect"),
        ("lmap x {1 2} {set x}", "expect"),
        ("coroutine c ::apply {{} {}}", "expect"),
        // EDA shells: 8.5 base (xilinx) and 8.6 base (synopsys).
        ("dict get {a 1} a", "xilinx-eda-tcl"),
        ("lmap x {1 2} {set x}", "synopsys-eda-tcl"),
    ];
    for (snippet, dialect) in clean {
        let codes = codes_for_dialect(snippet, dialect);
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "{dialect}: {snippet:?} is embedded-core and must not flag, got {codes:?}"
        );
    }
    // F5 reclassification (measurements §4, table "No 8.5 features
    // anywhere"): every 8.4/8.5 discriminator behaves as 8.4 in the iApp
    // context — `dict`, `lassign`, `apply` measured absent — so the old
    // 8.5-core rows now correctly flag.
    for gated in [
        "dict get {a 1} a",
        "lassign {1 2} a b",
        "apply {{x} {return $x}} 1",
    ] {
        let codes = codes_for_dialect(gated, "f5-iapps");
        assert!(
            codes.iter().any(|c| c == "W123" || c == "W002"),
            "f5-iapps: {gated:?} is 8.5+ core on a measured 8.4.6 fork and must flag, got {codes:?}"
        );
    }
}

#[test]
fn w123_vendor_profiles_still_gate_later_version_core() {
    // TN: the composed mask is the *embedded base* version, not a blanket
    // allow — core introduced after the base still flags.
    let flagged: &[(&str, &str)] = &[
        // lmap is 8.6; iApps embed 8.5.
        ("lmap x {1 2} {set x}", "f5-iapps"),
        ("coroutine c ::apply {{} {}}", "f5-iapps"),
        // lmap is 8.6; xilinx embeds 8.5.
        ("lmap x {1 2} {set x}", "xilinx-eda-tcl"),
        // zipfs is 9.0; expect embeds 8.6.
        ("zipfs root", "expect"),
        ("zipfs root", "synopsys-eda-tcl"),
    ];
    for (snippet, dialect) in flagged {
        let codes = codes_for_dialect(snippet, dialect);
        assert!(
            codes.iter().any(|c| c == "W123"),
            "{dialect}: {snippet:?} is post-base core and must draw W123, got {codes:?}"
        );
    }
}

#[test]
fn w123_vendor_profiles_still_flag_genuinely_unknown_commands() {
    // TP: the widened masks must not swallow real unknowns.
    for dialect in ["f5-iapps", "expect", "xilinx-eda-tcl"] {
        assert!(
            has_code("frobnicate_no_such_cmd a b\n", dialect, "W123"),
            "{dialect}: genuinely unknown command must still draw W123"
        );
    }
}

/// measurements §4b: the 31 iRules-"disabled" stock builtins are **two
/// mechanisms, not one**. The 16 interpreter-absent commands (`invalid
/// command name` even via `eval` at runtime) are a *language fact* and keep
/// the existing unavailable-command diagnostic (W002, with W123); the 15
/// compiler-refused commands are present in TMM's interpreter — reachable
/// via `eval`, `rename` demonstrably works — so a literal use draws the
/// distinct IRULE2004 *policy* warning instead, and the analyser never
/// claims the command does not exist.
#[test]
fn irules_4b_disabled_split_pins_two_diagnostic_severities() {
    for absent in tcl_registry::irules_policy::IRULES_INTERPRETER_ABSENT {
        let src = format!("when RULE_INIT {{ {absent} }}");
        let codes = codes_for_dialect(&src, "f5-irules");
        assert!(
            codes.iter().any(|c| c == "W002"),
            "{absent}: interpreter-absent keeps the language-fact W002, got {codes:?}"
        );
        assert!(
            !codes.iter().any(|c| c == "IRULE2004"),
            "{absent}: interpreter-absent is not a policy refusal, got {codes:?}"
        );
    }
    for refused in tcl_registry::irules_policy::IRULES_COMPILER_REFUSED {
        let src = format!("when RULE_INIT {{ {refused} }}");
        let codes = codes_for_dialect(&src, "f5-irules");
        assert!(
            codes.iter().any(|c| c == "IRULE2004"),
            "{refused}: compiler-refused draws the distinct policy warning, got {codes:?}"
        );
        assert!(
            !codes.iter().any(|c| c == "W002" || c == "W123"),
            "{refused}: present in TMM's interpreter — no W002 and no \
             'Unknown command' claim, got {codes:?}"
        );
    }
}

/// The §4b policy warning follows the §4c lexical scan: a compiler-refused
/// command written literally inside a braced `eval` body is still literal
/// rule source (the rule compiler scans braced script literals), while a
/// name reached only through dynamic `eval` (variable-held) is not flagged
/// — it works at runtime.
#[test]
fn irules_4b_policy_warning_follows_the_4c_lexical_scan() {
    let braced = "when RULE_INIT { eval {rename a b} }";
    let codes = codes_for_dialect(braced, "f5-irules");
    assert!(
        codes.iter().any(|c| c == "IRULE2004"),
        "a braced eval literal is scanned rule source, got {codes:?}"
    );
    let hidden = "when RULE_INIT { set c \"rename\"\n  uplevel #0 $c a b }";
    let codes = codes_for_dialect(hidden, "f5-irules");
    assert!(
        !codes.iter().any(|c| c == "IRULE2004" || c == "W002"),
        "a variable-held name is dynamic eval — reachable at runtime, \
         not flagged, got {codes:?}"
    );
}

#[test]
fn irules_stays_subtractive_under_the_profile() {
    // The subtractive-iRules trap (§9): the profile keeps the bare IRULES
    // mask + disable list, so nothing moves for iRules files.
    // Banned 8.4 core: exists elsewhere → W002 (and W123 both fire).
    for banned in ["exec /bin/true", "file exists /tmp", "socket -server x 80"] {
        let codes = codes_for_dialect(banned, "f5-irules");
        assert!(
            codes.iter().any(|c| c == "W002"),
            "f5-irules: {banned:?} is banned and must draw W002, got {codes:?}"
        );
    }
    // 8.5+/8.6 core: never present at ANY BIG-IP version.
    for versioned in ["dict get {a 1} a", "lmap x {1 2} {set x}"] {
        let codes = codes_for_dialect(versioned, "f5-irules");
        assert!(
            codes.iter().any(|c| c == "W123" || c == "W002"),
            "f5-irules: {versioned:?} must stay unavailable, got {codes:?}"
        );
    }
    // The universal core and F5 surface stay clean.
    for ok in ["set x 1", "log local0. hi"] {
        let codes = codes_for_dialect(ok, "f5-irules");
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "f5-irules: {ok:?} must stay clean, got {codes:?}"
        );
    }
}

#[test]
fn irules_alias_dialect_string_behaves_like_canonical() {
    // §2.4 alias canonicalisation: the legacy "irules" spelling resolves
    // through the profile catalogue like f5-irules, rather than falling
    // through to the permissive plain-Tcl view (a silent false negative).
    let codes = codes_for_dialect("exec /bin/true", "irules");
    assert!(
        codes.iter().any(|c| c == "W002"),
        "alias 'irules' must ban exec like f5-irules, got {codes:?}"
    );
    assert!(
        !has_code("set x 1", "irules", "W123"),
        "universal core stays clean under the alias"
    );
}

#[test]
fn unknown_dialect_strings_stay_permissive() {
    // §8: the PLAIN_TCL sink — a typo'd dialect must flag nothing.
    for snippet in ["dict get {a 1} a", "zipfs root", "exec /bin/true"] {
        let codes = codes_for_dialect(snippet, "definitely-not-a-dialect");
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "unknown dialect must stay permissive for {snippet:?}, got {codes:?}"
        );
    }
}

#[test]
fn w001_subcommand_checks_use_the_profile_mask() {
    // Subcommand-level: an 8.4-core ensemble's valid subcommands must not
    // draw the W001/W002 subcommand diagnostics under the vendor mask.
    // (F5 reclassification, measurements §4/§4a: the iApps host is the 8.4.6
    // fork, so this probes `string`, which the fork's 8.4 core really has,
    // rather than an 8.5-only command.)
    let codes = codes_for_dialect("string tolower ABC", "f5-iapps");
    assert!(
        !codes.iter().any(|c| c == "W001" || c == "W002"),
        "string tolower is 8.4-valid under f5-iapps, got {codes:?}"
    );
    // A genuinely unknown subcommand still fires W001 under the vendor mask.
    assert!(
        has_code("string zzznotasub x", "f5-iapps", "W001"),
        "unknown string subcommand must still draw W001 under f5-iapps"
    );
}

#[test]
fn tmsh_first_class_resolves_its_surface_and_gates_later_core() {
    // F5 reclassification (measurements §4a,
    // `docs/design/f5/bigip-irule-parser-measurements.md`): the
    // "TCL85|TMSH" hypothesis is falsified — `TmshCliScript` reports
    // patchlevel 8.4.6 and fails every 8.5 discriminator — so f5-tmsh is
    // the `f5-tcl` fork's 8.4 line plus the tmsh:: surface.
    // TP: the tmsh:: surface draws no unknown-command.
    for ok in [
        "tmsh::create ltm pool p1",
        "tmsh::log local0.info \"hi\"",
        "tmsh::list ltm virtual",
    ] {
        let codes = codes_for_dialect(ok, "f5-tmsh");
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "f5-tmsh: {ok:?} is the tmsh surface, got {codes:?}"
        );
    }
    // TN: the fork's 8.4 core is real.
    for ok in ["string tolower ABC", "array exists a", "catch {set x 1} e"] {
        let codes = codes_for_dialect(ok, "f5-tmsh");
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "f5-tmsh: {ok:?} is 8.4 core, got {codes:?}"
        );
    }
    // Reverse case, measurement-backed (measurements §4: all
    // sixteen 8.4/8.5 discriminators behave as 8.4 in tmsh — `dict`,
    // `lassign`, `apply` included): 8.5+ core is unknown on the fork.
    for gated in [
        "dict get {a 1} a",
        "lassign {1 2} a b",
        "apply {{x} {return $x}} 1",
        "lmap x {1 2} {set x}",
        "coroutine c ::apply {{} {}}",
        "zipfs root",
    ] {
        let codes = codes_for_dialect(gated, "f5-tmsh");
        assert!(
            codes.iter().any(|c| c == "W123" || c == "W002"),
            "f5-tmsh: {gated:?} is later-than-8.4 core and must flag, got {codes:?}"
        );
    }
    // FP-guard: the iApp-only surface is NOT part of the tmsh shell — it
    // resolves nowhere under TCL85|TMSH, drawing the disabled-in-dialect
    // diagnostic (the spec exists, in the *iApps* pack).
    let codes = codes_for_dialect("iapp::conf save\n", "f5-tmsh");
    assert!(
        codes.iter().any(|c| c == "W002" || c == "W123"),
        "iapp:: is the iApps host surface, not tmsh: {codes:?}"
    );
}

#[test]
fn bpf_precise_mask_keeps_90_core_and_drops_8x_relics() {
    // bpf = TCL90|BPF — a genuine Tcl 9.0 base.
    // TN: 9.0 core (including 8.5/8.6 additions carried into 9.0) resolves.
    for ok in [
        "dict get {a 1} a",
        "lmap x {1 2} {set x}",
        "coroutine c ::apply {{} {}}",
        "zipfs root",
    ] {
        let codes = codes_for_dialect(ok, "bpf");
        assert!(
            !codes.iter().any(|c| c == "W123" || c == "W002"),
            "bpf: {ok:?} is real on the 9.0 base, got {codes:?}"
        );
    }
    // TP (reverse case): 8.x-only relics removed at the 9.0 boundary are
    // unknown here — an ALL_TCL|BPF mask would wrongly admit them.
    for relic in ["tcltest::bytestring x", "case $x in a {puts hi}"] {
        let codes = codes_for_dialect(relic, "bpf");
        assert!(
            codes.iter().any(|c| c == "W123" || c == "W002"),
            "bpf: {relic:?} was removed at the 9.0 boundary, got {codes:?}"
        );
    }
}

#[test]
fn irules_subcommands_named_like_banned_commands_resolve_cleanly() {
    // `DNS::header cd` (the DNS
    // Checking-Disabled flag) and `IP::stats in` (inbound stats) are real
    // iRules subcommands that were bulk mis-tagged by name collision with
    // the banned `cd` command and the `in` operator spelling, and so drew
    // spurious subcommand diagnostics. Exclusion is keyed on the resolved
    // spec, never on a bare name.
    for snippet in ["DNS::header cd", "IP::stats in"] {
        let codes = codes_for_dialect(snippet, "f5-irules");
        assert!(
            !codes
                .iter()
                .any(|c| c == "W001" || c == "W002" || c == "W123"),
            "f5-irules: {snippet:?} is a real subcommand and must stay \
             clean, got {codes:?}"
        );
    }
    // TP retained: a genuinely unknown subcommand of the same commands
    // still flags.
    assert!(
        has_code("DNS::header zzznotasub", "f5-irules", "W001"),
        "unknown DNS::header subcommand must still draw W001"
    );
}

// Behaviour axis (dialect-profile-model.md): the expr grammar,
// mathfunc tiers, and octal policy resolve through the profile — including
// alias canonicalisation the string-keyed tables missed.

#[test]
fn w003_irules_alias_gates_like_the_canonical_profile() {
    // §2.4: `expr_grammar_base_version` had no arm for the legacy "irules"
    // spelling, so W003 silently never fired there — a false negative the
    // profile's alias canonicalisation fixes. Both TIPs gate on the 8.4
    // runtime, exactly as for "f5-irules".
    assert_eq!(w003_hits("expr {2 in {1 2 3}}", "irules").len(), 1);
    assert_eq!(w003_hits("if {$x lt $y} { puts hi }", "irules").len(), 1);
}

#[test]
fn w003_bpf_accepts_both_tips_on_its_tcl_9_runtime() {
    // bpf embeds Tcl 9.0: `in`/`ni` (TIP 201) and `lt`/`le`/`gt`/`ge`
    // (TIP 461) are all grammatical — no W003.
    assert_eq!(
        w003_hits("expr {2 in {1 2 3}}", "bpf"),
        [] as [(std::string::String, crate::analyser::types::Diagnostic); 0]
    );
    assert_eq!(
        w003_hits("if {$x lt $y} { puts hi }", "bpf"),
        [] as [(std::string::String, crate::analyser::types::Diagnostic); 0]
    );
}

/// The 9 iRules word operators (`contains`, `and`, …) evaluate outside the
/// iRules dialect — the lexer's word-operator recognition, the parser, and the
/// runtime evaluator (`tcl_expr_eval.rs`'s `apply_irules_string_op`) all treat
/// them as valid regardless of dialect, so `if {$x contains "foo"}` runs (and
/// misbehaves, since core Tcl has no such operator) in every non-iRules
/// dialect. W003 flags every one of them, the same family that flags TIP 201
/// (`in`/`ni`) and TIP 461 (`lt`/`le`/`gt`/`ge`).
#[test]
fn w003_fires_on_irules_word_operators_outside_irules() {
    for (src, op) in [
        (r#"if {$x contains "foo"} { puts hi }"#, "contains"),
        (r#"if {$x starts_with "foo"} { puts hi }"#, "starts_with"),
        (r#"if {$x ends_with "foo"} { puts hi }"#, "ends_with"),
        (r#"if {$x equals "foo"} { puts hi }"#, "equals"),
        (r#"if {$x matches_glob "foo*"} { puts hi }"#, "matches_glob"),
        (
            r#"if {$x matches_regex "foo.*"} { puts hi }"#,
            "matches_regex",
        ),
        ("if {$a and $b} { puts hi }", "and"),
        ("if {$a or $b} { puts hi }", "or"),
        ("if {not $a} { puts hi }", "not"),
    ] {
        let hits = w003_hits(src, "tcl9.0");
        assert_eq!(hits.len(), 1, "{src}: {hits:?}");
        assert_eq!(hits[0].0, op, "{src}");
        assert!(
            hits[0].1.message.contains("iRules"),
            "{}: {}",
            src,
            hits[0].1.message
        );
        // No safe mechanical rewrite exists for these — unlike `in`/`lt`,
        // there is no portable Tcl expression these fold into.
        assert_eq!(hits[0].1.fixes.len(), 0, "{src}");
    }
}

#[test]
fn w003_silent_on_irules_word_operators_inside_irules() {
    for src in [
        r#"if {$x contains "foo"} { puts hi }"#,
        r#"if {$x starts_with "foo"} { puts hi }"#,
        r#"if {$x ends_with "foo"} { puts hi }"#,
        r#"if {$x equals "foo"} { puts hi }"#,
        r#"if {$x matches_glob "foo*"} { puts hi }"#,
        r#"if {$x matches_regex "foo.*"} { puts hi }"#,
        "if {$a and $b} { puts hi }",
        "if {$a or $b} { puts hi }",
        "if {not $a} { puts hi }",
    ] {
        assert_eq!(w003_hits(src, "f5-irules").len(), 0, "{src}");
    }
}

#[test]
fn w003_fires_on_irules_word_operator_in_unbraced_multiword_expr() {
    // Exercises `emit_w003_dialect_invalid_expr_words`, the sibling path for
    // `expr`'s unbraced multi-word form (see
    // `w003_fires_on_unbraced_multiword_expr_at_a_tight_span` above).
    let hits = w003_hits("expr $a contains $b", "tcl9.0");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "contains");
    assert_eq!(
        w003_hits("expr $a contains $b", "f5-irules"),
        [] as [(std::string::String, crate::analyser::types::Diagnostic); 0]
    );
}

// Option-gating semantics (dialect-profile-model.md §5.2):
// intersects membership + version ceiling.  A plain `contains` rule silently
// drops inherited vendor options and never gates a version-ceiling leak.

#[test]
fn w004_version_gated_options_follow_the_profile_ceiling() {
    // TP: switch -nocase (8.5+) flags on the 8.4 base…
    assert!(
        has_code("switch -nocase a {a {} default {}}", "tcl8.4", "W004"),
        "switch -nocase is 8.5+ and must draw W004 under tcl8.4"
    );
    // …including the pinned-8.4 iRules runtime.
    assert!(
        has_code("switch -nocase a {a {} default {}}", "f5-irules", "W004"),
        "switch -nocase must draw W004 under f5-irules (8.4 base)"
    );
    // FP guard: it is clean at/above 8.5 — the composed vendor profiles
    // included (a plain contains rule could never satisfy a composed mask).
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "expect"] {
        assert!(
            !has_code("switch -nocase a {a {} default {}}", dialect, "W004"),
            "{dialect}: switch -nocase is real 8.5+ core"
        );
    }
    // F5 reclassification (measurements §4/§4a): f5-iapps is not in this list
    // — its host is the 8.4.6 fork, so the 8.5+ option correctly flags
    // there, exactly as it does under f5-irules.
    assert!(
        has_code("switch -nocase a {a {} default {}}", "f5-iapps", "W004"),
        "f5-iapps: switch -nocase is 8.5+ on a measured 8.4.6 fork"
    );
}

#[test]
fn w004_later_version_options_never_leak_into_supersets() {
    // regsub -command is 9.0+: clean there, flagged below — for plain
    // versions and composed vendor profiles alike (§5.2's ceiling guard).
    assert!(!has_code(
        r#"regsub -command {x} "axb" {string toupper}"#,
        "tcl9.0",
        "W004"
    ));
    for dialect in ["tcl8.6", "f5-iapps", "expect"] {
        assert!(
            has_code(
                r#"regsub -command {x} "axb" {string toupper}"#,
                dialect,
                "W004"
            ),
            "{dialect}: regsub -command is 9.0-only and must draw W004"
        );
    }
}

#[test]
fn vendor_command_inherited_options_resolve_cleanly() {
    // expect_after's options inherit the command's EXPECT gate — under the
    // old contains rule every one of them was "unavailable" the moment the
    // active mask composed (version|vendor). No W004 under expect.
    let codes = codes_for_dialect("expect_after -re {pattern} {send ok}", "expect");
    assert!(
        !codes.iter().any(|c| c == "W004"),
        "inherited expect options must resolve under expect, got {codes:?}"
    );
}

#[test]
fn expect_clause_flags_keep_pattern_and_body_roles_aligned() {
    let codes = codes_for_dialect(
        "expect {-re {^ready$} {puts ready} -timeout 5 timeout {puts slow}}",
        "expect",
    );
    assert!(
        !codes.iter().any(|c| c == "W123"),
        "Expect clause flags must not make a body look like a command: {codes:?}"
    );
}

#[test]
fn nested_expect_clause_body_reports_unknown_command() {
    let codes = codes_for_dialect("expect {ready {expect_body_unknown_command}}", "expect");
    assert!(
        codes.iter().any(|c| c == "W123"),
        "Expect clause bodies must be analysed for unresolved commands: {codes:?}"
    );
}

#[test]
fn unbraced_expect_action_keeps_its_complete_script_span() {
    // `list` runs cleanly first; a bare `puts` would raise `wrong # args`
    // and leave the unknown command unreached (tclsh 8.4.20 to 9.1.0).
    let codes = codes_for_dialect("expect {ready list;expect_semicolon_unknown}", "expect");
    assert!(
        codes.iter().any(|code| code == "W123"),
        "the command after a semicolon in an unbraced Expect action must be analysed: {codes:?}"
    );
}

#[test]
fn backslash_built_expect_action_stays_an_opaque_barrier() {
    let codes = codes_for_dialect(
        r"expect {ready puts\ ok\;expect_backslash_unknown}",
        "expect",
    );
    assert!(
        !codes.iter().any(|code| code == "W123"),
        "a decoded list value has no 1:1 source span and must not be partially walked: {codes:?}"
    );
}

#[test]
fn expect_list_comment_and_separator_patterns_still_walk_arm_bodies() {
    for pattern in ["#", ";"] {
        let codes = codes_for_dialect(
            &format!("expect {{{pattern} {{expect_literal_pattern_unknown}} default {{puts ok}}}}"),
            "expect",
        );
        assert!(
            codes.iter().any(|code| code == "W123"),
            "{pattern:?} is a Tcl list pattern, and its arm must be analysed: {codes:?}"
        );
    }
}

#[test]
fn inline_expect_clause_flags_reach_actions_without_switch_w106() {
    let codes = codes_for_dialect(
        "expect -re {ready} {expect_inline_unknown} -timeout 5 timeout {expect_timeout_unknown}",
        "expect",
    );
    assert_eq!(
        codes.iter().filter(|code| code.as_str() == "W123").count(),
        2,
        "both inline Expect actions must be analysed: {codes:?}"
    );
    assert!(
        !codes.iter().any(|code| code == "W106"),
        "the switch-only bracing diagnostic must not apply to Expect: {codes:?}"
    );
}

#[test]
fn abbreviated_inline_expect_flag_reaches_action() {
    let codes = codes_for_dialect("expect -not ready {expect_abbrev_unknown}", "expect");
    assert!(
        codes.iter().any(|code| code == "W123"),
        "a unique Expect flag abbreviation must not hide its action: {codes:?}"
    );
}

#[test]
fn outer_expect_value_options_do_not_make_a_final_pattern_a_clause_list() {
    for source in [
        "expect -timeout 5 {ready {expect_outer_timeout_unknown}}",
        "expect -i spawn {ready {expect_outer_spawn_unknown}}",
    ] {
        let codes = codes_for_dialect(source, "expect");
        assert!(
            !codes.iter().any(|code| code == "W123"),
            "the braced final pattern is not an Expect action list: {source:?}, {codes:?}"
        );
    }
}
