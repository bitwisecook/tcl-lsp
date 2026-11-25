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

//! W001, W002 and the bare-ensemble E001: unknown and abbreviated subcommands, version-gated commands and shadowing helpers.

use super::*;

/// FP guard (`ticklecharts` corpus shape): a document that
/// installs the documented "`TclOO` Tricks" wiki helper
/// `proc ::oo::Helpers::callback` and calls it bare from a method body must
/// draw **no** diagnostic under 8.6, where `callback` is not a core command.
///
/// A `TclOO` member body's `namespace path` is unconditionally
/// `::oo::Helpers` (tclsh 8.6.14 / 9.0.4), so the helper really is the
/// command that bare word reaches — `esnap.tcl`'s `[callback ReadBrowser]`
/// runs. Before the shadow check consulted that implicit path the bare word
/// only ever qualified to `::callback`, so the helper did not count and the
/// call drew a W002 "disabled in the active dialect profile" for the 9.0
/// builtin the program never sees.
#[test]
fn oo_helpers_user_helper_shadows_the_dialect_gated_builtin() {
    let src = "proc ::oo::Helpers::callback {method args} {\n    \
               return [list my $method {*}$args]\n}\n\
               oo::class create Snap {\n    \
               method Start {} { return [callback ReadBrowser] }\n    \
               method ReadBrowser {} { return 1 }\n}\n";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert!(
        !codes.contains(&"W002".to_string()) && !codes.contains(&"W123".to_string()),
        "the file's own ::oo::Helpers::callback is what the method body \
         reaches: {codes:?}"
    );
}

/// TP control for the same gate: with no such helper anywhere, an 8.6
/// document really has no `callback`, and both the dialect-availability
/// report and the unknown-command report must still fire (tclsh 8.6.14:
/// `invalid command name "callback"`).
#[test]
fn oo_helpers_builtin_without_a_user_helper_still_reports() {
    let src = "oo::class create Snap {\n    \
               method Start {} { return [callback ReadBrowser] }\n    \
               method ReadBrowser {} { return 1 }\n}\n";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert!(
        codes.contains(&"W002".to_string()) && codes.contains(&"W123".to_string()),
        "8.6 core has no `callback` and nothing supplies it: {codes:?}"
    );
}

/// TN control: the implicit path is a `TclOO` member-body fact only, so a
/// `proc ::oo::Helpers::NAME` must shadow **only** inside such a body.
///
/// Uses `lseq` (Tcl 9.0-only, and not an `oo::Helpers` member of any
/// version) so the two positions differ in nothing but the namespace path:
/// inside a method body `::oo::Helpers::lseq` is genuinely on the search
/// path and shadows, at the top level nothing puts `::oo::Helpers` there
/// (tclsh 9.0.4: a bare word at the top level is looked up in `::` alone),
/// so the 8.6 dialect-availability report must stand.
#[test]
fn oo_helpers_user_helper_shadows_only_inside_a_member_body() {
    let helper = "proc ::oo::Helpers::lseq {args} { return $args }\n";
    let top_level = format!("{helper}lseq 1 5\n");
    assert!(
        codes_for_dialect(&top_level, "tcl8.6").contains(&"W002".to_string()),
        "a top-level bare word never reaches ::oo::Helpers: {:?}",
        codes_for_dialect(&top_level, "tcl8.6")
    );
    let in_method =
        format!("{helper}oo::class create C {{\n    method m {{}} {{ return [lseq 1 5] }}\n}}\n");
    assert!(
        !codes_for_dialect(&in_method, "tcl8.6").contains(&"W002".to_string()),
        "a method body resolves the helper: {:?}",
        codes_for_dialect(&in_method, "tcl8.6")
    );
}

#[test]
fn disabled_definer_nested_in_catch_reports_w002_once_without_cascade() {
    // The feature-detection idiom: a Tcl 9-only definer probed under 8.6.
    // W002 reports the availability once; the definer's member keywords
    // (`property`, `constructor`) and the defined class name must NOT
    // cascade into W123 — the definer grammar still resolves the body
    // structurally even though the dialect gate fails.
    let src = "if {![catch {oo::configurable create Greeter {
    property greeting
                   constructor {g} { my configure -greeting $g }
}}]} { puts ok }
Greeter new x
";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert_eq!(
        codes.iter().filter(|c| *c == "W002").count(),
        1,
        "W002 exactly once: {codes:?}"
    );
    // The unavailable definer head is not reported a second time as
    // unresolved, and the member keywords do not cascade.
    assert_eq!(
        codes.iter().filter(|c| *c == "W123").count(),
        0,
        "no unresolved-command finding repeats the W002: {codes:?}"
    );
}

