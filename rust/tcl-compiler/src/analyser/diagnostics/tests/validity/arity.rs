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

//! E002, E003 and E005 against registry signatures: leading options, subcommands, shadowing, the surplus-argument span and argument-count shapes.

use super::*;

/// `interp create` reads a `-` word as a flag before or after the path, so
/// only the path is positional. tclsh 8.4.20 to 9.1.0 accept the first three
/// forms (`interp issafe x` is 1) and raise `wrong # args` for the rest.
#[test]
fn interp_create_counts_only_its_path_as_positional() {
    for (src, too_many) in [
        ("interp create x -safe", false),
        ("interp create -safe -- z", false),
        ("interp create -safe y", false),
        ("interp create a b", true),
        ("interp create -safe c d", true),
        ("interp create e -safe f", true),
    ] {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert_eq!(has_code(src, dialect, "E003"), too_many, "{dialect}: {src}");
        }
    }
}

/// Diagnostic codes for `src` under the Tcl 9.0 profile, in emission order.
fn all_codes(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, "tcl9.0")
        .diagnostics
        .into_iter()
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn e003_tp_call_between_two_declarations_checks_the_first_signature() {
    // TP. `p a b` sits between a zero-parameter `proc p`
    // and a later one-parameter redefinition; tclsh 9.0.4 and 8.6.16 both
    // fail it with `wrong # args: should be "p"` — the *first* signature.
    // `all_procs` alone only remembers the second, so the call resolved to
    // nothing and the guaranteed failure went unreported.
    let src = "proc p {} { return 1 }\np a b\nproc p {a} { return $a }\n";
    assert!(
        all_codes(src).contains(&"E003".to_string()),
        "the in-between call is checked against the first definition: {:?}",
        all_codes(src)
    );
}

#[test]
fn e003_tn_call_between_two_declarations_matching_the_first_is_silent() {
    // TN — the paired guard: the same position with an argument count the
    // *first* definition accepts (and the second would reject) must stay
    // silent. tclsh: `p` there returns `first`.
    let src = "proc p {} { return 1 }\np\nproc p {a} { return $a }\n";
    assert!(
        !all_codes(src).contains(&"E003".to_string()),
        "a call the in-effect definition accepts is not an arity error: {:?}",
        all_codes(src)
    );
}

#[test]
fn e003_tp_call_after_both_declarations_checks_the_last_signature() {
    // TP — last-redefinition-wins is still right after the second header:
    // tclsh fails a bare `p` with `wrong # args: should be "p a"`.
    let src = "proc p {} { return 1 }\nproc p {a} { return $a }\np\n";
    assert!(
        all_codes(src).contains(&"E002".to_string()),
        "the trailing call is checked against the second definition: {:?}",
        all_codes(src)
    );
}

// E002 / E003 arity

#[test]
fn e003_not_emitted_for_leading_switches() {
    // Declared option flags must be skipped
    // before counting positional args, or `regsub` (max arity 4) draws a false
    // E003 as soon as any switch appears.
    // These switches exist in every supported dialect.
    for snippet in [
        "regsub -all -line {x} $args {} str",
        "regsub -all {a} $b {} c",
        "regsub -nocase -all -- $pat $s {} out",
    ] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        let e003: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E003)
            .collect();
        assert_eq!(e003.len(), 0, "unexpected E003 for {snippet:?}: {e003:?}");
    }
}

#[test]
fn socket_server_form_counts_only_its_port_as_positional() {
    assert!(
        !has_code("socket -server accept 1234", "tcl8.6", "E002"),
        "the server callback is the value of -server; only the port is positional"
    );
    assert!(
        has_code("socket -server accept", "tcl8.6", "E002"),
        "the server form still requires its port"
    );
    assert!(
        !has_code("socket example.com 80", "tcl8.6", "E002"),
        "the client form still accepts host and port"
    );
    assert!(
        has_code("socket example.com", "tcl8.6", "E002"),
        "the client form still requires both host and port"
    );
}

#[test]
fn e003_not_emitted_for_value_taking_leading_option() {
    // `-start` consumes a value word, so
    // `regsub -start 0 $exp $str $sub out` is `-start` + its value +
    // 4 positional (exp/string/subSpec/varName) = max arity 4 → valid.
    // A name-only leading-option skip miscounted the `0` value word as a
    // positional and tripped a false E003.
    for snippet in [
        "regsub -start 0 $exp $str $sub out",
        "regsub -all -start 3 {a} $b {} c",
    ] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        let e003: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E003)
            .collect();
        assert_eq!(
            e003.len(),
            0,
            "unexpected E003 for value-taking option in {snippet:?}: {e003:?}"
        );
    }
}

