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

//! W004 options the dialect excludes, with their removal fix, and the W304 missing option terminator.

use super::*;

#[test]
fn w004_fires_on_regsub_command_in_tcl86() {
    // `regsub -command` is Tcl 9.0+ (TIP 463); on Tcl 8.6 it
    // should produce a W004 dialect-availability warning.
    let mut a = Analyser::new();
    let result = a.analyse("regsub -command {[A-Z]+} foo {bar} out", "tcl8.6");
    let w004: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W004)
        .collect();
    assert!(
        !w004.is_empty(),
        "expected W004 on tcl8.6 regsub -command, got {:?}",
        result.diagnostics
    );
    assert!(w004[0].message.contains("-command"));
    assert!(w004[0].message.contains("regsub"));
}

/// The option scan takes a two-level ensemble's *operation* table, so the
/// message names the operation and the gate inherits through it.
///
/// `namespace ensemble` is 8.5+, and `-parameters` narrows further to 8.6+
/// (absent from the 8.5 `namespace.n` ENSEMBLE OPTIONS list), so the scan has
/// to reach an option declared two levels down and still resolve both gates.
#[test]
fn w004_names_the_ensemble_operation_whose_option_table_answered() {
    let message_of = |src: &str, dialect: &str| -> String {
        let result = Analyser::new().analyse(src, dialect);
        result
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W004)
            .map_or_else(
                || panic!("expected W004 on {src:?} @ {dialect}: {result:?}"),
                |d| d.message.clone(),
            )
    };

    let created = message_of("namespace ensemble create -parameters arg", "tcl8.5");
    assert!(
        created.contains("'namespace' ensemble create") && created.contains("-parameters"),
        "{created}"
    );
    let configured = message_of("namespace ensemble configure ::E -parameters arg", "tcl8.5");
    assert!(
        configured.contains("'namespace' ensemble configure") && configured.contains("-parameters"),
        "{configured}"
    );

    // The operation table must not turn every option into a warning either:
    // `-parameters` is fine from 8.6, and `-map` from 8.5 (when `namespace
    // ensemble` itself arrived), under both operations.
    for src in [
        "namespace ensemble create -parameters arg",
        "namespace ensemble configure ::E -parameters arg",
        "namespace ensemble create -map {}",
        "namespace ensemble configure ::E -map {}",
    ] {
        assert_eq!(
            count_code_in(src, "W004", "tcl8.6"),
            0,
            "no option here is gated out of 8.6: {src}"
        );
    }

    // Dynamic nested selection has no selected positional prefix/table.
    // It cannot skip the dispatch word and borrow the parent's union options.
    assert_eq!(
        count_code_in("namespace ensemble $op -parameters arg", "W004", "tcl8.5"),
        0
    );
}

/// The tables really are different: `-command` is create-only and
/// `-namespace` configure-only, so neither is silently accepted on the other.
#[test]
fn the_two_ensemble_option_tables_do_not_share_their_distinctive_options() {
    let reg =
        std::sync::Arc::clone(tcl_registry::model::ingress::static_context_for("tcl").commands());
    let spec = reg.get("namespace").expect("`namespace` is a core command");
    let ensemble = spec
        .resolve_subcommand("ensemble")
        .expect("`namespace ensemble`");
    let names = |op: &str| -> Vec<&'static str> {
        ensemble
            .option_scope(Some(op), None, None, spec.surface)
            .options
            .iter()
            .map(|o| o.name)
            .collect()
    };
    // tclsh 8.6.16 / 9.0.4, byte identical: `configure ::E -command x` and
    // `create -namespace ::M` are each `bad option`, and the "must be" list
    // in each message is exactly the other five plus that table's own.
    assert!(names("create").contains(&"-command") && !names("create").contains(&"-namespace"));
    assert!(
        names("configure").contains(&"-namespace") && !names("configure").contains(&"-command")
    );
    // Tcl's unique-prefix rule reaches the operation word too — `conf`
    // resolves to `configure` (tclsh-confirmed) and so must select its table.
    assert_eq!(names("conf"), names("configure"));
    // `exists` takes no options *at all* and declares that explicitly, so it
    // does not inherit the union. `ENS_EXISTS` (`tclEnsemble.c`) takes
    // `cmdname` and never reaches an option table; pinned on tclsh 8.6.16 and
    // 9.0.4, byte identical, `namespace ensemble exists -namespace` answers
    // `0` — reading the flag as the command name, not rejecting it as an
    // option.
    assert_eq!(
        names("exists").len(),
        0,
        "`exists` must not inherit the parent union: {:?}",
        names("exists")
    );
    // The union still applies where it is the right answer: a dispatch word no
    // consumer can resolve abstains to the subcommand's own, wider table
    // rather than guessing which operation was meant.
    let union = names("$op");
    assert!(union.contains(&"-command") && union.contains(&"-namespace"));
}