#[test]
fn proc_nested_in_catch_registers_the_proc() {
    // `catch {proc p …}` still defines `p` — a later call is not unknown.
    let src = "if {[catch {proc p {} { return 1 }}]} { puts no }
p
";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert!(
        !codes.contains(&"W123".to_string()),
        "the nested proc must register: {codes:?}"
    );
}

#[test]
fn plain_script_body_nested_in_catch_still_descends() {
    // FP guard for the definer carve-out: an ordinary control-flow body
    // nested in the substitution is still walked, so a genuinely unknown
    // command inside it keeps its W123.
    let src = "if {[catch {if {1} { zz9unknowncmd a b }}]} { puts no }
";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert!(
        codes.contains(&"W123".to_string()),
        "unknown command inside a nested plain body still fires: {codes:?}"
    );
}

#[test]
fn enabled_definer_nested_in_catch_is_silent() {
    // Under a dialect where the definer exists there is no W002 and no
    // cascade either.
    let src = "if {![catch {oo::configurable create Greeter {
    property greeting
}}]}                { puts ok }
Greeter new x
";
    let codes = codes_for_dialect(src, "tcl9.0");
    assert!(
        !codes.contains(&"W002".to_string()) && !codes.contains(&"W123".to_string()),
        "enabled definer draws neither W002 nor a cascade: {codes:?}"
    );
}

#[test]
fn after_integer_ms_is_not_unknown_subcommand() {
    // ``after`` dispatches on cancel/idle/info, but its
    // first word may instead be a millisecond delay. An integer first word is
    // a valid time argument, not an unknown subcommand, so no W001 fires.
    for snippet in ["after 200 {puts \"Hello world!\"}", "after 0", "after 1000"] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::W001),
            "unexpected W001 for {snippet:?}: {:?}",
            result.diagnostics
        );
    }
    // A non-integer, non-subcommand first word is still a genuine error.
    let mut a = Analyser::new();
    let result = a.analyse("after bogus {puts hi}", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W001),
        "W001 expected for `after bogus`: {:?}",
        result.diagnostics
    );
}

#[test]
fn w001_accepts_unique_prefix_subcommand_abbreviations() {
    // Tcl ensemble dispatch accepts unique-prefix abbreviations; the analyser
    // must not flag them as unknown subcommands (W001).
    for snippet in [
        "string le $s",   // length
        "string leng $s", // length
        "info ex v",      // exists
        "dict k $d",      // keys
        "string rev $s",  // reverse
    ] {
        assert!(
            !has_code(snippet, "tcl8.6", "W001"),
            "unique-prefix abbreviation {snippet:?} must not trip W001"
        );
    }
    // A genuinely unknown word still fires.
    assert!(
        has_code("string zzz $s", "tcl8.6", "W001"),
        "an unknown subcommand must still fire W001"
    );
    // An ambiguous prefix (`string t` → tolower/totitle/toupper/trim…) is not
    // a valid abbreviation. It gets its own diagnostic naming the candidate
    // set (W145) rather than the unknown-subcommand guess (W001).
    assert!(
        has_code("string t $s", "tcl8.6", "W145"),
        "an ambiguous prefix must fire W145"
    );
    assert!(
        !has_code("string t $s", "tcl8.6", "W001"),
        "W145 replaces the W001 guess for an ambiguous prefix"
    );
}

#[test]
fn command_version_gates_fire_w002_once() {
    // Whole commands introduced after 8.4 are reported once, as unavailable
    // (W002, naming the release that has them), in older dialects, never
    // also as unresolved (W123), and are known once available.
    let cases = [
        ("apply {{} {return 1}}", "tcl8.5", "tcl8.4"),
        ("lreverse {a b c}", "tcl8.5", "tcl8.4"),
        ("lrepeat 3 x", "tcl8.5", "tcl8.4"),
        // NB: `const` is intentionally universal (valid in iRules), so it is
        // deliberately NOT version-gated — see const_.rs.
    ];
    for (snippet, ok, old) in cases {
        let codes: Vec<_> = Analyser::new()
            .analyse(snippet, old)
            .diagnostics
            .iter()
            .map(|d| d.code)
            .collect();
        assert_eq!(codes, [DiagCode::W002], "{snippet:?} on {old}");
        assert!(
            !Analyser::new()
                .analyse(snippet, ok)
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W123),
            "unexpected W123 for {snippet:?} on {ok}"
        );
    }
}