#[test]
fn e003_still_fires_past_value_taking_option() {
    // The value-word skip must not mask a genuine over-arity: after
    // `-start 0` there are 5 positional words (> max 4) → real E003.
    let mut a = Analyser::new();
    let result = a.analyse("regsub -start 0 a b c d e", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "expected E003 past a value-taking option, got {:?}",
        result.diagnostics
    );
}

#[test]
fn interp_optional_path_subcommands_no_arity_error() {
    // `interp issafe`/`exists`/`hidden` take an optional
    // `?path?`; the zero-arg idiom must not trip E002.
    for snippet in ["interp issafe", "interp exists", "interp hidden"] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        let arity_err: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
            .collect();
        assert_eq!(
            arity_err.len(),
            0,
            "unexpected arity error for {snippet:?}: {arity_err:?}"
        );
    }
    // The one-arg form is still accepted.
    let mut a = Analyser::new();
    let ok = a.analyse("interp issafe child", "tcl8.6");
    assert!(
        !ok.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003),
    );
}

#[test]
fn interp_create_with_options_no_arity_error() {
    // `interp create -safe -- name` — the option words are
    // skipped, leaving one positional (`name`) within the 0..=1 bound.
    let mut a = Analyser::new();
    let result = a.analyse("interp create -safe -- child", "tcl8.6");
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003),
        "unexpected arity error for `interp create -safe -- child`: {:?}",
        result.diagnostics
    );
    // Two positional names is a genuine over-arity.
    let mut a2 = Analyser::new();
    let over = a2.analyse("interp create a b", "tcl8.6");
    assert!(
        over.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "expected E003 for two-name `interp create`, got {:?}",
        over.diagnostics
    );
}

#[test]
fn e003_fires_on_genuine_over_arity() {
    // 5 positional args for `regsub` (max 4) is a real error.
    let mut a = Analyser::new();
    let result = a.analyse("regsub a b c d e", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "expected E003, got {:?}",
        result.diagnostics
    );
}

#[test]
fn e003_switch_options_are_dialect_filtered() {
    // `regsub -command` is Tcl 9.0+ (TIP 463).
    // Under 9.0 it is a real switch → skipped → 4 positional → OK.
    let mut a = Analyser::new();
    let r9 = a.analyse("regsub -command a b c d", "tcl9.0");
    assert!(
        !r9.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "unexpected E003 under tcl9.0: {:?}",
        r9.diagnostics
    );
    // Under 8.6 `-command` is no option (tclsh 8.6: `bad option
    // "-command"`), reported as an option the dialect lacks (dialect-leak
    // guard).
    let mut a2 = Analyser::new();
    let r8 = a2.analyse("regsub -command a b c d", "tcl8.6");
    assert!(
        r8.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 under tcl8.6, got {:?}",
        r8.diagnostics
    );
}

#[test]
fn e003_source_nopkg_is_tcl9_only() {
    // C Tcl 9.0 accepts `source -nopkg file`, then attempts to read `file`.
    // Tcl 8.6 rejects the same shape as a wrong-arity invocation. The registry
    // must therefore skip the flag only for the Tcl 9 dialect rather than
    // treating it as a universal source argument.
    let mut tcl9 = Analyser::new();
    let r9 = tcl9.analyse("source -nopkg missing.tcl", "tcl9.0");
    assert!(
        !r9.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "unexpected E003 under tcl9.0: {:?}",
        r9.diagnostics
    );

    let mut tcl86 = Analyser::new();
    let r86 = tcl86.analyse("source -nopkg missing.tcl", "tcl8.6");
    assert!(
        r86.diagnostics.iter().any(|d| d.code == DiagCode::W004),
        "expected W004 under tcl8.6: {:?}",
        r86.diagnostics
    );
}

#[test]
fn e003_suppressed_by_expanded_word() {
    // `{*}$rest` expands to an unknown count, so the expanded word
    // is excluded from the positional lower bound: `regsub a b c d
    // {*}$rest` has 4 non-expanded positional words (≤ max 4) and
    // must not trip E003, whereas the same five literal words do.
    let mut a = Analyser::new();
    let expanded = a.analyse("regsub a b c d {*}$rest", "tcl8.6");
    assert!(
        !expanded
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E003),
        "expansion should suppress E003: {:?}",
        expanded.diagnostics
    );
    let mut b = Analyser::new();
    let literal = b.analyse("regsub a b c d e", "tcl8.6");
    assert!(
        literal.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "control: five literal words should fire E003: {:?}",
        literal.diagnostics
    );
}