/// The option scan consumes a value word only for options the *operation*
/// really has.
///
/// tclsh 8.6.16 / 9.0.4: `namespace ensemble configure ::E -command x` is
/// `bad option "-command"` — `-command` is not a `configure` option, so it
/// takes no value there either. While the merged table claimed it did, the
/// scan swallowed the following word as its value and never looked at it; the
/// 8.5-only `-parameters` hiding behind it went unreported.
#[test]
fn the_ensemble_option_scan_consumes_values_only_for_the_operations_own_options() {
    assert_eq!(
        count_code_in(
            "namespace ensemble configure ::E -command -parameters arg",
            "W004",
            "tcl8.5",
        ),
        1,
        "`-command` is no `configure` option, so it swallows nothing and the \
         8.6-only `-parameters` behind it is still seen"
    );
    // `create` does have `-command`, so there the value word really is
    // consumed and there is nothing left to report.
    assert_eq!(
        count_code_in(
            "namespace ensemble create -command -parameters",
            "W004",
            "tcl8.5",
        ),
        0,
        "`create`'s `-command` consumes its value word"
    );
}

#[test]
fn w004_skips_option_value_that_looks_like_a_flag() {
    // `-stride` is Tcl 9.0+ and takes a value.  On tcl8.6 the switch itself is
    // W004, but its value word — even when it looks like a flag (`-stride`
    // again) — must not be re-tested as a second gated option: the value word
    // is skipped, or the call counts two W004s.
    assert_eq!(
        count_code("lsearch -stride -stride {a b} x", "W004"),
        1,
        "value word `-stride` was mistakenly re-tested as a second gated option"
    );
}

#[test]
fn w004_fires_on_lsearch_stride_in_tcl85() {
    // The W004 coverage requires the
    // option to exist in the registry.  `lsearch -stride` is
    // populated there.
    let mut a = Analyser::new();
    let result = a.analyse("lsearch -stride 2 {a b c d} b", "tcl8.5");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on tcl8.5 lsearch -stride, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_fires_on_version_gated_list_options() {
    // Options added after 8.4 must warn (W004) under an older dialect and stay
    // silent once available.
    let cases = [
        // (snippet, introduced-in dialect, a dialect that predates it)
        ("lsearch -index 0 {a b} x", "tcl8.5", "tcl8.4"), // -index: 8.5
        ("lsearch -nocase {a b} x", "tcl8.5", "tcl8.4"),  // -nocase: 8.5
        ("lsearch -bisect {a b} x", "tcl8.6", "tcl8.5"),  // -bisect: 8.6
        ("lsort -nocase {a b}", "tcl8.5", "tcl8.4"),      // -nocase: 8.5
        ("lsort -indices {a b}", "tcl8.5", "tcl8.4"),     // -indices: 8.5
        ("lsort -stride 2 {a b}", "tcl8.6", "tcl8.5"),    // -stride: 8.6
    ];
    for (snippet, ok_dialect, old_dialect) in cases {
        let mut a = Analyser::new();
        assert!(
            a.analyse(snippet, old_dialect)
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W004),
            "expected W004 for {snippet:?} on {old_dialect}"
        );
        let mut a = Analyser::new();
        assert!(
            !a.analyse(snippet, ok_dialect)
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W004),
            "unexpected W004 for {snippet:?} on {ok_dialect}"
        );
    }
}