#[test]
fn subcommand_version_gates_fire_w002() {
    // A subcommand added or removed across 8.4-9.1 still *exists* — just not in
    // every dialect.  Used in a dialect where it is absent it must warn with
    // W002 ("disabled in the active dialect profile"), the subcommand-level
    // analogue of the whole-command W002 check, NOT W001 ("Unknown
    // subcommand"), which is reserved for a name that exists in no dialect at
    // all (a genuine typo).  `info cmdtype` is a real Tcl 9.0 subcommand, so
    // flagging it as "unknown" under the default 8.6 profile would be wrong.
    let added = [
        // (snippet, first dialect it exists in, an older dialect)
        ("string reverse abc", "tcl8.5", "tcl8.4"),
        ("package prefer stable", "tcl8.5", "tcl8.4"),
        ("encoding dirs", "tcl8.5", "tcl8.4"),
        ("binary encode base64 abc", "tcl8.6", "tcl8.5"),
        ("binary decode base64 abc", "tcl8.6", "tcl8.5"),
        ("interp bgerror {}", "tcl8.5", "tcl8.4"),
        ("interp limit {} time", "tcl8.5", "tcl8.4"),
        // interp debug is documented from Tcl 8.5's interp.n (present in
        // the 8.5 SYNOPSIS/body, absent from 8.4's subcommand list).
        ("interp debug {}", "tcl8.5", "tcl8.4"),
        ("interp cancel", "tcl8.6", "tcl8.5"),
        ("interp children", "tcl8.6", "tcl8.5"),
        ("clock add 0 1 day", "tcl8.5", "tcl8.4"),
        ("clock microseconds", "tcl8.5", "tcl8.4"),
        ("clock milliseconds", "tcl8.5", "tcl8.4"),
        // `info cmdtype` is new in Tcl 9.0.
        ("info cmdtype foo", "tcl9.0", "tcl8.6"),
    ];
    for (snippet, ok, old) in added {
        let old_diags = Analyser::new().analyse(snippet, old).diagnostics;
        assert!(
            old_diags.iter().any(|d| d.code == DiagCode::W002),
            "expected W002 for {snippet:?} on {old}"
        );
        assert!(
            !old_diags.iter().any(|d| d.code == DiagCode::W001),
            "unexpected W001 (should be W002) for {snippet:?} on {old}"
        );
        assert!(
            !Analyser::new()
                .analyse(snippet, ok)
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W001 || d.code == DiagCode::W002),
            "unexpected W001/W002 for {snippet:?} on {ok}"
        );
    }
    // `trace variable/vdelete/vinfo` were removed in 9.0: known in 8.6, gone in
    // 9.0.  In 9.0 they still resolve in an older dialect, so they are W002
    // (disabled here), not W001 (unknown everywhere).
    for snippet in [
        "trace variable v w {}",
        "trace vdelete v w {}",
        "trace vinfo v",
    ] {
        assert!(
            !Analyser::new()
                .analyse(snippet, "tcl8.6")
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W001 || d.code == DiagCode::W002),
            "unexpected W001/W002 for {snippet:?} on tcl8.6"
        );
        let nine = Analyser::new().analyse(snippet, "tcl9.0").diagnostics;
        assert!(
            nine.iter().any(|d| d.code == DiagCode::W002),
            "expected W002 for {snippet:?} on tcl9.0 (removed, but exists in 8.x)"
        );
        assert!(
            !nine.iter().any(|d| d.code == DiagCode::W001),
            "unexpected W001 (should be W002) for {snippet:?} on tcl9.0"
        );
    }
    // Tcl 8.6 introduced preferred `interp children`, but Tcl 9 still accepts
    // legacy `interp slaves`.  It is therefore a W144 deprecation with a safe
    // registry-provided fix, not a W002 unavailable-subcommand diagnostic.
    let slaves = Analyser::new()
        .analyse("interp slaves", "tcl9.0")
        .diagnostics;
    assert!(
        slaves.iter().any(|d| d.code == DiagCode::W144),
        "expected W144 for deprecated-but-working `interp slaves` on tcl9.0"
    );
    assert!(
        !slaves
            .iter()
            .any(|d| d.code == DiagCode::W001 || d.code == DiagCode::W002),
        "`interp slaves` remains available in tcl9.0: {slaves:?}"
    );
    // A subcommand that exists in *no* dialect is still a genuine W001.
    let typo = Analyser::new()
        .analyse("string bogusxyz abc", "tcl8.6")
        .diagnostics;
    assert!(
        typo.iter().any(|d| d.code == DiagCode::W001),
        "expected W001 for a genuinely unknown subcommand"
    );
    assert!(
        !typo.iter().any(|d| d.code == DiagCode::W002),
        "unexpected W002 for a genuinely unknown subcommand"
    );
}