#[test]
fn e003_arity_is_dialect_aware_via_expand_syntax() {
    // End-to-end proof that the
    // document dialect reaches the analyser's segmenter (and thus the
    // lexer's `expand_syntax` flag).  `{*}` is the expansion operator
    // on 8.5+ but a plain braced word on 8.4, so for
    // `regsub a b c d {*}$rest`:
    //   * tcl8.4 — `{*}` closes a braced word that `$rest` follows, so the
    //     script does not parse (tclsh8.4: `extra characters after
    //     close-brace`) → E204.
    //   * tcl9.0 — `{*}$rest` expands, contributing an unbounded count;
    //     the 4 non-expanded words are ≤ max 4 → E003 is suppressed.
    // Before the dialect → `LexerConfig` wiring the analyser always
    // lexed with `expand_syntax` on, so 8.4 wrongly behaved like 9.0
    // (no E003) — this asserts the two now diverge.
    let codes = |dialect: &str| -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse("regsub a b c d {*}$rest", dialect)
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect()
    };
    let on_84 = codes("tcl8.4");
    assert!(
        on_84.iter().any(|c| c == "E204"),
        "8.4 reads `{{*}}` as a braced word followed by `$rest` → E204: {on_84:?}",
    );
    let on_90 = codes("tcl9.0");
    assert!(
        !on_90.iter().any(|c| c == "E003"),
        "9.0 expands `{{*}}` → 4 positional words ≤ max → no E003: {on_90:?}",
    );
}

// Subcommand-level E003 arity (per-subcommand signatures).

#[test]
fn e003_fires_on_subcommand_over_arity() {
    // `string length` takes exactly one argument — three positional
    // words must trip E003.
    let mut a = Analyser::new();
    let result = a.analyse("string length a b c", "tcl8.6");
    let e003: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    assert!(
        !e003.is_empty(),
        "expected E003 for `string length a b c`, got {:?}",
        result.diagnostics
    );
    assert!(
        e003[0].message.contains("string length"),
        "message should name the subcommand: {:?}",
        e003[0].message
    );
}

#[test]
fn e003_fires_on_file_link_over_arity() {
    // `file link ?-linktype? linkName ?target?` — `link` accepts at
    // most two positional args after its one option. A dynamic first word
    // may be `-symbolic` (tclsh 8.6: `file link -symbolic a b` is valid), so
    // only the literal form is a provable wrong-#args.
    let mut a = Analyser::new();
    let result = a.analyse("file link -symbolic a b c", "tcl8.6");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "expected E003 for `file link -symbolic a b c`, got {:?}",
        result.diagnostics
    );
}

#[test]
fn e003_namespace_which_extra_positional_fires() {
    // `namespace which ?-command? ?-variable? name` — exactly one trailing
    // `name`, the flags declared as options. Verified vs tclsh 9.0: a second
    // positional (`foo bar`) errors, but the flag forms are fine.
    fn e003(src: &str) -> bool {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E003)
    }
    // FN fix: a bare second positional is now flagged.
    assert!(
        e003("namespace which foo bar"),
        "extra positional must fire E003"
    );
    // Valid forms stay silent — the flags are skipped before counting.
    assert!(!e003("namespace which foo"), "one name is valid");
    assert!(
        !e003("namespace which -command foo"),
        "-command name is valid"
    );
    assert!(
        !e003("namespace which -variable foo"),
        "-variable name is valid"
    );
}

#[test]
fn e003_silent_for_subcommand_leading_options() {
    // Per-subcommand options (`file link -symbolic` / `-hard`,
    // `string match -nocase`) must be skipped before counting
    // positionals, so these well-formed calls stay silent.
    for snippet in [
        "file link -symbolic $a $b",
        "file link -hard $a $b",
        "string match -nocase $a $b",
    ] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        let e003: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E003)
            .collect();
        assert_eq!(e003.len(), 0, "unexpected E003 for {snippet:?}: {e003:?}");
    }
}