#[test]
fn w004_fires_on_lsearch_stride_in_tcl86() {
    // `lsearch -stride` is Tcl 9.0-only — tclsh8.6.14 rejects it with
    // `bad option "-stride"` (the 8.6 / TIP 351 `-stride` belongs to `lsort`),
    // so W004 fires on tcl8.6 just as it does on tcl8.5 above.
    let mut a = Analyser::new();
    let result = a.analyse("lsearch -stride 2 {a b c d} b", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on tcl8.6 lsearch -stride (9.0-only), got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_fires_on_clock_scan_validate_in_tcl86() {
    // `clock scan -validate` is Tcl 9.0+ (TIP 532); the
    // subcommand-scoped option table consults the active
    // dialect via the W004 emitter's `sub_match` branch.
    let mut a = Analyser::new();
    let result = a.analyse("clock scan {today} -validate 1", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on tcl8.6 clock scan -validate, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_fires_on_fconfigure_nodelay_in_tcl86() {
    // `fconfigure -nodelay` is Tcl 9.0+ (TIP 528).
    let mut a = Analyser::new();
    let result = a.analyse("fconfigure $chan -nodelay 1", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on tcl8.6 fconfigure -nodelay, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_fires_on_chan_configure_inputmode_in_tcl86() {
    // Subcommand-scoped option: `chan configure -inputmode` is
    // Tcl 9.0+ (TIP 160).
    let mut a = Analyser::new();
    let result = a.analyse("chan configure $chan -inputmode raw", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on tcl8.6 chan configure -inputmode, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_silent_on_regsub_command_in_tcl9() {
    // Same input on Tcl 9.0 — option is supported, no W004.
    let mut a = Analyser::new();
    let result = a.analyse("regsub -command {[A-Z]+} foo {bar} out", "tcl9.0");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "W004 should not fire on tcl9.0, got {:?}",
        result.diagnostics
    );
}

// --- Shadow suppression: a same-file proc / alias really is what gets
// called, so the registry builtin's dialect-restricted option does not
// apply. Mirrors the E002/E003 arity suppression exactly (same queue,
// same resolution order).

#[test]
fn w004_suppressed_when_shadowed_by_user_proc_before_call() {
    let mut a = Analyser::new();
    let src = "proc lsearch {l args} { return $l }\nlsearch -stride 2 {a b c d}\n";
    let result = a.analyse(src, "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "a user proc shadowing lsearch should suppress the builtin's W004, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_still_fires_when_shadowing_proc_defined_after_top_level_call() {
    // Top-level calls run in source order during load; a proc defined
    // *after* this call hasn't shadowed it yet, so the builtin (and its
    // W004) is still in effect — same order-gating as arity.
    let mut a = Analyser::new();
    let src = "lsearch -stride 2 {a b c d}\nproc lsearch {l args} { return $l }\n";
    let result = a.analyse(src, "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004: the builtin is still in effect before the shadowing proc is defined, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_suppressed_when_shadowing_proc_defined_after_call_inside_proc_body() {
    // Inside a proc body, the whole file has already loaded by the time the
    // body runs, so a later top-level proc definition still shadows.
    let mut a = Analyser::new();
    let src =
        "proc caller {} { lsearch -stride 2 {a b c d} }\nproc lsearch {l args} { return $l }\n";
    let result = a.analyse(src, "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "a proc-body call resolves after full-file load, so the later proc \
definition should still suppress W004, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_suppressed_when_shadowed_by_namespaced_proc_called_unqualified() {
    // The shadowing proc is `::myns::lsearch`; the call inside the same
    // namespace resolves current-namespace-first, exactly like arity's
    // namespace resolution — not just a global `::lsearch` check.
    let mut a = Analyser::new();
    let src = "namespace eval myns {\n    proc lsearch {l args} { return $l }\n    lsearch -stride 2 {a b c d}\n}\n";
    let result = a.analyse(src, "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "a namespaced proc shadowing lsearch, called unqualified inside its \
own namespace, should suppress W004, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_suppressed_when_command_aliased_to_user_proc() {
    let mut a = Analyser::new();
    let src = "proc mylsearch {l args} { return $l }\ninterp alias {} lsearch {} mylsearch\nlsearch -stride 2 {a b c d}\n";
    let result = a.analyse(src, "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "lsearch aliased to a user proc should suppress the builtin's W004, got {:?}",
        result.diagnostics
    );
}

// --- Abbreviated-subcommand resolution: W004 now shares the registry's
// unique-prefix subcommand resolver instead of a hand-rolled exact-name
// match, so a legal Tcl ensemble abbreviation is still checked.

#[test]
fn w004_fires_on_abbreviated_chan_configure_inputmode() {
    // `configure` is `chan`'s only subcommand starting with `conf`, so real
    // Tcl ensemble dispatch accepts the abbreviation.
    let mut a = Analyser::new();
    let result = a.analyse("chan conf $chan -inputmode raw", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 on the abbreviated 'chan conf -inputmode', got {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_abstains_on_dynamic_subcommand() {
    for snippet in [
        "chan $sub -inputmode raw $chan",
        "chan [x] -inputmode raw $chan",
    ] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
            "unexpected W004 for dynamic subcommand {snippet:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn w004_abstains_on_expanded_subcommand_word() {
    let mut a = Analyser::new();
    let result = a.analyse("chan {*}$sub -inputmode raw $chan", "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "unexpected W004 for a `{{*}}`-expanded subcommand word: {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_abstains_on_ambiguous_subcommand_prefix() {
    // `p` is ambiguous between `pending` / `pipe` / `pop` / `push` / `puts`
    // on `chan` — resolution must abstain rather than guess (and rather
    // than falling back to `chan`'s own unrelated top-level option table).
    let mut a = Analyser::new();
    let result = a.analyse("chan p $chan -inputmode raw", "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "unexpected W004 for an ambiguous subcommand prefix: {:?}",
        result.diagnostics
    );
}

#[test]
fn w004_abstains_on_unknown_subcommand_instead_of_scanning_parent_options() {
    let mut a = Analyser::new();
    let result = a.analyse("chan bogus $chan -inputmode raw", "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "unexpected W004 for an unknown chan subcommand: {:?}",
        result.diagnostics
    );
}

// --- Quick fix: "Remove '-option'" deletes the flag and its value word(s).

#[test]
fn w004_fix_removes_option_and_its_value() {
    let mut a = Analyser::new();
    let src = "lsearch -stride 2 {a b c d} b";
    let result = a.analyse(src, "tcl8.6");
    let w004: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W004)
        .collect();
    assert_eq!(w004.len(), 1, "{:?}", result.diagnostics);
    let fix = w004[0]
        .fixes
        .first()
        .expect("W004 should carry a remove-option fix");
    assert_eq!(fix.new_text, "");
    assert!(fix.description.contains("-stride"), "{:?}", fix.description);
    let mut applied = src.to_string();
    applied.replace_range(fix.span.start() as usize..fix.span.end() as usize, "");
    assert_eq!(applied, "lsearch {a b c d} b");
}

#[test]
fn w004_fix_removes_option_and_value_at_end_of_command() {
    let mut a = Analyser::new();
    // `fconfigure`, not `lsearch`: `lsearch` reserves its two trailing
    // operands, so its option can never be the last word — `lsearch -stride 2`
    // is a two-operand call in which `-stride` is the *list*, which tclsh
    // 8.6.18 confirms by returning `-1` rather than rejecting the switch.
    // `fconfigure` reserves nothing, so `-nodelay 1` really is a trailing
    // option-and-value pair, which is what this fix shape is about.
    let src = "fconfigure stdout -nodelay 1";
    let result = a.analyse(src, "tcl8.6");
    let w004: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W004)
        .collect();
    assert_eq!(w004.len(), 1, "{:?}", result.diagnostics);
    let fix = w004[0].fixes.first().expect("fix");
    let mut applied = src.to_string();
    applied.replace_range(fix.span.start() as usize..fix.span.end() as usize, "");
    // No following argument to extend through, so one separator remains
    // before the deleted range — cosmetic only (Tcl treats runs of
    // whitespace between words identically).
    assert_eq!(applied.trim_end(), "fconfigure stdout");
}

/// A word sitting in a command's **reserved trailing operand** is never an
/// option candidate, whatever its shape (#2136).
///
/// `Tcl_SubstObjCmd` scans switches only while `i < objc - 1` and
/// `Tcl_LsearchObjCmd` only while `i < objc - 2`, so the trailing operands are
/// data even when spelled like a switch. Measured on tclsh 8.6.18:
/// `puts [subst -commands]` prints `-commands`, and `puts [lsearch -stride 2]`
/// prints `-1` — neither rejects a dialect-gated option, because neither ever
/// reads those words as one.
#[test]
fn w004_does_not_scan_a_reserved_trailing_operand() {
    for (src, dialect) in [
        ("puts [subst -commands]", "tcl8.6"),
        ("puts [lsearch {-stride} {-stride}]", "tcl8.4"),
        ("lsearch -stride 2", "tcl8.6"),
    ] {
        let mut a = Analyser::new();
        let result = a.analyse(src, dialect);
        let w004: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W004)
            .collect();
        assert_eq!(
            w004.len(),
            0,
            "{src}: the trailing operand is not an option: {:?}",
            result.diagnostics
        );
    }
    // Control: the same option, in a position the command really scans, is
    // still reported.
    let mut a = Analyser::new();
    let result = a.analyse("lsearch -stride 2 {a b c d} b", "tcl8.4");
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W004)
            .count(),
        1,
        "{:?}",
        result.diagnostics
    );
}

#[test]
fn w004_fix_handles_braced_option_token_without_stray_closer() {
    // A braced flag/value (`{-stride}`) is a legal, if unusual, way to write
    // the same word; the fix must delete the whole wrapped token — never
    // leaving a stray `}` behind.
    let mut a = Analyser::new();
    let src = "lsearch {-stride} 2 {a b} x";
    let result = a.analyse(src, "tcl8.6");
    let w004: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W004)
        .collect();
    assert_eq!(w004.len(), 1, "{:?}", result.diagnostics);
    let fix = w004[0].fixes.first().expect("fix");
    let mut applied = src.to_string();
    applied.replace_range(fix.span.start() as usize..fix.span.end() as usize, "");
    assert_eq!(applied, "lsearch {a b} x");
    assert_eq!(
        applied.matches('{').count(),
        applied.matches('}').count(),
        "braces must stay balanced: {applied:?}"
    );
}

#[test]
fn w304_braced_switch_form_silent() {
    // FP-NAB-05: the two-arg braced switch form is unambiguous — no W304.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {x} { switch $x { -nocase {puts a} default {puts b} } }",
        "tcl",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W304),
        "W304 must not fire on a two-arg braced switch; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w304_split_switch_form_still_fires() {
    // TP control: the split (3+ arg) switch form with a dynamic string
    // before an explicit option still warrants `--`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc f {x} { switch $x -nocase {puts a} default {puts b} }",
        "tcl",
    );
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W304),
        "W304 must still fire on the split switch form; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w304_does_not_cross_proc_param_shadow() {
    // The outer `set path -force` must NOT be attributed to the inner
    // `$path` use — the proc param `path` shadows it.  W304 may still
    // fire on the substituted `file delete $path`, but never claiming
    // the value is `-force`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "set path -force\nproc useit {path} { file delete $path }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W304 && d.message.contains("-force")),
        "{:?}",
        r.diagnostics
    );
    // A top-level source substitution also supplies no current cell value.
    // Reaching a previous `set` textually does not prove intervening writes,
    // callbacks, aliases or trace effects are absent.
    let r2 = a.analyse(
        "set path -force\nproc p {path} {}\nfile delete $path\n",
        "tcl8.6",
    );
    let found = r2
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagCode::W304)
        .collect::<Vec<_>>();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Severity::Suggestion);
    assert!(found[0].registry_source().is_some());
    assert!(!found[0].message.contains("-force"));
}