#[test]
fn info_frame_is_dialect_gated_to_8_5_plus() {
    // `info frame` was introduced in Tcl 8.5 (TIP 280); it does not exist in
    // 8.4.  Because it *does* exist in 8.5+, using it under 8.4 is W002
    // ("disabled in the active dialect profile"), not W001 ("Unknown
    // subcommand") — the subcommand exists, just not in that dialect.
    assert!(
        has_code("info frame\n", "tcl8.4", "W002"),
        "info frame should be disabled-in-dialect (W002) in tcl8.4"
    );
    assert!(
        !has_code("info frame\n", "tcl8.4", "W001"),
        "info frame is a real subcommand, so not W001 in tcl8.4"
    );
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(
            !has_code("info frame\n", dialect, "W001"),
            "info frame should be known in {dialect}"
        );
        assert!(
            !has_code("info frame\n", dialect, "W002"),
            "info frame should be enabled in {dialect}"
        );
    }
}

#[test]
fn namespace_unknown_is_dialect_gated_to_8_5_plus() {
    // `namespace unknown` was added in 8.5, like the sibling `namespace path`;
    // 8.4's `namespace` ensemble has no `unknown` subcommand.  Using it under
    // 8.4 is W002 (disabled-in-dialect), not W001 (the subcommand exists), and
    // it is clean from 8.5 on.
    assert!(
        has_code("namespace unknown handler\n", "tcl8.4", "W002"),
        "namespace unknown should be disabled-in-dialect (W002) in tcl8.4"
    );
    assert!(
        !has_code("namespace unknown handler\n", "tcl8.4", "W001"),
        "namespace unknown is a real subcommand, so not W001 in tcl8.4"
    );
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(
            !has_code("namespace unknown handler\n", dialect, "W002"),
            "namespace unknown should be enabled in {dialect}"
        );
    }
}

#[test]
fn e001_fires_for_bare_subcommand_command() {
    // A subcommand-dispatch command (`string`, `dict`, `info`) invoked
    // with no subcommand at all is E001.
    for snippet in ["string", "dict", "info"] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        let e001: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E001)
            .collect();
        assert_eq!(
            e001.len(),
            1,
            "expected one E001 for {snippet:?}: {result:?}",
        );
        assert_eq!(
            e001[0].message,
            format!("'{snippet}' requires a subcommand")
        );
        assert_eq!(e001[0].severity, Severity::Error);
    }
}

#[test]
fn e001_quiet_when_subcommand_present() {
    let mut a = Analyser::new();
    let result = a.analyse("string length abc", "tcl8.6");
    assert!(!result.diagnostics.iter().any(|d| d.code == DiagCode::E001));
}

#[test]
fn e001_suppressed_by_shadowing_user_proc() {
    // A user proc named `string` shadows the builtin ensemble — a bare
    // `string` call resolves to the proc, so no E001.
    let mut a = Analyser::new();
    let result = a.analyse("proc string {} { return x }\nstring", "tcl8.6");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::E001),
        "shadowing proc should suppress E001: {result:?}"
    );
}

#[test]
fn e001_fn_true_negative_bare_history_has_a_default_subcommand() {
    // FP regression: `history` is a `WithSubcommands` registry command
    // (`add`/`change`/`clear`/`event`/`info`/`keep`/`nextid`/`redo`) but,
    // unlike `string`/`dict`/`info`, a bare call is well-defined Tcl —
    // history(n): "If no option is specified, the default is info."
    // Confirmed stable since Tcl 7.x; still true under Tcl 9. The registry
    // spec records this as `arity: Arity::at_least(0)` and the analyser
    // must honour it instead of assuming every ensemble-shaped command
    // requires its dispatch word.
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        assert!(
            !has_code("history", dialect, "E001"),
            "bare `history` defaults to `history info` — not E001 in {dialect}"
        );
    }
}

#[test]
fn e001_tp_bare_history_subcommand_sibling_still_required() {
    // Every *other* WithSubcommands command in the same family keeps
    // requiring its dispatch word — the `history` carve-out must not leak
    // into commands whose registry `arity.min` is genuinely 1.
    for snippet in [
        "array",
        "chan",
        "clock",
        "encoding",
        "file",
        "namespace",
        "package",
        "trace",
    ] {
        assert!(
            has_code(snippet, "tcl8.6", "E001"),
            "bare `{snippet}` should still require a subcommand"
        );
    }
}

#[test]
fn e001_tn_history_with_subcommand_is_unaffected() {
    // Sanity: a `history` call that *does* supply a subcommand is untouched
    // by the bare-call carve-out — still no E001, and a genuine unknown
    // subcommand is still W001.
    assert!(!has_code("history info", "tcl8.6", "E001"));
    assert!(!has_code("history clear", "tcl8.6", "E001"));
    assert!(has_code("history bogus", "tcl8.6", "W001"));
    assert!(!has_code("history bogus", "tcl8.6", "E001"));
}