#[test]
fn subcommand_arity_skips_unknown_and_dynamic_subcommands() {
    // An unknown subcommand is W001's job, not E003; a dynamic
    // subcommand word (`$sub`) can't be resolved, so neither path
    // should emit E003.
    for snippet in ["string $sub a b c", "string [x] a b c"] {
        let mut a = Analyser::new();
        let result = a.analyse(snippet, "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == DiagCode::E003),
            "unexpected E003 for {snippet:?}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn e002_fires_on_too_few_args() {
    // `regsub` requires at least 3 args (exp string subSpec).
    let mut a = Analyser::new();
    let result = a.analyse("regsub a b", "tcl8.6");
    let e002: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E002)
        .collect();
    assert!(
        !e002.is_empty(),
        "expected E002 for `regsub a b`, got {:?}",
        result.diagnostics
    );
    assert!(e002[0].message.contains("at least 3"));
}

#[test]
fn e003_shadow_is_namespace_scoped() {
    // A namespaced proc named `close` must
    // NOT suppress arity checks on a *global* `close` call (which
    // resolves to the builtin, max 2), but must suppress a `close`
    // call inside its own namespace (which resolves to the proc). The
    // namespace exists first (tclsh: `can't create procedure "::ns::close":
    // unknown namespace`), and the proc takes the three words the namespaced
    // call passes, so that call is valid.
    let src = "namespace eval ::ns {}\n\
                   proc ::ns::close {a b c} {}\n\
                   close x y z\n\
                   namespace eval ::ns { close x y z }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let e003: Vec<&Diagnostic> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    assert_eq!(
        e003.len(),
        1,
        "expected exactly one E003 (the global close), got {:?}",
        r.diagnostics
    );
    // The flagged call must be the top-level one, before the
    // `namespace eval` body (both call sites share the same text).
    let ns_eval_off = src.find("namespace eval").unwrap();
    let span = e003[0].span;
    assert!(
        (span.start() as usize) < ns_eval_off,
        "flagged the namespaced call instead of the global one: {:?}",
        &src[span.start() as usize..span.end() as usize],
    );
}

// reachable, in-order shadow gating

#[test]
fn e003_top_level_call_before_shadowing_proc_fires() {
    // A top-level `close x y z` *before* `proc close` resolves to
    // the builtin at load time (the proc does not exist yet), so the
    // builtin arity check must fire even though a same-named proc is
    // defined later in the file.  Exercises the in-order gate
    // (without it the post-walk flush silenced this).
    let src = "close x y z\nproc close {a b c d} {}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let e003: Vec<&Diagnostic> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    assert_eq!(
        e003.len(),
        1,
        "expected E003 on the top-level close before its shadowing proc, got {:?}",
        r.diagnostics
    );
    // The flagged call is the top-level one on line 1, not the proc on line 2.
    // E003 highlights only the surplus arguments, so the span starts at the
    // first excess word rather than the command name — assert it lands on the
    // first line.
    let line2 = src.find('\n').unwrap();
    assert!(
        (e003[0].span.start() as usize) < line2,
        "wrong call flagged: {:?}",
        &src[e003[0].span.start() as usize..e003[0].span.end() as usize],
    );
}

#[test]
fn e003_top_level_call_after_shadowing_proc_suppressed() {
    // The mirror image: once `proc close` is defined, a later
    // top-level `close x y z` resolves to the 4-param user proc, so
    // the builtin arity check is suppressed.
    let src = "proc close {a b c d} {}\nclose x y z\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "no E003 expected — the call follows its shadowing proc, got {:?}",
        r.diagnostics
    );
}

#[test]
fn e003_proc_body_call_not_order_gated() {
    // A call inside a proc body resolves when that proc is *invoked*
    // — after the whole script has loaded — so a shadowing proc
    // defined later in the file still suppresses the builtin check.
    // Order is only enforced for top-level calls.
    let src = "proc foo {} { close x y z }\nproc close {a b c d} {}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "no E003 expected — proc-body calls are not order-gated, got {:?}",
        r.diagnostics
    );
}

#[test]
fn e003_static_rename_onto_builtin_name_suppresses_arity() {
    // `rename OLD NEW` moves OLD's identity onto NEW — `rename myimpl
    // close` makes `close` a real command backed by `myimpl`'s own
    // 3-parameter signature, not the registry `close` builtin (max 2), so
    // a 3-argument call must not fire E003.  Order-gated the same way as a
    // proc: the rename statement must lexically precede a top-level call.
    // tclsh 8.4-9.0: `rename myimpl close` fails while `close` exists, so
    // the builtin is deleted first.
    let src =
        "proc myimpl {a b c} { return $a }\nrename close {}\nrename myimpl close\nclose 1 2 3\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::E003),
        "no E003 expected — close was renamed onto myimpl's 3-arg signature, got {:?}",
        r.diagnostics
    );
    let refused = "proc myimpl {a b c} { return $a }\nrename myimpl close\nclose 1 2 3\n";
    assert!(
        Analyser::new()
            .analyse(refused, "tcl8.6")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::E003),
        "a rename onto an existing command fails, so the builtin keeps its arity"
    );
}

#[test]
fn e003_top_level_call_before_rename_onto_builtin_still_fires() {
    // The mirror image of `e003_top_level_call_before_shadowing_proc_fires`
    // for a rename target: a top-level call *before* the `rename` runs
    // still reaches the original builtin at load time.
    let src = "close 1 2 3\nproc myimpl {a b c} { return $a }\nrename myimpl close\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let e003: Vec<&Diagnostic> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    assert_eq!(
        e003.len(),
        1,
        "expected E003 on the top-level close before the rename took effect, got {:?}",
        r.diagnostics
    );
    // Tight E003 span: the surplus argument of the line-1 call, not the
    // renamed proc on a later line.
    let line2 = src.find('\n').unwrap();
    assert!(
        (e003[0].span.start() as usize) < line2,
        "wrong call flagged: {:?}",
        &src[e003[0].span.start() as usize..e003[0].span.end() as usize],
    );
}

// E003 tight-range + registry arity-data corrections (verified against
// tclsh 9.0.4). See `rust/tcl-registry/src/commands/tcl/{lmap,global,
// variable,auto_load,auto_import}_.rs`.

/// Return the source text E003 highlights, so range tightness can be
/// asserted against the *problem* words rather than the whole command.
fn e003_highlighted_text(src: &str) -> String {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::E003)
        .unwrap_or_else(|| panic!("no E003 in {:?}", r.diagnostics));
    src[d.span.start() as usize..d.span.end() as usize].to_string()
}

#[test]
fn e003_highlights_only_the_surplus_arguments() {
    // TP + range precision: the squiggle covers exactly the excess words,
    // not the command name or the valid arguments.
    assert_eq!(
        e003_highlighted_text("lreverse {a b c} extra1 extra2\n"),
        "extra1 extra2",
        "E003 must highlight only the surplus arguments",
    );
    assert_eq!(
        e003_highlighted_text("string index abc 0 extra\n"),
        "extra",
        "E003 on a subcommand must highlight only the surplus word",
    );
    // Leading options are skipped: the surplus is measured after them.
    assert_eq!(
        e003_highlighted_text("puts -nonewline stdout hello extra\n"),
        "extra",
        "E003 must skip leading options when isolating the surplus",
    );
}

#[test]
fn e003_falls_back_to_whole_command_on_expansion() {
    // A `{*}`-expanded positional makes the first surplus word ambiguous, so
    // the highlight falls back to the whole command (still a TP).
    // `$extra` is a formal: an unset top-level read would end the script
    // first. tclsh 8.6 `p {}`: `wrong # args: should be "lreverse list"`.
    let src = "proc p {extra} { lreverse {a b c} {*}$extra more }\np {}\n";
    let text = e003_highlighted_text(src);
    assert!(
        text.starts_with("lreverse"),
        "expansion case should fall back to the whole command, got {text:?}",
    );
}

#[test]
fn lmap_odd_even_parity_fires_e005() {
    // FN fix: `lmap` shares `foreach`'s odd/even grammar. An even count is
    // `wrong # args` (tclsh 9.0.4); a valid odd count is silent.
    assert!(has_code("lmap a b c d\n", "tcl8.6", "E005"));
    assert!(!has_code("lmap x {1 2 3} {incr x}\n", "tcl8.6", "E005"));
    assert!(!has_code(
        "lmap a {1 2} b {3 4} {expr {$a+$b}}\n",
        "tcl8.6",
        "E005",
    ));
}

#[test]
fn global_and_variable_zero_args_are_no_op_not_e002() {
    // FP fix: bare `global` / `variable` are valid no-ops (C Tcl has no
    // `Tcl_WrongNumArgs` on either).
    for src in ["global\n", "variable\n", "global a b c\n"] {
        assert!(
            !has_code(src, "tcl8.6", "E002"),
            "{src:?} must not draw E002",
        );
    }
}

#[test]
fn auto_load_and_auto_import_arity_bounds() {
    // auto_load cmd ?namespace? — 1..2 valid, 3 too many.
    assert!(!has_code("auto_load foo\n", "tcl8.6", "E003"));
    assert!(!has_code("auto_load foo ::ns\n", "tcl8.6", "E003"));
    assert!(has_code("auto_load a b c\n", "tcl8.6", "E003"));
    // auto_import pattern — exactly 1.
    assert!(!has_code("auto_import xyz*\n", "tcl8.6", "E003"));
    assert!(has_code("auto_import a b\n", "tcl8.6", "E003"));
}

// E005 — argument-count *shape* (parity/predicate) mismatches on the
// registry's key/value-pair and paired-argument commands (`dict create`,
// `dict replace`, `dict update`, `foreach`, `switch`). All confirmed
// against tclsh 8.6.14's "wrong # args" behaviour.

fn arity_shape_codes(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code, DiagCode::E002 | DiagCode::E003 | DiagCode::E005))
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn dict_create_odd_key_value_tail_fires_e005() {
    // `dict create ?key value ...?` — an odd tail has an unpaired key with
    // no value (tclsh 8.6.14: `dict create a` fails "wrong # args").
    assert_eq!(
        arity_shape_codes("dict create a\n"),
        vec!["E005".to_owned()]
    );
    assert_eq!(
        arity_shape_codes("dict create a b c\n"),
        vec!["E005".to_owned()]
    );
}

#[test]
fn dict_create_even_key_value_tail_is_silent() {
    // TN: a paired (or empty) tail is the documented shape.
    assert_eq!(arity_shape_codes("dict create\n"), Vec::<String>::new());
    assert_eq!(arity_shape_codes("dict create a b\n"), Vec::<String>::new());
    assert_eq!(
        arity_shape_codes("dict create a b c d\n"),
        Vec::<String>::new()
    );
}

#[test]
fn dict_replace_even_tail_fires_e005() {
    // `dict replace dictionaryValue ?key value ...?` — the dict value
    // itself makes the *total* count odd; an even count leaves a
    // trailing key with no value (tclsh 8.6.14: `dict replace $d a` fails
    // "wrong # args").
    assert_eq!(
        arity_shape_codes("dict replace $d a\n"),
        vec!["E005".to_owned()]
    );
}

#[test]
fn dict_replace_odd_tail_is_silent() {
    assert_eq!(arity_shape_codes("dict replace $d\n"), Vec::<String>::new());
    assert_eq!(
        arity_shape_codes("dict replace $d a b\n"),
        Vec::<String>::new()
    );
}

#[test]
fn dict_update_odd_total_fires_e005() {
    // `dict update dictVar key varName ?key varName ...? body` — total
    // count is always even (dict var + N pairs + body); an odd total
    // means an unpaired key or a missing body (tclsh 8.6.14: `dict update
    // d k v extra body` — 5 words — fails "wrong # args").
    assert_eq!(
        arity_shape_codes("dict update d k v extra body\n"),
        vec!["E005".to_owned()]
    );
}

#[test]
fn dict_update_even_total_is_silent() {
    assert_eq!(
        arity_shape_codes("dict update d k v body\n"),
        Vec::<String>::new()
    );
    assert_eq!(
        arity_shape_codes("dict update d k1 v1 k2 v2 body\n"),
        Vec::<String>::new()
    );
}

#[test]
fn foreach_unpaired_varlist_fires_e005() {
    // `foreach varList list ?varList list ...? body` — total count is
    // always odd (N varList/list pairs + body); an even total leaves an
    // unpaired trailing var-list with no source list (tclsh 8.6.14:
    // `foreach x $l y {puts $x}` — 4 words — fails "wrong # args").
    assert_eq!(
        arity_shape_codes("foreach x $l y {puts $x}\n"),
        vec!["E005".to_owned()]
    );
}

#[test]
fn foreach_paired_varlists_are_silent() {
    assert_eq!(
        arity_shape_codes("foreach x $l {puts $x}\n"),
        Vec::<String>::new()
    );
    assert_eq!(
        arity_shape_codes("foreach x $l1 y $l2 {puts \"$x $y\"}\n"),
        Vec::<String>::new()
    );
}

#[test]
fn foreach_expanded_tail_never_false_fires_e005() {
    // FP guard: `{*}`-expanded args make the true final count unknowable
    // — the parity check must abstain exactly like E002 does.
    assert_eq!(
        arity_shape_codes("foreach x $l {*}$rest\n"),
        Vec::<String>::new()
    );
}

#[test]
fn switch_unpaired_pattern_fires_e005() {
    // `switch ?options? string pattern body ?pattern body ...?` — a flat
    // (non-braced) form's total count (subject + patterns/bodies) is
    // always odd; an even count (here 4: subject + 3 more words) leaves
    // an unpaired trailing pattern with no body (tclsh 8.4-9.0: `switch --
    // q a b c` fails "extra switch pattern with no body"). Without `--` a
    // subject such as `-exact` is an option and the words pair up.
    assert_eq!(
        arity_shape_codes("proc p {s} {switch -- $s a b c}\n"),
        vec!["E005".to_owned()]
    );
}

#[test]
fn switch_flat_pairs_are_silent() {
    assert_eq!(
        arity_shape_codes("switch $s a b c d\n"),
        Vec::<String>::new()
    );
}

#[test]
fn switch_single_braced_body_shorthand_is_silent() {
    // The `also_exact` union member: a single braced blob after the
    // subject (exactly 2 total args) is the documented shorthand for any
    // number of pattern/body pairs — never flagged, however many pairs
    // the braced blob's *content* logically holds.
    assert_eq!(
        arity_shape_codes("switch $s {a b c d e f}\n"),
        Vec::<String>::new()
    );
}

#[test]
fn switch_option_skip_does_not_shift_the_parity_check() {
    // Leading declared options (skipped before the parity check applies)
    // must not throw off the count `nargs_min` measures against.
    assert_eq!(
        arity_shape_codes("switch -exact -- $s a b c\n"),
        vec!["E005".to_owned()]
    );
    assert_eq!(
        arity_shape_codes("switch -exact -- $s a b c d\n"),
        Vec::<String>::new()
    );
}

#[test]
fn e005_does_not_double_fire_with_e002_or_e003() {
    // A genuinely too-few / too-many count is E002/E003's job, not
    // E005's — the shape check only applies once the count is already
    // within `[min, max]`.
    assert_eq!(arity_shape_codes("switch $s\n"), vec!["E002".to_owned()]);
    assert_eq!(arity_shape_codes("foreach x\n"), vec!["E002".to_owned()]);
}

// From Tcl's own `tools/findBadExternals.tcl`.
const ISSUE_703_SOURCE: &str = "\
proc main {argc argv} {
    lassign $argv libtcl
    try {
        switch $::tcl_platform(platform) {
            unix - macosx {
                exec nm --extern-only --defined-only $libtcl
            }
            windows {
                exec dumpbin /exports $libtcl
            }
        }
    } on ok result - trap NONE result {
        foreach line [split $result \\n] {
            puts $line
        }
        return 0
    } on error msg {
        puts stderr $msg
        return 1
    }
}
";

#[test]
fn issue_703_fallthrough_dash_has_no_e002() {
    // The solo `-` on the `on ok result - trap NONE result` line must not be
    // flagged as a zero-arg `-` command.
    assert_eq!(count_code(ISSUE_703_SOURCE, "E002"), 0);
}

#[test]
fn issue_703_braced_dash_has_no_e002() {
    let src =
        "proc p {} {\n    try {set x 1} on ok a {-} trap NONE b {\n        return $b\n    }\n}\n";
    assert_eq!(count_code(src, "E002"), 0);
}

#[test]
fn issue_703_genuine_dash_command_in_handler_still_flagged() {
    // A real zero-arg `-` *command* inside a handler body is still a genuine
    // arity error — the fix must not blunt E002 here.
    let src = "proc p {} {\n    try {set x 1} on error msg {\n        -\n    }\n}\n";
    assert_eq!(count_code(src, "E002"), 1);
}

#[test]
fn issue_703_genuine_dash_command_outside_try_still_flagged() {
    assert_eq!(count_code("proc f {} {\n    -\n}\n", "E002"), 1);
}

#[test]
fn scratch_dup_e003_per_item() {
    let mut a = Analyser::new();
    let r = a.analyse_per_item("set var 10 10\n", "tcl8.6");
    let e003: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E003)
        .collect();
    eprintln!("per_item E003 count = {}", e003.len());
    for d in &e003 {
        eprintln!("  span={:?} msg={}", d.span, d.message);
    }
}
